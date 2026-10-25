use media_spike::{loopback::MediaError, switching::run_switch_probe};
use std::{env, time::Duration};
#[tokio::test]
async fn empty_switch_measurements_are_rejected() {
    assert!(matches!(
        run_switch_probe("", "", "", "", Duration::ZERO).await,
        Err(MediaError::InvalidInput)
    ));
}
#[tokio::test]
#[ignore = "requires local SFU and two distinct publisher tokens plus a viewer token"]
async fn two_publishers_keep_sending_while_viewer_switches_focus() {
    let _lock = SWITCH_TEST_LOCK.lock().await;
    let report = run_switch_probe(
        &env::var("LIVEKIT_URL").unwrap(),
        &env::var("LIVEKIT_PUBLISH_TOKEN").unwrap(),
        &env::var("LIVEKIT_SECOND_PUBLISH_TOKEN").expect("set LIVEKIT_SECOND_PUBLISH_TOKEN"),
        &env::var("LIVEKIT_TOKEN").unwrap(),
        Duration::from_secs(2),
    )
    .await
    .expect("switching probe failed");
    assert!(
        report.submitted_frames[0] >= 90 && report.submitted_frames[1] >= 90,
        "{report:?}"
    );
    assert!(
        report.received_frames[0] >= 20 && report.received_frames[1] >= 20,
        "{report:?}"
    );
    assert_eq!(report.wrong_source_frames, 0);
    assert!(
        report.audio_verified,
        "no audio continuity proof: {report:?}"
    );
    assert!(
        report.audible_audio_frames.iter().all(|count| *count >= 80),
        "{report:?}"
    );
    assert!(report.max_audio_gap_ms < 250, "{report:?}");
    assert!(
        (420.0..460.0).contains(&report.tone_frequency_hz),
        "{report:?}"
    );
    assert!(!report.microphone_verified && !report.quality_certified);
}

static SWITCH_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[tokio::test]
#[ignore = "requires local SFU and two publishers plus viewer; attempts uncertified profiles"]
async fn requested_high_profiles_reach_the_decoder_at_the_requested_dimensions() {
    use media_spike::{codec::VideoProfile, switching::run_profile_probe};
    let _lock = SWITCH_TEST_LOCK.lock().await;
    for (profile, size) in [
        (VideoProfile::P1080F60, [1920, 1080]),
        (VideoProfile::P1440F60, [2560, 1440]),
    ] {
        let report = run_profile_probe(
            &env::var("LIVEKIT_URL").unwrap(),
            &env::var("LIVEKIT_PUBLISH_TOKEN").unwrap(),
            &env::var("LIVEKIT_SECOND_PUBLISH_TOKEN").unwrap(),
            &env::var("LIVEKIT_TOKEN").unwrap(),
            Duration::from_secs(2),
            profile,
        )
        .await
        .expect("profile probe failed");
        assert_eq!(report.received_size, [size, size], "{report:?}");
        assert!(
            report.frame_age_samples >= 20,
            "no optical frame-age proof: {report:?}"
        );
        assert!(
            report.frame_age_p95_ms.is_some_and(|age| age < 1000),
            "{report:?}"
        );
        assert!(
            report.received_frames.iter().all(|count| *count >= 20),
            "{report:?}"
        );
        assert!(report.audio_verified, "{report:?}");
        assert!(!report.quality_certified);
    }
}

