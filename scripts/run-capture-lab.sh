#!/bin/sh
# Separate bundle; no discovery/capture/publish is started automatically.
set -eu
WHISK_CAPTURE_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$WHISK_CAPTURE_ROOT"
if [ "$(uname -s)" != Darwin ]; then echo 'Capture Lab currently supports macOS.' >&2; exit 1; fi
WHISK_CAPTURE_REUSE=0
if [ "${1:-}" = --reuse-build ]; then WHISK_CAPTURE_REUSE=1; shift; fi
WHISK_CAPTURE_PROFILE=${1:-720p30}
WHISK_CAPTURE_SECONDS=${2:-600}
if [ "$#" -gt 2 ]; then echo 'Usage: run-capture-lab.sh [--reuse-build] [720p30|1080p60|1440p60] [1-600 seconds]' >&2; exit 1; fi
case "$WHISK_CAPTURE_PROFILE" in 720p30|1080p60|1440p60) ;; *) echo 'Unknown capture profile.' >&2; exit 1;; esac
WHISK_CAPTURE_SECONDS=$(python3 - "$WHISK_CAPTURE_SECONDS" <<'PY'
import sys
seconds=int(sys.argv[1])
if not 1<=seconds<=600:
    raise SystemExit('Duration must be1–600 seconds')
print(seconds)
PY
)
WHISK_CAPTURE_BUNDLE="$WHISK_CAPTURE_ROOT/target/Whisk Capture Lab.app"
python3 - "$WHISK_CAPTURE_BUNDLE" <<'PY'
import subprocess,sys
from pathlib import Path
exe=str(Path(sys.argv[1]).resolve()/'Contents/MacOS/whisk-capture-spike')
for line in subprocess.check_output(['ps','-axo','pid,comm'],text=True).splitlines():
    if line.strip().split(maxsplit=1)[-1]==exe:
        raise SystemExit('Close Capture Lab before relaunching; closing either lab window quits both.')
PY
if [ "$WHISK_CAPTURE_REUSE" = 1 ]; then
  test -x "$WHISK_CAPTURE_BUNDLE/Contents/MacOS/whisk-capture-spike" || { echo 'Build Capture Lab once before --reuse-build.' >&2; exit 1; }
else
  cargo build --locked -p media-spike --features desktop --bin whisk-capture-spike
  mkdir -p "$WHISK_CAPTURE_BUNDLE/Contents/MacOS" "$WHISK_CAPTURE_BUNDLE/Contents/Resources"
  cp target/debug/whisk-capture-spike "$WHISK_CAPTURE_BUNDLE/Contents/MacOS/whisk-capture-spike"
  cp assets/whisk.icns "$WHISK_CAPTURE_BUNDLE/Contents/Resources/whisk.icns"
  python3 - "$WHISK_CAPTURE_BUNDLE/Contents/Info.plist" <<'PY'
import plistlib,sys
with open(sys.argv[1],'wb') as output:
    plistlib.dump({'CFBundleExecutable':'whisk-capture-spike','CFBundleIdentifier':'dev.whisk.capture-lab','CFBundleName':'Whisk Capture Lab','CFBundleDisplayName':'Whisk Capture Lab','CFBundleVersion':'0.1.0','CFBundlePackageType':'APPL','CFBundleIconFile':'whisk.icns','NSHighResolutionCapable':True,'NSScreenCaptureUsageDescription':'Whisk captures only the test window you select and publishes it to your local test server for the bounded measurement you start.'},output)
PY
  codesign --force --deep --sign - "$WHISK_CAPTURE_BUNDLE"
