# Agent instructions — Whisk

**Product name:** Whisk (working name chosen by the owner). Use Whisk consistently in documentation, native app display labels and deployment instructions; do not assume branding/trademark clearance or silently add a final icon/tagline.

Read `docs/prd.md` and `docs/superpowers/README.md` before changing implementation decisions. The current ZIP is documentation-only; do not assume a Rust workspace exists yet.

When the official `obra/superpowers` skills are installed in your coding environment:
- Use brainstorming/design review for changes to product scope or unapproved architecture.
- Use writing-plans for any scope not already covered by an approved plan.
- Develop directly in the primary repository checkout on `main`, as requested by the owner. Use an isolated git worktree only when the owner explicitly requests one.
- Apply test-driven development for behavior-changing tasks; verify the red test genuinely fails first.
- Request/review spec compliance and code quality before merging; verify claims with current command output.
- Respect per-task `Files`, `Interfaces`, `Run` and `Expected` blocks in `docs/superpowers/plans`.

Project invariants: client multi-instance separation; one community/instance in v1; instance-qualified cache keys and protocol cursors; server-side permissions on REST/WS/media/storage; private attachment objects; no unverified 1440p60 claims; no default end-to-end encryption claims; Docker Compose single-node support. Never weaken security to satisfy an integration test.

Tech baseline: Rust, Tokio, Axum, SQLx + SQLite (WAL), GPUI, LiveKit Rust SDK + separate LiveKit service, local filesystem/S3 object adapters, OpenAPI 3.1 + documented WebSocket events. LiveKit's server is Go; the native Rust SDK includes C++ WebRTC pieces. Pin and audit dependencies when selecting them.

M0 is a spike and may produce throwaway code. On M0 completion, capture actual hardware/OS/codec/GPUI integration results before finalizing M1+ implementation details. The later plans contain **proposed** signatures and file paths that should be reconciled against the actual workspace, not blindly applied.
