//! Allowlisted ICE diagnostics. Never retain candidate addresses, URLs or fragments.
use livekit::webrtc::stats::{IceCandidatePairState, IceCandidateType, RtcStats};
use serde::Serialize;
#[derive(Debug, Serialize)]
pub struct TransportReport {
    pub protocol: Option<&'static str>,
    pub local_candidate_type: Option<&'static str>,
    pub remote_candidate_type: Option<&'static str>,
    pub state: Option<&'static str>,
    pub nominated: bool,
    pub rtt_ms: Option<f64>,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}
pub fn selected_transports(stats: &[RtcStats]) -> Vec<TransportReport> {
    stats
        .iter()
        .filter_map(|stat| {
            let RtcStats::Transport(transport) = stat else {
                return None;
            };
            let pair = stats.iter().find_map(|stat| match stat {
                RtcStats::CandidatePair(pair)
                    if pair.rtc.id == transport.transport.selected_candidate_pair_id
                        && !pair.rtc.id.is_empty() =>
                {
                    Some(&pair.candidate_pair)
                }
                _ => None,
            })?;
            let local = stats.iter().find_map(|stat| match stat {
                RtcStats::LocalCandidate(candidate)
                    if candidate.rtc.id == pair.local_candidate_id
                        && !candidate.rtc.id.is_empty() =>
                {
                    Some(&candidate.local_candidate)
                }
                _ => None,
            });
            let remote = stats.iter().find_map(|stat| match stat {
                RtcStats::RemoteCandidate(candidate)
                    if candidate.rtc.id == pair.remote_candidate_id
                        && !candidate.rtc.id.is_empty() =>
                {
                    Some(&candidate.remote_candidate)
                }
                _ => None,
            });
            Some(TransportReport {
                protocol: local.and_then(|candidate| match candidate.protocol.as_str() {
                    "udp" => Some("udp"),
                    "tcp" => Some("tcp"),
                    _ => None,
                }),
                local_candidate_type: local
                    .and_then(|candidate| candidate.candidate_type)
                    .map(candidate_type),
                remote_candidate_type: remote
                    .and_then(|candidate| candidate.candidate_type)
                    .map(candidate_type),
                state: pair.state.map(|state| match state {
                    IceCandidatePairState::Frozen => "frozen",
                    IceCandidatePairState::Waiting => "waiting",
                    IceCandidatePairState::InProgress => "in_progress",
                    IceCandidatePairState::Failed => "failed",
                    IceCandidatePairState::Succeeded => "succeeded",
                }),
                nominated: pair.nominated,
                rtt_ms: (pair.responses_received > 0
                    && pair.current_round_trip_time.is_finite()
                    && pair.current_round_trip_time >= 0.0)
                    .then_some(pair.current_round_trip_time * 1000.0),
                bytes_sent: pair.bytes_sent,
                bytes_received: pair.bytes_received,
            })
        })
        .collect()
}
fn candidate_type(kind: IceCandidateType) -> &'static str {
    match kind {
        IceCandidateType::Host => "host",
        IceCandidateType::Srflx => "srflx",
        IceCandidateType::Prflx => "prflx",
        IceCandidateType::Relay => "relay",
    }
}
