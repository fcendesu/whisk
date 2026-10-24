# Whisk — M1 Backend, Protocol and Docker Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create a working Rust server and Compose stack with one initialized community, versioned API/gateway contracts, durable migrations and diagnostics.

**Architecture:** Small Cargo workspace, Axum routes, SQLx SQLite with transactional durable event log, versioned protocol crate, Compose single-node hosting.

**Tech Stack:** Rust, Tokio, Axum, SQLx/SQLite WAL, OpenAPI 3.1, Docker Compose, Caddy, LiveKit.

**Spec:** `docs/superpowers/specs/2026-10-24-platform-design.md` (read with `docs/prd.md`)

**Status:** Planned, not executed. These paths/signatures are proposed; reconcile them with the pinned toolchain and real repository before implementing.

## Global Constraints

- Exactly one community per backend in v1; UUID instance identity persists across restart.
- LiveKit failure does not stop API/message reads; backend is Linux-hosted in Compose.
- Default registration is invite-only; test consistent SQLite storage across restart.
- Expose /api/v1/instance public metadata but not secrets; changes to protocol need tests.
- Prefer task-by-task test-first behavior, red → green → refactor → small commit; never mark tests passing without executing them.

## Review Focus

- Restart must not rotate instance_id (Task 2).
- Second owner/bootstrap attempt cannot create second community (Task 3).
- LiveKit down must not make basic HTTP liveness fail (Task 4).
- Durable event commit must be atomic with resource change (Task 5).
- Missing Compose secrets must stop launch with actionable error (Task 6).

---

## File and interface map

| Task | Implementation owner | Test owner |
|---|---|---|
| 1 · Cargo workspace and protocol IDs | `Cargo.toml; crates/protocol/src/lib.rs; crates/protocol/src/ids.rs` | `crates/protocol/tests/ids.rs` |
| 2 · SQLite pool and migrations | `services/server/src/db.rs; services/server/migrations/0001_initial.sql` | `services/server/tests/migrations.rs` |
| 3 · One-time owner bootstrap | `services/server/src/bootstrap.rs; services/server/src/routes/bootstrap.rs` | `services/server/tests/bootstrap.rs` |
| 4 · Instance discovery and health | `services/server/src/routes/instance.rs; services/server/src/routes/health.rs` | `services/server/tests/discovery.rs` |
| 5 · Durable event outbox and gateway foundation | `services/server/src/events.rs; services/server/src/routes/gateway.rs; contracts/openapi.yaml` | `services/server/tests/event_tx.rs` |
| 6 · Compose deploy smoke | `deploy/compose.yaml; deploy/.env.example; deploy/Caddyfile; deploy/livekit.yaml; scripts/smoke-compose.sh` | `scripts/test-compose-config.sh` |

### Task 1: Cargo workspace and protocol IDs

**Files:**
- Create: `Cargo.toml`
- Create: `crates/protocol/src/lib.rs`
- Create: `crates/protocol/src/ids.rs`
- Test: `crates/protocol/tests/ids.rs`

**Interfaces:**
- Consumes: Rust stable toolchain pinned after M0.
- Produces: `pub struct InstanceId(Uuid); pub struct CommunityId(Uuid); pub struct UserId(Uuid)`.

- [ ] **Step 1 — write failing test:** In `crates/protocol/tests/ids.rs`, add `instance_scoped_ids_do_not_alias`. Two same-shaped resource IDs from different instances remain distinct once qualified by `InstanceId`.
- [ ] **Step 2 — verify red:** Run `cargo test -p protocol instance_scoped_ids_do_not_alias`; expect **FAIL** from missing/incorrect behavior (not merely a test runner/config error).
- [ ] **Step 3 — implement minimum:** Set up workspace membership and typed IDs; implement canonical serialization and instance-qualified cache keys. Implement the interface above; update adjacent contract docs for interface changes.
- [ ] **Step 4 — verify green:** Run `cargo test -p protocol instance_scoped_ids_do_not_alias`; expect **PASS**. Then run the package test suite and relevant clippy/format checks.
- [ ] **Step 5 — commit:** Stage `Cargo.toml; crates/protocol/src/lib.rs; crates/protocol/src/ids.rs` and `crates/protocol/tests/ids.rs` (plus necessary docs); commit as `chore: define workspace and typed IDs`.

