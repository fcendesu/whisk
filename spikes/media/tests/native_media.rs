use media_spike::loopback::{MediaError, run_loopback};
use std::{env, time::Duration};
#[tokio::test]
async fn invalid_media_run_is_rejected_before_connecting() {
    for duration in [Duration::ZERO, Duration::from_secs(61)] {
        assert!(matches!(
            run_loopback("bad-url", "", "", duration).await,
            Err(MediaError::InvalidInput)
        ));
    }
}
#[tokio::test]
#[ignore = "requires loopback SFU with distinct publish and view tokens"]
async fn native_h264_frames_cross_the_sfu_and_decode_with_correct_colors() {
    let _lock = MEDIA_TEST_LOCK.lock().await;
    let url = env::var("LIVEKIT_URL").expect("set LIVEKIT_URL");
    let publisher = env::var("LIVEKIT_PUBLISH_TOKEN").expect("set LIVEKIT_PUBLISH_TOKEN");
    let viewer = env::var("LIVEKIT_TOKEN").expect("set LIVEKIT_TOKEN");
    let report = tokio::time::timeout(
        Duration::from_secs(35),
        run_loopback(&url, &publisher, &viewer, Duration::from_secs(3)),
    )
    .await
    .expect("media deadline exceeded")
    .expect("native media probe failed");
    assert!(report.submitted_frames >= 30, "{report:?}");
    assert!(report.decoded_frames >= 10, "{report:?}");
    assert_eq!((report.received_width, report.received_height), (1280, 720));
    assert_eq!(report.codec.as_deref(), Some("video/H264"));
    let [b, g, r, _] = report.last_pixel_bgra;
    assert!(
        r > g && g > b && r > 180 && b < 80,
        "BGRA channels reversed or blank: {report:?}"
    );
    assert!(!report.high_quality_certified);
}

#[tokio::test]
async fn cancelled_preview_does_not_connect_or_publish() {
    use media_spike::{
        capture::frame_channel,
        loopback::{PreviewTarget, run_loopback_preview},
    };
    use std::sync::{Arc, atomic::AtomicU64};
    let (frames, _) = frame_channel();
    let target = PreviewTarget {
        frames,
        generation: Arc::new(AtomicU64::new(2)),
        session: 1,
    };
    assert!(matches!(
        run_loopback_preview(
            "invalid",
            "publisher",
            "viewer",
            Duration::from_secs(1),
            target
        )
        .await,
        Err(MediaError::Cancelled)
    ));
}
#[tokio::test]
#[ignore = "requires loopback SFU with distinct publish and view tokens"]
async fn decoded_frames_are_delivered_to_preview() {
    let _lock = MEDIA_TEST_LOCK.lock().await;
    use media_spike::{
        capture::{FrameEvent, frame_channel},
        loopback::{PreviewTarget, run_loopback_preview},
    };
    use std::sync::{Arc, atomic::AtomicU64};
    let (frames, receiver) = frame_channel();
    let target = PreviewTarget {
        frames,
        generation: Arc::new(AtomicU64::new(1)),
        session: 1,
    };
    run_loopback_preview(
        &env::var("LIVEKIT_URL").unwrap(),
        &env::var("LIVEKIT_PUBLISH_TOKEN").unwrap(),
        &env::var("LIVEKIT_TOKEN").unwrap(),
        Duration::from_secs(2),
        target,
    )
    .await
    .unwrap();
    let last = receiver.borrow();
    let FrameEvent::Frame(frame) = &*last else {
        panic!("decoded preview never received a frame: {last:?}");
    };
    assert_eq!((frame.width, frame.height), (1280, 720));
    assert!(frame.bgra[2] > 180 && frame.bgra[0] < 80);
}

static MEDIA_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[tokio::test]
#[ignore = "requires loopback SFU and LIVEKIT_NOISE_TOKEN with publish grant"]
async fn unrelated_room_video_is_not_mistaken_for_this_publish() {
    use livekit::{
        options::{TrackPublishOptions, VideoCodec},
        prelude::*,
        webrtc::video_source::{RtcVideoSource, VideoResolution, native::NativeVideoSource},
    };
    let _lock = MEDIA_TEST_LOCK.lock().await;
    let url = env::var("LIVEKIT_URL").unwrap();
    let (noise, _events) = Room::connect(
        &url,
        &env::var("LIVEKIT_NOISE_TOKEN").expect("set LIVEKIT_NOISE_TOKEN"),
        RoomOptions::default(),
    )
    .await
    .unwrap();
    let source = NativeVideoSource::new(
        VideoResolution {
            width: 1280,
            height: 720,
        },
        true,
    );
    let track = LocalVideoTrack::create_video_track(
        "unrelated-black-source",
        RtcVideoSource::Native(source),
    );
    noise
        .local_participant()
        .publish_track(
            LocalTrack::Video(track),
            TrackPublishOptions {
                source: TrackSource::Screenshare,
                video_codec: VideoCodec::H264,
                simulcast: false,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    let result = run_loopback(
        &url,
        &env::var("LIVEKIT_PUBLISH_TOKEN").unwrap(),
        &env::var("LIVEKIT_TOKEN").unwrap(),
        Duration::from_secs(3),
    )
    .await;
    noise.close().await.unwrap();
    let report = result.expect("unrelated publisher must not hijack this probe");
    assert!(report.decoded_frames >= 10);
    assert!(report.last_pixel_bgra[2] > 180 && report.last_pixel_bgra[0] < 80);
}

#[tokio::test]
#[ignore = "requires loopback SFU with distinct publish and view tokens"]
async fn stopped_preview_cancels_a_running_media_session() {
    use media_spike::{
        capture::frame_channel,
        loopback::{PreviewTarget, run_loopback_preview},
    };
    use std::sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    };
    let _lock = MEDIA_TEST_LOCK.lock().await;
    let (frames, mut receiver) = frame_channel();
    let generation = Arc::new(AtomicU64::new(1));
    let target = PreviewTarget {
        frames,
        generation: generation.clone(),
        session: 1,
    };
    let url = env::var("LIVEKIT_URL").unwrap();
    let publisher = env::var("LIVEKIT_PUBLISH_TOKEN").unwrap();
    let viewer = env::var("LIVEKIT_TOKEN").unwrap();
    let task = tokio::spawn(async move {
        run_loopback_preview(&url, &publisher, &viewer, Duration::from_secs(30), target).await
    });
    tokio::time::timeout(Duration::from_secs(15), receiver.changed())
        .await
        .expect("no decoded frame")
        .unwrap();
    generation.store(2, Ordering::Release);
    let result = tokio::time::timeout(Duration::from_secs(8), task)
        .await
        .expect("cancel did not close session")
        .unwrap();
    assert!(matches!(result, Err(MediaError::Cancelled)), "{result:?}");
}
