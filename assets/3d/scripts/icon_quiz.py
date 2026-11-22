"""Quiz icon: short brass ladder of 5 rungs climbing to a crown finial."""

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

    lean = iconkit.empty("lean", rotation=(0, math.radians(-10.0), 0))
    parts = []
    for x in (-0.40, 0.40):
        rail = common.add_cylinder("rail", 0.065, 1.30,
                                   location=(x, 0, 0.65), mat=metal)
        common.smooth(rail)
        parts.append(rail)
    for z in (0.22, 0.46, 0.70, 0.94, 1.18):
        rung = common.add_cylinder("rung", 0.055, 0.86,
                                   location=(0, 0, z), mat=metal)
        rung.rotation_euler[0] = math.radians(90)
        common.smooth(rung)
        parts.append(rung)

    # crown finial: band facing the camera + spikes
    band = common.add_cylinder("crown_band", 0.34, 0.17,
                               location=(0, 0, 1.44), mat=metal)
    band.rotation_euler[0] = math.radians(90)
    common.bevel(band, 0.02)
    parts.append(band)
    for i in range(6):
        a = 2.0 * math.pi * i / 6.0
        spike = common.add_cone("crown_spike", 0.085, 0.0, 0.24,
                                location=(0.27 * math.cos(a), 0.27 * math.sin(a),
                                          1.60), mat=metal)
        spike.rotation_euler = (0.0, math.radians(22.0), a)
        parts.append(spike)
    parts.append(common.add_cone("crown_mid", 0.10, 0.0, 0.30,
                                 location=(0, 0, 1.65), mat=metal))

    for ob in parts:
        iconkit.parent_keep(ob, lean)

    common.frame_object(parts, margin=0.12, elevation=25.0, azimuth=30.0)
    common.setup_lights()
    common.render(common.variant_name("quiz"), common.OUT_DIR)


build()
