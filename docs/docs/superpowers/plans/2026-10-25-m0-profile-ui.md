# M0 high-profile native UI probe

Approved M0 Tasks 4/5 require actual GPUI frame handling, not only headless decode.
The current 33 ms polling interval imposes a roughly 30 FPS CPU preparation cap.
Keep existing granted Media/Audio/Speaker bundles unchanged; build Network Lab only.

- [x] RED/GREEN: profile-preview output reaches the bounded frame channel across
  both publisher focus phases; a pre-cancelled preview never connects.
- [x] Add a preview target to the existing experimental two-publisher profile
  probe, with generation cancellation and owned-room cleanup. Default CLI behavior
  remains unchanged; all quality certification flags remain false.
- [x] Wire an explicit experimental profile runner to Network Lab, including
  distinct publisher tokens and configurable bounded phase duration. Poll UI at
  8 ms to remove the artificial 30 FPS cap; do not claim GPU presentation counts.
- [x] Verify deterministic/live tests, lint/format/shell, scoped review, actual
  1080p60-requested native UI run and matching counter exports. Higher-profile
  actual screen capture and release certification remain separate pending gates.

Completed 2026-10-25: CPU preparation25.71→53.90fps in matched30sec attempts. GPU presentation and high-profile capture certification remain unknown. One relay connection timeout was not reproduced; original log retained.
