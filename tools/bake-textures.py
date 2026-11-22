#!/usr/bin/env python3
"""Bake the theme backgrounds (assets/img/{bg,side}-{dark,leather}.webp)
from the raw, text-free material textures in assets/textures/.

The veil, lamp light and vignette used to be drawn at runtime as stacked
full-window gradients; baking them keeps the renderer to one image layer.
Requires Pillow and numpy. Output is WebP only.
"""
import sys

import numpy as np
from PIL import Image

SRC = "assets/textures/{}.webp"
QUALITY = 86
OUT = "assets/img/{}.webp"
# 4K: window backgrounds are 3840x2160 (16:9); sidebars 480x2160, the
# 230-px nav at 200 % scale on a 2160-px-high screen. Both are drawn with
# ContentFit::Fill (the image bounds equal the widget bounds), so the
# renderer never has to crop an oversized image; the grain is
# non-directional enough that the slight stretch on other aspect ratios is
# invisible.
BG = (3840, 2160)
SIDE = (480, 2160)


def rgb(v):
    return np.array([(v >> 16) & 255, (v >> 8) & 255, v & 255], np.float32) / 255


def over(base, colour, a):
    return base * (1 - a) + rgb(colour) * a


def load(name, size):
    """Load a raw texture and centre-crop/scale it to `size` (Lanczos)."""
    im = Image.open(SRC.format(name)).convert("RGB")
    w, h = size
    s = max(w / im.width, h / im.height)
    im = im.resize((round(im.width * s), round(im.height * s)), Image.LANCZOS)
    left, top = (im.width - w) // 2, (im.height - h) // 2
    im = im.crop((left, top, left + w, top + h))
    return np.asarray(im, np.float32) / 255


def bake(tex, size, veil, veil_a, lamp=None, tint=None, vignette=None):
    global x, y
    w, h = size
    y = np.linspace(0, 1, h)[:, None, None]
    x = np.linspace(0, 1, w)[None, :, None]
    b = over(load(tex, size), veil, veil_a)
    if tint:
        b = over(b, *tint)
    if lamp:
        b = over(b, lamp, np.clip(1 - y / 0.45, 0, 1) * 0.10)  # warm pool from the top
        b = over(b, lamp, (1 - np.abs(x - 0.5) / 0.5) * 0.05)  # brightest mid-column
    if vignette:
        g = np.clip(1 - y / 0.15, 0, 1) * 0.10 + np.clip((y - 0.85) / 0.15, 0, 1) * 0.012
        b = over(b, vignette, g)
    # A whisper of film grain (±1 level) hides banding in the smooth
    # gradients after upscaling; seeded so re-bakes are reproducible.
    b = b + np.random.default_rng(5787).normal(0, 0.6 / 255, b.shape)
    return Image.fromarray((np.clip(b, 0, 1) * 255 + 0.5).astype("uint8"))


if __name__ == "__main__":
    out = {
        "bg-dark": bake("tex-walnut", BG, 0x120B08, 0.35, lamp=0xFFBE6E),
        "side-dark": bake("tex-leather", SIDE, 0x120B08, 0.45),
        # v0.5 hide theme: smoothed calf hide for the window (a light brown
        # veil darkens it a little so it never glares) and a darker
        # goatskin strip for the sidebar.
        "bg-leather": bake("tex-hide", BG, 0x4A2E18, 0.16, lamp=0xE6C89A, vignette=0x2A180A),
        "side-leather": bake("tex-hide-side", SIDE, 0x7A4E2E, 0.10),
    }
    only = set(sys.argv[1:])  # e.g. `bg-leather side-leather`
    for name, im in out.items():
        if only and name not in only:
            continue
        im.save(OUT.format(name), quality=QUALITY, method=6)
        print(name, im.resize((1, 1)).getpixel((0, 0)))
