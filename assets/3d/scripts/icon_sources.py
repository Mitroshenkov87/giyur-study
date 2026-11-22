"""Sources icon: stack of 3 thick leather-bound books (seforim) with brass
spine bands, lying flat."""

import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import iconkit

# (width, depth, height), z-centre, yaw, x/y shift
BOOKS = [
    ((1.18, 0.86, 0.24), 0.12, math.radians(-6.0), (0.0, 0.0)),
    ((1.08, 0.80, 0.22), 0.35, math.radians(5.0), (0.05, -0.03)),
    ((1.12, 0.84, 0.24), 0.58, math.radians(-3.0), (-0.03, 0.04)),
]


def build_book(metal, accent, parchment, size, z, yaw, shift):
    root = iconkit.empty("book", location=(shift[0], shift[1], 0),
                         rotation=(0, 0, yaw))
    w, d, h = size
    body = iconkit.round_box("book_body", size, accent,
                             location=(0, 0, z), bevel=0.035)
    pages = iconkit.round_box("book_pages", (w - 0.07, d - 0.07, 0.055),
                              parchment, location=(0, 0, z + h * 0.5 + 0.017),
                              bevel=0.02)
    parts = [body, pages]
    for bx in (-w * 0.30, w * 0.30):
        parts.append(iconkit.round_box(
            "spine_band", (0.08, d + 0.05, 0.065), metal,
            location=(bx, 0, z), bevel=0.02))
    for ob in parts:
        iconkit.parent_keep(ob, root)
    return parts


def build():
    common.reset_scene()
    common.setup_render()
    metal = common.theme_metal()
    accent = common.theme_accent()
    parchment = common.mat_parchment()

    objs = []
    for size, z, yaw, shift in BOOKS:
        objs += build_book(metal, accent, parchment, size, z, yaw, shift)

    common.frame_object(objs, margin=0.12, elevation=25.0, azimuth=30.0)
    common.setup_lights()
    common.render(common.variant_name("sources"), common.OUT_DIR)


build()
