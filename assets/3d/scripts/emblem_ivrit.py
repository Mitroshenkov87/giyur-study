"""Emblem: the Hebrew letter aleph (ivrit = Hebrew)."""

import math
import os
import sys

import bpy
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import emblemkit

FONT = os.path.normpath(os.path.join(
    os.path.dirname(os.path.abspath(__file__)),
    "..", "..", "fonts", "NotoSerifHebrew-Regular.ttf"))

TARGET_H = 1.12   # ~70% of the inner-disc diameter, still inside the walnut ring


def _fit(obj):
    """Scale the glyph in its plane so it fills the field. Keep extrusion."""
    bpy.context.view_layer.update()
    bb = [obj.matrix_world @ Vector(c) for c in obj.bound_box]
    xs = [p.x for p in bb]
    zs = [p.z for p in bb]
    h = max(zs) - min(zs)
    w = max(xs) - min(xs)
    if h < 1e-4:
        return
    s = TARGET_H / h
    if w * s > TARGET_H * 1.05:
        s = (TARGET_H * 1.05) / w
    # Local X = width, local Y = height, local Z = extrusion toward camera.
    obj.scale = (s, s, 1.0)
    bpy.context.view_layer.update()
    bb = [obj.matrix_world @ Vector(c) for c in obj.bound_box]
    cx = sum(p.x for p in bb) / 8.0
    cz = sum(p.z for p in bb) / 8.0
    obj.location.x -= cx
    obj.location.z -= cz
    bpy.context.view_layer.update()


def build():
    common.reset_scene()
    common.setup_render()
    objs, metal, accent = emblemkit.medallion()

    font = bpy.data.fonts.load(FONT)
    cu = bpy.data.curves.new("aleph_font", "FONT")
    cu.font = font
    cu.body = "\u05D0"
    cu.size = 1.0
    cu.extrude = emblemkit.RELIEF_D
    cu.bevel_depth = 0.010
    cu.bevel_resolution = 2
    cu.resolution_u = 12
    cu.align_x = "CENTER"
    cu.align_y = "CENTER"
    aleph = bpy.data.objects.new("aleph", cu)
    bpy.context.collection.objects.link(aleph)
    common.assign(aleph, metal)
    # Text lies in local XY and extrudes along +Z. Rx(+90) stands it up
    # (local +Y -> world +Z) and points the extrusion at the camera (-Y).
    aleph.rotation_euler = (math.radians(90.0), 0.0, 0.0)
    aleph.location = (0.0, emblemkit.RELIEF_Y, 0.0)
    _fit(aleph)
    objs.append(aleph)

    emblemkit.finish(objs, "ivrit")


build()
