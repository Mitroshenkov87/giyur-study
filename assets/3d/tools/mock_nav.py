#!/usr/bin/env python3
"""Mockup: paste 3D nav icons and section emblems onto real 1920x1080 app
screenshots (wood + leather). Pure illustration, not wired into the UI.
Usage: mock_nav.py <screenshot> <theme wood|leather> <img_dir> <out.webp> [hover_index]"""
import sys
from PIL import Image

shot, theme, IMG, out = sys.argv[1:5]
hover = int(sys.argv[5]) if len(sys.argv) > 5 else -1
im = Image.open(shot).convert("RGBA")
S = 1.5  # screenshot coords measured at 1280 wide
NAV = ["lessons", "calendar", "cards", "quiz", "tests", "progress", "settings", "sources"]
ROW_Y = [84, 121, 158, 194, 231, 268, 304, 341]   # row centres (1280 scale)
SHIFT, ICON = 44, 46

# 1) move nav text right to make room for icons
x0, x1 = int(180 * S), int(480 - SHIFT)
y0, y1 = int(66 * S), int(356 * S)
text = im.crop((x0, y0, x1, y1))
filler = im.crop((x0, 560, x1 + SHIFT, 560 + (y1 - y0)))
im.paste(filler, (x0, y0))
im.alpha_composite(text, (x0 + SHIFT, y0))
for i, (n, yc) in enumerate(zip(NAV, ROW_Y)):
    var = f"{n}-{theme}" + ("-hover" if i == hover else "")
    ic = Image.open(f"{IMG}/icons/{var}.webp").convert("RGBA").resize((ICON, ICON), Image.LANCZOS)
    im.alpha_composite(ic, (int(182 * S) + 2, int(yc * S - ICON / 2)))

# 2) section emblems over the old flat section tiles
SECT = ["osnovy", "ivrit", "shabbat", "kashrut", "prazdniki", "semya"]
TILE_Y = [296, 377, 453, 530, 606, 683]
E = 78
for n, yc in zip(SECT, TILE_Y):
    cx, cy = int(375.5 * S), int(yc * S)
    card = im.getpixel((int(352 * S) - 4, cy))
    im.paste(Image.new("RGBA", (70, 70), card), (cx - 35, cy - 35))
    em = Image.open(f"{IMG}/emblems/{n}-{theme}.webp").convert("RGBA").resize((E, E), Image.LANCZOS)
    im.alpha_composite(em, (cx - E // 2, cy - E // 2))
im.convert("RGB").save(out, "WEBP", quality=90, method=6)
print(out)
