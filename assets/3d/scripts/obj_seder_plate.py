"""Seder plate: ceramic plate with a cobalt-blue and gold rim and six
marked places holding large, readable Pesach foods: zeroa (roasted
shankbone), beitzah (brown roasted egg), maror (romaine leaves), charoset,
karpas (parsley) and chazeret (horseradish root with leaves)."""

import math
import os
import random
import sys

import bpy
from mathutils import Matrix, Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import foodkit

PLATE_PROFILE = [
    (0.0, 0.10), (1.20, 0.11), (1.34, 0.13), (1.42, 0.19), (1.60, 0.235),
    (1.66, 0.25), (1.69, 0.235), (1.68, 0.16), (1.60, 0.07), (1.05, 0.035),
    (0.70, 0.03), (0.66, 0.0), (0.0, 0.0),
]
TOP = 0.10
RING_R = 0.90


def mat_plate():
    """Glazed ceramic: warm white centre, cobalt band on the rim."""
    m, nt, bs = common._new_material("SederCeramic")
    L = nt.links
    tc = common._texcoord(nt)
    sep = nt.nodes.new("ShaderNodeSeparateXYZ")
    L.new(tc.outputs["Object"], sep.inputs["Vector"])
    comb = nt.nodes.new("ShaderNodeCombineXYZ")
    L.new(sep.outputs["X"], comb.inputs["X"])
    L.new(sep.outputs["Y"], comb.inputs["Y"])
    ln = nt.nodes.new("ShaderNodeVectorMath")
    ln.operation = "LENGTH"
    L.new(comb.outputs["Vector"], ln.inputs[0])
    ramp = common._ramp(nt, [(0.0, (0.66, 0.64, 0.58, 1.0)),
                             (0.395, (0.66, 0.64, 0.58, 1.0)),
                             (0.40, (0.020, 0.045, 0.22, 1.0)),
                             (0.475, (0.025, 0.055, 0.26, 1.0)),
                             (0.48, (0.66, 0.64, 0.58, 1.0)),
                             (1.0, (0.66, 0.64, 0.58, 1.0))])
    ramp.color_ramp.interpolation = "LINEAR"
    mr = nt.nodes.new("ShaderNodeMath")
    mr.operation = "DIVIDE"
    mr.inputs[1].default_value = 3.4
    L.new(ln.outputs["Value"], mr.inputs[0])
    L.new(mr.outputs["Value"], ramp.inputs["Fac"])
    L.new(ramp.outputs["Color"], bs.inputs["Base Color"])
    common._inp(bs, "Roughness", 0.18)
    common._inp(bs, "Coat Weight", 0.5)
    common._inp(bs, "Coat Roughness", 0.06)
    return m


def place(grp, loc, rot, scale=1.45):
    bpy.context.view_layer.update()
    m = (Matrix.Translation(loc) @ Matrix.Rotation(rot, 4, "Z")
         @ Matrix.Diagonal((scale, scale, scale, 1.0)))
    for ob in grp:
        ob.matrix_world = m @ ob.matrix_world
    return grp


