# Whisk — Realtime Voice, Game Streaming and Webcam — Design

**Scope:** audio processing, LiveKit room auth, OS capture, 1080p60/1440p60, multiple screen publishers. References: `../../prd.md` §4.3, `../../architecture.md`, `../../testing.md`.

## Runtime topology

The Rust server checks channel and media bits, then issues a short-lived room-scoped LiveKit token. The Rust native client uses LiveKit Rust SDK for WebRTC; OS capture APIs and codec availability determine actual publishing capability. GPUI has an independent video-frame renderer bridge with bounded queues; never block UI on decode. LiveKit forwards compressed media as an SFU and is not a video transcoding service by default.

## Voice

One active voice channel globally per native client, with mute/deafen, push-to-talk, VAD, per-user gain, device switch, WebRTC AEC/AGC and optionally RNNoise as a selectable mode after tests. Echo cancellation requires a playback reference; avoid combining duplicate noise filters by default.

## Video / screenshare

At least two users can publish their own screen stream concurrently in the same channel. A viewer can focus either stream while voice continues. Plan for 720p30 fallback and verified 1080p60/1440p60 hardware modes, H.264 baseline and negotiated optional newer codecs. Target efficient capture→encode→SFU→decode→GPUI, with measured frame drops and latency. Webcam publishes a separate track in a voice channel. System/app audio capture must be capability-checked by OS.

## High-risk integration gates

Windows Graphics Capture/WASAPI, macOS ScreenCaptureKit/VideoToolbox, Linux X11 and Wayland portals/PipeWire have different permission/device semantics. LiveKit native track publishing does not by itself prove GPU zero-copy or a 1440p60 working pipeline. M0 must benchmark actual GPUs, SDK revision and GPUI texture handling; fallback is visible and safe.

## Revocation boundary

Self-hosted LiveKit room tokens are not immediately invalidated when a participant is removed. Use short token lifetime, backend deny on fresh join, removal/permissions APIs and documented residual rejoin window; verify exact pinned release behavior. Do not advertise instantaneous permanent revocation or application E2EE.

## Alternatives

Rust-only SFU would impose protocol burden; software-only 1440p60 encoding is not a reasonable general default. Hardware codec support is platform-dependent and must be measured.
