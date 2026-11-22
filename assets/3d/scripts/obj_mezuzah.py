"""Mezuzah: slim (~1:6) rounded olive-wood case with brass end caps, a small
raised brass letter shin on the upper front and screw tabs top and bottom,
mounted slanted on a short walnut doorpost strip."""

import math
import os
import sys

import bpy
from mathutils import Matrix, Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import objkit

FONT = "/workspace/shorashim-wt-art/assets/fonts/NotoSerifHebrew-Regular.ttf"

W, D, H = 0.30, 0.17, 1.80      # case body (caps add a little height)
CAP_H = 0.13


def mat_olive_wood():
    """Olive wood: warm honey-tan with dark wavy streaks."""
    m, nt, bsdf = common._new_material("OliveWood")
    tc = common._texcoord(nt)
    wave = nt.nodes.new("ShaderNodeTexWave")
    wave.wave_type = "BANDS"
    wave.bands_direction = "X"
    wave.inputs["Scale"].default_value = 3.2
    wave.inputs["Distortion"].default_value = 11.0
    wave.inputs["Detail"].default_value = 5.0
    wave.inputs["Detail Scale"].default_value = 1.6
    mapn = nt.nodes.new("ShaderNodeMapping")
    mapn.inputs["Scale"].default_value = (1.0, 1.0, 0.35)
    nt.links.new(tc.outputs["Object"], mapn.inputs["Vector"])
    nt.links.new(mapn.outputs["Vector"], wave.inputs["Vector"])
    ramp = common._ramp(nt, [(0.0, (0.13, 0.065, 0.025, 1.0)),
                             (0.18, (0.30, 0.18, 0.08, 1.0)),
                             (0.45, (0.55, 0.40, 0.22, 1.0)),
                             (0.75, (0.62, 0.47, 0.27, 1.0)),
                             (1.0, (0.40, 0.26, 0.12, 1.0))])
    nt.links.new(wave.outputs["Fac"], ramp.inputs["Fac"])
    nt.links.new(ramp.outputs["Color"], bsdf.inputs["Base Color"])
    common._bump(nt, bsdf, wave.outputs["Fac"], strength=0.08, distance=0.02)
    common._inp(bsdf, "Roughness", 0.32)
    common._inp(bsdf, "Coat Weight", 0.35)
    common._inp(bsdf, "Coat Roughness", 0.15)
    return m


def screw(name, loc, mats):
    brass, hole = mats
    head = common.add_uv_sphere(name, 0.034, location=loc, scale=(1, 0.45, 1),
                                mat=brass, segments=24, rings=12)
    slot = objkit.cube(name + "_slot", (0.050, 0.012, 0.010),
                       (loc[0], loc[1] - 0.014, loc[2]), hole, rot=(0, 0.6, 0))
    return [head, slot]


def build():
    common.reset_scene()
    common.setup_render()
    brass = common.mat_brass()
    olive = mat_olive_wood()
    walnut = common.mat_walnut()
    hole = objkit.mat_ink((0.02, 0.012, 0.008, 1.0))
    case = []

    # main body: heavily rounded box, front slightly proud panel
    body = objkit.cube("case", (W, D, H), (0, 0, 0), olive, bevel=0.07)
    common.smooth(body)
    panel = objkit.cube("panel", (W - 0.10, 0.02, H - 0.30),
                        (0, -D / 2.0 - 0.002, 0.0), olive, bevel=0.009)
    case += [body, panel]

    # brass end caps + flat screw tabs
    for sgn in (1, -1):
        zc = sgn * (H / 2.0 + CAP_H / 2.0 - 0.03)
        cap = objkit.cube("cap", (W + 0.04, D + 0.03, CAP_H), (0, 0, zc),
                          brass, bevel=0.04)
        common.smooth(cap)
        band = common.add_cylinder("cap_bead", 0.012, W + 0.02,
                                   location=(0, -D / 2.0 - 0.012,
                                             zc - sgn * CAP_H * 0.42),
                                   vertices=16, mat=brass)
        band.rotation_euler = (0, math.radians(90), 0)
        zt = sgn * (H / 2.0 + CAP_H + 0.06)
        tab = objkit.cube("tab", (W * 0.62, 0.035, 0.20),
                          (0, D / 2.0 - 0.03, zt - sgn * 0.02), brass, bevel=0.03)
        common.smooth(tab)
        case += [cap, band, tab]
        case += screw("screw", (0, D / 2.0 - 0.05, zt + sgn * 0.005), (brass, hole))

    # small raised shin on the upper front
    cu = bpy.data.curves.new("shin_curve", "FONT")
    cu.body = "ש"
    cu.font = bpy.data.fonts.load(FONT)
    cu.size = 1.0
    cu.extrude = 0.012
    cu.bevel_depth = 0.004
    cu.bevel_resolution = 2
    cu.align_x = "CENTER"
    cu.align_y = "CENTER"
    shin = bpy.data.objects.new("shin", cu)
    bpy.context.collection.objects.link(shin)
    shin.rotation_euler = (math.radians(90.0), 0.0, 0.0)
    bpy.context.view_layer.update()
    s = 0.17 / max(shin.dimensions.x, shin.dimensions.z)
    shin.scale = (s, s, s)
    shin.location = (0.0, -D / 2.0 - 0.024, H / 2.0 - 0.32)
    common.assign(shin, brass)
    case.append(shin)
    plaque = objkit.cube("plaque", (W - 0.09, 0.012, 0.30),
                         (0, -D / 2.0 - 0.010, H / 2.0 - 0.32),
                         objkit.mat_dark_bronze((0.05, 0.03, 0.018, 1.0), 0.45),
                         bevel=0.04)
    case.append(plaque)
    # thin brass inlay line under the letter
    line = objkit.cube("inlay", (W - 0.14, 0.008, 0.014),
                       (0, -D / 2.0 - 0.012, H / 2.0 - 0.50), brass)
    case.append(line)

    # doorpost strip behind; case slanted (top toward the room), as mounted
    post = objkit.cube("doorpost", (0.54, 0.16, 2.75),
                       (0.05, D / 2.0 + 0.08, 0.10), walnut, bevel=0.025)
    trim = objkit.cube("doorpost_molding", (0.10, 0.20, 2.75),
                       (0.36, D / 2.0 + 0.06, 0.10), walnut, bevel=0.03)
    common.smooth(trim)
    slant = Matrix.Rotation(math.radians(-14.0), 4, "Y")
    for ob in case:
        ob.matrix_world = slant @ ob.matrix_world
    objs = case + [post, trim]
    lift = Matrix.Translation(Vector((0, 0, 1.48)))
    for ob in objs:
        ob.matrix_world = lift @ ob.matrix_world

    common.frame_object(objs, margin=0.08, elevation=14.0, azimuth=-24.0,
                        focal=80.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render("mezuzah", common.OUT_DIR)


build()
