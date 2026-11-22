"""Open Sefer Torah lying on a bimah: two thick parchment rolls on turned
wooden atzei chaim with round wooden discs, the warm cream sheet spread
between them showing four columns of fine text rows."""

import math
import os
import random
import sys

import bpy
from mathutils import Matrix

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import objkit

RR = 0.25            # parchment roll radius
XR = 1.02            # roll centre |x|
HD = 0.64            # sheet half-height (along the rollers, Y)
SHEET_Z = 2 * RR - 0.02
DISC_R = 0.36

# etz chaim profile along its axis (radius, z); z=0 is the bottom handle tip.
B = 0.46             # bottom handle length
ROLL_L = 2 * HD + 0.04
ETZ = [
    (0.0, 0.0), (0.05, 0.0), (0.075, 0.03), (0.07, 0.08), (0.05, 0.11),
    (0.065, 0.16), (0.085, 0.22), (0.065, 0.28), (0.045, 0.31),
    (0.05, 0.36), (0.085, 0.40), (0.085, 0.42),
    (DISC_R - 0.02, 0.425), (DISC_R, 0.44), (DISC_R, 0.465),
    (DISC_R - 0.02, 0.48), (0.08, B),
    (0.08, B + ROLL_L),
    (DISC_R - 0.02, B + ROLL_L + 0.02 - 0.005), (DISC_R, B + ROLL_L + 0.03),
    (DISC_R, B + ROLL_L + 0.055), (DISC_R - 0.02, B + ROLL_L + 0.07),
    (0.075, B + ROLL_L + 0.075), (0.06, B + ROLL_L + 0.11),
    (0.08, B + ROLL_L + 0.17), (0.055, B + ROLL_L + 0.23),
    (0.07, B + ROLL_L + 0.27), (0.04, B + ROLL_L + 0.30), (0.0, B + ROLL_L + 0.31),
]


def mat_klaf():
    m, nt, bs = common._new_material("Klaf")
    tc = common._texcoord(nt)
    n = common._noise(nt, scale=3.0, detail=5.0, roughness=0.6)
    nt.links.new(tc.outputs["Object"], n.inputs["Vector"])
    ramp = common._ramp(nt, [(0.0, (0.50, 0.38, 0.22, 1.0)),
                             (0.5, (0.62, 0.50, 0.32, 1.0)),
                             (1.0, (0.68, 0.57, 0.38, 1.0))])
    nt.links.new(n.outputs["Fac"], ramp.inputs["Fac"])
    nt.links.new(ramp.outputs["Color"], bs.inputs["Base Color"])
    f = common._noise(nt, scale=70.0, detail=2.0, y=-320)
    nt.links.new(tc.outputs["Object"], f.inputs["Vector"])
    common._bump(nt, bs, f.outputs["Fac"], strength=0.08, distance=0.01)
    common._inp(bs, "Roughness", 0.65)
    return m


def mat_roll_edge():
    """Parchment roll: cream with faint winding lines around the axis."""
    m, nt, bs = common._new_material("KlafRoll")
    tc = common._texcoord(nt)
    w = nt.nodes.new("ShaderNodeTexWave")
    w.wave_type = "RINGS"
    w.rings_direction = "Z"
    w.inputs["Scale"].default_value = 18.0
    w.inputs["Distortion"].default_value = 2.0
    nt.links.new(tc.outputs["Object"], w.inputs["Vector"])
    ramp = common._ramp(nt, [(0.0, (0.46, 0.35, 0.20, 1.0)),
                             (1.0, (0.64, 0.52, 0.34, 1.0))])
    nt.links.new(w.outputs["Fac"], ramp.inputs["Fac"])
    nt.links.new(ramp.outputs["Color"], bs.inputs["Base Color"])
    common._bump(nt, bs, w.outputs["Fac"], strength=0.1, distance=0.004)
    common._inp(bs, "Roughness", 0.65)
    return m


