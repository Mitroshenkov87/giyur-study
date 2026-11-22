"""Tefillin: black leather shel-rosh and shel-yad boxes on their titura bases,
with coiled black straps (retzuot) beside them."""

import math
import os
import sys

import bpy
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common

BLACK = (0.016, 0.013, 0.011, 1.0)


def box_on_titura(name, center_x, center_y, titura_w, bayit_s, leather, h=0.46):
    """Square titura slab with a sharp-edged bayit cube on top."""
    titura = common.add_cylinder(
        name + "_titura", titura_w * 0.5 * math.sqrt(2.0), 0.07,
        location=(center_x, center_y, 0.035), vertices=4, mat=leather)
    titura.rotation_euler.z = math.radians(45.0)
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    common.bevel(titura, 0.012, segments=2)
    bayit = common.add_cylinder(
        name + "_bayit", bayit_s * 0.5 * math.sqrt(2.0), h,
        location=(center_x, center_y, 0.07 + h * 0.5), vertices=4, mat=leather)
    bayit.rotation_euler.z = math.radians(45.0)
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    common.bevel(bayit, 0.012, segments=2)
    return titura, bayit


def coil(center, r0, r1, turns, z0, rise, leather, radius=0.05, flat=0.45):
    """Tightly rolled strap: flat spiral rising slightly, flattened to a band."""
    n = 120
    pts = []
    for i in range(n + 1):
        t = i / n
        ang = 2.0 * math.pi * turns * t
        r = r0 + (r1 - r0) * t
        pts.append(Vector((center[0] + r * math.cos(ang),
                           center[1] + r * math.sin(ang),
                           z0 + rise * t)))
    ob = common.tube_curve("coil", pts, radius, mat=leather, resolution=3)
    ob.scale.z = flat
    return ob


def strap_segment(name, pts, leather, radius=0.05, flat=0.45):
    ob = common.tube_curve(name, pts, radius, mat=leather, resolution=3)
    ob.scale.z = flat
    return ob


def build():
    common.reset_scene()
    common.setup_render()
    leather = common.mat_leather(BLACK)

    objs = []

    # shel rosh (head) — the larger box, on the left
    rosh_t, rosh_b = box_on_titura("rosh", -0.62, 0.12, 0.66, 0.47, leather)
    objs += [rosh_t, rosh_b]

    # shel yad (arm) — slightly smaller, on the right and forward
    yad_t, yad_b = box_on_titura("yad", 0.42, -0.18, 0.60, 0.42, leather, h=0.40)
    objs += [yad_t, yad_b]

    # ma'abarta: small strap loop standing on top of the rosh bayit
    loop_pts = []
    for i in range(13):
        a = math.pi * i / 12
        loop_pts.append(Vector((-0.62 + 0.055 * math.cos(a), 0.12,
                                0.53 + 0.10 * math.sin(a))))
    loop = strap_segment("rosh_loop", loop_pts, leather, radius=0.024, flat=0.6)
    loop.scale.y = 0.5
    objs.append(loop)

    # strap tail from the yad box lying on the ground
    objs.append(strap_segment(
        "yad_tail",
        [Vector((0.42, -0.48, 0.06)), Vector((0.55, -0.62, 0.045)),
         Vector((0.85, -0.66, 0.04)), Vector((1.05, -0.58, 0.04))],
        leather))

    # coiled retzuot beside the boxes
    objs.append(coil((1.30, -0.28), 0.09, 0.30, 4.5, 0.05, 0.09, leather))
    objs.append(strap_segment(
        "coilA_tail",
        [Vector((1.30 + 0.30 * math.cos(math.pi * 9), -0.28 + 0.30 * math.sin(math.pi * 9), 0.14)),
         Vector((1.05, 0.05, 0.05)), Vector((0.80, 0.28, 0.045))],
        leather))
    objs.append(coil((-1.35, -0.55), 0.08, 0.24, 4.0, 0.05, 0.08, leather,
                     radius=0.045))
    objs.append(strap_segment(
        "coilB_tail",
        [Vector((-1.35 + 0.24 * math.cos(math.pi * 8), -0.55 + 0.24 * math.sin(math.pi * 8), 0.13)),
         Vector((-1.05, -0.15, 0.05)), Vector((-0.78, 0.02, 0.045))],
        leather, radius=0.045))

    common.frame_object(objs, margin=0.1, focal=72.0,
                        elevation=28.0, azimuth=35.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render("tefillin", common.OUT_DIR)


build()
