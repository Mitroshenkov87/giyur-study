"""Seven-branched Temple menorah, gold, with small oil-lamp flames."""

import math
import os
import sys

import bpy
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common

CUP_TOP = 1.94
BRANCH_END_Z = 1.80


def branch_points(x_start, z_start, x_end, z_end, n=28):
    """Quarter circle centred on the stem axis at the cup height: the branch
    leaves the stem at (0, z_end - R) and curves outward and up to the cup
    at (x_end, z_end).  Paired left/right these form the classic U-shaped
    concentric arms of the Temple menorah (outer arm leaves the stem lowest)."""
    r = abs(x_end)
    side = 1.0 if x_end >= 0 else -1.0
    pts = []
    for i in range(n + 1):
        t = (math.pi / 2) * i / n
        pts.append(Vector((side * r * math.sin(t), 0.0, z_end - r * math.cos(t))))
    return pts


def cup(objs, x, gold, flame_mat):
    objs.append(common.add_uv_sphere("cup_knop", 0.075, location=(x, 0, BRANCH_END_Z),
                                     mat=gold))
    objs.append(common.add_cone("cup", 0.11, 0.165, 0.14, location=(x, 0, 1.87),
                                mat=gold))
    objs.append(common.add_torus("cup_rim", 0.155, 0.012, location=(x, 0, CUP_TOP),
                                 mat=gold))
    objs.append(common.add_uv_sphere("flame", 0.05, location=(x, 0, CUP_TOP + 0.09),
                                     scale=(1.0, 1.0, 1.6), mat=flame_mat,
                                     segments=32, rings=16))


def build():
    common.reset_scene()
    common.setup_render()
    gold = common.mat_gold()
    flame_mat = common.mat_flame()
    objs = []

    base1 = common.add_cylinder("base_wide", 0.55, 0.10, location=(0, 0, 0.05), mat=gold)
    common.bevel(base1, 0.02)
    base2 = common.add_cylinder("base_mid", 0.44, 0.12, location=(0, 0, 0.15), mat=gold)
    common.bevel(base2, 0.02)
    base_knop = common.add_uv_sphere("base_knop", 0.22, location=(0, 0, 0.26),
                                     scale=(1.0, 1.0, 0.65), mat=gold)
    stem = common.add_cylinder("stem", 0.085, 1.30, location=(0, 0, 0.90), mat=gold)
    common.smooth(stem)
    objs += [base1, base2, base_knop, stem]

    for z in (0.55, 0.95, 1.32):
        objs.append(common.add_uv_sphere("stem_knop", 0.13, location=(0, 0, z),
                                         scale=(1.0, 1.0, 0.75), mat=gold))
    for z in (0.72, 1.12):
        objs.append(common.add_torus("stem_ring", 0.10, 0.028, location=(0, 0, z),
                                     mat=gold))

    # central cup on top of the stem
    objs.append(common.add_cylinder("center_cup_stem", 0.045, 0.14,
                                    location=(0, 0, 1.53), mat=gold))
    objs.append(common.add_uv_sphere("center_cup_knop", 0.09, location=(0, 0, 1.60),
                                     mat=gold))
    objs.append(common.add_cone("center_cup", 0.11, 0.17, 0.14,
                                location=(0, 0, 1.87), mat=gold))
    objs.append(common.add_torus("center_cup_rim", 0.16, 0.012,
                                 location=(0, 0, CUP_TOP), mat=gold))
    objs.append(common.add_uv_sphere("center_flame", 0.05,
                                     location=(0, 0, CUP_TOP + 0.09),
                                     scale=(1.0, 1.0, 1.6), mat=flame_mat,
                                     segments=32, rings=16))

    # three branches per side, arcs rising from the stem
    ends = (0.55, 0.95, 1.35)
    starts = tuple(BRANCH_END_Z - e for e in ends)
    for side in (-1, 1):
        for x_end, z_start in zip(ends, starts):
            pts = branch_points(side * 0.10, z_start, side * x_end, BRANCH_END_Z)
            br = common.tube_curve("branch", pts, 0.045, mat=gold)
            objs.append(br)
            cup(objs, side * x_end, gold, flame_mat)

    common.frame_object(objs, azimuth=6.0, elevation=12.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render("menorah_temple", common.OUT_DIR)


build()
