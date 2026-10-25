# Whisk M0 microphone feasibility probe

Extends the approved M0 spike's PRD requirement for physical microphone audio;
M0 remains incomplete until real device evidence is collected. Existing screen
capture bundle must stay byte-for-byte unchanged.

## Approach

Use pinned LiveKit 0.9.7 PlatformAudio/CoreAudio in a separate Whisk Audio Lab
bundle with its own microphone purpose string. An explicit Start button runs a
bounded ten-second publish/decode test in a unique loopback fixture room. Stop
cancels it; microphone recording stops before asynchronous network cleanup.
Disable platform speaker playout to avoid feeding the microphone back to speakers.
Do not persist PCM, device identifiers, tokens, participant IDs or source labels.
A generated-input integration test verifies the common pipeline without accessing
physical devices; it must never claim microphone verification.

## Task 1: measured audio pipeline

Files: spikes/media/src/microphone.rs, tests/microphone.rs, src/lib.rs.
Interface: run_audio_probe(url, publisher, viewer, duration, input, cancelled).

- [x] RED: invalid/pre-cancelled probes must return before accessing native devices;
  generated Opus must cross the real SFU and remain marked generated.
- [x] Implement bounded joins, matching publication subscription, default device
  capture, PCM counter aggregation, cancellation and microphone-first cleanup.
- [x] GREEN: deterministic and explicit local integration tests; no physical access.

## Task 2: explicit native Audio Lab

Files: spikes/media/src/audio_desktop.rs, Cargo.toml, scripts/run-audio-lab.sh,
fixture/token.py and Python tests.

- [x] RED: operator-specified room must appear in token; invalid room rejected.
- [x] Add private room token generation and a separate GPUI app with Start/Stop,
  clear publishing state and sanitized metrics export acknowledgment.
- [x] Build/sign only Whisk Audio Lab; open idle without requesting microphone.
  Verify invalid endpoint and cancellation, then collect a user-started device test.

## Task 3: verification and record

- [x] Full deterministic Rust/Python tests and live SFU tests, fmt/clippy/shell checks.
- [x] Fresh-context scoped code review; fix important findings with red/green tests.
- [x] Preserve screen-capture bundle hashes; document physical mic/playback unknown
  until exercised. Record real device evidence only after user starts/allows test.

Completed 2026-10-25: device report saved; review cancellation finding fixed with red/green tests. Speaker playback and full M0 remain incomplete.
