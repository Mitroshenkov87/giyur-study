"""Quiz icon: short brass ladder of 5 rungs climbing to a small crown."""

import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import iconkit


def _crown_point(name, width, height, thick, mat, x, z):
    """Triangle in XY, stood up in the XZ plane (facing the camera)."""
    pts = [(-width * 0.5, 0.0), (width * 0.5, 0.0), (0.0, height)]
    ob = iconkit.prism(name, pts, thick, mat, bevel_w=0.012)
    ob.rotation_euler = (math.radians(90), 0, 0)
    ob.location = (x, 0.0, z)
    return ob


def build():
    common.reset_scene()
    common.setup_render()
    metal = common.theme_metal()

    # Slight lean back so the ladder has depth but the rungs stay horizontal.
    lean = iconkit.empty("lean", rotation=(math.radians(-8.0), 0, 0))
    parts = []
    rail_h = 1.42
    rail_x = 0.46
    for x in (-rail_x, rail_x):
        rail = common.add_cylinder("rail", 0.078, rail_h,
                                   location=(x, 0, rail_h * 0.5), mat=metal)
        common.smooth(rail)
        parts.append(rail)
        foot = common.add_uv_sphere("foot", 0.10, location=(x, 0, 0.02),
                                    scale=(1.15, 1.15, 0.55), mat=metal)
        parts.append(foot)

    # Cylinders are born along Z. Ry +90 lays them along X, rail to rail.
    for z in (0.24, 0.48, 0.72, 0.96, 1.20):
        rung = common.add_cylinder("rung", 0.058, rail_x * 2 + 0.20,
                                   location=(0, 0, z), mat=metal)
        rung.rotation_euler = (0, math.radians(90), 0)
        common.smooth(rung)
        parts.append(rung)

    band_z = rail_h + 0.02
    band = iconkit.round_box("crown_band", (1.02, 0.16, 0.14), metal,
                             location=(0, 0, band_z), bevel=0.025)
    parts.append(band)
    # Five points in a row: short, tall, tallest, tall, short. Reads as a
    # crown at 48 px, not a ring of cones on a disc.
    specs = ((-0.38, 0.20, 0.16), (-0.19, 0.30, 0.16),
             (0.0, 0.40, 0.18), (0.19, 0.30, 0.16), (0.38, 0.20, 0.16))
    for i, (x, h, w) in enumerate(specs):
        parts.append(_crown_point("point%d" % i, w, h, 0.12, metal,
                                  x, band_z + 0.05))

    for ob in parts:
        iconkit.parent_keep(ob, lean)

    common.frame_object(parts, margin=0.12, elevation=25.0, azimuth=30.0)
    common.setup_lights()
    common.render(common.variant_name("quiz"), common.OUT_DIR)


build()
