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
    assert!(!report.audio_verified && !report.quality_certified);
}
