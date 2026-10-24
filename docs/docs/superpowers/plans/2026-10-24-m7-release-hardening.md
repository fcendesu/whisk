# Whisk — M7 Release, Packaging and Operations Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship Windows/Linux/macOS packages and a reproducible Docker operator workflow only after security, restore and measured media performance gates pass.

**Architecture:** Cross-platform packaging and CI; backup/restore and storage migration scripts; independent smoke/performance conformance suites and a documented support matrix.

**Tech Stack:** Cargo CI, packaging/signing per platform, Docker Compose, shell/Python test runners, LiveKit and S3 fixtures.

**Spec:** `docs/superpowers/specs/2026-10-24-platform-design.md` (read with `docs/prd.md`)

**Status:** Planned, not executed. These paths/signatures are proposed; reconcile them with the pinned toolchain and real repository before implementing.

## Global Constraints

- License remains undecided until dependency and redistribution audit.
- Whisk is the selected working name, but domain/trademark/bundle-ID review is required before a public release; a logo or tagline is not yet approved.
- Do not claim feature parity without native screen/camera/audio device tests across supported platforms.
- Backup must include SQLite and object bytes; migration preserves attachment IDs.
- Reject release if high-risk negative auth tests or media benchmark gates fail.
- Prefer task-by-task test-first behavior, red → green → refactor → small commit; never mark tests passing without executing them.

## Review Focus

- Restore on fresh machine preserves instance ID and private attachments (Task 1).
- Incomplete copy during RustFS→R2 switch must abort before cutover (Task 2).
- GPU unsupported on one OS must show user-visible fallback (Task 3).
- Clean install must reject unconfigured secrets and expose necessary UDP diagnosis (Task 4).
- Release must block if audited license or permission matrix incomplete (Task 5).

---

## File and interface map

| Task | Implementation owner | Test owner |
|---|---|---|
| 1 · Backup and restore scripts | `scripts/backup.sh; scripts/restore.sh; docs/operator/backup-restore.md` | `scripts/tests/backup_restore.sh` |
| 2 · Object-store migration verification | `scripts/migrate-storage.rs; docs/operator/storage-migration.md` | `scripts/tests/storage_migration.sh` |
| 3 · Desktop packaging and matrix | `ci/desktop-builds.yaml; docs/operator/platform-matrix.md` | `scripts/tests/desktop_smoke.sh` |
| 4 · Operator one-command Docker smoke | `scripts/smoke-production-compose.sh; docs/operator/install.md` | `scripts/tests/compose_ops.sh` |
| 5 · Release quality gate and license audit | `docs/release-checklist.md; scripts/check-release-gates.sh; docs/operator/dependencies.md` | `scripts/tests/release_gate.sh` |

### Task 1: Backup and restore scripts

**Files:**
- Create: `scripts/backup.sh`
- Create: `scripts/restore.sh`
- Create: `docs/operator/backup-restore.md`
- Test: `scripts/tests/backup_restore.sh`

**Interfaces:**
- Consumes: Docker volumes, SQLite online backup, attachment backend.
- Produces: `bash scripts/backup.sh --output <dir>; bash scripts/restore.sh --input <dir>`.

- [ ] **Step 1 — write failing test:** In `scripts/tests/backup_restore.sh`, add `restored_instance_and_attachments_equal`. Fresh Compose instance after restore has same instance UUID, message count, roles, audit and attachment checksums.
- [ ] **Step 2 — verify red:** Run `bash scripts/tests/backup_restore.sh`; expect **FAIL** from missing/incorrect behavior (not merely a test runner/config error).
- [ ] **Step 3 — implement minimum:** Write consistent SQLite + object snapshot and restore verification with cutover failure handling. Implement the interface above; update adjacent contract docs for interface changes.
- [ ] **Step 4 — verify green:** Run `bash scripts/tests/backup_restore.sh`; expect **PASS**. Then run the package test suite and relevant clippy/format checks.
- [ ] **Step 5 — commit:** Stage `scripts/backup.sh; scripts/restore.sh; docs/operator/backup-restore.md` and `scripts/tests/backup_restore.sh` (plus necessary docs); commit as `ops: add tested backup and restore`.

### Task 2: Object-store migration verification

**Files:**
- Create: `scripts/migrate-storage.rs`
- Create: `docs/operator/storage-migration.md`
- Test: `scripts/tests/storage_migration.sh`

**Interfaces:**
- Consumes: StorageBackend local/RustFS/R2, database attachment manifest.
- Produces: `fn migrate_objects(source: &dyn StorageBackend, target: &dyn StorageBackend, manifest: &Manifest) -> Result<MigrationReport, MigrationError>`.

- [ ] **Step 1 — write failing test:** In `scripts/tests/storage_migration.sh`, add `checksum_mismatch_prevents_cutover`. Deliberately corrupt one target object; migration reports failure and does not switch active backend.
- [ ] **Step 2 — verify red:** Run `bash scripts/tests/storage_migration.sh`; expect **FAIL** from missing/incorrect behavior (not merely a test runner/config error).
- [ ] **Step 3 — implement minimum:** Freeze writes, copy, SHA-256 verification and rollback procedure; preserve object key and attachment IDs. Implement the interface above; update adjacent contract docs for interface changes.
- [ ] **Step 4 — verify green:** Run `bash scripts/tests/storage_migration.sh`; expect **PASS**. Then run the package test suite and relevant clippy/format checks.
- [ ] **Step 5 — commit:** Stage `scripts/migrate-storage.rs; docs/operator/storage-migration.md` and `scripts/tests/storage_migration.sh` (plus necessary docs); commit as `ops: support verified storage cutover`.

