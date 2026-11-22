"""Siddur: closed prayer book — dark leather cover with a gold-embossed
border, gilt page edges, gold spine bands, and a burgundy ribbon bookmark."""

import math
import os
import sys

import bpy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import objkit

# page block / cover sizes
BW, BD, BT = 1.34, 0.98, 0.22          # block width, depth, thickness
CW, CD, CT = 1.46, 1.06, 0.045         # cover width, depth, thickness
COVER_Z = BT / 2.0 + CT / 2.0 + 0.002  # cover centre height
TOP_Z = COVER_Z + CT / 2.0             # front cover top surface


def ribbon(mat):
    pts = [(0.30, 0.30, -0.156), (0.30, 0.05, -0.160), (0.305, -0.30, -0.172),
           (0.29, -0.70, -0.187), (0.30, -1.02, -0.190), (0.30, -1.22, -0.190)]
    width, thick, n = 0.12, 0.006, len(pts)
    verts = []
    for i, (x, y, z) in enumerate(pts):
        t = i / (n - 1.0)
        w = width * (1.0 - max(0.0, t - 0.82) / 0.18)  # taper to a point
        verts += [(x - w / 2.0, y, z), (x + w / 2.0, y, z)]
    verts += [(x, y, z - thick) for (x, y, z) in verts]
    faces = []
    for i in range(n - 1):
        a, b = 2 * i, 2 * i + 1
        faces += [(a, b, b + 2, a + 2),
                  (a + 2 * n, a + 2 + 2 * n, b + 2 + 2 * n, b + 2 * n),
                  (a, a + 2 * n, b + 2 * n, b),
                  (b, b + 2 * n, b + 2 + 2 * n, b + 2)]
    faces.append((0, 1, 2 * n + 1, 2 * n))
    mesh = bpy.data.meshes.new("ribbon_mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    ob = bpy.data.objects.new("ribbon", mesh)
    bpy.context.collection.objects.link(ob)
    ob.data.materials.append(mat)
    return ob


def build():
    common.reset_scene()
    common.setup_render()
    leather = common.mat_leather()
    gold = common.mat_gold()
    parchment = common.mat_parchment()
    ribbon_mat = common.mat_velvet()
    objs = []

    # page block + gilt edges (fore-edge, head, tail)
    objs.append(objkit.cube("pages", (BW, BD, BT), (0, 0, 0), parchment, bevel=0.012))
    objs.append(objkit.cube("gilt_fore", (0.020, BD - 0.02, BT - 0.005),
                            (BW / 2.0 + 0.002, 0, 0), gold))
    objs.append(objkit.cube("gilt_head", (BW - 0.01, BD - 0.02, 0.020),
                            (0, 0, BT / 2.0 + 0.002), gold))
    objs.append(objkit.cube("gilt_tail", (BW - 0.01, BD - 0.02, 0.020),
                            (0, 0, -BT / 2.0 - 0.002), gold))

    # covers and rounded leather spine
    objs.append(objkit.cube("cover_front", (CW, CD, CT), (0, 0, COVER_Z),
                            leather, bevel=0.02))
    objs.append(objkit.cube("cover_back", (CW, CD, CT), (0, 0, -COVER_Z),
                            leather, bevel=0.02))
    spine = common.add_cylinder("spine", 0.133, CD, location=(-BW / 2.0, 0, 0),
                                mat=leather)
    spine.rotation_euler = (math.radians(90.0), 0.0, 0.0)
    common.smooth(spine)
    objs.append(spine)

    # gold spine bands
    for y in (-0.36, 0.0, 0.36):
        band = common.add_torus("spine_band", 0.134, 0.015,
                                location=(-BW / 2.0, y, 0), mat=gold)
        band.rotation_euler = (math.radians(90.0), 0.0, 0.0)
        objs.append(band)

    # gold-embossed double border on the front cover + centre diamond
    z = TOP_Z + 0.004
    for inset, bw in ((0.10, 0.020), (0.17, 0.013)):
        hw, hd = CW / 2.0 - inset, CD / 2.0 - inset
        objs.append(objkit.cube("frame_h", (2 * hw, bw, 0.009), (0, hd, z), gold))
        objs.append(objkit.cube("frame_h", (2 * hw, bw, 0.009), (0, -hd, z), gold))
        objs.append(objkit.cube("frame_v", (bw, 2 * hd, 0.009), (hw, 0, z), gold))
        objs.append(objkit.cube("frame_v", (bw, 2 * hd, 0.009), (-hw, 0, z), gold))
    diamond = objkit.cube("emblem", (0.15, 0.15, 0.010), (0, 0, z + 0.001), gold,
                          rot=(0, 0, math.radians(45.0)))
    objs.append(diamond)

    objs.append(ribbon(ribbon_mat))

    common.frame_object(objs)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render("siddur", common.OUT_DIR)


build()
