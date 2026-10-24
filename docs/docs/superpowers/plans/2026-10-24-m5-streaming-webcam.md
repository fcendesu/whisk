# Whisk — M5 Screen Streaming and Webcam Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Support multiple concurrent publishers in one voice channel, selectable stream focus, webcams and hardware-certified 1080p60/1440p60.

**Architecture:** Use platform-specific capture sources wrapped by one Rust trait, LiveKit video tracks and a bounded GPUI decoder-frame renderer. Keep voice and video states independent.

**Tech Stack:** GPUI, LiveKit Rust SDK, Windows Graphics Capture, macOS ScreenCaptureKit, Linux PipeWire/X11, hardware codecs by tested capability.

**Spec:** `docs/superpowers/specs/2026-10-24-media-design.md` (read with `docs/prd.md`)

**Status:** Planned, not executed. These paths/signatures are proposed; reconcile them with the pinned toolchain and real repository before implementing.

## Global Constraints

- At least two concurrent screen publishers; viewer can select stream without voice disconnect.
- 1440p60 and 1080p60 are supported only on certified OS/GPU driver matrices; fallback shown clearly.
- Only one focused high-resolution video required at a time; background decode is throttled.
- System/application audio capture is OS capability-dependent and must disclose unsupported cases.
- Prefer task-by-task test-first behavior, red → green → refactor → small commit; never mark tests passing without executing them.

## Review Focus

- Denied screen portal permission returns clear UI error (Task 1).
- Unsupported codec profile degrades to 720p30 not crash (Task 2).
- Slow viewer must not grow frame queue (Task 3).
- Changing focused publisher must not drop voice (Task 4).
- Unplugged camera and missing system audio capability must produce usable fallback (Tasks 5–6).

---

## File and interface map

| Task | Implementation owner | Test owner |
|---|---|---|
| 1 · Native screen/window sources | `crates/media-engine/src/capture/mod.rs; crates/media-engine/src/capture/windows.rs; crates/media-engine/src/capture/macos.rs; crates/media-engine/src/capture/linux.rs` | `crates/media-engine/tests/capture_permissions.rs` |
| 2 · Codec capabilities and publishing | `crates/media-engine/src/video_profiles.rs; crates/media-engine/src/publisher.rs` | `crates/media-engine/tests/profiles.rs` |
| 3 · GPUI decoded frame surface | `crates/gpui-video/src/renderer.rs; crates/gpui-video/src/queue.rs` | `crates/gpui-video/tests/backpressure.rs` |
| 4 · Multi-stream selection and voice isolation | `crates/media-engine/src/subscriber.rs; apps/desktop/src/ui/stream_picker.rs` | `crates/media-engine/tests/multi_stream.rs` |
| 5 · Camera publish/unpublish controls | `crates/media-engine/src/camera.rs; apps/desktop/src/ui/camera_tile.rs` | `crates/media-engine/tests/camera_state.rs` |
| 6 · Application audio capability and benchmark | `crates/media-engine/src/system_audio.rs; crates/media-engine/src/metrics.rs` | `crates/media-engine/tests/audio_capability.rs` |

### Task 1: Native screen/window sources

**Files:**
- Create: `crates/media-engine/src/capture/mod.rs`
- Create: `crates/media-engine/src/capture/windows.rs`
- Create: `crates/media-engine/src/capture/macos.rs`
- Create: `crates/media-engine/src/capture/linux.rs`
- Test: `crates/media-engine/tests/capture_permissions.rs`

**Interfaces:**
- Consumes: Verified M0 OS capture adapters and permission semantics.
- Produces: `trait ScreenCapturer { async fn enumerate(&self) -> Result<Vec<CaptureSource>, CaptureError>; async fn start(&self, source: SourceId) -> Result<VideoFrames, CaptureError>; }`.

- [ ] **Step 1 — write failing test:** In `crates/media-engine/tests/capture_permissions.rs`, add `denied_capture_permission_reports_error`. Deny Wayland/macOS/Windows capture permission -> surfaced PermissionDenied state, no fake live stream.
- [ ] **Step 2 — verify red:** Run `cargo test -p media-engine denied_capture_permission_reports_error`; expect **FAIL** from missing/incorrect behavior (not merely a test runner/config error).
- [ ] **Step 3 — implement minimum:** Implement per-OS backends and permission/source picker; add source removal recovery. Implement the interface above; update adjacent contract docs for interface changes.
- [ ] **Step 4 — verify green:** Run `cargo test -p media-engine denied_capture_permission_reports_error`; expect **PASS**. Then run the package test suite and relevant clippy/format checks.
- [ ] **Step 5 — commit:** Stage `crates/media-engine/src/capture/mod.rs; crates/media-engine/src/capture/windows.rs; crates/media-engine/src/capture/macos.rs; crates/media-engine/src/capture/linux.rs` and `crates/media-engine/tests/capture_permissions.rs` (plus necessary docs); commit as `feat: add native capture abstraction`.

