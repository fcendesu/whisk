//! Explicit audio probe. Reports contain counters only, never recorded PCM.
use crate::{
    loopback::MediaError,
    room::{ProbeRoom, connect_probe},
    transport::{TransportReport, selected_transports},
};
use futures_util::StreamExt;
use livekit::{
    PlatformAudio,
    options::TrackPublishOptions,
    prelude::*,
    rtc_engine::lk_runtime::LkRuntime,
    webrtc::{
        audio_frame::AudioFrame,
        audio_source::{RtcAudioSource, native::NativeAudioSource},
        audio_stream::native::NativeAudioStream,
        peer_connection_factory::native::PeerConnectionFactoryExt,
        stats::RtcStats,
    },
};
use serde::Serialize;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::time::{Instant, MissedTickBehavior, interval, sleep, sleep_until, timeout};

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioInput {
    Generated,
    Microphone,
    SpeakerTest,
}
#[derive(Debug, Serialize)]
pub struct AudioReport {
    pub audio_source: AudioInput,
    pub decoded_frames: u64,
    pub decoded_samples: u64,
    pub non_silent_frames: u64,
    pub peak_sample: u16,
    pub duration_ms: u64,
    pub codec: Option<String>,
    pub microphone_verified: bool,
    pub playback_verified: bool,
    pub playback_started: bool,
    pub user_heard_tone: Option<bool>,
    pub publisher_transports: Vec<TransportReport>,
    pub viewer_transports: Vec<TransportReport>,
}

/// Owns the physical device only while the measurement is active. This probe
/// runs in a separate process: disabling ADM playout cannot affect another app.
struct MicrophoneGuard(PlatformAudio);
impl MicrophoneGuard {
    fn start() -> Result<Self, MediaError> {
        let guard = Self(PlatformAudio::new().map_err(|_| MediaError::Capture)?);
        let runtime = LkRuntime::instance();
        runtime.pc_factory().set_adm_playout_enabled(false);
        let _ = runtime.pc_factory().stop_playout();
        Ok(guard)
    }
}
impl Drop for MicrophoneGuard {
    fn drop(&mut self) {
        let _ = self.0.stop_recording();
        // Disables capture even if stopping the native device returned an error.
        LkRuntime::instance()
            .pc_factory()
            .set_adm_recording_enabled(false);
    }
}

/// Hardware boundary for deterministic speaker lifecycle checks without devices.
trait SpeakerDevice {
    fn disable_recording(&self);
    fn acquire(&self) -> bool;
    fn start_playout(&self) -> bool;
    fn stop_playout(&self);
    fn release(&self);
}
struct NativeSpeakerDevice(Arc<LkRuntime>);
impl SpeakerDevice for NativeSpeakerDevice {
    fn disable_recording(&self) {
        let factory = self.0.pc_factory();
        factory.set_adm_recording_enabled(false);
        let _ = factory.stop_recording();
    }
    fn acquire(&self) -> bool {
        self.0.pc_factory().acquire_platform_adm()
    }
    fn start_playout(&self) -> bool {
        let factory = self.0.pc_factory();
        factory.set_adm_playout_enabled(true);
        (factory.playout_is_initialized() || factory.init_playout()) && factory.start_playout()
    }
    fn stop_playout(&self) {
        let factory = self.0.pc_factory();
        let _ = factory.stop_playout();
        factory.set_adm_playout_enabled(false);
    }
    fn release(&self) {
        self.0.pc_factory().release_platform_adm();
    }
}
/// Does not use PlatformAudio::new(), which would enable microphone recording.
struct SpeakerGuard<D: SpeakerDevice = NativeSpeakerDevice> {
    device: D,
}
impl SpeakerGuard {
    fn start() -> Result<Self, MediaError> {
        Self::start_on(NativeSpeakerDevice(LkRuntime::instance()))
    }
}
impl<D: SpeakerDevice> SpeakerGuard<D> {
    fn start_on(device: D) -> Result<Self, MediaError> {
        device.disable_recording();
        if !device.acquire() {
            return Err(MediaError::Capture);
        }
        let guard = Self { device };
        guard.device.disable_recording();
        if !guard.device.start_playout() {
            return Err(MediaError::Capture);
        }
        Ok(guard)
    }
}
impl<D: SpeakerDevice> Drop for SpeakerGuard<D> {
    fn drop(&mut self) {
        self.device.stop_playout();
        self.device.disable_recording();
        self.device.release();
    }
}

