#!/bin/sh
# Build and open the local macOS M0 preview. No screen access is requested here.
set -eu
WHISK_PROJECT_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$WHISK_PROJECT_ROOT"
if [ "$(uname -s)" != Darwin ]; then
  echo 'The M0 native preview is currently verified on macOS only.' >&2
  exit 1
fi
WHISK_BUNDLE_NAME='Whisk Media Lab'
WHISK_BUNDLE_ID='dev.whisk.media-lab'
case "${1:-}" in
  '') ;;
  --network)
    WHISK_BUNDLE_NAME='Whisk Network Lab'
    WHISK_BUNDLE_ID='dev.whisk.network-lab'
    docker compose -f spikes/media/fixture/compose.yaml up -d
    curl -fsS --retry 5 --retry-connrefused --retry-delay 1 http://127.0.0.1:7880 >/dev/null
    LIVEKIT_URL=ws://127.0.0.1:7880
    LIVEKIT_PUBLISH_TOKEN=$(python3 spikes/media/fixture/token.py --publisher)
    LIVEKIT_TOKEN=$(python3 spikes/media/fixture/token.py)
    WHISK_MEDIA_NETWORK_MODE=1
    export LIVEKIT_URL LIVEKIT_PUBLISH_TOKEN LIVEKIT_TOKEN WHISK_MEDIA_NETWORK_MODE
    ;;
  *) echo 'Usage: run-media-lab.sh [--network]' >&2; exit 1 ;;
esac
cargo build --locked -p media-spike --features desktop --bin whisk-desktop-spike
WHISK_BUNDLE="$WHISK_PROJECT_ROOT/target/$WHISK_BUNDLE_NAME.app"
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
<key>NSScreenCaptureUsageDescription</key><string>Whisk previews the screen or window you select. This build does not broadcast it.</string>
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
if [ "${1:-}" = --network ]; then
  "$WHISK_BUNDLE/Contents/MacOS/whisk-desktop-spike"
else
  open "$WHISK_BUNDLE"
fi
