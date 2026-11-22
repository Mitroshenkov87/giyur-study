"""Lulav and etrog: a full, closed palm frond (lulav) with myrtle (hadassim,
tight whorls of small oval leaves) and willow (aravot, long narrow leaves)
bound in a woven green holder, lying diagonally; a bright yellow bumpy
etrog with its pitam in an open silver box lined with velvet."""

import math
import os
import random
import sys

import bmesh
import bpy
from mathutils import Matrix, Vector, noise

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import foodkit

L = 2.55            # lulav length (local +Z)
STALK = 0.40        # bare stalk at the bottom
HOLDER = (0.34, 0.78)


def width(z):
    """Half-width of the closed frond along its length (local X)."""
    t = min((z - STALK) / (L - STALK), 1.0)
    if t < 0:
        return 0.06
    return 0.06 + 0.075 * math.sin(math.pi * min(1.0, 0.25 + 0.9 * t)) ** 0.6 * (1 - t) ** 0.35


def mat_palm():
    m, nt, bs = common._new_material("PalmLeaf")
    tc = common._texcoord(nt)
    w = nt.nodes.new("ShaderNodeTexWave")
    w.bands_direction = "X"
    w.inputs["Scale"].default_value = 26.0
    w.inputs["Distortion"].default_value = 0.6
    nt.links.new(tc.outputs["Object"], w.inputs["Vector"])
    n = common._noise(nt, scale=1.5, detail=2.0)
    nt.links.new(tc.outputs["Object"], n.inputs["Vector"])
    mix = nt.nodes.new("ShaderNodeMath")
    mix.operation = "MULTIPLY_ADD"
    mix.inputs[1].default_value = 0.35
    nt.links.new(w.outputs["Fac"], mix.inputs[0])
    nt.links.new(n.outputs["Fac"], mix.inputs[2])
    ramp = common._ramp(nt, [(0.0, (0.05, 0.12, 0.025, 1.0)),
                             (0.45, (0.10, 0.20, 0.04, 1.0)),
                             (0.8, (0.20, 0.27, 0.07, 1.0)),
                             (1.0, (0.30, 0.32, 0.12, 1.0))])
    nt.links.new(mix.outputs["Value"], ramp.inputs["Fac"])
    nt.links.new(ramp.outputs["Color"], bs.inputs["Base Color"])
    common._bump(nt, bs, w.outputs["Fac"], strength=0.2, distance=0.004)
    common._inp(bs, "Roughness", 0.35)
    common._inp(bs, "Coat Weight", 0.25)
    return m


def mesh_obj(name, bm, mat):
    mesh = bpy.data.meshes.new(name + "_mesh")
    bm.to_mesh(mesh)
    bm.free()
    ob = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(ob)
    ob.data.materials.append(mat)
    common.smooth(ob)
    return ob


def blade(bm, x0, layer, z0, z1, bw, spread, phase):
    """One closed palm leaflet: V-folded strip along +Z, tip splays slightly."""
    n = 28
    rows = []
    for i in range(n + 1):
        t = i / n
        z = z0 + (z1 - z0) * t
        hw = bw * (1.0 - t ** 4) ** 0.6 * (0.6 + 0.4 * min(1.0, t * 6)) + 0.001
        xc = x0 * width(z) / 0.085 + spread * t ** 3 + 0.004 * math.sin(9 * t + phase)
        yc = layer * (0.018 + 0.01 * t)
        fold = 0.6 * hw
        rows.append([bm.verts.new((xc - hw, yc + layer * fold * 0.3, z)),
                     bm.verts.new((xc, yc - layer * fold * 0.5, z)),
                     bm.verts.new((xc + hw, yc + layer * fold * 0.3, z))])
    for i in range(n):
        for j in range(2):
            bm.faces.new((rows[i][j], rows[i + 1][j], rows[i + 1][j + 1], rows[i][j + 1]))


