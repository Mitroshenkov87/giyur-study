"""Kotel: Western Wall vignette — Herodian ashlar courses, paper notes in the
joints, a plant tuft growing from a crack."""

import math
import os
import sys

import bpy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common

COURSE_H = 0.42
DEPTH = 0.55
# courses bottom-to-top, each a list of (x0, x1) stone spans
COURSES = [
    [(-1.20, 1.20)],
    [(-1.20, 0.05), (0.05, 1.20)],
    [(-1.20, 1.20)],
    [(-1.20, -0.25), (-0.25, 1.20)],
    [(-1.20, 1.20)],
]
# small depth offsets per stone so the faces are not one flat plane
Y_OFFS = [0.01, 0.03, 0.00, 0.04, 0.02, 0.01, 0.03, 0.00]

NOTES = [
    # (x, joint level, tilt_ry, tilt_rz)
    (0.45, 1, 0.16, 0.08),
    (-0.88, 2, -0.18, -0.05),
    (0.12, 3, 0.10, 0.38),
    (-0.42, 1, -0.12, 0.12),
]


def add_box(name, dims, loc, mat, bevel_w=0.03, rot=(0.0, 0.0, 0.0)):
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=loc, rotation=rot)
    ob = bpy.context.object
    ob.name = name
    ob.scale = dims
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    if mat:
        common.assign(ob, mat)
    if bevel_w:
        common.bevel(ob, bevel_w, segments=2)
    return ob


def paper_mat():
    m, nt, bsdf = common._new_material("Paper")
    common._inp(bsdf, "Base Color", (0.93, 0.92, 0.88, 1.0))
    common._inp(bsdf, "Roughness", 0.85)
    return m


def leaf_mat():
    m, nt, bsdf = common._new_material("LeafGreen")
    common._inp(bsdf, "Base Color", (0.14, 0.32, 0.07, 1.0))
    common._inp(bsdf, "Roughness", 0.55)
    common._inp(bsdf, "Sheen Weight", 0.25)
    return m


def build_wall(stone):
    objs = []
    yi = 0
    for level, spans in enumerate(COURSES):
        z0 = level * COURSE_H
        for (x0, x1) in spans:
            w = x1 - x0
            yoff = Y_OFFS[yi % len(Y_OFFS)]
            yi += 1
            ob = add_box(
                "stone",
                (w - 0.015, DEPTH, COURSE_H - 0.015),
                ((x0 + x1) * 0.5, yoff, z0 + COURSE_H * 0.5),
                stone, bevel_w=0.045)
            objs.append(ob)
    return objs


def build_notes(paper):
    objs = []
    for (x, level, ry, rz) in NOTES:
        z = level * COURSE_H
        ob = add_box(
            "note", (0.10, 0.012, 0.14),
            (x, -DEPTH * 0.5 - 0.004, z),
            paper, bevel_w=0.004, rot=(0.0, ry, rz))
        objs.append(ob)
    return objs


def build_plant(green):
    """Tuft sprouting from the vertical joint at x=0.05 (course 2 joint)."""
    objs = []
    base = (0.05, -DEPTH * 0.5 - 0.01, 1.26)
    blades = 7
    for i in range(blades):
        azim = -0.5 + i * (math.pi / (blades - 1))  # fan across the front
        tilt = 0.30 + 0.35 * ((i * 37) % 10) / 10.0
        depth = 0.30 + 0.18 * ((i * 53) % 10) / 10.0
        r1 = 0.020
        ob = common.add_cone(
            "blade", r1, 0.002, depth,
            location=(base[0] + math.sin(azim) * math.sin(tilt) * depth * 0.5,
                      base[1] - math.cos(azim) * math.sin(tilt) * depth * 0.5 - 0.02,
                      base[2] + math.cos(tilt) * depth * 0.5),
            vertices=12, mat=green)
        ob.rotation_euler = (math.cos(azim) * tilt, -math.sin(azim) * tilt, 0.0)
        # re-position after tilt so the blade root sits at the base point
        objs.append(ob)
    # a couple of dry blades
    for k in range(2):
        m2, nt2, b2 = common._new_material("Dry%d" % k)
        common._inp(b2, "Base Color", (0.42, 0.30, 0.12, 1.0))
        common._inp(b2, "Roughness", 0.8)
        ob = common.add_cone(
            "dry_blade", 0.014, 0.002, 0.26,
            location=(base[0] - 0.06 + k * 0.12,
                      base[1] - 0.03,
                      base[2] + 0.10),
            vertices=10, mat=m2)
        ob.rotation_euler = (0.55, -0.3 + k * 0.5, 0.0)
        objs.append(ob)
    return objs


def build():
    common.reset_scene()
    common.setup_render()
    stone = common.mat_stone()

    objs = build_wall(stone)
    objs += build_notes(paper_mat())
    objs += build_plant(leaf_mat())

    common.frame_object(objs, margin=0.1, focal=70.0,
                        elevation=22.0, azimuth=32.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render("kotel", common.OUT_DIR)


build()
