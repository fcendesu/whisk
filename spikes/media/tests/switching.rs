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
