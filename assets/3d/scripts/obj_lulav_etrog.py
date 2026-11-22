"""Lulav (palm frond with myrtle and willow, bound with rings) leaning next
to an etrog in an open silver box."""

import math
import os
import sys

import bpy
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import foodkit

SPINE_LEN = 2.75
BASE = Vector((-0.42, 0.18, 0.02))
LEAN = 0.55


def spine_pt(s):
    return Vector((BASE.x + LEAN * s ** 1.7,
                   BASE.y + 0.06 * s,
                   BASE.z + SPINE_LEN * s))


def spine_tangent(s):
    eps = 1e-3
    return (spine_pt(min(1.0, s + eps)) - spine_pt(max(0.0, s - eps))).normalized()


def spine_radius(s):
    return 0.050 * (1.0 - 0.72 * s) + 0.006


def build_spine(palm_mat, tip_mat):
    n = 26
    pts = [spine_pt(i / (n - 1)) for i in range(n)]
    radii = [spine_radius(i / (n - 1)) for i in range(n)]
    spine = foodkit.tapered_tube("lulav_spine", pts, radii, sides=14,
                                 mat=palm_mat)
    tip = common.add_cone("lulav_tip", 0.016, 0.002, 0.16,
                          location=spine_pt(1.0) + spine_tangent(1.0) * 0.07,
                          vertices=10, mat=tip_mat)
    q = Vector((0, 0, 1)).rotation_difference(spine_tangent(1.0))
    tip.rotation_mode = "QUATERNION"
    tip.rotation_quaternion = q
    return [spine, tip]


def build_leaflets(palm_mat):
    objs = []
    n = 17
    for i in range(n):
        s = 0.16 + 0.79 * i / (n - 1)
        p = spine_pt(s)
        length = 0.16 + 0.62 * (1.0 - 0.55 * s)
        elev = math.radians(26 + 46 * s)
        width = 0.055
        for side in (-1, 1):
            leaf = foodkit.ribbon("leaflet", length, width, curl=0.6,
                                  fold=0.4, mat=palm_mat, stations=8,
                                  thickness=0.005)
            jitter = math.radians(6.0 * math.sin(i * 3.1 + side))
            a = -elev if side > 0 else math.pi + elev
            leaf.rotation_euler = (0.0, a + jitter * side,
                                   math.radians(3.0) * side)
            leaf.location = p + Vector((0.0, 0.012 * side, 0.0))
            objs.append(leaf)
    return objs


def build_binding(fiber_mat, myrtle_mat, willow_mat):
    objs = []
    # three binding rings around spine + lower bundle
    for s in (0.135, 0.185, 0.235):
        p = spine_pt(s)
        r = spine_radius(s) + 0.020
        ring = common.add_torus("binding_ring", r, 0.016, location=p,
                                mat=fiber_mat, major_segments=32,
                                minor_segments=10)
        q = Vector((0, 0, 1)).rotation_difference(spine_tangent(s))
        ring.rotation_mode = "QUATERNION"
        ring.rotation_quaternion = q
        objs.append(ring)
    # myrtle (oval leaves) and willow (long slivers) twigs behind the rings
    for side in (-1, 1):
        for j, s0 in enumerate((0.115, 0.175, 0.235)):
            is_willow = j == 2
            start = spine_pt(s0) + Vector((0.0, 0.02, -0.02))
            droop = -0.28 if is_willow else -0.18
            out = Vector((side * 0.55, 0.10, droop)).normalized()
            length = 0.62 if is_willow else 0.48
            end = start + out * length
            mid = start + out * length * 0.5 + Vector((0, 0, 0.03))
            r0 = 0.012 if is_willow else 0.014
            twig = foodkit.tapered_tube("twig", [start, mid, end],
                                        [r0, r0 * 0.8, r0 * 0.35],
                                        sides=8,
                                        mat=willow_mat if is_willow else myrtle_mat)
            objs.append(twig)
            for k in range(1, 5):
                t = k / 5.0
                lp = start + (end - start) * t + Vector((0, 0, 0.02))
                if is_willow:
                    leaf = common.add_uv_sphere(
                        "willow_leaf", 0.045, location=lp,
                        scale=(2.6, 0.35, 0.6),
                        mat=willow_mat, segments=12, rings=8)
                    leaf.rotation_euler.z = side * 0.5 + k * 0.7
                else:
                    leaf = common.add_uv_sphere(
                        "myrtle_leaf", 0.038, location=lp,
                        scale=(1.0, 0.55, 1.6),
                        mat=myrtle_mat, segments=12, rings=8)
                    leaf.rotation_euler = (0.4 * side, 0.3 * k, side * 0.6)
                objs.append(leaf)
    return objs


