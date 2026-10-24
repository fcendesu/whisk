//! Native generated or explicitly selected capture video through a self-hosted SFU.
use crate::{
    capture::VideoFrame as BgraFrame,
    room::{ProbeRoom, connect_probe},
};
use futures_util::StreamExt;
use livekit::{
    options::{TrackPublishOptions, VideoCodec, VideoEncoding},
    prelude::*,
    webrtc::{
        native::yuv_helper::argb_to_i420,
        stats::RtcStats,
        video_frame::{I420Buffer, VideoFormatType, VideoFrame, VideoRotation},
        video_source::{RtcVideoSource, VideoResolution, native::NativeVideoSource},
        video_stream::native::NativeVideoStream,
    },
};
use serde::Serialize;
use std::time::Duration;
use tokio::time::{Instant, MissedTickBehavior, interval, sleep_until, timeout};
#[derive(Debug, Serialize)]
pub struct LoopbackReport {
    pub submitted_frames: u64,
    pub decoded_frames: u64,
    pub received_width: u32,
    pub received_height: u32,
    pub last_pixel_bgra: [u8; 4],
    pub codec: Option<String>,
    pub encoder: Option<String>,
    pub decoder: Option<String>,
    pub duration_ms: u64,
    pub high_quality_certified: bool,
    pub publisher_transports: Vec<crate::transport::TransportReport>,
    pub viewer_transports: Vec<crate::transport::TransportReport>,
}
#[derive(Debug, Serialize)]
#[serde(tag = "error", rename_all = "snake_case")]
pub enum MediaError {
    Cancelled,
    Capture,
    InvalidInput,
    Connect,
    Publish,
    Subscribe,
    Decode,
    Stats,
    NoFrames,
    Close,
}
impl std::fmt::Display for MediaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", serde_json::to_string(self).unwrap())
    }
}
impl std::error::Error for MediaError {}
pub async fn run_loopback(
    url: &str,
    publisher_token: &str,
    viewer_token: &str,
    duration: Duration,
) -> Result<LoopbackReport, MediaError> {
    run_internal(url, publisher_token, viewer_token, duration, None, None).await
}

