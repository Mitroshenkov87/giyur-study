"""Tallit: folded white wool prayer shawl with dark stripes near the ends,
a silver atarah collar band, and knotted tzitzit fringes at the corners."""

import math
import os
import sys

import bpy
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common

NAVY = (0.045, 0.06, 0.13, 1.0)
WHITE = (0.93, 0.93, 0.90, 1.0)


def tallit_cloth():
    """White wool with dark stripes near the two short ends (X axis)."""
    m, nt, bsdf = common._new_material("TallitCloth")
    tc = common._texcoord(nt)
    wave = nt.nodes.new("ShaderNodeTexWave")
    wave.location = (-760, 140)
    wave.wave_type = "BANDS"
    wave.bands_direction = "X"
    if "Scale" in wave.inputs:
        wave.inputs["Scale"].default_value = 1.0
    if "Distortion" in wave.inputs:
        wave.inputs["Distortion"].default_value = 0.0
    ramp = common._ramp(nt, [(0.0, NAVY), (0.12, WHITE),
                             (0.88, WHITE), (1.0, NAVY)])
    ramp.color_ramp.interpolation = "CONSTANT"
    nt.links.new(tc.outputs["Generated"], wave.inputs["Vector"])
    nt.links.new(wave.outputs["Fac"], ramp.inputs["Fac"])
    nt.links.new(ramp.outputs["Color"], bsdf.inputs["Base Color"])
    weave = common._noise(nt, scale=220.0, detail=2.0, y=-320)
    common._bump(nt, bsdf, weave.outputs["Fac"], strength=0.15, distance=0.005)
    common._inp(bsdf, "Roughness", 0.85)
    common._inp(bsdf, "Sheen Weight", 0.45)
    return m


def tzitzit_white():
    m, nt, bsdf = common._new_material("TzitzitWhite")
    common._inp(bsdf, "Base Color", (0.96, 0.96, 0.94, 1.0))
    common._inp(bsdf, "Roughness", 0.70)
    common._inp(bsdf, "Sheen Weight", 0.30)
    return m


def add_slab(name, dims, loc, cloth):
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=loc)
    ob = bpy.context.object
    ob.name = name
    ob.scale = dims
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    common.assign(ob, cloth)
    common.bevel(ob, 0.035, segments=3)
    return ob


def tzitzit_bundle(objs, cx, cy, white):
    """One corner fringe: wound top + knot + 4 hanging strings."""
    top_z, floor_z = 0.02, -0.46
    winder = common.add_cone(
        "tz_wind", 0.022, 0.028, 0.13,
        location=(cx, cy, top_z - 0.065), vertices=24, mat=white)
    knot = common.add_uv_sphere("tz_knot", 0.024,
                                location=(cx, cy, top_z - 0.145),
                                scale=(1.0, 1.0, 0.75), mat=white,
                                segments=24, rings=12)
    objs += [winder, knot]
    k = top_z - 0.16
    for i, (dx, dy) in enumerate(((-0.018, -0.012), (0.016, -0.016),
                                  (-0.010, 0.018), (0.018, 0.010))):
        sx, sy = cx + dx, cy + dy
        drift = 0.03 * ((i % 2) * 2 - 1)
        pts = [Vector((sx, sy, k)),
               Vector((sx * 1.0 + dx * 2.0, sy + dy * 2.0, k - 0.16)),
               Vector((sx + dx * 3.0 + drift, sy + dy * 3.0, floor_z + 0.03)),
               Vector((sx + dx * 4.0 + drift * 1.6,
                       sy + dy * 4.0, floor_z + 0.005))]
        objs.append(common.tube_curve("tz_string", pts, 0.008,
                                      mat=white, resolution=2))


def build():
    common.reset_scene()
    common.setup_render()
    cloth = tallit_cloth()
    white = tzitzit_white()
    silver = common.mat_silver()
    gold = common.mat_gold()

    objs = []
    # folded stack: three slabs + a slightly smaller top layer
    objs.append(add_slab("fold1", (1.50, 1.05, 0.10), (0.0, 0.0, 0.05), cloth))
    objs.append(add_slab("fold2", (1.50, 1.05, 0.10),
                         (0.02, -0.015, 0.145), cloth))
    objs.append(add_slab("fold3", (1.50, 1.05, 0.10),
                         (-0.015, 0.02, 0.24), cloth))
    objs.append(add_slab("fold_top", (1.46, 1.01, 0.05),
                         (0.005, 0.005, 0.315), cloth))

    # atarah collar band near the +X (neck) end, with gold pinstripes
    top_z = 0.315 + 0.025
    atarah = add_slab("atarah", (0.17, 1.00, 0.014), (0.50, 0.005, top_z),
                      None)
    common.assign(atarah, silver)
    for gx in (0.50 - 0.075, 0.50 + 0.075):
        pin = add_slab("atarah_pin", (0.012, 1.00, 0.016), (gx, 0.005, top_z),
                       None)
        common.assign(pin, gold)
        objs.append(pin)
    objs.append(atarah)

    # tzitzit at three visible corners
    tzitzit_bundle(objs, -0.74, -0.50, white)
    tzitzit_bundle(objs, -0.74, 0.50, white)
    tzitzit_bundle(objs, 0.74, -0.50, white)

    common.frame_object(objs, margin=0.1, focal=72.0,
                        elevation=26.0, azimuth=35.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render("tallit", common.OUT_DIR)


build()