def lulav(palm, stalk_mat):
    rng = random.Random(2)
    objs = []
    # core / stalk (flattened tapered tube)
    pts = [Vector((0, 0, z)) for z in (0.0, 0.3, 0.8, 1.5, 2.1, L - 0.05)]
    radii = [0.045, 0.044, 0.042, 0.035, 0.022, 0.006]
    core = foodkit.tapered_tube("lulav_core", pts, radii, sides=16, mat=stalk_mat)
    core.scale = (1.0, 0.7, 1.0)
    objs.append(core)
    bm = bmesh.new()
    nb = 9
    for layer in (-1, 1):
        for k in range(nb):
            x0 = (-1.0 + 2.0 * (k + 0.5) / nb) * 0.075
            z0 = STALK + rng.uniform(0.0, 0.25)
            z1 = L - 0.02 - rng.uniform(0.15, 0.4) * (abs(x0) / 0.085) ** 1.5 - rng.uniform(0, 0.06)
            spread = (x0 / 0.085) * rng.uniform(0.005, 0.03)
            blade(bm, x0, layer, z0, z1, 0.034, spread, rng.uniform(0, 6))
    # central closed leaf (the tiyomet) running to the tip
    blade(bm, 0.0, 1, STALK, L + 0.06, 0.02, 0.0, 0.0)
    fr = mesh_obj("lulav_blades", bm, palm)
    sol = fr.modifiers.new("solidify", "SOLIDIFY")
    sol.thickness = 0.004
    objs.append(fr)
    return objs


def holder(mat):
    prof = [(0.0, HOLDER[0]), (0.085, HOLDER[0]), (0.10, HOLDER[0] + 0.03),
            (0.115, 0.5 * (HOLDER[0] + HOLDER[1])), (0.13, HOLDER[1] - 0.03),
            (0.14, HOLDER[1]), (0.12, HOLDER[1] + 0.005), (0.0, HOLDER[1])]
    ob = common.lathe("koishelekh", prof, segments=40, mat=mat)
    ob.scale = (1.15, 0.85, 1.0)
    rings = []
    for z in (HOLDER[0] + 0.05, HOLDER[1] - 0.04):
        r = common.add_torus("holder_ring", 0.12 if z > 0.5 else 0.10, 0.012,
                             location=(0, 0, z), mat=mat, major_segments=40)
        r.scale = (1.15, 0.85, 1.0)
        rings.append(r)
    return [ob] + rings


def hadassim(leaf_mat, stem_mat):
    """Three myrtle branches with tight whorls of 3 oval leaves."""
    rng = random.Random(5)
    bm = bmesh.new()
    stems = []
    for b, (bx, by, lean) in enumerate(((0.10, 0.06, 0.10), (0.13, -0.05, 0.16),
                                        (0.05, 0.10, 0.05))):
        top = 1.55 + 0.08 * b
        base = Vector((bx * 0.7, by * 0.7, HOLDER[0] + 0.1))
        tipv = Vector((bx + lean, by, top))
        stems.append(common.tube_curve("hadas_stem", [base, (base + tipv) / 2, tipv],
                                       0.008, mat=stem_mat))
        z = HOLDER[1] - 0.04
        k = 0
        while z < top - 0.01:
            t = (z - base.z) / (top - base.z)
            c = base.lerp(tipv, t)
            size = 0.085 * (1.0 - 0.45 * max(0.0, (t - 0.75) / 0.25))
            for j in range(3):
                a = j * 2 * math.pi / 3 + k * math.pi / 3 + rng.uniform(-0.15, 0.15)
                d = Vector((math.cos(a), math.sin(a), 0.0))
                up = 0.95  # leaves point steeply upward (overlapping scales)
                ax = (d * 0.55 + Vector((0, 0, up))).normalized()
                q = Vector((0, 0, 1)).rotation_difference(ax)
                mtx = (Matrix.Translation(c + d * 0.012 + ax * size * 0.55)
                       @ q.to_matrix().to_4x4()
                       @ Matrix.Diagonal((size * 0.42, size * 0.18, size * 0.55, 1.0)))
                bmesh.ops.create_uvsphere(bm, u_segments=10, v_segments=6, radius=1.0,
                                          matrix=mtx)
            z += 0.05
            k += 1
    return stems + [mesh_obj("hadas_leaves", bm, leaf_mat)]


