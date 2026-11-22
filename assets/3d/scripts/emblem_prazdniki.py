"""Emblem: a ram's horn. Thick crescent, flared bell with a dark oval
mouth, blunt mouthpiece, growth ridges. Not a curved arrow."""

import math
import os
import sys

import bpy
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import emblemkit

SIDES = 32
N_BODY = 140
# Growth ridges: shallow, regular rings (a few % of the width), not bulges.
RIDGES = 16
RIDGE_W = 0.035
RIDGE_D = 0.030
# Smooth crescent in the face of the coin. t = 0 is the mouthpiece,
# t = 1 is the bell. The arc is wide enough that a thick tube does not
# fold through itself. The last part of the path turns toward the camera
# so the flared mouth is an oval on the end of the horn.
_CX, _CZ = -0.02, -0.02
_RX, _RZ = 0.56, 0.50
_A0 = math.radians(-68.0)
_A1 = math.radians(168.0)


def _arc(t):
    """Arc progress, angle, and centre. Progress eases off at the bell."""
    t = min(max(t, 0.0), 1.0)
    s = t * (2.0 - t)
    ang = _A0 + (_A1 - _A0) * s
    # The centreline opens slightly toward the bell, like a horn, not a circle.
    grow = 1.0 + 0.20 * s
    y = 0.040
    if t > 0.68:
        u = (t - 0.68) / 0.32
        u = u * u * (3.0 - 2.0 * u)
        y = 0.040 + (-0.22 - 0.040) * (u ** 0.80)
    c = Vector((
        _CX + _RX * grow * math.cos(ang),
        y,
        _CZ + _RZ * grow * math.sin(ang),
    ))
    outward = Vector((math.cos(ang), 0.0, math.sin(ang)))
    return c, outward


def _center(t):
    c, _out = _arc(t)
    return c


def _halfwidth(t, ridges=True):
    """In-plane half-width. Blunt mouthpiece, thick shank, wide bell.

    Model units, before fit_xz. The tip stays a large fraction of the shank
    so it reads as a mouthpiece. The bell is sized so that, after
    fit_xz(0.70 * disc), its opening is about 35% of the inner field.
    """
    mouth, shank, bell = 0.195, 0.305, 0.460
    flare = 0.55
    if t < flare:
        u = (t / flare) ** 0.65
        w = mouth + (shank - mouth) * u
    else:
        u = ((t - flare) / (1.0 - flare)) ** 1.08
        w = shank + (bell - shank) * u
    if ridges and 0.18 < t < 0.76:
        fade = math.sin((t - 0.18) / 0.58 * math.pi)
        w *= 1.0 + RIDGE_W * math.sin(t * math.pi * 2.0 * RIDGES) * fade
    return w


def _halfdepth(t, ridges=True):
    """Roundness of the tube. Close to the in-plane width on the shank,
    a bit smaller at the bell so the mouth stays oval.

    Kept in check so the horn stands proud of the field without a spike.
    """
    w = _halfwidth(t, ridges=False)
    u = max(0.0, min(1.0, (t - 0.62) / 0.38))
    u = u * u * (3.0 - 2.0 * u)
    d = 0.150 + (w * 0.62 - 0.150) * u
    if ridges and 0.18 < t < 0.76:
        fade = math.sin((t - 0.18) / 0.58 * math.pi)
        d *= 1.0 + RIDGE_D * math.sin(t * math.pi * 2.0 * RIDGES) * fade
    return d


def _flare_shift(t):
    """How far the section centre moves outward as the bell opens.

    The extra width grows away from the crescent, so the mouth stays wide
    without swallowing the inner curve.
    """
    extra = max(0.0, _halfwidth(t, ridges=False) - 0.305)
    return extra * 0.75


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
    arcs = [_arc(t) for t in ts]
    pts = [c for c, _o in arcs]
    outwards = [o for _c, o in arcs]
    tang, normals, binorms = _frames(pts)

    verts, faces, fmats = [], [], []
    rings = []

    def place(i, t):
        return pts[i] + outwards[i] * _flare_shift(t)

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

    # Shallow dome. Radius stays near the full mouthpiece so the end is
    # a rounded nub, not a point.
    rd0 = _halfdepth(0.0)
    rw0 = _halfwidth(0.0)
    for k in range(1, 5):
        f = 0.88 + 0.12 * (k / 5.0)
        c = place(0, 0.0) - tang[0] * (min(rd0, rw0) * 0.28 * (1.0 - k / 5.0))
        add_ring(_ring(c, normals[0], binorms[0], rd0 * f, rw0 * f))
        if len(rings) > 1:
            stitch(rings[-2], rings[-1], 0)
    tip_c = place(0, 0.0) - tang[0] * (min(rd0, rw0) * 0.32)
    ci = len(verts)
    verts.append(tuple(tip_c))
    a0 = rings[0]
    for j in range(SIDES):
        j2 = (j + 1) % SIDES
        faces.append((ci, a0 + j2, a0 + j))
        fmats.append(0)

    for i, t in enumerate(ts):
        c = place(i, t)
        add_ring(_ring(c, normals[i], binorms[i], _halfdepth(t), _halfwidth(t)))
        stitch(rings[-2], rings[-1], 0)

    # Lip curls from the flare into the bore. The inner half is the dark mouth.
    bi = N_BODY - 1
    rd_out = _halfdepth(1.0, ridges=False)
    rw_out = _halfwidth(1.0, ridges=False)
    rd_in = rd_out * 0.70
    rw_in = rw_out * 0.70
    for k in range(1, 7):
        ang = math.pi * k / 6.0
        rd = (rd_out + rd_in) * 0.5 + (rd_out - rd_in) * 0.5 * math.cos(ang)
        rw = (rw_out + rw_in) * 0.5 + (rw_out - rw_in) * 0.5 * math.cos(ang)
        c = place(bi, 1.0) + tang[bi] * (0.050 * math.sin(ang))
        add_ring(_ring(c, normals[bi], binorms[bi], rd, rw))
        mat = 0 if ang < math.pi * 0.55 else 1
        stitch(rings[-2], rings[-1], mat)

    # Short bore so the bell reads as a hollow oval, not a solid cap.
    last_t = 1.0
    for k in range(1, 9):
        t = 1.0 - 0.18 * k / 8.0
        fi = max(0, min(N_BODY - 1, int(round(t * (N_BODY - 1)))))
        c = place(fi, t)
        add_ring(_ring(
            c, normals[fi], binorms[fi],
            _halfdepth(t, ridges=False) * 0.68,
            _halfwidth(t, ridges=False) * 0.68,
        ))
        stitch(rings[-2], rings[-1], 1)
        last_t = t
    fi = max(0, min(N_BODY - 1, int(round(last_t * (N_BODY - 1)))))
    end_c = place(fi, last_t) - tang[fi] * 0.05
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
    sub = ob.modifiers.new("smooth", "SUBSURF")
    sub.levels = 1
    sub.render_levels = 2
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
    # Long side of the horn ~70% of the inner-field diameter.
    emblemkit.fit_xz(syms, emblemkit.DISC_R * 0.70)
    bpy.context.view_layer.update()
    objs.extend(syms)
    emblemkit.finish(objs, "prazdniki")


build()
