"""Decor: L-shaped corner bracket, darker brass, engraved scroll (both themes)."""

import math
import os
import sys

import bpy
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import emblemkit

L = 0.95
W = 0.30
# Front face of the bracket (toward the camera) after extrusion.
FRONT_Y = -0.14


def _bool_cut(target, cutter):
    bpy.context.view_layer.objects.active = target
    target.select_set(True)
    mod = target.modifiers.new("engrave", "BOOLEAN")
    mod.operation = "DIFFERENCE"
    mod.object = cutter
    for solver in ("FLOAT", "EXACT", "MANIFOLD"):
        try:
            mod.solver = solver
            break
        except (TypeError, AttributeError):
            continue
    ok = False
    try:
        bpy.ops.object.modifier_apply(modifier=mod.name)
        ok = True
    except RuntimeError:
        pass
    target.select_set(False)
    if ok:
        cutter.hide_render = True
        cutter.hide_set(True)
    return ok


def build():
    common.reset_scene()
    common.setup_render()
    # Dark worked brass. Flat faces at dark=0 blow out to pale gold.
    metal = emblemkit.medal_metal(1.0)
    groove_mat = emblemkit.medal_metal(1.0)
    # Near-black bronze so the curl reads as an engraving, not a highlight.
    groove_ink, _nt, gbsdf = emblemkit._bsdf("Engraved")
    emblemkit._set(gbsdf, "Base Color", (0.035, 0.018, 0.008, 1.0))
    emblemkit._set(gbsdf, "Metallic", 1.0)
    emblemkit._set(gbsdf, "Roughness", 0.45)
    objs = []

    outline = [(0, 0), (L, 0), (L, W), (W, W), (W, L), (0, L)]
    n = len(outline)
    y_base, depth = 0.04, 0.18
    verts = [(x, y_base, z) for x, z in outline] + [
        (x, y_base - depth, z) for x, z in outline]
    faces = [tuple(range(n - 1, -1, -1)), tuple(range(n, 2 * n))]
    for i in range(n):
        j = (i + 1) % n
        faces.append((i, j, n + j, n + i))
    mesh = bpy.data.meshes.new("bracket_mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    bracket = bpy.data.objects.new("bracket", mesh)
    bpy.context.collection.objects.link(bracket)
    common.assign(bracket, metal)
    common.bevel(bracket, 0.045, segments=3)
    objs.append(bracket)

    # Spiral cutter sunk through the front face so the curl is a groove.
    turns, steps = 2.15, 64
    spiral = []
    for i in range(steps):
        t = i / (steps - 1)
        a = 2.0 * math.pi * turns * t + 0.5
        r = 0.028 + 0.145 * t
        spiral.append(Vector((0.22 + r * math.cos(a), FRONT_Y + 0.018,
                              0.22 + r * math.sin(a))))
    curl = common.tube_curve("scroll_curl", spiral, 0.026, mat=groove_mat,
                             resolution=3)
    bpy.context.view_layer.objects.active = curl
    curl.select_set(True)
    bpy.ops.object.convert(target="MESH")
    curl.select_set(False)
    cut_ok = _bool_cut(bracket, curl)
    if not cut_ok:
        # Fallback: a dark raised inlay, still reads as the engraved curl.
        curl.hide_render = False
        curl.hide_set(False)
        curl.location.y -= 0.02
        objs.append(curl)
    else:
        # A darker filament in the bottom of the groove so the line stays
        # visible under a frontal light, not only as a shadow.
        inner = []
        for i in range(steps):
            t = i / (steps - 1)
            a = 2.0 * math.pi * turns * t + 0.5
            r = 0.028 + 0.145 * t
            # Sit just proud of the face so the dark curl survives a frontal light.
            inner.append(Vector((0.22 + r * math.cos(a), FRONT_Y - 0.010,
                                 0.22 + r * math.sin(a))))
        wire = common.tube_curve("scroll_ink", inner, 0.013, mat=groove_ink,
                                 resolution=2)
        objs.append(wire)

    # Screw heads near the arm ends — dark cups, not pale studs.
    dark = emblemkit.ink()
    for x, z in ((L - 0.16, W * 0.50), (W * 0.50, L - 0.16)):
        hole = common.add_cylinder(
            "screw", 0.038, 0.04, location=(x, FRONT_Y + 0.012, z),
            vertices=24, mat=dark)
        hole.rotation_euler = (math.radians(90), 0, 0)
        objs.append(hole)
        pin = common.add_cylinder(
            "screw_slot", 0.012, 0.055, location=(x, FRONT_Y - 0.01, z),
            vertices=12, mat=metal)
        pin.rotation_euler = (math.radians(90), 0, 0)
        objs.append(pin)

    common.frame_object(objs, margin=0.10, focal=75.0,
                        elevation=22.0, azimuth=32.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render(common.variant_name("corner"), common.OUT_DIR)


build()
