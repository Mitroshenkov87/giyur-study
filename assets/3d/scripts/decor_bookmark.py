"""Decor: deep burgundy silk ribbon bookmark with a brass tip (both themes)."""

import math
import os
import sys

import bpy
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import emblemkit

# Smooth hanging ribbon, top -> bottom. A sine keeps it silk, not a bent bar.
PATH = []
for _i in range(14):
    _t = _i / 13.0
    PATH.append((0.085 * math.sin(_t * math.pi * 1.35), 1.00 - _t * 1.42))
V_BL = (-0.16, -0.50)
V_BR = (0.12, -0.50)
V_TIP = (-0.02, -0.74)
HALF_W = 0.155


def ribbon_mesh(name, mat):
    n = len(PATH)
    verts = []
    for i in range(n):
        a = PATH[max(i - 1, 0)]
        b = PATH[min(i + 1, n - 1)]
        d = Vector((b[0] - a[0], 0.0, b[1] - a[1])).normalized()
        p = Vector((-d.z, 0.0, d.x)) * HALF_W
        c = Vector((PATH[i][0], 0.0, PATH[i][1]))
        verts.append(tuple(c - p))
        verts.append(tuple(c + p))
    verts += [(V_BL[0], 0.0, V_BL[1]), (V_BR[0], 0.0, V_BR[1]),
              (V_TIP[0], 0.0, V_TIP[1])]
    ibl, ibr, itip = 2 * n, 2 * n + 1, 2 * n + 2
    faces = []
    for i in range(n - 1):
        l0, r0, l1, r1 = 2 * i, 2 * i + 1, 2 * i + 2, 2 * i + 3
        faces.append((l0, l1, r1, r0))
    faces.append((2 * n - 2, ibl, ibr, 2 * n - 1))
    faces.append((ibl, itip, ibr))
    mesh = bpy.data.meshes.new(name + "_mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    ob = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(ob)
    common.assign(ob, mat)
    sol = ob.modifiers.new("thickness", "SOLIDIFY")
    sol.thickness = 0.030
    bpy.context.view_layer.objects.active = ob
    ob.select_set(True)
    bpy.ops.object.modifier_apply(modifier=sol.name)
    ob.select_set(False)
    common.bevel(ob, 0.006, segments=1)
    return ob


def build():
    common.reset_scene()
    common.setup_render()
    metal = emblemkit.medal_metal(0.15)
    # Deep burgundy, saturated. Sheen stays tinted inside emblemkit.silk.
    silk = emblemkit.silk((0.065, 0.003, 0.005, 1.0))
    objs = [ribbon_mesh("ribbon", silk)]

    tip_outline = [(-0.185, -0.44), (0.145, -0.44), (0.10, -0.58),
                   (-0.02, -0.80), (-0.16, -0.58)]
    n = len(tip_outline)
    depth = 0.07
    y0 = -0.02
    verts = [(x, y0, z) for x, z in tip_outline] + [
        (x, y0 - depth, z) for x, z in tip_outline]
    faces = [tuple(range(n - 1, -1, -1)), tuple(range(n, 2 * n))]
    for i in range(n):
        j = (i + 1) % n
        faces.append((i, j, n + j, n + i))
    mesh = bpy.data.meshes.new("tip_mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    tip = bpy.data.objects.new("brass_tip", mesh)
    bpy.context.collection.objects.link(tip)
    common.assign(tip, metal)
    common.bevel(tip, 0.016, segments=2)
    objs.append(tip)

    loop = common.add_torus(
        "loop", 0.090, 0.024, location=(0.0, -0.02, 1.05),
        mat=metal, major_segments=36, minor_segments=12)
    loop.rotation_euler = (math.radians(90), 0, 0)
    objs.append(loop)

    common.frame_object(objs, margin=0.12, focal=72.0,
                        elevation=14.0, azimuth=22.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render(common.variant_name("bookmark"), common.OUT_DIR)


build()
