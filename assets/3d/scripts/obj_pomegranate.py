"""Pomegranate: a whole deep-crimson fruit with its crown (calyx), plus a
half whose cut face is turned to the camera: packed glossy ruby arils
separated by cream pith membranes."""

import math
import os
import random
import sys

import bmesh
import bpy
from mathutils import Matrix, Vector, noise

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import foodkit

R = 0.50


def skin_mat():
    """Leathery crimson with a brownish blush."""
    m, nt, bs = common._new_material("PomSkin")
    L = nt.links
    tc = common._texcoord(nt)
    n = common._noise(nt, scale=2.2, detail=4.0, roughness=0.55)
    L.new(tc.outputs["Object"], n.inputs["Vector"])
    ramp = common._ramp(nt, [(0.0, (0.075, 0.004, 0.008, 1.0)),
                             (0.40, (0.15, 0.006, 0.014, 1.0)),
                             (0.60, (0.19, 0.012, 0.016, 1.0)),
                             (0.80, (0.17, 0.04, 0.018, 1.0)),
                             (1.0, (0.14, 0.06, 0.02, 1.0))])
    L.new(n.outputs["Fac"], ramp.inputs["Fac"])
    L.new(ramp.outputs["Color"], bs.inputs["Base Color"])
    fine = common._noise(nt, scale=45.0, detail=3.0, y=-320)
    L.new(tc.outputs["Object"], fine.inputs["Vector"])
    common._bump(nt, bs, fine.outputs["Fac"], strength=0.12, distance=0.01)
    common._inp(bs, "Roughness", 0.42)
    common._inp(bs, "Coat Weight", 0.30)
    common._inp(bs, "Coat Roughness", 0.30)
    return m


def calyx_mat():
    return foodkit.prin("Calyx", (0.13, 0.035, 0.02, 1.0), rough=0.7,
                        ramp=[(0.0, (0.10, 0.03, 0.02, 1.0)),
                              (1.0, (0.22, 0.07, 0.03, 1.0))],
                        ramp_scale=6.0, bump=0.2, bump_scale=40.0)


def aril_mat():
    m, nt, bs = common._new_material("Aril")
    common._inp(bs, "Base Color", (0.36, 0.004, 0.025, 1.0))
    common._inp(bs, "Roughness", 0.10)
    common._inp(bs, "Transmission Weight", 0.45)
    common._inp(bs, "IOR", 1.36)
    common._inp(bs, "Subsurface Weight", 0.35)
    common._inp(bs, "Subsurface Radius", (1.0, 0.08, 0.06))
    common._inp(bs, "Coat Weight", 0.7)
    common._inp(bs, "Coat Roughness", 0.04)
    return m


def pith_mat():
    return foodkit.prin("Pith", (0.62, 0.50, 0.32, 1.0), rough=0.75,
                        subsurface=0.1,
                        ramp=[(0.0, (0.70, 0.55, 0.30, 1.0)),
                              (0.6, (0.64, 0.48, 0.24, 1.0)),
                              (1.0, (0.60, 0.38, 0.20, 1.0))],
                        ramp_scale=8.0, bump=0.25, bump_scale=60.0)


def fruit_radius(d):
    """Slightly hexagonal, lumpy fruit."""
    a = math.atan2(d.y, d.x)
    return R * (1.0 + 0.025 * math.cos(6 * a) * (1 - abs(d.z))
                + 0.03 * noise.noise(d * 1.8 + Vector((3, 1, 2))))


def whole_fruit(skin, loc):
    ob = common.add_uv_sphere("pom_whole", 1.0, location=loc, mat=skin,
                              segments=96, rings=48)
    for v in ob.data.vertices:
        d = v.co.normalized()
        r = fruit_radius(d)
        v.co = d * r
        v.co.z *= 0.92
        if v.co.z > 0.38:                       # shoulder dips to the crown
            v.co.z -= 0.12 * (v.co.z - 0.38)
    return ob


def crown(calyx, top):
    """Calyx: short neck with 6 pointed sepals flaring outwards."""
    objs = []
    neck = common.lathe("calyx_neck",
                        [(0.0, -0.05), (0.10, -0.05), (0.085, 0.02),
                         (0.080, 0.08), (0.092, 0.11), (0.06, 0.105),
                         (0.0, 0.07)], segments=32, mat=calyx)
    neck.location = top
    objs.append(neck)
    from mathutils import Quaternion
    for i in range(6):
        a = i * math.pi / 3.0 + 0.2
        tilt = math.radians(32 + 8 * math.sin(i * 2.3))
        sep = common.add_cone("sepal", 0.040, 0.003, 0.13, vertices=10, mat=calyx)
        sep.scale = (1.0, 0.55, 1.0)
        axis = Vector((-math.sin(a), math.cos(a), 0.0))
        q = Quaternion(axis, tilt) @ Quaternion((0, 0, 1), a + math.pi / 2)
        sep.rotation_mode = "QUATERNION"
        sep.rotation_quaternion = q
        base = Vector(top) + Vector((0.075 * math.cos(a), 0.075 * math.sin(a), 0.10))
        sep.location = base + q @ Vector((0, 0, 0.065))
        common.smooth(sep)
        objs.append(sep)
    return objs


