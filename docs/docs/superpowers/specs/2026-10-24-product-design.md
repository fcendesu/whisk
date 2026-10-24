# Whisk v1 — Product Design

**Status:** Based on the 2026-10-24 accepted PRD scope; implementation not started.
**Authority:** `../../prd.md` (complete functional requirements and acceptance details).

## Intent and rationale

Whisk is a fast, native communication application for friends and gaming groups that they can deploy themselves. It offers reliable text, voice, webcams and screen/game streaming without asking operators to adopt a central SaaS account. The app is inspired by Discord and MicUp but is not presented as a complete Discord replacement.

## Naming and visual identity

The selected working name is **Whisk**, inspired by using a kitchen whisk or wooden spatula as a pretend microphone. The brand should read as friendly, communal, and a little playful while the product stays performant and serious about self-hosting. **Whisk Desktop**, **Whisk Server**, and **Whisk Media** are documentation names; **Whisk CLI** is future-only. A subtle utensil-as-microphone logo and *“Your space. Your voice.”* are **ideas, not signed-off design requirements**. Check domain/trademark availability before public release. See `../../adr/0004-whisk-brand-identity.md`.

## Goals

- One Docker-ready backend = one community in v1; easy setup for a small group.
- One Rust+GPUI client navigates multiple separate instances through a Discord-like instance sidebar.
- Independent identity, permissions, messages, storage and notification state per instance.
- Rich communication: channels, DMs, voice, webcam, shareable and selectable simultaneous streams, files, roles and moderation.
- Efficient local rendering, codec pipeline and reasonable idle usage based on measurements.

## Non-goals / anti-goals

No required central login or cross-instance federation. No bot marketplace, recording, mobile/web client or arbitrary large broadcasts in v1. Avoid forcing Kubernetes, external cloud object storage or unrelated containers on the default Docker deployment. Do not label an unverified codec path as 1440p60 supported. Do not conflate DTLS/SRTP with user-to-user end-to-end encryption.

## Approaches and tradeoffs

1. **Chosen:** GPUI native client + Rust/Axum backend + self-hosted LiveKit media + local/S3 storage abstraction. Favors app efficiency and development pace; media backend is Go with native C++ SDK internals.
2. All-Rust SFU/WebRTC from scratch: maximum control but high protocol/security/codec maintenance and slower release; rejected for v1.
3. Tauri/Electron frontend: broader prebuilt UI ecosystem but conflicts with native GPUI goal; rejected.

## Architectural boundaries

See `../../architecture.md`, `../../database.md`, `../../api.md`, `../../events.md`, `../../permissions.md`, `../../deployment.md` and the six more focused specs in this directory. Instance identity and permission checks cannot be bypassed by local UI, stale session state or direct object paths.

## User journeys

Admin deploys Compose → bootstraps one owner/community → invites user → user adds server URL in desktop client → instance appears in unified rail → user sends messages and DMs → user joins voice → publishes webcam/screen stream → another viewer selects between simultaneous streams → moderator controls access and audits changes. Recovery covers disconnects, expired invitations, oversized attachments, camera/capture permission revocation and LiveKit rejoin limitation.

## Sign-off gates

M0 media feasibility is **not yet proven**; M1–M7 plans are proposed execution routes. The PRD locked product features; fine-grained overwrite semantics, codec matrices, version pins and final open-source license remain open ADRs. Record meaningful changes before implementing.
