# Whisk — Database contract — proposed SQLite v1

Primary relational data is persisted per instance in SQLite WAL; the client has a distinct SQLite cache. Use SQLx migrations, `PRAGMA foreign_keys=ON`, UTC timestamps and UUID/ULID string IDs. Schema details must be made exact in M1 migrations and compatibility tests.

| Entity | Key fields and constraints |
|---|---|
| instance_settings | singleton `instance_id`, `base_url`, `registration_policy`, `attachment_limit_bytes`, `protocol_version` |
| communities | `id`, `owner_user_id`, `is_default`; service enforces exactly one in v1 |
| users | `id`, normalized unique `username`, `password_hash`, `status`, `avatar_attachment_id` |
| sessions | `id`, `user_id`, `refresh_hash`, `expires_at`, `revoked_at` |
| invites | `id`, unique `token_hash`, `community_id`, `created_by`, `role_id`, expiry/max uses/counter/revoked |
| memberships | composite unique `(community_id, user_id)`, `timeout_until` |
| roles, member_roles | ordered roles and role grants scoped to community; assignment hierarchy enforced |
| channel_categories, channels | ordered categories; channel `type=text|voice`, foreign key community |
| channel_overwrites | unique `(channel_id, target_type, target_id)`, allow/deny bitsets |
| dm_threads, dm_participants | normalized 1:1 pair unique, exactly two participants |
| messages | `id`, `author_id`, exactly one `channel_id` XOR `dm_id`, monotonic `created_seq`, `client_nonce`, optional tombstone |
| reactions | unique `(message_id, user_id, emoji)` |
| attachments, upload_intents | private opaque `object_key`, `sha256`, state, upload idempotency, owner message, quota |
| read_states, blocks | per user/conversation read cursor; instance-local user block relations |
| moderation_actions, bans | append-only audit; revokeable bans, actor and subject |
| durable_events | **instance-wide** integer sequence `seq` primary key, event id, subject, visibility tags, JSON payload |
| idempotency_keys | unique `(actor_id, scope, client_nonce)` and committed result |

## Consistency invariants

1. Message insert **and** corresponding `durable_events` row commit in one transaction; never broadcast an uncommitted message.
2. Repeated message create with identical `(actor, conversation, client_nonce)` returns original message, not a duplicate.
3. Invite redemption increments `use_count` atomically under max-uses/expiry check.
4. Updates with `revision` implement optimistic concurrency; stale requests return `409 conflict`.
5. `messages`: `CHECK ((channel_id IS NULL) != (dm_id IS NULL))`; no cross-instance foreign keys ever.
6. Attachment transitions: `pending -> ready -> deleted`; a pending/foreign/hidden object may not be downloaded.
7. Event backfill always re-evaluates **current** visibility; a revoked member cannot replay secret events from an old cursor.
8. Read-state sequence cannot regress; soft-delete is audited; purges use explicit retention jobs.

## Required indexes

`messages(channel_id,created_seq DESC)`, `messages(dm_id,created_seq DESC)`, `idempotency_keys(actor_id,scope,client_nonce)`, `durable_events(seq)`, `channels(community_id,category_id,position)`, `attachments(object_key UNIQUE)`, `sessions(user_id,revoked_at,expires_at)`, `read_states(user_id,conversation_type,conversation_id)`. Final DDL should use proper partial indexes for nullable conversation IDs.

## Transaction/concurrency traps

- Assign durable event sequence inside the message transaction; avoid event-order inversion under competing writers.
- SQLite single-writer behavior is fine for a small community, but use short transactions and retry/busy timeout testing.
- Avoid assuming client-visible consecutive event `seq` values: authorization filters can create gaps; maintain an instance cursor watermark and use snapshot recovery when needed.
- Treat any migration changing role resolution or visibility as an auth-impacting migration requiring negative tests.
