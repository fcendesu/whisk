#!/bin/sh
# Physical microphone access occurs only after Start in the separate Audio Lab.
set -eu
WHISK_PROJECT_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$WHISK_PROJECT_ROOT"
if [ "$(uname -s)" != Darwin ]; then echo 'Audio Lab packaging currently supports macOS.' >&2; exit 1; fi
case "${1:-}" in
  ''|--reuse-build) ;;
  *) echo 'Usage: run-audio-lab.sh [--reuse-build]' >&2; exit 1 ;;
esac
if [ "$#" -gt 1 ]; then echo 'Usage: run-audio-lab.sh [--reuse-build]' >&2; exit 1; fi
WHISK_AUDIO_BUNDLE="$WHISK_PROJECT_ROOT/target/Whisk Audio Lab.app"
if [ "${1:-}" = --reuse-build ]; then
  if [ ! -x "$WHISK_AUDIO_BUNDLE/Contents/MacOS/whisk-audio-spike" ]; then
    echo 'Build Audio Lab once without --reuse-build.' >&2; exit 1
  fi
else
  cargo build --locked -p media-spike --features desktop --bin whisk-audio-spike
  mkdir -p "$WHISK_AUDIO_BUNDLE/Contents/MacOS" "$WHISK_AUDIO_BUNDLE/Contents/Resources"
  cp target/debug/whisk-audio-spike "$WHISK_AUDIO_BUNDLE/Contents/MacOS/whisk-audio-spike"
  cp assets/whisk.icns "$WHISK_AUDIO_BUNDLE/Contents/Resources/whisk.icns"
  python3 - "$WHISK_AUDIO_BUNDLE/Contents/Info.plist" <<'PYTHON'
import plistlib
import sys
with open(sys.argv[1], "wb") as output:
    plistlib.dump({
        "CFBundleExecutable": "whisk-audio-spike",
        "CFBundleIdentifier": "dev.whisk.audio-lab",
        "CFBundleName": "Whisk Audio Lab",
        "CFBundleDisplayName": "Whisk Audio Lab",
        "CFBundleVersion": "0.1.0",
        "CFBundlePackageType": "APPL",
        "CFBundleIconFile": "whisk.icns",
        "NSHighResolutionCapable": True,
        "NSMicrophoneUsageDescription": "Whisk uses your microphone only during the ten-second test you start and sends the audio to your local media test server.",
    }, output)
PYTHON
  codesign --force --deep --sign - "$WHISK_AUDIO_BUNDLE"
fi
docker compose -f spikes/media/fixture/compose.yaml up -d
curl -fsS --retry 5 --retry-connrefused --retry-delay 1 http://127.0.0.1:7880 >/dev/null
WHISK_AUDIO_ROOM="whisk-audio-$(python3 -c 'import uuid; print(uuid.uuid4().hex)')"
# A new room per launch prevents other probe participants from receiving the mic.
# Credentials are short-lived and derived from the public local-fixture secret.
WHISK_AUDIO_PUBLISH_TOKEN=$(python3 spikes/media/fixture/token.py --publisher --room "$WHISK_AUDIO_ROOM")
WHISK_AUDIO_VIEW_TOKEN=$(python3 spikes/media/fixture/token.py --room "$WHISK_AUDIO_ROOM")
open -W -n --env LIVEKIT_URL=ws://127.0.0.1:7880 --env "LIVEKIT_PUBLISH_TOKEN=$WHISK_AUDIO_PUBLISH_TOKEN" --env "LIVEKIT_TOKEN=$WHISK_AUDIO_VIEW_TOKEN" "$WHISK_AUDIO_BUNDLE"