def batch_spheres(name, items, mat, seg=14, rings=9):
    """items: (center, radius, scale_vec, rot_z). One mesh, many arils."""
    bm = bmesh.new()
    for c, r, sc, rz in items:
        mtx = (Matrix.Translation(c) @ Matrix.Rotation(rz, 4, "Z")
               @ Matrix.Diagonal((sc[0], sc[1], sc[2], 1.0)))
        bmesh.ops.create_uvsphere(bm, u_segments=seg, v_segments=rings,
                                  radius=r, matrix=mtx)
    mesh = bpy.data.meshes.new(name + "_mesh")
    bm.to_mesh(mesh)
    bm.free()
    ob = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(ob)
    ob.data.materials.append(mat)
    common.smooth(ob)
    return ob


def half_fruit(skin, aril, pith):
    rng = random.Random(7)
    objs = []
    prof = []
    for k in range(17):
        t = -math.pi / 2 + (math.pi / 2) * k / 16
        prof.append((R * math.cos(t), 0.92 * R * math.sin(t)))
    prof[0] = (0.0, prof[0][1])
    prof += [(0.965 * R, 0.006), (0.965 * R, -0.03), (0.0, -0.03)]
    shell = common.lathe("pom_half", prof, segments=96, mat=skin)
    objs.append(shell)
    # cream rind / pith base of the cut face
    disc = common.add_cylinder("pom_rind", 0.968 * R, 0.014,
                               location=(0, 0, 0.0), vertices=96, mat=pith)
    objs.append(disc)
    # membranes: radial walls + a central core
    walls = []
    for i in range(6):
        a = i * math.pi / 3 + rng.uniform(-0.2, 0.2)
        walls.append(a)
        ln = 0.80 * R
        w = foodkit.rounded_box("membrane", (ln, 0.016, 0.050),
                                (0.5 * ln * math.cos(a), 0.5 * ln * math.sin(a), 0.026),
                                pith, bevel_w=0.009, segments=2)
        w.rotation_euler.z = a
        objs.append(w)
    core = common.add_uv_sphere("membrane_core", 0.06, location=(0, 0, 0.02),
                                scale=(1, 1, 0.5), mat=pith, segments=20, rings=10)
    objs.append(core)
    # pack arils (Poisson-ish) inside chambers, avoiding the walls
    pts = []
    sp = 0.074
    rows = int(R / (sp * 0.866)) + 2
    for j in range(-rows, rows + 1):
        for i in range(-rows, rows + 1):
            p = Vector(((i + 0.5 * (j % 2)) * sp + rng.uniform(-0.010, 0.010),
                        j * sp * 0.866 + rng.uniform(-0.010, 0.010), 0.0))
            r = p.length
            if r < 0.08 or r > 0.90 * R:
                continue
            near_wall = False
            for wa in walls:
                d = abs(-math.sin(wa) * p.x + math.cos(wa) * p.y)
                along = math.cos(wa) * p.x + math.sin(wa) * p.y
                if d < 0.030 and 0 < along < 0.82 * R:
                    near_wall = True
                    break
            if not near_wall:
                pts.append(p)
    items = []
    for p in pts:
        rr = rng.uniform(0.047, 0.054)
        dome = 0.025 * (1 - (p.length / R) ** 2)
        c = Vector((p.x, p.y, 0.022 + dome + rng.uniform(-0.006, 0.006)))
        items.append((c, rr, (1.0, rng.uniform(0.75, 0.9), 1.05), rng.uniform(0, 6.3)))
    objs.append(batch_spheres("arils", items, aril))
    return objs


def build():
    common.reset_scene()
    common.setup_render()
    skin, calyx, aril, pith = skin_mat(), calyx_mat(), aril_mat(), pith_mat()
    objs = []

    wloc = (-0.50, 0.40, 0.92 * R)
    objs.append(whole_fruit(skin, wloc))
    objs += crown(calyx, (wloc[0], wloc[1], wloc[2] + 0.92 * R * 0.93))

    half = half_fruit(skin, aril, pith)
    pivot = bpy.data.objects.new("half_pivot", None)
    bpy.context.collection.objects.link(pivot)
    for ob in half:
        ob.parent = pivot
    pivot.rotation_euler = (math.radians(62), 0.0, math.radians(34))
    bpy.context.view_layer.update()
    mn, _ = common._world_bbox(half)
    pivot.location = (0.42, -0.30, -mn.z)
    objs += half

    # a few loose arils in front
    rng = random.Random(3)
    loose = []
    for dx, dy in ((-0.05, -0.62), (0.12, -0.70), (-0.22, -0.52), (0.30, -0.78)):
        loose.append((Vector((dx, dy, 0.040)), 0.046,
                      (1.0, 0.85, 0.9), rng.uniform(0, 6)))
    objs.append(batch_spheres("arils_loose", loose, aril))

    common.frame_object(objs, margin=0.10, elevation=26.0, azimuth=30.0,
                        focal=70.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render("pomegranate", common.OUT_DIR)


build()