### Task 2: SQLite pool and migrations

**Files:**
- Create: `services/server/src/db.rs`
- Create: `services/server/migrations/0001_initial.sql`
- Test: `services/server/tests/migrations.rs`

**Interfaces:**
- Consumes: Typed IDs from Task 1, SQLite file path.
- Produces: `async fn connect_database(url: &str) -> Result<SqlitePool, DbError>`.

- [ ] **Step 1 — write failing test:** In `services/server/tests/migrations.rs`, add `database_restart_preserves_instance_id`. Reopening same SQLite file preserves singleton instance ID and exactly one default community.
- [ ] **Step 2 — verify red:** Run `cargo test -p server database_restart_preserves_instance_id`; expect **FAIL** from missing/incorrect behavior (not merely a test runner/config error).
- [ ] **Step 3 — implement minimum:** Enable WAL/foreign keys, create v1 schema and indices from docs/database.md; database setup is idempotent. Implement the interface above; update adjacent contract docs for interface changes.
- [ ] **Step 4 — verify green:** Run `cargo test -p server database_restart_preserves_instance_id`; expect **PASS**. Then run the package test suite and relevant clippy/format checks.
- [ ] **Step 5 — commit:** Stage `services/server/src/db.rs; services/server/migrations/0001_initial.sql` and `services/server/tests/migrations.rs` (plus necessary docs); commit as `feat: add durable sqlite schema`.

### Task 3: One-time owner bootstrap

**Files:**
- Create: `services/server/src/bootstrap.rs`
- Create: `services/server/src/routes/bootstrap.rs`
- Test: `services/server/tests/bootstrap.rs`

**Interfaces:**
- Consumes: Database pool, users/communities tables.
- Produces: `async fn initialize_owner(db: &SqlitePool, input: OwnerBootstrap) -> Result<OwnerId, BootstrapError>`.

- [ ] **Step 1 — write failing test:** In `services/server/tests/bootstrap.rs`, add `bootstrap_is_single_use`. Second bootstrap attempt returns `AlreadyInitialized`, and row count remains one community/one owner.
- [ ] **Step 2 — verify red:** Run `cargo test -p server bootstrap_is_single_use`; expect **FAIL** from missing/incorrect behavior (not merely a test runner/config error).
- [ ] **Step 3 — implement minimum:** Use one-time secret and transaction; validate password and config; refuse public bootstrap after initialized. Implement the interface above; update adjacent contract docs for interface changes.
- [ ] **Step 4 — verify green:** Run `cargo test -p server bootstrap_is_single_use`; expect **PASS**. Then run the package test suite and relevant clippy/format checks.
- [ ] **Step 5 — commit:** Stage `services/server/src/bootstrap.rs; services/server/src/routes/bootstrap.rs` and `services/server/tests/bootstrap.rs` (plus necessary docs); commit as `feat: secure one-time owner initialization`.

### Task 4: Instance discovery and health

**Files:**
- Create: `services/server/src/routes/instance.rs`
- Create: `services/server/src/routes/health.rs`
- Test: `services/server/tests/discovery.rs`

**Interfaces:**
- Consumes: Instance settings from Task 2, axum router.
- Produces: `fn instance_routes(state: AppState) -> Router; fn health_routes(state: AppState) -> Router`.

- [ ] **Step 1 — write failing test:** In `services/server/tests/discovery.rs`, add `public_instance_info_redacts_secrets`. GET /api/v1/instance exposes ID/protocol/limits; never exposes LiveKit secret, DB URL, user directory.
- [ ] **Step 2 — verify red:** Run `cargo test -p server public_instance_info_redacts_secrets`; expect **FAIL** from missing/incorrect behavior (not merely a test runner/config error).
- [ ] **Step 3 — implement minimum:** Define JSON snake_case response and errors; /health/live not dependent on SFU availability, /health/ready reports degraded service. Implement the interface above; update adjacent contract docs for interface changes.
- [ ] **Step 4 — verify green:** Run `cargo test -p server public_instance_info_redacts_secrets`; expect **PASS**. Then run the package test suite and relevant clippy/format checks.
- [ ] **Step 5 — commit:** Stage `services/server/src/routes/instance.rs; services/server/src/routes/health.rs` and `services/server/tests/discovery.rs` (plus necessary docs); commit as `feat: add instance discovery and health`.

