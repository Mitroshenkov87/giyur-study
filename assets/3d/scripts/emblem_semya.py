"""Emblem: chuppah wedding canopy on four poles (semya = family)."""

import math
import os
import sys

import bpy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import emblemkit

FRONT_Z, BACK_Z = 0.30, 0.54
FRONT_X, BACK_X = 0.42, 0.30
POLE_BOTTOM = -0.46


def canopy_cloth(name, mat, ry):
    """Draped cloth: bilinear patch front edge -> back edge with sag."""
    NU, NV = 12, 6
    verts = []
    for j in range(NV):
        t = j / (NV - 1)                      # 0 front -> 1 back
        x_r = FRONT_X + (BACK_X - FRONT_X) * t
        z0 = FRONT_Z + (BACK_Z - FRONT_Z) * t
        y = ry + (-0.01) + (0.09 * t)
        for i in range(NU):
            u = -1.0 + 2.0 * i / (NU - 1)
            sag = (0.10 * math.sin(math.pi * t) + 0.05 * (1.0 - t)) * (1.0 - u * u)
            verts.append((x_r * u, y, z0 - sag))
    faces = []
    for j in range(NV - 1):
        for i in range(NU - 1):
            a = j * NU + i
            faces.append((a, a + 1, a + 1 + NU, a + NU))
    mesh = bpy.data.meshes.new(name + "_mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    ob = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(ob)
    common.assign(ob, mat)
    common.smooth(ob)
    sol = ob.modifiers.new("thickness", "SOLIDIFY")
    sol.thickness = 0.022
    bpy.context.view_layer.objects.active = ob
    ob.select_set(True)
    bpy.ops.object.modifier_apply(modifier=sol.name)
    ob.select_set(False)
    common.bevel(ob, 0.012, segments=2)
    return ob


def build():
    common.reset_scene()
    common.setup_render()
    objs, metal, accent = emblemkit.medallion()
    ry = emblemkit.R_Y
    cloth = common.mat_velvet()

    # four poles: two front (lower), two back (taller, behind the cloth)
    for sx in (-1, 1):
        fp = common.add_cylinder(
            "pole_front", 0.030, FRONT_Z - POLE_BOTTOM,
            location=(sx * FRONT_X, ry - 0.01, (FRONT_Z + POLE_BOTTOM) * 0.5),
            vertices=24, mat=metal)
        common.smooth(fp)
        objs.append(fp)
        objs.append(common.add_uv_sphere(
            "finial_front", 0.048, location=(sx * FRONT_X, ry - 0.01, FRONT_Z + 0.03),
            mat=metal, segments=24, rings=12))
        bp = common.add_cylinder(
            "pole_back", 0.030, BACK_Z - POLE_BOTTOM,
            location=(sx * BACK_X, ry + 0.09, (BACK_Z + POLE_BOTTOM) * 0.5),
            vertices=24, mat=metal)
        common.smooth(bp)
        objs.append(bp)
        objs.append(common.add_uv_sphere(
            "finial_back", 0.048, location=(sx * BACK_X, ry + 0.09, BACK_Z + 0.03),
            mat=metal, segments=24, rings=12))

    objs.append(canopy_cloth("canopy", cloth, ry))

    # gold trim cord along the sagging front edge
    cord = []
    for i in range(13):
        u = -1.0 + 2.0 * i / 12
        cord.append((FRONT_X * u, ry - 0.03, FRONT_Z - 0.05 * (1.0 - u * u)))
    objs.append(common.tube_curve("front_trim", cord, 0.018, mat=metal))

    emblemkit.finish(objs, "semya")


build()
