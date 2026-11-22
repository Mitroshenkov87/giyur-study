"""Closed Sefer Torah standing upright: deep wine velvet mantle with gold
embroidered bands, an embroidered crown motif and gold fringe; silver
rimonim with little bells on top; turned wooden handles below the hem."""

import math
import os
import sys

import bpy
from mathutils import Matrix, Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import objkit

YS = 0.66            # mantle is elliptical in plan
HEM_Z = 0.60
TOP_Z = 2.30
RX = 0.62

MANTLE = [
    (0.0, HEM_Z + 0.02), (RX - 0.02, HEM_Z), (RX + 0.01, HEM_Z + 0.03),
    (RX + 0.005, HEM_Z + 0.5), (RX - 0.01, 1.4), (RX - 0.025, 2.0),
    (RX - 0.04, 2.17), (RX - 0.08, 2.25), (RX - 0.16, TOP_Z - 0.01),
    (0.0, TOP_Z),
]

HANDLE = [  # bottom etz chaim handle, z from 0 up into the mantle
    (0.0, 0.0), (0.075, 0.0), (0.085, 0.03), (0.07, 0.07), (0.05, 0.10),
    (0.06, 0.15), (0.045, 0.21), (0.04, 0.30), (0.055, 0.33), (0.20, 0.355),
    (0.21, 0.385), (0.19, 0.40), (0.05, 0.41), (0.05, 0.62), (0.0, 0.62),
]

TOP_ROD = [(0.0, 2.2), (0.045, 2.2), (0.045, 2.50), (0.0, 2.50)]

RIMON = [  # silver finial on a socket, z0 = 2.38
    (0.0, 2.38), (0.06, 2.38), (0.075, 2.42), (0.05, 2.46), (0.045, 2.53),
    (0.075, 2.57), (0.13, 2.64), (0.15, 2.72), (0.13, 2.80), (0.07, 2.84),
    (0.045, 2.88), (0.07, 2.92), (0.10, 2.97), (0.09, 3.03), (0.05, 3.06),
    (0.03, 3.10), (0.045, 3.14), (0.03, 3.18), (0.0, 3.20),
]

BELL = [(0.0, 0.0), (0.012, 0.0), (0.035, -0.008), (0.037, -0.025),
        (0.030, -0.05), (0.016, -0.065), (0.0, -0.07)]


def mat_wine_velvet():
    m, nt, bs = common._new_material("WineVelvet")
    tc = common._texcoord(nt)
    n = common._noise(nt, scale=6.0, detail=3.0)
    nt.links.new(tc.outputs["Object"], n.inputs["Vector"])
    ramp = common._ramp(nt, [(0.0, (0.040, 0.002, 0.010, 1.0)),
                             (1.0, (0.085, 0.004, 0.020, 1.0))])
    nt.links.new(n.outputs["Fac"], ramp.inputs["Fac"])
    nt.links.new(ramp.outputs["Color"], bs.inputs["Base Color"])
    common._inp(bs, "Roughness", 0.7)
    common._inp(bs, "Sheen Weight", 1.0)
    common._inp(bs, "Sheen Roughness", 0.3)
    common._inp(bs, "Sheen Tint", (0.70, 0.16, 0.24, 1.0))
    fuzz = common._noise(nt, scale=260.0, detail=2.0, y=-320)
    nt.links.new(tc.outputs["Object"], fuzz.inputs["Vector"])
    common._bump(nt, bs, fuzz.outputs["Fac"], strength=0.2, distance=0.006)
    return m


def mat_embroidery():
    """Gold thread: metallic with a fine stitch bump."""
    m, nt, bs = common._new_material("GoldThread")
    common._inp(bs, "Base Color", (0.78, 0.53, 0.17, 1.0))
    common._inp(bs, "Metallic", 0.9)
    common._inp(bs, "Roughness", 0.32)
    tc = common._texcoord(nt)
    w = nt.nodes.new("ShaderNodeTexWave")
    w.inputs["Scale"].default_value = 60.0
    w.bands_direction = "DIAGONAL"
    nt.links.new(tc.outputs["Object"], w.inputs["Vector"])
    common._bump(nt, bs, w.outputs["Fac"], strength=0.35, distance=0.004)
    return m


def surface_y(x, z_scale=1.0):
    """Front surface y of the elliptical mantle at x (mid height)."""
    rx = RX + 0.005
    return -YS * rx * math.sqrt(max(0.0, 1.0 - (x / rx) ** 2))


