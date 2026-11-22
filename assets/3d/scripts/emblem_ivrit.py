"""Emblem: the Hebrew letter aleph (ivrit = Hebrew)."""

import math
import os
import sys

import bpy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import emblemkit

FONT = os.path.join(os.path.dirname(os.path.abspath(__file__)),
                    "..", "..", "fonts", "NotoSerifHebrew-Regular.ttf")


def build():
    common.reset_scene()
    common.setup_render()
    objs, metal, accent = emblemkit.medallion()

    font = bpy.data.fonts.load(FONT)
    cu = bpy.data.curves.new("aleph_font", "FONT")
    cu.font = font
    cu.body = "\u05D0"
    cu.size = 1.45
    cu.extrude = emblemkit.RELIEF_D
    cu.bevel_depth = 0.012
    cu.bevel_resolution = 3
    cu.align_x = "CENTER"
    cu.align_y = "CENTER"
    aleph = bpy.data.objects.new("aleph", cu)
    bpy.context.collection.objects.link(aleph)
    common.assign(aleph, metal)
    # text lies in the local XY plane extruded along +Z; stand it in XZ
    # with the extrusion pointing at the camera (-Y)
    aleph.rotation_euler = (math.radians(90), 0, 0)
    aleph.location = (0, emblemkit.RELIEF_Y, 0.02)
    objs.append(aleph)

    emblemkit.finish(objs, "ivrit")


build()
