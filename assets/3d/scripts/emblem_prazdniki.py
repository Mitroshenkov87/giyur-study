"""Emblem: shofar (prazdniki = holidays)."""

import math
import os
import sys

import bpy
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import emblemkit

SIDES = 24
ARC_R = 0.38
ARC_C = (0.0, -0.05)
A_START = math.radians(175.0)
A_END = math.radians(-15.0)
BELL_R = 0.20
TIP_R = 0.062


def sweep(name, path, radii, mat, sides=SIDES):
    """Sweep a circle of varying radius along a 3D path (ring frame: Z-up)."""
    n = len(path)
    verts = []
    for i in range(n):
        p = path[i]
        t = (path[min(i + 1, n - 1)] - path[max(i - 1, 0)]).normalized()
        b = t.cross(Vector((0.0, 0.0, 1.0)))
        if b.length < 1e-6:
            b = Vector((1.0, 0.0, 0.0))
        b.normalize()
        nn = b.cross(t).normalized()
        for k in range(sides):
            a = 2.0 * math.pi * k / sides
            verts.append(tuple(p + radii[i] * (math.cos(a) * b + math.sin(a) * nn)))
    faces = []
    for i in range(n - 1):
        for k in range(sides):
            k2 = (k + 1) % sides
            faces.append((i * sides + k, i * sides + k2,
                          (i + 1) * sides + k2, (i + 1) * sides + k))
    verts.append(tuple(path[0]))
    c0 = len(verts) - 1
    verts.append(tuple(path[-1]))
    c1 = len(verts) - 1
    for k in range(sides):
        k2 = (k + 1) % sides
        faces.append((c0, k2, k))
        faces.append((c1, (n - 1) * sides + k, (n - 1) * sides + k2))
    mesh = bpy.data.meshes.new(name + "_mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    ob = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(ob)
    common.assign(ob, mat)
    common.smooth(ob)
    common.bevel(ob, 0.008, segments=2)
    return ob


def build():
    common.reset_scene()
    common.setup_render()
    objs, metal, accent = emblemkit.medallion()
    ry = emblemkit.R_Y

    n = 28
    path, radii = [], []
    for i in range(n):
        t = i / (n - 1)
        a = A_START + (A_END - A_START) * t
        path.append(Vector((ARC_C[0] + ARC_R * math.cos(a), ry,
                            ARC_C[1] + ARC_R * math.sin(a))))
        r = BELL_R + (TIP_R - BELL_R) * t
        if t < 0.06:  # gentle flare at the bell lip
            r += 0.012 * (1 - t / 0.06)
        radii.append(r)
    objs.append(sweep("horn", path, radii, metal))

    # growth ridges around the horn
    for t in (0.22, 0.42, 0.62):
        i = int(t * (n - 1))
        p = path[i]
        tangent = (path[min(i + 1, n - 1)] - path[max(i - 1, 0)]).normalized()
        ring = common.add_torus("ridge", radii[i] + 0.006, 0.016,
                                location=p, mat=metal, major_segments=32,
                                minor_segments=10)
        ring.rotation_euler = tangent.to_track_quat("Z", "Y").to_euler()
        objs.append(ring)

    # dark hollow at the bell and a cut tip at the mouthpiece
    bell_t = tangent = (path[1] - path[0]).normalized()
    objs.append(common.add_uv_sphere(
        "bell_hollow", BELL_R * 0.82, location=path[0] - bell_t * 0.02,
        scale=(1.0, 0.55, 1.0), mat=accent, segments=32, rings=16))
    tip_dir = (path[-1] - path[-2]).normalized()
    objs.append(common.add_uv_sphere(
        "mouthpiece", TIP_R * 0.8, location=path[-1] + tip_dir * 0.01,
        scale=(1.0, 0.6, 1.0), mat=accent, segments=24, rings=12))

    emblemkit.finish(objs, "prazdniki")


build()
