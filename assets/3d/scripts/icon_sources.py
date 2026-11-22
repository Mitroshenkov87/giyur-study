"""Sources icon: stack of 3 thick leather-bound books (seforim).

Lying flat, then pitched toward the camera so the dark covers are the face
you read, with a parchment fore-edge and brass spine bands.
"""

import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import iconkit

# width, depth, height, local yaw, xy shift, leather RGBA
BOOKS = [
    ((1.28, 0.92, 0.26), math.radians(-8.0), (0.00, 0.00),
     (0.11, 0.040, 0.015, 1.0)),   # deep brown
    ((1.16, 0.86, 0.24), math.radians(6.0), (0.04, 0.03),
     (0.12, 0.022, 0.016, 1.0)),   # oxblood
    ((1.22, 0.90, 0.25), math.radians(-3.0), (-0.02, -0.02),
     (0.024, 0.040, 0.026, 1.0)),  # dark green-black
]


def build_book(metal, parchment, size, z, yaw, shift, color):
    root = iconkit.empty("book", location=(shift[0], shift[1], z),
                         rotation=(0, 0, yaw))
    w, d, h = size
    leather = iconkit.mat_leather_color("Cover", color, roughness=0.62)
    # Page block. Shifted toward -Y so the fore-edge sticks out past the cover.
    pages = iconkit.round_box("pages", (w - 0.10, d * 0.92, h * 0.70), parchment,
                              location=(0.0, -0.07, 0.0), bevel=0.012)
    top = iconkit.round_box("cover_top", (w, d, h * 0.22), leather,
                            location=(0.0, 0.02, h * 0.40), bevel=0.018)
    bot = iconkit.round_box("cover_bot", (w, d * 0.96, h * 0.18), leather,
                            location=(0.0, 0.02, -h * 0.40), bevel=0.015)
    # Spine along +Y, plus two bands that cross the cover (visible from above).
    spine = iconkit.round_box("spine", (w + 0.03, 0.10, h * 0.96), leather,
                              location=(0.0, d * 0.48, 0.0), bevel=0.012)
    parts = [pages, top, bot, spine]
    for bx in (-w * 0.22, w * 0.22):
        parts.append(iconkit.round_box(
            "band", (0.11, d * 0.82, 0.08), metal,
            location=(bx, 0.06, h * 0.50), bevel=0.012))
    for ob in parts:
        iconkit.parent_keep(ob, root)
    return root, parts


def build():
    common.reset_scene()
    common.setup_render()
    metal = common.theme_metal()
    parchment = iconkit.mat_parchment()

    # Pitch the whole stack so the covers face the 25° camera.
    stack = iconkit.empty("stack", rotation=(math.radians(-50.0), 0.0,
                                             math.radians(16.0)))
    objs = []
    z = 0.0
    for size, yaw, shift, color in BOOKS:
        h = size[2]
        zc = z + h * 0.5
        root, parts = build_book(metal, parchment, size, zc, yaw, shift, color)
        iconkit.parent_keep(root, stack)
        objs += parts
        z += h - 0.01

    common.frame_object(objs, margin=0.12, elevation=25.0, azimuth=30.0)
    common.setup_lights()
    common.render(common.variant_name("sources"), common.OUT_DIR)


build()
