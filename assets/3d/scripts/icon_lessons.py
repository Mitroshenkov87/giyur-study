"""Lessons icon: open book with fanned pages and brass corner caps."""

import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import iconkit


def build_side(sign, metal, accent, parchment):
    """One cover + page block, hinged at the spine (origin), outer edge up."""
    root = iconkit.empty("SideL" if sign < 0 else "SideR",
                         rotation=(0, -math.radians(16.0) * sign, 0))
    cover = iconkit.round_box("cover", (0.76, 1.46, 0.08), accent,
                              location=(sign * 0.40, 0.0, 0.04), bevel=0.03)
    pages = iconkit.round_box("pages", (0.70, 1.40, 0.22), parchment,
                              location=(sign * 0.39, 0.0, 0.19), bevel=0.035)
    # two fanned sheets sliding toward the outer edge
    sheet1 = iconkit.round_box("sheet1", (0.70, 1.41, 0.035), parchment,
                               location=(sign * 0.40, 0.0, 0.308), bevel=0.015)
    sheet2 = iconkit.round_box("sheet2", (0.70, 1.41, 0.035), parchment,
                               location=(sign * 0.41, 0.0, 0.343), bevel=0.015)
    caps = []
    for sy in (-1, 1):
        caps.append(iconkit.round_box(
            "corner_cap", (0.14, 0.14, 0.13), metal,
            location=(sign * 0.80, sy * 0.63, 0.145), bevel=0.03))
    parts = [cover, pages, sheet1, sheet2] + caps
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
    objs += build_side(-1, metal, accent, parchment)
    objs += build_side(1, metal, accent, parchment)

    spine = common.add_cylinder("spine", 0.105, 1.46, location=(0, 0, 0.075),
                                mat=accent)
    spine.rotation_euler[0] = math.radians(90)
    common.smooth(spine)
    objs.append(spine)
    for sy in (-1, 1):
        band = common.add_torus("spine_band", 0.105, 0.022,
                                location=(0, sy * 0.42, 0.075), mat=metal)
        band.rotation_euler[0] = math.radians(90)
        objs.append(band)

    common.frame_object(objs, margin=0.12, elevation=25.0, azimuth=30.0)
    common.setup_lights()
    common.render(common.variant_name("lessons"), common.OUT_DIR)


build()
