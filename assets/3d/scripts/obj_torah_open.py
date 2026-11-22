"""Open Sefer Torah: two turned wooden rollers (atzei chaim) with round
discs, the parchment scroll spread between them in a gentle sag, four
columns of text rows on the parchment, lying at a slight angle."""

import math
import os
import random
import sys

import bpy
from mathutils import Matrix

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import objkit

HALF_W = 1.30   # parchment half width (X)
HALF_D = 0.42   # parchment half depth (Y)
ROLLER_X = 1.30
ROLLER_Z = 0.075

# Turned roller profile (radius, length): tip, handle knobs, disc, rod,
# disc, knobs, tip. Length 2.36 -> spans y in [-1.18, 1.18] once centred.
ROLLER_PROFILE = [
    (0.000, 0.00),
    (0.055, 0.02),
    (0.075, 0.07),
    (0.115, 0.13), (0.115, 0.26),
    (0.080, 0.30),
    (0.125, 0.36), (0.125, 0.50),
    (0.082, 0.54),
    (0.150, 0.58), (0.158, 0.645), (0.150, 0.71),
    (0.085, 0.75),
    (0.085, 1.61),
    (0.150, 1.65), (0.158, 1.715), (0.150, 1.78),
    (0.082, 1.82),
    (0.125, 1.86), (0.125, 2.00),
    (0.080, 2.04),
    (0.115, 2.10), (0.115, 2.23),
    (0.075, 2.28),
    (0.050, 2.315),
    (0.000, 2.36),
]


def surface_z(x, y):
    """Sagging sheet with a gentle upturn that tucks onto the rollers."""
    t = min(max((abs(x) - 0.95) / 0.35, 0.0), 1.0)
    return (-0.045 * (1.0 - (x / HALF_W) ** 2)
            + 0.13 * t * t
            - 0.018 * (y / HALF_D) ** 2)


def slope_x(x, y):
    e = 0.005
    return (surface_z(x + e, y) - surface_z(x - e, y)) / (2.0 * e)


def sheet(mat):
    nu, nv, thick = 72, 20, 0.012
    verts, faces = [], []
    for i in range(nu + 1):
        x = -HALF_W + 2.0 * HALF_W * i / nu
        for j in range(nv + 1):
            y = -HALF_D + 2.0 * HALF_D * j / nv
            verts.append((x, y, surface_z(x, y)))
    off = len(verts)
    for i in range(nu + 1):
        x = -HALF_W + 2.0 * HALF_W * i / nu
        for j in range(nv + 1):
            y = -HALF_D + 2.0 * HALF_D * j / nv
            verts.append((x, y, surface_z(x, y) - thick))
    for i in range(nu):
        for j in range(nv):
            a = i * (nv + 1) + j
            b = (i + 1) * (nv + 1) + j
            faces.append((a, b, b + 1, a + 1))
            faces.append((off + a + 1, off + b + 1, off + b, off + a))
    for i in range(nu):  # rim along +/- y
        a = i * (nv + 1)
        b = (i + 1) * (nv + 1)
        faces.append((a, off + a, off + b, b))
        a = i * (nv + 1) + nv
        b = (i + 1) * (nv + 1) + nv
        faces.append((a, b, off + b, off + a))
    for j in range(nv):  # rim along +/- x
        a = j
        b = j + 1
        faces.append((a, b, off + b, off + a))
        a = nu * (nv + 1) + j
        b = nu * (nv + 1) + j + 1
        faces.append((a, off + a, off + b, b))
    mesh = bpy.data.meshes.new("parchment_mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    ob = bpy.data.objects.new("parchment", mesh)
    bpy.context.collection.objects.link(ob)
    common.assign(ob, mat)
    common.smooth(ob)
    return ob


def text_rows(mat):
    """Four text columns as procedural rows of small ink dashes."""
    random.seed(11)
    boxes = []
    for side in (-1, 1):
        for col in (0.33, 0.83):
            x0 = side * col - 0.165
            for row in range(13):
                y = -0.30 + row * 0.05
                pos = 0.0
                while 0.33 - pos > 0.048:
                    w = random.uniform(0.045, min(0.105, 0.33 - pos))
                    xc = x0 + pos + w / 2.0
                    slope = slope_x(xc, y)
                    z = surface_z(xc, y) + 0.006
                    boxes.append((xc, y, z, w - 0.022, 0.020, 0.008,
                                  -math.atan(slope)))
                    pos += w + random.uniform(0.024, 0.04)
    return objkit.add_boxes("text_rows", boxes, mat)


def build():
    common.reset_scene()
    common.setup_render()
    walnut = common.mat_walnut()
    parchment = common.mat_parchment()
    ink = objkit.mat_ink()
    brass = common.mat_brass()
    objs = []

    objs.append(sheet(parchment))
    objs.append(text_rows(ink))

    for sign in (-1, 1):
        roller = common.lathe("roller", ROLLER_PROFILE, segments=64, mat=walnut)
        roller.rotation_euler = (math.radians(90.0), 0.0, 0.0)  # +Z -> -Y
        roller.location = (sign * ROLLER_X, 1.18, ROLLER_Z)
        objs.append(roller)
        for tip_sign in (-1, 1):
            objs.append(common.add_uv_sphere(
                "handle_tip", 0.032,
                location=(sign * ROLLER_X, tip_sign * 1.165, ROLLER_Z),
                mat=brass, segments=24, rings=12))

    # lie at a slight angle, as on a bimah
    turn = Matrix.Rotation(math.radians(12.0), 4, "Z")
    for ob in objs:
        ob.matrix_world = turn @ ob.matrix_world

    common.frame_object(objs, margin=0.1, elevation=26.0, azimuth=40.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render("torah_open", common.OUT_DIR)


build()
