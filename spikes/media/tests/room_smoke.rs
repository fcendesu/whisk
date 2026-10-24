use media_spike::room::{ProbeError, connect_probe, connect_probe_with_timeout};
use std::time::Duration;

#[tokio::test]
async fn unreachable_media_endpoint_is_reported() {
    // Retain the bound socket without listening: no other process can steal it.
    let socket = tokio::net::TcpSocket::new_v4().unwrap();
    socket.bind("127.0.0.1:0".parse().unwrap()).unwrap();
    let url = format!("ws://{}", socket.local_addr().unwrap());
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        connect_probe_with_timeout(&url, "fixture-token", Duration::from_millis(300)),
    )
    .await
    .expect("probe must not hang");
    assert!(
        matches!(result, Err(ProbeError::Connect { .. })),
        "{result:?}"
    );
}

#[tokio::test]
async fn stalled_signaling_is_bounded() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    let result = tokio::time::timeout(
        Duration::from_secs(3),
        connect_probe_with_timeout(&url, "fixture-token", Duration::from_millis(100)),
    )
    .await
    .expect("outer deadline exceeded");
    assert!(
        matches!(result, Err(ProbeError::Connect { .. })),
        "{result:?}"
    );
}

#[tokio::test]
async fn invalid_inputs_are_rejected_before_network_io() {
    for (url, token) in [
        ("not a url", "token"),
        ("file:///etc/passwd", "token"),
        ("ws://user:password@localhost:7880", "token"),
        ("ws://localhost:7880?access_token=secret", "token"),
        ("ws://localhost:7880#secret", "token"),
        ("ws://localhost:7880", ""),
        ("ws://localhost:7880", "  "),
    ] {
        assert!(matches!(
            connect_probe(url, token).await,
            Err(ProbeError::InvalidInput)
        ));
    }
}
