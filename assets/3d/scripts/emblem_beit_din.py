"""Emblem: three high-backed chairs for the three judges (beit din)."""

import math
import os
import sys

import bpy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import emblemkit

BACK_W = 0.20
BACK_TOP = 0.42
SEAT_Z = (-0.28, -0.19)


def chair(objs, cx, metal, accent):
    zr = emblemkit.RELIEF_Y

    # back: rectangle with a rounded (semicircular) top, plus inset panel
    outline = [(-BACK_W, -0.05), (-BACK_W, BACK_TOP)]
    for i in range(17):
        a = math.radians(180 - 180 * i / 16)
        outline.append((BACK_W * math.cos(a), BACK_TOP + BACK_W * math.sin(a)))
    outline.append((BACK_W, -0.05))
    outline = [(x + cx, z) for x, z in outline]
    objs.append(emblemkit.extrude_polygon(
        "back", outline, zr, 0.06, metal, bevel_width=0.014))

    inset = [(-0.13, 0.02), (-0.13, BACK_TOP)]
    for i in range(13):
        a = math.radians(180 - 180 * i / 12)
        inset.append((0.13 * math.cos(a), BACK_TOP + 0.13 * math.sin(a)))
    inset.append((0.13, 0.02))
    inset = [(x + cx, z) for x, z in inset]
    objs.append(emblemkit.extrude_polygon(
        "back_inset", inset, zr - 0.014, 0.014, accent, bevel_width=0.008))

    # gold cord along the top of the back
    cord = []
    for i in range(12):
        a = math.radians(20 + 140 * i / 11)
        cord.append((cx + (BACK_W + 0.008) * math.cos(a), zr - 0.055,
                     BACK_TOP + (BACK_W + 0.008) * math.sin(a)))
    objs.append(common.tube_curve("back_cord", cord, 0.018, mat=metal))

    # seat (proud of the back) and two front legs
    seat = [(-0.21 + cx, SEAT_Z[0]), (0.21 + cx, SEAT_Z[0]),
            (0.21 + cx, SEAT_Z[1]), (-0.21 + cx, SEAT_Z[1])]
    objs.append(emblemkit.extrude_polygon(
        "seat", seat, zr + 0.02, 0.10, metal, bevel_width=0.014))
    for sx in (-1, 1):
        leg = [(-0.028 + cx + sx * 0.16, -0.46), (0.028 + cx + sx * 0.16, -0.46),
               (0.028 + cx + sx * 0.16, SEAT_Z[0]), (-0.028 + cx + sx * 0.16, SEAT_Z[0])]
        objs.append(emblemkit.extrude_polygon(
            "leg", leg, zr, 0.06, metal, bevel_width=0.01))


def build():
    common.reset_scene()
    common.setup_render()
    objs, metal, accent = emblemkit.medallion()

    for cx in (-0.52, 0.0, 0.52):
        chair(objs, cx, metal, accent)

    emblemkit.finish(objs, "beit_din")


build()
