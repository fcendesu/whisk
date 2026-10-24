# M0 native media probe

This disposable spike implements the connection and local capture/preview slices of the
[M0 feasibility plan](../../docs/docs/superpowers/plans/2026-10-24-m0-media-feasibility.md).
It uses the real native LiveKit SDK, without a browser or cloud account.

## Native preview window

On macOS, run `./scripts/run-media-lab.sh` from the repository root. The script
builds the optional `desktop` feature, creates an ad hoc signed development app
in `target/Whisk Media Lab.app`, and opens it. The icon is the supplied Whisk
asset. This is a media feasibility shell, not the messaging application.

- **Test video surface** renders a local BGRA test pattern through GPUI.
- **Refresh sources** checks existing screen-recording access, then discovers
  displays and titled windows on a background worker.
- Select a source to request a 1280×720, 30 FPS local preview.
- **Stop preview** clears the image and invalidates pending starts. A full command
  queue cannot discard the cancellation signal; the worker also checks generation.

Permission checks never request access automatically. If access is not granted,
the window shows System Settings instructions. Granting it is a manual operator
step; restart the app if macOS requires it. Capture callbacks keep only the latest
frame; the native queue and renderer queue are each bounded to two. BGRA row
padding is removed before CPU upload; retired GPU images are released after the
current window update. There is no zero-copy claim.

Pinned integration: ScreenCaptureKit Rust bindings 11.1.0 and GPUI 0.2.2. GPUI
uses `runtime_shaders` so Command Line Tools can compile without the full Xcode
Metal compiler. `build.rs` resolves Swift compatibility libraries from the active
`swiftc` toolchain and adds its runtime search paths. This development app is not
signed/notarized for distribution and remains dependent on the build toolchain;
release packaging is still M7 work. Native API waits run on a worker and may take
up to the bindings' default 30-second completion deadline.

## Connection probe

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

## Additional native-preview evidence — 2026-10-24

- ScreenCaptureKit and GPUI build/link on macOS arm64 using Command Line Tools.
- Ten deterministic tests pass, including permission gating, padded native rows,
  invalid frame bounds, terminal errors surviving late callbacks, and a slow
  viewer consuming 180 frames with a queue depth of at most two.
- The packaged app opens; clicking the test button renders the GPU surface and
  Stop clears it. Refresh reports this machine's ungranted screen access.
- Native screen pixels remain unverified because capture access is not granted.
- Independent review found an active-window image cleanup issue; cleanup now
  defers until GPUI restores the window. Pattern/Stop cycling was checked manually;
  there is no automated GPU-allocation regression test.
- Current upstream `block` and `proc-macro-error2` versions emit Rust future-
  compatibility warnings. They do not fail current checks; resolve before release.

## Remaining M0 gates

- Task 2: exercise display/window capture and revocation with actual granted OS
  permission; the native adapter, permission-denial path and frame handoff exist.
- Task 3: captured/generated video publishing, measured codec selection/fallback.
- Task 4: connect the pinned, bounded GPUI preview to decoded remote frames.
  The local renderer and GPU pattern have been exercised; remote decode has not.
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
