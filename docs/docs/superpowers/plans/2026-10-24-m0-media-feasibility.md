# Whisk — M0 Native Media Feasibility Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prove that a native GPUI shell can capture, publish and render WebRTC screen video, and record actual 1080p60/1440p60 results on at least one supported OS.

**Architecture:** A disposable Rust probe workspace with a thin capture adapter, a LiveKit room connector and an isolated GPUI frame surface. Collect reproducible timing counters before committing to zero-copy or renderer APIs.

**Tech Stack:** Rust, GPUI pinned revision, LiveKit Rust SDK, Tokio, platform-specific capture, JSON/CSV benchmark output.

**Spec:** `docs/superpowers/specs/2026-10-24-media-design.md` (read with `docs/prd.md`)

**Status:** Planned, not executed. These paths/signatures are proposed; reconcile them with the pinned toolchain and real repository before implementing.

## Global Constraints

- Spike code may be thrown away; preserve reproducible setup notes and testable proof before M1.
- Windows/Linux/macOS remain release goals, but M0 proves at least one reference path first.
- No 1440p60 support claim until 10-minute measured run; report codec, encoder path, frames and latency.
- Use a real self-hosted LiveKit SFU; no hidden Cloud dependency.
- Prefer task-by-task test-first behavior, red → green → refactor → small commit; never mark tests passing without executing them.

## Review Focus

- Permission denied during native screen capture should be explained (Task 2).
- Missing or unsupported encoder should fall back without crashing (Task 3).
- GPU decoder must not create unbounded frame queues (Task 4).
- Second stream must not interrupt first participant audio (Task 5).
- Unreachable UDP/TURN path needs reproducible diagnostic output (Task 1).

---

## File and interface map

| Task | Implementation owner | Test owner |
|---|---|---|
| 1 · LiveKit connection smoke | `spikes/media/src/room.rs` | `spikes/media/tests/room_smoke.rs` |
| 2 · Native screen source discovery | `spikes/media/src/capture.rs` | `spikes/media/tests/capture.rs` |
| 3 · Codec negotiation probe | `spikes/media/src/codec.rs` | `spikes/media/tests/codec.rs` |
| 4 · GPUI frame rendering probe | `spikes/media/src/render.rs` | `spikes/media/tests/render_queue.rs` |
| 5 · End-to-end two-publisher benchmark | `spikes/media/src/benchmark.rs` | `spikes/media/tests/metrics.rs` |

### Task 1: LiveKit connection smoke

**Files:**
- Create: `spikes/media/src/room.rs`
- Test: `spikes/media/tests/room_smoke.rs`

**Interfaces:**
- Consumes: Self-hosted LiveKit URL, room token from test fixture.
- Produces: `async fn connect_probe(url: &str, token: &str) -> Result<ProbeRoom, ProbeError>`.

- [ ] **Step 1 — write failing test:** In `spikes/media/tests/room_smoke.rs`, add `unreachable_media_endpoint_is_reported`. Unavailable SFU yields structured `ProbeError::Connect`, without hanging the UI; valid token joins room.
- [ ] **Step 2 — verify red:** Run `cargo test -p media-spike room_smoke -- --nocapture`; expect **FAIL** from missing/incorrect behavior (not merely a test runner/config error).
- [ ] **Step 3 — implement minimum:** Join/leave test room, record signaling/ICE states and network-error diagnostic; never put API secret in client. Implement the interface above; update adjacent contract docs for interface changes.
- [ ] **Step 4 — verify green:** Run `cargo test -p media-spike room_smoke -- --nocapture`; expect **PASS**. Then run the package test suite and relevant clippy/format checks.
- [ ] **Step 5 — commit:** Stage `spikes/media/src/room.rs` and `spikes/media/tests/room_smoke.rs` (plus necessary docs); commit as `spike: verify native LiveKit join`.

### Task 2: Native screen source discovery

**Files:**
- Create: `spikes/media/src/capture.rs`
- Test: `spikes/media/tests/capture.rs`

**Interfaces:**
- Consumes: Target-OS source-picker API, portal permission prompt where applicable.
- Produces: `fn available_sources() -> Result<Vec<CaptureSource>, CaptureError>; fn open_source(id: SourceId) -> Result<FrameStream, CaptureError>`.

- [ ] **Step 1 — write failing test:** In `spikes/media/tests/capture.rs`, add `denied_capture_permission_is_safe`. Denied permission produces `CaptureError::PermissionDenied`; no blank phantom stream or crash.
- [ ] **Step 2 — verify red:** Run `cargo test -p media-spike denied_capture_permission_is_safe`; expect **FAIL** from missing/incorrect behavior (not merely a test runner/config error).
- [ ] **Step 3 — implement minimum:** Build smallest single-OS frame source and document OS/API/permission prompt; enumerate screen and window where supported. Implement the interface above; update adjacent contract docs for interface changes.
- [ ] **Step 4 — verify green:** Run `cargo test -p media-spike denied_capture_permission_is_safe`; expect **PASS**. Then run the package test suite and relevant clippy/format checks.
- [ ] **Step 5 — commit:** Stage `spikes/media/src/capture.rs` and `spikes/media/tests/capture.rs` (plus necessary docs); commit as `spike: inspect native capture`.