def aravot(leaf_mat, stem_mat):
    """Two willow branches: reddish stems, long narrow alternating leaves."""
    rng = random.Random(8)
    bm = bmesh.new()
    stems = []
    for b, (bx, by, lean) in enumerate(((-0.11, 0.04, -0.14), (-0.08, -0.07, -0.08))):
        top = 1.42 + 0.06 * b
        base = Vector((bx * 0.6, by * 0.6, HOLDER[0] + 0.1))
        tipv = Vector((bx + lean, by, top))
        stems.append(common.tube_curve("arava_stem", [base, (base + tipv) / 2, tipv],
                                       0.007, mat=stem_mat))
        z = HOLDER[1] - 0.02
        k = 0
        while z < top - 0.03:
            t = (z - base.z) / (top - base.z)
            c = base.lerp(tipv, t)
            a = k * 2.4 + rng.uniform(-0.3, 0.3)
            d = Vector((math.cos(a), math.sin(a), 0.0))
            ax = (d * 0.35 + Vector((0, 0, 1))).normalized()
            q = Vector((0, 0, 1)).rotation_difference(ax)
            ln = 0.20 * (1.0 - 0.3 * t)
            mtx = (Matrix.Translation(c + d * 0.01 + ax * ln * 0.5)
                   @ q.to_matrix().to_4x4() @ Matrix.Rotation(a, 4, "Z")
                   @ Matrix.Diagonal((0.026, 0.007, ln * 0.5, 1.0)))
            bmesh.ops.create_uvsphere(bm, u_segments=8, v_segments=8, radius=1.0,
                                      matrix=mtx)
            z += 0.05
            k += 1
    return stems + [mesh_obj("arava_leaves", bm, leaf_mat)]


def mat_etrog():
    m, nt, bs = common._new_material("Etrog")
    tc = common._texcoord(nt)
    n = common._noise(nt, scale=3.0, detail=4.0)
    nt.links.new(tc.outputs["Object"], n.inputs["Vector"])
    ramp = common._ramp(nt, [(0.0, (0.55, 0.50, 0.02, 1.0)),
                             (0.35, (0.78, 0.58, 0.015, 1.0)),
                             (1.0, (0.80, 0.62, 0.02, 1.0))])
    nt.links.new(n.outputs["Fac"], ramp.inputs["Fac"])
    nt.links.new(ramp.outputs["Color"], bs.inputs["Base Color"])
    v = nt.nodes.new("ShaderNodeTexVoronoi")
    v.inputs["Scale"].default_value = 28.0
    v.feature = "SMOOTH_F1"
    nt.links.new(tc.outputs["Object"], v.inputs["Vector"])
    common._bump(nt, bs, v.outputs["Distance"], strength=0.45, distance=0.02)
    common._inp(bs, "Roughness", 0.40)
    common._inp(bs, "Coat Weight", 0.2)
    common._inp(bs, "Subsurface Weight", 0.06)
    return m


def etrog(skin, pitam_mat, loc):
    ob = common.add_uv_sphere("etrog", 1.0, mat=skin, segments=64, rings=32)
    for vtx in ob.data.vertices:
        d = vtx.co.normalized()
        z = d.z
        # lemon shape: bulging middle, pointed at the top (pitam end)
        r = 0.20 * (1.0 + 0.04 * noise.noise(d * 3.0))
        r *= 1.0 - 0.28 * max(0.0, z) ** 3
        vtx.co = Vector((d.x * r, d.y * r, d.z * 0.29 * (1.0 + 0.08 * max(0, z))))
    pitam = common.lathe("pitam", [(0.0, 0.0), (0.018, 0.0), (0.012, 0.03),
                                   (0.009, 0.055), (0.016, 0.07), (0.0, 0.08)],
                         segments=16, mat=pitam_mat)
    pitam.location = (0, 0, 0.295)
    stem = common.add_cylinder("etrog_stem", 0.012, 0.04, location=(0, 0, -0.30),
                               mat=pitam_mat, vertices=12)
    grp = [ob, pitam, stem]
    m = (Matrix.Translation(loc) @ Matrix.Rotation(math.radians(-62), 4, "Y")
         @ Matrix.Rotation(math.radians(20), 4, "X"))
    for g in grp:
        g.matrix_world = m @ g.matrix_world
    return grp