#[tokio::test]
async fn pre_cancelled_profile_preview_never_connects() {
    use media_spike::{
        capture::frame_channel, codec::VideoProfile, loopback::PreviewTarget,
        switching::run_profile_preview,
    };
    use std::sync::{Arc, atomic::AtomicU64};
    let (frames, _) = frame_channel();
    let target = PreviewTarget {
        frames,
        generation: Arc::new(AtomicU64::new(2)),
        session: 1,
    };
    let result = run_profile_preview(
        "",
        "",
        "",
        "",
        Duration::from_secs(1),
        VideoProfile::P1080F60,
        target,
    )
    .await;
    assert!(matches!(result, Err(MediaError::Cancelled)), "{result:?}");
}
#[tokio::test]
#[ignore = "requires local SFU and two publishers; preview remains generated video/audio"]
async fn profile_preview_forwards_both_focused_publishers_to_the_ui_channel() {
    use media_spike::{
        capture::{FrameEvent, frame_channel},
        codec::VideoProfile,
        loopback::PreviewTarget,
        switching::run_profile_preview,
    };
    use std::sync::{Arc, atomic::AtomicU64};
    let _lock = SWITCH_TEST_LOCK.lock().await;
    let (frames, mut receiver) = frame_channel();
    let target = PreviewTarget {
        frames,
        generation: Arc::new(AtomicU64::new(1)),
        session: 1,
    };
    let url = env::var("LIVEKIT_URL").unwrap();
    let first = env::var("LIVEKIT_PUBLISH_TOKEN").unwrap();
    let second = env::var("LIVEKIT_SECOND_PUBLISH_TOKEN").unwrap();
    let viewer = env::var("LIVEKIT_TOKEN").unwrap();
    let mut task = tokio::spawn(async move {
        run_profile_preview(
            &url,
            &first,
            &second,
            &viewer,
            Duration::from_secs(1),
            VideoProfile::P1080F60,
            target,
        )
        .await
    });
    let mut colors = [false, false];
    let report = tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            tokio::select! {
             result=&mut task=>break result.unwrap().unwrap(),
             changed=receiver.changed()=>{
              if changed.is_err(){break task.await.unwrap().unwrap();}
              if let FrameEvent::Frame(frame)=&*receiver.borrow_and_update(){
               assert_eq!((frame.width,frame.height),(1920,1080));
               colors[0]|=frame.bgra[2]>180 && frame.bgra[0]<80;
               colors[1]|=frame.bgra[0]>180 && frame.bgra[2]<80;
              }
             }
            }
        }
    })
    .await
    .expect("profile preview deadline exceeded");
    assert_eq!(
        colors,
        [true, true],
        "both focus phases must reach UI: {report:?}"
    );
    assert!(!report.quality_certified);
}

#[tokio::test]
#[ignore = "requires local SFU; generated profile preview cancellation"]
async fn stopping_a_profile_preview_cancels_its_owned_rooms() {
    use media_spike::{
        capture::{FrameEvent, frame_channel},
        codec::VideoProfile,
        loopback::PreviewTarget,
        switching::run_profile_preview,
    };
    use std::sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    };
    let _lock = SWITCH_TEST_LOCK.lock().await;
    let (frames, mut receiver) = frame_channel();
    let generation = Arc::new(AtomicU64::new(1));
    let target = PreviewTarget {
        frames,
        generation: generation.clone(),
        session: 1,
    };
    let url = env::var("LIVEKIT_URL").unwrap();
    let first = env::var("LIVEKIT_PUBLISH_TOKEN").unwrap();
    let second = env::var("LIVEKIT_SECOND_PUBLISH_TOKEN").unwrap();
    let viewer = env::var("LIVEKIT_TOKEN").unwrap();
    let task = tokio::spawn(async move {
        run_profile_preview(
            &url,
            &first,
            &second,
            &viewer,
            Duration::from_secs(3),
            VideoProfile::P720F30,
            target,
        )
        .await
    });
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            receiver.changed().await.unwrap();
            if matches!(&*receiver.borrow_and_update(), FrameEvent::Frame(_)) {
                break;
            }
        }
    })
    .await
    .expect("profile preview did not start");
    generation.store(2, Ordering::Release);
    let result = tokio::time::timeout(Duration::from_secs(20), task)
        .await
        .expect("profile cancellation exceeded cleanup bound")
        .unwrap();
    assert!(matches!(result, Err(MediaError::Cancelled)), "{result:?}");
}
