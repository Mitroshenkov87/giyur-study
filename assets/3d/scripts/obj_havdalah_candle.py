"""Havdalah candle: a 4-strand over-under plait of blue and white wax,
several wicks gathered into one torch flame, in a small brass holder."""

import math
import os
import sys

import bpy
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common

CANDLE_Z0 = 0.32
CANDLE_H = 1.36
STRAND_R = 0.050
LANE_PITCH = 0.112
LIFT = 0.078
N_CYCLES = 5
BLUE = (0.10, 0.20, 0.58, 1.0)
WHITE = (0.76, 0.72, 0.64, 1.0)
# Each cycle: left pair crosses (left strand over), right pair crosses
# (left of that pair over), then the middle pair crosses (right strand over).
# Every strand travels the full width. See the braid-word check in comments.
WORD = ((0, True), (2, True), (1, False))


def _braid_samples(n_cycles, n_per=7):
    """Samples of (lane, lift) for each of 4 strands. Lift > 0 is 'over'."""
    order = [0, 1, 2, 3]
    pos = [0.0, 1.0, 2.0, 3.0]
    samples = [[(pos[s], 0.0)] for s in range(4)]
    for _ in range(n_cycles):
        for i, over_left in WORD:
            a, b = order[i], order[i + 1]
            la, lb = pos[a], pos[b]
            for k in range(1, n_per + 1):
                af = 0.5 - 0.5 * math.cos(math.pi * k / n_per)
                ypeak = math.sin(math.pi * k / n_per)
                lift = [0.0, 0.0, 0.0, 0.0]
                npos = pos[:]
                npos[a] = la + (lb - la) * af
                npos[b] = lb + (la - lb) * af
                if over_left:
                    lift[a] = ypeak
                    lift[b] = -0.42 * ypeak
                else:
                    lift[b] = ypeak
                    lift[a] = -0.42 * ypeak
                for s in range(4):
                    samples[s].append((npos[s], lift[s]))
            order[i], order[i + 1] = b, a
            pos[a], pos[b] = lb, la
    return samples


def _pinch(lane, lift, u):
    """Gather the plait into the cup and into the wick bundle."""
    root = 0.0 if u > 0.055 else (0.055 - u) / 0.055
    tip = 0.0 if u < 0.90 else (u - 0.90) / 0.10
    root = root * root * (3.0 - 2.0 * root)
    tip = tip * tip * (3.0 - 2.0 * tip)
    p = max(root, tip)
    lane = lane + (1.5 - lane) * p
    lift *= 1.0 - p
    return lane, lift, p


def _frames(pts):
    tang = []
    n = len(pts)
    for i in range(n):
        d = pts[min(i + 1, n - 1)] - pts[max(i - 1, 0)]
        tang.append(d.normalized() if d.length > 1e-8 else Vector((0.0, 0.0, 1.0)))
    nrm = tang[0].cross(Vector((1.0, 0.0, 0.0)))
    if nrm.length < 1e-6:
        nrm = tang[0].cross(Vector((0.0, 1.0, 0.0)))
    nrm.normalize()
    normals = [nrm]
    for i in range(1, n):
        axis = tang[i - 1].cross(tang[i])
        if axis.length < 1e-8:
            nrm = normals[-1].copy()
        else:
            axis.normalize()
            ang = math.acos(max(-1.0, min(1.0, tang[i - 1].dot(tang[i]))))
            c, s = math.cos(ang), math.sin(ang)
            v = normals[-1]
            nrm = v * c + axis.cross(v) * s + axis * axis.dot(v) * (1.0 - c)
        nrm = (nrm - tang[i] * nrm.dot(tang[i])).normalized()
        normals.append(nrm)
    binorms = [tang[i].cross(normals[i]).normalized() for i in range(n)]
    return normals, binorms


