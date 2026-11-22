"""Kiddush cup: silver goblet with engraved bands, filled with red wine."""

import os
import sys

import bpy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common

# Outer profile bottom-to-top, over the lip and back down the inside wall.
PROFILE = [
    (0.0, 0.00),
    (0.34, 0.00),
    (0.36, 0.02),
    (0.36, 0.05),
    (0.30, 0.07),
    (0.10, 0.09),
    (0.085, 0.13),
    (0.105, 0.16),
    (0.075, 0.20),
    (0.070, 0.46),
    (0.160, 0.53),
    (0.150, 0.57),
    (0.100, 0.63),
    (0.140, 0.67),
    (0.300, 0.78),
    (0.400, 0.98),
    (0.440, 1.22),
    (0.450, 1.38),
    (0.440, 1.44),
    (0.420, 1.44),
    (0.432, 1.37),
    (0.425, 1.22),
    (0.385, 0.99),
    (0.290, 0.80),
    (0.0, 0.74),
]

WINE_PROFILE = [
    (0.0, 1.26),
    (0.420, 1.26),
    (0.428, 1.28),
    (0.429, 1.33),
    (0.0, 1.35),
]


def build():
    common.reset_scene()
    common.setup_render()
    silver = common.mat_silver()
    gold = common.mat_gold()
    wine_mat = common.mat_wine()

    cup = common.lathe("goblet", PROFILE, segments=96, mat=silver)
    common.bevel(cup, 0.008, segments=2)

    # engraved band: two fine gold rings on the bowl
    band_low = common.add_torus("engrave_low", 0.405, 0.010, location=(0, 0, 1.00), mat=gold)
    band_high = common.add_torus("engrave_high", 0.423, 0.010, location=(0, 0, 1.08), mat=gold)

    wine = common.lathe("wine", WINE_PROFILE, segments=96, mat=wine_mat)

    common.frame_object([cup, band_low, band_high, wine])
    common.setup_lights()
    common.add_shadow_catcher()
    common.render("kiddush_cup", common.OUT_DIR)


build()