def etrog_box(silver, lining, c):
    bw, bd, bh = 0.86, 0.62, 0.22
    t = 0.035
    objs = []
    x, y = c
    objs.append(foodkit.rounded_box("box_floor", (bw, bd, 0.05), (x, y, 0.025),
                                    silver, bevel_w=0.02))
    for dx, dy, sx, sy in ((0, -bd / 2 + t / 2, bw, t), (0, bd / 2 - t / 2, bw, t),
                           (-bw / 2 + t / 2, 0, t, bd), (bw / 2 - t / 2, 0, t, bd)):
        objs.append(foodkit.rounded_box("box_wall", (sx, sy, bh), (x + dx, y + dy, bh / 2),
                                        silver, bevel_w=0.012))
    # rim bead
    for dx, dy, sx, sy in ((0, -bd / 2, bw + 0.02, 0.03), (0, bd / 2, bw + 0.02, 0.03),
                           (-bw / 2, 0, 0.03, bd + 0.02), (bw / 2, 0, 0.03, bd + 0.02)):
        objs.append(foodkit.rounded_box("box_rim", (sx, sy, 0.03),
                                        (x + dx, y + dy, bh + 0.01), silver, bevel_w=0.012))
    objs.append(foodkit.rounded_box("lining", (bw - 2 * t, bd - 2 * t, 0.06),
                                    (x, y, 0.08), lining, bevel_w=0.025))
    # open lid hinged at the back, leaning back ~110 deg
    lid = foodkit.rounded_box("lid", (bw + 0.02, bd + 0.02, 0.05), (0, -bd / 2 - 0.01, 0.0),
                              silver, bevel_w=0.02)
    lid_in = foodkit.rounded_box("lid_lining", (bw - 0.08, bd - 0.08, 0.02),
                                 (0, -bd / 2 - 0.01, -0.03), lining, bevel_w=0.008)
    knob = common.add_uv_sphere("lid_knob", 0.04, location=(0, -bd / 2 - 0.01, 0.05),
                                mat=silver, segments=16, rings=8)
    hinge = Matrix.Translation((x, y + bd / 2, bh + 0.02)) @ Matrix.Rotation(
        math.radians(-105), 4, "X")
    for g in (lid, lid_in, knob):
        g.matrix_world = hinge @ g.matrix_world
    objs += [lid, lid_in, knob]
    return objs


def build():
    common.reset_scene()
    common.setup_render()
    palm = mat_palm()
    stalk = foodkit.prin("PalmStalk", (0.22, 0.24, 0.08, 1.0), rough=0.45)
    holder_mat = foodkit.prin("HolderWeave", (0.12, 0.22, 0.05, 1.0), rough=0.6,
                              ramp=[(0.0, (0.08, 0.16, 0.03, 1.0)),
                                    (1.0, (0.20, 0.28, 0.08, 1.0))],
                              ramp_scale=60.0, bump=0.6, bump_scale=90.0)
    myrtle = foodkit.prin("Myrtle", (0.025, 0.13, 0.03, 1.0), rough=0.3, coat=0.35)
    willow = foodkit.prin("Willow", (0.12, 0.25, 0.08, 1.0), rough=0.45)
    willow_stem = foodkit.prin("WillowStem", (0.25, 0.06, 0.04, 1.0), rough=0.5)
    myrtle_stem = foodkit.prin("MyrtleStem", (0.07, 0.10, 0.03, 1.0), rough=0.5)
    silver = common._metal("EtrogSilver", (0.56, 0.57, 0.60, 1.0), 0.2,
                           noise_scale=45.0, bump_strength=0.04, bump_distance=0.01)
    lining = common.mat_velvet((0.012, 0.018, 0.12, 1.0))
    pitam_mat = foodkit.prin("Pitam", (0.18, 0.10, 0.04, 1.0), rough=0.7)

    lul = lulav(palm, stalk) + holder(holder_mat) + hadassim(myrtle, myrtle_stem) \
        + aravot(willow, willow_stem)
    # stand the bundle up, leaning slightly right and back
    bpy.context.view_layer.update()
    lay = (Matrix.Translation((-0.42, 0.20, 0.0)) @ Matrix.Rotation(math.radians(9), 4, "Y")
           @ Matrix.Rotation(math.radians(8), 4, "X") @ Matrix.Rotation(math.radians(-25), 4, "Z"))
    for ob in lul:
        ob.matrix_world = lay @ ob.matrix_world
    bpy.context.view_layer.update()
    mn, _ = common._world_bbox(lul)
    for ob in lul:
        ob.location.z -= mn.z
    objs = lul
    box_c = (0.48, -0.32)
    objs += etrog_box(silver, lining, box_c)
    objs += etrog(mat_etrog(), pitam_mat, (box_c[0], box_c[1] + 0.02, 0.32))

    common.frame_object(objs, margin=0.05, elevation=20.0, azimuth=24.0, focal=75.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render("lulav_etrog", common.OUT_DIR)


build()