### Task 5: Durable event outbox and gateway foundation

**Files:**
- Create: `services/server/src/events.rs`
- Create: `services/server/src/routes/gateway.rs`
- Create: `contracts/openapi.yaml`
- Test: `services/server/tests/event_tx.rs`

**Interfaces:**
- Consumes: SQLite pool, shared protocol event ID and sequence.
- Produces: `async fn append_event(tx: &mut Transaction<Sqlite>, event: NewEvent) -> Result<i64, EventError>`.

- [ ] **Step 1 — write failing test:** In `services/server/tests/event_tx.rs`, add `rolled_back_write_emits_no_event`. Rollback a data mutation; assert durable_event row absent, no successful event broadcast.
- [ ] **Step 2 — verify red:** Run `cargo test -p server rolled_back_write_emits_no_event`; expect **FAIL** from missing/incorrect behavior (not merely a test runner/config error).
- [ ] **Step 3 — implement minimum:** Implement DB event insert and WS handshake scaffold, define `/sync` cursor/watermark semantics; lint OpenAPI. Implement the interface above; update adjacent contract docs for interface changes.
- [ ] **Step 4 — verify green:** Run `cargo test -p server rolled_back_write_emits_no_event`; expect **PASS**. Then run the package test suite and relevant clippy/format checks.
- [ ] **Step 5 — commit:** Stage `services/server/src/events.rs; services/server/src/routes/gateway.rs; contracts/openapi.yaml` and `services/server/tests/event_tx.rs` (plus necessary docs); commit as `feat: add transactional event outbox`.

### Task 6: Compose deploy smoke

**Files:**
- Create: `deploy/compose.yaml`
- Create: `deploy/.env.example`
- Create: `deploy/Caddyfile`
- Create: `deploy/livekit.yaml`
- Create: `scripts/smoke-compose.sh`
- Test: `scripts/test-compose-config.sh`

**Interfaces:**
- Consumes: Image build and secrets from previous tasks.
- Produces: `bash scripts/smoke-compose.sh; bash scripts/test-compose-config.sh`.

- [ ] **Step 1 — write failing test:** In `scripts/test-compose-config.sh`, add `compose_missing_secrets_is_rejected`. Compose config/lint rejects empty bootstrap and LiveKit secrets; stack persists SQLite and object volumes on restart.
- [ ] **Step 2 — verify red:** Run `bash scripts/test-compose-config.sh`; expect **FAIL** from missing/incorrect behavior (not merely a test runner/config error).
- [ ] **Step 3 — implement minimum:** Create pinned single-node API/LiveKit/Caddy stack with persistent volumes, ports, startup docs and negative env validation. Implement the interface above; update adjacent contract docs for interface changes.
- [ ] **Step 4 — verify green:** Run `bash scripts/test-compose-config.sh`; expect **PASS**. Then run the package test suite and relevant clippy/format checks.
- [ ] **Step 5 — commit:** Stage `deploy/compose.yaml; deploy/.env.example; deploy/Caddyfile; deploy/livekit.yaml; scripts/smoke-compose.sh` and `scripts/test-compose-config.sh` (plus necessary docs); commit as `chore: add docker compose self hosting`.

## Plan self-review and handoff

- [ ] Each stated spec requirement for this milestone maps to a task or a documented later milestone.
- [ ] Each test name/assertion and interface is consistent with the owner task; all five Review Focus items are covered.
- [ ] Paths have been checked against the actual repository after scaffolding; no phantom pre-existing files are assumed.
- [ ] Recheck version-pinned LiveKit/GPUI signatures and OS capture APIs before execution.
- [ ] Get plan review before implementation. Prefer subagent-driven implementation and independent spec/code review when the agent supports it.
