"""Emblem: kosher fish with fins and scales (kashrut)."""

import math
import os
import sys

import bpy
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import emblemkit

# body + forked tail outline, CCW as seen from the camera; head at +x
BODY = [
    (0.60, 0.0), (0.48, 0.14), (0.15, 0.24), (-0.15, 0.26), (-0.42, 0.14),
    (-0.60, 0.30), (-0.52, 0.0), (-0.60, -0.30), (-0.42, -0.14),
    (-0.15, -0.26), (0.15, -0.22), (0.48, -0.12),
]
DORSAL = [(-0.28, 0.22), (-0.18, 0.38), (-0.06, 0.24), (0.06, 0.40),
          (0.18, 0.22)]
PECTORAL = [(0.10, -0.08), (0.26, -0.27), (0.33, -0.05)]


def build():
    common.reset_scene()
    common.setup_render()
    objs, metal, accent = emblemkit.medallion()
    zr = emblemkit.RELIEF_Y
    ry = emblemkit.R_Y

    # fins slightly behind the body front to avoid coplanar overlap
    objs.append(emblemkit.extrude_polygon(
        "dorsal_fin", DORSAL, zr + 0.015, 0.075, metal, bevel_width=0.01))
    objs.append(emblemkit.extrude_polygon(
        "pectoral_fin", PECTORAL, zr + 0.015, 0.075, metal, bevel_width=0.01))
    objs.append(emblemkit.extrude_polygon(
        "body", BODY, zr, 0.08, metal, bevel_width=0.015))

    # gill arc behind the head
    gill = []
    for i in range(13):
        a = math.radians(120 + 240 * i / 12)
        gill.append(Vector((0.44 + 0.20 * math.cos(a), ry - 0.075,
                            0.0 + 0.20 * math.sin(a))))
    objs.append(common.tube_curve("gill", gill, 0.018, mat=accent))

    # eye
    objs.append(common.add_uv_sphere(
        "eye", 0.05, location=(0.44, ry - 0.075, 0.075), scale=(1.0, 0.45, 1.0),
        mat=accent, segments=24, rings=12))

    # staggered scale bumps
    for row, zrow in enumerate((-0.10, 0.0, 0.10)):
        for col in range(4):
            x = -0.25 + col * 0.135 + (0.055 if row % 2 else 0.0)
            if row == 2:
                x -= 0.02
            objs.append(common.add_uv_sphere(
                "scale", 0.055, location=(x, ry - 0.075, zrow),
                scale=(1.0, 0.4, 0.85), mat=accent, segments=20, rings=10))

    emblemkit.finish(objs, "kashrut")


build()
