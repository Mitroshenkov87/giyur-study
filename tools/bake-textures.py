#!/usr/bin/env python3
"""Bake the theme backgrounds (assets/img/{bg,side}-{dark,parchment}.webp)
from the raw, text-free material textures in assets/textures/.

The veil, lamp light and vignette used to be drawn at runtime as stacked
full-window gradients; baking them keeps the renderer to one image layer.
Requires Pillow and numpy. Output is WebP only.
"""
import numpy as np
from PIL import Image

SRC = "assets/textures/{}.webp"
OUT = "assets/img/{}.webp"
H = W = 1024
y = np.linspace(0, 1, H)[:, None, None]
x = np.linspace(0, 1, W)[None, :, None]


def rgb(v):
    return np.array([(v >> 16) & 255, (v >> 8) & 255, v & 255], np.float32) / 255


def over(base, colour, a):
    return base * (1 - a) + rgb(colour) * a


def load(name):
    return np.asarray(Image.open(SRC.format(name)).convert("RGB"), np.float32) / 255


def bake(tex, veil, veil_a, lamp=None, tint=None, vignette=None):
    b = over(load(tex), veil, veil_a)
    if tint:
        b = over(b, *tint)
    if lamp:
        b = over(b, lamp, np.clip(1 - y / 0.45, 0, 1) * 0.10)  # warm pool from the top
        b = over(b, lamp, (1 - np.abs(x - 0.5) / 0.5) * 0.05)  # brightest mid-column
    if vignette:
        g = np.clip(1 - y / 0.15, 0, 1) * 0.10 + np.clip((y - 0.85) / 0.15, 0, 1) * 0.012
        b = over(b, vignette, g)
    return Image.fromarray((np.clip(b, 0, 1) * 255).astype("uint8"))


if __name__ == "__main__":
    out = {
        "bg-dark": bake("tex-walnut", 0x120B08, 0.35, lamp=0xFFBE6E),
        "side-dark": bake("tex-leather", 0x120B08, 0.45),
        "bg-parchment": bake("tex-klaf", 0xE8D6B0, 0.55, lamp=0xFFF1CC, vignette=0x3A2510),
        "side-parchment": bake("tex-klaf", 0xE8D6B0, 0.55, tint=(0x6B4712, 0.06)),
    }
    for name, im in out.items():
        im.save(OUT.format(name), quality=80, method=6)
        print(name, im.resize((1, 1)).getpixel((0, 0)))
