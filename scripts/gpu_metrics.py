"""Allowlisted whole-system AGX utilization, not per-process GPU attribution."""
import argparse
import json
import math
import os
import plistlib
import subprocess
import time
from pathlib import Path


def extract_gpu_utilization(entries):
    result = []
    for entry in entries:
        stats = entry.get('PerformanceStatistics', {})
        gpu = {}
        for source, target in [('Device Utilization %', 'device_percent'),
                               ('Renderer Utilization %', 'renderer_percent'),
                               ('Tiler Utilization %', 'tiler_percent')]:
            value = stats.get(source)
            valid = type(value) in (int, float) and math.isfinite(value) and 0 <= value <= 100
            gpu[target] = float(value) if valid else None
        result.append(gpu)
    return result


def sample():
    try:
        data = subprocess.check_output(['ioreg', '-a', '-r', '-c', 'AGXAccelerator', '-d', '1'],
                                       stderr=subprocess.DEVNULL, timeout=2)
        return extract_gpu_utilization(plistlib.loads(data))
    except (OSError, ValueError, TypeError, AttributeError, subprocess.SubprocessError):
        return []


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--duration', type=int, default=60)
    parser.add_argument('--pid', type=int)
    parser.add_argument('--until-file', type=Path)
    args = parser.parse_args()
    if not 1 <= args.duration <= 720 or (args.pid is not None and args.pid <= 0):
        parser.error('duration must be1-720 seconds and PID must be positive')
    start = time.monotonic()
    while time.monotonic() - start < args.duration:
        if args.until_file is not None and args.until_file.exists():
            break
        if args.pid is not None:
            try:
                os.kill(args.pid, 0)  # Liveness check only; never terminate another process.
            except ProcessLookupError:
                break
            except PermissionError:
                break
        print(json.dumps({'elapsed_ms': round((time.monotonic()-start)*1000),
                          'scope': 'whole_system_agx_snapshot', 'gpus': sample()},
                         allow_nan=False), flush=True)
        time.sleep(1)


if __name__ == '__main__':
    main()