def leaf(name, length, width, mat, curl=0.35, ruffle=0.025, seed=0, nu=18, nv=7):
    """Romaine-like leaf: pale midrib along +X, ruffled edges, curling up."""
    rng = random.Random(seed)
    verts, faces = [], []
    for i in range(nu + 1):
        t = i / nu
        x = length * t
        z0 = curl * length * t ** 2
        hw = 0.5 * width * math.sin(math.pi * min(1.0, 0.15 + 0.95 * t)) ** 0.7
        for j in range(nv + 1):
            v = -1.0 + 2.0 * j / nv
            y = hw * v
            z = z0 + 0.35 * hw * v * v + ruffle * abs(v) ** 2 * math.sin(28 * t + 3 * v + seed)
            verts.append((x, y, z))
    for i in range(nu):
        for j in range(nv):
            a = i * (nv + 1) + j
            faces.append((a, a + nv + 1, a + nv + 2, a + 1))
    mesh = bpy.data.meshes.new(name + "_mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    ob = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(ob)
    ob.data.materials.append(mat)
    common.smooth(ob)
    sol = ob.modifiers.new("solidify", "SOLIDIFY")
    sol.thickness = 0.008
    return ob


def make_materials():
    M = {}
    M["roast"] = foodkit.prin("Roast", (0.20, 0.08, 0.03, 1.0), rough=0.45, coat=0.2,
                              ramp=[(0.0, (0.08, 0.03, 0.012, 1.0)),
                                    (0.5, (0.24, 0.10, 0.035, 1.0)),
                                    (1.0, (0.34, 0.16, 0.06, 1.0))],
                              ramp_scale=8.0, bump=0.4, bump_scale=35.0)
    M["bone"] = foodkit.prin("Bone", (0.62, 0.55, 0.42, 1.0), rough=0.5,
                             ramp=[(0.0, (0.50, 0.40, 0.28, 1.0)),
                                   (1.0, (0.66, 0.58, 0.44, 1.0))], ramp_scale=6.0)
    M["egg"] = foodkit.prin("RoastEgg", (0.34, 0.18, 0.08, 1.0), rough=0.45, coat=0.15,
                            ramp=[(0.0, (0.08, 0.04, 0.02, 1.0)),
                                  (0.30, (0.26, 0.13, 0.05, 1.0)),
                                  (0.6, (0.40, 0.23, 0.10, 1.0)),
                                  (1.0, (0.46, 0.28, 0.13, 1.0))],
                            ramp_scale=7.0, bump=0.1, bump_scale=60.0)
    M["romaine"] = foodkit.prin("Romaine", (0.10, 0.36, 0.06, 1.0), rough=0.45,
                                subsurface=0.1, coat=0.15,
                                ramp=[(0.0, (0.06, 0.24, 0.03, 1.0)),
                                      (0.6, (0.14, 0.42, 0.07, 1.0)),
                                      (1.0, (0.36, 0.55, 0.16, 1.0))],
                                ramp_scale=5.0, bump=0.3, bump_scale=40.0)
    M["horseradish"] = foodkit.prin("Horseradish", (0.40, 0.30, 0.18, 1.0), rough=0.75,
                                    ramp=[(0.0, (0.26, 0.18, 0.09, 1.0)),
                                          (1.0, (0.48, 0.38, 0.24, 1.0))],
                                    ramp_scale=10.0, bump=0.5, bump_scale=30.0)
    M["horse_cut"] = foodkit.prin("HorseradishCut", (0.68, 0.64, 0.52, 1.0), rough=0.6)
    M["charoset"] = foodkit.prin("Charoset", (0.22, 0.09, 0.03, 1.0), rough=0.6,
                                 ramp=[(0.0, (0.10, 0.035, 0.012, 1.0)),
                                       (0.5, (0.24, 0.10, 0.035, 1.0)),
                                       (1.0, (0.36, 0.17, 0.06, 1.0))],
                                 ramp_scale=14.0, bump=0.7, bump_scale=45.0)
    M["nut"] = foodkit.prin("Walnut", (0.42, 0.26, 0.12, 1.0), rough=0.6, bump=0.4)
    M["parsley"] = foodkit.prin("Parsley", (0.05, 0.28, 0.04, 1.0), rough=0.45,
                                coat=0.2, subsurface=0.1,
                                ramp=[(0.0, (0.03, 0.18, 0.02, 1.0)),
                                      (1.0, (0.10, 0.40, 0.06, 1.0))],
                                ramp_scale=12.0, bump=0.4, bump_scale=60.0)
    M["stem"] = foodkit.prin("Stem", (0.20, 0.42, 0.10, 1.0), rough=0.5)
    return M


def zeroa(M):
    """Shankbone lying flat: bone shaft with knobby ends, roasted meat on one end."""
    pts = [Vector((-0.26, 0, 0.07)), Vector((-0.05, 0.01, 0.065)), Vector((0.18, -0.01, 0.07))]
    shaft = foodkit.tapered_tube("zeroa_shaft", pts, [0.045, 0.036, 0.042], sides=16,
                                 mat=M["bone"])
    common.smooth(shaft)
    k1 = common.add_uv_sphere("zeroa_knob", 0.075, location=(-0.29, 0.03, 0.07),
                              scale=(0.8, 1.0, 0.85), mat=M["bone"])
    k2 = common.add_uv_sphere("zeroa_knob", 0.06, location=(-0.28, -0.05, 0.065),
                              mat=M["bone"])
    meat = common.add_uv_sphere("zeroa_meat", 0.14, location=(0.15, 0, 0.10),
                                scale=(1.35, 1.0, 0.85), mat=M["roast"])
    for v in meat.data.vertices:
        v.co *= 1.0 + 0.12 * math.sin(7 * v.co.x / 0.14 + 2 * v.co.y / 0.14)
    return [shaft, k1, k2, meat]


def beitzah(M):
    prof = [(0.0, 0.0), (0.09, 0.01), (0.135, 0.07), (0.145, 0.14),
            (0.13, 0.21), (0.09, 0.27), (0.0, 0.30)]
    egg = common.lathe("beitzah", prof, segments=48, mat=M["egg"])
    egg.rotation_euler = (0, math.radians(90), math.radians(90))
    egg.location = (0.0, -0.15, 0.14)
    return [egg]


def maror(M):
    grp = []
    for i, (a, ln, w, c) in enumerate(((-0.6, 0.48, 0.22, 0.30), (0.4, 0.44, 0.20, 0.42),
                                      (2.6, 0.40, 0.18, 0.36), (1.6, 0.36, 0.17, 0.5))):
        lf = leaf("maror_leaf", ln, w, M["romaine"], curl=c, seed=i)
        lf.rotation_euler = (0, -0.12, a)
        lf.location = (-0.18 * math.cos(a), -0.18 * math.sin(a), 0.01 + 0.012 * i)
        grp.append(lf)
    return grp


def charoset(M):
    rng = random.Random(5)
    mound = common.add_uv_sphere("charoset", 0.22, location=(0, 0, 0.0),
                                 scale=(1.0, 1.0, 0.55), mat=M["charoset"],
                                 segments=40, rings=20)
    for v in mound.data.vertices:
        v.co *= 1.0 + 0.06 * math.sin(11 * v.co.x / 0.22) * math.cos(9 * v.co.y / 0.22)
    grp = [mound]
    for i in range(7):
        a = rng.uniform(0, 6.28)
        r = rng.uniform(0.0, 0.15)
        z = 0.11 * math.sqrt(max(0.0, 1 - (r / 0.22) ** 2)) + 0.005
        nut = foodkit.rounded_box("nut", (0.05, 0.04, 0.03),
                                  (r * math.cos(a), r * math.sin(a), z), M["nut"],
                                  bevel_w=0.01, segments=2)
        nut.rotation_euler = (rng.uniform(-0.4, 0.4), rng.uniform(-0.4, 0.4), a)
        grp.append(nut)
    return grp


def karpas(M):
    """Parsley sprigs: stems ending in clusters of ruffled leaflets."""
    rng = random.Random(9)
    grp = []
    for s in range(4):
        a = -0.9 + s * 0.6
        tip = Vector((0.10 * math.cos(a) + 0.10, 0.20 * math.sin(a), 0.11))
        base = Vector((-0.22, 0.0, 0.03))
        mid = (base + tip) * 0.5 + Vector((0, 0, 0.03))
        grp.append(common.tube_curve("stem", [base, mid, tip], 0.012, mat=M["stem"]))
        for k in range(9):
            off = Vector((rng.uniform(-0.07, 0.07), rng.uniform(-0.07, 0.07),
                          rng.uniform(-0.01, 0.04)))
            lf = common.add_uv_sphere("parsley", 0.045, location=tip + off,
                                      scale=(1.0, 0.8, 0.55), mat=M["parsley"],
                                      segments=12, rings=8)
            for v in lf.data.vertices:
                v.co *= 1.0 + 0.18 * math.sin(6 * math.atan2(v.co.y, v.co.x))
            lf.rotation_euler = (rng.uniform(-0.5, 0.5), rng.uniform(-0.5, 0.5),
                                 rng.uniform(0, 6))
            grp.append(lf)
    return grp


def chazeret(M):
    """Horseradish root lying down, cut face showing, with a few leaves."""
    pts = [Vector((-0.24, 0, 0.065)), Vector((-0.05, 0.02, 0.06)),
           Vector((0.14, -0.02, 0.045)), Vector((0.27, 0.01, 0.025))]
    root = foodkit.tapered_tube("horseradish", pts, [0.065, 0.06, 0.045, 0.012],
                                sides=18, mat=M["horseradish"])
    common.smooth(root)
    cut = common.add_cylinder("cut", 0.063, 0.01, location=(-0.245, 0, 0.065),
                              mat=M["horse_cut"], vertices=24)
    cut.rotation_euler = (0, math.radians(90), 0)
    grp = [root, cut]
    for i, a in enumerate((2.6, 3.4)):
        lf = leaf("chazeret_leaf", 0.34, 0.16, M["romaine"], curl=0.3, seed=10 + i)
        lf.rotation_euler = (0, -0.1, a)
        lf.location = (-0.20, 0.06 * (i - 0.5), 0.03)
        grp.append(lf)
    return grp


def build():
    common.reset_scene()
    common.setup_render()
    M = make_materials()
    plate_mat = mat_plate()
    gold = common.mat_gold()
    objs = []

    plate = common.lathe("plate", PLATE_PROFILE, segments=128, mat=plate_mat)
    objs.append(plate)
    for r, z, t in ((1.675, 0.245, 0.016), (1.36, 0.13, 0.010)):
        objs.append(common.add_torus("gold_line", r, t, location=(0, 0, z), mat=gold,
                                     major_segments=128))

    builders = (zeroa, beitzah, maror, charoset, karpas, chazeret)
    for k, fn in enumerate(builders):
        ang = math.radians(90 + k * 60)
        x, y = RING_R * math.cos(ang), RING_R * math.sin(ang)
        ring = common.add_torus("place_ring", 0.38, 0.010, location=(x, y, TOP + 0.003),
                                mat=gold, major_segments=64)
        ring.scale.z = 0.4
        objs.append(ring)
        grp = fn(M)
        objs += place(grp, Vector((x, y, TOP)), ang + math.radians(90))

    common.frame_object(objs, margin=0.04, elevation=40.0, azimuth=18.0,
                        focal=68.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render("seder_plate", common.OUT_DIR)


build()
