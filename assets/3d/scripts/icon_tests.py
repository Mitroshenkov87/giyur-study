"""Tests icon: rolled parchment scroll with a brass check-mark seal,
an inkwell and a quill."""

import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import iconkit


def build_seal(metal, accent):
    """Brass disc + accent inset + chunky check mark, leaning on the scroll.

    Built at the origin in local space (disc axis = Z, face = +Z), returned
    as parts of a group whose root is placed/tilted by the caller."""
    disc = common.add_cylinder("seal_disc", 0.21, 0.07,
                               location=(0, 0, 0), vertices=48, mat=metal)
    common.bevel(disc, 0.015)
    inset = common.add_cylinder("seal_inset", 0.155, 0.05,
                                location=(0, 0, 0.035), vertices=48, mat=accent)
    common.bevel(inset, 0.01)
    # check mark in the XY face plane: short arm down to the elbow, long arm up
    short = iconkit.round_box("check_a", (0.17, 0.07, 0.08), metal,
                              location=(-0.105, -0.035, 0.065), bevel=0.025,
                              rotation=(0, 0, math.radians(-45)))
    long_ = iconkit.round_box("check_b", (0.30, 0.07, 0.08), metal,
                              location=(0.05, 0.015, 0.065), bevel=0.025,
                              rotation=(0, 0, math.radians(46)))
    return [disc, inset, short, long_]


def build():
    common.reset_scene()
    common.setup_render()
    metal = common.theme_metal()
    accent = common.theme_accent()
    parchment = common.mat_parchment()
    wax = common.mat_wax()
    ink = iconkit.simple_mat("Ink", (0.015, 0.012, 0.02, 1.0), roughness=0.25)

    # rolled sheet, axis along X
    scroll = common.add_cylinder("scroll", 0.22, 1.05,
                                 location=(0, 0, 0.24), mat=parchment)
    scroll.rotation_euler[1] = math.radians(90)
    common.smooth(scroll)
    objs = [scroll]
    for sx in (-1, 1):
        cap = common.add_cylinder("scroll_cap", 0.07, 0.10,
                                  location=(sx * 0.575, 0, 0.24), mat=metal)
        cap.rotation_euler[1] = math.radians(90)
        objs.append(cap)

    # leaning check-mark seal
    seal_root = iconkit.empty("seal",
                              location=(-0.05, -0.21, 0.26),
                              rotation=(math.radians(-75.0), 0, 0))
    seal = build_seal(metal, accent)
    for ob in seal:
        iconkit.parent_keep(ob, seal_root)
    objs += seal

    # inkwell with a dark ink surface
    well = common.add_cylinder("inkwell", 0.13, 0.18,
                               location=(0.45, -0.16, 0.09), mat=metal)
    common.bevel(well, 0.015)
    rim = common.add_torus("inkwell_rim", 0.105, 0.02,
                           location=(0.45, -0.16, 0.185), mat=metal)
    surface = common.add_cylinder("ink_surface", 0.085, 0.02,
                                  location=(0.45, -0.16, 0.178), mat=ink)
    objs += [well, rim, surface]

    # quill: gently curved ivory shaft + brass nib in the well
    quill = common.tube_curve(
        "quill",
        [(0.47, -0.14, 0.16), (0.40, -0.10, 0.38), (0.24, 0.0, 0.72)],
        0.032, mat=wax)
    objs.append(quill)
    nib = common.add_cone("nib", 0.045, 0.012, 0.14,
                          location=(0.475, -0.145, 0.10), mat=metal)
    objs.append(nib)

    common.frame_object(objs, margin=0.12, elevation=25.0, azimuth=30.0)
    common.setup_lights()
    common.render(common.variant_name("tests"), common.OUT_DIR)


build()
