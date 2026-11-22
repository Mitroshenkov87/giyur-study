"""Decor: horizontal divider rule with a pomegranate finial (both themes)."""

import math
import os
import sys

import bpy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common


def build():
    common.reset_scene()
    common.setup_render()
    metal = common.theme_metal()
    accent = common.theme_accent()
    objs = []

    # the rule
    rod = common.add_cylinder("rule", 0.022, 3.9, location=(0, 0, 0),
                              vertices=32, mat=metal)
    rod.rotation_euler[1] = math.radians(90)
    common.smooth(rod)
    objs.append(rod)

    for sx in (-1, 1):
        x = sx * 1.95
        objs.append(common.add_uv_sphere("end_knob", 0.055, location=(x, 0, 0),
                                         mat=metal, segments=32, rings=16))
        collar = common.add_torus("end_collar", 0.034, 0.013,
                                  location=(sx * 1.84, 0, 0), mat=accent,
                                  major_segments=32)
        collar.rotation_euler[1] = math.radians(90)
        objs.append(collar)
        objs.append(common.add_uv_sphere("mid_bead", 0.030,
                                         location=(sx * 0.92, 0, 0), mat=metal,
                                         segments=24, rings=12))

    # pomegranate finial at the centre
    body = common.add_uv_sphere("pomegranate", 0.165,
                                location=(0, 0, -0.01), scale=(1.0, 0.88, 1.04),
                                mat=metal, segments=48, rings=24)
    objs.append(body)
    crown = common.add_cone("calyx", 0.082, 0.022, 0.10,
                            location=(0, 0, 0.185), vertices=32, mat=metal)
    common.smooth(crown)
    objs.append(crown)
    for k in range(5):
        a = 2.0 * math.pi * k / 5
        objs.append(common.add_uv_sphere(
            "calyx_tip", 0.020,
            location=(0.062 * math.cos(a), 0.062 * math.sin(a), 0.145),
            mat=metal, segments=16, rings=8))
    objs.append(common.add_cylinder("stem", 0.016, 0.06,
                                    location=(0, 0, 0.255), vertices=16,
                                    mat=metal))
    for sx in (-1, 1):
        leaf = common.add_uv_sphere("leaf", 0.05,
                                    location=(sx * 0.09, -0.01, 0.135),
                                    scale=(1.7, 0.45, 0.7), mat=accent,
                                    segments=24, rings=12)
        leaf.rotation_euler[2] = sx * math.radians(28)
        objs.append(leaf)

    common.frame_object(objs, margin=0.12, focal=72.0,
                        elevation=18.0, azimuth=25.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render(common.variant_name("divider"), common.OUT_DIR)


build()
