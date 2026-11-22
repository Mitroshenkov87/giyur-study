"""A stack of three round hand-made (shmurah) matzot with wavy edges,
perforation rows and toasted spots."""

import math
import os
import sys

import bpy
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import foodkit

R = 0.56


def mat_dimple():
    return foodkit.prin("MatzahDimple", (0.38, 0.24, 0.10, 1.0), rough=0.85)


def make_matzah(idx, z, rot, mat, dimple_mat):
    # squashed sphere = thick hand-made disc with a gently bulged top
    ob = common.add_uv_sphere("matzah", 1.0, location=(0, 0, z),
                              scale=(R, R, 0.055), mat=mat,
                              segments=64, rings=32)
    p = idx * 1.7
    mesh = ob.data
    for v in mesh.vertices:
        x, y = v.co.x, v.co.y
        r = math.hypot(x, y)
        if r > 1e-6:
            a = math.atan2(y, x)
            f = 1.0 + (0.040 * math.sin(3 * a + p)
                       + 0.028 * math.sin(8 * a + 2.1 * p)
                       + 0.018 * math.sin(13 * a + 0.7 * p)) * (r / R)
            v.co.x = x * f
            v.co.y = y * f
        if abs(v.co.z) > 0.02 and r > 0.3:
            v.co.z += 0.012 * math.sin(5 * a + p) * (r / R)
    common.bevel(ob, 0.006, segments=2)
    common.smooth(ob)
    ob.rotation_euler.z = rot

    # perforation: three dotted rows across the top
    dots = []
    ca, sa = math.cos(rot), math.sin(rot)
    for row in range(3):
        ang = rot + row * math.pi / 3.0
        ca, sa = math.cos(ang), math.sin(ang)
        for k in range(-3, 4):
            rr = k * 0.13
            # top surface of the squashed sphere at radius rr
            zt = z + 0.055 * math.sqrt(max(0.0, 1.0 - (rr / R) ** 2))
            dots.append(common.add_uv_sphere(
                "dimple", 0.016, location=(rr * ca, rr * sa, zt - 0.002),
                scale=(1.0, 1.0, 0.45), mat=dimple_mat,
                segments=10, rings=6))
    return [ob] + dots


def build():
    common.reset_scene()
    common.setup_render()
    matzah_mat = common.mat_matzah()
    dimple_mat = mat_dimple()

    objs = []
    placements = [
        (0.045, 0.4, (0.0, 0.0)),
        (0.105, 2.6, (0.03, -0.02)),
        (0.165, 1.2, (-0.025, 0.025)),
    ]
    for idx, (z, rot, off) in enumerate(placements):
        made = make_matzah(idx, z, rot, matzah_mat, dimple_mat)
        for ob in made:
            ob.location.x += off[0]
            ob.location.y += off[1]
        objs += made

    common.frame_object(objs, margin=0.14, elevation=32.0, azimuth=30.0,
                        focal=72.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render("matzah", common.OUT_DIR)


build()
