# M0 native media probe

This disposable spike implements the connection and local capture/preview slices of the
[M0 feasibility plan](../../docs/docs/superpowers/plans/2026-10-24-m0-media-feasibility.md).
It uses the real native LiveKit SDK, without a browser or cloud account.

## Native preview window

On macOS, run `./scripts/run-media-lab.sh` from the repository root. The script
builds the optional `desktop` feature, creates an ad hoc signed development app
in `target/Whisk Media Lab.app`, and opens it. The icon is the supplied Whisk
asset. This is a media feasibility shell, not the messaging application.

- **Test video surface** renders a local BGRA test pattern through GPUI.
- **Refresh sources** checks existing screen-recording access, then discovers
  displays and titled windows on a background worker.
- Select a source to request a 1280×720, 30 FPS local preview.
- **Publish selected window** explicitly sends only the selected window to the
  configured SFU, decodes it as a second participant, and stops after 30 seconds.
  Display publishing is not enabled in this probe. Use the capture-network launch
  command below to supply the local fixture credentials.
- **Stop preview** clears the image and invalidates pending starts. A full command
  queue cannot discard the cancellation signal; the worker also checks generation.

Permission checks never request access automatically. If access is not granted,
the window shows System Settings instructions. Granting it is a manual operator
step; restart the app if macOS requires it. Ad hoc development rebuilds can change
the code identity and require manually enabling capture again. In a later
relaunch here, preflight returned denied and the UI safely showed instructions;
permission persistence is not verified. `codesign -dr -` confirms this bundle's
implicit requirement is a code hash, and no valid development signing identity
is currently configured. Stable certificate-based signing remains packaging work;
no keychain credentials or privacy settings were changed. See Apple's
[code identity explanation](https://developer.apple.com/library/archive/technotes/tn2206/).
Capture callbacks keep only the latest
frame; the native queue and renderer queue are each bounded to two. BGRA row
padding is removed before CPU upload; retired GPU images are released after the
current window update. There is no zero-copy claim.

Pinned integration: ScreenCaptureKit Rust bindings 11.1.0 and GPUI 0.2.2. GPUI
uses `runtime_shaders` so Command Line Tools can compile without the full Xcode
Metal compiler. `build.rs` resolves Swift compatibility libraries from the active
`swiftc` toolchain and adds its runtime search paths. This development app is not
signed/notarized for distribution and remains dependent on the build toolchain;
release packaging is still M7 work. Native API waits run on a worker and may take
up to the bindings' default 30-second completion deadline.

## Native network-video test

```sh
# Open a separate network test app without changing the screen-capture app bundle
./scripts/run-media-lab.sh --network
```

This starts the pinned loopback SFU, generates separate two-minute publish/view
room tokens, and launches `target/Whisk Network Lab.app` with those credentials
in its environment. Click **Test network video**. For 30 seconds, one native
participant publishes a generated 1280×720 BGRA pattern as H.264 and a second
participant decodes it into the bounded GPUI frame surface. No captured screen
is published. Stop invalidates the session and closes its media connections.
If the fixture tokens expire before another test, close and relaunch with the
script to issue fresh tokens. The fixture is left running; teardown is explicit
with the Compose command below.

For real captured-window publishing, close the existing Media Lab first, then run:

```sh
./scripts/run-media-lab.sh --capture-network
```

This rebuilds the same `Whisk Media Lab.app` identity used for manually granted
screen access and supplies the loopback fixture credentials. Refresh sources,
select a window for local preview, then click **Publish selected window**. Selecting
alone never publishes. Stop, source replacement, app shutdown, capture failure,
and the 30-second deadline end the session and release the native capture stream.
Tokens are minted after compilation; relaunch if the two-minute join window has
expired. No microphone or system audio is captured by this path.

A shorter CLI run produces a JSON measurement record:

```sh
export LIVEKIT_URL=ws://127.0.0.1:7880
export LIVEKIT_PUBLISH_TOKEN="$(python3 spikes/media/fixture/token.py --publisher)"
export LIVEKIT_TOKEN="$(python3 spikes/media/fixture/token.py)"
cargo run --locked -p media-spike -- --loopback
unset LIVEKIT_PUBLISH_TOKEN LIVEKIT_TOKEN
```

The decoder matches this run's publication SID, so unrelated room publishers
cannot supply the test's frames. Reports count application-submitted and decoded
frames separately, exclude the SDK's black keepalive from generated-pattern proof, and read
codec/encoder/decoder implementation from actual WebRTC stats. Implementation
strings such as VideoToolbox identify the reported backend; they are not a
certification of hardware acceleration or a high-quality profile. Empty stats
remain unknown. CPU copies convert BGRA → I420 → encoded video → decoded BGRA.

The codec-policy tests require exact profile certification for high-quality
selections and otherwise choose 720p30; the current network probe itself only
attempts 720p30. Policy tests alone do not prove encoder availability or certify
1080p60/1440p60.

## Connection probe

Requirements: Rust 1.99.0, Apple Command Line Tools on macOS, Python 3, and
running Docker with Compose. macOS arm64 is the only verified build so far.

From the repository root:

```sh
docker compose -f spikes/media/fixture/compose.yaml up -d
export LIVEKIT_URL=ws://127.0.0.1:7880
export LIVEKIT_TOKEN="$(python3 spikes/media/fixture/token.py)"
cargo run --locked -p media-spike
unset LIVEKIT_TOKEN
```

The operator-side Python fixture issues a two-minute JWT with a unique identity,
restricted to joining/subscribing in `whisk-m0-smoke`. It contains the public
`devkey`/`secret` pair used by LiveKit development mode; these are disposable test
credentials. The native probe only receives a room token, never an API secret.
The SFU image is pinned by version and digest. Its host ports bind to loopback,
with the advertised node IP set to `127.0.0.1` for this local-only fixture.
Do not reuse this deployment or its credentials outside local development.
Remote deployments need TLS and proper operator-managed credentials.

Successful output is one JSON report with SDK, OS, architecture, join timing,
bounded connection-state history and `media_verified: false`. The probe observes
room events for two seconds and closes the room. Connect is bounded to ten
seconds with SDK join retries disabled; close is bounded to five seconds.
Failures exit nonzero with a JSON `invalid_input`, `connect`, or `close` error;
connection kinds distinguish timeout, rejected credentials, and transport failure.
Raw SDK errors, tokens and endpoint URLs are excluded from diagnostics.

```sh
# Network-failure and CLI regression tests (no SFU needed)
cargo test --workspace --locked

# Real SFU join/leave and authentication rejection (refresh token first)
export LIVEKIT_URL=ws://127.0.0.1:7880
export LIVEKIT_TOKEN="$(python3 spikes/media/fixture/token.py)"
cargo test --locked -p media-spike --test live_room -- --ignored --nocapture
unset LIVEKIT_TOKEN

# Tear down the dedicated fixture
docker compose -f spikes/media/fixture/compose.yaml down
```

The two live tests are ignored by default and fail if explicitly invoked without
the required environment. Use `--test room_smoke` for the deterministic connection
tests: the original plan's `room_smoke` function filter selects zero tests.

## Evidence — 2026-10-24

Verified locally on macOS arm64 with Rust 1.99.0, LiveKit SDK 0.9.7,
webrtc-sys 0.3.51, and the Docker-hosted LiveKit server 1.13.10:

- Native SDK builds and links using Command Line Tools and `-ObjC`.
- Five deterministic tests pass; their initial stubs failed before implementation.
- Two explicitly invoked live tests pass: real room join/leave and rejected JWT.
- [Recorded CLI report](results/2026-10-24-macos-arm64-connection.json) records a
  connected → disconnected transition (199 ms local join; not a media benchmark).

The SDK's public room events expose aggregate connection state. This probe does
not expose raw signaling/ICE candidate-pair telemetry and does not prove that UDP
or TURN is usable; closed-port tests exercise signaling failures only. A joined
room also does not prove working audio/video.

## Additional native-preview evidence — 2026-10-24

- ScreenCaptureKit and GPUI build/link on macOS arm64 using Command Line Tools.
- Ten deterministic tests pass, including permission gating, padded native rows,
  invalid frame bounds, terminal errors surviving late callbacks, and a slow
  viewer consuming 180 frames with a queue depth of at most two.
- The packaged app opens; clicking the test button renders the GPU surface and
  Stop clears it. Refresh reports this machine's ungranted screen access.
- Capture access was subsequently granted manually; see captured-window evidence below.
- Independent review found an active-window image cleanup issue; cleanup now
  defers until GPUI restores the window. Pattern/Stop cycling was checked manually;
  there is no automated GPU-allocation regression test.
- Current upstream `block` and `proc-macro-error2` versions emit Rust future-
  compatibility warnings. They do not fail current checks; resolve before release.

## Network-video evidence — 2026-10-24

- Twenty-one deterministic tests pass; seven explicitly invoked live tests pass.
- [Three-second native loopback report](results/2026-10-24-macos-arm64-h264-loopback.json):
  88 submitted / 88 decoded frames, 1280×720, H.264, VideoToolbox encoder/decoder.
- The actual network test window rendered the decoded warm-orange pattern and
  completed its 30-second run with 898 decoded frames. This is a decode counter,
  not a measured count of GPU-presented frames. No camera, microphone or captured
  screen was transmitted.
- Live regressions cover publication isolation with another publisher present,
  delivery to the preview handoff, and cancellation during an active session.
- Review also found an asynchronous completion race; the terminal result now
  travels through the frame channel and survives sender closure. Its regression
  test failed before the repair, then passed.

Run all thirteen opt-in SFU tests with `./scripts/test-native-media.sh`. This operator
script issues fresh local fixture tokens without printing them; tests sharing
identities serialize. Tests still require the explicitly started local Docker SFU
and do not contact a cloud account. Stop the fixture when finished.

The benchmark-counter validator additionally rejects absent timestamps/counts,
reset or inconsistent counters, zero-duration windows, and empty capture runs.
It calculates rates/drops from the measured window and marks durations shorter
than ten minutes as insufficient. This helper is not connected to hardware
CPU/GPU/latency measurements or GPUI presentation counters yet; its duration flag
alone is not a capability certification.

## Two-publisher focus probe

```sh
export LIVEKIT_URL=ws://127.0.0.1:7880
export LIVEKIT_PUBLISH_TOKEN="$(python3 spikes/media/fixture/token.py --publisher)"
export LIVEKIT_SECOND_PUBLISH_TOKEN="$(python3 spikes/media/fixture/token.py --publisher)"
export LIVEKIT_TOKEN="$(python3 spikes/media/fixture/token.py)"
cargo run --locked -p media-spike -- --switch
unset LIVEKIT_PUBLISH_TOKEN LIVEKIT_SECOND_PUBLISH_TOKEN LIVEKIT_TOKEN
```

Two independently authenticated native publishers keep sending distinct generated
720p30 patterns for both phases. The viewer consumes one native stream, closes
its sink, and switches to the other. Publication IDs and decoded color checks
verify focus isolation. This short software probe is not the full ten-minute
multi-stream hardware/audio benchmark.

The [recorded four-second run](results/2026-10-24-macos-arm64-two-publishers.json)
submitted 120 frames from each publisher and decoded 54 / 60 frames in the
respective focus windows, with zero wrong-source frames. Received counts include
only the focused stream, so subtracting them from both publishers' combined
submission count would not be a valid drop-rate calculation. Audio and quality
certification flags remain false. Both SFU subscriptions stay active in this M0
probe; background subscription/layer adaptation remains future M5 work.

## Remaining M0 gates

Window capture, H.264 publish/decode, GPUI CPU preparation, physical microphone
publish/decode, generated audio continuity during video switching, UDP TURN and
controlled network failures have measured evidence below. M0 is incomplete:

- Microphone continuity with actual video focus switching still needs device
  evidence. Separate microphone and user-confirmed speaker paths now work.
- Permission revocation, device removal/recovery and signing persistence remain
  unverified. The unchanged screen bundle now works after an app-scoped reset.
- Sustained actual-capture-to-GPUI behavior at higher profiles is not certified.
  Ten-minute 1080/1440 records use generated video and native decode counters.
- GPU presentation, per-process GPU/media-engine use, capture-to-display latency
  and zero-copy remain unknown. CPU preparation and whole-system AGX counters
  are reported under their actual scope.
- Dependency/license and packaged-runtime audits remain required before release.

No high-profile release certification, GPU zero-copy, production deployment or
cross-platform media support is claimed. M1 remains gated on M0 evidence.

Upstream references: [pinned SDK source](https://docs.rs/crate/livekit/0.9.7/source/),
[macOS build requirements](https://github.com/livekit/rust-sdks#macos), and
[server release](https://github.com/livekit/livekit/releases/tag/v1.13.10).

## Captured-window evidence — 2026-10-24

- The user enabled screen recording access. Native source discovery and local
  capture of a Whisk test window now work. Changing that window updates the preview.
- [Thirty-second captured-window report](results/2026-10-24-macos-arm64-captured-window.json):
  869 submitted frames, 870 decoder outputs, 1280×720, H.264 with reported
  VideoToolbox encoder/decoder. The GPUI view visibly showed the test window, then
  its change from a checkerboard to an idle screen after the SFU round trip.
- Captured content can legitimately be black, so capture-mode decoder counts
  include SDK keepalive outputs. Counts are not a one-to-one delivery/drop metric
  or GPU presentation counter. No screenshot, title, token or endpoint is saved
  in the report. No captured screen was sent outside the local fixture.
- A live regression supplies blue capture frames and verifies blue decoded pixels;
  substituting the synthetic orange source fails. A deterministic regression
  rejects an already failed capture before connecting. Generation cancellation
  and terminal frame-channel handling are shared with the network preview.
- Permission revocation, microphone/audio continuity, raw ICE/UDP/TURN diagnostics,
  sustained CPU/GPU/latency measurements and high-profile certification remain open.

## Generated audio, profile attempts and timing diagnostics

The `--switch` probe now publishes a generated 440 Hz mono PCM tone from publisher
one and keeps its native audio decoder across both viewer focus phases. Audio
feeding and observation run on a separate worker from CPU video conversion. The
report counts non-silent decoded frames in each phase, checks tone frequency,
and includes startup, inter-frame and trailing silence in a 250 ms continuity
budget. `audio_verified` refers only to this generated PCM receive test;
`microphone_verified` remains false. It does not test device capture or speaker
playback and does not request microphone access.

Experimental profile measurements explicitly bypass release profile selection
only to collect evidence. They never add certified profiles to `MediaCapabilities`:

```sh
# Two five-minute phases: ten minutes of native video observation plus setup.
./scripts/benchmark-media.sh 720p30 300 target/media-benchmark/720p30
./scripts/benchmark-media.sh 1080p60 300 target/media-benchmark/1080p60
./scripts/benchmark-media.sh 1440p60 300 target/media-benchmark/1440p60
```

The macOS operator script starts the local fixture and writes `measurement.json`,
`resources.txt` and `environment.json`. `/usr/bin/time -l` measures the native
three-participant client process, including setup; Docker/SFU resources are
excluded. This is a dev build under uncontrolled machine load. It is not a release
performance certification. Existing output directories containing measurements
are rejected rather than overwritten; the dedicated SFU remains running until
explicit teardown.

Each generated video frame carries a small 512×8 pixel optical timestamp strip.
A decoded frame's monotonic age includes CPU conversion, encode, SFU transport,
decode and application observation, with millisecond precision. Magic/checksum
validation rejects damaged marks and future timestamps. Valid old samples remain
in the distribution. Nearest-rank p50/p95/max summarize up to 100,000 samples;
no samples produces unknown values. This excludes GPUI rendering, GPU completion,
monitor presentation and separate-machine clock synchronization.

Publisher and viewer ICE transports are labeled separately. Diagnostics follow
`selectedCandidatePairId` and retain only allowlisted protocol, candidate types,
state, nomination, RTT and aggregate byte counters. Candidate addresses, ports,
URLs and username fragments are not retained. Missing stats stay unknown. Local
UDP success is not a TURN or blocked-network test; controlled UDP/TURN failure
fixtures remain pending.

### Preliminary sustained 1440p60 result

The [initial ten-minute measurement](results/2026-10-24-macos-arm64-1440p60-initial.json)
used the earlier shared audio/video loop, constant-color frames, and no optical
clock. Actual decoded size was 2560×1440, with VideoToolbox reported for both
encoders and decoders. Viewer phases averaged 59.7 and 38.3 FPS; the largest
observed audio gap was 304 ms, failing the continuity budget. The native process
peaked at approximately 232 MiB resident memory. See the
[resource record](results/2026-10-24-macos-arm64-1440p60-initial-resources.txt) and
[measurement conditions](results/2026-10-24-macos-arm64-1440p60-initial-environment.json).

This does not establish sustained 1440p60 support. It motivated separating the
audio worker from video conversion. High-quality release profiles remain
uncertified, and product selection continues to fall back to 720p30. Full M0
still needs sustained corrected-worker/profile results, actual capture-plus-GPUI
performance counters, GPU/presentation timings, permission revocation and
controlled network failure evidence. Physical microphone/speaker testing is also
unverified.

### Sustained 720p30 baseline and short corrected-worker probes

The [ten-minute 720p30 record](results/2026-10-24-macos-arm64-720p30-sustained.json)
shows 29.98/29.97 focused decode FPS, 17,984 valid optical samples with no damaged
stamps, application frame-age p50/p95/max of 25/29/145 ms, and a 92 ms maximum
generated-audio receive gap. Both publisher and viewer selected UDP. This baseline
used the shared loop with corrected silence accounting, before audio was moved
to its own worker. See its
[conditions](results/2026-10-24-macos-arm64-720p30-sustained-environment.json) and
[process resources](results/2026-10-24-macos-arm64-720p30-sustained-resources.txt).
It is a headless native receive measurement, not a GPU presentation or capture
performance certification.

Short ten-second corrected-worker probes, collected while the baseline trial
was also active:

| Requested | Focused decode FPS by phase | Application frame-age p95 | Largest audio gap |
|---|---:|---:|---:|
| [720p30](results/2026-10-24-macos-arm64-720p30-timing.json) | 28.39 / 29.98 | 29 ms | 52 ms |
| [1080p60](results/2026-10-24-macos-arm64-1080p60-timing.json) | 59.75 / 59.96 | 36 ms | 58 ms |
| [1440p60](results/2026-10-24-macos-arm64-1440p60-timing.json) | 56.75 / 57.38 | 185 ms | 61 ms |

All three retained the expected dimensions, identified VideoToolbox encoders and
decoders, delivered valid optical stamps, and passed generated-audio continuity.
These short runs do not certify either 60 FPS profile. Thirty deterministic tests
and nine explicit local-SFU tests pass. Formatting, clippy, shell syntax and diff
checks pass; the previously documented upstream future-compatibility warnings
remain. Corrected-worker sustained retests and the remaining M0 gates are still
pending.

## Current sustained profile matrix — 2026-10-24

| Native profile trial | Focused decode FPS by phase | Frame-age p95 | Largest generated-audio gap | Audio continuity |
|---|---:|---:|---:|---|
| [720p30 baseline](results/2026-10-24-macos-arm64-720p30-sustained.json), shared loop | 29.98 / 29.97 | 29 ms | 92 ms | Pass |
| [1080p60](results/2026-10-24-macos-arm64-1080p60-sustained.json), dedicated audio worker | 59.99 / 59.59 | 45 ms | 132 ms | Pass |
| [1440p60](results/2026-10-24-macos-arm64-1440p60-sustained.json), dedicated audio worker | 53.66 / 48.65 | 78 ms | 61 ms | Pass |

Each observation lasted ten minutes with two video publishers and a viewer
switching focus at five minutes. All expected decoded sizes were retained, all
optical samples were valid, and no wrong-source frames were observed. VideoToolbox
was the reported encoder/decoder backend. The corrected 1440p60 run preserved
audio but did not sustain the requested video rate. None of these headless trials
certifies screen-capture throughput, GPUI presentation, hardware acceleration or
release-quality tiers. High-profile product selections remain uncertified.

Full [1080p60 conditions](results/2026-10-24-macos-arm64-1080p60-sustained-environment.json)
and [resource use](results/2026-10-24-macos-arm64-1080p60-sustained-resources.txt), plus
[1440p60 conditions](results/2026-10-24-macos-arm64-1440p60-sustained-environment.json)
and [resource use](results/2026-10-24-macos-arm64-1440p60-sustained-resources.txt), accompany
the raw records. `gpu-samples.jsonl` now records one-second whole-system AGX
snapshots. This is not Whisk attribution and may not represent VideoToolbox
media-engine activity. Only the final approximately 94 seconds of the corrected
1440p60 trial were sampled; the 1080p60 trial was sampled throughout. Raw GPU
records are retained beside their corresponding reports.

## Bounded network failure fixtures

`--relay-only` requires a relay in the native RTC configuration. Against the local
fixture without TURN, it exits nonzero with a sanitized timeout rather than
silently using direct UDP. The [CLI failure record](results/2026-10-24-macos-arm64-required-relay-no-turn.json)
includes actual elapsed time and contains no credentials or endpoint address.
A successful TURN deployment is not verified by this failure test.

The full live-test script additionally creates a uniquely named signaling-only
SFU, checks HTTP readiness, leaves UDP/TCP RTC ports unpublished, and advertises
an unreachable loopback address. That negative fixture uses ICE Lite so the
server cannot initiate a peer-reflexive route around the intentionally unavailable
ports. Direct connection to it must fail within the probe deadline. Its container
and network are removed on script exit; the ordinary loopback fixture remains
running. No host firewall or global network settings are changed.

Current fixtures explicitly use loopback ICE discovery settings, avoiding the
upstream default public STUN configuration. These settings and the ICE Lite
option follow the [pinned LiveKit configuration reference](https://raw.githubusercontent.com/livekit/livekit/v1.13.10/config-sample.yaml).
Earlier sustained records predate this local-only discovery change; the complete
live suite was rerun against the updated fixtures. Successful UDP tests, missing-
relay failure, and reachable-signaling/unreachable-media failure now have separate
controls.

## GPUI preparation measurements

Stop a preview, or let its bounded network test complete, then wait for
**Last UI measurement saved beside the app** before closing it. The native worker
writes `target/Whisk Media Lab-preview-metrics.json` (or the corresponding Network
Lab filename). Source replacement also queues the previous measurement. Closing
without that acknowledgement does not guarantee an export.

Counters include CPU-prepared frames, the BGRA bytes copied by the UI bridge,
maximum observed bridge queue depth, view-tree builds, and p95 image-preparation
microseconds. The observation window includes setup after the UI session starts.
These are CPU-side measurements: `gpu_presented_frames` remains unknown, and no
capture/drop or display-presentation claim is derived from them. Samples are
bounded to 100,000 and contain no frame pixels, window titles or room credentials.

The [manual static-pattern record](results/2026-10-24-macos-arm64-ui-pattern-metrics.json)
confirms one prepared 640×360 frame, 921,600 copied BGRA bytes, and a 174 µs
preparation sample. It is a rendering/export check, not a sustained FPS result.
A review caught a shutdown race in an initial automatic-on-quit export; that
promise was removed in favor of the explicit save acknowledgement.

Current verification: 45 deterministic Rust tests, four Python tests and seventeen
explicit local-SFU tests pass, plus formatting/clippy/shell checks. Remote CI has
not been run. Screen-capture access/live preview and physical microphone
publish/decode have been exercised. Full M0 is incomplete as described above.


## Successful local TURN media — 2026-10-25

`media-spike --loopback-relay` applies `RelayOnly` to both native room
connections. The [measured UDP TURN report](results/2026-10-25-macos-arm64-turn-udp-loopback.json)
records generated 1280×720 H.264 through the embedded self-hosted relay, with
VideoToolbox encoding/decoding and selected local `relay` candidates on both
publisher and viewer paths. This is a three-second local media proof, not a
TURN/TLS, restrictive-firewall, public deployment, or sustained-quality claim.
It sends generated pixels and uses no screen or microphone permission.

`./scripts/test-native-media.sh` now creates a separate disposable relay fixture
as well as the signaling-only failure fixture. It checks decoded colors and
dimensions, successful relay candidates and nonzero transport byte counters on
both media legs. A separate test against the ordinary no-TURN fixture proves
that required-relay media fails instead of silently using available direct UDP.
Both extra Compose projects are removed on exit. Ports 3478/UDP must be available;
signaling uses an assigned loopback port. RTC ports and relay-allocation ports
are not published from the TURN fixture. No host firewall is changed.

The fixture enables loopback candidates and permits the embedded relay to reach
only the container's loopback SFU address; the server also enforces its embedded
media-port policy. Its allocation range has room for both native participants.
Configuration follows the [pinned LiveKit TURN settings](https://raw.githubusercontent.com/livekit/livekit/v1.13.10/config-sample.yaml)
and [embedded relay implementation](https://raw.githubusercontent.com/livekit/livekit/v1.13.10/pkg/service/turn.go).
These development-only settings do not change a production deployment.

The current Media Lab bundle remains unchanged. Screen-recording settings show
the correct app enabled, but its preflight still reports denial after a Finder
restart. Captured-window UI measurements remain pending resolution of that
permission mismatch. M0 remains incomplete.


## Reusing a granted bundle and measuring the native UI

Quit the lab, then run:

```sh
./scripts/run-media-lab.sh --capture-network --reuse-build
```

`--reuse-build` skips compilation, bundle copying, plist changes and signing.
It fails clearly if the chosen Media Lab or Network Lab bundle does not exist.
Use `--network --reuse-build` for the separately identified Network Lab. Omit the
flag when intentionally rebuilding. Reuse runs the existing binary, so it does
not include later Rust changes until a rebuild. It preserves bundle identity;
it cannot repair a stale macOS permission grant.

The launcher now registers network runs through LaunchServices with fresh
short-lived local-fixture credentials. Direct executable launch previously left
an unregistered instance, and UI tooling could open a second instance without
those credentials. These are disposable local fixture tokens derived from the
public development secret; they are never printed. The launcher waits until
the app closes. Quit an existing instance before relaunching for fresh tokens.

The [30-second generated-network report](results/2026-10-25-macos-arm64-network-ui-loopback.json)
records 895 submitted/decoded 1280×720 H.264 frames with VideoToolbox encoding
and decoding. The [corresponding GPUI preparation report](results/2026-10-25-macos-arm64-network-ui-metrics.json)
records 849 CPU-prepared frames over 30,790 ms, approximately 27.57 preparation
FPS, a maximum queue depth of two, 3,129,753,600 copied BGRA bytes, and 1,056 µs
p95 CPU image-preparation time. Decode and UI preparation are different counters;
these numbers do not measure GPU presentation or screen-capture drops. The UI
visibly rendered the generated orange frame and acknowledged saving metrics.

All four files inside the frozen Media Lab bundle had identical SHA256 hashes
before and after the reuse run, and signature verification passed. Its refreshed
capture preflight still reported denial; no privacy settings were changed.


## Capture permission recovery verified — 2026-10-25

After the user confirmed completing the app-scoped permission reset and re-add,
the unchanged Media Lab successfully enumerated sources. Selecting only our
Whisk Network Lab window displayed its checkerboard in the local preview.
Stopping the source pattern visibly changed the captured image to the source
window's idle view, confirming live updates. Capture was stopped afterward;
no captured screen was published in this verification.

The [local capture UI record](results/2026-10-25-macos-arm64-local-capture-ui-metrics.json)
contains 605 CPU-prepared frames over 21,333 ms, approximately 28.36 preparation
FPS, maximum queue depth two, and 991 µs p95 CPU image preparation. This is a
short 720p30-requested preview check, not sustained-quality or GPU-presentation
certification. The metrics save acknowledgement was observed before export.

All four bundle files still match their earlier SHA256 hashes. The current
permission mismatch is resolved for this unchanged bundle; permission persistence
across future signing changes and revocation behavior remain unverified.


## Physical microphone probe — 2026-10-25

```sh
./scripts/run-audio-lab.sh
# Quit the app, then reuse the signed bundle with fresh fixture tokens:
./scripts/run-audio-lab.sh --reuse-build
```

The separate Whisk Audio Lab (`dev.whisk.audio-lab`) remains idle until its
**Start 10-second microphone test** button is pressed. It declares its own macOS
microphone purpose string, leaving the granted screen app unchanged. Every launch
creates a unique room with separate short-lived publisher/viewer tokens, so
ordinary probe clients cannot subscribe to the microphone. The UI rejects
non-local fixture addresses before device access; this guard was manually
verified with `example.invalid` and placeholder credentials.

Pinned LiveKit `PlatformAudio` supplies CoreAudio microphone frames. The probe
disables speaker playout before publishing, matches the expected publication
SID, decodes mono 48 kHz PCM and saves counters only. No PCM, device identifiers,
source names or credentials are retained. Stop signals cancellation; recording
ends before asynchronous stats/unpublish/room-close cleanup. The physical capture
budget includes track setup; connects precede device initialization. Synchronous
native driver calls cannot be preempted by Rust async deadlines.

The [user-started device report](results/2026-10-25-macos-arm64-microphone-loopback.json)
records 740 decoded frames / 355,200 samples in a 7,445 ms decoded observation
window, 81 frames above the input threshold, peak sample 5,302 and `audio/opus`,
with active UDP paths on both participants. Setup consumed part of the ten-second
capture budget. The UI acknowledged microphone off and counter-report save.
This proves physical microphone publish/decode, not speaker audibility,
continuous speech delivery or certified audio quality.

Review found that Stop during joining could still enter device initialization.
Two deterministic regressions failed, then passed with cancellation checks before
and after source setup, dropping a cancelled source before publication. The
saved device run preceded this repair; the rebuilt lab includes the repair,
validated by those regressions and the full generated-input suite. Automated
media tests never access physical devices or claim microphone verification.

Final checks: 37 deterministic Rust, 15 live SFU and four Python tests pass;
fmt/clippy/shell/diff checks and Audio Lab signature verification pass. Existing
upstream future-compatibility notices remain. All four screen-bundle files still
match their previously recorded SHA256 hashes. Physical speaker playback and the
remaining M0 gates above are still open.


A [repeat physical-device run](results/2026-10-25-macos-arm64-microphone-loopback-repeat.json)
completed after the repaired Audio Lab binary was built: 797 decoded frames,
382,560 samples, 107 frames above threshold, peak 12,394, and an 8,013 ms decoded
observation window. Microphone verification is true; playback verification is
false. The bundled executable and latest build have the same Mach-O UUID; their raw
file hashes differ because packaging re-signs the executable. Signature
verification passed. On the following reopen,
macOS displayed a force-quit/window-recovery prompt; a fresh window opened in
the microphone-off state. The prompt alone does not establish a crash cause.


## Speaker playback verified — 2026-10-25

```sh
./scripts/run-audio-lab.sh --speaker
# Quit before reusing the same signed bundle and refreshing fixture credentials:
./scripts/run-audio-lab.sh --speaker --reuse-build
```

Whisk Speaker Lab is a separate binary/bundle (`dev.whisk.speaker-lab`), selected
at compile time rather than by a runtime environment flag. Packaging it does
not modify the previously granted microphone or screen-capture apps. It has no
microphone purpose string and does not construct `PlatformAudio`, whose default
constructor enables microphone recording. Its speaker-only guard disables
recording before and after platform ADM acquisition, supplies generated PCM,
and stops/releases output before network cleanup. Three device-boundary tests
verify recording-disable order and success/failure resource ownership without
opening real devices. Cancellation uses the existing checked source boundary.

**Play quiet 3-second tone** sends a 440 Hz, low-level generated signal through
the private local SFU room to the system output. The lab opens idle, never starts
playback automatically, and leaves system volume unchanged. Requests longer
than three seconds are rejected before device access. Native output buffering
and synchronous device stop time are not hard realtime guarantees.

Playback initially remains unverified even when native playout starts and Opus
frames decode. The user must choose **I heard the tone** or **No sound** after
the test. Unknown, no-sound and heard results remain distinct. Confirmation is
written off the UI thread, and the UI acknowledges the actual file write.

The [confirmed physical speaker record](results/2026-10-25-macos-arm64-speaker-loopback.json)
contains 300 decoded frames / 144,000 samples, 296 non-silent frames, peak sample
1,339, `audio/opus` and active UDP on both legs. The decoded observation plus
native stop took 3,035 ms. The user confirmed hearing the tone in the app and in
chat; the saved report has `playback_started`, `user_heard_tone` and
`playback_verified` true, while `microphone_verified` stays false. This is a short
physical playback proof, not audio-quality or full M0 certification.

Current checks: 44 deterministic Rust, 15 explicit local-SFU and four Python
tests pass, with fmt/clippy/shell/diff checks and speaker-bundle signature
verification. Independent review found no actionable issues. Full SHA256
manifests confirm the existing Media Lab and Audio Lab bundles are unchanged.


## Experimental high-profile GPUI comparison — 2026-10-25

```sh
WHISK_MEDIA_REPORT_PATH="$PWD/target/profile-native.json" \
  ./scripts/run-profile-lab.sh 1080p60 15
# After quitting, reuse the same Network Lab build with fresh private-room tokens:
./scripts/run-profile-lab.sh 1080p60 15 --reuse-build
```

The runner supports 720p30/1080p60/1440p60 experimental targets and 1–300 seconds
per focus phase. It rebuilds Network Lab only, uses two distinct publishers and
one viewer in a unique local room, and switches focused generated video while
generated audio continues. It does not capture a screen or open physical audio
devices. Normal captured-window publishing remains on its existing 720p30 path.
No profile is added to the release-certified capability list.

The existing profile probe now forwards decoded BGRA frames into the bounded
watch/GPUI bridge. Pre-cancelled previews never connect; active cancellation stops
the probe/audio generator and closes its three owned rooms. A live regression
observes both orange and blue focused publishers at 1920×1080. Another active-stop
regression fails when the cancellation branch is deliberately disabled.

A 33 ms UI poll imposed an artificial preparation limit. The rebuilt Network Lab
polls at 8 ms, notifying/rendering only when state changes. The [comparison record](results/2026-10-25-macos-arm64-profile-ui-comparison.json)
links these same-machine development observations on macOS 27.0.1 / Apple M4:

| Measurement | 33 ms polling | 8 ms polling |
|---|---:|---:|
| UI-prepared frames | 814 | 1,711 |
| UI observation window | 31,664 ms | 31,745 ms |
| CPU preparation FPS | 25.71 | 53.90 |
| p95 CPU image preparation | 1,522 µs | 1,272 µs |
| Maximum bridge queue depth | 2 | 2 |
| Native decode FPS, phases A/B | 57.80 / 53.45 | 57.13 / 60.00 |
| Maximum generated-audio receive gap | 111 ms | 245 ms |

Both phases received 1920×1080 with VideoToolbox encoder/decoder. Both generated
audio checks passed, but the 245 ms gap is close to the 250 ms test threshold.
The foreground/workload conditions were not controlled. CPU preparation includes
startup and differs from view-tree builds and GPU/display presentation. These
30-second generated-video records are not sustained actual-capture proof or a
60 FPS release certification. The UI visibly rendered orange then blue and
acknowledged saving its metrics. Existing Media, Audio and Speaker bundle hashes
are unchanged; only Network Lab was rebuilt.

Raw records: [33 ms native](results/2026-10-25-macos-arm64-1080-ui-33ms-native.json),
[33 ms preparation](results/2026-10-25-macos-arm64-1080-ui-33ms-preparation.json),
[8 ms native](results/2026-10-25-macos-arm64-1080-ui-8ms-native.json),
[8 ms preparation](results/2026-10-25-macos-arm64-1080-ui-8ms-preparation.json).

Verification: 45 deterministic Rust, 17 live-SFU and four Python tests pass,
with fmt/clippy/shell/diff checks. Independent scoped review found no issues.
One earlier full live run failed `required_relay_carries_h264_on_both_publisher_and_viewer_paths`
with a connection timeout. The isolated test, its full seven-test group, and a
fresh complete live run then passed without deadline changes/retries in the
runner. The timeout cause was not reproduced; the failed log is retained in the
execution ledger directory. This remains an observed fixture/network limitation.
