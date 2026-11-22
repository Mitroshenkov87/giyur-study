"""Seder plate: round ceramic plate with six small bowls holding the
symbolic Pesach foods (zeroa, beitzah, maror, charoset, karpas, chazeret)."""

import math
import os
import sys

import bpy
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import foodkit

PLATE_PROFILE = [
    (0.0, 0.16), (0.85, 0.17), (1.20, 0.20), (1.42, 0.27), (1.54, 0.345),
    (1.63, 0.355), (1.66, 0.34), (1.68, 0.20), (1.64, 0.08), (1.10, 0.055),
    (0.66, 0.05), (0.62, 0.02), (0.56, 0.0), (0.0, 0.0),
]

BOWL_PROFILE = [
    (0.0, 0.02), (0.14, 0.02), (0.22, 0.05), (0.28, 0.12), (0.315, 0.20),
    (0.325, 0.235), (0.305, 0.235), (0.28, 0.17), (0.22, 0.075),
    (0.14, 0.03), (0.0, 0.025),
]

RING_R = 1.02
BOWL_Z = 0.205
ITEM_Z = 0.255


def make_materials():
    ceramic = foodkit.prin("Ceramic", (0.93, 0.92, 0.88, 1.0), rough=0.16,
                           coat=0.55, coat_rough=0.08)
    roast = foodkit.prin("Roast", (0.30, 0.12, 0.05, 1.0), rough=0.5,
                         ramp=[(0.0, (0.16, 0.06, 0.025, 1.0)),
                               (0.55, (0.38, 0.16, 0.06, 1.0)),
                               (1.0, (0.24, 0.09, 0.03, 1.0))],
                         ramp_scale=5.0, bump=0.2, bump_scale=30.0)
    egg_mat = foodkit.prin("RoastEgg", (0.72, 0.58, 0.38, 1.0), rough=0.55,
                           ramp=[(0.0, (0.55, 0.40, 0.22, 1.0)),
                                 (0.6, (0.82, 0.68, 0.46, 1.0)),
                                 (1.0, (0.62, 0.46, 0.26, 1.0))],
                           ramp_scale=3.5)
    maror_mat = foodkit.prin("Maror", (0.78, 0.72, 0.58, 1.0), rough=0.8,
                             ramp=[(0.0, (0.60, 0.52, 0.38, 1.0)),
                                   (1.0, (0.85, 0.80, 0.68, 1.0))],
                             ramp_scale=4.0, bump=0.25, bump_scale=25.0)
    charoset_mat = foodkit.prin("Charoset", (0.30, 0.14, 0.05, 1.0), rough=0.7,
                                ramp=[(0.0, (0.18, 0.07, 0.02, 1.0)),
                                      (1.0, (0.42, 0.22, 0.08, 1.0))],
                                ramp_scale=6.0, bump=0.5, bump_scale=40.0)
    apple_mat = foodkit.prin("CharosetApple", (0.55, 0.25, 0.10, 1.0),
                             rough=0.5, coat=0.2)
    celery_mat = foodkit.prin("Celery", (0.62, 0.78, 0.45, 1.0), rough=0.6)
    parsley_mat = foodkit.prin("Parsley", (0.10, 0.34, 0.12, 1.0), rough=0.65)
    lettuce_mat = foodkit.prin("Lettuce", (0.36, 0.62, 0.25, 1.0), rough=0.6)
    return dict(ceramic=ceramic, roast=roast, egg=egg_mat, maror=maror_mat,
                charoset=charoset_mat, apple=apple_mat, celery=celery_mat,
                parsley=parsley_mat, lettuce=lettuce_mat)


def zeroa(roast, loc, rot):
    """Roasted shankbone: shaft with two knobby ends."""
    pts = [Vector((0, 0, -0.14)), Vector((0.03, 0.01, 0.0)),
           Vector((-0.02, 0, 0.14))]
    radii = [0.05, 0.038, 0.048]
    bone = foodkit.tapered_tube("zeroa_shaft", pts, radii, mat=roast)
    k1 = common.add_uv_sphere("zeroa_knob", 0.082, location=(0, 0, -0.15),
                              scale=(1.0, 0.85, 0.9), mat=roast)
    k2 = common.add_uv_sphere("zeroa_knob", 0.075, location=(0, 0, 0.16),
                              scale=(1.0, 0.9, 0.95), mat=roast)
    grp = [bone, k1, k2]
    for ob in grp:
        ob.rotation_euler.z = rot
        ob.location.x += loc[0]
        ob.location.y += loc[1]
        ob.location.z += loc[2]
    return grp


def beitzah(egg_mat, loc, rot):
    profile = [(0.0, 0.0), (0.10, 0.005), (0.145, 0.06), (0.16, 0.13),
               (0.145, 0.20), (0.10, 0.255), (0.0, 0.28)]
    egg = common.lathe("beitzah", profile, segments=48, mat=egg_mat)
    egg.rotation_euler.z = rot
    egg.location = loc
    return [egg]


