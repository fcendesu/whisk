# Whisk — M6 Roles and Moderation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement custom roles, overwrite precedence, audited moderation and cross-transport authorization without escalation.

**Architecture:** A pure permission evaluator with table-driven/property tests controls endpoint/gateway/storage/media access; mutations write append-only audit events and terminate active forbidden subscriptions.

**Tech Stack:** Rust, SQLx, Axum, property tests, LiveKit server APIs.

**Spec:** `docs/superpowers/specs/2026-10-24-permissions-design.md` (read with `docs/prd.md`)

**Status:** Planned, not executed. These paths/signatures are proposed; reconcile them with the pinned toolchain and real repository before implementing.

## Global Constraints

- Agree overwrite conflict resolution before migrations: docs/permissions.md contains a proposal only.
- Owner immutable protected authority; moderator only targets strictly lower hierarchy.
- Ban/timeout override channel allows and media grants.
- Media stale-token reconnection window requires mitigation/documentation.
- Prefer task-by-task test-first behavior, red → green → refactor → small commit; never mark tests passing without executing them.

## Review Focus

- Conflicting role grants/overwrites must resolve predictably (Task 1).
- Role manager cannot grant higher permissions to self (Task 2).
- Replaying a formerly visible event after denial must be denied (Task 3).
- Kick/ban must update active voice participant and future joins (Task 4).
- Audit entries must be durable even after restart (Task 5).

---

## File and interface map

| Task | Implementation owner | Test owner |
|---|---|---|
| 1 · Pure evaluator and overwrite semantics | `crates/protocol/src/permissions.rs; services/server/src/permissions/evaluator.rs` | `services/server/tests/permission_matrix.rs` |
| 2 · Roles and hierarchy endpoints | `services/server/src/roles/service.rs; services/server/src/routes/roles.rs` | `services/server/tests/role_escalation.rs` |
| 3 · Authorize gateway and private objects | `services/server/src/events/visibility.rs; services/server/src/files/authorize.rs` | `services/server/tests/revoked_access.rs` |
| 4 · Moderation actions and media removal | `services/server/src/moderation/service.rs; services/server/src/media/moderate.rs` | `services/server/tests/moderation_actions.rs` |
| 5 · Append-only audit and regression suite | `services/server/src/moderation/audit.rs; services/server/src/routes/audit.rs` | `services/server/tests/audit.rs` |

### Task 1: Pure evaluator and overwrite semantics

**Files:**
- Create: `crates/protocol/src/permissions.rs`
- Create: `services/server/src/permissions/evaluator.rs`
- Test: `services/server/tests/permission_matrix.rs`

**Interfaces:**
- Consumes: Community roles, channel overrides, membership states.
- Produces: `fn effective_permissions(member: &MemberContext, channel: Option<&ChannelContext>) -> PermissionSet`.

- [ ] **Step 1 — write failing test:** In `services/server/tests/permission_matrix.rs`, add `conflicting_role_overwrites_are_deterministic`. Exhaustive table for owner/member, role-specific allow/deny conflicts, member-specific deny and timeout override.
- [ ] **Step 2 — verify red:** Run `cargo test -p server conflicting_role_overwrites_are_deterministic`; expect **FAIL** from missing/incorrect behavior (not merely a test runner/config error).
- [ ] **Step 3 — implement minimum:** Implement final *approved* allow/deny semantics and fixed bit allocation; keep evaluator side-effect free. Implement the interface above; update adjacent contract docs for interface changes.
- [ ] **Step 4 — verify green:** Run `cargo test -p server conflicting_role_overwrites_are_deterministic`; expect **PASS**. Then run the package test suite and relevant clippy/format checks.
- [ ] **Step 5 — commit:** Stage `crates/protocol/src/permissions.rs; services/server/src/permissions/evaluator.rs` and `services/server/tests/permission_matrix.rs` (plus necessary docs); commit as `feat: centralize permission resolution`.

### Task 2: Roles and hierarchy endpoints

**Files:**
- Create: `services/server/src/roles/service.rs`
- Create: `services/server/src/routes/roles.rs`
- Test: `services/server/tests/role_escalation.rs`

**Interfaces:**
- Consumes: Evaluator from Task 1, M1 DB.
- Produces: `async fn assign_role(actor: UserId, target: UserId, role: RoleId) -> Result<(), PermissionError>`.

- [ ] **Step 1 — write failing test:** In `services/server/tests/role_escalation.rs`, add `role_manager_cannot_assign_peer_or_higher_role`. Actor cannot assign a role >= own highest hierarchy or grants beyond their management policy.
- [ ] **Step 2 — verify red:** Run `cargo test -p server role_manager_cannot_assign_peer_or_higher_role`; expect **FAIL** from missing/incorrect behavior (not merely a test runner/config error).
- [ ] **Step 3 — implement minimum:** CRUD custom roles, hierarchy checks, prevent modifying owner special role, audit changes. Implement the interface above; update adjacent contract docs for interface changes.
- [ ] **Step 4 — verify green:** Run `cargo test -p server role_manager_cannot_assign_peer_or_higher_role`; expect **PASS**. Then run the package test suite and relevant clippy/format checks.
- [ ] **Step 5 — commit:** Stage `services/server/src/roles/service.rs; services/server/src/routes/roles.rs` and `services/server/tests/role_escalation.rs` (plus necessary docs); commit as `feat: enforce role hierarchy`.

