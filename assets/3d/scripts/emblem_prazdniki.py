"""Emblem: shofar (prazdniki = holidays)."""

import math
import os
import sys

import bpy
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import emblemkit

SIDES = 28
# Bell at the upper right, mouthpiece curling down to the lower left.
ARC_R = 0.50
ARC_C = (0.0, 0.0)
A_BELL = math.radians(24.0)
A_TIP = math.radians(220.0)
BELL_R = 0.24
TIP_R = 0.055


def sweep(name, path, radii, mat, sides=SIDES):
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
            # Flatten slightly toward the camera so the relief reads as a horn
            # carved on the face rather than a fat torus.
            radial = math.cos(a) * b + math.sin(a) * nn
            radial = Vector((radial.x, radial.y * 0.62, radial.z))
            verts.append(tuple(p + radii[i] * radial))
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
    common.bevel(ob, 0.006, segments=1)
    return ob


def build():
    common.reset_scene()
    common.setup_render()
    objs, metal, accent = emblemkit.medallion()
    ry = emblemkit.R_Y
    syms = []

    n = 32
    path, radii = [], []
    for i in range(n):
        t = i / (n - 1)
        # Ease the radius: wide bell, long slim body, small mouthpiece.
        a = A_BELL + (A_TIP - A_BELL) * t
        path.append(Vector((ARC_C[0] + ARC_R * math.cos(a), ry,
                            ARC_C[1] + ARC_R * math.sin(a))))
        u = t ** 0.72
        r = BELL_R + (TIP_R - BELL_R) * u
        if t < 0.08:
            r += 0.035 * (1.0 - t / 0.08)
        radii.append(r)
    horn = sweep("horn", path, radii, metal)
    syms.append(horn)

    for t in (0.18, 0.36, 0.54, 0.72):
        i = int(t * (n - 1))
        p = path[i]
        tangent = (path[min(i + 1, n - 1)] - path[max(i - 1, 0)]).normalized()
        ring = common.add_torus(
            "ridge", radii[i] * 0.92 + 0.012, 0.013,
            location=p, mat=metal, major_segments=28, minor_segments=8)
        ring.rotation_euler = tangent.to_track_quat("Z", "Y").to_euler()
        ring.scale = (1.0, 0.62, 1.0)
        syms.append(ring)

    bell_t = (path[1] - path[0]).normalized()
    hollow = common.add_uv_sphere(
        "bell_hollow", BELL_R * 0.62,
        location=path[0] + bell_t * 0.01,
        scale=(1.0, 0.40, 1.0), mat=accent, segments=28, rings=14)
    hollow.rotation_euler = bell_t.to_track_quat("Y", "Z").to_euler()
    syms.append(hollow)

    tip_dir = (path[-1] - path[-2]).normalized()
    syms.append(common.add_uv_sphere(
        "mouthpiece", TIP_R * 0.85,
        location=path[-1] + tip_dir * 0.012,
        scale=(1.0, 0.55, 1.0), mat=accent, segments=20, rings=10))

    emblemkit.fit_xz(syms, 0.60)
    objs.extend(syms)
    emblemkit.finish(objs, "prazdniki")


build()
