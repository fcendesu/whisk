#!/bin/sh
# Physical microphone access occurs only after Start in the separate Audio Lab.
set -eu
WHISK_PROJECT_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$WHISK_PROJECT_ROOT"
if [ "$(uname -s)" != Darwin ]; then echo 'Audio Lab packaging currently supports macOS.' >&2; exit 1; fi
WHISK_REUSE_AUDIO=0
WHISK_SPEAKER_MODE=0
for WHISK_AUDIO_ARGUMENT in "$@"; do
  case "$WHISK_AUDIO_ARGUMENT" in
    --reuse-build) WHISK_REUSE_AUDIO=1 ;;
    --speaker) WHISK_SPEAKER_MODE=1 ;;
    *) echo 'Usage: run-audio-lab.sh [--speaker] [--reuse-build]' >&2; exit 1 ;;
  esac
done
WHISK_AUDIO_NAME='Whisk Audio Lab'
WHISK_AUDIO_EXECUTABLE='whisk-audio-spike'
WHISK_AUDIO_ID='dev.whisk.audio-lab'
if [ "$WHISK_SPEAKER_MODE" = 1 ]; then
  WHISK_AUDIO_NAME='Whisk Speaker Lab'
  WHISK_AUDIO_EXECUTABLE='whisk-speaker-spike'
  WHISK_AUDIO_ID='dev.whisk.speaker-lab'
fi
WHISK_AUDIO_BUNDLE="$WHISK_PROJECT_ROOT/target/$WHISK_AUDIO_NAME.app"
if [ "$WHISK_REUSE_AUDIO" = 1 ]; then
  if [ ! -x "$WHISK_AUDIO_BUNDLE/Contents/MacOS/$WHISK_AUDIO_EXECUTABLE" ]; then
    echo 'Build Audio Lab once without --reuse-build.' >&2; exit 1
  fi
else
  cargo build --locked -p media-spike --features desktop --bin "$WHISK_AUDIO_EXECUTABLE"
  mkdir -p "$WHISK_AUDIO_BUNDLE/Contents/MacOS" "$WHISK_AUDIO_BUNDLE/Contents/Resources"
  cp "target/debug/$WHISK_AUDIO_EXECUTABLE" "$WHISK_AUDIO_BUNDLE/Contents/MacOS/$WHISK_AUDIO_EXECUTABLE"
  cp assets/whisk.icns "$WHISK_AUDIO_BUNDLE/Contents/Resources/whisk.icns"
  python3 - "$WHISK_AUDIO_BUNDLE/Contents/Info.plist" "$WHISK_AUDIO_NAME" "$WHISK_AUDIO_EXECUTABLE" "$WHISK_AUDIO_ID" "$WHISK_SPEAKER_MODE" <<'PYTHON'
import plistlib
import sys
path, name, executable, identity, speaker = sys.argv[1:]
data = {
    "CFBundleExecutable": executable, "CFBundleIdentifier": identity,
    "CFBundleName": name, "CFBundleDisplayName": name,
    "CFBundleVersion": "0.1.0", "CFBundlePackageType": "APPL",
    "CFBundleIconFile": "whisk.icns", "NSHighResolutionCapable": True,
}
if speaker != "1":
    data["NSMicrophoneUsageDescription"] = "Whisk uses your microphone only during the ten-second test you start and sends the audio to your local media test server."
with open(path, "wb") as output:
    plistlib.dump(data, output)

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
