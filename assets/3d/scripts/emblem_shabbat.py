"""Emblem: two shabbat candles with lit flames."""

import os
import sys

import bpy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import emblemkit

CANDLE_X = 0.27
BASE_Z = -0.46
TOP_Z = 0.345


def candle(objs, x, wax, metal, ry):
    objs.append(common.add_uv_sphere(
        "candle_base", 0.13, location=(x, ry, BASE_Z),
        scale=(1.0, 0.6, 1.0), mat=metal, segments=40, rings=20))
    body = common.add_cone("candle", 0.092, 0.074, TOP_Z - BASE_Z - 0.06,
                           location=(x, ry, (BASE_Z + TOP_Z) * 0.5 - 0.01),
                           vertices=48, mat=wax)
    common.bevel(body, 0.012, segments=3)
    common.smooth(body)
    objs.append(body)
    objs.append(common.add_cylinder(
        "wick", 0.013, 0.06, location=(x, ry, TOP_Z + 0.02), vertices=12,
        mat=metal))


def build():
    common.reset_scene()
    common.setup_render()
    objs, metal, accent = emblemkit.medallion()
    ry = emblemkit.R_Y
    wax = common.mat_wax()
    flame_mat = common.mat_flame()

    for x in (-CANDLE_X, CANDLE_X):
        candle(objs, x, wax, metal, ry)
        objs.append(common.add_uv_sphere(
            "flame", 0.07, location=(x, ry - 0.005, TOP_Z + 0.115),
            scale=(0.75, 0.55, 1.6), mat=flame_mat, segments=32, rings=16))

    emblemkit.finish(objs, "shabbat")


build()
