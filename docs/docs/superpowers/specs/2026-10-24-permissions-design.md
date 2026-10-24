# Whisk — Roles, Authorization and Moderation — Design

**Scope:** custom role grants, hierarchy, channel overwrites, bans/timeouts, audit logs. References: `../../permissions.md`, `../../database.md`.

## Central evaluator

Implement one pure authorization function; depend on it in HTTP, gateway filtering/backfill, attachment GET/PUT and media token issuance. Never accept client's requested permissions, room names or storage keys as authority. Default role templates are Owner, Administrator, Moderator and Member, plus custom roles and hierarchy. Owner is protected; moderators can act only on lower-ranked subjects.

## Explicit policy gaps

The product baseline left conflict resolution for multiple channel roles and optional member-target overwrites open. `../../permissions.md` suggests combining role denies and allows with allow-winning collisions, then applying member overwrite last. This is **proposed**, not user-approved. Decide before creating M6 migrations or hardcoding permission bit formats.

## Moderation lifecycle

Kick/ban/timeout and message management generate append-only audit entries; active sessions/gateway resources are invalidated where relevant. Media moderator actions invoke LiveKit server APIs, preventing fresh tokens but not falsely claiming old JWTs cannot reconnect until expiry.

## Testing

Table-driven permission checks over default/custom roles, visibility, missing membership, disabled accounts, object downloads, channel denial, role escalation, concurrent timeout and old tokens. Negative tests must exercise direct API calls, not merely hidden UI controls.
