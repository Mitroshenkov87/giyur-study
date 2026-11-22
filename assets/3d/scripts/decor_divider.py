"""Decor: horizontal divider — thicker brass rule, pomegranate finial."""

import math
import os
import sys

import bpy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import emblemkit

# Length / overall height stays near 8:1. The rule itself is thick enough
# to survive a small thumbnail (radius was 0.022 and disappeared).
HALF_LEN = 1.72
BAR_H = 0.28   # vertical thickness of the rule, seen from the front


def build():
    common.reset_scene()
    common.setup_render()
    metal = emblemkit.medal_metal(0.2)
    fruit = emblemkit.fruit_red()
    calyx = emblemkit.leaf_green()
    objs = []

    # Flat bar rather than a wire, so the rule stays visible when centred
    # in a square frame. Scale is applied before the bevel.
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=(0, 0, 0))
    rod = bpy.context.object
    rod.name = "rule"
    rod.scale = (HALF_LEN * 2.0, 0.09, BAR_H)
    bpy.ops.object.transform_apply(scale=True)
    common.assign(rod, metal)
    common.bevel(rod, 0.035, segments=2)
    objs.append(rod)

    for sx in (-1, 1):
        objs.append(common.add_uv_sphere(
            "end_knob", 0.145, location=(sx * (HALF_LEN + 0.02), 0, 0),
            mat=metal, segments=28, rings=14))
        collar = common.add_torus(
            "end_collar", 0.09, 0.028,
            location=(sx * (HALF_LEN - 0.22), 0, 0),
            mat=metal, major_segments=28, minor_segments=10)
        collar.rotation_euler[1] = math.radians(90)
        objs.append(collar)

    # Pomegranate finial, large against the rule.
    body = common.add_uv_sphere(
        "pomegranate", 0.46, location=(0, 0, 0.06),
        scale=(1.0, 0.82, 0.98), mat=fruit, segments=48, rings=24)
    objs.append(body)
    mount = common.add_torus(
        "pom_mount", 0.16, 0.04, location=(0, 0, -0.02),
        mat=metal, major_segments=28, minor_segments=10)
    mount.rotation_euler[0] = math.radians(90)
    objs.append(mount)
    for k in range(6):
        a = 2.0 * math.pi * k / 6.0
        sep = common.add_cone(
            "sepal", 0.07, 0.016, 0.38,
            location=(0.10 * math.cos(a), 0.06 * math.sin(a), 0.58),
            vertices=8, mat=calyx)
        sep.rotation_euler = (math.sin(a) * 0.45, -math.cos(a) * 0.45, 0.0)
        common.smooth(sep)
        objs.append(sep)
    objs.append(common.add_uv_sphere(
        "stem_knob", 0.055, location=(0, 0, 0.78), mat=metal,
        segments=14, rings=8))

    common.frame_object(objs, margin=0.10, focal=70.0,
                        elevation=14.0, azimuth=16.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render(common.variant_name("divider"), common.OUT_DIR)


build()