pub async fn run_audio_probe(
    url: &str,
    publisher_token: &str,
    viewer_token: &str,
    duration: Duration,
    input: AudioInput,
    cancelled: Arc<AtomicBool>,
) -> Result<AudioReport, MediaError> {
    if cancelled.load(Ordering::Acquire) {
        return Err(MediaError::Cancelled);
    }
    if duration.is_zero()
        || duration > Duration::from_secs(30)
        || (matches!(input, AudioInput::SpeakerTest) && duration > Duration::from_secs(3))
        || publisher_token.trim().is_empty()
        || viewer_token.trim().is_empty()
    {
        return Err(MediaError::InvalidInput);
    }
    let publisher = tokio::select! {
     result=connect_probe(url,publisher_token)=>result.map_err(|_|MediaError::Connect)?,
     _=wait_cancelled(&cancelled)=>return Err(MediaError::Cancelled),
    };
    let viewer = tokio::select! {
     result=connect_probe(url,viewer_token)=>result.map_err(|_|MediaError::Connect),
     _=wait_cancelled(&cancelled)=>Err(MediaError::Cancelled),
    };
    let mut viewer = match viewer {
        Ok(viewer) => viewer,
        Err(error) => {
            let _ = publisher.close().await;
            return Err(error);
        }
    };
    let result = pipeline(&publisher, &mut viewer, duration, input, &cancelled).await;
    let publisher_closed = publisher.close().await;
    let viewer_closed = viewer.close().await;
    let report = result?;
    publisher_closed.map_err(|_| MediaError::Close)?;
    viewer_closed.map_err(|_| MediaError::Close)?;
    Ok(report)
}

