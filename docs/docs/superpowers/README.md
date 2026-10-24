# Whisk — Superpowers design → plans → implementation

The project's current product discussion is represented by `../prd.md` and the dated design specs. These are source documents for the corresponding milestone plans, not evidence that the code has been written.

## Sequence

| Phase | Design | Plan |
|---|---|---|
| M0 | [`media-design`](specs/2026-10-24-media-design.md) | [`m0-media-feasibility`](plans/2026-10-24-m0-media-feasibility.md) |
| M1 | [`platform-design`](specs/2026-10-24-platform-design.md) | [`m1-foundation`](plans/2026-10-24-m1-foundation.md) |
| M2 | [`identity-client-design`](specs/2026-10-24-identity-client-design.md) | [`m2-identity-client`](plans/2026-10-24-m2-identity-client.md) |
| M3 | [`messaging-storage-design`](specs/2026-10-24-messaging-storage-design.md) | [`m3-messaging-storage`](plans/2026-10-24-m3-messaging-storage.md) |
| M4 | [`media-design`](specs/2026-10-24-media-design.md) | [`m4-voice`](plans/2026-10-24-m4-voice.md) |
| M5 | [`media-design`](specs/2026-10-24-media-design.md) | [`m5-streaming-webcam`](plans/2026-10-24-m5-streaming-webcam.md) |
| M6 | [`permissions-design`](specs/2026-10-24-permissions-design.md) | [`m6-permissions-moderation`](plans/2026-10-24-m6-permissions-moderation.md) |
| M7 | [`platform-design`](specs/2026-10-24-platform-design.md) | [`m7-release-hardening`](plans/2026-10-24-m7-release-hardening.md) |

## Required workflow

1. Install Superpowers separately in your development agent; refer to the upstream docs at <https://github.com/obra/superpowers>.
2. Review the feature design before running its plan. Record any amendments explicitly in an ADR; seek approval for user-visible changes.
3. Work from a git worktree; run one plan task at a time, writing the failing test before implementation.
4. Run the test commands, check real output and make small commits. Untested manual media spikes must have reproducible benchmark logs.
5. After each milestone, verify acceptance and update hardware/network compatibility matrices.

All documented paths under `apps/`, `crates/`, `services/` or `deploy/` describe **intended future files**, not current contents. Their exact names can be revised before scaffolding as long as interface contracts are updated together.
