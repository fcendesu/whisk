# Whisk

Native, self-hosted communication for friends and gaming communities.

Whisk's product baseline is Rust + GPUI desktop, Rust/Axum server, and a
self-hosted LiveKit SFU. One desktop client connects to multiple independent
instances; each instance hosts one community in v1.

## Current build

Implementation has started with **M0 Task 1: native LiveKit connection probe**.
The Rust workspace contains `media-spike`, a disposable command-line diagnostic
for the media feasibility gate. The desktop application and backend are pending.

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
cargo test --workspace --locked
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Live SFU tests are explicit opt-in tests; follow the probe README to run them.
