#!/bin/sh
# Build and open the local macOS M0 preview. No screen access is requested here.
set -eu
WHISK_PROJECT_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$WHISK_PROJECT_ROOT"
if [ "$(uname -s)" != Darwin ]; then
  echo 'The M0 native preview is currently verified on macOS only.' >&2
  exit 1
fi
WHISK_LAB_MODE=''
WHISK_REUSE_BUILD=0
for WHISK_ARGUMENT in "$@"; do
  case "$WHISK_ARGUMENT" in
    --reuse-build) WHISK_REUSE_BUILD=1 ;;
    --network|--capture-network)
      if [ -n "$WHISK_LAB_MODE" ]; then
        echo 'Choose one lab mode: --network or --capture-network.' >&2
        exit 1
      fi
      WHISK_LAB_MODE=$WHISK_ARGUMENT
      ;;
    *) echo 'Usage: run-media-lab.sh [--network|--capture-network] [--reuse-build]' >&2; exit 1 ;;
  esac
done
WHISK_BUNDLE_NAME='Whisk Media Lab'
WHISK_BUNDLE_ID='dev.whisk.media-lab'
if [ "$WHISK_LAB_MODE" = --network ]; then
  WHISK_BUNDLE_NAME='Whisk Network Lab'
  WHISK_BUNDLE_ID='dev.whisk.network-lab'
fi
WHISK_BUNDLE="$WHISK_PROJECT_ROOT/target/$WHISK_BUNDLE_NAME.app"
if [ "$WHISK_REUSE_BUILD" = 1 ]; then
  if [ ! -x "$WHISK_BUNDLE/Contents/MacOS/whisk-desktop-spike" ]; then
    echo 'No existing lab build. Run once without --reuse-build.' >&2
    exit 1
  fi
else
  cargo build --locked -p media-spike --features desktop --bin whisk-desktop-spike
  mkdir -p "$WHISK_BUNDLE/Contents/MacOS" "$WHISK_BUNDLE/Contents/Resources"
  cp target/debug/whisk-desktop-spike "$WHISK_BUNDLE/Contents/MacOS/whisk-desktop-spike"
  cp assets/whisk.icns "$WHISK_BUNDLE/Contents/Resources/whisk.icns"
  cat > "$WHISK_BUNDLE/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>whisk-desktop-spike</string>
<key>CFBundleIdentifier</key><string>dev.whisk.media-lab</string>
<key>CFBundleName</key><string>Whisk Media Lab</string>
<key>CFBundleDisplayName</key><string>Whisk Media Lab</string>
<key>CFBundleVersion</key><string>0.1.0</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleIconFile</key><string>whisk.icns</string>
<key>NSHighResolutionCapable</key><true/>
<key>NSScreenCaptureUsageDescription</key><string>Whisk previews the screen or window you select. Publish selected window explicitly sends that window to your configured media server for a bounded test.</string>
</dict></plist>
PLIST
  python3 - "$WHISK_BUNDLE/Contents/Info.plist" "$WHISK_BUNDLE_NAME" "$WHISK_BUNDLE_ID" <<'PYTHON'
import plistlib
import sys
path, name, identity = sys.argv[1:]
with open(path, "rb") as source:
    data = plistlib.load(source)
data.update(CFBundleIdentifier=identity, CFBundleName=name, CFBundleDisplayName=name)
with open(path, "wb") as destination:
    plistlib.dump(data, destination)
PYTHON
  codesign --force --deep --sign - "$WHISK_BUNDLE"
fi
if [ "$WHISK_LAB_MODE" = --network ] || [ "$WHISK_LAB_MODE" = --capture-network ]; then
  docker compose -f spikes/media/fixture/compose.yaml up -d
  curl -fsS --retry 5 --retry-connrefused --retry-delay 1 http://127.0.0.1:7880 >/dev/null
  # Mint after compilation so build time cannot consume the 120-second join window.
  LIVEKIT_URL=ws://127.0.0.1:7880
  WHISK_MEDIA_ROOM="whisk-media-$(python3 -c 'import uuid; print(uuid.uuid4().hex)')"
  LIVEKIT_PUBLISH_TOKEN=$(python3 spikes/media/fixture/token.py --publisher --room "$WHISK_MEDIA_ROOM")
  LIVEKIT_TOKEN=$(python3 spikes/media/fixture/token.py --room "$WHISK_MEDIA_ROOM")
  export LIVEKIT_URL LIVEKIT_PUBLISH_TOKEN LIVEKIT_TOKEN
  # LaunchServices registration makes this the same app selected by UI tools.
  # These are disposable, short-lived local-fixture credentials, never printed.
  set -- --env "LIVEKIT_URL=$LIVEKIT_URL" --env "LIVEKIT_PUBLISH_TOKEN=$LIVEKIT_PUBLISH_TOKEN" --env "LIVEKIT_TOKEN=$LIVEKIT_TOKEN"
  if [ "$WHISK_LAB_MODE" = --network ]; then
    set -- "$@" --env WHISK_MEDIA_NETWORK_MODE=1
  fi
  if [ -n "${WHISK_MEDIA_PROFILE:-}" ]; then
    LIVEKIT_SECOND_PUBLISH_TOKEN=$(python3 spikes/media/fixture/token.py --publisher --room "$WHISK_MEDIA_ROOM")
    set -- "$@" --env "LIVEKIT_SECOND_PUBLISH_TOKEN=$LIVEKIT_SECOND_PUBLISH_TOKEN" --env "WHISK_MEDIA_PROFILE=$WHISK_MEDIA_PROFILE" --env "WHISK_MEDIA_PHASE_SECONDS=${WHISK_MEDIA_PHASE_SECONDS:-15}"
  fi
  if [ -n "${WHISK_MEDIA_REPORT_PATH:-}" ]; then
    set -- "$@" --env "WHISK_MEDIA_REPORT_PATH=$WHISK_MEDIA_REPORT_PATH"
  fi
  open -W -n "$@" "$WHISK_BUNDLE"
else
  open "$WHISK_BUNDLE"
fi
