# Whisk — M3 Text, DM, Sync and Attachments Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver reliable channels/DMs with offline recovery and private 25 MiB files through provider-independent local/S3 storage.

**Architecture:** Transactional HTTP message writes with a durable event stream, client-local sync cache and a private storage adapter with explicit attachment lifecycle.

**Tech Stack:** Axum, SQLx/SQLite, Tokio WebSockets, GPUI lists, AWS SDK for Rust S3 adapter, client SQLite.

**Spec:** `docs/superpowers/specs/2026-10-24-messaging-storage-design.md` (read with `docs/prd.md`)

**Status:** Planned, not executed. These paths/signatures are proposed; reconcile them with the pinned toolchain and real repository before implementing.

## Global Constraints

- Max attachment size 26,214,400 bytes by default; local storage deploy requires no RustFS.
- Event delivery is at-least-once, visible effects are idempotent; cursor is instance-scoped.
- DM participant access and channel visibility checked before HTTP read, WS event and file GET.
- Stable opaque object key must survive local/RustFS/R2 migration.
- Prefer task-by-task test-first behavior, red → green → refactor → small commit; never mark tests passing without executing them.

## Review Focus

- Retries with duplicate client_nonce must create only one message (Task 2).
- Private WS event gaps must not falsely trigger missing-event recovery (Task 3).
- Non-DM-participant requests to a known DM ID must be denied (Task 4).
- 25 MiB+1 streaming body must fail and leave no ready attachment (Task 5).
- Attachment GET must fail after member loses channel access (Task 6).

---

## File and interface map

| Task | Implementation owner | Test owner |
|---|---|---|
| 1 · Channels and categories CRUD | `services/server/src/channels/repo.rs; services/server/src/routes/channels.rs` | `services/server/tests/channels.rs` |
| 2 · Message writes with idempotency | `services/server/src/messages/write.rs; services/server/src/routes/messages.rs` | `services/server/tests/message_idempotency.rs` |
| 3 · WS resume and client replay | `services/server/src/events/backfill.rs; crates/client-core/src/sync.rs; crates/client-cache/src/events.rs` | `crates/client-core/tests/reconnect.rs` |
| 4 · DM threads and blocking | `services/server/src/dms/repo.rs; services/server/src/routes/dms.rs` | `services/server/tests/dm_auth.rs` |
| 5 · Local/S3 storage adapter | `crates/storage/src/lib.rs; crates/storage/src/local.rs; crates/storage/src/s3.rs` | `crates/storage/tests/conformance.rs` |
| 6 · Attachment upload pipeline | `services/server/src/files/service.rs; services/server/src/routes/files.rs` | `services/server/tests/attachment_security.rs` |

### Task 1: Channels and categories CRUD

**Files:**
- Create: `services/server/src/channels/repo.rs`
- Create: `services/server/src/routes/channels.rs`
- Test: `services/server/tests/channels.rs`

**Interfaces:**
- Consumes: M1 schema, permission evaluator placeholder enforcing membership.
- Produces: `async fn create_channel(db: &SqlitePool, actor: UserId, input: NewChannel) -> Result<Channel, DomainError>`.

- [ ] **Step 1 — write failing test:** In `services/server/tests/channels.rs`, add `second_community_channel_is_denied`. Channel cannot be created for arbitrary nonexistent/foreign community; insertion needs valid membership and manage permission.
- [ ] **Step 2 — verify red:** Run `cargo test -p server second_community_channel_is_denied`; expect **FAIL** from missing/incorrect behavior (not merely a test runner/config error).
- [ ] **Step 3 — implement minimum:** Implement ordered channels/categories and event emission; HTTP mutations check permissions and revision. Implement the interface above; update adjacent contract docs for interface changes.
- [ ] **Step 4 — verify green:** Run `cargo test -p server second_community_channel_is_denied`; expect **PASS**. Then run the package test suite and relevant clippy/format checks.
- [ ] **Step 5 — commit:** Stage `services/server/src/channels/repo.rs; services/server/src/routes/channels.rs` and `services/server/tests/channels.rs` (plus necessary docs); commit as `feat: add channel management`.

### Task 2: Message writes with idempotency

