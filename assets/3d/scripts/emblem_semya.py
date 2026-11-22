"""Emblem: chuppah wedding canopy on four poles (semya = family)."""

import math
import os
import sys

import bpy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import emblemkit

# Drawn in the XZ plane so a nearly frontal camera still shows four poles.
# Front pair is wider and lower; back pair is narrower and taller.
FRONT_X, BACK_X = 0.50, 0.30
FRONT_Z, BACK_Z = 0.22, 0.55
POLE_Z0 = -0.48
POLE_R = 0.050


def canopy_cloth(name, mat, ry):
    nu, nv = 14, 7
    verts = []
    for j in range(nv):
        t = j / (nv - 1)
        xh = FRONT_X + (BACK_X - FRONT_X) * t
        z0 = FRONT_Z + (BACK_Z - FRONT_Z) * t
        y = ry - 0.02 + 0.045 * t
        for i in range(nu):
            u = -1.0 + 2.0 * i / (nu - 1)
            sag = (0.14 * (1.0 - 0.35 * t) + 0.04 * math.sin(math.pi * t)) * (1.0 - u * u)
            verts.append((xh * u, y - sag * 0.15, z0 - sag))
    faces = []
    for j in range(nv - 1):
        for i in range(nu - 1):
            a = j * nu + i
            faces.append((a, a + 1, a + 1 + nu, a + nu))
    mesh = bpy.data.meshes.new(name + "_mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    ob = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(ob)
    common.assign(ob, mat)
    common.smooth(ob)
    sol = ob.modifiers.new("thickness", "SOLIDIFY")
    sol.thickness = 0.028
    bpy.context.view_layer.objects.active = ob
    ob.select_set(True)
    bpy.ops.object.modifier_apply(modifier=sol.name)
    ob.select_set(False)
    common.bevel(ob, 0.008, segments=1)
    return ob


def build():
    common.reset_scene()
    common.setup_render()
    objs, metal, accent = emblemkit.medallion()
    ry = emblemkit.R_Y
    cloth_mat = emblemkit.silk((0.07, 0.003, 0.006, 1.0))

    for sx in (-1, 1):
        fz = (FRONT_Z + POLE_Z0) * 0.5
        fp = common.add_cylinder(
            "pole_front", POLE_R, FRONT_Z - POLE_Z0,
            location=(sx * FRONT_X, ry - 0.015, fz),
            vertices=20, mat=metal)
        common.smooth(fp)
        objs.append(fp)
        objs.append(common.add_uv_sphere(
            "finial_front", 0.072,
            location=(sx * FRONT_X, ry - 0.015, FRONT_Z + 0.02),
            mat=metal, segments=20, rings=10))
        bz = (BACK_Z + POLE_Z0) * 0.5
        bp = common.add_cylinder(
            "pole_back", POLE_R, BACK_Z - POLE_Z0,
            location=(sx * BACK_X, ry + 0.02, bz),
            vertices=20, mat=metal)
        common.smooth(bp)
        objs.append(bp)
        objs.append(common.add_uv_sphere(
            "finial_back", 0.072,
            location=(sx * BACK_X, ry + 0.02, BACK_Z + 0.02),
            mat=metal, segments=20, rings=10))

    objs.append(canopy_cloth("canopy", cloth_mat, ry))

    cord = []
    for i in range(16):
        u = -1.0 + 2.0 * i / 15
        sag = 0.14 * (1.0 - u * u)
        cord.append((FRONT_X * u, ry - 0.055, FRONT_Z - sag))
    objs.append(common.tube_curve("front_trim", cord, 0.020, mat=metal))

    # Short valance under the front edge so the cloth reads as a canopy.
    val = [(-FRONT_X, FRONT_Z), (FRONT_X, FRONT_Z),
           (FRONT_X * 0.92, FRONT_Z - 0.10), (-FRONT_X * 0.92, FRONT_Z - 0.10)]
    objs.append(emblemkit.extrude_polygon(
        "valance", val, ry - 0.01, 0.03, cloth_mat, bevel_width=0.006))

    emblemkit.finish(objs, "semya")


build()
