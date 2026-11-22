"""Progress icon: pomegranate (613 mitzvot, fullness) on a small plinth."""

import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import iconkit

PLINTH_PROFILE = [
    (0.0, 0.00),
    (0.46, 0.00),
    (0.50, 0.03),
    (0.50, 0.07),
    (0.42, 0.10),
    (0.40, 0.12),
    (0.40, 0.24),
    (0.44, 0.28),
    (0.44, 0.31),
    (0.0, 0.33),
]


def build():
    common.reset_scene()
    common.setup_render()
    metal = common.theme_metal()
    accent = iconkit.accent()
    fruit_mat = iconkit.mat_pomegranate()
    calyx_mat = iconkit.simple_mat("Calyx", (0.12, 0.045, 0.02, 1.0),
                                   roughness=0.55)

    plinth = common.lathe("plinth", PLINTH_PROFILE, segments=64, mat=metal)
    common.bevel(plinth, 0.012)
    top = common.add_cylinder("plinth_top", 0.41, 0.06,
                              location=(0, 0, 0.345), mat=accent)
    common.bevel(top, 0.02)

    # fruit body, slightly squat
    fruit = common.add_uv_sphere("fruit", 0.40, location=(0, 0, 0.80),
                                 scale=(1.0, 1.0, 0.95), mat=fruit_mat)
    # calyx crown: ring of spikes + taller centre spike
    calyx = []
    for i in range(6):
        a = 2.0 * math.pi * i / 6.0
        spike = common.add_cone("calyx", 0.085, 0.0, 0.22,
                                location=(0.115 * math.cos(a),
                                          0.115 * math.sin(a), 1.155),
                                mat=calyx_mat)
        spike.rotation_euler = (0.0, math.radians(26.0), a)
        calyx.append(spike)
    calyx.append(common.add_cone("calyx_mid", 0.10, 0.0, 0.26,
                                 location=(0, 0, 1.18), mat=calyx_mat))

    common.frame_object([plinth, top, fruit] + calyx,
                        margin=0.12, elevation=25.0, azimuth=30.0)
    common.setup_lights()
    common.render(common.variant_name("progress"), common.OUT_DIR)


build()
