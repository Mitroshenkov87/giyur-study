"""Havdalah candle: multi-wick candle of twisted blue and white strands,
lit, standing in a small brass holder."""

import math
import os
import sys

import bpy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common

CANDLE_Z0 = 0.26   # braid start (embedded in holder cup)
CANDLE_H = 1.35
HELIX_R = 0.085
STRAND_R = 0.078
TURNS = 9.0
BLUE = (0.13, 0.22, 0.52, 1.0)
WHITE = (0.93, 0.90, 0.80, 1.0)


def flame_core_mat():
    core = common.mat_flame((1.0, 0.88, 0.50, 1.0))
    for node in core.node_tree.nodes:
        if node.type == "BSDF_PRINCIPLED":
            node.inputs["Emission Strength"].default_value = 14.0
    return core


def build():
    common.reset_scene()
    common.setup_render()
    brass = common.mat_brass()
    wax_white = common.mat_wax(WHITE)
    wax_blue = common.mat_wax(BLUE)
    flame_mat = common.mat_flame()
    core_mat = flame_core_mat()
    wick_mat = common.mat_leather((0.02, 0.015, 0.01, 1.0))

    objs = []

    # brass holder: shallow dish with a raised cup in the middle
    dish_prof = [
        (0.0, 0.00), (0.30, 0.00), (0.44, 0.02), (0.47, 0.05),
        (0.46, 0.08), (0.40, 0.10), (0.34, 0.10), (0.10, 0.055),
        (0.0, 0.05),
    ]
    dish = common.lathe("holder_dish", dish_prof, segments=72, mat=brass)
    common.bevel(dish, 0.010, segments=2)
    objs.append(dish)
    cup = common.add_cone("holder_cup", 0.22, 0.17, 0.30,
                          location=(0, 0, 0.22), mat=brass)
    common.bevel(cup, 0.015, segments=2)
    objs.append(cup)

    # 4 twisted strands, alternating white / blue
    n_pts = 72
    for s in range(4):
        phase = s * math.pi / 2.0
        pts = []
        for i in range(n_pts + 1):
            t = i / n_pts
            a = 2.0 * math.pi * TURNS * t + phase
            pts.append((HELIX_R * math.cos(a),
                        HELIX_R * math.sin(a),
                        CANDLE_Z0 + CANDLE_H * t))
        strand = common.tube_curve("strand", pts, STRAND_R,
                                   mat=wax_white if s % 2 == 0 else wax_blue,
                                   resolution=4)
        objs.append(strand)

    # wax caps where the strands gather at both ends
    objs.append(common.add_uv_sphere("cap_bottom", 0.135, location=(0, 0, CANDLE_Z0),
                                     scale=(1.0, 1.0, 0.6), mat=wax_white))
    cap_top_z = CANDLE_Z0 + CANDLE_H
    objs.append(common.add_uv_sphere("cap_top", 0.130, location=(0, 0, cap_top_z),
                                     scale=(1.0, 1.0, 0.55), mat=wax_white))

    # 4 wicks close together, 4 small flames + one larger central flame
    for k in range(4):
        a = k * math.pi / 2.0 + math.pi / 4.0
        wx, wy = 0.045 * math.cos(a), 0.045 * math.sin(a)
        objs.append(common.add_cylinder("wick", 0.008, 0.08,
                                        location=(wx, wy, cap_top_z + 0.05),
                                        vertices=12, mat=wick_mat))
        objs.append(common.add_uv_sphere("small_flame", 0.030,
                                         location=(wx, wy, cap_top_z + 0.115),
                                         scale=(1.0, 0.85, 1.7), mat=flame_mat,
                                         segments=24, rings=12))
    objs.append(common.add_uv_sphere("main_flame", 0.060,
                                     location=(0, 0, cap_top_z + 0.16),
                                     scale=(1.0, 0.85, 1.9), mat=core_mat,
                                     segments=32, rings=16))

    common.frame_object(objs, margin=0.12, elevation=20.0, azimuth=32.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render("havdalah_candle", common.OUT_DIR)


build()
