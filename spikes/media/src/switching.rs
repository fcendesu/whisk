//! Two synthetic video publishers with generated audio continuity across viewer focus changes.
use crate::{
    capture::VideoFrame,
    codec::VideoProfile,
    loopback::{MediaError, bgra_to_i420, decoded_bgra},
    room::{ProbeRoom, connect_probe},
};
use futures_util::StreamExt;
use livekit::{
    options::{TrackPublishOptions, VideoCodec, VideoEncoding},
    prelude::*,
    webrtc::{
        audio_source::{AudioSourceOptions, RtcAudioSource, native::NativeAudioSource},
        stats::RtcStats,
        video_frame::{VideoFrame as RtcFrame, VideoRotation},
        video_source::{RtcVideoSource, VideoResolution, native::NativeVideoSource},
        video_stream::native::NativeVideoStream,
    },
};
use serde::Serialize;
use std::time::Duration;
use tokio::time::{Instant, MissedTickBehavior, interval, sleep_until, timeout};

#[derive(Debug, Serialize)]
pub struct SwitchingReport {
    pub submitted_frames: [u64; 2],
    pub received_size: [[u32; 2]; 2],
    pub requested_profile: VideoProfile,
    pub requested_fps: u32,
    pub frame_age_samples: u64,
    pub invalid_timestamps: u64,
    pub frame_age_p50_ms: Option<u32>,
    pub frame_age_p95_ms: Option<u32>,
    pub frame_age_max_ms: Option<u32>,
    pub encoder: [Option<String>; 2],
    pub decoder: [Option<String>; 2],
    pub phase_duration_ms: [u64; 2],
    pub received_frames: [u64; 2],
    pub wrong_source_frames: u64,
    pub duration_ms: u64,
    pub audio_verified: bool,
    pub audio_source_calls: u64,
    pub audio_source_max_interval_ms: u64,
    pub audio_source_max_wait_ms: u64,
    pub audio_received_frames: [u64; 2],
    pub audio_receive_max_interval_ms: u64,
    pub audio_observation_ms: u64,
    pub audio_packets_received: Option<u64>,
    pub audio_packets_lost: Option<i64>,
    pub audio_jitter_ms: Option<f64>,
    pub audio_concealed_samples: Option<u64>,
    pub audio_rtp_stats_scope: &'static str,
    pub video_submit_max_us: u64,
    pub video_frame_prepare_max_us: u64,
    pub audible_audio_frames: [u64; 2],
    pub max_audio_gap_ms: u64,
    pub tone_frequency_hz: f64,
    pub microphone_verified: bool,
    pub audio_source: &'static str,
    pub quality_certified: bool,
    pub publisher_transports: [Vec<crate::transport::TransportReport>; 2],
    pub viewer_transports: [Vec<crate::transport::TransportReport>; 2],
}

pub async fn run_switch_probe(
    url: &str,
    first_token: &str,
    second_token: &str,
    viewer_token: &str,
    phase: Duration,
) -> Result<SwitchingReport, MediaError> {
    run_profile_probe(
        url,
        first_token,
        second_token,
        viewer_token,
        phase,
        VideoProfile::P720F30,
    )
    .await
}

/// Experimental profile attempts bypass release policy only to collect evidence.
/// Success does not certify capability; product profile selection still uses select_codec.
pub async fn run_profile_probe(
    url: &str,
    first: &str,
    second: &str,
    viewer: &str,
    phase: Duration,
    profile: VideoProfile,
) -> Result<SwitchingReport, MediaError> {
    run_profile_internal(url, first, second, viewer, phase, profile, None).await
}

/// Feed the same bounded GPUI bridge used by the native capture preview.
pub async fn run_profile_preview(
    url: &str,
    first: &str,
    second: &str,
    viewer: &str,
    phase: Duration,
    profile: VideoProfile,
    target: crate::loopback::PreviewTarget,
) -> Result<SwitchingReport, MediaError> {
    run_profile_internal(url, first, second, viewer, phase, profile, Some(target)).await
}