async fn pipeline(
    publisher: &ProbeRoom,
    viewer: &mut ProbeRoom,
    duration: Duration,
    input: AudioInput,
    cancelled: &AtomicBool,
) -> Result<AudioReport, MediaError> {
    let capture_deadline =
        matches!(input, AudioInput::Microphone).then(|| Instant::now() + duration);
    let mut device = None;
    let mut speakers = None;
    let generated = match input {
        AudioInput::Generated | AudioInput::SpeakerTest => {
            if matches!(input, AudioInput::SpeakerTest) {
                speakers = Some(start_if_active(cancelled, SpeakerGuard::start)?);
            }
            Some(NativeAudioSource::new(Default::default(), 48000, 1, 100))
        }
        AudioInput::Microphone => {
            device = Some(start_if_active(cancelled, MicrophoneGuard::start)?);
            None
        }
    };
    let source = generated
        .as_ref()
        .map(|source| RtcAudioSource::Native(source.clone()))
        .unwrap_or(RtcAudioSource::Device);
    let track = LocalAudioTrack::create_audio_track("whisk-m0-audio-probe", source);
    let participant = publisher.sdk_room().local_participant();
    let publication = tokio::select! {
     _=capture_expired(capture_deadline)=>return Err(MediaError::NoFrames),
     _=wait_cancelled(cancelled)=>return Err(MediaError::Cancelled),
     result=timeout(Duration::from_secs(10),participant.publish_track(LocalTrack::Audio(track.clone()),TrackPublishOptions {source:TrackSource::Microphone,..Default::default()}))=>result.map_err(|_|MediaError::Publish)?.map_err(|_|MediaError::Publish)?,
    };
    let result=async {
  let remote=tokio::select! {
   _=capture_expired(capture_deadline)=>return Err(MediaError::NoFrames),
   _=wait_cancelled(cancelled)=>return Err(MediaError::Cancelled),
   result=timeout(Duration::from_secs(10),async {
    loop {match viewer.next_event().await {
     Some(RoomEvent::TrackSubscribed {track:RemoteTrack::Audio(track),publication:remote,..}) if remote.sid()==publication.sid()=>return Ok(track),
     Some(RoomEvent::ConnectionStateChanged(ConnectionState::Disconnected|ConnectionState::Reconnecting))|None=>return Err(MediaError::Connect),
     _=>{},
    }}
   })=>result.map_err(|_|MediaError::Subscribe)??,
  };
  // Microphone mode has playback off; SpeakerTest supplies generated PCM only.
  let mut stream=NativeAudioStream::new(remote.rtc_track(),48000,1);
  let started=Instant::now();
  let mut ticks=interval(Duration::from_millis(20));
  ticks.set_missed_tick_behavior(MissedTickBehavior::Skip);
  let mut sample_offset=0u64;
  let mut report=AudioReport {audio_source:input,decoded_frames:0,decoded_samples:0,non_silent_frames:0,peak_sample:0,duration_ms:0,codec:None,microphone_verified:false,playback_verified:false,playback_started:speakers.is_some(),user_heard_tone:None,publisher_transports:Vec::new(),viewer_transports:Vec::new()};
  let measured=async {
   loop {tokio::select! {
    _=sleep_until(capture_deadline.unwrap_or(started+duration))=>break,
    _=wait_cancelled(cancelled)=>return Err(MediaError::Cancelled),
    _=ticks.tick(), if generated.is_some()=>{
     let samples:Vec<i16>=(0..960).map(|offset|((if matches!(input,AudioInput::SpeakerTest){1200.0}else{8000.0})*(std::f64::consts::TAU*440.0*(sample_offset+offset) as f64/48000.0).sin()) as i16).collect();
     sample_offset+=960;
     let frame=AudioFrame {data:samples.into(),sample_rate:48000,num_channels:1,samples_per_channel:960};
     timeout(Duration::from_millis(500),generated.as_ref().unwrap().capture_frame(&frame)).await.map_err(|_|MediaError::Publish)?.map_err(|_|MediaError::Publish)?;
    },
    event=viewer.next_event()=>match event {
     Some(RoomEvent::TrackUnsubscribed {publication:remote,..}) if remote.sid()==publication.sid()=>return Err(MediaError::Decode),
     Some(RoomEvent::ConnectionStateChanged(ConnectionState::Disconnected|ConnectionState::Reconnecting))|None=>return Err(MediaError::Connect),
     _=>{},
    },
    frame=stream.next()=>{
     let frame=frame.ok_or(MediaError::Decode)?;
     if frame.sample_rate!=48000 || frame.num_channels!=1 || frame.data.is_empty() {return Err(MediaError::Decode);}
     report.decoded_frames+=1;
     report.decoded_samples+=frame.data.len() as u64;
     let peak=frame.data.iter().map(|sample|sample.unsigned_abs()).max().unwrap_or(0);
     report.peak_sample=report.peak_sample.max(peak);
     if peak>500 {report.non_silent_frames+=1;}
    },
   }}
   Ok(())
  }.await;
  stream.close();
  // End physical recording before stats, unpublish, and room-close waits.
  drop(device.take());
  drop(speakers.take());
  measured?;
  report.duration_ms=started.elapsed().as_millis() as u64;
  if report.decoded_frames==0 {return Err(MediaError::NoFrames);}
  report.microphone_verified=matches!(input,AudioInput::Microphone) && report.non_silent_frames>0;
  let local=timeout(Duration::from_secs(3),track.get_stats()).await.map_err(|_|MediaError::Stats)?.map_err(|_|MediaError::Stats)?;
  let received=timeout(Duration::from_secs(3),remote.get_stats()).await.map_err(|_|MediaError::Stats)?.map_err(|_|MediaError::Stats)?;
  let codec_id=local.iter().find_map(|stat|match stat {RtcStats::OutboundRtp(out)=>Some(out.stream.codec_id.as_str()),_=>None});
  report.codec=local.iter().find_map(|stat|match stat {RtcStats::Codec(codec) if Some(codec.rtc.id.as_str())==codec_id=>Some(codec.codec.mime_type.clone()),_=>None});
  report.publisher_transports=selected_transports(&local);
  report.viewer_transports=selected_transports(&received);
  Ok(report)
 }.await;
    drop(device.take());
    drop(speakers.take());
    let unpublished = timeout(
        Duration::from_secs(5),
        publisher
            .sdk_room()
            .local_participant()
            .unpublish_track(&publication.sid()),
    )
    .await;
    let report = result?;
    unpublished
        .map_err(|_| MediaError::Close)?
        .map_err(|_| MediaError::Close)?;
    Ok(report)
}
async fn wait_cancelled(cancelled: &AtomicBool) {
    while !cancelled.load(Ordering::Acquire) {
        sleep(Duration::from_millis(25)).await;
    }
}

async fn capture_expired(deadline: Option<Instant>) {
    match deadline {
        Some(deadline) => sleep_until(deadline).await,
        None => std::future::pending::<()>().await,
    }
}

