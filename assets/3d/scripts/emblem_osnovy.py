"""Emblem: rolled Torah scroll between two rollers (osnovy = basics)."""

import os
import sys

import bpy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import emblemkit

ROLLER_X = 0.33
ROLLER_R = 0.075
PARCH_W = 0.26
PARCH_H = 0.44


def dash_rows(name, mat, y_top, rows=6, per_row=5):
    """One mesh of small raised dashes reading as text lines."""
    verts, faces = [], []
    rng = [0.068, 0.048, 0.060, 0.044, 0.066, 0.052]
    for r in range(rows):
        z = -0.315 + r * 0.126
        x = -0.205
        for i in range(per_row):
            w = rng[(r * 2 + i) % len(rng)]
            h = 0.026
            if x + w > 0.21:
                break
            b = len(verts)
            for dy in (0.0, -0.02):
                verts += [(x, y_top + dy, z - h / 2), (x + w, y_top + dy, z - h / 2),
                          (x + w, y_top + dy, z + h / 2), (x, y_top + dy, z + h / 2)]
            faces += [(b, b + 1, b + 2, b + 3),
                      (b + 7, b + 6, b + 5, b + 4),
                      (b, b + 4, b + 5, b + 1),
                      (b + 1, b + 5, b + 6, b + 2),
                      (b + 2, b + 6, b + 7, b + 3),
                      (b + 3, b + 7, b + 4, b)]
            x += w + 0.028
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

    for side in (-1, 1):
        x = side * ROLLER_X
        rod = common.add_cylinder("roller", ROLLER_R, 1.10,
                                  location=(x, ry, 0), vertices=32, mat=metal)
        common.smooth(rod)
        objs.append(rod)
        for sz in (-1, 1):
            # rounded end knob + small handle pointing at the viewer
            objs.append(common.add_uv_sphere(
                "roller_cap", 0.095, location=(x, ry, sz * 0.56),
                scale=(1.0, 0.75, 1.0), mat=metal, segments=32, rings=16))
            handle = common.add_cylinder("handle", 0.032, 0.07,
                                         location=(x, ry - 0.085, sz * 0.60),
                                         vertices=24, mat=metal)
            handle.rotation_euler = (1.5707963, 0, 0)  # axis along Y
            objs.append(handle)
            objs.append(common.add_uv_sphere(
                "handle_tip", 0.045, location=(x, ry - 0.13, sz * 0.60),
                mat=metal, segments=24, rings=12))
            objs.append(common.add_torus(
                "roller_band", ROLLER_R + 0.008, 0.014,
                location=(x, ry, sz * 0.42), mat=accent, major_segments=32))

    objs.append(emblemkit.extrude_polygon(
        "parchment",
        [(-PARCH_W, -PARCH_H), (PARCH_W, -PARCH_H),
         (PARCH_W, PARCH_H), (-PARCH_W, PARCH_H)],
        emblemkit.RELIEF_Y, 0.05, metal, bevel_width=0.02))
    objs.append(dash_rows("text_rows", accent, emblemkit.RELIEF_Y - 0.05))

    emblemkit.finish(objs, "osnovy")


build()
