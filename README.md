# Whisk

Native, self-hosted communication for friends and gaming communities.

Whisk's product baseline is Rust + GPUI desktop, Rust/Axum server, and a
self-hosted LiveKit SFU. One desktop client connects to multiple independent
instances; each instance hosts one community in v1.

## Current build

Implementation is in **M0: native media feasibility**. The workspace now has a
real LiveKit connection probe and a runnable **Whisk Media Lab** GPUI window with
the provided icon, source discovery, local screen/window preview, and a GPU test
pattern. Native H.264 publishing, SFU forwarding, and remote-frame decode into
GPUI work with generated frames and explicitly selected captured windows.
Generated-audio continuity, experimental profile attempts, optical frame-age
measurements and selected ICE-path diagnostics are also implemented. Local UDP
TURN media and physical microphone publish/decode have been verified. A separate
**Whisk Audio Lab** provides an explicit, bounded microphone test with speaker
playback off and counter-only reports. The separate **Whisk Speaker Lab** has a
short generated-tone test with user-confirmed physical playback.
The production messaging desktop and backend are pending.

On macOS, build and open the preview from this checkout:

```sh
./scripts/run-media-lab.sh
```

It creates `target/Whisk Media Lab.app`. Capture starts only when you select a
source and remains local until **Publish selected window** is clicked with the
network fixture configured. Screen access is checked without
opening a permission prompt. If macOS denies access, the window explains how to
allow it manually and retry.

- [Run the native probe](spikes/media/README.md)
- [Product requirements](docs/docs/prd.md)
- [Architecture](docs/docs/architecture.md)
- [Milestone plans](docs/docs/superpowers/README.md)
- [Provided icon assets](assets/README.md)

The documentation pack retains its imported `docs/docs/` structure and historical
“implementation pending” baseline. Current implementation evidence and remaining
gates are recorded in the probe README. The existing repository license remains
in place; native dependency redistribution and final product licensing still
require the audit specified by the PRD.

## Development

Rust 1.99.0 is pinned in `rust-toolchain.toml`; Cargo locks the dependency graph.
The first build downloads and compiles LiveKit's native C++ WebRTC components.
macOS needs Apple Command Line Tools and the configured `-ObjC` linker flag.
Other platform builds and media capabilities have not been verified yet.

```sh
cargo test --workspace --all-features --locked
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
```

Live SFU tests are explicit opt-in tests (`./scripts/test-native-media.sh`).
`./scripts/run-media-lab.sh --network` opens the separate network-video test app.
Follow the probe README for setup, measured evidence, and remaining M0 gates.

`./scripts/run-media-lab.sh --capture-network` enables the selected-window
publish/decode test using the local fixture. Selecting a source alone stays local.

`./scripts/run-audio-lab.sh` opens the separate microphone test app. Use
`--reuse-build` with either lab launcher to preserve its existing signed bundle.
Quit the app before relaunching to refresh short-lived fixture credentials.

`./scripts/run-audio-lab.sh --speaker` opens the separate speaker test. It keeps
the microphone off and requires explicit playback feedback before verification.
