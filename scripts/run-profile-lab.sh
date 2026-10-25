#!/bin/sh
# Experimental generated two-publisher GPUI measurement; no physical capture.
set -eu
WHISK_PROFILE_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$WHISK_PROFILE_ROOT"
WHISK_MEDIA_PROFILE=${1:-1080p60}
case "$WHISK_MEDIA_PROFILE" in 720p30|1080p60|1440p60) ;; *) echo 'Choose 720p30, 1080p60 or 1440p60.' >&2; exit 1 ;; esac
WHISK_MEDIA_PHASE_SECONDS=$(python3 - "${2:-15}" <<'PYTHON'
import sys
try:
    value = int(sys.argv[1])
    if not 1 <= value <= 300:
        raise ValueError()
except ValueError:
    sys.exit("Phase duration must be an integer from 1 to 300 seconds.")
print(value)
PYTHON
)
if [ "$#" -gt 3 ]; then echo 'Usage: run-profile-lab.sh [PROFILE] [PHASE_SECONDS] [--reuse-build]' >&2; exit 1; fi
case "${3:-}" in ''|--reuse-build) ;; *) echo 'Third argument must be --reuse-build.' >&2; exit 1 ;; esac
export WHISK_MEDIA_PROFILE WHISK_MEDIA_PHASE_SECONDS
if [ "${3:-}" = --reuse-build ]; then
  exec ./scripts/run-media-lab.sh --network --reuse-build
else
  exec ./scripts/run-media-lab.sh --network
fi
