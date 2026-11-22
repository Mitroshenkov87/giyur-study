"""Pirkei Avot: a thick open book (curved page spreads, visible page block,
dark leather covers) resting on a walnut table-top shtender with a ledge."""

import math
import os
import random
import sys

import bpy
from mathutils import Matrix, Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import objkit

PW = 0.88          # page width (gutter to fore-edge)
PD = 0.62          # page half-height (along the spine)
COVER_T = 0.035
TILT = math.radians(24.0)
DESK_W, DESK_L, DESK_T = 2.25, 1.75, 0.06


def page_top(x):
    """Height of the page surface above the cover (open book curve)."""
    u = min(abs(x) / PW, 1.0)
    return 0.025 + 0.20 * math.sin(u * math.pi * 0.5) ** 0.55 - 0.05 * u ** 3


def mat_page_edge():
    """Page block side: cream with fine horizontal page lines."""
    m, nt, bs = common._new_material("PageEdge")
    tc = common._texcoord(nt)
    wave = nt.nodes.new("ShaderNodeTexWave")
    wave.bands_direction = "Z"
    wave.inputs["Scale"].default_value = 90.0
    wave.inputs["Distortion"].default_value = 1.5
    nt.links.new(tc.outputs["Object"], wave.inputs["Vector"])
    ramp = common._ramp(nt, [(0.0, (0.42, 0.34, 0.22, 1.0)),
                             (1.0, (0.70, 0.62, 0.46, 1.0))])
    nt.links.new(wave.outputs["Fac"], ramp.inputs["Fac"])
    nt.links.new(ramp.outputs["Color"], bs.inputs["Base Color"])
    common._bump(nt, bs, wave.outputs["Fac"], strength=0.2, distance=0.004)
    common._inp(bs, "Roughness", 0.8)
    return m


def mat_paper():
    m, nt, bs = common._new_material("Paper")
    tc = common._texcoord(nt)
    n = common._noise(nt, scale=3.0, detail=3.0)
    nt.links.new(tc.outputs["Object"], n.inputs["Vector"])
    ramp = common._ramp(nt, [(0.0, (0.58, 0.48, 0.32, 1.0)),
                             (1.0, (0.70, 0.61, 0.44, 1.0))])
    nt.links.new(n.outputs["Fac"], ramp.inputs["Fac"])
    nt.links.new(ramp.outputs["Color"], bs.inputs["Base Color"])
    f = common._noise(nt, scale=80.0, detail=2.0, y=-320)
    nt.links.new(tc.outputs["Object"], f.inputs["Vector"])
    common._bump(nt, bs, f.outputs["Fac"], strength=0.06, distance=0.01)
    common._inp(bs, "Roughness", 0.75)
    return m


