# M0 physical speaker probe

Completes the physical-playback evidence slice of the approved media spike.
Preserve existing Media Lab and Audio Lab bundles and permissions.

- [x] RED/GREEN: speaker-only input rejects durations above three seconds and
  cancellation before hardware access. Playback confirmation requires generated
  speaker mode, started playout, decoded non-silent frames and an explicit human
  yes; no sound records false, and missing feedback stays unknown.
- [x] Implement a native speaker guard that disables microphone recording before
  acquiring the platform ADM, uses manual generated PCM, and stops/releases output
  before network cleanup. Keep generated-only automated tests silent.
- [x] Add a separately compiled Speaker Lab using the shared audio UI, with Play,
  Stop and explicit heard/no-sound feedback. Launch idle with a unique local room;
  package only the new speaker bundle, without a microphone usage declaration.
- [x] Verify deterministic/Python/live tests, lint/format/shell, native idle UI,
  unchanged bundle manifests, and scoped independent review. Physical playback
  remains unverified until a user starts the tone and confirms hearing it.

Completed 2026-10-25: 300-frame Opus playback record, explicit human heard confirmation saved. Existing signed bundles unchanged. Full M0 remains incomplete.
