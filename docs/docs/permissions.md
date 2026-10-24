# Whisk — Permission model and moderation — proposed v1

All decisions are made by one server-side permission evaluator consumed by HTTP, WS replay, attachment authorization, LiveKit token issuance and moderation actions. UI hiding is not an access-control mechanism.

## Permission bits

`VIEW_CHANNEL`, `SEND_MESSAGES`, `READ_HISTORY`, `ATTACH_FILES`, `MANAGE_MESSAGES`, `CREATE_INVITES`, `MANAGE_CHANNELS`, `CONNECT_VOICE`, `SPEAK`, `STREAM_SCREEN`, `USE_CAMERA`, `MUTE_MEMBERS`, `MOVE_MEMBERS`, `MANAGE_MEMBERS`, `TIMEOUT_MEMBERS`, `KICK_MEMBERS`, `BAN_MEMBERS`, `MANAGE_ROLES`, `MANAGE_COMMUNITY`, `VIEW_AUDIT_LOG`.

| Default role | Chat/read, file, voice/stream | Invite | Moderation | Roles/channels/settings |
|---|---|---|---|---|
| Owner | Yes | Yes | Yes | Full protected owner authority |
| Administrator | Yes | Yes | Yes, only below their top role | Yes, subject to owner restrictions |
| Moderator | Yes | Yes | Timeout/kick/ban/mute below own rank | No by default |
| Member | Yes (subject to channel overwrites) | No by default | No | No |

Role templates are configurable; the **owner bypass** and rank constraints are server-enforced invariants.

## Evaluation order — propose to freeze before M6

1. Reject unauthenticated, banned, disabled or nonmember subjects.
2. Apply owner bypass for community permission checks (not a bypass of disabled account or privacy isolation).
3. Start from union of user's role grants.
4. Apply community `@everyone` channel overwrite (deny then allow).
5. Collect applicable role overwrites; combine all denies and all allows; apply `(bits & ~denies) | allows` (thus **role allow wins where both present**, matching the chosen v1 policy). The operation and tests must be pinned before coding.
6. Apply optional member-target overwrite after role overwrites, if supported by final migration (proposal: include it in v1 to avoid undefined behavior).
7. Apply ban/timeout/content-object restrictions at final gate; timeout forbids sends/publishing even if a channel overwrite allows it.

**Needs approval:** exact conflict behavior of multiple role overwrites and individual member overwrites were originally open ADRs; the above is a suggested policy, not a user-approved choice. Mark in ADR and do not freeze until M6.

## Required tests

- User denied `VIEW_CHANNEL` cannot read cached content via API, WS replay, or old attachment link.
- Moderator cannot assign a role with `MANAGE_ROLES` beyond the moderator's own effective hierarchy.
- Ban revokes future sessions/media grants and removes active LiveKit participant. Short-lived old room tokens may still have a residual reconnection window in self-hosted LiveKit; document/test it.
- `TIMEOUT_MEMBERS` cannot affect equal/higher hierarchy and prevents sending/streaming.
- DM participants only can read; blocked users cannot start new DMs under agreed policy.
- Audits are append-only; actor, subject, timestamp, reason, before/after captured.
