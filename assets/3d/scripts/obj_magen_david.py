"""Magen David: Star of David emblem, gold, bevelled, slightly tilted."""

import math
import os
import sys

import bpy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common

OUTER_R = 1.0
INNER_R = 0.5
EXTRUDE = 0.10


def build_star(gold):
    """Hexagram outline (12 pts, alternating outer/inner radius) as an
    extruded flat curve -> one solid six-pointed star."""
    cu = bpy.data.curves.new("star_curve", "CURVE")
    cu.dimensions = "2D"
    cu.resolution_u = 1
    cu.render_resolution_u = 1
    cu.extrude = EXTRUDE
    cu.bevel_depth = 0.022
    cu.bevel_resolution = 3
    sp = cu.splines.new("POLY")
    pts = []
    for i in range(12):
        ang = math.radians(90.0 - i * 30.0)
        rad = OUTER_R if i % 2 == 0 else INNER_R
        pts.append((rad * math.cos(ang), rad * math.sin(ang)))
    sp.points.add(len(pts) - 1)
    for p, co in zip(sp.points, pts):
        p.co = (co[0], co[1], 0.0, 1.0)
    sp.use_cyclic_u = True
    ob = bpy.data.objects.new("magen_david", cu)
    bpy.context.collection.objects.link(ob)
    common.assign(ob, gold)
    # stand the emblem upright, facing the camera
    ob.rotation_euler.x = math.radians(90.0)
    return ob


def build():
    common.reset_scene()
    common.setup_render()
    gold = common.mat_gold()

    star = build_star(gold)

    # nearly frontal, slightly tilted toward the camera
    common.frame_object([star], margin=0.12, focal=72.0,
                        elevation=16.0, azimuth=20.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render("magen_david", common.OUT_DIR)


build()
