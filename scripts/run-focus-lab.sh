#!/bin/sh
# Launches idle. Only the explicit Start button enables microphone capture.
set -eu
WHISK_FOCUS_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$WHISK_FOCUS_ROOT"
if [ "$(uname -s)" != Darwin ]; then echo 'Focus Lab packaging currently supports macOS.' >&2; exit 1; fi
WHISK_FOCUS_REUSE=0
case "${1:-}" in
  --reuse-build) WHISK_FOCUS_REUSE=1 ;;
  '') ;;
  *) echo 'Usage: run-focus-lab.sh [--reuse-build]' >&2; exit 1 ;;
esac
if [ "$#" -gt 1 ]; then echo 'Usage: run-focus-lab.sh [--reuse-build]' >&2; exit 1; fi
WHISK_FOCUS_APP="$WHISK_FOCUS_ROOT/target/Whisk Focus Lab.app"
if [ "$WHISK_FOCUS_REUSE" = 1 ]; then
  test -x "$WHISK_FOCUS_APP/Contents/MacOS/whisk-focus-spike" || { echo 'Build Focus Lab once before --reuse-build.' >&2; exit 1; }
else
  cargo build --locked -p media-spike --features desktop --bin whisk-focus-spike
  mkdir -p "$WHISK_FOCUS_APP/Contents/MacOS" "$WHISK_FOCUS_APP/Contents/Resources"
  cp target/debug/whisk-focus-spike "$WHISK_FOCUS_APP/Contents/MacOS/whisk-focus-spike"
  cp assets/whisk.icns "$WHISK_FOCUS_APP/Contents/Resources/whisk.icns"
  python3 - "$WHISK_FOCUS_APP/Contents/Info.plist" <<'PYTHON'
import plistlib,sys
with open(sys.argv[1], 'wb') as output:
    plistlib.dump({'CFBundleExecutable':'whisk-focus-spike','CFBundleIdentifier':'dev.whisk.focus-lab','CFBundleName':'Whisk Focus Lab','CFBundleDisplayName':'Whisk Focus Lab','CFBundleVersion':'0.1.0','CFBundlePackageType':'APPL','CFBundleIconFile':'whisk.icns','NSHighResolutionCapable':True,'NSMicrophoneUsageDescription':'Whisk captures microphone audio only during the focus test you start, for up to fifteen seconds, and sends it to your local test server. No audio recording is saved.'},output)
PYTHON
  codesign --force --deep --sign - "$WHISK_FOCUS_APP"
fi
docker compose -f spikes/media/fixture/compose.yaml up -d
curl -fsS --retry 5 --retry-connrefused --retry-delay 1 http://127.0.0.1:7880 >/dev/null
WHISK_FOCUS_ROOM="whisk-focus-$(python3 -c 'import uuid; print(uuid.uuid4().hex)')"
WHISK_FOCUS_FIRST=$(python3 spikes/media/fixture/token.py --publisher --room "$WHISK_FOCUS_ROOM")
WHISK_FOCUS_SECOND=$(python3 spikes/media/fixture/token.py --publisher --room "$WHISK_FOCUS_ROOM")
WHISK_FOCUS_VIEWER=$(python3 spikes/media/fixture/token.py --room "$WHISK_FOCUS_ROOM")
open -W -n --env LIVEKIT_URL=ws://127.0.0.1:7880 --env "LIVEKIT_PUBLISH_TOKEN=$WHISK_FOCUS_FIRST" --env "LIVEKIT_SECOND_PUBLISH_TOKEN=$WHISK_FOCUS_SECOND" --env "LIVEKIT_TOKEN=$WHISK_FOCUS_VIEWER" "$WHISK_FOCUS_APP"