// Source setup is synchronous; check on both sides of that boundary. A source
// created during cancellation must be dropped before it can be published.
fn start_if_active<T>(
    cancelled: &AtomicBool,
    start: impl FnOnce() -> Result<T, MediaError>,
) -> Result<T, MediaError> {
    if cancelled.load(Ordering::Acquire) {
        return Err(MediaError::Cancelled);
    }
    let source = start()?;
    if cancelled.load(Ordering::Acquire) {
        return Err(MediaError::Cancelled);
    }
    Ok(source)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancellation_after_join_does_not_call_the_device_factory() {
        let cancelled = AtomicBool::new(true);
        let result = start_if_active(&cancelled, || -> Result<(), MediaError> {
            panic!("must not open a device after Stop")
        });
        assert!(matches!(result, Err(MediaError::Cancelled)));
    }
    #[test]
    fn cancellation_during_setup_drops_the_device_before_returning() {
        struct Device<'a>(&'a AtomicBool);
        impl Drop for Device<'_> {
            fn drop(&mut self) {
                self.0.store(true, Ordering::Release);
            }
        }
        let cancelled = AtomicBool::new(false);
        let stopped = AtomicBool::new(false);
        let result = start_if_active(&cancelled, || {
            cancelled.store(true, Ordering::Release);
            Ok(Device(&stopped))
        });
        assert!(matches!(result, Err(MediaError::Cancelled)));
        assert!(stopped.load(Ordering::Acquire));
    }
}

impl AudioReport {
    /// Call only for the user's explicit response to a completed speaker test.
    pub fn confirm_playback(&mut self, heard: bool) -> Result<(), MediaError> {
        if !matches!(self.audio_source, AudioInput::SpeakerTest)
            || !self.playback_started
            || self.decoded_frames == 0
            || self.non_silent_frames == 0
        {
            return Err(MediaError::InvalidInput);
        }
        self.user_heard_tone = Some(heard);
        self.playback_verified = heard;
        Ok(())
    }
}

#[cfg(test)]
mod speaker_device_tests {
    use super::*;
    use std::{cell::RefCell, rc::Rc};
    struct Device {
        events: Rc<RefCell<Vec<&'static str>>>,
        acquire: bool,
        start: bool,
    }
    impl SpeakerDevice for Device {
        fn disable_recording(&self) {
            self.events.borrow_mut().push("recording_off");
        }
        fn acquire(&self) -> bool {
            assert_eq!(
                self.events.borrow().last(),
                Some(&"recording_off"),
                "microphone must be disabled before acquiring hardware"
            );
            self.events.borrow_mut().push("acquire");
            self.acquire
        }
        fn start_playout(&self) -> bool {
            assert_eq!(self.events.borrow().last(), Some(&"recording_off"));
            self.events.borrow_mut().push("start");
            self.start
        }
        fn stop_playout(&self) {
            self.events.borrow_mut().push("stop");
        }
        fn release(&self) {
            self.events.borrow_mut().push("release");
        }
    }
    #[test]
    fn speaker_start_and_drop_keep_recording_disabled() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let guard = SpeakerGuard::start_on(Device {
            events: events.clone(),
            acquire: true,
            start: true,
        })
        .unwrap_or_else(|_| panic!("expected speaker guard"));
        drop(guard);
        assert_eq!(
            *events.borrow(),
            [
                "recording_off",
                "acquire",
                "recording_off",
                "start",
                "stop",
                "recording_off",
                "release"
            ]
        );
    }
    #[test]
    fn failed_speaker_start_releases_acquired_hardware() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let result = SpeakerGuard::start_on(Device {
            events: events.clone(),
            acquire: true,
            start: false,
        });
        assert!(matches!(result, Err(MediaError::Capture)));
        assert_eq!(
            *events.borrow(),
            [
                "recording_off",
                "acquire",
                "recording_off",
                "start",
                "stop",
                "recording_off",
                "release"
            ]
        );
    }
    #[test]
    fn failed_acquisition_does_not_release_unowned_hardware() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let result = SpeakerGuard::start_on(Device {
            events: events.clone(),
            acquire: false,
            start: true,
        });
        assert!(matches!(result, Err(MediaError::Capture)));
        assert_eq!(*events.borrow(), ["recording_off", "acquire"]);
    }
}
