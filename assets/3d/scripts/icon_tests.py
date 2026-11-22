"""Tests icon: rolled parchment scroll with a brass check-mark seal,
an inkwell and a quill."""

import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import iconkit


def build_seal(metal, accent):
    """Brass disc + accent inset + chunky check mark.

    Built at the origin, disc axis = Z. The caller parents the parts to an
    empty that leans the seal onto the scroll."""
    disc = common.add_cylinder("seal_disc", 0.24, 0.07,
                               location=(0, 0, 0), vertices=48, mat=metal)
    common.bevel(disc, 0.015)
    inset = common.add_cylinder("seal_inset", 0.175, 0.045,
                                location=(0, 0, 0.04), vertices=48, mat=accent)
    common.bevel(inset, 0.01)
    # Both arms share the bottom vertex of the check so it reads as one mark.
    vertex = (-0.01, -0.07)
    s_ang, s_len = math.radians(125), 0.24
    l_ang, l_len = math.radians(36), 0.40
    short = iconkit.round_box(
        "check_a", (s_len, 0.095, 0.08), metal,
        location=(vertex[0] + math.cos(s_ang) * s_len * 0.5,
                  vertex[1] + math.sin(s_ang) * s_len * 0.5, 0.09),
        bevel=0.018, rotation=(0, 0, s_ang))
    long_ = iconkit.round_box(
        "check_b", (l_len, 0.095, 0.08), metal,
        location=(vertex[0] + math.cos(l_ang) * l_len * 0.5,
                  vertex[1] + math.sin(l_ang) * l_len * 0.5, 0.09),
        bevel=0.018, rotation=(0, 0, l_ang))
    return [disc, inset, short, long_]


def _spiral(name, x, z, radius, tube, mat, turns=1.45):
    """Cinnamon-roll end of the scroll, in the YZ plane."""
    pts = []
    n = 40
    for i in range(n):
        t = turns * 2.0 * math.pi * i / (n - 1)
        r = tube * 1.4 + (radius - tube * 2.2) * (i / (n - 1))
        pts.append((x, r * math.sin(t), z + r * math.cos(t)))
    return common.tube_curve(name, pts, tube, mat=mat, resolution=2)


def build():
    common.reset_scene()
    common.setup_render()
    metal = common.theme_metal()
    accent = iconkit.accent()
    parchment = iconkit.mat_parchment()
    ivory = iconkit.mat_ivory()
    ink = iconkit.simple_mat("Ink", (0.02, 0.012, 0.015, 1.0), roughness=0.18)

    # Rolled sheet, axis along X. The near (+X) end carries a visible spiral.
    sz = 0.32
    scroll = common.add_cylinder("scroll", 0.24, 1.05,
                                 location=(0, 0, sz), mat=parchment)
    scroll.rotation_euler[1] = math.radians(90)
    common.smooth(scroll)
    objs = [scroll]
    for sx, x in ((-1, -0.40), (1, 0.40)):
        lip = common.add_torus("scroll_lip", 0.24, 0.045,
                               location=(x, 0, sz), mat=parchment)
        lip.rotation_euler[1] = math.radians(90)
        objs.append(lip)
    # Sit the coils just past each end so the roll is the end cap, not buried.
    objs.append(_spiral("spiral_near", 0.64, sz, 0.28, 0.072, parchment, turns=1.25))
    objs.append(_spiral("spiral_far", -0.62, sz, 0.26, 0.060, parchment, turns=1.15))
    # brass pin in the middle of the near spiral, and a cap on the far end
    pin = common.add_cylinder("scroll_pin", 0.045, 0.10,
                              location=(0.62, 0, sz), mat=metal)
    pin.rotation_euler[1] = math.radians(90)
    far = common.add_cylinder("scroll_cap", 0.07, 0.08,
                              location=(-0.62, 0, sz), mat=metal)
    far.rotation_euler[1] = math.radians(90)
    objs += [pin, far]

    # +X rotation turns the check face (local +Z) toward the camera (-Y).
    seal_root = iconkit.empty("seal",
                              location=(-0.06, -0.46, sz + 0.02),
                              rotation=(math.radians(58.0), 0.0, math.radians(-18)))
    seal = build_seal(metal, accent)
    for ob in seal:
        iconkit.parent_keep(ob, seal_root)
    objs += seal

    well = common.add_cylinder("inkwell", 0.14, 0.20,
                               location=(0.62, -0.34, 0.10), mat=metal)
    common.bevel(well, 0.018)
    rim = common.add_torus("inkwell_rim", 0.115, 0.022,
                           location=(0.62, -0.34, 0.20), mat=metal)
    surface = common.add_cylinder("ink_surface", 0.09, 0.02,
                                  location=(0.62, -0.34, 0.195), mat=ink)
    objs += [well, rim, surface]

    # Thin shaft. The feather is a flat vane, not a second fat tube.
    quill = common.tube_curve(
        "quill",
        [(0.62, -0.36, 0.16), (0.50, -0.30, 0.38), (0.28, -0.16, 0.78),
         (0.14, -0.06, 1.08)],
        0.011, mat=ivory, resolution=3)
    objs.append(quill)
    # Long narrow vane. Asymmetric so it reads as a feather, not a leaf.
    leaf = [
        (0.00, 0.00), (0.09, 0.10), (0.15, 0.28), (0.12, 0.48),
        (0.05, 0.66), (0.00, 0.78), (-0.03, 0.66), (-0.08, 0.48),
        (-0.11, 0.28), (-0.06, 0.10),
    ]
    vane_mat = iconkit.simple_mat("Vane", (0.32, 0.23, 0.12, 1.0), roughness=0.72)
    vane = iconkit.prism("vane", leaf, 0.028, vane_mat, bevel_w=0.0)
    vane.rotation_euler = (math.radians(58), math.radians(8), math.radians(-24))
    vane.location = (0.42, -0.34, 0.32)
    objs.append(vane)
    nib = common.add_cone("nib", 0.028, 0.006, 0.11,
                          location=(0.64, -0.35, 0.12), mat=metal)
    nib.rotation_euler = (math.radians(18), 0, 0)
    nib.scale = (0.55, 1.0, 1.0)
    objs.append(nib)

    common.frame_object(objs, margin=0.12, elevation=25.0, azimuth=30.0)
    common.setup_lights()
    common.render(common.variant_name("tests"), common.OUT_DIR)


build()