### Task 3: Codec negotiation probe

**Files:**
- Create: `spikes/media/src/codec.rs`
- Test: `spikes/media/tests/codec.rs`

**Interfaces:**
- Consumes: FrameStream from Task 2, LiveKit SDK video track interfaces.
- Produces: `fn select_codec(cap: &MediaCapabilities, requested: VideoProfile) -> Result<EncoderChoice, CodecError>`.

- [ ] **Step 1 — write failing test:** In `spikes/media/tests/codec.rs`, add `unsupported_1440p60_encoder_falls_back`. Absent certified hardware encoder -> explicit 720p30 fallback, not false 1440p60 success.
- [ ] **Step 2 — verify red:** Run `cargo test -p media-spike unsupported_1440p60_encoder_falls_back`; expect **FAIL** from missing/incorrect behavior (not merely a test runner/config error).
- [ ] **Step 3 — implement minimum:** Publish a generated/captured test frame path, log actual software/hardware encoder chosen; H.264 baseline. Implement the interface above; update adjacent contract docs for interface changes.
- [ ] **Step 4 — verify green:** Run `cargo test -p media-spike unsupported_1440p60_encoder_falls_back`; expect **PASS**. Then run the package test suite and relevant clippy/format checks.
- [ ] **Step 5 — commit:** Stage `spikes/media/src/codec.rs` and `spikes/media/tests/codec.rs` (plus necessary docs); commit as `spike: probe codec capabilities`.

### Task 4: GPUI frame rendering probe

**Files:**
- Create: `spikes/media/src/render.rs`
- Test: `spikes/media/tests/render_queue.rs`

**Interfaces:**
- Consumes: Decoded frame callbacks/format from LiveKit probe.
- Produces: `fn enqueue_latest(queue: &mut FrameQueue, frame: DecodedFrame); fn current_frame(queue: &FrameQueue) -> Option<DecodedFrame>`.

- [ ] **Step 1 — write failing test:** In `spikes/media/tests/render_queue.rs`, add `slow_viewer_does_not_accumulate_frames`. Inject 180 frames into depth-2 queue, assert length <= 2 and latest frame wins.
- [ ] **Step 2 — verify red:** Run `cargo test -p media-spike slow_viewer_does_not_accumulate_frames`; expect **FAIL** from missing/incorrect behavior (not merely a test runner/config error).
- [ ] **Step 3 — implement minimum:** Create GPUI view/texture bridge; report copies vs GPU import availability, frame timing, queue bounds. Implement the interface above; update adjacent contract docs for interface changes.
- [ ] **Step 4 — verify green:** Run `cargo test -p media-spike slow_viewer_does_not_accumulate_frames`; expect **PASS**. Then run the package test suite and relevant clippy/format checks.
- [ ] **Step 5 — commit:** Stage `spikes/media/src/render.rs` and `spikes/media/tests/render_queue.rs` (plus necessary docs); commit as `spike: render decoded frames in gpui`.

### Task 5: End-to-end two-publisher benchmark

**Files:**
- Create: `spikes/media/src/benchmark.rs`
- Test: `spikes/media/tests/metrics.rs`

**Interfaces:**
- Consumes: Capture, codec, room and renderer from Tasks 1–4.
- Produces: `fn summarize(samples: &[FrameSample]) -> BenchmarkSummary`.

- [ ] **Step 1 — write failing test:** In `spikes/media/tests/metrics.rs`, add `missing_frame_samples_fails_report`. Report generator rejects missing duration/captured/rendered counts; includes 10-minute run profile.
- [ ] **Step 2 — verify red:** Run `cargo test -p media-spike missing_frame_samples_fails_report`; expect **FAIL** from missing/incorrect behavior (not merely a test runner/config error).
- [ ] **Step 3 — implement minimum:** Two concurrent publishers, one switchable viewer; measure 720p30, 1080p60, 1440p60 and log measured FPS/drop/CPU/GPU/latency, OS/GPU/drivers. Implement the interface above; update adjacent contract docs for interface changes.
- [ ] **Step 4 — verify green:** Run `cargo test -p media-spike missing_frame_samples_fails_report`; expect **PASS**. Then run the package test suite and relevant clippy/format checks.
- [ ] **Step 5 — commit:** Stage `spikes/media/src/benchmark.rs` and `spikes/media/tests/metrics.rs` (plus necessary docs); commit as `spike: measure streaming feasibility`.

## Plan self-review and handoff

- [ ] Each stated spec requirement for this milestone maps to a task or a documented later milestone.
- [ ] Each test name/assertion and interface is consistent with the owner task; all five Review Focus items are covered.
- [ ] Paths have been checked against the actual repository after scaffolding; no phantom pre-existing files are assumed.
- [ ] Recheck version-pinned LiveKit/GPUI signatures and OS capture APIs before execution.
- [ ] Get plan review before implementation. Prefer subagent-driven implementation and independent spec/code review when the agent supports it.