### Task 3: Authorize gateway and private objects

**Files:**
- Create: `services/server/src/events/visibility.rs`
- Create: `services/server/src/files/authorize.rs`
- Test: `services/server/tests/revoked_access.rs`

**Interfaces:**
- Consumes: Evaluator + durable events + storage from M3.
- Produces: `fn can_read_event(actor: UserId, event: &DurableEvent, ctx: &AuthContext) -> bool; async fn authorize_attachment(actor: UserId, id: AttachmentId) -> Result<ObjectKey, Error>`.

- [ ] **Step 1 — write failing test:** In `services/server/tests/revoked_access.rs`, add `revoked_member_cannot_backfill_or_download`. After role overwrite deny, old cursor backfill and old private attachment URL return no restricted bytes.
- [ ] **Step 2 — verify red:** Run `cargo test -p server revoked_member_cannot_backfill_or_download`; expect **FAIL** from missing/incorrect behavior (not merely a test runner/config error).
- [ ] **Step 3 — implement minimum:** Check effective permissions at replay and GET, invalidate subscriptions/client views after permission changed event. Implement the interface above; update adjacent contract docs for interface changes.
- [ ] **Step 4 — verify green:** Run `cargo test -p server revoked_member_cannot_backfill_or_download`; expect **PASS**. Then run the package test suite and relevant clippy/format checks.
- [ ] **Step 5 — commit:** Stage `services/server/src/events/visibility.rs; services/server/src/files/authorize.rs` and `services/server/tests/revoked_access.rs` (plus necessary docs); commit as `feat: enforce visibility across transports`.

### Task 4: Moderation actions and media removal

**Files:**
- Create: `services/server/src/moderation/service.rs`
- Create: `services/server/src/media/moderate.rs`
- Test: `services/server/tests/moderation_actions.rs`

**Interfaces:**
- Consumes: Evaluator, LiveKit admin client from M4.
- Produces: `async fn ban_member(actor: UserId, target: UserId, reason: String) -> Result<ModerationAction, ModerationError>`.

- [ ] **Step 1 — write failing test:** In `services/server/tests/moderation_actions.rs`, add `ban_denies_text_file_and_new_room_token`. Ban blocks message/file/media operations, triggers participant removal, writes one durable audit event.
- [ ] **Step 2 — verify red:** Run `cargo test -p server ban_denies_text_file_and_new_room_token`; expect **FAIL** from missing/incorrect behavior (not merely a test runner/config error).
- [ ] **Step 3 — implement minimum:** Implement timeout/kick/ban/unban, remove active subscriptions/media, document JWT residual exposure. Implement the interface above; update adjacent contract docs for interface changes.
- [ ] **Step 4 — verify green:** Run `cargo test -p server ban_denies_text_file_and_new_room_token`; expect **PASS**. Then run the package test suite and relevant clippy/format checks.
- [ ] **Step 5 — commit:** Stage `services/server/src/moderation/service.rs; services/server/src/media/moderate.rs` and `services/server/tests/moderation_actions.rs` (plus necessary docs); commit as `feat: implement server-side moderation`.

### Task 5: Append-only audit and regression suite

**Files:**
- Create: `services/server/src/moderation/audit.rs`
- Create: `services/server/src/routes/audit.rs`
- Test: `services/server/tests/audit.rs`

**Interfaces:**
- Consumes: Moderator actions and transaction state from Task 4.
- Produces: `async fn record_audit(tx: &mut Transaction<Sqlite>, entry: AuditEntry) -> Result<(), AuditError>`.

- [ ] **Step 1 — write failing test:** In `services/server/tests/audit.rs`, add `audit_history_survives_restart_and_is_restricted`. After restart authorized moderator reads action; member cannot read; no edit/delete route exists.
- [ ] **Step 2 — verify red:** Run `cargo test -p server audit_history_survives_restart_and_is_restricted`; expect **FAIL** from missing/incorrect behavior (not merely a test runner/config error).
- [ ] **Step 3 — implement minimum:** Persist actor/target/reason/before-after and add audit paging; run full negative permission matrix. Implement the interface above; update adjacent contract docs for interface changes.
- [ ] **Step 4 — verify green:** Run `cargo test -p server audit_history_survives_restart_and_is_restricted`; expect **PASS**. Then run the package test suite and relevant clippy/format checks.
- [ ] **Step 5 — commit:** Stage `services/server/src/moderation/audit.rs; services/server/src/routes/audit.rs` and `services/server/tests/audit.rs` (plus necessary docs); commit as `feat: preserve immutable moderation audit`.

## Plan self-review and handoff

- [ ] Each stated spec requirement for this milestone maps to a task or a documented later milestone.
- [ ] Each test name/assertion and interface is consistent with the owner task; all five Review Focus items are covered.
- [ ] Paths have been checked against the actual repository after scaffolding; no phantom pre-existing files are assumed.
- [ ] Recheck version-pinned LiveKit/GPUI signatures and OS capture APIs before execution.
- [ ] Get plan review before implementation. Prefer subagent-driven implementation and independent spec/code review when the agent supports it.
