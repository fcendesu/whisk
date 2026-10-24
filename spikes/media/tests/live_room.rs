//! Run explicitly against the loopback SFU fixture; never silently skip missing config.
use media_spike::room::{FailureKind, ProbeError, connect_probe};
use std::{env, time::Duration};

#[tokio::test]
#[ignore = "requires self-hosted LiveKit fixture and short-lived LIVEKIT_TOKEN"]
async fn valid_token_joins_and_leaves_self_hosted_room() {
    let url = env::var("LIVEKIT_URL").expect("set LIVEKIT_URL");
    let token = env::var("LIVEKIT_TOKEN").expect("set LIVEKIT_TOKEN");
    let mut room = connect_probe(&url, &token).await.expect("join failed");
    room.observe_for(Duration::from_millis(100)).await;
    let report = room.close().await.expect("disconnect failed");
    assert_eq!(report.states.first().unwrap().state, "connected");
    assert_eq!(report.states.last().unwrap().state, "disconnected");
    assert!(!report.media_verified);
    assert!(report.states.len() <= 64);
}

#[tokio::test]
#[ignore = "requires self-hosted LiveKit fixture"]
async fn rejected_token_is_distinct_from_transport_failure() {
    let url = env::var("LIVEKIT_URL").expect("set LIVEKIT_URL");
    let result = connect_probe(&url, "invalid.fixture.token").await;
    assert!(
        matches!(
            result,
            Err(ProbeError::Connect {
                kind: FailureKind::Rejected
            })
        ),
        "{result:?}"
    );
}
