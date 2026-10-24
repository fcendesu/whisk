# Whisk — Superpowers Documentation Pack

**Status:** v1 product baseline; **implementation has not begun**. 2026-10-24.

Whisk is a native desktop communication app for gaming communities and friend groups, inspired by Discord and MicUp but **not marketed as a Discord replacement**. The desktop client is Rust + GPUI; the self-hosted backend is Rust + Axum; LiveKit supplies voice/video transport. v1 supports one community per backend instance, while a single desktop client connects to many independent instances in one Discord-like sidebar.

**Name rationale:** *Whisk* references the playful habit of holding a kitchen whisk or wooden spatula like a microphone when talking. It is the user-selected working name; trademark and domain availability remain unverified. A whisk-shaped microphone mark and “Your space. Your voice.” are **proposed creative directions**, not finalized assets.

## Start here

1. Read [`docs/README.md`](docs/README.md) for the documentation map.
2. Read [`docs/prd.md`](docs/prd.md) for the complete baseline PRD and locked decisions.
3. Read [`docs/superpowers/specs/2026-10-24-product-design.md`](docs/superpowers/specs/2026-10-24-product-design.md) and the feature-specific design documents.
4. **Before coding**, review the M0 feasibility plan: [`docs/superpowers/plans/2026-10-24-m0-media-feasibility.md`](docs/superpowers/plans/2026-10-24-m0-media-feasibility.md).
5. Follow the milestone plans sequentially, adapting proposed paths to the actual repository after M0; do not claim a requirement passes until measured.

## Superpowers integration

This pack **follows the `obra/superpowers` documentation convention**: design specs in `docs/superpowers/specs/YYYY-MM-DD-...-design.md` and execution plans in `docs/superpowers/plans/YYYY-MM-DD-....md`. Each plan uses the Superpowers agentic-worker header, TDD loops, and checkable review gates. **The Superpowers plugin/skills themselves are not bundled or installed by this ZIP.** Install the official framework separately for your coding agent: <https://github.com/obra/superpowers>. Use its `brainstorming`, `writing-plans`, `using-git-worktrees`, `test-driven-development`, and execution/review skills as appropriate.

The PRD contains explicitly open engineering decisions. Plan paths, signatures, tests and proposed solutions are a **starting contract for review**, not claims that code exists.

## Project naming

- **Whisk Desktop:** native Rust + GPUI client.
- **Whisk Server:** Docker-ready Rust/Axum backend.
- **Whisk Media:** self-hosted LiveKit integration (the LiveKit server is Go).
- **Whisk CLI:** possible later; not in v1.
- See [`docs/adr/0004-whisk-brand-identity.md`](docs/adr/0004-whisk-brand-identity.md) for naming status and release gates.

## Locked product choices

- Windows, macOS, Linux (X11 and Wayland) native desktop releases.
- Invite-only default registration; optional operator-enabled public registration.
- Multi-instance unified sidebar, instance-local identity/caches, one active voice channel per app.
- Text channels, DMs, voice, webcam calls within voice rooms, multiple simultaneous screen streams with selectable viewer focus, attachments, advanced roles/moderation.
- 1080p60/1440p60 are **verified hardware capability targets**, not unconditional guarantees.
- Docker Compose self-hosting; local private storage by default; optional S3-compatible backend (RustFS/R2/AWS S3).
- 25 MiB default attachment cap; project license undecided until audit.

## What's not in the ZIP

No application implementation, binaries, installed Superpowers skills, deployable images, or benchmark results. All checkboxes in plans are intentionally unchecked.
