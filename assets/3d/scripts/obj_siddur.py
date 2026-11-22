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


def mat_black_leather():
    """Near-black brown leather; rough enough that the big key light does not
    wash the flat cover out to beige."""
    m, nt, bs = common._new_material("SiddurLeather")
    tc = common._texcoord(nt)
    n = common._noise(nt, scale=4.0, detail=3.0)
    nt.links.new(tc.outputs["Object"], n.inputs["Vector"])
    ramp = common._ramp(nt, [(0.0, (0.0025, 0.0014, 0.0009, 1.0)),
                             (1.0, (0.008, 0.0042, 0.0024, 1.0))])
    nt.links.new(n.outputs["Fac"], ramp.inputs["Fac"])
    nt.links.new(ramp.outputs["Color"], bs.inputs["Base Color"])
    grain = nt.nodes.new("ShaderNodeTexVoronoi")
    grain.inputs["Scale"].default_value = 160.0
    nt.links.new(tc.outputs["Object"], grain.inputs["Vector"])
    common._bump(nt, bs, grain.outputs["Distance"], strength=0.25, distance=0.004)
    common._inp(bs, "Roughness", 0.62)
    common._inp(bs, "Specular IOR Level", 0.25)
    return m


def build():
    common.reset_scene()
    common.setup_render()
    leather = mat_black_leather()
    gold = common.mat_gold()
    parchment = common.mat_parchment()
    ribbon_mat = objkit.mat_ink((0.05, 0.0015, 0.007, 1.0))
    ribbon_mat.node_tree.nodes.get("Principled BSDF").inputs["Roughness"].default_value = 0.6
    objs = []

    # page block + gilt edges (fore-edge, head, tail)
    objs.append(objkit.cube("pages", (BW, BD, BT), (0, 0, 0), parchment, bevel=0.012))
    objs.append(objkit.cube("gilt_fore", (0.020, BD - 0.02, BT - 0.005),
                            (BW / 2.0 + 0.002, 0, 0), gold))
    objs.append(objkit.cube("gilt_head", (BW - 0.04, 0.020, BT - 0.005),
                            (0.01, BD / 2.0 + 0.002, 0), gold))
    objs.append(objkit.cube("gilt_tail", (BW - 0.04, 0.020, BT - 0.005),
                            (0.01, -BD / 2.0 - 0.002, 0), gold))

    # covers and rounded leather spine
    objs.append(objkit.cube("cover_front", (CW, CD, CT), (0, 0, COVER_Z),
                            leather, bevel=0.02))
    objs.append(objkit.cube("cover_back", (CW, CD, CT), (0, 0, -COVER_Z),
                            leather, bevel=0.02))
    spine = common.add_cylinder("spine", COVER_Z + CT / 2.0, CD,
                                location=(-BW / 2.0 + 0.02, 0, 0), mat=leather,
                                vertices=64)
    spine.scale.x = 0.38
    spine.rotation_euler = (math.radians(90.0), 0.0, 0.0)
    common.smooth(spine)
    objs.append(spine)

    # gold spine bands
    for y in (-0.36, 0.0, 0.36):
        band = common.add_torus("spine_band", COVER_Z + CT / 2.0, 0.012,
                                location=(-BW / 2.0 + 0.02, y, 0), mat=gold)
        band.scale.x = 0.40
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
    # corner fleurons + central medallion (ring with a lozenge)
    for sx in (-1, 1):
        for sy in (-1, 1):
            objs.append(objkit.cube("fleuron", (0.06, 0.06, 0.010),
                                    (sx * (CW / 2.0 - 0.135), sy * (CD / 2.0 - 0.135), z),
                                    gold, rot=(0, 0, math.radians(45.0))))
    med = common.add_torus("medallion", 0.15, 0.012, location=(0, 0, z), mat=gold)
    med.scale = (1.0, 1.25, 0.4)
    objs.append(med)
    diamond = objkit.cube("emblem", (0.11, 0.11, 0.010), (0, 0, z + 0.001), gold,
                          rot=(0, 0, math.radians(45.0)))
    diamond.scale.y = 1.25
    objs.append(diamond)

    objs.append(ribbon(ribbon_mat))

    common.frame_object(objs, margin=0.08, elevation=30.0, azimuth=32.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render("siddur", common.OUT_DIR)


build()
