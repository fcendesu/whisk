#!/bin/sh
# Build and open the local macOS M0 preview. No screen access is requested here.
set -eu
WHISK_PROJECT_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$WHISK_PROJECT_ROOT"
if [ "$(uname -s)" != Darwin ]; then
  echo 'The M0 native preview is currently verified on macOS only.' >&2
  exit 1
fi
cargo build --locked -p media-spike --features desktop --bin whisk-desktop-spike
WHISK_BUNDLE="$WHISK_PROJECT_ROOT/target/Whisk Media Lab.app"
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
codesign --force --deep --sign - "$WHISK_BUNDLE"
open "$WHISK_BUNDLE"
