# Whisk — M2 Identity and Unified Multi-Instance Desktop Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let the native client authenticate into three independent invite-only instances and switch between them in a unified Discord-style sidebar with isolated sessions/caches.

**Architecture:** Instance-local auth in Axum, revocable sessions, OS credential storage and per-instance client connections behind a GPUI navigation shell.

**Tech Stack:** Rust, Axum, SQLx, Argon2id, GPUI, OS keychain APIs, Tokio, SQLite client cache.

**Spec:** `docs/superpowers/specs/2026-10-24-identity-client-design.md` (read with `docs/prd.md`)

**Status:** Planned, not executed. These paths/signatures are proposed; reconcile them with the pinned toolchain and real repository before implementing.

## Global Constraints

- Invite-only default, atomic max-uses redemption; never put refresh tokens in SQLite/logging.
- Each profile keyed by `(instance_id, account_id)`; no cross-instance identity or DM merging.
- Failure of one instance cannot block the rest; one active voice connection globally.
- Prefer task-by-task test-first behavior, red → green → refactor → small commit; never mark tests passing without executing them.

## Review Focus

- Expiring invite during concurrent signup cannot oversubscribe max uses (Task 1).
- Reused refresh token must be revoked rather than rotated again (Task 2).
- Unknown or changed server instance ID must block silent login (Task 3).
- Removing profile A must not wipe profile B cache (Task 4).
- Disconnected instance A must not prevent using B in sidebar (Task 5).

---

## File and interface map

| Task | Implementation owner | Test owner |
|---|---|---|
| 1 · Invite creation and redemption | `services/server/src/auth/invites.rs; services/server/src/routes/invites.rs` | `services/server/tests/invite_flow.rs` |
| 2 · Password sessions with refresh rotation | `services/server/src/auth/sessions.rs; services/server/src/routes/auth.rs` | `services/server/tests/refresh.rs` |
| 3 · Instance discovery and trust | `crates/client-core/src/discovery.rs; crates/client-core/src/profiles.rs` | `crates/client-core/tests/discovery.rs` |
| 4 · OS credentials and separated caches | `crates/client-core/src/credentials.rs; crates/client-cache/src/lib.rs` | `crates/client-cache/tests/isolation.rs` |
| 5 · GPUI unified sidebar + connection manager | `apps/desktop/src/app.rs; apps/desktop/src/ui/instance_rail.rs; crates/client-core/src/connections.rs` | `crates/client-core/tests/failure_isolation.rs` |

### Task 1: Invite creation and redemption

**Files:**
- Create: `services/server/src/auth/invites.rs`
- Create: `services/server/src/routes/invites.rs`
- Test: `services/server/tests/invite_flow.rs`

**Interfaces:**
- Consumes: Communities, user and role tables from M1.
- Produces: `async fn redeem_invite(tx: &mut Transaction<Sqlite>, raw: &str) -> Result<InviteGrant, InviteError>`.

- [ ] **Step 1 — write failing test:** In `services/server/tests/invite_flow.rs`, add `concurrent_single_use_invite_redeems_once`. Two signups race on max_uses=1; exactly one succeeds, and used_count remains 1.
- [ ] **Step 2 — verify red:** Run `cargo test -p server concurrent_single_use_invite_redeems_once`; expect **FAIL** from missing/incorrect behavior (not merely a test runner/config error).
- [ ] **Step 3 — implement minimum:** Hash invite tokens, expire/revoke validation, issuer role restriction, owner invite defaults. Implement the interface above; update adjacent contract docs for interface changes.
- [ ] **Step 4 — verify green:** Run `cargo test -p server concurrent_single_use_invite_redeems_once`; expect **PASS**. Then run the package test suite and relevant clippy/format checks.
- [ ] **Step 5 — commit:** Stage `services/server/src/auth/invites.rs; services/server/src/routes/invites.rs` and `services/server/tests/invite_flow.rs` (plus necessary docs); commit as `feat: add invitation authentication`.

### Task 2: Password sessions with refresh rotation

**Files:**
- Create: `services/server/src/auth/sessions.rs`
- Create: `services/server/src/routes/auth.rs`
- Test: `services/server/tests/refresh.rs`

**Interfaces:**
- Consumes: Users/sessions from M1; Argon2id.
- Produces: `async fn rotate_refresh(db: &SqlitePool, token: SecretString) -> Result<SessionPair, AuthError>`.

- [ ] **Step 1 — write failing test:** In `services/server/tests/refresh.rs`, add `replayed_refresh_token_is_denied`. First refresh returns new pair, second reuse of old refresh is rejected and logged.
- [ ] **Step 2 — verify red:** Run `cargo test -p server replayed_refresh_token_is_denied`; expect **FAIL** from missing/incorrect behavior (not merely a test runner/config error).
- [ ] **Step 3 — implement minimum:** Argon2id password hashing, revocable hashed refresh tokens, login/logout-all, throttling. Implement the interface above; update adjacent contract docs for interface changes.
- [ ] **Step 4 — verify green:** Run `cargo test -p server replayed_refresh_token_is_denied`; expect **PASS**. Then run the package test suite and relevant clippy/format checks.
- [ ] **Step 5 — commit:** Stage `services/server/src/auth/sessions.rs; services/server/src/routes/auth.rs` and `services/server/tests/refresh.rs` (plus necessary docs); commit as `feat: add revocable account sessions`.

