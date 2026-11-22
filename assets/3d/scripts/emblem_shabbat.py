"""Emblem: two shabbat candles with lit flames."""

import os
import sys

import bpy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import emblemkit

CANDLE_X = 0.32


def _flame(name, mat, location):
    """Teardrop, Z-up, sitting on `location`. Wide enough to read at 64 px."""
    profile = [(0.0, 0.0), (0.045, 0.02), (0.078, 0.10),
               (0.055, 0.20), (0.022, 0.32), (0.0, 0.42)]
    ob = common.lathe(name, profile, segments=24, mat=mat)
    ob.location = location
    return ob


def candle(objs, x, wax, metal, flame, core, ry):
    base = common.add_cylinder(
        "stick_base", 0.175, 0.06, location=(x, ry, -0.50),
        vertices=40, mat=metal)
    common.bevel(base, 0.014, segments=2)
    objs.append(base)
    foot = common.add_uv_sphere(
        "stick_foot", 0.08, location=(x, ry, -0.44),
        scale=(1.15, 0.7, 0.55), mat=metal, segments=28, rings=14)
    objs.append(foot)
    stem = common.add_cylinder(
        "stick_stem", 0.042, 0.40, location=(x, ry, -0.22),
        vertices=24, mat=metal)
    common.smooth(stem)
    objs.append(stem)
    objs.append(common.add_uv_sphere(
        "stick_node", 0.064, location=(x, ry, 0.00),
        mat=metal, segments=24, rings=12))
    cup = common.add_cone(
        "stick_cup", 0.11, 0.068, 0.12, location=(x, ry, 0.09),
        vertices=32, mat=metal)
    common.smooth(cup)
    objs.append(cup)
    body = common.add_cylinder(
        "candle", 0.068, 0.32, location=(x, ry, 0.28),
        vertices=32, mat=wax)
    common.bevel(body, 0.008, segments=2)
    common.smooth(body)
    objs.append(body)
    objs.append(common.add_cylinder(
        "wick", 0.012, 0.05, location=(x, ry, 0.46),
        vertices=10, mat=metal))
    objs.append(_flame("flame", flame, (x, ry - 0.004, 0.44)))
    core_ob = _flame("flame_core", core, (x, ry - 0.004, 0.48))
    core_ob.scale = (0.42, 0.42, 0.50)
    objs.append(core_ob)


def build():
    common.reset_scene()
    common.setup_render()
    objs, metal, accent = emblemkit.medallion()
    ry = emblemkit.R_Y
    wax = common.mat_wax(color=(0.62, 0.52, 0.36, 1.0))
    flame = emblemkit.flame_mat(2.6, (1.0, 0.32, 0.03, 1.0))
    core = emblemkit.flame_mat(4.2, (1.0, 0.55, 0.12, 1.0))

    for x in (-CANDLE_X, CANDLE_X):
        candle(objs, x, wax, metal, flame, core, ry)

    emblemkit.finish(objs, "shabbat")


build()