**Files:**
- Create: `services/server/src/messages/write.rs`
- Create: `services/server/src/routes/messages.rs`
- Test: `services/server/tests/message_idempotency.rs`

**Interfaces:**
- Consumes: Channels and transactional events from M1, scoped actor.
- Produces: `async fn create_message(db: &SqlitePool, actor: UserId, target: ConversationId, input: NewMessage) -> Result<Message, DomainError>`.

- [ ] **Step 1 — write failing test:** In `services/server/tests/message_idempotency.rs`, add `duplicate_client_nonce_returns_original_message`. Submit same actor/target/nonce twice; same message ID and exactly one durable event.
- [ ] **Step 2 — verify red:** Run `cargo test -p server duplicate_client_nonce_returns_original_message`; expect **FAIL** from missing/incorrect behavior (not merely a test runner/config error).
- [ ] **Step 3 — implement minimum:** Insert message and outbox in one transaction; edit/delete/reactions with revision checks; safe text Markdown rendering on client. Implement the interface above; update adjacent contract docs for interface changes.
- [ ] **Step 4 — verify green:** Run `cargo test -p server duplicate_client_nonce_returns_original_message`; expect **PASS**. Then run the package test suite and relevant clippy/format checks.
- [ ] **Step 5 — commit:** Stage `services/server/src/messages/write.rs; services/server/src/routes/messages.rs` and `services/server/tests/message_idempotency.rs` (plus necessary docs); commit as `feat: add message write model`.

### Task 3: WS resume and client replay

**Files:**
- Create: `services/server/src/events/backfill.rs`
- Create: `crates/client-core/src/sync.rs`
- Create: `crates/client-cache/src/events.rs`
- Test: `crates/client-core/tests/reconnect.rs`

**Interfaces:**
- Consumes: Durable events and client profiles from M1/M2.
- Produces: `async fn apply_page(cache: &ClientCache, page: SyncPage) -> Result<SyncCursor, SyncError>`.

- [ ] **Step 1 — write failing test:** In `crates/client-core/tests/reconnect.rs`, add `private_seq_gap_does_not_force_bad_replay`. A filtered gap does not duplicate messages; authorized missing events apply, cursor advances only after persisted changes.
- [ ] **Step 2 — verify red:** Run `cargo test -p client-core private_seq_gap_does_not_force_bad_replay`; expect **FAIL** from missing/incorrect behavior (not merely a test runner/config error).
- [ ] **Step 3 — implement minimum:** Backfill with authorized watermark, cursor expiration snapshot path, reconnect jitter and cache invalidation. Implement the interface above; update adjacent contract docs for interface changes.
- [ ] **Step 4 — verify green:** Run `cargo test -p client-core private_seq_gap_does_not_force_bad_replay`; expect **PASS**. Then run the package test suite and relevant clippy/format checks.
- [ ] **Step 5 — commit:** Stage `services/server/src/events/backfill.rs; crates/client-core/src/sync.rs; crates/client-cache/src/events.rs` and `crates/client-core/tests/reconnect.rs` (plus necessary docs); commit as `feat: support reliable offline synchronization`.

### Task 4: DM threads and blocking

**Files:**
- Create: `services/server/src/dms/repo.rs`
- Create: `services/server/src/routes/dms.rs`
- Test: `services/server/tests/dm_auth.rs`

**Interfaces:**
- Consumes: Auth and message write model, canonical two-user pair key.
- Produces: `async fn get_or_create_dm(db: &SqlitePool, actor: UserId, peer: UserId) -> Result<DmThread, DomainError>`.

- [ ] **Step 1 — write failing test:** In `services/server/tests/dm_auth.rs`, add `nonparticipant_cannot_read_dm`. Known DM ID requested by user C returns opaque denied/not-found; blocked peer cannot create new DM.
- [ ] **Step 2 — verify red:** Run `cargo test -p server nonparticipant_cannot_read_dm`; expect **FAIL** from missing/incorrect behavior (not merely a test runner/config error).
- [ ] **Step 3 — implement minimum:** Enforce exactly two participants, unique pair, history pagination, unread state and blocks. Implement the interface above; update adjacent contract docs for interface changes.
- [ ] **Step 4 — verify green:** Run `cargo test -p server nonparticipant_cannot_read_dm`; expect **PASS**. Then run the package test suite and relevant clippy/format checks.
- [ ] **Step 5 — commit:** Stage `services/server/src/dms/repo.rs; services/server/src/routes/dms.rs` and `services/server/tests/dm_auth.rs` (plus necessary docs); commit as `feat: add instance-local direct messages`.