fi
WHISK_CAPTURE_RESULTS="$WHISK_CAPTURE_ROOT/target/capture-benchmark/$WHISK_CAPTURE_PROFILE-$(python3 -c 'import uuid; print(uuid.uuid4().hex[:8])')"
mkdir -p "$WHISK_CAPTURE_RESULTS"
printf '%s\n' "$WHISK_CAPTURE_RESULTS" > target/capture-benchmark/latest-result-dir
python3 - "$WHISK_CAPTURE_RESULTS/environment.json" "$WHISK_CAPTURE_PROFILE" "$WHISK_CAPTURE_SECONDS" <<'PY'
import json,platform,subprocess,sys
from pathlib import Path
Path(sys.argv[1]).write_text(json.dumps({'os':platform.mac_ver()[0],'hardware':subprocess.check_output(['sysctl','-n','machdep.cpu.brand_string'],text=True).strip(),'arch':platform.machine(),'sdk':'livekit0.9.7; ScreenCaptureKit11.1.0; GPUI0.2.2','build':'cargo dev unoptimized','requested_profile':sys.argv[2],'requested_seconds':int(sys.argv[3]),'source':'same-process owned animated test window; explicitly selected','process_resource_scope':'Capture Lab process includes owned animation, native capture/publisher/decoder and GPUI; SFU/container excluded. ps CPU percent is a process lifetime average, not interval CPU or codec-specific attribution. Samples start at the explicit publish phase, include setup/cleanup and stop at saved terminal reports.','foreground_load_controlled':False,'gpu_presented_fps':None,'capture_to_display_latency_ms':None,'gpu_utilization':None,'quality_certified':False},indent=2)+'\n')
PY
docker compose -f spikes/media/fixture/compose.yaml up -d
curl -fsS --retry 5 --retry-connrefused --retry-delay 1 http://127.0.0.1:7880 >/dev/null
WHISK_CAPTURE_ROOM="whisk-capture-$(python3 -c 'import uuid; print(uuid.uuid4().hex)')"
WHISK_CAPTURE_PUBLISH=$(python3 spikes/media/fixture/token.py --publisher --room "$WHISK_CAPTURE_ROOM")
WHISK_CAPTURE_VIEW=$(python3 spikes/media/fixture/token.py --room "$WHISK_CAPTURE_ROOM")
open -W -n --env LIVEKIT_URL=ws://127.0.0.1:7880 --env "LIVEKIT_PUBLISH_TOKEN=$WHISK_CAPTURE_PUBLISH" --env "LIVEKIT_TOKEN=$WHISK_CAPTURE_VIEW" --env "WHISK_CAPTURE_PROFILE=$WHISK_CAPTURE_PROFILE" --env "WHISK_CAPTURE_SECONDS=$WHISK_CAPTURE_SECONDS" --env "WHISK_MEDIA_REPORT_PATH=$WHISK_CAPTURE_RESULTS/measurement.json" --env "WHISK_CAPTURE_STATS_PATH=$WHISK_CAPTURE_RESULTS/capture-stats.json" --env "WHISK_CAPTURE_UI_REPORT_PATH=$WHISK_CAPTURE_RESULTS/ui-metrics.json" --env "WHISK_CAPTURE_START_PATH=$WHISK_CAPTURE_RESULTS/started.json" --env "WHISK_CAPTURE_OUTCOME_PATH=$WHISK_CAPTURE_RESULTS/outcome.json" "$WHISK_CAPTURE_BUNDLE" &
WHISK_CAPTURE_OPEN=$!
# Look up only executable paths, never process arguments/environments containing tokens.
WHISK_CAPTURE_PID=$(python3 - "$WHISK_CAPTURE_BUNDLE" <<'PY'
import subprocess,sys,time
from pathlib import Path
exe=str(Path(sys.argv[1]).resolve()/'Contents/MacOS/whisk-capture-spike')
for _ in range(50):
    for line in subprocess.check_output(['ps','-axo','pid,comm'],text=True).splitlines():
        fields=line.strip().split(maxsplit=1)
        if len(fields)==2 and fields[1]==exe:
            print(fields[0]);raise SystemExit
    time.sleep(.1)
raise SystemExit('Capture Lab did not start')
PY
)
python3 scripts/sample_capture_resources.py --pid "$WHISK_CAPTURE_PID" --result-dir "$WHISK_CAPTURE_RESULTS" > "$WHISK_CAPTURE_RESULTS/process-samples.jsonl" &
WHISK_CAPTURE_SAMPLER=$!
wait "$WHISK_CAPTURE_OPEN"
wait "$WHISK_CAPTURE_SAMPLER"

python3 - "$WHISK_CAPTURE_RESULTS" <<'PY'
import json,sys
from pathlib import Path
root=Path(sys.argv[1]);outcome=root/'outcome.json'
if not outcome.exists():
    outcome.write_text(json.dumps({'status':'process_closed_before_terminal_report','source':'owned_native_window','quality_certified':False,'partial_network_counters':None},indent=2)+'\n')
PY