def maror(maror_mat, loc, rot):
    profile = [(0.0, 0.0), (0.10, 0.01), (0.13, 0.08), (0.09, 0.20),
               (0.055, 0.30), (0.02, 0.34), (0.0, 0.35)]
    root = common.lathe("maror", profile, segments=40, mat=maror_mat)
    r1 = common.add_cone("maror_rootlet", 0.014, 0.004, 0.09,
                         location=(0.09, 0.02, 0.02), vertices=10,
                         mat=maror_mat)
    r2 = common.add_cone("maror_rootlet", 0.012, 0.004, 0.08,
                         location=(-0.08, -0.03, 0.02), vertices=10,
                         mat=maror_mat)
    grp = [root, r1, r2]
    for ob in grp:
        ob.rotation_euler.z = rot
        ob.location.x += loc[0]
        ob.location.y += loc[1]
        ob.location.z += loc[2]
    return grp


def charoset(charoset_mat, apple_mat, loc, rot):
    mound = common.add_uv_sphere("charoset_mound", 0.16,
                                 location=(0, 0, 0.07),
                                 scale=(1.0, 1.0, 0.6), mat=charoset_mat)
    cubes = []
    for i, (dx, dy) in enumerate(((0.06, 0.04), (-0.07, 0.02), (0.0, -0.07))):
        c = foodkit.rounded_box("charoset_cube", (0.055, 0.055, 0.05),
                                (dx, dy, 0.13 + 0.01 * i), apple_mat,
                                bevel_w=0.014, segments=2)
        c.rotation_euler.z = i * 0.9
        cubes.append(c)
    grp = [mound] + cubes
    for ob in grp:
        ob.rotation_euler.z += rot
        ob.location.x += loc[0]
        ob.location.y += loc[1]
        ob.location.z += loc[2]
    return grp


def karpas(celery_mat, parsley_mat, loc, rot):
    stick = foodkit.rounded_box("karpas_stick", (0.09, 0.09, 0.36),
                                (0, 0, 0.18), celery_mat, bevel_w=0.03)
    grp = [stick]
    for i in range(3):
        a = -0.5 + i * 0.5
        leaf = common.add_uv_sphere("parsley_leaf", 0.075,
                                    location=(0.10 * math.sin(a),
                                              0.10 * math.cos(a), 0.40),
                                    scale=(1.5, 0.5, 1.0), mat=parsley_mat,
                                    segments=20, rings=12)
        leaf.rotation_euler.z = a
        grp.append(leaf)
    for ob in grp:
        ob.rotation_euler.z += rot
        ob.location.x += loc[0]
        ob.location.y += loc[1]
        ob.location.z += loc[2]
    return grp


def chazeret(lettuce_mat, loc, rot):
    grp = []
    for i, ang in enumerate((-0.45, 0.35)):
        leaf = foodkit.ribbon("romaine_leaf", 0.46, 0.17, curl=0.55,
                              fold=0.35, mat=lettuce_mat, stations=9)
        leaf.rotation_euler = (0.0, math.radians(62), ang + rot)
        leaf.location = (loc[0], loc[1], loc[2] + 0.03 + i * 0.02)
        grp.append(leaf)
    return grp


def build():
    common.reset_scene()
    common.setup_render()
    M = make_materials()
    gold = common.mat_gold()
    objs = []

    plate = common.lathe("plate", PLATE_PROFILE, segments=96, mat=M["ceramic"])
    bevel = common.add_torus("plate_rim_gold", 1.60, 0.020,
                             location=(0, 0, 0.345), mat=gold)
    inner = common.add_torus("plate_ring_gold", 1.32, 0.012,
                             location=(0, 0, 0.235), mat=gold)
    objs += [plate, bevel, inner]

    builders = (zeroa, beitzah, maror, charoset, karpas, chazeret)
    for k, fn in enumerate(builders):
        ang = math.radians(-90 + k * 60)
        x, y = RING_R * math.sin(ang), RING_R * math.cos(ang)
        bowl = common.lathe("bowl", BOWL_PROFILE, segments=48,
                            mat=M["ceramic"])
        bowl.location = (x, y, BOWL_Z)
        objs.append(bowl)
        if fn is zeroa:
            grp = fn(M["roast"], (x, y, ITEM_Z + 0.15), ang + 0.4)
        elif fn is beitzah:
            grp = fn(M["egg"], (x, y, ITEM_Z), ang)
        elif fn is maror:
            grp = fn(M["maror"], (x, y, ITEM_Z), ang)
        elif fn is charoset:
            grp = fn(M["charoset"], M["apple"], (x, y, ITEM_Z), ang)
        elif fn is karpas:
            grp = fn(M["celery"], M["parsley"], (x, y, ITEM_Z), ang)
        else:
            grp = fn(M["lettuce"], (x, y, ITEM_Z), ang)
        objs += grp

    common.frame_object(objs, margin=0.15, elevation=31.0, azimuth=22.0,
                        focal=68.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render("seder_plate", common.OUT_DIR)


build()
