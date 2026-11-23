#!/usr/bin/env bash
# Package Shorashim-x86_64.AppImage (Linux).
#
#   tools/make_appimage.sh <shorashim binary> <out dir>
#
# AppDir layout: usr/bin/shorashim, and its course files (content/) and
# music in usr/share/shorashim, where the app looks for them. The icon is
# converted from assets/icon/shorashim.webp at build time (nothing
# committed). System libraries (GPU drivers, X11/Wayland, ALSA) come from
# the user's system, as with any desktop app.
set -euo pipefail

bin=${1:?shorashim binary}
out=${2:?out dir}
root=$(cd "$(dirname "$0")/.." && pwd)
mkdir -p "$out"
out=$(cd "$out" && pwd)
work=$(mktemp -d)
appdir="$work/Shorashim.AppDir"

mkdir -p "$appdir/usr/bin" "$appdir/usr/share/shorashim" \
  "$appdir/usr/share/applications" "$appdir/usr/share/icons/hicolor/256x256/apps"
cp "$bin" "$appdir/usr/bin/shorashim"
chmod +x "$appdir/usr/bin/shorashim"
cp -R "$root/content" "$appdir/usr/share/shorashim/content"
cp -R "$root/assets/music" "$appdir/usr/share/shorashim/music"
cp "$root/LICENSE" "$root/NOTICE" "$appdir/usr/share/shorashim/"

# Icon: WebP → PNG for the desktop (AppImage tools need PNG or SVG).
venv="$work/venv"
python3 -m venv "$venv"
"$venv/bin/pip" install --quiet pillow
"$venv/bin/python" - "$root/assets/icon/shorashim-256.webp" \
  "$appdir/usr/share/icons/hicolor/256x256/apps/shorashim.png" <<'PY'
import sys
from PIL import Image
Image.open(sys.argv[1]).convert("RGBA").save(sys.argv[2])
PY
cp "$appdir/usr/share/icons/hicolor/256x256/apps/shorashim.png" "$appdir/shorashim.png"

cat > "$appdir/usr/share/applications/shorashim.desktop" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=Shorashim
Comment=Foundations of Judaism and preparation for giyur
Exec=shorashim
Icon=shorashim
Categories=Education;
Terminal=false
DESKTOP
cp "$appdir/usr/share/applications/shorashim.desktop" "$appdir/shorashim.desktop"

cat > "$appdir/AppRun" <<'RUN'
#!/bin/sh
here=$(dirname "$(readlink -f "$0")")
exec "$here/usr/bin/shorashim" "$@"
RUN
chmod +x "$appdir/AppRun"

tool="$work/appimagetool"
curl -fsSL -o "$tool" \
  https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage
chmod +x "$tool"
rm -f "$out/Shorashim-x86_64.AppImage"
ARCH=x86_64 "$tool" --appimage-extract-and-run --no-appstream \
  "$appdir" "$out/Shorashim-x86_64.AppImage"
rm -rf "$work"
ls -l "$out/Shorashim-x86_64.AppImage"