### Task 5: Local/S3 storage adapter

**Files:**
- Create: `crates/storage/src/lib.rs`
- Create: `crates/storage/src/local.rs`
- Create: `crates/storage/src/s3.rs`
- Test: `crates/storage/tests/conformance.rs`

**Interfaces:**
- Consumes: Opaque keys, path and AWS S3 configuration.
- Produces: `trait StorageBackend { async fn put(&self, key: &ObjectKey, body: ByteStream) -> Result<ObjectInfo, StorageError>; async fn get_range(...); async fn stat(...); async fn delete(...); }`.

- [ ] **Step 1 — write failing test:** In `crates/storage/tests/conformance.rs`, add `object_key_round_trip_on_local_and_s3`. Binary object survives put/head/ranged-get/delete on local backend and configured RustFS/S3 fixture.
- [ ] **Step 2 — verify red:** Run `cargo test -p storage object_key_round_trip_on_local_and_s3`; expect **FAIL** from missing/incorrect behavior (not merely a test runner/config error).
- [ ] **Step 3 — implement minimum:** Implement private atomic local write and portable S3 subset; optional profile for RustFS; no public ACL dependence. Implement the interface above; update adjacent contract docs for interface changes.
- [ ] **Step 4 — verify green:** Run `cargo test -p storage object_key_round_trip_on_local_and_s3`; expect **PASS**. Then run the package test suite and relevant clippy/format checks.
- [ ] **Step 5 — commit:** Stage `crates/storage/src/lib.rs; crates/storage/src/local.rs; crates/storage/src/s3.rs` and `crates/storage/tests/conformance.rs` (plus necessary docs); commit as `feat: abstract private attachment storage`.

### Task 6: Attachment upload pipeline

**Files:**
- Create: `services/server/src/files/service.rs`
- Create: `services/server/src/routes/files.rs`
- Test: `services/server/tests/attachment_security.rs`

**Interfaces:**
- Consumes: StorageBackend from Task 5, channels/DM auth.
- Produces: `async fn finalize_attachment(db: &SqlitePool, store: &dyn StorageBackend, actor: UserId, id: AttachmentId) -> Result<Attachment, DomainError>`.

- [ ] **Step 1 — write failing test:** In `services/server/tests/attachment_security.rs`, add `overlimit_and_old_hidden_attachment_are_rejected`. 25 MiB accepted; 25 MiB+1 rejected; after VIEW_CHANNEL revocation old URL returns denied, no public object.
- [ ] **Step 2 — verify red:** Run `cargo test -p server overlimit_and_old_hidden_attachment_are_rejected`; expect **FAIL** from missing/incorrect behavior (not merely a test runner/config error).
- [ ] **Step 3 — implement minimum:** Introduce intent/upload/finalize state machine, streamed limits, hash/mime validation, message linkage and orphan GC. Implement the interface above; update adjacent contract docs for interface changes.
- [ ] **Step 4 — verify green:** Run `cargo test -p server overlimit_and_old_hidden_attachment_are_rejected`; expect **PASS**. Then run the package test suite and relevant clippy/format checks.
- [ ] **Step 5 — commit:** Stage `services/server/src/files/service.rs; services/server/src/routes/files.rs` and `services/server/tests/attachment_security.rs` (plus necessary docs); commit as `feat: enforce private attachment lifecycle`.

## Plan self-review and handoff

- [ ] Each stated spec requirement for this milestone maps to a task or a documented later milestone.
- [ ] Each test name/assertion and interface is consistent with the owner task; all five Review Focus items are covered.
- [ ] Paths have been checked against the actual repository after scaffolding; no phantom pre-existing files are assumed.
- [ ] Recheck version-pinned LiveKit/GPUI signatures and OS capture APIs before execution.
- [ ] Get plan review before implementation. Prefer subagent-driven implementation and independent spec/code review when the agent supports it.