def crown_emblem(mat, zc, width=0.46):
    """Flat embroidered crown appliqué following the mantle's front curve."""
    w = width
    n = 4                                   # 5 points
    tips = [(-w / 2 + w * i / n, 0.27 if i in (0, n) else (0.33 if i == 2 else 0.30))
            for i in range(n + 1)]
    pts = [(-w / 2, 0.0), (w / 2, 0.0), (w / 2, 0.07)]
    for i in range(n, -1, -1):
        pts.append(tips[i])
        if i > 0:
            pts.append(((tips[i][0] + tips[i - 1][0]) / 2, 0.13))
    pts.append((-w / 2, 0.07))
    # extrude along the surface normal (approx. -y), conform to curvature
    verts = []
    for x, z in pts:
        y = surface_y(x)
        verts.append((x, y - 0.004, zc + z))
    for x, z in pts:
        y = surface_y(x)
        verts.append((x, y - 0.022, zc + z))
    k = len(pts)
    faces = [tuple(range(k))[::-1], tuple(range(k, 2 * k))]
    for i in range(k):
        j = (i + 1) % k
        faces.append((i, j, k + j, k + i))
    mesh = bpy.data.meshes.new("crown_mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    ob = bpy.data.objects.new("crown", mesh)
    bpy.context.collection.objects.link(ob)
    ob.data.materials.append(mat)
    objs = [ob]
    # balls on the crown points and a jewel row on the band
    for x, z in tips:
        objs.append(common.add_uv_sphere("crown_ball", 0.026,
                                         location=(x, surface_y(x) - 0.02, zc + z + 0.02),
                                         mat=mat, segments=16, rings=8))
    for i in range(5):                      # jewels on the band
        x = -w / 2 + w * (i + 0.5) / 5
        objs.append(common.add_uv_sphere("crown_jewel", 0.018,
                                         location=(x, surface_y(x) - 0.024, zc + 0.035),
                                         scale=(1, 0.5, 1), mat=mat, segments=12, rings=6))
    return objs


def build():
    common.reset_scene()
    common.setup_render()
    wood = objkit.mat_dark_walnut("RollerWood", axis="Z")
    velvet = mat_wine_velvet()
    gold = mat_embroidery()
    silver = common.mat_silver()
    objs = []

    mantle = common.lathe("mantle", MANTLE, segments=96, mat=velvet)
    mantle.scale.y = YS
    objs.append(mantle)

    for x in (-0.28, 0.28):
        h = common.lathe("handle", HANDLE, segments=40, mat=wood)
        h.location.x = x
        rod = common.lathe("top_rod", TOP_ROD, segments=24, mat=wood)
        rod.location.x = x
        rim = common.lathe("rimon", RIMON, segments=48, mat=silver)
        rim.location.x = x
        collar = common.add_torus("collar", 0.08, 0.02, location=(x, 0, TOP_Z - 0.005),
                                  mat=gold)
        objs += [h, rod, rim, collar]
        # bells hanging from a little ring of arms
        for k in range(5):
            a = 2 * math.pi * k / 5 + 0.3
            ax, ay = x + 0.17 * math.cos(a), 0.17 * math.sin(a)
            arm = common.tube_curve("arm", [(x + 0.10 * math.cos(a), 0.10 * math.sin(a), 2.79),
                                            (x + 0.15 * math.cos(a), 0.15 * math.sin(a), 2.80),
                                            (ax, ay, 2.77)], 0.008, mat=silver)
            bell = common.lathe("bell", BELL, segments=20, mat=silver)
            bell.location = (ax, ay, 2.755)
            clap = common.add_uv_sphere("clapper", 0.012, location=(ax, ay, 2.68),
                                        mat=silver, segments=10, rings=6)
            objs += [arm, bell, clap]

    # embroidered bands: upper band, hem band, thin pin-lines
    def band(z, r, thick, name="band"):
        t = common.add_torus(name, r, thick, location=(0, 0, z), mat=gold,
                             major_segments=96, minor_segments=10)
        t.scale = (1.0, YS, 0.55)
        return t
    objs += [band(2.05, RX - 0.03, 0.035), band(1.92, RX - 0.017, 0.012),
             band(HEM_Z + 0.10, RX + 0.012, 0.035), band(HEM_Z + 0.22, RX + 0.01, 0.010)]
    # beads between the upper lines
    for k in range(28):
        a = 2 * math.pi * k / 28
        r = RX - 0.012
        objs.append(common.add_uv_sphere("bead", 0.017,
                    location=(r * math.cos(a), YS * r * math.sin(a), 1.985),
                    mat=gold, segments=12, rings=6))
    # gold fringe below the hem
    for k in range(64):
        a = 2 * math.pi * k / 64
        r = RX
        objs.append(common.add_cylinder("fringe", 0.010, 0.07,
                    location=(r * math.cos(a), YS * r * math.sin(a), HEM_Z - 0.02),
                    vertices=6, mat=gold))
    objs += crown_emblem(gold, 1.10, width=0.58)

    common.frame_object(objs, margin=0.08, elevation=18.0, azimuth=28.0, focal=80.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render("torah_rolled", common.OUT_DIR)


build()
