//! Generated audio runs independently of the CPU video conversion/viewer loop.
use crate::{benchmark::AudioContinuity, loopback::MediaError};
use futures_util::StreamExt;
use livekit::{
    prelude::RemoteAudioTrack,
    webrtc::{
        audio_frame::AudioFrame, audio_source::native::NativeAudioSource,
        audio_stream::native::NativeAudioStream,
    },
};
use std::time::Duration;
use tokio::{
    sync::{oneshot, watch},
    task::JoinHandle,
    time::{Instant, MissedTickBehavior, interval, timeout},
};

pub(crate) struct ToneReport {
    pub frames: [u64; 2],
    pub max_gap_ms: u64,
    pub frequency_hz: f64,
    pub verified: bool,
}
pub(crate) struct AudioTask {
    phase: watch::Sender<bool>,
    stop: Option<oneshot::Sender<()>>,
    task: JoinHandle<Result<ToneReport, MediaError>>,
}
impl AudioTask {
    pub fn start(source: NativeAudioSource, track: RemoteAudioTrack, started: Instant) -> Self {
        let (phase, phase_rx) = watch::channel(false);
        let (stop, stop_rx) = oneshot::channel();
        let task = tokio::task::spawn_blocking(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|_| MediaError::Decode)?;
            runtime.block_on(monitor(source, track, started, phase_rx, stop_rx))
        });
        Self {
            phase,
            stop: Some(stop),
            task,
        }
    }
    pub fn focus(&self, second: bool) {
        self.phase.send_replace(second);
    }
    pub async fn stopped_unexpectedly(&mut self) -> MediaError {
        match (&mut self.task).await {
            Ok(Err(error)) => error,
            _ => MediaError::Decode,
        }
    }
    pub async fn finish(mut self) -> Result<ToneReport, MediaError> {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        timeout(Duration::from_secs(2), &mut self.task)
            .await
            .map_err(|_| MediaError::Decode)?
            .map_err(|_| MediaError::Decode)?
    }
}
impl Drop for AudioTask {
    fn drop(&mut self) {
        // spawn_blocking cannot be aborted once started. Closing this explicit
        // signal ends monitor; its source wait is independently bounded to500ms.
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
    }
}
async fn monitor(
    source: NativeAudioSource,
    track: RemoteAudioTrack,
    started: Instant,
    phase: watch::Receiver<bool>,
    mut stop: oneshot::Receiver<()>,
) -> Result<ToneReport, MediaError> {
    let mut stream = NativeAudioStream::new(track.rtc_track(), 48_000, 1);
    let mut ticks = interval(Duration::from_millis(20));
    ticks.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut generated = 0u64;
    let mut measured = 0u64;
    let mut crossings = 0u64;
    let mut previous = 0i16;
    let mut continuity = AudioContinuity::default();
    loop {
        tokio::select! {
            _=&mut stop => break,
            _=ticks.tick()=>{
                let samples:Vec<i16>=(0..960).map(|offset| (8000.0*(std::f64::consts::TAU*440.0*(generated+offset) as f64/48_000.0).sin()) as i16).collect();
                generated+=960;
                let frame=AudioFrame { data:samples.into(),sample_rate:48_000,num_channels:1,samples_per_channel:960 };
                timeout(Duration::from_millis(500),source.capture_frame(&frame)).await.map_err(|_|MediaError::Publish)?.map_err(|_|MediaError::Publish)?;
            }
            frame=stream.next()=>{
                let frame=frame.ok_or(MediaError::Decode)?;
                if frame.sample_rate!=48_000 || frame.num_channels!=1 { return Err(MediaError::Decode); }
                if frame.data.iter().any(|sample|sample.unsigned_abs()>500) {
                    continuity.observe(*phase.borrow(),started.elapsed().as_millis() as u64);
                    for sample in frame.data.iter().copied() { if (sample>=0)!=(previous>=0) {crossings+=1;} previous=sample; }
                    measured+=frame.data.len() as u64;
                }
            }
        }
    }
    let continuity = continuity.finish(started.elapsed().as_millis() as u64);
    stream.close();
    let frequency_hz = if measured > 0 {
        crossings as f64 * 48_000.0 / (2.0 * measured as f64)
    } else {
        0.0
    };
    Ok(ToneReport {
        frames: continuity.audible_frames,
        max_gap_ms: continuity.max_gap_ms,
        frequency_hz,
        verified: continuity.continuous && (420.0..460.0).contains(&frequency_hz),
    })
}
