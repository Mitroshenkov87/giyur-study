#!/usr/bin/env python3
"""Convert every <name>.png in a directory to <name>.webp (1024) +
<name>-512.webp + <name>-256.webp, alpha kept, then delete the PNG.

The object is auto-centred: the bounding box of solid alpha (the object, not
its faint contact shadow) is centred in a square canvas with a fixed margin,
so every object in the set sits the same way. The faint shadow is faded out
towards the canvas border so it never ends in a hard edge."""

import glob
import os
import sys

from PIL import Image, ImageChops, ImageDraw, ImageFilter

QUALITY = 92
METHOD = 6
SIZE = 1024
FILL = 0.84        # object's longer side spans 84% of the canvas
SOLID = 160        # alpha threshold that counts as "object"


def _edge_fade(size, border):
    m = Image.new("L", (size, size), 0)
    d = ImageDraw.Draw(m)
    d.rectangle([border, border, size - border, size - border], fill=255)
    return m.filter(ImageFilter.GaussianBlur(border / 2))


def fit(im):
    a = im.getchannel("A")
    box = a.point(lambda v: 255 if v >= SOLID else 0).getbbox() or a.getbbox()
    if box is None:
        return im.resize((SIZE, SIZE), Image.LANCZOS)
    l, t, r, b = box
    cx, cy = (l + r) / 2, (t + b) / 2
    side = max(r - l, b - t) / FILL
    scale = SIZE / side
    big = im.resize((max(1, round(im.width * scale)), max(1, round(im.height * scale))),
                    Image.LANCZOS)
    out = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    out.paste(big, (round(SIZE / 2 - cx * scale), round(SIZE / 2 - cy * scale)))
    alpha = ImageChops.multiply(out.getchannel("A"), _edge_fade(SIZE, SIZE // 14))
    out.putalpha(alpha)
    return out


def convert(out_dir):
    pngs = sorted(glob.glob(os.path.join(out_dir, "*.png")))
    if not pngs:
        print(f"to_webp: no PNG files in {out_dir}")
        return
    for path in pngs:
        name = os.path.splitext(os.path.basename(path))[0]
        im = fit(Image.open(path).convert("RGBA"))
        base = os.path.join(out_dir, name)
        im.save(base + ".webp", "WEBP", quality=QUALITY, method=METHOD)
        for s in (512, 256):
            im.resize((s, s), Image.LANCZOS).save(f"{base}-{s}.webp", "WEBP",
                                                  quality=QUALITY, method=METHOD)
        os.remove(path)
        print(f"to_webp: {name}.png -> {name}.webp + -512 + -256")


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("usage: to_webp.py <out_dir>")
        sys.exit(2)
    convert(sys.argv[1])