### Task 2: Codec capabilities and publishing

**Files:**
- Create: `crates/media-engine/src/video_profiles.rs`
- Create: `crates/media-engine/src/publisher.rs`
- Test: `crates/media-engine/tests/profiles.rs`

**Interfaces:**
- Consumes: Capture source + LiveKit media connection, M0 codec findings.
- Produces: `fn negotiate_profile(requested: VideoProfile, hw: &HardwareCapabilities, peer: &PeerCapabilities) -> NegotiatedProfile`.

- [ ] **Step 1 — write failing test:** In `crates/media-engine/tests/profiles.rs`, add `unavailable_1440p60_falls_back_explicitly`. No capable certified encoder -> negotiated mode 720p30 with reason string; otherwise true 1440p60 requested.
- [ ] **Step 2 — verify red:** Run `cargo test -p media-engine unavailable_1440p60_falls_back_explicitly`; expect **FAIL** from missing/incorrect behavior (not merely a test runner/config error).
- [ ] **Step 3 — implement minimum:** Encode/publish H.264 track, device codec selection, adaptive bitrate, avoid extra frame copies where supported. Implement the interface above; update adjacent contract docs for interface changes.
- [ ] **Step 4 — verify green:** Run `cargo test -p media-engine unavailable_1440p60_falls_back_explicitly`; expect **PASS**. Then run the package test suite and relevant clippy/format checks.
- [ ] **Step 5 — commit:** Stage `crates/media-engine/src/video_profiles.rs; crates/media-engine/src/publisher.rs` and `crates/media-engine/tests/profiles.rs` (plus necessary docs); commit as `feat: add adaptive screen track publishing`.

### Task 3: GPUI decoded frame surface

**Files:**
- Create: `crates/gpui-video/src/renderer.rs`
- Create: `crates/gpui-video/src/queue.rs`
- Test: `crates/gpui-video/tests/backpressure.rs`

**Interfaces:**
- Consumes: Decoded frames + M0 texture-copy measurements.
- Produces: `fn submit_frame(&mut self, frame: VideoFrame); fn take_latest(&mut self) -> Option<VideoFrame>`.

- [ ] **Step 1 — write failing test:** In `crates/gpui-video/tests/backpressure.rs`, add `frame_queue_bounded_during_slow_render`. Feed frames 4x faster than paint; queue stays bounded and favors latest frame.
- [ ] **Step 2 — verify red:** Run `cargo test -p gpui-video frame_queue_bounded_during_slow_render`; expect **FAIL** from missing/incorrect behavior (not merely a test runner/config error).
- [ ] **Step 3 — implement minimum:** Use dedicated media thread, GPUI invalidation and measured GPU/CPU copies; support fullscreen/resize. Implement the interface above; update adjacent contract docs for interface changes.
- [ ] **Step 4 — verify green:** Run `cargo test -p gpui-video frame_queue_bounded_during_slow_render`; expect **PASS**. Then run the package test suite and relevant clippy/format checks.
- [ ] **Step 5 — commit:** Stage `crates/gpui-video/src/renderer.rs; crates/gpui-video/src/queue.rs` and `crates/gpui-video/tests/backpressure.rs` (plus necessary docs); commit as `feat: render live video without frame backlog`.

### Task 4: Multi-stream selection and voice isolation

**Files:**
- Create: `crates/media-engine/src/subscriber.rs`
- Create: `apps/desktop/src/ui/stream_picker.rs`
- Test: `crates/media-engine/tests/multi_stream.rs`

**Interfaces:**
- Consumes: Multiple room video tracks and bounded renderer.
- Produces: `fn select_stream(&mut self, publisher: UserId, track: TrackId) -> Result<(), StreamError>`.

