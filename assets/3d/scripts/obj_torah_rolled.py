"""Closed Sefer Torah standing upright: two wooden rollers side by side,
dressed in a deep burgundy velvet mantle with a gold embroidered band,
topped by two brass rimonim finials and a small gold crown on the front."""

import math
import os
import sys

import bpy
from mathutils import Matrix, Quaternion, Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import objkit

BELL_YSCALE = 0.8   # mantles are elliptical in plan, not round

# Velvet bell, bottom hem -> gathered cap.
BELL = [
    (0.000, 0.24), (0.66, 0.25), (0.74, 0.30), (0.77, 0.40),
    (0.74, 0.70), (0.68, 1.00), (0.61, 1.30), (0.545, 1.60),
    (0.49, 1.85), (0.45, 2.00), (0.42, 2.07), (0.34, 2.10),
    (0.22, 2.11), (0.000, 2.12),
]

# Wooden roller: rod with a small turned tip top and bottom.
ROLLER = [
    (0.000, 0.00), (0.075, 0.015), (0.065, 0.06), (0.065, 2.42),
    (0.058, 2.48), (0.045, 2.53), (0.040, 2.56), (0.000, 2.58),
]

# Brass rimon (pomegranate finial) seated on each roller top.
RIMON = [
    (0.000, 2.46), (0.105, 2.46), (0.105, 2.52),
    (0.055, 2.545),
    (0.09, 2.60), (0.10, 2.665), (0.09, 2.73),
    (0.048, 2.765),
    (0.078, 2.815), (0.078, 2.875), (0.042, 2.895),
    (0.06, 2.945), (0.05, 2.99),
    (0.02, 3.03), (0.033, 3.08), (0.000, 3.12),
]


def crown(gold):
    """Small gold crown (band + six prongs) sewn onto the front slope."""
    parts = []
    band = common.lathe("crown_band",
                        [(0.15, 0.0), (0.155, 0.02), (0.15, 0.10),
                         (0.138, 0.14)], segments=48, mat=gold)
    parts.append(band)
    for k in range(6):
        a = 2.0 * math.pi * k / 6.0
        r = 0.118
        loc = Vector((r * math.cos(a), r * math.sin(a), 0.205))
        axis = Vector((-math.sin(a), math.cos(a), 0.0))
        q = Quaternion(axis, math.radians(14.0))
        prong = common.add_cone("crown_prong", 0.042, 0.0, 0.15, location=loc, mat=gold)
        prong.rotation_euler = q.to_euler()
        parts.append(prong)
        tip = loc + (q @ Vector((0, 0, 1))) * 0.078
        parts.append(common.add_uv_sphere("crown_ball", 0.02, location=tip,
                                          mat=gold, segments=16, rings=8))
    pos = Vector((0.0, -0.443, 1.396))
    m = Matrix.Translation(pos) @ Matrix.Rotation(math.radians(78.0), 4, "X")
    for ob in parts:
        ob.matrix_world = m @ ob.matrix_world
    return parts


def build():
    common.reset_scene()
    common.setup_render()
    walnut = common.mat_walnut()
    velvet = common.mat_velvet()
    gold = common.mat_gold()
    objs = []

    bell = common.lathe("mantle", BELL, segments=96, mat=velvet)
    bell.scale.y = BELL_YSCALE
    objs.append(bell)

    for x in (-0.30, 0.30):
        roller = common.lathe("roller", ROLLER, segments=48, mat=walnut)
        roller.location = (x, 0.0, 0.0)
        objs.append(roller)
        rimon = common.lathe("rimon", RIMON, segments=48, mat=gold)
        rimon.location = (x, 0.0, 0.0)
        objs.append(rimon)
        collar = common.add_torus("collar", 0.075, 0.015, location=(x, 0, 2.16),
                                  mat=gold)
        objs.append(collar)

    # gold embroidery: two rings with a row of beads between, + hem band
    for z, r in ((1.70, 0.523), (1.86, 0.487)):
        ring = common.add_torus("gold_band", r, 0.020, location=(0, 0, z), mat=gold)
        ring.scale.y = BELL_YSCALE
        objs.append(ring)
    for k in range(12):
        a = 2.0 * math.pi * k / 12.0
        objs.append(common.add_uv_sphere(
            "bead", 0.027, location=(0.505 * math.cos(a),
                                     BELL_YSCALE * 0.505 * math.sin(a), 1.78),
            mat=gold, segments=16, rings=8))
    hem = common.add_torus("hem_band", 0.755, 0.024, location=(0, 0, 0.33), mat=gold)
    hem.scale.y = BELL_YSCALE
    objs.append(hem)

    objs += crown(gold)

    common.frame_object(objs)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render("torah_rolled", common.OUT_DIR)


build()
