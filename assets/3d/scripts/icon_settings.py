"""Settings icon: brass gear/cog with a wooden (or leather) hub."""

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

    cog = iconkit.gear("cog", teeth=9, r_root=0.44, r_tip=0.60,
                       thickness=0.20, mat=metal, bevel_w=0.015)
    # stand it up nearly facing the camera, slight tilt
    cog.rotation_euler = (math.radians(80.0), 0, 0)

    hub = common.add_cylinder("hub", 0.21, 0.34, location=(0, -0.02, 0),
                              mat=accent)
    hub.rotation_euler[0] = math.radians(90)
    common.bevel(hub, 0.03)
    smooth_hub = hub
    axle = common.add_cylinder("axle", 0.075, 0.12,
                               location=(0, -0.21, 0), mat=metal)
    axle.rotation_euler[0] = math.radians(90)
    cap = common.add_uv_sphere("axle_cap", 0.09, location=(0, -0.27, 0),
                               scale=(1.0, 0.7, 1.0), mat=metal)

    common.frame_object([cog, smooth_hub, axle, cap],
                        margin=0.12, elevation=25.0, azimuth=30.0)
    common.setup_lights()
    common.render(common.variant_name("settings"), common.OUT_DIR)


build()
