"""Mezuzah: slim rounded brass case with a raised letter shin on the front
and small screw holes top and bottom, leaning back slightly as if mounted."""

import math
import os
import sys

import bpy
from mathutils import Matrix

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import objkit

FONT = "/workspace/shorashim-wt-art/assets/fonts/NotoSerifHebrew-Regular.ttf"

W, D, H = 0.46, 0.27, 1.92


def build():
    common.reset_scene()
    common.setup_render()
    brass = common.mat_brass()
    bronze = objkit.mat_dark_bronze()
    hole_mat = objkit.mat_ink((0.018, 0.011, 0.007, 1.0))
    objs = []

    # case body + removable back plate + slightly proud front cover
    case = objkit.cube("case", (W, D, H), (0, 0, H / 2.0), brass, bevel=0.05)
    back = objkit.cube("back_plate", (W - 0.06, 0.024, H - 0.07),
                       (0, D / 2.0 + 0.002, H / 2.0), bronze, bevel=0.02)
    front = objkit.cube("front_plate", (W - 0.07, 0.018, H - 0.12),
                        (0, -D / 2.0 - 0.002, H / 2.0), brass, bevel=0.02)
    objs += [case, back, front]

    # raised letter shin, centred a touch above middle
    cu = bpy.data.curves.new("shin_curve", "FONT")
    cu.body = "\u05E9"
    cu.font = bpy.data.fonts.load(FONT)
    cu.size = 1.0
    cu.extrude = 0.028
    cu.bevel_depth = 0.005
    cu.bevel_resolution = 3
    cu.align_x = "CENTER"
    cu.align_y = "CENTER"
    shin = bpy.data.objects.new("shin", cu)
    bpy.context.collection.objects.link(shin)
    shin.rotation_euler = (math.radians(90.0), 0.0, 0.0)
    bpy.context.view_layer.update()
    s = 0.98 / shin.dimensions.z
    shin.scale = (s, s, s)
    shin.location = (0.0, -D / 2.0 - 0.0115, H / 2.0 + 0.05)
    common.assign(shin, bronze)
    objs.append(shin)

    # screw holes top and bottom: dark inset + deeper core
    for z in (0.22, H - 0.22):
        ring = common.add_cylinder("screw", 0.030, 0.020, location=(0, -0.149, z),
                                   vertices=32, mat=bronze)
        ring.rotation_euler = (math.radians(90.0), 0.0, 0.0)
        core = common.add_cylinder("screw_hole", 0.016, 0.024,
                                   location=(0, -0.151, z), vertices=24, mat=hole_mat)
        core.rotation_euler = (math.radians(90.0), 0.0, 0.0)
        objs += [ring, core]

    # slight backward lean, as if tilted on a doorpost
    lean = Matrix.Rotation(math.radians(-10.0), 4, "X")
    for ob in objs:
        ob.matrix_world = lean @ ob.matrix_world

    common.frame_object(objs)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render("mezuzah", common.OUT_DIR)


build()
