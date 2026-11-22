"""Pomegranate: whole fruit with its crown, plus a half opened to show
translucent red arils."""

import math
import os
import sys

import bpy
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import foodkit

FRUIT_R = 0.52


def skin_mat():
    return foodkit.prin(
        "PomSkin", (0.55, 0.04, 0.05, 1.0), rough=0.35, coat=0.25,
        coat_rough=0.3, subsurface=0.05,
        ramp=[(0.0, (0.40, 0.02, 0.03, 1.0)),
              (0.55, (0.66, 0.06, 0.07, 1.0)),
              (1.0, (0.34, 0.02, 0.04, 1.0))],
        ramp_scale=3.0, bump=0.22, bump_scale=30.0)


def calyx_mat():
    return foodkit.prin("Calyx", (0.36, 0.13, 0.08, 1.0), rough=0.8)


def aril_mat():
    return foodkit.prin("Aril", (0.60, 0.01, 0.03, 1.0), rough=0.12,
                        coat=0.5, coat_rough=0.1, subsurface=0.10,
                        transmission=0.3, ior=1.4)


def pith_mat():
    return foodkit.prin("Pith", (0.92, 0.88, 0.78, 1.0), rough=0.7)


def crown(calyx, z_top, cx, cy, phase=0.0):
    objs = []
    for i in range(6):
        a = phase + i * math.pi / 3.0
        tilt = math.radians(38 + 7 * math.sin(i * 2.3))
        depth = 0.20 + 0.03 * math.sin(i * 1.7)
        cx0, cy0 = 0.10 * math.cos(a), 0.10 * math.sin(a)
        sep = common.add_cone("calyx_sepal", 0.050, 0.006, depth,
                              location=(cx + cx0, cy + cy0,
                                        z_top + 0.05 + 0.03 * math.cos(tilt)),
                              vertices=12, mat=calyx)
        # rotate the vertical cone outward around the tangential axis
        axis = Vector((-math.sin(a), math.cos(a), 0.0))
        sep.rotation_mode = "QUATERNION"
        sep.rotation_quaternion = QuaternionFromAxisAngle(axis, tilt)
        objs.append(sep)
    stem = common.add_uv_sphere("calyx_stem", 0.045,
                                location=(cx, cy, z_top + 0.045),
                                scale=(1.0, 1.0, 0.7), mat=calyx,
                                segments=20, rings=12)
    objs.append(stem)
    return objs


def QuaternionFromAxisAngle(axis, angle):
    from mathutils import Quaternion
    return Quaternion(axis, angle)


HALF_PROFILE = [
    (0.0, -0.46), (0.18, -0.425), (0.33, -0.33), (0.44, -0.185),
    (0.50, 0.0), (0.455, 0.0), (0.40, -0.16), (0.29, -0.27),
    (0.13, -0.34), (0.0, -0.355),
]

PITH_PROFILE = [
    (0.0, -0.06), (0.35, -0.05), (0.42, -0.015), (0.44, 0.0), (0.0, 0.0),
]


def dome_z(r):
    return 0.10 * math.sqrt(max(0.0, 1.0 - (r / 0.42) ** 2)) - 0.02


def aril_dome(aril, loc, rot):
    objs = []
    rings = [(0.0, 1), (0.095, 5), (0.19, 10), (0.285, 14), (0.375, 17)]
    for r, count in rings:
        for k in range(count):
            a = (k + 0.5 * (int(r * 100) % 2)) * 2.0 * math.pi / count
            x, y = r * math.cos(a), r * math.sin(a)
            z = dome_z(r) + 0.02
            s = common.add_uv_sphere("aril", 0.050,
                                     location=(x, y, z), mat=aril,
                                     segments=16, rings=10)
            objs.append(s)
    for ob in objs:
        ob.rotation_euler.z = rot
        ob.location.x += loc[0]
        ob.location.y += loc[1]
        ob.location.z += loc[2]
    return objs


def build():
    common.reset_scene()
    common.setup_render()
    skin = skin_mat()
    calyx = calyx_mat()
    aril = aril_mat()
    pith = pith_mat()
    objs = []

    # whole fruit, upright
    whole = common.add_uv_sphere("pomegranate_whole", FRUIT_R,
                                 location=(-0.55, 0.10, 0.475),
                                 scale=(1.0, 1.0, 0.90), mat=skin)
    objs.append(whole)
    wz = 0.475 + FRUIT_R * 0.90
    objs += crown(calyx, wz - 0.06, -0.55, 0.10)

    # half, cut face tilted toward the camera
    half_loc = Vector((0.48, -0.12, 0.40))
    half_rot = (math.radians(-52), 0.0, math.radians(18))
    shell = common.lathe("pomegranate_half", HALF_PROFILE, segments=64,
                         mat=skin)
    pith_ob = common.lathe("pomegranate_pith", PITH_PROFILE, segments=64,
                           mat=pith)
    grp = [shell, pith_ob] + aril_dome(aril, (0, 0, 0), 0.0)
    for ob in grp:
        ob.rotation_euler = half_rot
        ob.location = half_loc
    objs += grp

    # a few loose arils on the ground
    for i, (dx, dy) in enumerate(((0.10, 0.28), (0.30, 0.18), (0.02, 0.42))):
        objs.append(common.add_uv_sphere("aril_loose", 0.050,
                                         location=(0.35 + dx, -0.30 + dy,
                                                   0.052),
                                         mat=aril, segments=16, rings=10))

    common.frame_object(objs, margin=0.15, elevation=26.0, azimuth=30.0,
                        focal=70.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render("pomegranate", common.OUT_DIR)


build()