def mesh_tube(name, points, radius, mat, sides=14):
    """Round tube. NURBS bevel bulges into spheres wherever a plait reverses."""
    pts = [Vector(p) for p in points]
    normals, binorms = _frames(pts)
    verts, faces = [], []
    for i, p in enumerate(pts):
        for j in range(sides):
            a = 2.0 * math.pi * j / sides
            verts.append(tuple(p + (normals[i] * math.cos(a)
                                   + binorms[i] * math.sin(a)) * radius))
    for i in range(len(pts) - 1):
        for j in range(sides):
            nj = (j + 1) % sides
            faces.append((i * sides + j, i * sides + nj,
                          (i + 1) * sides + nj, (i + 1) * sides + j))
    for i, sign in ((0, True), (len(pts) - 1, False)):
        ci = len(verts)
        verts.append(tuple(pts[i]))
        for j in range(sides):
            nj = (j + 1) % sides
            a, b = i * sides + j, i * sides + nj
            faces.append((ci, b, a) if sign else (ci, a, b))
    mesh = bpy.data.meshes.new(name + "_mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    ob = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(ob)
    if mat:
        common.assign(ob, mat)
    common.smooth(ob)
    return ob


def _set_emission(mat, strength):
    for node in mat.node_tree.nodes:
        if node.type == "BSDF_PRINCIPLED" and "Emission Strength" in node.inputs:
            node.inputs["Emission Strength"].default_value = strength


def build():
    common.reset_scene()
    common.setup_render()
    brass = common.mat_brass()
    wax_white = common.mat_wax(WHITE)
    wax_blue = common.mat_wax(BLUE)
    flame_mat = common.mat_flame((1.0, 0.12, 0.0, 1.0))
    _set_emission(flame_mat, 2.8)
    core_mat = common.mat_flame((1.0, 0.45, 0.05, 1.0))
    _set_emission(core_mat, 6.0)
    wick_mat = common.mat_leather((0.02, 0.015, 0.01, 1.0))

    objs = []
    dish_prof = [
        (0.0, 0.00), (0.28, 0.00), (0.46, 0.02), (0.50, 0.055),
        (0.48, 0.09), (0.40, 0.11), (0.30, 0.10), (0.12, 0.055),
        (0.0, 0.05),
    ]
    dish = common.lathe("holder_dish", dish_prof, segments=64, mat=brass)
    common.bevel(dish, 0.008, segments=2)
    objs.append(dish)
    cup = common.add_cone("holder_cup", 0.20, 0.155, 0.28,
                          location=(0, 0, 0.22), vertices=48, mat=brass)
    common.bevel(cup, 0.012, segments=2)
    objs.append(cup)
    objs.append(common.add_torus("holder_ring", 0.175, 0.012,
                                 location=(0, 0, 0.34), mat=brass))

    samples = _braid_samples(N_CYCLES)
    n = len(samples[0])
    ends = []
    for s in range(4):
        pts = []
        for j in range(n):
            u = j / (n - 1)
            lane, lift = samples[s][j]
            lane, lift, _pinch_amt = _pinch(lane, lift, u)
            # Wide axis of the plait faces the camera (camera sits toward -Y).
            x = (lane - 1.5) * LANE_PITCH
            y = -lift * LIFT
            z = CANDLE_Z0 + CANDLE_H * u
            pts.append(Vector((x, y, z)))
        mat = wax_blue if s % 2 == 0 else wax_white
        objs.append(mesh_tube("strand%d" % s, pts, STRAND_R, mat))
        ends.append(pts[-1])

    top = max(p.z for p in ends)
    meet = Vector((0.0, 0.0, top + 0.04))
    for end in ends:
        mid = (end + meet) * 0.5
        mid.z += 0.01
        objs.append(common.tube_curve(
            "wick", [end + Vector((0.0, 0.0, 0.012)), mid, meet],
            0.0065, mat=wick_mat, resolution=2))
    # One torch: a lathed teardrop, orange outside, yellow core toward camera.
    flame = common.lathe("flame", [
        (0.000, 0.00), (0.028, 0.02), (0.048, 0.07), (0.052, 0.13),
        (0.040, 0.21), (0.022, 0.30), (0.008, 0.38), (0.000, 0.44),
    ], segments=32, mat=flame_mat)
    flame.location = (0.0, -0.006, meet.z + 0.01)
    objs.append(flame)
    core = common.lathe("flame_core", [
        (0.000, 0.02), (0.016, 0.05), (0.020, 0.10), (0.012, 0.18), (0.000, 0.26),
    ], segments=24, mat=core_mat)
    core.location = (0.0, -0.012, meet.z + 0.03)
    objs.append(core)

    common.frame_object(objs, margin=0.12, focal=72.0, elevation=18.0, azimuth=28.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render("havdalah_candle", common.OUT_DIR)


build()
