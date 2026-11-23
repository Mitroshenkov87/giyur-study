"""Prayers icon: a siddur lying open on a small wooden shtender (lectern).

The slanted desk with a brass ledge is what sets it apart from the plain open
book of «Уроки»: this is the book you pray from, not the one you study.
"""

import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import iconkit

SLOPE = math.radians(32.0)   # desk tilt, rising away from the viewer
DESK_W, DESK_D, DESK_T = 1.56, 1.12, 0.09
PAGE_W, PAGE_D, PAGE_T = 0.60, 0.84, 0.07
PAGE_TILT = math.radians(7.0)


def build():
    common.reset_scene()
    common.setup_render()
    metal = common.theme_metal()
    wood = common.mat_walnut()
    parchment = iconkit.mat_parchment()
    cover = iconkit.mat_leather_color("SiddurCover", (0.035, 0.018, 0.010, 1.0), 0.6)
    ink = iconkit.simple_mat("Ink", (0.05, 0.030, 0.016, 1.0), roughness=0.8)
    ribbon = iconkit.simple_mat("Ribbon", (0.24, 0.012, 0.018, 1.0), roughness=0.45)
    objs = []

    # Side panels: right triangles (front low, back high), under the desk.
    h_back = DESK_D * math.sin(SLOPE)
    depth = DESK_D * math.cos(SLOPE)
    tri = [(-depth / 2.0, 0.0), (depth / 2.0, 0.0), (depth / 2.0, h_back)]
    for sx in (-1, 1):
        side = iconkit.prism("side", tri, 0.08, wood, bevel_w=0.012)
        side.rotation_euler = (math.radians(90.0), 0.0, math.radians(90.0))
        side.location = (sx * (DESK_W / 2.0 - 0.10), 0.0, 0.0)
        objs.append(side)
    objs.append(iconkit.round_box("base_front", (DESK_W - 0.12, 0.10, 0.08), wood,
                                  location=(0.0, -depth / 2.0 + 0.05, 0.04), bevel=0.015))

    # The desk frame: x across, y up the slope, z out of the board.
    desk = iconkit.empty("desk", location=(0.0, 0.0, h_back / 2.0 + 0.05),
                         rotation=(SLOPE, 0.0, 0.0))
    parts = [
        iconkit.round_box("board", (DESK_W, DESK_D, DESK_T), wood, bevel=0.02),
        iconkit.round_box("ledge", (DESK_W * 0.96, 0.08, 0.16), metal,
                          location=(0.0, -DESK_D / 2.0 + 0.02, 0.09), bevel=0.02),
        iconkit.round_box("cover", (PAGE_W * 2 + 0.10, PAGE_D + 0.06, 0.04), cover,
                          location=(0.0, 0.03, DESK_T / 2.0 + 0.02), bevel=0.012),
    ]
    for ob in parts:
        iconkit.parent_keep(ob, desk)
    objs += parts

    z_page = DESK_T / 2.0 + 0.04 + PAGE_T / 2.0
    for sign in (-1, 1):
        page = iconkit.empty("page", location=(sign * (PAGE_W / 2.0 + 0.01), 0.03, z_page),
                             rotation=(0.0, -sign * PAGE_TILT, 0.0))
        iconkit.parent_keep(page, desk)
        pp = [iconkit.round_box("pages", (PAGE_W, PAGE_D, PAGE_T), parchment, bevel=0.012)]
        # Four lines of "text" per page, right-aligned like Hebrew.
        for i in range(4):
            w = PAGE_W * (0.70 if i % 2 == 0 else 0.56)
            x = (PAGE_W * 0.36) - w / 2.0
            pp.append(iconkit.round_box(
                "line", (w, 0.045, 0.008), ink,
                location=(x, 0.26 - i * 0.16, PAGE_T / 2.0 + 0.003), bevel=0.0))
        if sign == 1:
            pp.append(iconkit.round_box("ribbon", (0.06, PAGE_D * 0.98, 0.010), ribbon,
                                        location=(-0.18, -0.02, PAGE_T / 2.0 + 0.006),
                                        bevel=0.0))
        for ob in pp:
            iconkit.parent_keep(ob, page)
        objs += pp
    # Brass corner caps on the board front.
    for sx in (-1, 1):
        cap = iconkit.round_box("cap", (0.14, 0.14, DESK_T + 0.02), metal,
                                location=(sx * (DESK_W / 2.0 - 0.06), DESK_D / 2.0 - 0.06, 0.0),
                                bevel=0.015)
        iconkit.parent_keep(cap, desk)
        objs.append(cap)

    common.frame_object(objs, margin=0.10, elevation=28.0, azimuth=30.0)
    common.setup_lights()
    common.render(common.variant_name("prayers"), common.OUT_DIR)


build()
