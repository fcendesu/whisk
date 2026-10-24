use livekit::webrtc::stats::RtcStats;
use media_spike::transport::selected_transports;

#[test]
fn diagnostics_follow_the_selected_pair_and_omit_addresses_and_credentials() {
    let stats: Vec<RtcStats> = serde_json::from_value(serde_json::json!([
        {"type":"candidate-pair","id":"stale","state":"succeeded","nominated":true,"localCandidateId":"wrong","remoteCandidateId":"wrong","bytesSent":999},
        {"type":"transport","id":"transport","selectedCandidatePairId":"active"},
        {"type":"candidate-pair","id":"active","state":"succeeded","nominated":true,"localCandidateId":"local","remoteCandidateId":"remote","bytesSent":1234,"bytesReceived":4321,"responsesReceived":2,"currentRoundTripTime":0.125},
        {"type":"local-candidate","id":"local","protocol":"udp","candidateType":"relay","address":"192.0.2.12","url":"turn:private-secret","usernameFragment":"private-fragment"},
        {"type":"remote-candidate","id":"remote","protocol":"udp","candidateType":"host","address":"192.0.2.22"}
    ])).unwrap();
    let reports = selected_transports(&stats);
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].protocol, Some("udp"));
    assert_eq!(reports[0].local_candidate_type, Some("relay"));
    assert_eq!(reports[0].state, Some("succeeded"));
    assert_eq!(reports[0].rtt_ms, Some(125.0));
    assert_eq!(
        (reports[0].bytes_sent, reports[0].bytes_received),
        (1234, 4321)
    );
    let json = serde_json::to_string(&reports).unwrap();
    for private in ["192.0.2", "private-secret", "private-fragment"] {
        assert!(!json.contains(private));
    }
}

#[test]
fn absent_candidate_data_is_unknown_instead_of_a_udp_or_turn_success() {
    let stats: Vec<RtcStats> = serde_json::from_value(serde_json::json!([
        {"type":"transport","id":"transport","selectedCandidatePairId":"active"},
        {"type":"candidate-pair","id":"active"}
    ]))
    .unwrap();
    let reports = selected_transports(&stats);
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].protocol, None);
    assert_eq!(reports[0].rtt_ms, None);
    assert_eq!(reports[0].state, None);
    assert!(selected_transports(&[]).is_empty());
}
