"""Decor: round red wax seal stamped with a Star of David (both themes)."""

import math
import os
import sys

import bpy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common

R = 0.55
SEG = 40


def jitter(i, salt):
    v = math.sin(i * 12.9898 + salt * 78.233) * 43758.5453
    return 2.0 * (v - math.floor(v)) - 1.0


def wax_blob(mat):
    """Irregular-rimmed wax disc facing the camera (XZ plane), slightly domed."""
    verts = [(0.0, 0.02, 0.0), (0.0, -0.195, 0.0)]  # back apex, front apex
    rings = [
        (0.0, 1.00, 0.05, 11.0),    # back
        (-0.12, 1.00, 0.04, 23.0),  # mid
        (-0.17, 0.88, 0.03, 37.0),  # front
    ]
    for y, rf, amp, salt in rings:
        for i in range(SEG):
            a = 2.0 * math.pi * i / SEG
            r = R * rf * (1.0 + amp * jitter(i, salt))
            verts.append((r * math.cos(a), y + 0.012 * jitter(i, salt + 5.0),
                          r * math.sin(a)))
    r0, r1, r2 = 2, 2 + SEG, 2 + 2 * SEG
    faces = []
    for i in range(SEG):
        i2 = (i + 1) % SEG
        faces.append((0, r0 + i, r0 + i2))                 # back fan
        faces.append((r0 + i, r1 + i, r1 + i2, r0 + i2))   # wall back
        faces.append((r1 + i, r2 + i, r2 + i2, r1 + i2))   # wall front
        faces.append((1, r2 + i2, r2 + i))                 # front dome
    mesh = bpy.data.meshes.new("wax_mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    ob = bpy.data.objects.new("wax", mesh)
    bpy.context.collection.objects.link(ob)
    common.assign(ob, mat)
    common.smooth(ob)
    common.bevel(ob, 0.012, segments=2)
    return ob


def triangle_ring(name, r_out, r_in, y_front, depth, rot, mat):
    """Extruded equilateral triangle frame (hollow), facing the camera."""
    verts = []
    for y in (y_front + depth, y_front):  # back then front
        for r in (r_out, r_in):
            for k in range(3):
                a = math.radians(90.0 + 120.0 * k) + rot
                verts.append((r * math.cos(a), y, r * math.sin(a)))
    faces = []
    for k in range(3):
        k2 = (k + 1) % 3
        faces.append((6 + k, 9 + k, 9 + k2, 6 + k2))   # front (normal -Y)
        faces.append((k, k2, 3 + k2, 3 + k))           # back
        faces.append((k, 6 + k, 6 + k2, k2))           # outer wall
        faces.append((3 + k2, 9 + k2, 9 + k, 3 + k))   # inner wall
    mesh = bpy.data.meshes.new(name + "_mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    ob = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(ob)
    common.assign(ob, mat)
    common.bevel(ob, 0.008, segments=2)
    return ob


def build():
    common.reset_scene()
    common.setup_render()
    wax_mat = common.mat_wax(color=(0.34, 0.02, 0.035, 1.0))
    stamp_mat = common.mat_wax(color=(0.20, 0.008, 0.014, 1.0))

    objs = [wax_blob(wax_mat)]
    # Star of David stamped slightly below the wax face (front is y ~ -0.17)
    objs.append(triangle_ring("tri_up", 0.32, 0.255, -0.135, 0.05, 0.0,
                              stamp_mat))
    objs.append(triangle_ring("tri_down", 0.32, 0.255, -0.135, 0.05,
                              math.radians(60.0), stamp_mat))

    common.frame_object(objs, margin=0.14, focal=72.0,
                        elevation=16.0, azimuth=25.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render(common.variant_name("seal"), common.OUT_DIR)


build()
