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

Run all nine opt-in SFU tests with `./scripts/test-native-media.sh`. This operator
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

- Task 2: exercise display/window capture and revocation with actual granted OS
  permission; the native adapter, permission-denial path and frame handoff exist.
- Task 3: connect actual captured frames after permission verification; generated
  H.264 publish/decode and conservative profile-policy tests exist.
- Task 4: instrument frame timing and GPU presentation/copy metrics. The remote
  decoded frame → GPUI bridge has now been exercised in the network window.
- Task 5: ten-minute hardware measurements with real capture and audio
  continuity; the two-synthetic-publisher focus switch is now exercised.
- Verify actual microphone publish/playback and audio continuity while switching
  streams before declaring the milestone complete.
- Record OS/hardware/driver/codec details, frame drops, latency and CPU/GPU use;
  verify UDP/TURN failures, platform capture permissions, and SDK cancellation
  cleanup under repeated failures. Timeout tests only prove caller deadlines.
- Pin GPUI when checking its real renderer APIs. Audit dependency licenses and
  native binaries before distribution; this connection slice is not that audit.

No 1080p60, 1440p60, GPU zero-copy, production deployment, or cross-platform media
support is claimed. M1 and later milestones remain gated on M0 evidence.

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
