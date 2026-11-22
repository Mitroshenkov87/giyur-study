"""Emblem: a ram's horn. Crescent body, flared bell with a dark mouth,
thin mouthpiece, growth ridges. Not a curved arrow."""

import math
import os
import sys

import bpy
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import emblemkit

SIDES = 22
N_BODY = 48
# Mouthpiece -> bell. The crescent stays in the face of the coin. The last
# step also comes toward the camera, so the flared mouth is an oval on the
# end of the horn rather than a ring floating beside it.
WAY = [
    Vector((0.28, 0.02, -0.52)),   # thin mouthpiece, low
    Vector((0.55, 0.03, -0.12)),   # sweep out to the right
    Vector((0.22, 0.04, 0.28)),    # up and back toward the left
    Vector((-0.18, 0.04, 0.36)),   # top of the crescent
    Vector((-0.42, 0.02, 0.10)),   # wide end, still on the face
    Vector((-0.56, -0.20, -0.06)), # bell mouth: onward, and toward the camera
]


def _center(t):
    """Catmull-Rom through WAY. t = 0 at the mouthpiece, 1 at the bell."""
    n = len(WAY)
    seg = min(max(t, 0.0), 1.0) * (n - 1)
    i = min(int(seg), n - 2)
    u = seg - i
    p0 = WAY[max(i - 1, 0)]
    p1 = WAY[i]
    p2 = WAY[min(i + 1, n - 1)]
    p3 = WAY[min(i + 2, n - 1)]
    u2 = u * u
    u3 = u2 * u
    return 0.5 * (
        (2.0 * p1)
        + (-p0 + p2) * u
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * u2
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * u3
    )


def _radius(t, ridges=True):
    """Pointed mouthpiece, thick body, then a wide cup. Not a linear arrow taper."""
    if t < 0.12:
        u = t / 0.12
        r = 0.022 + 0.040 * (u ** 0.85)
    else:
        u = (t - 0.12) / 0.88
        # Body stays thick; the flare is only the last quarter.
        # Hold a thick body, then open into the cup.
        r = 0.080 + 0.030 * (u ** 0.75)
        if u > 0.55:
            f = (u - 0.55) / 0.45
            r += 0.155 * (f ** 1.1)
    # Coarse growth rings. Few, deep, so they survive the medallion's small size.
    if ridges and 0.20 < t < 0.70:
        fade = math.sin((t - 0.20) / 0.50 * math.pi)
        r *= 1.0 + 0.26 * math.sin(t * math.pi * 13.0) * fade
    return r


def _rot(v, axis, ang):
    c, s = math.cos(ang), math.sin(ang)
    return v * c + axis.cross(v) * s + axis * axis.dot(v) * (1.0 - c)


def _frames(pts):
    n = len(pts)
    tang = []
    for i in range(n):
        d = pts[min(i + 1, n - 1)] - pts[max(i - 1, 0)]
        tang.append(d.normalized() if d.length > 1e-8 else Vector((1.0, 0.0, 0.0)))
    nrm = tang[0].cross(Vector((0.0, 0.0, 1.0)))
    if nrm.length < 1e-6:
        nrm = tang[0].cross(Vector((0.0, 1.0, 0.0)))
    nrm.normalize()
    normals = [nrm]
    binorms = [tang[0].cross(nrm).normalized()]
    for i in range(1, n):
        axis = tang[i - 1].cross(tang[i])
        if axis.length < 1e-8:
            nrm = normals[-1].copy()
        else:
            axis.normalize()
            ang = math.acos(max(-1.0, min(1.0, tang[i - 1].dot(tang[i]))))
            nrm = _rot(normals[-1], axis, ang)
        nrm = (nrm - tang[i] * nrm.dot(tang[i])).normalized()
        normals.append(nrm)
        binorms.append(tang[i].cross(nrm).normalized())
    return tang, normals, binorms


def _ring(c, nrm, bn, rx, ry):
    out = []
    for j in range(SIDES):
        a = 2.0 * math.pi * j / SIDES
        out.append(c + nrm * (rx * math.cos(a)) + bn * (ry * math.sin(a)))
    return out


