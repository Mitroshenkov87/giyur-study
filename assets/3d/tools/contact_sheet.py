#!/usr/bin/env python3
"""Contact sheet of all rendered art: objects on the dark theme, icons and
emblems on both theme backgrounds. Usage: contact_sheet.py <img_dir> <out.webp>"""
import glob, os, sys
from PIL import Image, ImageDraw, ImageFont

IMG, OUT = sys.argv[1], sys.argv[2]
FONT = os.path.join(os.path.dirname(__file__), "..", "..", "fonts", "NotoSans-Regular.ttf")
BOLD = os.path.join(os.path.dirname(__file__), "..", "..", "fonts", "NotoSans-Bold.ttf")
f = ImageFont.truetype(FONT, 18); fb = ImageFont.truetype(BOLD, 30)
DARK, LIGHT = (28, 18, 13), (168, 144, 104)
DARK_SIDE, LIGHT_SIDE = (18, 11, 8), (58, 38, 24)  # wood sidebar / dark goatskin sidebar
TEXT_D, TEXT_L = (239, 227, 204), (42, 26, 12)

def grid(title, items, cell, cols, bg, fg, label=True):
    rows = (len(items) + cols - 1) // cols
    lh = 28 if label else 4
    im = Image.new("RGB", (cols * cell + 40, rows * (cell + lh) + 70), bg)
    d = ImageDraw.Draw(im); d.text((20, 18), title, font=fb, fill=fg)
    for i, (name, path) in enumerate(items):
        x, y = 20 + (i % cols) * cell, 64 + (i // cols) * (cell + lh)
        t = Image.open(path).convert("RGBA").resize((cell - 12, cell - 12), Image.LANCZOS)
        tile = Image.new("RGBA", t.size, bg + (255,)); tile.alpha_composite(t)
        im.paste(tile.convert("RGB"), (x + 6, y + 6))
        if label:
            d.text((x + cell // 2, y + cell + 2), name, font=f, fill=fg, anchor="mt")
    return im

def pick(folder, pattern):
    out = []
    for p in sorted(glob.glob(os.path.join(IMG, folder, pattern))):
        n = os.path.basename(p)[:-5]
        if n.endswith("-512") or n.endswith("-256"):
            continue
        out.append((n, p))
    return out

parts = []
objs = pick("objects", "*.webp")
if objs:
    parts.append(grid(f"Objects ({len(objs)}) — dark wood theme", objs, 300, 6, DARK, TEXT_D))
    parts.append(grid("Objects — light leather theme", objs, 150, 12, LIGHT, TEXT_L, label=False))
for theme, bg, fg in (("wood", DARK_SIDE, TEXT_D), ("leather", LIGHT_SIDE, TEXT_D)):
    ic = [(n.replace("icon_", "").replace(f"-{theme}", ""), p) for n, p in pick("icons", f"icon_*-{theme}*.webp")]
    if ic:
        parts.append(grid(f"Nav icons — {theme} (rest / hover)", ic, 150, 8, bg, fg))
for theme, bg, fg in (("wood", DARK, TEXT_D), ("leather", LIGHT, TEXT_L)):
    em = [(n.replace("emblem_", "").replace(f"-{theme}", ""), p) for n, p in pick("emblems", f"emblem_*-{theme}.webp")]
    dc = [(n.replace("decor_", "").replace(f"-{theme}", ""), p) for n, p in pick("decor", f"decor_*-{theme}.webp")]
    if em or dc:
        parts.append(grid(f"Section emblems + decor — {theme}", em + dc, 220, 6, bg, fg))
W = max(p.width for p in parts); H = sum(p.height for p in parts)
sheet = Image.new("RGB", (W, H), DARK); y = 0
for p in parts:
    sheet.paste(p, (0, y)); y += p.height
sheet.save(OUT, "WEBP", quality=88, method=6)
print(OUT, sheet.size)