### Task 3: Instance discovery and trust

**Files:**
- Create: `crates/client-core/src/discovery.rs`
- Create: `crates/client-core/src/profiles.rs`
- Test: `crates/client-core/tests/discovery.rs`

**Interfaces:**
- Consumes: GET /api/v1/instance protocol from M1.
- Produces: `async fn verify_instance(url: Url, known: Option<InstanceId>) -> Result<InstanceMetadata, DiscoveryError>`.

- [ ] **Step 1 — write failing test:** In `crates/client-core/tests/discovery.rs`, add `changed_instance_identity_requires_confirmation`. Stored URL pointing to different `instance_id` returns IdentityChanged and does not reuse credentials.
- [ ] **Step 2 — verify red:** Run `cargo test -p client-core changed_instance_identity_requires_confirmation`; expect **FAIL** from missing/incorrect behavior (not merely a test runner/config error).
- [ ] **Step 3 — implement minimum:** Validate HTTPS/compatibility, normalize origin, persist stable identity and per-account connection profile. Implement the interface above; update adjacent contract docs for interface changes.
- [ ] **Step 4 — verify green:** Run `cargo test -p client-core changed_instance_identity_requires_confirmation`; expect **PASS**. Then run the package test suite and relevant clippy/format checks.
- [ ] **Step 5 — commit:** Stage `crates/client-core/src/discovery.rs; crates/client-core/src/profiles.rs` and `crates/client-core/tests/discovery.rs` (plus necessary docs); commit as `feat: verify instance identity in client`.

### Task 4: OS credentials and separated caches

**Files:**
- Create: `crates/client-core/src/credentials.rs`
- Create: `crates/client-cache/src/lib.rs`
- Test: `crates/client-cache/tests/isolation.rs`

**Interfaces:**
- Consumes: Profile key from Task 3; OS keychain abstraction.
- Produces: `fn cache_namespace(instance: InstanceId, account: UserId) -> CacheNamespace; async fn remove_profile(key: ProfileKey, purge: bool)`.

- [ ] **Step 1 — write failing test:** In `crates/client-cache/tests/isolation.rs`, add `removing_one_profile_preserves_other_cache`. Data stored for instance B still exists after removing A with purge=true; no bearer token inside cache.
- [ ] **Step 2 — verify red:** Run `cargo test -p client-cache removing_one_profile_preserves_other_cache`; expect **FAIL** from missing/incorrect behavior (not merely a test runner/config error).
- [ ] **Step 3 — implement minimum:** Keychain token references, SQLite namespace per profile, individual profile removal and resource cleanup. Implement the interface above; update adjacent contract docs for interface changes.
- [ ] **Step 4 — verify green:** Run `cargo test -p client-cache removing_one_profile_preserves_other_cache`; expect **PASS**. Then run the package test suite and relevant clippy/format checks.
- [ ] **Step 5 — commit:** Stage `crates/client-core/src/credentials.rs; crates/client-cache/src/lib.rs` and `crates/client-cache/tests/isolation.rs` (plus necessary docs); commit as `feat: isolate secure profile state`.

### Task 5: GPUI unified sidebar + connection manager

**Files:**
- Create: `apps/desktop/src/app.rs`
- Create: `apps/desktop/src/ui/instance_rail.rs`
- Create: `crates/client-core/src/connections.rs`
- Test: `crates/client-core/tests/failure_isolation.rs`

**Interfaces:**
- Consumes: Profiles from Tasks 3–4, server auth from Tasks 1–2.
- Produces: `async fn connect_profile(&self, key: ProfileKey) -> Result<ConnectionHandle, ConnectionError>; fn select_profile(&mut self, key: ProfileKey)`.

- [ ] **Step 1 — write failing test:** In `crates/client-core/tests/failure_isolation.rs`, add `instance_failure_is_isolated`. Three instance connections; A disconnects while B continues fetching and C updates unread badge.
- [ ] **Step 2 — verify red:** Run `cargo test -p client-core instance_failure_is_isolated`; expect **FAIL** from missing/incorrect behavior (not merely a test runner/config error).
- [ ] **Step 3 — implement minimum:** Render rail with instance icons/badges, connect/reconnect independently, show statuses and prevent multiple voice sessions. Implement the interface above; update adjacent contract docs for interface changes.
- [ ] **Step 4 — verify green:** Run `cargo test -p client-core instance_failure_is_isolated`; expect **PASS**. Then run the package test suite and relevant clippy/format checks.
- [ ] **Step 5 — commit:** Stage `apps/desktop/src/app.rs; apps/desktop/src/ui/instance_rail.rs; crates/client-core/src/connections.rs` and `crates/client-core/tests/failure_isolation.rs` (plus necessary docs); commit as `feat: add multi-instance gpui navigation`.

## Plan self-review and handoff

- [ ] Each stated spec requirement for this milestone maps to a task or a documented later milestone.
- [ ] Each test name/assertion and interface is consistent with the owner task; all five Review Focus items are covered.
- [ ] Paths have been checked against the actual repository after scaffolding; no phantom pre-existing files are assumed.
- [ ] Recheck version-pinned LiveKit/GPUI signatures and OS capture APIs before execution.
- [ ] Get plan review before implementation. Prefer subagent-driven implementation and independent spec/code review when the agent supports it.
