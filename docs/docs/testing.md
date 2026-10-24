# Whisk — Acceptance criteria, CI, performance and release gates

The plans provide TDD at task level; this document defines cross-component gates for a **user-visible v1**, not just compiling Rust.

## End-to-end suite

| Scenario | Passing observation |
|---|---|
| Fresh Docker host | Running api + LiveKit + reverse proxy; owner bootstrap once; only one community |
| Invite signup | Invite-only by default, expiry/use caps atomic, invalid tokens rejected |
| Three independently hosted instances | One native sidebar; separate accounts/caches/unread badges; one instance outage does not block others |
| Text offline/replay | Message send/edit/delete and role-change replay after 2-minute outage; no duplicate UI messages |
| DM isolation | Third party with known DM ID receives denied/opaque response |
| File safety | 25 MiB accepted; 25 MiB + 1 byte rejected; restricted old URLs denied; no traversal or unsafe inline types |
| Media authorization | Room-scope enforced; banned users denied fresh tokens; existing participants removed and residual token TTL documented |
| Multipublisher media | At least two streams in one voice room; third viewer can switch focused stream without voice dropping |
| Webcam | Camera and voice coexist, stop/unplug/permissions recovered gracefully |
| Role hierarchy | No moderator privilege escalation, overwrite tests and audit enforced at all request paths |
| Persistence | Backup/restore recovers messages, roles, attachments and audit log |
| Storage portability | Local filesystem, RustFS and R2 adapter conformance; copy/digest verification without changing client attachment ID |

## Desktop platform matrix

| Target | UI | Microphone | System audio | Screen/window capture | HW encode | Video renderer | Global PTT |
|---|---|---|---|---|---|---|---|
| Windows | Verify | Verify | Verify | Verify | Verify | Verify | Verify |
| Linux X11 | Verify | Verify | Verify | Verify | Verify | Verify | Verify |
| Linux Wayland | Verify | Verify | Portal/PipeWire | Portal/PipeWire | Verify | Verify | Verify |
| macOS | Verify | Verify | Verify | ScreenCaptureKit | VideoToolbox | Verify | Verify |

**Do not mark unsupported capabilities complete because a backend API exists.** Permission prompts, revoked access, capture device unplugging, graphics-driver compatibility and UI frame stability are part of each check.

## Performance measurement protocol

- Define reference hardware and OS before reporting numbers, including codec and GPU driver.
- Measure 10-minute idle RSS and CPU with one and three instances; aspirational targets: <250 MiB RSS, <1% CPU **on selected reference hardware**, not universal promises.
- Scroll 10k-message paginated history with frame-time traces; target 60 FPS UI responsiveness without loading all records.
- Measure online message create → peer visible p95 in controlled same-region test of >=1000 messages; target <300ms.
- Stream **1080p60 and 1440p60** for 10 minutes on certified hardware, recording delivered/decoded/displayed fps, encode/decode latency, frame drops, CPU/GPU, bandwidth, thermal throttling and capture-to-display latency.
- Exercise two publishers and one subscriber; test switching without a voice interruption. Confirm hardware acceleration actually engaged; don't infer from codec string alone.
- Verify no growing frame queue and no GPUI main-thread blocking I/O.

## Naming and publication gate

Release packaging and visible application labels should consistently identify **Whisk Desktop** and **Whisk Server**. A public launch must separately clear trademarks, domain/namespace availability and platform bundle identifiers. Branding must not silently change instance UUIDs, APIs, event names or permissions.

## Ship/no-ship

No v1 release before M0 proves a viable native streaming pipeline on a reference OS, M7 validates all three desktop OS targets, permissions negative tests pass, and Docker restore tests pass. Known platform exceptions must be clearly disclosed and assessed against feature commitments.