def _build_horn(metal, bore):
    ts = [i / (N_BODY - 1) for i in range(N_BODY)]
    pts = [_center(t) for t in ts]
    tang, normals, binorms = _frames(pts)

    verts, faces, fmats = [], [], []
    rings = []

    def add_ring(samples):
        idx = len(verts)
        for p in samples:
            verts.append(tuple(p))
        rings.append(idx)
        return idx

    def stitch(a, b, mat, flip=False):
        for j in range(SIDES):
            j2 = (j + 1) % SIDES
            if flip:
                faces.append((a + j, b + j, b + j2, a + j2))
            else:
                faces.append((a + j, a + j2, b + j2, b + j))
            fmats.append(mat)

    # Rounded point, not a triangular arrowhead. A short dome on the thin end.
    r0 = _radius(0.0)
    for k in range(1, 5):
        f = k / 5.0
        c = pts[0] - tang[0] * (r0 * 0.95 * (1.0 - f))
        add_ring(_ring(c, normals[0], binorms[0], r0 * f, r0 * f * 0.92))
        if len(rings) > 1:
            stitch(rings[-2], rings[-1], 0)
    tip_c = pts[0] - tang[0] * (r0 * 1.05)
    ci = len(verts)
    verts.append(tuple(tip_c))
    a0 = rings[0]
    for j in range(SIDES):
        j2 = (j + 1) % SIDES
        faces.append((ci, a0 + j2, a0 + j))
        fmats.append(0)

    for i, t in enumerate(ts):
        r = _radius(t)
        # Slightly oval so the tube has a horn section, still round in the round.
        add_ring(_ring(pts[i], normals[i], binorms[i], r, r * 0.90))
        stitch(rings[-2], rings[-1], 0)

    # Lip curls from the flare into the bore. The inner half is the dark mouth.
    bi = N_BODY - 1
    r_out = _radius(1.0, ridges=False)
    r_in = r_out * 0.76
    for k in range(1, 7):
        ang = math.pi * k / 6.0
        r = (r_out + r_in) * 0.5 + (r_out - r_in) * 0.5 * math.cos(ang)
        c = pts[bi] + tang[bi] * (0.028 * math.sin(ang))
        add_ring(_ring(c, normals[bi], binorms[bi], r, r * 0.90))
        mat = 0 if ang < math.pi * 0.55 else 1
        stitch(rings[-2], rings[-1], mat)

    # Short bore so the bell reads as a hollow oval, not a solid cap.
    last_t = 1.0
    for k in range(1, 9):
        t = 1.0 - 0.20 * k / 8.0
        fi = max(0, min(N_BODY - 1, int(round(t * (N_BODY - 1)))))
        r = _radius(t, ridges=False) * 0.74
        c = _center(t)
        add_ring(_ring(c, normals[fi], binorms[fi], r, r * 0.90))
        stitch(rings[-2], rings[-1], 1)
        last_t = t
    fi = max(0, min(N_BODY - 1, int(round(last_t * (N_BODY - 1)))))
    end_c = _center(last_t) - tang[fi] * 0.04
    ci = len(verts)
    verts.append(tuple(end_c))
    a1 = rings[-1]
    for j in range(SIDES):
        j2 = (j + 1) % SIDES
        faces.append((ci, a1 + j, a1 + j2))
        fmats.append(1)

    mesh = bpy.data.meshes.new("horn_mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    for poly, mi in zip(mesh.polygons, fmats):
        poly.material_index = mi
    ob = bpy.data.objects.new("horn", mesh)
    bpy.context.collection.objects.link(ob)
    ob.data.materials.append(metal)
    ob.data.materials.append(bore)
    common.smooth(ob)
    bpy.context.view_layer.objects.active = ob
    ob.select_set(True)
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_all(action="SELECT")
    bpy.ops.mesh.normals_make_consistent(inside=False)
    bpy.ops.object.mode_set(mode="OBJECT")
    ob.select_set(False)
    return ob


def _bore_mat():
    m, _nt, bsdf = common._new_material("HornBore")
    common._inp(bsdf, "Base Color", (0.010, 0.005, 0.003, 1.0))
    common._inp(bsdf, "Roughness", 0.94)
    return m


def build():
    common.reset_scene()
    common.setup_render()
    objs, metal, _accent = emblemkit.medallion()
    horn = _build_horn(metal, _bore_mat())
    syms = [horn]
    emblemkit.fit_xz(syms, 0.58)
    objs.extend(syms)
    emblemkit.finish(objs, "prazdniki")


build()
