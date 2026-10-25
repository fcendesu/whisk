use media_spike::{
    loopback::MediaError,
    microphone::{AudioInput, run_audio_probe},
};
use std::{
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
};
#[tokio::test]
async fn invalid_microphone_probe_never_opens_a_device() {
    for duration in [Duration::ZERO, Duration::from_secs(31)] {
        let result = run_audio_probe(
            "",
            "",
            "",
            duration,
            AudioInput::Microphone,
            Arc::new(AtomicBool::new(false)),
        )
        .await;
        assert!(
            matches!(result, Err(MediaError::InvalidInput)),
            "{result:?}"
        );
    }
}
#[tokio::test]
async fn pre_cancelled_microphone_probe_never_opens_a_device() {
    let result = run_audio_probe(
        "",
        "",
        "",
        Duration::from_secs(10),
        AudioInput::Microphone,
        Arc::new(AtomicBool::new(true)),
    )
    .await;
    assert!(matches!(result, Err(MediaError::Cancelled)), "{result:?}");
}
#[tokio::test]
#[ignore = "requires local SFU fixture; generated audio only, no physical devices"]
async fn generated_opus_crosses_the_same_pipeline_without_claiming_microphone_access() {
    let _lock = LIVE_AUDIO_LOCK.lock().await;
    let report = run_audio_probe(
        &std::env::var("LIVEKIT_URL").unwrap(),
        &std::env::var("LIVEKIT_PUBLISH_TOKEN").unwrap(),
        &std::env::var("LIVEKIT_TOKEN").unwrap(),
        Duration::from_secs(2),
        AudioInput::Generated,
        Arc::new(AtomicBool::new(false)),
    )
    .await
    .expect("audio pipeline failed");
    assert!(report.decoded_frames >= 30, "{report:?}");
    assert!(report.decoded_samples >= 24000, "{report:?}");
    assert!(
        report.non_silent_frames >= 10 && report.peak_sample > 1000,
        "{report:?}"
    );
    assert_eq!(report.codec.as_deref(), Some("audio/opus"));
    assert!(!report.microphone_verified && !report.playback_verified);
    assert!(!report.playback_started && report.user_heard_tone.is_none());
    assert!(matches!(report.audio_source, AudioInput::Generated));
    assert!(!report.publisher_transports.is_empty() && !report.viewer_transports.is_empty());
    let json = serde_json::to_string(&report).unwrap();
    for forbidden in ["device_id", "device_name", "token", "pcm", "credential"] {
        assert!(!json.contains(forbidden));
    }
}

static LIVE_AUDIO_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
#[tokio::test]
#[ignore = "requires local SFU; generated audio cancellation only"]
async fn stopped_audio_probe_cancels_before_the_requested_end() {
    use std::sync::atomic::Ordering;
    let _lock = LIVE_AUDIO_LOCK.lock().await;
    let cancelled = Arc::new(AtomicBool::new(false));
    let stop = cancelled.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(700)).await;
        stop.store(true, Ordering::Release);
    });
    let result = tokio::time::timeout(
        Duration::from_secs(20),
        run_audio_probe(
            &std::env::var("LIVEKIT_URL").unwrap(),
            &std::env::var("LIVEKIT_PUBLISH_TOKEN").unwrap(),
            &std::env::var("LIVEKIT_TOKEN").unwrap(),
            Duration::from_secs(30),
            AudioInput::Generated,
            cancelled,
        ),
    )
    .await
    .expect("audio stop deadline exceeded");
    assert!(matches!(result, Err(MediaError::Cancelled)), "{result:?}");
}