async fn run_internal(
    url: &str,
    publisher_token: &str,
    viewer_token: &str,
    duration: Duration,
    target: Option<PreviewTarget>,
    mut input: Option<tokio::sync::watch::Receiver<crate::capture::FrameEvent>>,
) -> Result<LoopbackReport, MediaError> {
    if target.as_ref().is_some_and(PreviewTarget::cancelled) {
        return Err(MediaError::Cancelled);
    }
    if duration.is_zero()
        || duration > Duration::from_secs(60)
        || publisher_token.trim().is_empty()
        || viewer_token.trim().is_empty()
    {
        return Err(MediaError::InvalidInput);
    }
    if let Some(input) = input.as_mut() {
        if input.has_changed().is_err()
            || matches!(
                &*input.borrow(),
                crate::capture::FrameEvent::Error(_) | crate::capture::FrameEvent::Finished(_)
            )
        {
            return Err(MediaError::Capture);
        }
        input.mark_changed();
    }
    let publisher = tokio::select! {
        result = connect_probe(url, publisher_token) => result.map_err(|_| MediaError::Connect)?,
        _ = wait_cancelled(target.as_ref()) => return Err(MediaError::Cancelled),
    };
    let viewer_result = tokio::select! {
        result = connect_probe(url, viewer_token) => result.map_err(|_| MediaError::Connect),
        _ = wait_cancelled(target.as_ref()) => Err(MediaError::Cancelled),
    };
    let mut viewer = match viewer_result {
        Ok(viewer) => viewer,
        Err(error) => {
            let _ = publisher.close().await;
            return Err(error);
        }
    };
    let result = pipeline(&publisher, &mut viewer, duration, target.as_ref(), input).await;
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
    target: Option<&PreviewTarget>,
    mut input: Option<tokio::sync::watch::Receiver<crate::capture::FrameEvent>>,
) -> Result<LoopbackReport, MediaError> {
    let source = NativeVideoSource::new(
        VideoResolution {
            width: 1280,
            height: 720,
        },
        true,
    );
    let track = LocalVideoTrack::create_video_track(
        if input.is_some() {
            "whisk-m0-selected-window"
        } else {
            "whisk-m0-synthetic-screen"
        },
        RtcVideoSource::Native(source.clone()),
    );
    let publication = timeout(
        Duration::from_secs(10),
        publisher.sdk_room().local_participant().publish_track(
            LocalTrack::Video(track.clone()),
            TrackPublishOptions {
                source: TrackSource::Screenshare,
                video_codec: VideoCodec::H264,
                simulcast: false,
                video_encoding: Some(VideoEncoding {
                    max_bitrate: 2_000_000,
                    max_framerate: 30.0,
                }),
                ..Default::default()
            },
        ),
    )
    .await
    .map_err(|_| MediaError::Publish)?
    .map_err(|_| MediaError::Publish)?;
    let result = async {
        let remote = tokio::select! {
            _ = wait_cancelled(target) => return Err(MediaError::Cancelled),
            result = timeout(Duration::from_secs(10), async {
            loop {
                match viewer.next_event().await {
                    Some(RoomEvent::TrackSubscribed { track: RemoteTrack::Video(track), publication: remote_publication, .. }) if remote_publication.sid() == publication.sid() => return Ok(track),
                    Some(_) => {},
                    None => return Err(MediaError::Subscribe),
                }
            }
        }) => result.map_err(|_| MediaError::Subscribe)??,
        };
        let mut stream = NativeVideoStream::new(remote.rtc_track());
        let started = Instant::now();
        let end = started + duration;
        let mut ticks = interval(Duration::from_secs_f64(1.0 / 30.0));
        ticks.set_missed_tick_behavior(MissedTickBehavior::Skip);
        // Warm orange with deliberately distinct B/G/R values: catches channel-order bugs.
        let bgra: Vec<u8> = [40, 140, 230, 255].into_iter().cycle().take(1280 * 720 * 4).collect();
        let pattern = BgraFrame { width: 1280, height: 720, bgra: bgra.into() };
        let mut report = LoopbackReport { submitted_frames:0, decoded_frames:0, received_width:0, received_height:0, last_pixel_bgra:[0;4], codec:None, encoder:None, decoder:None, duration_ms:0, high_quality_certified:false, publisher_transports:Vec::new(),viewer_transports:Vec::new() };
        let captured = input.is_some();
        loop {
            tokio::select! {
                _ = sleep_until(end) => break,
                _ = wait_cancelled(target) => return Err(MediaError::Cancelled),
                _ = ticks.tick(), if !captured => {
                    let buffer = bgra_to_i420(&pattern)?;
                    let frame = VideoFrame { rotation: VideoRotation::VideoRotation0, timestamp_us: started.elapsed().as_micros().min(i64::MAX as u128) as i64, frame_metadata:None, buffer };
                    if source.capture_frame(&frame) { report.submitted_frames += 1; }
                }
                capture = next_capture_frame(&mut input), if captured => {
                    let captured_frame = capture?;
                    let buffer = bgra_to_i420(&captured_frame)?;
                    let frame = VideoFrame { rotation: VideoRotation::VideoRotation0, timestamp_us: started.elapsed().as_micros().min(i64::MAX as u128) as i64, frame_metadata:None, buffer };
                    if source.capture_frame(&frame) { report.submitted_frames += 1; }
                }
                event = viewer.next_event() => {
                    match event {
                        Some(RoomEvent::TrackUnsubscribed { publication: remote_publication, .. }) if remote_publication.sid() == publication.sid() => return Err(MediaError::Decode),
                        Some(RoomEvent::ConnectionStateChanged(ConnectionState::Disconnected | ConnectionState::Reconnecting)) | None => return Err(MediaError::Connect),
                        _ => {},
                    }
                }
                frame = stream.next() => {
                    let frame = frame.ok_or(MediaError::Decode)?;
                    let bgra = decoded_bgra(&frame)?;
                    let pixel: [u8; 4] = bgra.bgra[..4].try_into().map_err(|_| MediaError::Decode)?;
                    // Exclude SDK-generated black keepalive frames from our proof.
                    if !captured && pixel[..3].iter().all(|value| *value < 15) { continue; }
                    if let Some(target) = target { target.frames.send(crate::capture::FrameEvent::Frame(std::sync::Arc::new(bgra.clone()))); }
                    report.decoded_frames += 1;
                    report.received_width = bgra.width;
                    report.received_height = bgra.height;
                    report.last_pixel_bgra = pixel;
                }
            }
        }
        report.duration_ms = started.elapsed().as_millis() as u64;
        stream.close();
        if report.submitted_frames == 0 || report.decoded_frames == 0 { return Err(MediaError::NoFrames); }
        let local_stats = timeout(Duration::from_secs(3), track.get_stats()).await.map_err(|_| MediaError::Stats)?.map_err(|_| MediaError::Stats)?;
        let remote_stats = timeout(Duration::from_secs(3), remote.get_stats()).await.map_err(|_| MediaError::Stats)?.map_err(|_| MediaError::Stats)?;
        report.publisher_transports = crate::transport::selected_transports(&local_stats);
        report.viewer_transports = crate::transport::selected_transports(&remote_stats);
        let mut codec_id = None;
        for stat in &local_stats {
            if let RtcStats::OutboundRtp(out) = stat {
                codec_id = Some(out.stream.codec_id.as_str());
                report.encoder = nonempty(&out.outbound.encoder_implementation);
            }
        }
        for stat in &local_stats {
            if let RtcStats::Codec(codec) = stat
                && Some(codec.rtc.id.as_str()) == codec_id { report.codec = nonempty(&codec.codec.mime_type); }
        }
        for stat in &remote_stats {
            if let RtcStats::InboundRtp(input) = stat { report.decoder = nonempty(&input.inbound.decoder_implementation); }
        }
        Ok(report)
    }.await;
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

fn nonempty(value: &str) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value.to_owned())
    }
}