- [ ] **Step 1 — write failing test:** In `crates/media-engine/tests/multi_stream.rs`, add `switching_video_stream_preserves_voice`. Two screen publishers + one viewer; switch selected track and assert voice room/audio track unchanged.
- [ ] **Step 2 — verify red:** Run `cargo test -p media-engine switching_video_stream_preserves_voice`; expect **FAIL** from missing/incorrect behavior (not merely a test runner/config error).
- [ ] **Step 3 — implement minimum:** Maintain publisher list, single focused stream high quality, paused/low-quality background subscriptions. Implement the interface above; update adjacent contract docs for interface changes.
- [ ] **Step 4 — verify green:** Run `cargo test -p media-engine switching_video_stream_preserves_voice`; expect **PASS**. Then run the package test suite and relevant clippy/format checks.
- [ ] **Step 5 — commit:** Stage `crates/media-engine/src/subscriber.rs; apps/desktop/src/ui/stream_picker.rs` and `crates/media-engine/tests/multi_stream.rs` (plus necessary docs); commit as `feat: support concurrent streams and selection`.

### Task 5: Camera publish/unpublish controls

**Files:**
- Create: `crates/media-engine/src/camera.rs`
- Create: `apps/desktop/src/ui/camera_tile.rs`
- Test: `crates/media-engine/tests/camera_state.rs`

**Interfaces:**
- Consumes: Room connection and available camera devices.
- Produces: `async fn publish_camera(&mut self, device: CameraId) -> Result<CameraTrack, CameraError>`.

- [ ] **Step 1 — write failing test:** In `crates/media-engine/tests/camera_state.rs`, add `unplugged_camera_unpublishes_cleanly`. Removing active camera emits ended state and lets voice continue; no GPUI panic.
- [ ] **Step 2 — verify red:** Run `cargo test -p media-engine unplugged_camera_unpublishes_cleanly`; expect **FAIL** from missing/incorrect behavior (not merely a test runner/config error).
- [ ] **Step 3 — implement minimum:** Capture camera alongside mic/screen, publish as independent track, render responsive tiles. Implement the interface above; update adjacent contract docs for interface changes.
- [ ] **Step 4 — verify green:** Run `cargo test -p media-engine unplugged_camera_unpublishes_cleanly`; expect **PASS**. Then run the package test suite and relevant clippy/format checks.
- [ ] **Step 5 — commit:** Stage `crates/media-engine/src/camera.rs; apps/desktop/src/ui/camera_tile.rs` and `crates/media-engine/tests/camera_state.rs` (plus necessary docs); commit as `feat: add webcam in voice channels`.

### Task 6: Application audio capability and benchmark

**Files:**
- Create: `crates/media-engine/src/system_audio.rs`
- Create: `crates/media-engine/src/metrics.rs`
- Test: `crates/media-engine/tests/audio_capability.rs`

**Interfaces:**
- Consumes: OS loopback APIs and video metrics from M0.
- Produces: `fn system_audio_capability(platform: PlatformInfo) -> AudioCaptureCapability; fn summarize_stream(samples: &[FrameSample]) -> StreamReport`.

- [ ] **Step 1 — write failing test:** In `crates/media-engine/tests/audio_capability.rs`, add `unsupported_audio_capture_is_visible`. No available OS loopback returns Unsupported with displayed explanation; report includes FPS/drop/latency metadata.
- [ ] **Step 2 — verify red:** Run `cargo test -p media-engine unsupported_audio_capture_is_visible`; expect **FAIL** from missing/incorrect behavior (not merely a test runner/config error).
- [ ] **Step 3 — implement minimum:** Implement audio-source negotiation where available and create 10-minute 1080p60/1440p60 test report format per OS/GPU. Implement the interface above; update adjacent contract docs for interface changes.
- [ ] **Step 4 — verify green:** Run `cargo test -p media-engine unsupported_audio_capture_is_visible`; expect **PASS**. Then run the package test suite and relevant clippy/format checks.
- [ ] **Step 5 — commit:** Stage `crates/media-engine/src/system_audio.rs; crates/media-engine/src/metrics.rs` and `crates/media-engine/tests/audio_capability.rs` (plus necessary docs); commit as `feat: expose capture capabilities and streaming telemetry`.

## Plan self-review and handoff

- [ ] Each stated spec requirement for this milestone maps to a task or a documented later milestone.
- [ ] Each test name/assertion and interface is consistent with the owner task; all five Review Focus items are covered.
- [ ] Paths have been checked against the actual repository after scaffolding; no phantom pre-existing files are assumed.
- [ ] Recheck version-pinned LiveKit/GPUI signatures and OS capture APIs before execution.
- [ ] Get plan review before implementation. Prefer subagent-driven implementation and independent spec/code review when the agent supports it.
