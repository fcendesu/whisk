use media_spike::{
    loopback::MediaError,
    microphone::{AudioInput, AudioReport, run_audio_probe},
};
use std::{
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
};
fn report() -> AudioReport {
    AudioReport {
        audio_source: AudioInput::SpeakerTest,
        decoded_frames: 100,
        decoded_samples: 48000,
        non_silent_frames: 100,
        peak_sample: 1200,
        duration_ms: 1000,
        codec: Some("audio/opus".into()),
        microphone_verified: false,
        playback_verified: false,
        playback_started: true,
        user_heard_tone: None,
        publisher_transports: Vec::new(),
        viewer_transports: Vec::new(),
    }
}
#[tokio::test]
async fn speaker_duration_is_bounded_before_any_device_access() {
    let result = run_audio_probe(
        "invalid",
        "publisher",
        "viewer",
        Duration::from_secs(4),
        AudioInput::SpeakerTest,
        Arc::new(AtomicBool::new(false)),
    )
    .await;
    assert!(
        matches!(result, Err(MediaError::InvalidInput)),
        "{result:?}"
    );
}
#[tokio::test]
async fn pre_cancelled_speaker_test_never_acquires_a_device() {
    let result = run_audio_probe(
        "invalid",
        "publisher",
        "viewer",
        Duration::from_secs(3),
        AudioInput::SpeakerTest,
        Arc::new(AtomicBool::new(true)),
    )
    .await;
    assert!(matches!(result, Err(MediaError::Cancelled)), "{result:?}");
}
#[test]
fn decode_alone_cannot_certify_playback() {
    let mut cases = vec![report(), report(), report(), report()];
    cases[0].audio_source = AudioInput::Generated;
    cases[1].playback_started = false;
    cases[2].decoded_frames = 0;
    cases[3].non_silent_frames = 0;
    for mut value in cases {
        assert!(matches!(
            value.confirm_playback(true),
            Err(MediaError::InvalidInput)
        ));
        assert!(!value.playback_verified && value.user_heard_tone.is_none());
    }
}
#[test]
fn user_confirmation_and_no_sound_are_distinct_from_unknown() {
    let mut value = report();
    assert!(value.user_heard_tone.is_none() && !value.playback_verified);
    value.confirm_playback(false).unwrap();
    assert_eq!(value.user_heard_tone, Some(false));
    assert!(!value.playback_verified);
    value.confirm_playback(true).unwrap();
    assert_eq!(value.user_heard_tone, Some(true));
    assert!(value.playback_verified);
    assert!(!value.microphone_verified);
}