pub fn bgra_to_i420(frame: &BgraFrame) -> Result<I420Buffer, MediaError> {
    let width = frame.width;
    let height = frame.height;
    let size = (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(MediaError::InvalidInput)?;
    if width == 0 || height == 0 || width > 3840 || height > 2160 || size != frame.bgra.len() {
        return Err(MediaError::InvalidInput);
    }
    let mut buffer = I420Buffer::new(width, height);
    let (sy, su, sv) = buffer.strides();
    let (y, u, v) = buffer.data_mut();
    argb_to_i420(
        &frame.bgra,
        width * 4,
        y,
        sy,
        u,
        su,
        v,
        sv,
        width as i32,
        height as i32,
    );
    Ok(buffer)
}

pub fn decoded_bgra(
    frame: &livekit::webrtc::video_frame::BoxVideoFrame,
) -> Result<BgraFrame, MediaError> {
    let width = frame.buffer.width();
    let height = frame.buffer.height();
    if width == 0 || height == 0 || width > 3840 || height > 2160 {
        return Err(MediaError::Decode);
    }
    let mut bgra = vec![0u8; width as usize * height as usize * 4];
    frame.buffer.to_argb(
        VideoFormatType::ARGB,
        &mut bgra,
        width * 4,
        width as i32,
        height as i32,
    );
    Ok(BgraFrame {
        width,
        height,
        bgra: bgra.into(),
    })
}

pub struct PreviewTarget {
    pub frames: crate::capture::FrameSender,
    pub generation: std::sync::Arc<std::sync::atomic::AtomicU64>,
    pub session: u64,
}
pub async fn run_loopback_preview(
    url: &str,
    publisher_token: &str,
    viewer_token: &str,
    duration: Duration,
    target: PreviewTarget,
) -> Result<LoopbackReport, MediaError> {
    run_internal(
        url,
        publisher_token,
        viewer_token,
        duration,
        Some(target),
        None,
    )
    .await
}

impl PreviewTarget {
    fn cancelled(&self) -> bool {
        self.generation.load(std::sync::atomic::Ordering::Acquire) != self.session
    }
}
async fn wait_cancelled(target: Option<&PreviewTarget>) {
    match target {
        None => std::future::pending::<()>().await,
        Some(target) => loop {
            if target.cancelled() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        },
    }
}

/// Relay only frames provided by an explicitly selected native capture stream.
pub async fn run_capture_preview(
    url: &str,
    publisher_token: &str,
    viewer_token: &str,
    duration: Duration,
    input: tokio::sync::watch::Receiver<crate::capture::FrameEvent>,
    target: PreviewTarget,
) -> Result<LoopbackReport, MediaError> {
    run_internal(
        url,
        publisher_token,
        viewer_token,
        duration,
        Some(target),
        Some(input),
    )
    .await
}

async fn next_capture_frame(
    input: &mut Option<tokio::sync::watch::Receiver<crate::capture::FrameEvent>>,
) -> Result<std::sync::Arc<BgraFrame>, MediaError> {
    use crate::capture::FrameEvent;
    let Some(input) = input else {
        return std::future::pending().await;
    };
    loop {
        input.changed().await.map_err(|_| MediaError::Capture)?;
        let event = input.borrow_and_update().clone();
        match event {
            FrameEvent::Frame(frame) => return Ok(frame),
            FrameEvent::Pending => {}
            FrameEvent::Error(_) | FrameEvent::Finished(_) => return Err(MediaError::Capture),
        }
    }
}