async fn run_profile_internal(
    url: &str,
    first_token: &str,
    second_token: &str,
    viewer_token: &str,
    phase: Duration,
    profile: VideoProfile,
    target: Option<crate::loopback::PreviewTarget>,
) -> Result<SwitchingReport, MediaError> {
    if target
        .as_ref()
        .is_some_and(crate::loopback::PreviewTarget::cancelled)
    {
        return Err(MediaError::Cancelled);
    }
    if phase.is_zero()
        || phase > Duration::from_secs(300)
        || [first_token, second_token, viewer_token]
            .iter()
            .any(|token| token.trim().is_empty())
    {
        return Err(MediaError::InvalidInput);
    }
    let first = connect_preview_room(url, first_token, target.as_ref()).await?;
    let second = match connect_preview_room(url, second_token, target.as_ref()).await {
        Ok(room) => room,
        Err(error) => {
            let _ = first.close().await;
            return Err(error);
        }
    };
    let mut viewer = match connect_preview_room(url, viewer_token, target.as_ref()).await {
        Ok(room) => room,
        Err(error) => {
            let _ = first.close().await;
            let _ = second.close().await;
            return Err(error);
        }
    };
    let result = tokio::select! {
        result=probe([&first, &second], &mut viewer, phase, profile,target.as_ref())=>result,
        _=crate::loopback::wait_cancelled(target.as_ref())=>Err(MediaError::Cancelled),
    };
    let first_closed = first.close().await;
    let second_closed = second.close().await;
    let viewer_closed = viewer.close().await;
    let report = result?;
    if first_closed.is_err() || second_closed.is_err() || viewer_closed.is_err() {
        return Err(MediaError::Close);
    }
    Ok(report)
}

async fn connect_preview_room(
    url: &str,
    token: &str,
    target: Option<&crate::loopback::PreviewTarget>,
) -> Result<ProbeRoom, MediaError> {
    tokio::select! {
        result=connect_probe(url,token)=>result.map_err(|_|MediaError::Connect),
        _=crate::loopback::wait_cancelled(target)=>Err(MediaError::Cancelled),
    }
}

