# M0 native media probe

This disposable spike implements the first connection slice of the
[M0 feasibility plan](../../docs/docs/superpowers/plans/2026-10-24-m0-media-feasibility.md).
It uses the real native LiveKit SDK, without a browser or cloud account.

## Run locally

Requirements: Rust 1.99.0, Apple Command Line Tools on macOS, Python 3, and
running Docker with Compose. macOS arm64 is the only verified build so far.

From the repository root:

```sh
docker compose -f spikes/media/fixture/compose.yaml up -d
export LIVEKIT_URL=ws://127.0.0.1:7880
export LIVEKIT_TOKEN="$(python3 spikes/media/fixture/token.py)"
cargo run --locked -p media-spike
unset LIVEKIT_TOKEN
```

The operator-side Python fixture issues a two-minute JWT with a unique identity,
restricted to joining/subscribing in `whisk-m0-smoke`. It contains the public
`devkey`/`secret` pair used by LiveKit development mode; these are disposable test
credentials. The native probe only receives a room token, never an API secret.
The SFU image is pinned by version and digest. Its host ports bind to loopback,
with the advertised node IP set to `127.0.0.1` for this local-only fixture.
Do not reuse this deployment or its credentials outside local development.
Remote deployments need TLS and proper operator-managed credentials.

Successful output is one JSON report with SDK, OS, architecture, join timing,
bounded connection-state history and `media_verified: false`. The probe observes
room events for two seconds and closes the room. Connect is bounded to ten
seconds with SDK join retries disabled; close is bounded to five seconds.
Failures exit nonzero with a JSON `invalid_input`, `connect`, or `close` error;
connection kinds distinguish timeout, rejected credentials, and transport failure.
Raw SDK errors, tokens and endpoint URLs are excluded from diagnostics.

```sh
# Network-failure and CLI regression tests (no SFU needed)
cargo test --workspace --locked

# Real SFU join/leave and authentication rejection (refresh token first)
export LIVEKIT_URL=ws://127.0.0.1:7880
export LIVEKIT_TOKEN="$(python3 spikes/media/fixture/token.py)"
cargo test --locked -p media-spike --test live_room -- --ignored --nocapture
unset LIVEKIT_TOKEN

# Tear down the dedicated fixture
docker compose -f spikes/media/fixture/compose.yaml down
```

The two live tests are ignored by default and fail if explicitly invoked without
the required environment. Use `--test room_smoke` for the deterministic connection
tests: the original plan's `room_smoke` function filter selects zero tests.

## Evidence — 2026-10-24

Verified locally on macOS arm64 with Rust 1.99.0, LiveKit SDK 0.9.7,
webrtc-sys 0.3.51, and the Docker-hosted LiveKit server 1.13.10:

- Native SDK builds and links using Command Line Tools and `-ObjC`.
- Five deterministic tests pass; their initial stubs failed before implementation.
- Two explicitly invoked live tests pass: real room join/leave and rejected JWT.
- [Recorded CLI report](results/2026-10-24-macos-arm64-connection.json) records a
  connected → disconnected transition (199 ms local join; not a media benchmark).

The SDK's public room events expose aggregate connection state. This probe does
not expose raw signaling/ICE candidate-pair telemetry and does not prove that UDP
or TURN is usable; closed-port tests exercise signaling failures only. A joined
room also does not prove working audio/video.

## Remaining M0 gates

- Task 2: native screen/window discovery, permission-denial handling and capture.
- Task 3: captured/generated video publishing, measured codec selection/fallback.
- Task 4: pinned GPUI integration and bounded decoded-frame rendering.
- Task 5: two concurrent publishers, selectable viewer, ten-minute measurements.
- Verify actual microphone publish/playback and audio continuity while switching
  streams before declaring the milestone complete.
- Record OS/hardware/driver/codec details, frame drops, latency and CPU/GPU use;
  verify UDP/TURN failures, platform capture permissions, and SDK cancellation
  cleanup under repeated failures. Timeout tests only prove caller deadlines.
- Pin GPUI when checking its real renderer APIs. Audit dependency licenses and
  native binaries before distribution; this connection slice is not that audit.

No 1080p60, 1440p60, GPU zero-copy, production deployment, or cross-platform media
support is claimed. M1 and later milestones remain gated on M0 evidence.

Upstream references: [pinned SDK source](https://docs.rs/crate/livekit/0.9.7/source/),
[macOS build requirements](https://github.com/livekit/rust-sdks#macos), and
[server release](https://github.com/livekit/livekit/releases/tag/v1.13.10).
