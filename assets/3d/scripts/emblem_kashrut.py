"""Emblem: kosher fish with fins and scales (kashrut)."""

import math
import os
import sys

import bpy
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import emblemkit

# Head at +x. Separate convex-ish parts so the bevel stays well behaved.
BODY = [
    (0.46, 0.00), (0.36, 0.15), (0.10, 0.23), (-0.16, 0.21),
    (-0.36, 0.10), (-0.36, -0.10), (-0.16, -0.19), (0.10, -0.21),
    (0.36, -0.13),
]
TAIL = [
    (-0.30, 0.09), (-0.58, 0.30), (-0.44, 0.02),
    (-0.58, -0.26), (-0.30, -0.09),
]
DORSAL = [(-0.08, 0.16), (-0.02, 0.40), (0.16, 0.38), (0.26, 0.14)]
PECTORAL = [(0.16, -0.02), (0.36, -0.24), (0.28, 0.02)]
PELVIC = [(-0.02, -0.14), (0.10, -0.32), (0.16, -0.10)]


def build():
    common.reset_scene()
    common.setup_render()
    objs, metal, accent = emblemkit.medallion()
    zr = emblemkit.RELIEF_Y
    ry = emblemkit.R_Y
    syms = []

    syms.append(emblemkit.extrude_polygon(
        "dorsal_fin", DORSAL, zr + 0.01, 0.07, metal, bevel_width=0.012))
    syms.append(emblemkit.extrude_polygon(
        "pectoral_fin", PECTORAL, zr + 0.005, 0.075, metal, bevel_width=0.01))
    syms.append(emblemkit.extrude_polygon(
        "pelvic_fin", PELVIC, zr + 0.005, 0.07, metal, bevel_width=0.01))
    syms.append(emblemkit.extrude_polygon(
        "tail", TAIL, zr + 0.005, 0.085, metal, bevel_width=0.012))
    syms.append(emblemkit.extrude_polygon(
        "body", BODY, zr, 0.105, metal, bevel_width=0.016))

    front = zr - 0.10
    gill = []
    for i in range(12):
        a = math.radians(110 + 220 * i / 11)
        gill.append(Vector((0.30 + 0.16 * math.cos(a), front - 0.012,
                            0.0 + 0.16 * math.sin(a))))
    syms.append(common.tube_curve("gill", gill, 0.016, mat=accent))

    # Eye: pale ring + dark pupil, both proud of the head.
    syms.append(common.add_uv_sphere(
        "eye_white", 0.055, location=(0.32, front - 0.012, 0.07),
        scale=(1.0, 0.45, 1.0), mat=emblemkit.parchment(),
        segments=24, rings=12))
    pupil = emblemkit.ink()
    syms.append(common.add_uv_sphere(
        "eye", 0.030, location=(0.335, front - 0.028, 0.075),
        scale=(1.0, 0.45, 1.0), mat=pupil, segments=20, rings=10))

    # Overlapping scale discs on the flank.
    for row, zrow in enumerate((-0.08, 0.02, 0.11)):
        for col in range(4):
            x = -0.22 + col * 0.12 + (0.06 if row % 2 else 0.0)
            sc = common.add_uv_sphere(
                "scale", 0.058, location=(x, front - 0.01, zrow),
                scale=(1.05, 0.32, 0.82), mat=metal, segments=16, rings=8)
            syms.append(sc)

    emblemkit.fit_xz(syms, 0.60)
    objs.extend(syms)
    emblemkit.finish(objs, "kashrut")


build()
