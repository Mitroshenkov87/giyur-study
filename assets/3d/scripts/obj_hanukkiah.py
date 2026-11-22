"""Hanukkiah: nine-branched Hanukkah menorah. 8 candles in a row + shamash
raised and set in front of the centre. Brass, white wax candles, warm flames."""

import math
import os
import sys

import bpy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common

ROW_Z = 0.66          # cup rim height of the 8 regular lights
SHAMASH_Z = 1.10      # cup rim height of the shamash
CANDLE_H = 0.34
SHAMASH_CANDLE_H = 0.40


def flame_core_mat():
    core = common.mat_flame((1.0, 0.88, 0.50, 1.0))
    for node in core.node_tree.nodes:
        if node.type == "BSDF_PRINCIPLED":
            node.inputs["Emission Strength"].default_value = 14.0
    return core


def candle(objs, x, y, cup_top, height, brass, wax, wick_mat, flame_mat, core_mat):
    # cup: conical holder + rim ring
    objs.append(common.add_cone("cup", 0.105, 0.145, 0.10,
                                location=(x, y, cup_top - 0.05), mat=brass))
    objs.append(common.add_torus("cup_rim", 0.135, 0.014,
                                 location=(x, y, cup_top), mat=brass))
    # white candle, slight taper toward the top
    top = cup_top + height
    objs.append(common.add_cone("candle", 0.058, 0.052, height,
                                location=(x, y, cup_top + height * 0.5), mat=wax))
    # wick + flame + bright core
    objs.append(common.add_cylinder("wick", 0.008, 0.05,
                                    location=(x, y, top + 0.02), vertices=16,
                                    mat=wick_mat))
    objs.append(common.add_uv_sphere("flame", 0.048, location=(x, y, top + 0.085),
                                     scale=(1.0, 0.82, 1.75), mat=flame_mat,
                                     segments=32, rings=16))
    objs.append(common.add_uv_sphere("flame_core", 0.020,
                                     location=(x, y, top + 0.055),
                                     scale=(1.0, 0.85, 1.5), mat=core_mat,
                                     segments=24, rings=12))


def build():
    common.reset_scene()
    common.setup_render()
    brass = common.mat_brass()
    wax = common.mat_wax()
    flame_mat = common.mat_flame()
    core_mat = flame_core_mat()
    wick_mat = common.mat_leather((0.02, 0.015, 0.01, 1.0))

    objs = []

    # two-tier footed base plate
    base1 = common.add_cylinder("base_low", 0.52, 0.07,
                                location=(0, 0, 0.035),
                                vertices=64, mat=brass)
    base1.scale = (3.4, 1.0, 1.0)
    common.bevel(base1, 0.025, segments=3)
    base2 = common.add_cylinder("base_high", 0.44, 0.07,
                                location=(0, 0, 0.10),
                                vertices=64, mat=brass)
    base2.scale = (3.6, 0.82, 1.0)
    common.bevel(base2, 0.025, segments=3)
    objs += [base1, base2]

    # connecting rail under the 8 cups
    rail = common.add_cylinder("rail", 0.030, 3.05, location=(0, 0, 0.50),
                               vertices=24, mat=brass)
    rail.rotation_euler = (0.0, math.radians(90.0), 0.0)
    objs.append(rail)

    # 8 regular lights in a straight row, equal height
    for i in range(8):
        x = -1.40 + i * 0.40
        objs.append(common.add_cylinder("post", 0.035, 0.44,
                                        location=(x, 0, 0.33),
                                        vertices=20, mat=brass))
        candle(objs, x, 0.0, ROW_Z, CANDLE_H,
               brass, wax, wick_mat, flame_mat, core_mat)

    # shamash: taller stem set in front of the centre
    sy = -0.52
    objs.append(common.add_cylinder("shamash_stem", 0.040, 0.96,
                                    location=(0, sy, 0.55), vertices=24, mat=brass))
    objs.append(common.add_uv_sphere("shamash_knop", 0.075, location=(0, sy, 0.62),
                                     scale=(1.0, 1.0, 0.8), mat=brass))
    objs.append(common.add_torus("shamash_ring", 0.052, 0.014,
                                 location=(0, sy, 1.00), mat=brass))
    candle(objs, 0, sy, SHAMASH_Z, SHAMASH_CANDLE_H,
           brass, wax, wick_mat, flame_mat, core_mat)

    common.frame_object(objs, margin=0.12, elevation=21.0, azimuth=30.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render("hanukkiah", common.OUT_DIR)


build()
