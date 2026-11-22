"""Lessons icon: open book with fanned pages and brass corner caps."""

import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import iconkit


def build_side(sign, metal, accent, parchment):
    """One cover + page block, hinged at the spine (origin), outer edge up."""
    # Hinge around the spine (Y through the origin). Outer edge lifts.
    root = iconkit.empty("SideL" if sign < 0 else "SideR",
                         rotation=(0, -math.radians(28.0) * sign, 0))
    cover = iconkit.round_box("cover", (0.78, 1.50, 0.09), accent,
                              location=(sign * 0.42, 0.0, 0.045), bevel=0.025)
    pages = iconkit.round_box("pages", (0.72, 1.42, 0.20), parchment,
                              location=(sign * 0.40, 0.0, 0.185), bevel=0.02)
    # two sheets fanned a little further open than the cover
    sheet1 = iconkit.round_box("sheet1", (0.70, 1.38, 0.045), parchment,
                               location=(sign * 0.42, 0.0, 0.30), bevel=0.012,
                               rotation=(0, -math.radians(6.0) * sign, 0))
    sheet2 = iconkit.round_box("sheet2", (0.68, 1.34, 0.04), parchment,
                               location=(sign * 0.44, 0.0, 0.345), bevel=0.012,
                               rotation=(0, -math.radians(12.0) * sign, 0))
    caps = []
    for sy in (-1, 1):
        caps.append(iconkit.round_box(
            "corner_cap", (0.18, 0.18, 0.16), metal,
            location=(sign * 0.74, sy * 0.64, 0.16), bevel=0.03))
    parts = [cover, pages, sheet1, sheet2] + caps
    for ob in parts:
        iconkit.parent_keep(ob, root)
    return parts


def build():
    common.reset_scene()
    common.setup_render()
    metal = common.theme_metal()
    accent = iconkit.accent()
    parchment = iconkit.mat_parchment()

    objs = []
    objs += build_side(-1, metal, accent, parchment)
    objs += build_side(1, metal, accent, parchment)

    spine = common.add_cylinder("spine", 0.11, 1.50, location=(0, 0, 0.08),
                                mat=accent)
    spine.rotation_euler[0] = math.radians(90)
    common.smooth(spine)
    objs.append(spine)
    for sy in (-1, 1):
        band = common.add_torus("spine_band", 0.11, 0.028,
                                location=(0, sy * 0.48, 0.08), mat=metal)
        band.rotation_euler[0] = math.radians(90)
        objs.append(band)

    common.frame_object(objs, margin=0.12, elevation=25.0, azimuth=30.0)
    common.setup_lights()
    common.render(common.variant_name("lessons"), common.OUT_DIR)


build()