def build_etrog_box(silver, gold):
    bx, by = 0.62, -0.12
    objs = []
    objs.append(foodkit.rounded_box("box_base", (1.05, 0.82, 0.12),
                                    (bx, by, 0.06), silver, bevel_w=0.03))
    wall_h = 0.34
    zc = 0.12 + wall_h * 0.5
    objs.append(foodkit.rounded_box("box_wall_f", (1.05, 0.06, wall_h),
                                    (bx, by - 0.38, zc), silver, bevel_w=0.02))
    objs.append(foodkit.rounded_box("box_wall_b", (1.05, 0.06, wall_h),
                                    (bx, by + 0.38, zc), silver, bevel_w=0.02))
    objs.append(foodkit.rounded_box("box_wall_l", (0.06, 0.70, wall_h),
                                    (bx - 0.495, by, zc), silver, bevel_w=0.02))
    objs.append(foodkit.rounded_box("box_wall_r", (0.06, 0.70, wall_h),
                                    (bx + 0.495, by, zc), silver, bevel_w=0.02))
    zr = 0.12 + wall_h
    objs.append(foodkit.rounded_box("box_rim_f", (1.09, 0.075, 0.045),
                                    (bx, by - 0.38, zr), gold, bevel_w=0.015))
    objs.append(foodkit.rounded_box("box_rim_b", (1.09, 0.075, 0.045),
                                    (bx, by + 0.38, zr), gold, bevel_w=0.015))
    objs.append(foodkit.rounded_box("box_rim_l", (0.075, 0.70, 0.045),
                                    (bx - 0.495, by, zr), gold, bevel_w=0.015))
    objs.append(foodkit.rounded_box("box_rim_r", (0.075, 0.70, 0.045),
                                    (bx + 0.495, by, zr), gold, bevel_w=0.015))
    return objs, (bx, by)


def build_etrog(etrog_mat, calyx_mat, box_center):
    bx, by = box_center
    etrog = common.add_uv_sphere("etrog", 0.34, location=(bx, by, 0.46),
                                 scale=(0.85, 0.72, 1.05), mat=etrog_mat)
    etrog.rotation_euler = (math.radians(9), math.radians(-11),
                            math.radians(6))
    pitam = common.add_cylinder("pitam", 0.014, 0.07,
                                location=(0, 0, 0), vertices=10,
                                mat=calyx_mat)
    pitam.parent = etrog
    pitam.location = (0.0, 0.0, 0.385)
    knob = common.add_uv_sphere("pitam_knob", 0.022, location=(0, 0, 0),
                                mat=calyx_mat, segments=12, rings=8)
    knob.parent = etrog
    knob.location = (0.0, 0.0, 0.43)
    return [etrog, pitam, knob]


def build():
    common.reset_scene()
    common.setup_render()
    silver = common.mat_silver()
    gold = common.mat_gold()
    palm_mat = foodkit.prin("PalmSpine", (0.45, 0.50, 0.18, 1.0), rough=0.5,
                            ramp=[(0.0, (0.35, 0.42, 0.12, 1.0)),
                                  (1.0, (0.55, 0.56, 0.22, 1.0))],
                            ramp_scale=3.0, bump=0.15, bump_scale=20.0)
    leaflet_mat = foodkit.prin("PalmLeaf", (0.18, 0.44, 0.12, 1.0), rough=0.45,
                               coat=0.15, subsurface=0.04,
                               ramp=[(0.0, (0.12, 0.34, 0.08, 1.0)),
                                     (0.6, (0.24, 0.52, 0.15, 1.0)),
                                     (1.0, (0.15, 0.40, 0.10, 1.0))],
                               ramp_scale=2.5, bump=0.1, bump_scale=35.0)
    myrtle_mat = foodkit.prin("Myrtle", (0.08, 0.30, 0.10, 1.0), rough=0.5,
                              coat=0.15)
    willow_mat = foodkit.prin("Willow", (0.38, 0.52, 0.22, 1.0), rough=0.55)
    fiber_mat = foodkit.prin("Fiber", (0.55, 0.42, 0.25, 1.0), rough=0.8)
    etrog_mat = foodkit.prin(
        "EtrogSkin", (0.80, 0.66, 0.14, 1.0), rough=0.42, coat=0.2,
        subsurface=0.06,
        ramp=[(0.0, (0.60, 0.55, 0.10, 1.0)),
              (0.45, (0.88, 0.72, 0.16, 1.0)),
              (0.75, (0.66, 0.68, 0.18, 1.0)),
              (1.0, (0.72, 0.60, 0.13, 1.0))],
        ramp_scale=4.0, ramp_detail=7.0, bump=0.3, bump_scale=26.0)
    calyx_mat = foodkit.prin("PitamWood", (0.42, 0.30, 0.14, 1.0), rough=0.7)

    objs = []
    objs += build_spine(palm_mat, leaflet_mat)
    objs += build_leaflets(leaflet_mat)
    objs += build_binding(fiber_mat, myrtle_mat, willow_mat)
    box_objs, box_center = build_etrog_box(silver, gold)
    objs += box_objs
    objs += build_etrog(etrog_mat, calyx_mat, box_center)

    common.frame_object(objs, margin=0.14, elevation=21.0, azimuth=30.0,
                        focal=72.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render("lulav_etrog", common.OUT_DIR)


build()