def page_block(name, side, top_mat, edge_mat, nu=36, nv=10):
    """Block between z=0 (on cover) and page_top(x); x from gutter to fore-edge."""
    verts, faces, fmat = [], [], []
    xs = [side * PW * i / nu for i in range(nu + 1)]
    ys = [-PD + 2 * PD * j / nv for j in range(nv + 1)]
    for x in xs:
        for y in ys:
            verts.append((x, y, page_top(x)))
    off = len(verts)
    for x in xs:
        for y in ys:
            verts.append((x, y, 0.0))
    W = nv + 1
    for i in range(nu):
        for j in range(nv):
            a, b = i * W + j, (i + 1) * W + j
            f = (a, b, b + 1, a + 1)
            faces.append(f if side > 0 else f[::-1]); fmat.append(0)
            fb = (off + a + 1, off + b + 1, off + b, off + a)
            faces.append(fb if side > 0 else fb[::-1]); fmat.append(1)
    for j in range(nv):                    # fore-edge
        a, b = nu * W + j, nu * W + j + 1
        f = (a, b, off + b, off + a)
        faces.append(f if side > 0 else f[::-1]); fmat.append(1)
    for i in range(nu):                    # head / tail edges
        a, b = i * W, (i + 1) * W
        f = (a, b, off + b, off + a)
        faces.append(f[::-1] if side > 0 else f); fmat.append(1)
        a, b = i * W + nv, (i + 1) * W + nv
        f = (a, off + a, off + b, b)
        faces.append(f[::-1] if side > 0 else f); fmat.append(1)
    mesh = bpy.data.meshes.new(name + "_mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    ob = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(ob)
    ob.data.materials.append(top_mat)
    ob.data.materials.append(edge_mat)
    for p, mi in zip(mesh.polygons, fmat):
        p.material_index = mi
        p.use_smooth = True
    return ob


def text_rows(mat):
    rng = random.Random(4)
    boxes = []
    e = 0.004
    for side in (-1, 1):
        x_lo, x_hi = 0.14 * PW, 0.88 * PW
        # chapter heading: one bold short row
        for row in range(21):
            y = PD * 0.82 - row * 0.060
            if row == 0:
                segs = [(0.40 * PW, 0.64 * PW, 0.026)]
            else:
                segs = []
                pos = x_hi
                stop = x_lo + (rng.uniform(0.0, 0.3) if row in (7, 15) else 0.0)
                while pos - stop > 0.04:
                    w = rng.uniform(0.03, min(0.09, pos - stop))
                    segs.append((pos - w, pos - 0.008, 0.013))
                    pos -= w + rng.uniform(0.006, 0.012)
            for a, b, hgt in segs:
                xc = side * (a + b) * 0.5
                slope = (page_top(xc + e) - page_top(xc - e)) / (2 * e)
                boxes.append((xc, y, page_top(xc) + 0.003, b - a, hgt, 0.004,
                              -math.atan(slope)))
    return objkit.add_boxes("text_rows", boxes, mat)


def side_panel(name, x, mat):
    """Trapezoid side board of the shtender (YZ profile, extruded in X)."""
    y0, y1 = -DESK_L * 0.5 * math.cos(TILT), DESK_L * 0.5 * math.cos(TILT)
    z_front = 0.16
    z_back = z_front + DESK_L * math.sin(TILT)
    prof = [(y0 + 0.04, 0.0), (y1 - 0.02, 0.0), (y1 - 0.02, z_back - 0.02),
            (y0 + 0.04, z_front - 0.02)]
    t = 0.07
    verts = [(x - t / 2, y, z) for y, z in prof] + [(x + t / 2, y, z) for y, z in prof]
    faces = [(0, 1, 2, 3), (7, 6, 5, 4), (0, 4, 5, 1), (1, 5, 6, 2), (2, 6, 7, 3), (3, 7, 4, 0)]
    mesh = bpy.data.meshes.new(name + "_mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    ob = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(ob)
    ob.data.materials.append(mat)
    common.bevel(ob, 0.015, 2)
    return ob


def build():
    common.reset_scene()
    common.setup_render()
    leather = common.mat_leather((0.07, 0.035, 0.018, 1.0))
    paper = mat_paper()
    edge = mat_page_edge()
    ink = objkit.mat_ink((0.05, 0.025, 0.012, 1.0))
    gold = common.mat_gold()
    walnut = objkit.mat_dark_walnut()

    # ---- book (local: gutter along Y at x=0, covers at z<0)
    book = []
    for side in (-1, 1):
        book.append(page_block("pages", side, paper, edge))
        cv = objkit.cube("cover", (PW + 0.07, 2 * PD + 0.10, COVER_T),
                         (side * (PW + 0.07) / 2, 0, -COVER_T / 2), leather,
                         bevel=0.012)
        book.append(cv)
        # gold tooled line around the cover edge (visible rim)
        for yy in (-(PD + 0.035), PD + 0.035):
            book.append(objkit.cube("tool", (PW - 0.05, 0.012, 0.006),
                                    (side * (PW / 2 + 0.03), yy, 0.001), gold))
    spine = common.add_cylinder("spine", 0.06, 2 * PD + 0.10,
                                location=(0, 0, -0.03), mat=leather, vertices=32)
    spine.rotation_euler = (math.radians(90), 0, 0)
    common.smooth(spine)
    book.append(spine)
    book.append(text_rows(ink))
    # ribbon bookmark lying in the gutter, hanging over the bottom
    rib = common.tube_curve("ribbon", [(0.01, PD * 0.4, page_top(0.01) + 0.004),
                                       (0.02, -PD, 0.03), (0.04, -PD - 0.08, -0.08),
                                       (0.05, -PD - 0.10, -0.22)], 0.012,
                            mat=common.mat_velvet((0.25, 0.012, 0.03, 1.0)))
    rib.data.bevel_resolution = 0
    book.append(rib)

    # ---- shtender
    stand = []
    rot = Matrix.Rotation(TILT, 4, "X")
    z_front = 0.16
    mid = Vector((0, 0, z_front + 0.5 * DESK_L * math.sin(TILT)))
    desk = objkit.cube("desk", (DESK_W, DESK_L, DESK_T), (0, 0, 0), walnut, bevel=0.02)
    desk.matrix_world = Matrix.Translation(mid) @ rot @ Matrix.Translation((0, 0, DESK_T / 2))
    stand.append(desk)
    ledge = objkit.cube("ledge", (DESK_W, 0.07, 0.15), (0, 0, 0), walnut, bevel=0.02)
    ledge.matrix_world = (Matrix.Translation(mid) @ rot
                          @ Matrix.Translation((0, -DESK_L / 2 + 0.035, DESK_T + 0.03)))
    stand.append(ledge)
    for sx in (-1, 1):
        stand.append(side_panel("side", sx * (DESK_W / 2 - 0.12), walnut))
    stand.append(objkit.cube("back_rail", (DESK_W - 0.24, 0.06, 0.20),
                             (0, DESK_L * 0.5 * math.cos(TILT) - 0.06, 0.10),
                             walnut, bevel=0.015))

    # place the book: on the desk surface, bottom edge against the ledge
    s = -DESK_L / 2 + 0.07 + PD + 0.07
    place = (Matrix.Translation(mid) @ rot
             @ Matrix.Translation((0, s, DESK_T + COVER_T + 0.004)))
    for ob in book:
        ob.matrix_world = place @ ob.matrix_world
    objs = stand + book

    common.frame_object(objs, margin=0.06, elevation=34.0, azimuth=22.0,
                        focal=72.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render("pirkei_avot", common.OUT_DIR)


build()
