"""Emblem: three high-backed chairs for the three judges (beit din)."""

import math
import os
import sys

import bpy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import emblemkit

HALF_W = 0.145
BACK_TOP = 0.36
SEAT_Z = -0.16


def arch(cx, half_w, z0, z_top, n=14):
    """CCW rounded-top rectangle, as seen from the camera."""
    pts = [(cx - half_w, z0), (cx + half_w, z0), (cx + half_w, z_top)]
    for i in range(1, n):
        a = math.pi * i / n
        pts.append((cx + half_w * math.cos(a), z_top + half_w * math.sin(a)))
    pts.append((cx - half_w, z_top))
    return pts


def chair(objs, cx, metal, accent):
    zr = emblemkit.RELIEF_Y
    objs.append(emblemkit.extrude_polygon(
        "back", arch(cx, HALF_W, -0.02, BACK_TOP),
        zr, 0.055, metal, bevel_width=0.012))
    # Inset panel on the camera side of the back.
    objs.append(emblemkit.extrude_polygon(
        "back_inset", arch(cx, HALF_W * 0.62, 0.04, BACK_TOP - 0.02, n=10),
        zr - 0.040, 0.028, accent, bevel_width=0.006))

    seat = [(cx - 0.175, SEAT_Z - 0.06), (cx + 0.175, SEAT_Z - 0.06),
            (cx + 0.175, SEAT_Z + 0.05), (cx - 0.175, SEAT_Z + 0.05)]
    objs.append(emblemkit.extrude_polygon(
        "seat", seat, zr - 0.01, 0.09, metal, bevel_width=0.012))

    for sx in (-1, 1):
        lx = cx + sx * 0.125
        leg = [(lx - 0.030, -0.50), (lx + 0.030, -0.50),
               (lx + 0.030, SEAT_Z - 0.04), (lx - 0.030, SEAT_Z - 0.04)]
        objs.append(emblemkit.extrude_polygon(
            "leg", leg, zr - 0.005, 0.055, metal, bevel_width=0.008))


def build():
    common.reset_scene()
    common.setup_render()
    objs, metal, accent = emblemkit.medallion()
    for cx in (-0.40, 0.0, 0.40):
        chair(objs, cx, metal, accent)
    emblemkit.finish(objs, "beit_din")


build()