### Task 3: Desktop packaging and matrix

**Files:**
- Create: `ci/desktop-builds.yaml`
- Create: `docs/operator/platform-matrix.md`
- Test: `scripts/tests/desktop_smoke.sh`

**Interfaces:**
- Consumes: M2 GPUI app, M4/M5 media modules.
- Produces: `bash scripts/tests/desktop_smoke.sh <platform>`.

- [ ] **Step 1 — write failing test:** In `scripts/tests/desktop_smoke.sh`, add `unsupported_encoder_fallback_is_reported`. On software-only device, 1440p60 request displays disabled/negotiated fallback instead of claiming success.
- [ ] **Step 2 — verify red:** Run `bash scripts/tests/desktop_smoke.sh linux`; expect **FAIL** from missing/incorrect behavior (not merely a test runner/config error).
- [ ] **Step 3 — implement minimum:** Build Whisk Desktop installers/assets for Windows, Linux Wayland/X11 and macOS; verify app display name and package metadata; record media permissions, hardware codecs and signature strategy. Keep visual assets provisional until approved. Implement the interface above; update adjacent contract docs for interface changes.
- [ ] **Step 4 — verify green:** Run `bash scripts/tests/desktop_smoke.sh linux`; expect **PASS**. Then run the package test suite and relevant clippy/format checks.
- [ ] **Step 5 — commit:** Stage `ci/desktop-builds.yaml; docs/operator/platform-matrix.md` and `scripts/tests/desktop_smoke.sh` (plus necessary docs); commit as `release: establish native platform builds`.

### Task 4: Operator one-command Docker smoke

**Files:**
- Create: `scripts/smoke-production-compose.sh`
- Create: `docs/operator/install.md`
- Test: `scripts/tests/compose_ops.sh`

**Interfaces:**
- Consumes: M1 Compose and bootstrap, health/API/media settings.
- Produces: `bash scripts/smoke-production-compose.sh`.

- [ ] **Step 1 — write failing test:** In `scripts/tests/compose_ops.sh`, add `missing_media_udp_or_secret_surfaces_diagnostic`. Invalid secrets fail before startup; unreachable ICE/TURN yields actionable warning while text health works.
- [ ] **Step 2 — verify red:** Run `bash scripts/tests/compose_ops.sh`; expect **FAIL** from missing/incorrect behavior (not merely a test runner/config error).
- [ ] **Step 3 — implement minimum:** Document reverse proxy, ports, CGNAT/TURN, secrets, update/rollback, live/ready endpoints and first invite. Implement the interface above; update adjacent contract docs for interface changes.
- [ ] **Step 4 — verify green:** Run `bash scripts/tests/compose_ops.sh`; expect **PASS**. Then run the package test suite and relevant clippy/format checks.
- [ ] **Step 5 — commit:** Stage `scripts/smoke-production-compose.sh; docs/operator/install.md` and `scripts/tests/compose_ops.sh` (plus necessary docs); commit as `ops: harden docker installation`.

### Task 5: Release quality gate and license audit

**Files:**
- Create: `docs/release-checklist.md`
- Create: `scripts/check-release-gates.sh`
- Create: `docs/operator/dependencies.md`
- Test: `scripts/tests/release_gate.sh`

**Interfaces:**
- Consumes: All milestone acceptance reports and pending ADRs.
- Produces: `bash scripts/check-release-gates.sh --evidence reports/`.

- [ ] **Step 1 — write failing test:** In `scripts/tests/release_gate.sh`, add `release_rejected_if_media_or_license_unverified`. Missing license choice, missing 1440p60 matrix or failing auth negative test causes nonzero exit; complete verified evidence passes.
- [ ] **Step 2 — verify red:** Run `bash scripts/tests/release_gate.sh`; expect **FAIL** from missing/incorrect behavior (not merely a test runner/config error).
- [ ] **Step 3 — implement minimum:** Audit dependencies/native codec terms and the Whisk name (trademark/domain/package IDs); run security/perf tests, document issues, gate public v1 release. Implement the interface above; update adjacent contract docs for interface changes.
- [ ] **Step 4 — verify green:** Run `bash scripts/tests/release_gate.sh`; expect **PASS**. Then run the package test suite and relevant clippy/format checks.
- [ ] **Step 5 — commit:** Stage `docs/release-checklist.md; scripts/check-release-gates.sh; docs/operator/dependencies.md` and `scripts/tests/release_gate.sh` (plus necessary docs); commit as `release: enforce auditable ship gates`.

## Plan self-review and handoff

- [ ] Each stated spec requirement for this milestone maps to a task or a documented later milestone.
- [ ] Each test name/assertion and interface is consistent with the owner task; all five Review Focus items are covered.
- [ ] Paths have been checked against the actual repository after scaffolding; no phantom pre-existing files are assumed.
- [ ] Recheck version-pinned LiveKit/GPUI signatures and OS capture APIs before execution.
- [ ] Get plan review before implementation. Prefer subagent-driven implementation and independent spec/code review when the agent supports it.
