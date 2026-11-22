"""Emblem: balance scales (etika = ethics)."""

import math
import os
import sys

import bpy
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import emblemkit

# Shallow bowl, opening up. Radii are large enough to read at 64 px.
PAN_PROFILE = [
    (0.0, -0.035), (0.11, -0.035), (0.155, -0.01), (0.175, 0.045),
    (0.150, 0.050), (0.128, 0.012), (0.0, 0.012),
]


def build():
    common.reset_scene()
    common.setup_render()
    objs, metal, accent = emblemkit.medallion()
    ry = emblemkit.R_Y

    base = common.add_cylinder(
        "base", 0.24, 0.07, location=(0.0, ry, -0.48), vertices=40, mat=metal)
    common.bevel(base, 0.016, segments=2)
    objs.append(base)
    objs.append(common.add_uv_sphere(
        "base_dome", 0.09, location=(0.0, ry, -0.42), mat=metal,
        segments=24, rings=12))
    post = common.add_cylinder(
        "post", 0.042, 0.78, location=(0.0, ry, -0.02), vertices=24, mat=metal)
    common.smooth(post)
    objs.append(post)
    objs.append(common.add_torus(
        "post_collar", 0.062, 0.016, location=(0.0, ry, 0.34),
        mat=metal, major_segments=28, minor_segments=10))
    objs.append(common.add_uv_sphere(
        "top_finial", 0.055, location=(0.0, ry, 0.50), mat=metal,
        segments=20, rings=10))

    beam = common.add_cylinder(
        "beam", 0.030, 1.02, location=(0.0, ry, 0.44), vertices=20, mat=metal)
    beam.rotation_euler[1] = math.radians(90)
    common.smooth(beam)
    objs.append(beam)

    for sx in (-1, 1):
        objs.append(common.add_uv_sphere(
            "beam_knob", 0.048, location=(sx * 0.52, ry, 0.44),
            mat=metal, segments=20, rings=10))
        px = sx * 0.40
        pan = common.lathe("pan", PAN_PROFILE, segments=36, mat=metal)
        pan.location = (px, ry, 0.05)
        common.bevel(pan, 0.006, segments=1)
        objs.append(pan)
        # Two thick chains, left and right of the pan, plus one centre.
        for k, ang in enumerate((50.0, 130.0, 90.0)):
            a = math.radians(ang)
            rim = Vector((px + 0.15 * math.cos(a), ry,
                          0.10 + 0.05 * math.sin(a)))
            top = Vector((px + (0.04 if k != 2 else 0.0) * sx, ry, 0.44))
            mid = (rim + top) * 0.5 + Vector((0.0, 0.0, -0.04))
            objs.append(common.tube_curve(
                "chain", [top, mid, rim], 0.014, mat=metal, resolution=2))

    emblemkit.finish(objs, "etika")


build()
