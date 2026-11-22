"""Pirkei Avot: thick open book with two curved page spreads and text rows,
dark leather covers with gold spine bands, resting on a small wooden
reading stand (shtender) tilted toward the viewer."""

import math
import os
import random
import sys

import bpy
from mathutils import Matrix, Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import objkit

PW = 1.02          # page half-width (spine to fore-edge)
PD = 0.68          # page half-depth
PAGE_T = 0.18      # page block thickness
COVER_LIFT = 0.185
COVER_T = 0.045
TILT = math.radians(20.0)


def page_top(x):
    u = abs(x) / PW
    return 0.02 + 0.10 * u * u - 0.025 * math.exp(-((abs(x)) / 0.05) ** 2)


def curved_slab(name, x0, x1, yh, top_fn, thick, mat, nu=28, nv=12):
    verts, faces = [], []
    for i in range(nu + 1):
        x = x0 + (x1 - x0) * i / nu
        for j in range(nv + 1):
            y = -yh + 2.0 * yh * j / nv
            verts.append((x, y, top_fn(x)))
    off = len(verts)
    for i in range(nu + 1):
        x = x0 + (x1 - x0) * i / nu
        for j in range(nv + 1):
            y = -yh + 2.0 * yh * j / nv
            verts.append((x, y, top_fn(x) - thick))
    for i in range(nu):
        for j in range(nv):
            a = i * (nv + 1) + j
            b = (i + 1) * (nv + 1) + j
            faces.append((a, b, b + 1, a + 1))
            faces.append((off + a + 1, off + b + 1, off + b, off + a))
    for j in range(nv):        # rims at x0 / x1
        a, b = j, j + 1
        faces.append((a, off + a, off + b, b))
        a, b = nu * (nv + 1) + j, nu * (nv + 1) + j + 1
        faces.append((a, b, off + b, off + a))
    for i in range(nu):        # rims at +/- y
        a, b = i * (nv + 1), (i + 1) * (nv + 1)
        faces.append((a, b, off + b, off + a))
        a, b = i * (nv + 1) + nv, (i + 1) * (nv + 1) + nv
        faces.append((a, off + a, off + b, b))
    mesh = bpy.data.meshes.new(name + "_mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    ob = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(ob)
    common.assign(ob, mat)
    common.smooth(ob)
    return ob


def text_rows(mat):
    random.seed(4)
    boxes = []
    e = 0.005
    for side in (-1, 1):
        for col in (0.31, 0.71):
            x0 = side * col - 0.165
            for row in range(11):
                y = -0.50 + row * 0.10
                pos = 0.0
                while 0.33 - pos > 0.05:
                    w = random.uniform(0.05, min(0.105, 0.33 - pos))
                    xc = x0 + pos + w / 2.0
                    slope = (page_top(xc + e) - page_top(xc - e)) / (2.0 * e)
                    boxes.append((xc, y, page_top(xc) + 0.006,
                                  w - 0.022, 0.022, 0.008,
                                  -math.atan(slope)))
                    pos += w + random.uniform(0.026, 0.042)
    return objkit.add_boxes("text_rows", boxes, mat)


def build():
    common.reset_scene()
    common.setup_render()
    leather = common.mat_leather()
    parchment = common.mat_parchment()
    ink = objkit.mat_ink()
    gold = common.mat_gold()
    wood = common.mat_light_wood()
    book_objs = []

    # pages and covers, gutter dip at the spine
    for x0, x1, c0, c1 in ((0.0, PW, -0.02, PW + 0.08),
                           (-PW, 0.0, -PW - 0.08, 0.02)):
        book_objs.append(curved_slab("pages", x0, x1, PD, page_top, PAGE_T,
                                     parchment))
        cover_top = lambda x: page_top(x) - COVER_LIFT
        book_objs.append(curved_slab("cover", c0, c1, PD + 0.08, cover_top,
                                     COVER_T, leather))
    book_objs.append(text_rows(ink))

    # rounded leather spine + gold bands
    spine = common.add_cylinder("spine", 0.115, 2 * PD + 0.16,
                                location=(-0.02, 0, -0.135), mat=leather)
    spine.rotation_euler = (math.radians(90.0), 0.0, 0.0)
    common.smooth(spine)
    book_objs.append(spine)
    for y in (-0.48, 0.0, 0.48):
        band = common.add_torus("spine_band", 0.117, 0.018,
                                location=(-0.02, y, -0.135), mat=gold)
        band.rotation_euler = (math.radians(90.0), 0.0, 0.0)
        book_objs.append(band)

    # shtender: base, column, slanted desk, front lip
    desk_c = Vector((0.0, 0.2, 0.9))
    n = Vector((0.0, -math.sin(TILT), math.cos(TILT)))
    objs = [
        objkit.cube("base", (1.5, 1.2, 0.10), (0, 0.215, 0.05), wood, bevel=0.03),
        objkit.cube("column", (0.34, 0.34, 0.76), (0, 0.215, 0.48), wood, bevel=0.02),
    ]
    desk = objkit.cube("desk", (1.9, 3.0, 0.09), desk_c, wood, bevel=0.03,
                       rot=(TILT, 0, 0))
    objs.append(desk)
    lip = objkit.cube("lip", (1.9, 0.07, 0.16), (0, 0, 0), wood, bevel=0.015)
    lip.matrix_world = desk.matrix_world @ Matrix.Translation((0, -1.505, 0.03))
    objs.append(lip)

    # place the book on the desk: spine along X, front half against the lip
    c = desk_c + n * (0.045 + 0.19)
    place = (Matrix.Translation(c) @ Matrix.Rotation(TILT, 4, "X")
             @ Matrix.Translation((0, 0.55, 0)) @ Matrix.Rotation(math.radians(90.0), 4, "Z"))
    for ob in book_objs:
        ob.matrix_world = place @ ob.matrix_world
    objs += book_objs

    common.frame_object(objs, margin=0.1, elevation=30.0, azimuth=35.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render("pirkei_avot", common.OUT_DIR)


build()
