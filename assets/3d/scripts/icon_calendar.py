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
    accent = iconkit.accent()
    parchment = iconkit.mat_parchment()

    # The whole board tilts back together. Parts are built in the untilted
    # local frame; the empty applies the lean.
    tilt = iconkit.empty("Tilt", rotation=(math.radians(-14.0), 0, 0))

    width, height, thick = 1.22, 1.12, 0.10
    cz = 0.64
    board = iconkit.round_box("back_board", (width, thick, height), accent,
                              location=(0, 0.08, cz), bevel=0.03)
    pages = iconkit.round_box("pages", (width - 0.18, 0.14, height - 0.20),
                              parchment, location=(0, -0.02, cz), bevel=0.025)
    ft = 0.13
    frame = [
        iconkit.round_box("frame_top", (width + 0.10, 0.16, ft), metal,
                          location=(0, -0.06, cz + height * 0.5 - 0.02),
                          bevel=0.02),
        iconkit.round_box("frame_bot", (width + 0.10, 0.20, ft + 0.02), metal,
                          location=(0, -0.08, cz - height * 0.5 + 0.02),
                          bevel=0.02),
        iconkit.round_box("frame_l", (ft, 0.16, height + 0.06), metal,
                          location=(-(width * 0.5) + 0.02, -0.06, cz),
                          bevel=0.02),
        iconkit.round_box("frame_r", (ft, 0.16, height + 0.06), metal,
                          location=(width * 0.5 - 0.02, -0.06, cz),
                          bevel=0.02),
    ]
    # Horns point +Y in the mesh; +90° about X stands them up, facing the camera.
    moon = iconkit.crescent("moon", 0.22, 0.168, 0.10, 0.07, metal, bevel_w=0.0)
    moon.rotation_euler = (math.radians(90), 0, 0)
    moon.location = (0, -0.05, cz + height * 0.5 + 0.20)

    # Side plates of a desk easel. Prism is XY; Ry -90 maps local X onto world Z,
    # so the tuples below are (z, y): feet on the ground, top tucked under the board.
    tri_pts = [(0.0, -0.22), (0.0, 0.72), (1.02, 0.12)]
    easel = []
    for x in (width * 0.5 + 0.02, -(width * 0.5 + 0.02)):
        tri = iconkit.prism("easel", tri_pts, 0.09, accent, 0.012)
        tri.rotation_euler = (0.0, math.radians(-90), 0.0)
        tri.location = (x, 0.0, 0.0)
        easel.append(tri)

    parts = [board, pages, *frame, moon, *easel]
    for ob in parts:
        iconkit.parent_keep(ob, tilt)

    common.frame_object(parts, margin=0.12, elevation=25.0, azimuth=30.0)
    common.setup_lights()
    common.render(common.variant_name("calendar"), common.OUT_DIR)


build()
