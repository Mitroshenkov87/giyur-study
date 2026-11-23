#!/usr/bin/env bash
# Package Shorashim.app and an unsigned Shorashim-macOS.dmg (macOS only).
#
#   tools/make_macos_app.sh <shorashim binary> <out dir>
#
# The bundle holds the app binary in Contents/MacOS and its course files
# (content/) and music in Contents/Resources, where the app looks for them.
# The icon comes from assets/icon/shorashim.webp (converted at build time,
# nothing committed). The app is ad-hoc signed only (no Apple ID, free);
# Gatekeeper will still call it an unidentified developer's app.
set -euo pipefail

bin=${1:?shorashim binary}
out=${2:?out dir}
root=$(cd "$(dirname "$0")/.." && pwd)
version=$(sed -n 's/^version = "\(.*\)"/\1/p' "$root/Cargo.toml" | head -n1)

app="$out/Shorashim.app"
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp "$bin" "$app/Contents/MacOS/shorashim"
chmod +x "$app/Contents/MacOS/shorashim"
cp -R "$root/content" "$app/Contents/Resources/content"
cp -R "$root/assets/music" "$app/Contents/Resources/music"
cp "$root/LICENSE" "$root/NOTICE" "$app/Contents/Resources/"

# Icon: WebP → .icns with Pillow in a throwaway venv.
venv=$(mktemp -d)
python3 -m venv "$venv"
"$venv/bin/pip" install --quiet pillow
"$venv/bin/python" - "$root/assets/icon/shorashim.webp" "$app/Contents/Resources/Shorashim.icns" <<'PY'
import sys
from PIL import Image
img = Image.open(sys.argv[1]).convert("RGBA")
img.save(sys.argv[2], format="ICNS", sizes=[(16, 16), (32, 32), (64, 64), (128, 128), (256, 256), (512, 512)])
PY
rm -rf "$venv"

cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key><string>en</string>
  <key>CFBundleDisplayName</key><string>Shorashim</string>
  <key>CFBundleName</key><string>Shorashim</string>
  <key>CFBundleExecutable</key><string>shorashim</string>
  <key>CFBundleIdentifier</key><string>io.github.mitroshenkov87.shorashim</string>
  <key>CFBundleIconFile</key><string>Shorashim</string>
  <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$version</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>LSApplicationCategoryType</key><string>public.app-category.education</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSHumanReadableCopyright</key><string>Copyright 2026 Aleksandr Mitroshenkov. Apache-2.0</string>
</dict>
</plist>
PLIST
plutil -lint "$app/Contents/Info.plist"

# Ad-hoc signature (Apple Silicon refuses to run unsigned code at all).
codesign --force --deep --sign - "$app"
codesign --verify --verbose "$app"

# DMG: the app and a link to /Applications to drag it onto.
stage=$(mktemp -d)
cp -R "$app" "$stage/"
ln -s /Applications "$stage/Applications"
rm -f "$out/Shorashim-macOS.dmg"
hdiutil create -volname "Shorashim" -srcfolder "$stage" -ov -format UDZO \
  -imagekey zlib-level=9 "$out/Shorashim-macOS.dmg"
rm -rf "$stage"
ls -l "$out/Shorashim-macOS.dmg"
