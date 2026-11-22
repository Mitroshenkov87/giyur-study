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
    parchment = iconkit.mat_parchment()

    # Cards lie in XY. A Z rotation on each empty fans the stack.
    cw, cd, ct = 1.05, 1.38, 0.09
    objs = []
    roots = []
    for rot, z in ((math.radians(-12.0), ct * 0.5),
                   (0.0, ct * 1.55),
                   (math.radians(12.0), ct * 2.60)):
        root = iconkit.empty("card_root", rotation=(0, 0, rot))
        card = iconkit.round_box("card", (cw, cd, ct), parchment,
                                 location=(0, 0, z), bevel=0.03)
        iconkit.parent_keep(card, root)
        objs.append(card)
        roots.append(root)

    top_z = ct * 2.60 + ct * 0.5 + 0.04
    # L-shaped brass corner on the top card's upper-right
    bar_x = iconkit.round_box("corner_a", (0.34, 0.12, 0.10), metal,
                              location=(cw * 0.5 - 0.21, cd * 0.5 - 0.09, top_z),
                              bevel=0.025)
    bar_y = iconkit.round_box("corner_b", (0.12, 0.34, 0.10), metal,
                              location=(cw * 0.5 - 0.09, cd * 0.5 - 0.21, top_z),
                              bevel=0.025)
    # Large א, no bevel — the bevelled glyph filled its counters and read
    # as a knot. Flat to the card, big enough to survive 48 px.
    alef = iconkit.hebrew_letter(
        "א", "alef", size=1.45, extrude=0.10, mat=metal,
        location=(0.0, -0.02, top_z - 0.02))
    alef.data.bevel_depth = 0.0
    alef.data.resolution_u = 14
    for ob in (bar_x, bar_y, alef):
        iconkit.parent_keep(ob, roots[2])
    objs += [bar_x, bar_y, alef]

    # Higher than the usual 25° so the א on the top card stays legible.
    common.frame_object(objs, margin=0.14, elevation=48.0, azimuth=24.0)
    common.setup_lights()
    common.render(common.variant_name("cards"), common.OUT_DIR)


build()
