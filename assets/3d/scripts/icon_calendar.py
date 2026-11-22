"""Calendar icon: standing desk calendar (luach) in a brass frame,
with a crescent moon emblem on top (Jewish lunar calendar)."""

import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import iconkit


def build():
    common.reset_scene()
    common.setup_render()
    metal = common.theme_metal()
    accent = common.theme_accent()
    parchment = common.mat_parchment()

    # everything on the board tilts back together
    tilt = iconkit.empty("Tilt", rotation=(math.radians(-12.0), 0, 0))
    board = iconkit.round_box("back_board", (1.20, 0.09, 1.04), accent,
                              location=(0, 0.10, 0.60), bevel=0.04)
    pages = iconkit.round_box("pages", (1.04, 0.16, 0.90), parchment,
                              location=(0, -0.015, 0.60), bevel=0.03)
    frame = [
        iconkit.round_box("frame_top", (1.22, 0.11, 0.10), metal,
                          location=(0, -0.055, 1.075), bevel=0.03),
        iconkit.round_box("frame_bot", (1.22, 0.20, 0.10), metal,
                          location=(0, -0.10, 0.125), bevel=0.03),
        iconkit.round_box("frame_l", (0.10, 0.11, 1.06), metal,
                          location=(-0.56, -0.055, 0.60), bevel=0.03),
        iconkit.round_box("frame_r", (0.10, 0.11, 1.06), metal,
                          location=(0.56, -0.055, 0.60), bevel=0.03),
    ]
    # crescent emblem standing on top, horns up, facing the camera
    moon = iconkit.crescent("moon", 0.175, 0.135, 0.082, 0.075, metal, 0.012)
    moon.rotation_euler = (math.radians(90), math.radians(-90), 0)
    moon.location = (0, -0.04, 1.30)
    for ob in (board, pages, *frame, moon):
        iconkit.parent_keep(ob, tilt)

    # A-frame easel: triangular side plates (polygon pts are (z, y))
    tri_pts = [(-0.02, 0.06), (0.62, 0.06), (-0.02, 1.02)]
    easel = []
    for x in (0.605, -0.605):
        tri = iconkit.prism("easel", tri_pts, 0.07, accent, 0.015)
        tri.rotation_euler[1] = math.radians(-90)
        tri.location = (x, 0.10, 0.0)
        easel.append(tri)

    common.frame_object([board, pages, *frame, moon, *easel],
                        margin=0.12, elevation=25.0, azimuth=30.0)
    common.setup_lights()
    common.render(common.variant_name("calendar"), common.OUT_DIR)


build()
