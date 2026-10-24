#!/bin/sh
# Experimental native SFU profile measurements; no certification implied.
set -eu
WHISK_PROJECT_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$WHISK_PROJECT_ROOT"
WHISK_PROFILE=${1:-720p30}
WHISK_PHASE_SECONDS=${2:-300}
WHISK_REPORT_DIR=${3:-target/media-benchmark/$WHISK_PROFILE}
case "$WHISK_PROFILE" in 720p30|1080p60|1440p60) ;; *) echo 'Unknown profile.' >&2; exit 1;; esac
WHISK_PHASE_SECONDS=$(python3 - "$WHISK_PHASE_SECONDS" <<'PY'
import sys
value = int(sys.argv[1])
if not 1 <= value <= 300:
    raise SystemExit('Phase duration must be 1-300 seconds; two phases are measured.')
print(value)
PY
)
if [ "$(uname -s)" != Darwin ]; then
  echo 'The resource measurement command currently supports macOS only.' >&2
  exit 1
fi
if [ -e "$WHISK_REPORT_DIR/measurement.json" ] || [ -e "$WHISK_REPORT_DIR/native-finished" ]; then
  echo 'A measurement already exists at this destination. Choose another directory.' >&2
  exit 1
fi
mkdir -p "$WHISK_REPORT_DIR"
cargo build --locked -p media-spike --bin media-spike
docker compose -f spikes/media/fixture/compose.yaml up -d
curl -fsS --retry 5 --retry-connrefused --retry-delay 1 http://127.0.0.1:7880 >/dev/null
LIVEKIT_URL=ws://127.0.0.1:7880
LIVEKIT_PUBLISH_TOKEN=$(python3 spikes/media/fixture/token.py --publisher)
LIVEKIT_SECOND_PUBLISH_TOKEN=$(python3 spikes/media/fixture/token.py --publisher)
LIVEKIT_TOKEN=$(python3 spikes/media/fixture/token.py)
export LIVEKIT_URL LIVEKIT_PUBLISH_TOKEN LIVEKIT_SECOND_PUBLISH_TOKEN LIVEKIT_TOKEN
trap 'touch "$WHISK_REPORT_DIR/native-finished"' EXIT
python3 scripts/gpu_metrics.py --duration "$((WHISK_PHASE_SECONDS * 2 + 120))" --until-file "$WHISK_REPORT_DIR/native-finished" > "$WHISK_REPORT_DIR/gpu-samples.jsonl" &
WHISK_GPU_SAMPLER=$!
WHISK_NATIVE_STATUS=0
/usr/bin/time -l target/debug/media-spike --profile "$WHISK_PROFILE" "$WHISK_PHASE_SECONDS" > "$WHISK_REPORT_DIR/measurement.json" 2> "$WHISK_REPORT_DIR/resources.txt" || WHISK_NATIVE_STATUS=$?
touch "$WHISK_REPORT_DIR/native-finished"
wait "$WHISK_GPU_SAMPLER"
trap - EXIT
if [ "$WHISK_NATIVE_STATUS" -ne 0 ]; then exit "$WHISK_NATIVE_STATUS"; fi
python3 - "$WHISK_REPORT_DIR" <<'PY'
import json, platform, subprocess, sys
from pathlib import Path
root = Path(sys.argv[1])
report = json.loads((root / 'measurement.json').read_text())
displays = json.loads(subprocess.check_output(['system_profiler', 'SPDisplaysDataType', '-json']))
gpus = [{key: gpu.get(key) for key in ('sppci_model', 'sppci_cores', 'spdisplays_mtlgpufamilysupport')} for gpu in displays.get('SPDisplaysDataType', [])]
snapshots = [json.loads(line) for line in (root / 'gpu-samples.jsonl').read_text().splitlines()]
means = []
for index in range(max((len(sample['gpus']) for sample in snapshots), default=0)):
    values = [sample['gpus'][index]['device_percent'] for sample in snapshots if len(sample['gpus']) > index and sample['gpus'][index]['device_percent'] is not None]
    means.append({'snapshot_device_index': index, 'valid_samples':len(values), 'sample_mean_device_percent':sum(values)/len(values) if values else None, 'max_device_percent':max(values) if values else None})
metadata = {
    'os': platform.system(), 'os_version': platform.mac_ver()[0],
    'architecture': platform.machine(),
    'cpu': subprocess.check_output(['sysctl', '-n', 'machdep.cpu.brand_string'], text=True).strip(),
    'build': 'cargo dev unoptimized',
    'gpu_inventory': gpus,
    'workload': 'two native color video publishers with an optical timestamp strip, one generated audio track, one native viewer; no GPUI or screen capture',
    'resource_method': '/usr/bin/time -l for native three-participant client process; SFU/container resources excluded',
    'focused_decode_fps': [count * 1000 / duration for count, duration in zip(report['received_frames'], report['phase_duration_ms'])],
    'gpu_utilization': None,
    'whole_system_agx_snapshots': means,
    'gpu_snapshot_scope': 'whole-system AGX snapshots, not Whisk attribution; may not reflect VideoToolbox media-engine activity', 'gpu_presented_frames': None, 'display_end_to_end_latency_ms': None,
    'frame_age_method': 'same-process monotonic optical stamp before BGRA/I420 conversion to application decode; excludes GPU presentation',
    'other_machine_load': 'uncontrolled; other development apps and tests may be active',
    'ten_minute_window': report['duration_ms'] >= 600000,
    'quality_certified': False,
}
(root / 'environment.json').write_text(json.dumps(metadata, indent=2) + '\n')
print(json.dumps({'profile': report['requested_profile'], 'duration_ms': report['duration_ms'], 'focused_decode_fps': metadata['focused_decode_fps'], 'generated_audio_verified': report['audio_verified'], 'quality_certified': False}))
PY
