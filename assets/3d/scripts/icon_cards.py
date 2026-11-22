"""Cards icon: fanned stack of 3 thick index cards; top card has a brass
corner and a raised Hebrew letter alef."""

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
    parchment = common.mat_parchment()

    objs = []
    roots = []
    for rot, z in ((math.radians(-14.0), 0.035),
                   (0.0, 0.115),
                   (math.radians(14.0), 0.195)):
        root = iconkit.empty("card_root", rotation=(0, 0, rot))
        card = iconkit.round_box("card", (0.92, 1.22, 0.07), parchment,
                                 location=(0, 0.06, z), bevel=0.03)
        iconkit.parent_keep(card, root)
        objs.append(card)
        roots.append(root)

    # brass L-corner on the top card (parented so it follows the fan)
    bar1 = iconkit.round_box("corner_a", (0.20, 0.10, 0.11), metal,
                             location=(0.36, 0.635, 0.245), bevel=0.03)
    bar2 = iconkit.round_box("corner_b", (0.10, 0.22, 0.11), metal,
                             location=(0.415, 0.55, 0.245), bevel=0.03)
    alef = iconkit.hebrew_letter("א", "alef", size=0.55, extrude=0.045,
                                 mat=metal, location=(0, 0.05, 0.225))
    for ob in (bar1, bar2, alef):
        iconkit.parent_keep(ob, roots[2])
    objs += [bar1, bar2, alef]

    common.frame_object(objs, margin=0.12, elevation=25.0, azimuth=30.0)
    common.setup_lights()
    common.render(common.variant_name("cards"), common.OUT_DIR)


build()
