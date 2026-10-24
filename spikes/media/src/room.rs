//! Disposable M0 connection probe. This does not certify any media profile.
use livekit::{ConnectionState, Room, RoomError, RoomEvent, RoomOptions, rtc_engine::EngineError};
use livekit_signaling::SignalError;
use serde::Serialize;
use std::{fmt, time::Duration};
use tokio::{
    sync::mpsc,
    time::{Instant, timeout, timeout_at},
};
use url::Url;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const CLOSE_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_STATES: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureKind {
    Timeout,
    Rejected,
    Transport,
}

/// Deliberately excludes SDK error strings, which can contain credentials.
#[derive(Debug, Serialize)]
#[serde(tag = "error", rename_all = "snake_case")]
pub enum ProbeError {
    InvalidInput,
    Connect { kind: FailureKind },
    Close { kind: FailureKind },
}

impl fmt::Display for ProbeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            serde_json::to_string(self).expect("serializable probe error")
        )
    }
}
impl std::error::Error for ProbeError {}

#[derive(Debug, Serialize)]
pub struct StateSample {
    pub elapsed_ms: u64,
    pub state: &'static str,
}

#[derive(Debug, Serialize)]
pub struct ProbeReport {
    pub sdk: &'static str,
    pub platform: &'static str,
    pub arch: &'static str,
    pub join_ms: u64,
    pub states: Vec<StateSample>,
    pub ice_detail: &'static str,
    pub media_verified: bool,
    pub ice_policy: IcePolicy,
}

pub struct ProbeRoom {
    room: Room,
    events: mpsc::UnboundedReceiver<RoomEvent>,
    started: Instant,
    report: ProbeReport,
}

impl fmt::Debug for ProbeRoom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProbeRoom")
            .field("report", &self.report)
            .finish_non_exhaustive()
    }
}

pub async fn connect_probe(url: &str, token: &str) -> Result<ProbeRoom, ProbeError> {
    connect_probe_with_timeout(url, token, CONNECT_TIMEOUT).await
}

pub async fn connect_probe_with_timeout(
    url: &str,
    token: &str,
    deadline: Duration,
) -> Result<ProbeRoom, ProbeError> {
    connect_probe_with_policy(url, token, deadline, IcePolicy::All).await
}

pub async fn connect_probe_with_policy(
    url: &str,
    token: &str,
    deadline: Duration,
    policy: IcePolicy,
) -> Result<ProbeRoom, ProbeError> {
    let endpoint = Url::parse(url).map_err(|_| ProbeError::InvalidInput)?;
    if !matches!(endpoint.scheme(), "ws" | "wss")
        || endpoint.host_str().is_none()
        || !endpoint.username().is_empty()
        || endpoint.password().is_some()
        || endpoint.query().is_some()
        || endpoint.fragment().is_some()
        || token.trim().is_empty()
        || deadline.is_zero()
    {
        return Err(ProbeError::InvalidInput);
    }
    let started = Instant::now();
    let mut options = RoomOptions::default();
    options.join_retries = 0;
    options.connect_timeout = deadline;
    options.rtc_config.ice_transport_type = match policy {
        IcePolicy::All => livekit::webrtc::peer_connection_factory::IceTransportsType::All,
        IcePolicy::RelayOnly => livekit::webrtc::peer_connection_factory::IceTransportsType::Relay,
    };
    let (room, events) = timeout(deadline, Room::connect(endpoint.as_str(), token, options))
        .await
        .map_err(|_| ProbeError::Connect {
            kind: FailureKind::Timeout,
        })?
        .map_err(|error| ProbeError::Connect {
            kind: classify(error),
        })?;
    let report = ProbeReport {
        sdk: livekit::SDK_VERSION,
        platform: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        join_ms: millis(started.elapsed()),
        states: vec![StateSample {
            elapsed_ms: millis(started.elapsed()),
            state: state_name(room.connection_state()),
        }],
        ice_detail: "room events are aggregate; selected candidate pairs are available in media-probe reports",
        media_verified: false,
        ice_policy: policy,
    };
    Ok(ProbeRoom {
        room,
        events,
        started,
        report,
    })
}

impl ProbeRoom {
    /// Consume events for a fixed interval; keep at most 64 connection transitions.
    pub async fn observe_for(&mut self, duration: Duration) {
        let deadline = Instant::now() + duration;
        while let Ok(Some(event)) = timeout_at(deadline, self.events.recv()).await {
            if Instant::now() >= deadline {
                break;
            }
            if let RoomEvent::ConnectionStateChanged(state) = event {
                self.record(state_name(state));
            }
        }
    }

    pub async fn close(mut self) -> Result<ProbeReport, ProbeError> {
        timeout(CLOSE_TIMEOUT, self.room.close())
            .await
            .map_err(|_| ProbeError::Close {
                kind: FailureKind::Timeout,
            })?
            .map_err(|_| ProbeError::Close {
                kind: FailureKind::Transport,
            })?;
        self.record(state_name(self.room.connection_state()));
        Ok(self.report)
    }

    fn record(&mut self, state: &'static str) {
        if self
            .report
            .states
            .last()
            .is_some_and(|last| last.state == state)
        {
            return;
        }
        if self.report.states.len() == MAX_STATES {
            self.report.states.remove(0);
        }
        self.report.states.push(StateSample {
            elapsed_ms: millis(self.started.elapsed()),
            state,
        });
    }
}

fn state_name(state: ConnectionState) -> &'static str {
    match state {
        ConnectionState::Connected => "connected",
        ConnectionState::Disconnected => "disconnected",
        ConnectionState::Reconnecting => "reconnecting",
    }
}

fn millis(duration: Duration) -> u64 {
    duration.as_millis().min(u64::MAX as u128) as u64
}

fn classify(error: RoomError) -> FailureKind {
    match error {
        RoomError::Engine(EngineError::Signal(SignalError::Client(status, _)))
            if matches!(status.as_u16(), 401 | 403) =>
        {
            FailureKind::Rejected
        }
        RoomError::Engine(EngineError::Signal(SignalError::TokenFormat)) => FailureKind::Rejected,
        RoomError::Engine(EngineError::Signal(SignalError::Timeout(_))) => FailureKind::Timeout,
        _ => FailureKind::Transport,
    }
}

impl ProbeRoom {
    pub(crate) fn sdk_room(&self) -> &Room {
        &self.room
    }
    pub(crate) async fn next_event(&mut self) -> Option<RoomEvent> {
        self.events.recv().await
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IcePolicy {
    All,
    RelayOnly,
}
