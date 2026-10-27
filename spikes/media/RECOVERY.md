# M0 permission and device recovery checks

[Issue #15](https://github.com/fcendesu/whisk/issues/15) is a verification task.
Generated/injected tests do not establish physical device or macOS permission
recovery. Keep these cases separate when reviewing M0 acceptance (#16).

## Tested and pending cases

| Case | Current evidence | Limit |
|---|---|---|
| Capture denied before enumeration/start | `denied_capture_permission_is_safe` checks the backend is not called | Deterministic backend boundary, not an OS toggle |
| Capture terminal event after native decode starts | `injected_terminal_events_return_capture_errors_after_decode_and_reject_late_frames` injects PermissionDenied, SourceGone, backend error and source end through the live local SFU path; a guaranteed late frame is sent before polling the probe again, and the feeder stays alive | Supplied fixture frames; no screen/device access or physical revocation; returned capture error does not independently prove successful SFU unpublication/room closure |
| Cancel before/during microphone setup | Factory/guard regressions reject starts after cancellation and drop a source created during cancellation | Synchronous native driver calls cannot be preempted |
| Speaker start/acquisition failure and cancellation | Device-boundary regressions disable recording, stop/release acquired playback and reject cancelled startup | Fake device boundary; no physical output removal |
| Frozen grant reused across launches | #13/#14 device/capture runs reused unchanged bundles successfully | Does not prove grants survive a new signing identity |
| Physical capture permission revocation and restore | Pending explicit manual opt-in below | Do not mark passing from injected errors |
| USB/Bluetooth microphone removal and reconnect | Not tested: user reports built-in microphone only | Built-in input cannot be unplugged; silence/mute/default-input fallback is not a removal test |
| Physical microphone permission revoke/restore or hung-driver recovery | Not tested by this protocol | No microphone permission changes are authorized by the screen procedure |

## Concrete manual screen-permission procedure

The test uses only the existing **Whisk Capture Lab** (`dev.whisk.capture-lab`)
and its own animated test window. The current user opt-in is required before
changing this app's grant. Other apps and their grants stay untouched.

1. Launch idle with `./scripts/run-capture-lab.sh --reuse-build 720p30 10`.
   The ten-second flag applies to a published observation; local preview continues
   until Stop, revocation or process exit. Do not rebuild, copy or sign the bundle. Bring **Whisk — Capture Lab** above
   the animated window if it covers the controls.
2. After agreeing to this procedure, refresh sources and explicitly select
   **Whisk capture test window** for local preview. No publication or audio is
   needed. Verify live animation before revocation.
3. The user switches **Whisk Capture Lab** off under System Settings → Privacy &
   Security → Screen & System Audio Recording. If macOS requests **Quit & Reopen**,
   use it; permission changes may require restarting the process.
4. Record whether the active preview ends with an actionable state or macOS
   closes the process. Do not interpret a retained frozen image as live capture.
   Relaunch the same build if needed, refresh sources while denied and verify
   the permission explanation and absence of a running preview.
5. The user switches only **Whisk Capture Lab** back on and restarts it if macOS
   requests that. Reuse the same build, refresh, reselect the owned window and
   verify live updates. Stop preview and close both lab windows afterward.
6. Record actual observed states, any OS-forced quit, bundle/signature hashes and
   capture-stop evidence separately from unknown internals. A new grant or UI
   message alone is not proof of recovery.

This procedure does not remove a physical device or test microphone/speaker
recovery. If the owner keeps the current grant unchanged, record the physical
screen cases as pending rather than resetting permissions automatically.

## Signing and safe development recovery

Existing ad hoc development rebuilds changed executable/signing identity and
previously caused macOS capture denial despite an apparently enabled grant.
User-led app-scoped re-add/re-enable and restart restored the frozen Media Lab;
the dated reports in README record the observed behavior. The root identity/
policy mechanism was not independently isolated, so do not promise every
rebuild will invalidate (or preserve) a grant.

Use the appropriate `--reuse-build` launcher for a granted bundle. It refreshes
short-lived fixture credentials without changing the executable, Info.plist,
resources or signature. Credentials expiring after two minutes are connection
failures, not proof of lost screen permission; frozen-bundle relaunch is enough
for that case. Real permission denial should be checked in System Settings for
the exact app identity, followed by a user-approved app-scoped correction/restart.

Never use a broad TCC reset, edit the TCC database, disable SIP or weaken signing
requirements to make a probe pass. New instrumented code belongs in a separately
identified opt-in lab if the working signed bundle must be preserved. The
currently tested bundles are not a production signing/notarization strategy.

Physical removal/reconnect remains an evidence gap with the available hardware.
#16 must classify that gap explicitly; this document does not approve deferring
an M0 requirement or complete #15 by itself.


## Cleanup observability limit

The current network probe attempts unpublication and room closure before returning
an active capture error, but preserves that primary error before checking cleanup
results. A bounded `Err(Capture)` therefore proves termination of the probe future,
not independently verified removal of every SFU track/participant. A separate
observer or structured cleanup outcome is required for that claim. No physical
recording/playback safety claim should be inferred from this injected network test.
