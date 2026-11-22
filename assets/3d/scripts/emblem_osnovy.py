"""Emblem: rolled Torah scroll between two rollers (osnovy = basics)."""

import os
import sys

import bpy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import emblemkit

ROLLER_X = 0.34
ROLLER_R = 0.082
ROLLER_H = 0.98


def text_columns(name, mat, y_front):
    """Two columns of thick raised bars — text at emblem scale."""
    verts, faces = [], []
    for col in range(2):
        x0 = -0.30 + col * 0.32
        for r in range(6):
            z = -0.36 + r * 0.135
            w = 0.26 if (r + col) % 2 == 0 else 0.18
            h = 0.046
            b = len(verts)
            x1 = x0 + w
            for dy in (0.0, -0.016):
                verts += [(x0, y_front + dy, z), (x1, y_front + dy, z),
                          (x1, y_front + dy, z + h), (x0, y_front + dy, z + h)]
            faces += [(b, b + 1, b + 2, b + 3),
                      (b + 7, b + 6, b + 5, b + 4),
                      (b, b + 4, b + 5, b + 1),
                      (b + 1, b + 5, b + 6, b + 2),
                      (b + 2, b + 6, b + 7, b + 3),
                      (b + 3, b + 7, b + 4, b)]
    mesh = bpy.data.meshes.new(name + "_mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    ob = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(ob)
    common.assign(ob, mat)
    return ob


def build():
    common.reset_scene()
    common.setup_render()
    objs, metal, accent = emblemkit.medallion()
    ry = emblemkit.R_Y
    klaf = emblemkit.parchment()
    dark = emblemkit.ink()

    # Parchment sits just behind the rollers so the rollers overlap its edges.
    sheet_front = emblemkit.RELIEF_Y - 0.02
    objs.append(emblemkit.extrude_polygon(
        "parchment",
        [(-0.46, -0.44), (0.46, -0.44), (0.46, 0.44), (-0.46, 0.44)],
        sheet_front, 0.045, klaf, bevel_width=0.012))
    objs.append(text_columns("text_rows", dark, sheet_front - 0.048))

    for side in (-1, 1):
        x = side * ROLLER_X
        rod = common.add_cylinder(
            "roller", ROLLER_R, ROLLER_H, location=(x, ry, 0.0),
            vertices=32, mat=metal)
        common.smooth(rod)
        objs.append(rod)
        for sz in (-1, 1):
            z = sz * (ROLLER_H * 0.5 + 0.01)
            cap = common.add_cylinder(
                "roller_disc", 0.118, 0.055, location=(x, ry, z),
                vertices=32, mat=metal)
            common.bevel(cap, 0.012, segments=2)
            objs.append(cap)
            objs.append(common.add_uv_sphere(
                "roller_knob", 0.055, location=(x, ry, z + sz * 0.055),
                mat=metal, segments=24, rings=12))
            # atzei chaim handle, along the roller
            handle = common.add_cylinder(
                "handle", 0.026, 0.10,
                location=(x, ry, z + sz * 0.13), vertices=16, mat=metal)
            common.smooth(handle)
            objs.append(handle)
        for z in (-0.22, 0.22):
            band = common.add_torus(
                "roller_band", ROLLER_R + 0.012, 0.016,
                location=(x, ry, z), mat=accent, major_segments=28,
                minor_segments=10)
            objs.append(band)

    emblemkit.finish(objs, "osnovy")


build()
