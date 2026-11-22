"""Shabbat candles: a pair of lit white candles in silver candlesticks."""

import math
import os
import sys

import bpy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common

# Candlestick profiles bottom-to-top (lathe of revolution), solid tops.
STICK_A = [  # taller, slimmer
    (0.0, 0.00), (0.26, 0.00), (0.28, 0.02), (0.27, 0.045), (0.20, 0.07),
    (0.09, 0.095), (0.06, 0.14), (0.055, 0.34), (0.09, 0.40), (0.055, 0.46),
    (0.05, 0.62), (0.10, 0.68), (0.13, 0.73), (0.15, 0.77), (0.14, 0.80),
    (0.09, 0.82), (0.0, 0.82),
]
STICK_B = [  # shorter, wider foot
    (0.0, 0.00), (0.30, 0.00), (0.32, 0.02), (0.31, 0.05), (0.22, 0.08),
    (0.10, 0.11), (0.07, 0.17), (0.065, 0.30), (0.11, 0.36), (0.07, 0.42),
    (0.06, 0.52), (0.11, 0.58), (0.14, 0.63), (0.16, 0.67), (0.15, 0.70),
    (0.10, 0.72), (0.0, 0.72),
]


def flame_core_mat():
    core = common.mat_flame((1.0, 0.88, 0.50, 1.0))
    for node in core.node_tree.nodes:
        if node.type == "BSDF_PRINCIPLED":
            node.inputs["Emission Strength"].default_value = 14.0
    return core


def candle(objs, x, y, z0, height, wax, wick_mat, flame_mat, core_mat):
    top = z0 + height
    # tapered white candle with a rounded lip and two soft wax drips
    objs.append(common.add_cone("candle", 0.092, 0.080, height,
                                location=(x, y, z0 + height * 0.5), mat=wax))
    objs.append(common.add_torus("candle_lip", 0.082, 0.013,
                                 location=(x, y, top), mat=wax))
    objs.append(common.add_torus("drip_low", 0.090, 0.018,
                                 location=(x, y, top - 0.30), mat=wax))
    objs.append(common.add_torus("drip_high", 0.086, 0.014,
                                 location=(x, y, top - 0.13), mat=wax))
    objs.append(common.add_cylinder("wick", 0.009, 0.05,
                                    location=(x, y, top + 0.02), vertices=16,
                                    mat=wick_mat))
    objs.append(common.add_uv_sphere("flame", 0.055, location=(x, y, top + 0.095),
                                     scale=(1.0, 0.85, 1.8), mat=flame_mat,
                                     segments=32, rings=16))
    objs.append(common.add_uv_sphere("flame_core", 0.023,
                                     location=(x, y, top + 0.06),
                                     scale=(1.0, 0.85, 1.5), mat=core_mat,
                                     segments=24, rings=12))


def build():
    common.reset_scene()
    common.setup_render()
    silver = common.mat_silver()
    wax = common.mat_wax()
    flame_mat = common.mat_flame()
    core_mat = flame_core_mat()
    wick_mat = common.mat_leather((0.02, 0.015, 0.01, 1.0))

    objs = []
    for profile, x, y, candle_h, ring_r, ring_z in (
        (STICK_A, -0.52, 0.06, 1.05, 0.205, 0.068),
        (STICK_B, 0.55, -0.06, 0.78, 0.245, 0.075),
    ):
        stick = common.lathe("candlestick", profile, segments=72, mat=silver)
        common.bevel(stick, 0.008, segments=2)
        stick.location = (x, y, 0.0)
        objs.append(stick)
        # engraved ring where the stem meets the foot
        objs.append(common.add_torus("foot_ring", ring_r,
                                     0.010, location=(x, y, ring_z), mat=silver))
        candle(objs, x, y, profile[-1][1] - 0.005, candle_h,
               wax, wick_mat, flame_mat, core_mat)

    common.frame_object(objs, margin=0.12, elevation=22.0, azimuth=34.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render("shabbat_candles", common.OUT_DIR)


build()
