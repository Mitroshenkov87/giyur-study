"""Decor: L-shaped corner bracket with a scroll curl (both themes)."""

import math
import os
import sys

import bpy
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common

L = 0.85   # arm length
W = 0.26   # arm width


def build():
    common.reset_scene()
    common.setup_render()
    metal = common.theme_metal()
    accent = common.theme_accent()
    objs = []

    # L outline (XZ plane, CCW as seen from the camera), rounded by bevel
    outline = [(0, 0), (L, 0), (L, W), (W, W), (W, L), (0, L)]
    n = len(outline)
    y_base, depth = 0.02, 0.12
    verts = [(x, y_base, z) for x, z in outline] + [(x, y_base - depth, z)
                                                    for x, z in outline]
    faces = [tuple(range(n - 1, -1, -1)), tuple(range(n, 2 * n))]
    for i in range(n):
        j = (i + 1) % n
        faces.append((i, j, n + j, n + i))
    mesh = bpy.data.meshes.new("bracket_mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    bracket = bpy.data.objects.new("bracket", mesh)
    bpy.context.collection.objects.link(bracket)
    common.assign(bracket, metal)
    common.bevel(bracket, 0.05, segments=4)
    objs.append(bracket)

    # engraved scroll curl: flat spiral on the corner boss
    spiral = []
    turns, steps = 2.25, 40
    for i in range(steps):
        t = i / (steps - 1)
        a = 2.0 * math.pi * turns * t
        r = 0.025 + 0.085 * t
        spiral.append(Vector((0.125 + r * math.cos(a), -0.135,
                              0.125 + r * math.sin(a))))
    objs.append(common.tube_curve("scroll_curl", spiral, 0.021, mat=metal))
    objs.append(common.add_uv_sphere(
        "scroll_center", 0.026, location=(0.125, -0.135, 0.125), mat=metal,
        segments=16, rings=8))

    # small screw holes near the arm ends
    for x, z in ((L - 0.14, W * 0.5), (W * 0.5, L - 0.14)):
        hole = common.add_cylinder("screw_hole", 0.028, 0.05,
                                   location=(x, -0.105, z), vertices=24,
                                   mat=accent)
        hole.rotation_euler = (math.radians(90), 0, 0)
        objs.append(hole)

    common.frame_object(objs, margin=0.10, focal=72.0,
                        elevation=20.0, azimuth=28.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render(common.variant_name("corner"), common.OUT_DIR)


build()