async fn probe(
    publishers: [&ProbeRoom; 2],
    viewer: &mut ProbeRoom,
    phase: Duration,
    profile: VideoProfile,
    target: Option<&crate::loopback::PreviewTarget>,
) -> Result<SwitchingReport, MediaError> {
    let (width, height, fps, bitrate) = match profile {
        VideoProfile::P720F30 => (1280, 720, 30, 2_000_000),
        VideoProfile::P1080F60 => (1920, 1080, 60, 6_000_000),
        VideoProfile::P1440F60 => (2560, 1440, 60, 10_000_000),
    };
    let sources: [NativeVideoSource; 2] =
        std::array::from_fn(|_| NativeVideoSource::new(VideoResolution { width, height }, true));
    let tracks: [LocalVideoTrack; 2] = std::array::from_fn(|index| {
        LocalVideoTrack::create_video_track(
            &format!("whisk-synthetic-{index}"),
            RtcVideoSource::Native(sources[index].clone()),
        )
    });
    let mut publications = Vec::new();
    for (publisher, track) in publishers.iter().zip(&tracks) {
        let publication = timeout(
            Duration::from_secs(10),
            publisher.sdk_room().local_participant().publish_track(
                LocalTrack::Video(track.clone()),
                TrackPublishOptions {
                    source: TrackSource::Screenshare,
                    video_codec: VideoCodec::H264,
                    simulcast: false,
                    video_encoding: Some(VideoEncoding {
                        max_bitrate: bitrate,
                        max_framerate: fps as f64,
                    }),
                    ..Default::default()
                },
            ),
        )
        .await
        .map_err(|_| MediaError::Publish)?
        .map_err(|_| MediaError::Publish)?;
        publications.push(publication);
    }
    let audio_source = NativeAudioSource::new(AudioSourceOptions::default(), 48_000, 1, 100);
    let audio_track = LocalAudioTrack::create_audio_track(
        "whisk-generated-440hz-tone",
        RtcAudioSource::Native(audio_source.clone()),
    );
    let audio_publication = timeout(
        Duration::from_secs(10),
        publishers[0].sdk_room().local_participant().publish_track(
            LocalTrack::Audio(audio_track),
            TrackPublishOptions {
                source: TrackSource::Microphone,
                dtx: false,
                ..Default::default()
            },
        ),
    )
    .await
    .map_err(|_| MediaError::Publish)?
    .map_err(|_| MediaError::Publish)?;
    let result=async {
        let mut remote:[Option<RemoteVideoTrack>;2]=[None,None];
        let mut remote_audio = None;
        timeout(Duration::from_secs(10),async {
            while remote.iter().any(Option::is_none) || remote_audio.is_none() {
                match viewer.next_event().await {
                    Some(RoomEvent::TrackSubscribed {track:RemoteTrack::Video(track),publication,..})=>{
                        if let Some(index)=publications.iter().position(|expected|expected.sid()==publication.sid()) { remote[index]=Some(track); }
                    }
                    Some(RoomEvent::TrackSubscribed {track:RemoteTrack::Audio(track),publication,..}) if publication.sid() == audio_publication.sid() => { remote_audio = Some(track); }
                    Some(_)=>{},
                    None=>return Err(MediaError::Subscribe),
                }
            }
            Ok(())
        }).await.map_err(|_|MediaError::Subscribe)??;
        let selected=[remote[0].take().ok_or(MediaError::Subscribe)?,remote[1].take().ok_or(MediaError::Subscribe)?];
        let mut patterns=[[40,140,230,255],[230,120,40,255]].map(|color|VideoFrame {width,height,bgra:color.into_iter().cycle().take(width as usize * height as usize * 4).collect::<Vec<_>>().into()});
        let mut report=SwitchingReport{submitted_frames:[0;2],received_size:[[0;2];2],requested_profile:profile,requested_fps:fps,frame_age_samples:0,invalid_timestamps:0,frame_age_p50_ms:None,frame_age_p95_ms:None,frame_age_max_ms:None,encoder:[None,None],decoder:[None,None],phase_duration_ms:[0;2],received_frames:[0;2],wrong_source_frames:0,duration_ms:0,audio_verified:false,audio_source_calls:0,audio_source_max_interval_ms:0,audio_source_max_wait_ms:0,audio_received_frames:[0;2],audio_receive_max_interval_ms:0,audio_observation_ms:0,audio_packets_received:None,audio_packets_lost:None,audio_jitter_ms:None,audio_concealed_samples:None,audio_rtp_stats_scope:"track_cumulative_after_observation",video_submit_max_us:0,video_frame_prepare_max_us:0,audible_audio_frames:[0;2],max_audio_gap_ms:0,tone_frequency_hz:0.0,microphone_verified:false,audio_source:"generated_440hz_pcm",quality_certified:false,publisher_transports:[Vec::new(),Vec::new()],viewer_transports:[Vec::new(),Vec::new()]};
        let mut ages = Vec::new();
        let started=Instant::now();
        let remote_audio = remote_audio.ok_or(MediaError::Subscribe)?;
        let mut audio = crate::audio_probe::AudioTask::start(audio_source, remote_audio.clone(), started);
        for (focus,track) in selected.iter().enumerate() {
            audio.focus(focus == 1);
            let mut stream=NativeVideoStream::new(track.rtc_track());
            let phase_start=Instant::now();
            let end=phase_start+phase;
            let mut ticks=interval(Duration::from_secs_f64(1./fps as f64));
            ticks.set_missed_tick_behavior(MissedTickBehavior::Skip);
            loop {
                tokio::select! {
                    _=sleep_until(end)=>break,
                    error=audio.stopped_unexpectedly()=>return Err(error),
                    _=ticks.tick()=>{
                        let prepare_started = Instant::now();
                        for index in 0..2 {
                            crate::frame_clock::stamp(&mut patterns[index], started.elapsed().as_millis() as u32)?;
                            let frame=RtcFrame{rotation:VideoRotation::VideoRotation0,timestamp_us:started.elapsed().as_micros().min(i64::MAX as u128) as i64,frame_metadata:None,buffer:bgra_to_i420(&patterns[index])?};
                            if sources[index].capture_frame(&frame) { report.submitted_frames[index]+=1; }
                        }
                        report.video_submit_max_us = report.video_submit_max_us.max(prepare_started.elapsed().as_micros().min(u64::MAX as u128) as u64);
                    }
                    event=viewer.next_event()=>{
                        match event {
                            Some(RoomEvent::ConnectionStateChanged(ConnectionState::Disconnected | ConnectionState::Reconnecting)) | None=>return Err(MediaError::Connect),
                            Some(RoomEvent::TrackUnsubscribed{publication,..}) if (publication.sid()==publications[focus].sid() || publication.sid()==audio_publication.sid())=>return Err(MediaError::Decode),
                            _=>{},
                        }
                    }
                    frame=stream.next()=>{
                        let prepare_started = Instant::now();
                        let frame=decoded_bgra(&frame.ok_or(MediaError::Decode)?)?;
                        report.video_frame_prepare_max_us = report.video_frame_prepare_max_us.max(prepare_started.elapsed().as_micros().min(u64::MAX as u128) as u64);
                        report.received_size[focus] = [frame.width, frame.height];
                        let [b,g,r,_]:[u8;4]=frame.bgra[..4].try_into().map_err(|_|MediaError::Decode)?;
                        if b<15 && g<15 && r<15 { continue; }
                        let correct=if focus==0 {r>180 && b<80 && g>80} else {b>180 && r<80 && g>80};
                        if !correct { report.wrong_source_frames+=1; }
                        let now = started.elapsed().as_millis() as u32;
                        match crate::frame_clock::frame_age_ms(&frame, now) {
                            Some(age) => { if ages.len() < 100_000 { ages.push(age); } }
                            None => { report.invalid_timestamps += 1; }
                        }
                        report.received_frames[focus]+=1;
                        if let Some(target)=target {target.frames.send(crate::capture::FrameEvent::Frame(std::sync::Arc::new(frame)));}
                    }
                }
            }
            stream.close();
            report.phase_duration_ms[focus] = phase_start.elapsed().as_millis() as u64;
        }
        let audio = audio.finish().await?;
        report.frame_age_samples = ages.len() as u64;
        if let Some([p50,p95,max]) = crate::benchmark::latency_percentiles(&mut ages) {
            report.frame_age_p50_ms = Some(p50);
            report.frame_age_p95_ms = Some(p95);
            report.frame_age_max_ms = Some(max);
        }
        report.tone_frequency_hz = audio.frequency_hz;
        report.duration_ms=started.elapsed().as_millis() as u64;
        report.audible_audio_frames = audio.frames;
        report.max_audio_gap_ms = audio.max_gap_ms;
        report.audio_verified = audio.verified;
        report.audio_source_calls = audio.source_calls;
        report.audio_source_max_interval_ms = audio.source_max_interval_ms;
        report.audio_source_max_wait_ms = audio.source_max_wait_ms;
        report.audio_received_frames = audio.received_frames;
        report.audio_receive_max_interval_ms = audio.receive_max_interval_ms;
        report.audio_observation_ms = audio.observation_ms;
        if let Ok(Ok(stats)) = timeout(Duration::from_secs(2), remote_audio.get_stats()).await {
            for stat in stats {
                if let RtcStats::InboundRtp(input) = stat && input.stream.kind == "audio" {
                    report.audio_packets_received = Some(input.received.packets_received);
                    report.audio_packets_lost = Some(input.received.packets_lost);
                    report.audio_jitter_ms = Some(input.received.jitter * 1000.0);
                    report.audio_concealed_samples = Some(input.inbound.concealed_samples);
                }
            }
        }
        for index in 0..2 {
            let stats = timeout(Duration::from_secs(3), tracks[index].get_stats()).await.map_err(|_|MediaError::Stats)?.map_err(|_|MediaError::Stats)?;
            report.publisher_transports[index] = crate::transport::selected_transports(&stats);
            for stat in stats { if let RtcStats::OutboundRtp(out) = stat && !out.outbound.encoder_implementation.is_empty() { report.encoder[index] = Some(out.outbound.encoder_implementation); } }
            let stats = timeout(Duration::from_secs(3), selected[index].get_stats()).await.map_err(|_|MediaError::Stats)?.map_err(|_|MediaError::Stats)?;
            report.viewer_transports[index] = crate::transport::selected_transports(&stats);
            for stat in stats { if let RtcStats::InboundRtp(input) = stat && !input.inbound.decoder_implementation.is_empty() { report.decoder[index] = Some(input.inbound.decoder_implementation); } }
        }
        if report.received_frames.contains(&0) { return Err(MediaError::NoFrames); }
        Ok(report)
    }.await;
    let mut clean = true;
    for (publisher, publication) in publishers.iter().zip(&publications) {
        if !matches!(
            timeout(
                Duration::from_secs(5),
                publisher
                    .sdk_room()
                    .local_participant()
                    .unpublish_track(&publication.sid())
            )
            .await,
            Ok(Ok(_))
        ) {
            clean = false;
        }
    }
    if !matches!(
        timeout(
            Duration::from_secs(5),
            publishers[0]
                .sdk_room()
                .local_participant()
                .unpublish_track(&audio_publication.sid())
        )
        .await,
        Ok(Ok(_))
    ) {
        clean = false;
    }
    let report = result?;
    if !clean {
        return Err(MediaError::Close);
    }
    Ok(report)
}
