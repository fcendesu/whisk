# Whisk — ADR-0002: LiveKit SFU and portable storage

**Status:** Media selected, storage design approved in concept; implementation requires feasibility/conformance tests.

**Context:** Native Rust/GPUI UI should be lightweight and high quality, while hosting remains operationally simple. Fully self-implemented WebRTC/SFU would add major development and maintenance costs.

**Decision:** Use LiveKit server (Go) plus the Rust SDK (and native WebRTC internals) for voice/webcam/screenshare. Use Rust/Axum for chat/auth. Local private filesystem storage is the default; isolate an S3-compatible adapter suitable for RustFS, Cloudflare R2 and AWS S3. Compose deploys a single backend instance with optional RustFS profile.

**Alternatives:** Custom pure Rust SFU (more control, higher effort); mandatory RustFS container (more services for very small operators); provider-specific R2 storage only (portability/hosting lock-in).

**Consequences:** M0 must prove 1440p60 capability and GPUI decoder rendering; self-hosted LiveKit stale-token limitation needs short grants, removal calls and tests; S3 migration copies and checksum-verifies rather than simply updating an endpoint.
