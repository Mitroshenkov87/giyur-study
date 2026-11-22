"""Hanukkiah: eight brass cups in a row and a taller gold shamash set in
front. Cups are turned (knop, bowl, flared rim); the platter stands on feet.
Wax is ivory, not blown-out white."""

import math
import os
import sys

import bpy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common

ROW_RIM = 0.74
SHAMASH_RIM = 1.32
CANDLE_H = 0.32
SHAMASH_CANDLE_H = 0.30
WAX = (0.74, 0.66, 0.50, 1.0)


def flame_core_mat():
    core = common.mat_flame((1.0, 0.86, 0.48, 1.0))
    for node in core.node_tree.nodes:
        if node.type == "BSDF_PRINCIPLED":
            node.inputs["Emission Strength"].default_value = 14.0
    return core


def cup_profile(s):
    """Turned oil cup: neck, knop, rounded bowl, flared lip. Rim at local z=0."""
    return [
        (0.000, -0.155 * s),
        (0.042 * s, -0.150 * s),
        (0.058 * s, -0.132 * s),
        (0.034 * s, -0.114 * s),
        (0.080 * s, -0.096 * s),
        (0.082 * s, -0.078 * s),
        (0.036 * s, -0.062 * s),
        (0.052 * s, -0.046 * s),
        (0.098 * s, -0.026 * s),
        (0.128 * s, -0.010 * s),
        (0.162 * s, 0.002 * s),
        (0.150 * s, 0.016 * s),
        (0.128 * s, 0.010 * s),
        (0.000, 0.008 * s),
    ]


def add_cup(objs, x, y, rim_z, mat, scale=1.0):
    cup = common.lathe("cup", cup_profile(scale), segments=40, mat=mat)
    cup.location = (x, y, rim_z)
    objs.append(cup)
    objs.append(common.add_torus(
        "cup_lip", 0.150 * scale, 0.012 * scale,
        location=(x, y, rim_z + 0.004 * scale), mat=mat,
        major_segments=40, minor_segments=10))


def add_candle(objs, x, y, rim_z, height, wax, wick_mat, flame_mat, core_mat, scale=1.0):
    top = rim_z + height
    objs.append(common.add_cone(
        "candle", 0.055 * scale, 0.046 * scale, height,
        location=(x, y, rim_z + height * 0.5 - 0.01), vertices=24, mat=wax))
    objs.append(common.add_cylinder(
        "wick", 0.007, 0.045, location=(x, y, top + 0.012),
        vertices=12, mat=wick_mat))
    objs.append(common.add_uv_sphere(
        "flame", 0.046 * scale, location=(x, y, top + 0.075),
        scale=(1.0, 0.82, 1.7), mat=flame_mat, segments=24, rings=12))
    objs.append(common.add_uv_sphere(
        "flame_core", 0.018 * scale, location=(x, y, top + 0.048),
        scale=(1.0, 0.85, 1.45), mat=core_mat, segments=16, rings=10))


def add_foot(objs, x, y, mat):
    objs.append(common.add_cylinder(
        "foot_pad", 0.070, 0.022, location=(x, y, 0.011), vertices=24, mat=mat))
    objs.append(common.add_uv_sphere(
        "foot_ball", 0.052, location=(x, y, 0.046),
        scale=(1.0, 1.0, 0.72), mat=mat, segments=20, rings=12))
    objs.append(common.add_cylinder(
        "foot_neck", 0.026, 0.045, location=(x, y, 0.078), vertices=16, mat=mat))


def build():
    common.reset_scene()
    common.setup_render()
    brass = common._metal("HanukBrass", (0.42, 0.24, 0.07, 1.0), 0.34,
                          noise_scale=8.0, bump_strength=0.04, bump_distance=0.01)
    gold = common._metal("ShamashGold", (0.78, 0.52, 0.14, 1.0), 0.20)
    wax = common.mat_wax(WAX)
    flame_mat = common.mat_flame((1.0, 0.28, 0.03, 1.0))
    for node in flame_mat.node_tree.nodes:
        if node.type == "BSDF_PRINCIPLED" and "Emission Strength" in node.inputs:
            node.inputs["Emission Strength"].default_value = 4.0
    core_mat = flame_core_mat()
    wick_mat = common.mat_leather((0.02, 0.015, 0.01, 1.0))
    objs = []

    # Feet first, two of them proud of the front edge so they read in 3/4 view.
    for x, y in ((-1.42, -0.18), (-1.42, 0.20), (1.42, -0.18), (1.42, 0.20),
                 (-0.55, -0.56), (0.55, -0.56)):
        add_foot(objs, x, y, brass)

    base1 = common.add_cylinder("base_low", 0.50, 0.07,
                                location=(0, 0, 0.125), vertices=64, mat=brass)
    base1.scale = (3.30, 0.98, 1.0)
    common.bevel(base1, 0.02, segments=2)
    base2 = common.add_cylinder("base_high", 0.42, 0.055,
                                location=(0, 0, 0.180), vertices=64, mat=brass)
    base2.scale = (3.15, 0.78, 1.0)
    common.bevel(base2, 0.016, segments=2)
    objs += [base1, base2]

    rail = common.add_cylinder("rail", 0.028, 2.95, location=(0, 0, 0.46),
                               vertices=20, mat=brass)
    rail.rotation_euler = (0.0, math.radians(90.0), 0.0)
    objs.append(rail)

    for i in range(8):
        x = -1.40 + i * 0.40
        objs.append(common.add_cylinder(
            "post", 0.032, 0.36, location=(x, 0.0, 0.40), vertices=18, mat=brass))
        objs.append(common.add_uv_sphere(
            "post_knop", 0.055, location=(x, 0.0, 0.50),
            scale=(1.0, 1.0, 0.72), mat=brass, segments=20, rings=10))
        objs.append(common.add_torus(
            "post_ring", 0.048, 0.010, location=(x, 0.0, 0.58), mat=brass,
            major_segments=28, minor_segments=8))
        add_cup(objs, x, 0.0, ROW_RIM, brass, scale=1.0)
        add_candle(objs, x, 0.0, ROW_RIM, CANDLE_H, wax, wick_mat, flame_mat, core_mat)

    # Shamash: gold, forward of the row, higher cup, so it cannot be missed.
    sy = -0.62
    objs.append(common.add_cylinder(
        "shamash_stem", 0.042, 0.92, location=(0.0, sy, 0.62), vertices=24, mat=gold))
    for z, r in ((0.36, 0.070), (0.62, 0.078), (0.90, 0.064)):
        objs.append(common.add_uv_sphere(
            "shamash_knop", r, location=(0.0, sy, z),
            scale=(1.0, 1.0, 0.70), mat=gold, segments=24, rings=12))
    objs.append(common.add_torus(
        "shamash_ring", 0.070, 0.014, location=(0.0, sy, 1.00), mat=gold))
    # Small curved bracket tying the shamash back to the rail.
    objs.append(common.tube_curve(
        "shamash_arm",
        [(0.0, -0.02, 0.46), (0.0, -0.24, 0.50), (0.0, sy + 0.02, 0.62)],
        0.022, mat=gold, resolution=3))
    add_cup(objs, 0.0, sy, SHAMASH_RIM, gold, scale=1.40)
    add_candle(objs, 0.0, sy, SHAMASH_RIM, SHAMASH_CANDLE_H,
               wax, wick_mat, flame_mat, core_mat, scale=1.12)

    common.frame_object(objs, margin=0.11, focal=70.0, elevation=20.0, azimuth=28.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render("hanukkiah", common.OUT_DIR)


build()