def surface_z(x):
    """Sheet: tangent to the roll tops, a slight sag in the middle."""
    return SHEET_Z - 0.06 * (1.0 - (x / XR) ** 2)


def sheet(mat):
    nu, nv, thick = 80, 8, 0.008
    verts, faces = [], []
    xs = [-XR + 2 * XR * i / nu for i in range(nu + 1)]
    ys = [-HD + 2 * HD * j / nv for j in range(nv + 1)]
    for zoff in (0.0, -thick):
        for x in xs:
            for y in ys:
                verts.append((x, y, surface_z(x) + zoff))
    off = (nu + 1) * (nv + 1)
    W = nv + 1
    for i in range(nu):
        for j in range(nv):
            a, b = i * W + j, (i + 1) * W + j
            faces.append((a, b, b + 1, a + 1))
            faces.append((off + a + 1, off + b + 1, off + b, off + a))
    for i in range(nu):
        a, b = i * W, (i + 1) * W
        faces.append((a, off + a, off + b, b))
        a, b = i * W + nv, (i + 1) * W + nv
        faces.append((a, b, off + b, off + a))
    mesh = bpy.data.meshes.new("sheet_mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    ob = bpy.data.objects.new("sheet", mesh)
    bpy.context.collection.objects.link(ob)
    common.assign(ob, mat)
    common.smooth(ob)
    return ob


def text_rows(mat):
    """Four columns of fine 'Hebrew' rows, right-aligned with ragged left."""
    rng = random.Random(11)
    boxes = []
    col_w, gap = 0.36, 0.13
    span = 4 * col_w + 3 * gap
    x_start = -span / 2
    e = 0.004
    for c in range(4):
        x0 = x_start + c * (col_w + gap)
        x1 = x0 + col_w
        for row in range(31):
            y = HD - 0.10 - row * 0.0385
            pos = x1
            while pos - x0 > 0.02:
                w = rng.uniform(0.022, min(0.075, pos - x0))
                xc = pos - w / 2
                sl = (surface_z(xc + e) - surface_z(xc - e)) / (2 * e)
                boxes.append((xc, y, surface_z(xc) + 0.002, w - 0.008, 0.015,
                              0.003, -math.atan(sl)))
                pos -= w + rng.uniform(0.004, 0.009)
    return objkit.add_boxes("text_rows", boxes, mat)


def build():
    common.reset_scene()
    common.setup_render()
    wood = objkit.mat_dark_walnut("EtzWood", axis="Z")
    klaf = mat_klaf()
    roll_mat = mat_roll_edge()
    ink = objkit.mat_ink((0.035, 0.02, 0.012, 1.0))
    objs = [sheet(klaf), text_rows(ink)]

    for sgn in (-1, 1):
        etz = common.lathe("etz_chaim", ETZ, segments=64, mat=wood)
        # axis +Z -> +Y, bottom handle toward the viewer (-Y)
        etz.rotation_euler = (math.radians(-90.0), 0.0, 0.0)
        etz.location = (sgn * XR, -(B + ROLL_L / 2), RR + 0.0)
        objs.append(etz)
        roll = common.add_cylinder("klaf_roll", RR, ROLL_L - 0.02,
                                   location=(sgn * XR, 0.0, RR), vertices=64,
                                   mat=roll_mat)
        roll.rotation_euler = (math.radians(90.0), 0.0, 0.0)
        common.bevel(roll, 0.01, 2)
        common.smooth(roll)
        objs.append(roll)

    turn = Matrix.Rotation(math.radians(8.0), 4, "Z")
    lift = Matrix.Translation((0, 0, DISC_R - RR))
    for ob in objs:
        ob.matrix_world = lift @ turn @ ob.matrix_world

    common.frame_object(objs, margin=0.06, elevation=36.0, azimuth=24.0,
                        focal=72.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render("torah_open", common.OUT_DIR)


build()
