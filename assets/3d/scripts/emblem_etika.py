"""Emblem: balance scales (etika = ethics)."""

import math
import os
import sys

import bpy
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import emblemkit

PAN_PROFILE = [(0.0, -0.04), (0.10, -0.04), (0.15, -0.01), (0.17, 0.045),
               (0.145, 0.045), (0.125, 0.0), (0.0, 0.0)]


def build():
    common.reset_scene()
    common.setup_render()
    objs, metal, accent = emblemkit.medallion()
    ry = emblemkit.R_Y

    # base + post
    base = common.add_cylinder("base", 0.22, 0.10, location=(0, ry, -0.43),
                               vertices=48, mat=metal)
    common.bevel(base, 0.02)
    objs.append(base)
    objs.append(common.add_uv_sphere("base_dome", 0.10,
                                     location=(0, ry, -0.345), mat=metal))
    post = common.add_cylinder("post", 0.040, 0.90, location=(0, ry, 0.045),
                               vertices=32, mat=metal)
    common.smooth(post)
    objs.append(post)
    for z in (-0.12, 0.12):
        objs.append(common.add_torus("post_collar", 0.052, 0.014,
                                     location=(0, ry, z), mat=metal,
                                     major_segments=32))
    objs.append(common.add_uv_sphere("top_finial", 0.055,
                                     location=(0, ry, 0.52), mat=metal))

    # beam
    beam = common.add_cylinder("beam", 0.028, 1.04, location=(0, ry, 0.485),
                               vertices=24, mat=metal)
    beam.rotation_euler[1] = math.radians(90)
    objs.append(beam)
    for sx in (-1, 1):
        objs.append(common.add_uv_sphere("beam_knob", 0.045,
                                         location=(sx * 0.53, ry, 0.485),
                                         mat=metal, segments=24, rings=12))

        # pan + three chains
        px = sx * 0.50
        pan = common.lathe("pan", PAN_PROFILE, segments=40, mat=metal)
        pan.location = (px, ry, 0.16)
        common.bevel(pan, 0.008, segments=2)
        objs.append(pan)
        for k in range(3):
            a = math.radians(90 + 120 * k)
            rim = Vector((px + 0.15 * math.cos(a), ry, 0.205 + 0.15 * math.sin(a)))
            top = Vector((px, ry, 0.485))
            mid = (rim + top) * 0.5 + Vector((0, 0, -0.012))
            objs.append(common.tube_curve("chain", [top, mid, rim], 0.011,
                                          mat=metal, resolution=2))

    emblemkit.finish(objs, "etika")


build()
