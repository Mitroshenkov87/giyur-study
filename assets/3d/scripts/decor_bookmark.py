"""Decor: burgundy silk ribbon bookmark with a brass tip (both themes)."""

import math
import os
import sys

import bpy
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common

# centreline of the hanging ribbon (x, z), top -> bottom
PATH = [(0.00, 0.95), (0.06, 0.70), (0.08, 0.42), (0.02, 0.14),
        (-0.05, -0.14), (-0.04, -0.40)]
V_BL = (-0.17, -0.54)   # bottom-left of the V cut
V_BR = (0.09, -0.54)    # bottom-right of the V cut
V_TIP = (-0.04, -0.72)  # V tip
HALF_W = 0.125


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
    faces.append((2 * n - 2, ibl, ibr, 2 * n - 1))  # bottom edge
    faces.append((ibl, itip, ibr))                  # the V
    mesh = bpy.data.meshes.new(name + "_mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    ob = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(ob)
    common.assign(ob, mat)
    sol = ob.modifiers.new("thickness", "SOLIDIFY")
    sol.thickness = 0.018
    bpy.context.view_layer.objects.active = ob
    ob.select_set(True)
    bpy.ops.object.modifier_apply(modifier=sol.name)
    ob.select_set(False)
    common.bevel(ob, 0.008, segments=2)
    return ob


def build():
    common.reset_scene()
    common.setup_render()
    metal = common.theme_metal()
    silk = common.mat_velvet()
    objs = []

    objs.append(ribbon_mesh("ribbon", silk))

    # brass sleeve over the V-cut end
    tip_outline = [(-0.185, -0.49), (0.105, -0.49), (0.075, -0.60),
                   (-0.04, -0.76), (-0.155, -0.60)]
    n = len(tip_outline)
    y_front, depth = -0.045, 0.055
    verts = [(x, -0.02, z) for x, z in tip_outline] + [(x, -0.02 - depth, z)
                                                       for x, z in tip_outline]
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
    common.bevel(tip, 0.018, segments=3)
    objs.append(tip)

    # hanging loop at the top
    loop = common.add_torus("loop", 0.085, 0.022, location=(0.0, -0.01, 1.03),
                            mat=metal, major_segments=40, minor_segments=12)
    loop.rotation_euler = (math.radians(90), 0, 0)
    objs.append(loop)

    common.frame_object(objs, margin=0.12, focal=72.0,
                        elevation=14.0, azimuth=24.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render(common.variant_name("bookmark"), common.OUT_DIR)


build()
