//! Run explicitly against the loopback SFU fixture; never silently skip missing config.
use media_spike::room::{FailureKind, ProbeError, connect_probe};
use std::{env, time::Duration};

#[tokio::test]
#[ignore = "requires self-hosted LiveKit fixture and short-lived LIVEKIT_TOKEN"]
async fn valid_token_joins_and_leaves_self_hosted_room() {
    let _lock = ROOM_TEST_LOCK.lock().await;
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

static ROOM_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[tokio::test]
#[ignore = "requires loopback SFU fixture with no TURN service configured"]
async fn relay_required_without_turn_cannot_silently_use_the_direct_udp_path() {
    use media_spike::room::{IcePolicy, connect_probe_with_policy};
    let _lock = ROOM_TEST_LOCK.lock().await;
    let result = tokio::time::timeout(
        Duration::from_secs(10),
        connect_probe_with_policy(
            &env::var("LIVEKIT_URL").expect("set LIVEKIT_URL"),
            &env::var("LIVEKIT_TOKEN").expect("set LIVEKIT_TOKEN"),
            Duration::from_secs(3),
            IcePolicy::RelayOnly,
        ),
    )
    .await
    .expect("relay diagnostic exceeded its deadline");
    match result {
        Ok(room) => {
            let _ = room.close().await;
            panic!("relay-only probe silently used a non-relay route");
        }
        Err(error) => assert!(
            matches!(
                error,
                ProbeError::Connect {
                    kind: FailureKind::Timeout | FailureKind::Transport
                }
            ),
            "{error:?}"
        ),
    }
}

#[tokio::test]
#[ignore = "requires isolated signaling-only SFU fixture and LIVEKIT_UNREACHABLE_MEDIA_URL"]
async fn reachable_signaling_with_unreachable_media_fails_with_a_bounded_diagnostic() {
    use media_spike::room::connect_probe_with_timeout;
    let _lock = ROOM_TEST_LOCK.lock().await;
    let result = tokio::time::timeout(
        Duration::from_secs(10),
        connect_probe_with_timeout(
            &env::var("LIVEKIT_UNREACHABLE_MEDIA_URL").expect("set isolated fixture URL"),
            &env::var("LIVEKIT_TOKEN").expect("set LIVEKIT_TOKEN"),
            Duration::from_secs(3),
        ),
    )
    .await
    .expect("media-path failure exceeded deadline");
    match result {
        Ok(room) => {
            let _ = room.close().await;
            panic!("signaling-only fixture unexpectedly had a working media path");
        }
        Err(error) => assert!(
            matches!(
                error,
                ProbeError::Connect {
                    kind: FailureKind::Timeout | FailureKind::Transport
                }
            ),
            "{error:?}"
        ),
    }
}
