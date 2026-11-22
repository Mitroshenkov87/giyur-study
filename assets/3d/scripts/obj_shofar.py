"""Shofar: a ram's horn. One rising crescent with a half-twist, not a spiral.
Wide oval bell, hollow, with a thick rim and a dark bore; narrow darker
mouthpiece; honey-amber keratin, glossy, with growth ridges."""

import math
import os
import sys

import bpy
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common

N_BODY = 72
N_SIDES = 28
N_RIM = 8
N_INNER = 36
# Bore stops before the mouthpiece so the bell reads as a deep hollow.
INNER_T = 0.34

# Mouthpiece (t=0) runs out to the right, then the horn hooks up into a
# crescent whose bell opens toward the camera. One bend, not a spiral.
P0 = Vector((-1.35, 0.12, 0.05))
P1 = Vector((-0.20, 0.02, 0.10))
P2 = Vector((1.05, 0.10, 0.40))
P3 = Vector((0.55, -0.55, 1.25))


def _bezier(p0, p1, p2, p3, t):
    u = 1.0 - t
    return (u * u * u) * p0 + (3 * u * u * t) * p1 + (3 * u * t * t) * p2 + (t * t * t) * p3


def centerline(t):
    return _bezier(P0, P1, P2, P3, t)


def _smooth_axes(t):
    """Unridged oval radii. Narrow and round at the tip, wide oval at the bell."""
    grow = t ** 1.65
    r = 0.030 + 0.250 * grow
    oval = 1.0 + 0.55 * (t ** 1.2)
    rx = r * oval
    ry = r / math.sqrt(oval)
    # small mouthpiece collar just behind the tip
    if t < 0.07:
        k = math.sin(math.pi * t / 0.07)
        collar = 0.72 + 0.28 * k
        rx *= collar
        ry *= collar
    return rx, ry


def _lip(t):
    s = max(0.0, (t - 0.92) / 0.08)
    s = s * s
    return 0.055 * s, 0.036 * s


def _ridge(t):
    """Growth rings, fading at the tip and across the lip."""
    fade = math.sin(min(max(t, 0.0), 1.0) * math.pi) ** 0.6
    fade *= 1.0 - max(0.0, (t - 0.90) / 0.10)
    wave = (0.065 * math.sin(t * math.pi * 8.0)
            + 0.035 * math.sin(t * math.pi * 3.2 + 0.6))
    return 1.0 + wave * fade


def outer_axes(t):
    rx, ry = _smooth_axes(t)
    m = _ridge(t)
    lx, ly = _lip(t)
    return rx * m + lx, ry * m + ly


def inner_axes(t):
    rx, ry = _smooth_axes(t)
    return rx * 0.74, ry * 0.74


def twist_at(t):
    """Half turn along the horn so the oval rolls and finishes wide again."""
    return math.radians(18.0) + math.pi * t


def _rot(v, axis, ang):
    c, s = math.cos(ang), math.sin(ang)
    return v * c + axis.cross(v) * s + axis * axis.dot(v) * (1.0 - c)


def path_frames(pts):
    n = len(pts)
    tang = []
    for i in range(n):
        a = pts[max(i - 1, 0)]
        b = pts[min(i + 1, n - 1)]
        d = b - a
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


def _sample_path(n):
    ts = [i / (n - 1) for i in range(n)]
    pts = [centerline(t) for t in ts]
    tang, normals, binorms = path_frames(pts)
    return ts, pts, tang, normals, binorms


def _ring(c, nrm, bn, rx, ry, twist, n_sides):
    ct, st = math.cos(twist), math.sin(twist)
    out = []
    for j in range(n_sides):
        a = 2.0 * math.pi * j / n_sides
        lx = rx * math.cos(a)
        ly = ry * math.sin(a)
        ox = lx * ct - ly * st
        oy = lx * st + ly * ct
        out.append(c + nrm * ox + bn * oy)
    return out


def _add_ring(verts, uvs, pts, u):
    base = len(verts)
    for p in pts:
        verts.append(tuple(p))
        uvs.append((u, 0.0))
    # v around the ring
    for j in range(len(pts)):
        uvs[base + j] = (u, j / len(pts))
    return base


def _stitch(faces, fmats, a, b, n_sides, mat):
    for j in range(n_sides):
        nj = (j + 1) % n_sides
        faces.append((a + j, a + nj, b + nj, b + j))
        fmats.append(mat)


def _fan(verts, uvs, faces, fmats, ring_index, center, u, n_sides, mat, reverse):
    ci = len(verts)
    verts.append(tuple(center))
    uvs.append((u, 0.5))
    for j in range(n_sides):
        nj = (j + 1) % n_sides
        if reverse:
            faces.append((ci, ring_index + nj, ring_index + j))
        else:
            faces.append((ci, ring_index + j, ring_index + nj))
        fmats.append(mat)


def build_horn():
    n_sides = N_SIDES
    ts, pts, tang, normals, binorms = _sample_path(N_BODY)
    verts, uvs, faces, fmats = [], [], [], []
    ring_ids = []  # vertex index of ring start, parallel to the ring sequence

    def push(samples, u):
        idx = _add_ring(verts, uvs, samples, u)
        if ring_ids:
            _stitch(faces, fmats, ring_ids[-1], idx, n_sides, 0 if u < 1.001 else 0)
        ring_ids.append(idx)
        return idx

    # Mouthpiece dome, then the outer shell (material 0).
    tip_c = pts[0] - tang[0] * 0.045
    for k in range(1, 5):
        f = k / 5.0
        rx, ry = _smooth_axes(0.0)
        samples = _ring(pts[0] - tang[0] * (0.045 * (1.0 - f)),
                        normals[0], binorms[0],
                        rx * f, ry * f, twist_at(0.0), n_sides)
        idx = _add_ring(verts, uvs, samples, 0.0)
        if ring_ids:
            _stitch(faces, fmats, ring_ids[-1], idx, n_sides, 0)
        ring_ids.append(idx)
    _fan(verts, uvs, faces, fmats, ring_ids[0], tip_c, 0.0, n_sides, 0, reverse=True)

    for i, t in enumerate(ts):
        rx, ry = outer_axes(t)
        samples = _ring(pts[i], normals[i], binorms[i], rx, ry, twist_at(t), n_sides)
        idx = _add_ring(verts, uvs, samples, t)
        _stitch(faces, fmats, ring_ids[-1], idx, n_sides, 0)
        ring_ids.append(idx)

    # Rounded lip: outer radius curls over to the bore radius.
    bell_i = N_BODY - 1
    rx_o, ry_o = outer_axes(1.0)
    rx_i, ry_i = inner_axes(1.0)
    for k in range(1, N_RIM + 1):
        ang = math.pi * k / N_RIM
        rx = (rx_o + rx_i) * 0.5 + (rx_o - rx_i) * 0.5 * math.cos(ang)
        ry = (ry_o + ry_i) * 0.5 + (ry_o - ry_i) * 0.5 * math.cos(ang)
        c = pts[bell_i] + tang[bell_i] * (0.030 * math.sin(ang))
        samples = _ring(c, normals[bell_i], binorms[bell_i], rx, ry,
                        twist_at(1.0), n_sides)
        idx = _add_ring(verts, uvs, samples, 1.0)
        # Horn covers the lip; the cavity starts once the surface has curled inside.
        mat = 0 if ang < math.pi * 0.62 else 1
        _stitch(faces, fmats, ring_ids[-1], idx, n_sides, mat)
        ring_ids.append(idx)

    # Bore, walking back down the centreline. Centre comes from the curve
    # itself so the rings don't stack on the nearest body sample.
    last_c = pts[bell_i]
    last_fi = bell_i
    last_t = 1.0
    last_rx, last_ry = inner_axes(1.0)
    for k in range(1, N_INNER + 1):
        t = 1.0 - (1.0 - INNER_T) * k / N_INNER
        fi = max(0, min(N_BODY - 1, int(round(t * (N_BODY - 1)))))
        rx, ry = inner_axes(t)
        c = centerline(t)
        samples = _ring(c, normals[fi], binorms[fi], rx, ry, twist_at(t), n_sides)
        idx = _add_ring(verts, uvs, samples, t)
        _stitch(faces, fmats, ring_ids[-1], idx, n_sides, 1)
        ring_ids.append(idx)
        last_c, last_fi, last_t, last_rx, last_ry = c, fi, t, rx, ry

    end_c = last_c - tang[last_fi] * 0.05
    for k in range(1, 4):
        f = 1.0 - k / 4.0
        samples = _ring(last_c - tang[last_fi] * (0.05 * k / 4.0),
                        normals[last_fi], binorms[last_fi],
                        last_rx * f, last_ry * f, twist_at(last_t), n_sides)
        idx = _add_ring(verts, uvs, samples, last_t)
        _stitch(faces, fmats, ring_ids[-1], idx, n_sides, 1)
        ring_ids.append(idx)
    _fan(verts, uvs, faces, fmats, ring_ids[-1], end_c, last_t, n_sides, 1, reverse=False)

    mesh = bpy.data.meshes.new("shofar_mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    for poly, mi in zip(mesh.polygons, fmats):
        poly.material_index = mi
    uvl = mesh.uv_layers.new(name="UVMap")
    for poly in mesh.polygons:
        for li in poly.loop_indices:
            vi = mesh.loops[li].vertex_index
            uvl.data[li].uv = uvs[vi]
    ob = bpy.data.objects.new("shofar", mesh)
    bpy.context.collection.objects.link(ob)
    common.smooth(ob)
    bpy.ops.object.select_all(action="DESELECT")
    bpy.context.view_layer.objects.active = ob
    ob.select_set(True)
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_all(action="SELECT")
    bpy.ops.mesh.normals_make_consistent(inside=False)
    bpy.ops.object.mode_set(mode="OBJECT")
    ob.select_set(False)
    return ob


def horn_material():
    """Glossy keratin: dark tip, honey middle, cream, amber bell. Base <= 0.8."""
    m, nt, bsdf = common._new_material("HornKeratin")
    common._inp(bsdf, "Roughness", 0.26)
    common._inp(bsdf, "Coat Weight", 0.06)
    common._inp(bsdf, "Coat Roughness", 0.22)
    common._inp(bsdf, "Specular IOR Level", 0.50)
    common._inp(bsdf, "Subsurface Weight", 0.03)
    common._inp(bsdf, "Subsurface Radius", (0.85, 0.45, 0.22))

    tc = nt.nodes.new("ShaderNodeTexCoord")
    tc.location = (-1100, 80)
    sep = nt.nodes.new("ShaderNodeSeparateXYZ")
    sep.location = (-900, 180)
    nt.links.new(tc.outputs["UV"], sep.inputs["Vector"])
    # Albedo stays low: AgX plus the key light lifts 0.55 up to pale tan.
    ramp = common._ramp(nt, [
        (0.00, (0.05, 0.020, 0.008, 1.0)),
        (0.16, (0.14, 0.055, 0.015, 1.0)),
        (0.40, (0.32, 0.14, 0.035, 1.0)),
        (0.64, (0.48, 0.26, 0.07, 1.0)),
        (0.84, (0.36, 0.17, 0.045, 1.0)),
        (1.00, (0.18, 0.07, 0.02, 1.0)),
    ])
    nt.links.new(sep.outputs["X"], ramp.inputs["Fac"])

    mottling = common._noise(nt, scale=5.5, detail=4.0, roughness=0.55, y=40)
    nt.links.new(tc.outputs["UV"], mottling.inputs["Vector"])
    mix = nt.nodes.new("ShaderNodeMixRGB")
    mix.location = (-40, 80)
    mix.blend_type = "MULTIPLY"
    mix.inputs[0].default_value = 1.0
    shade = nt.nodes.new("ShaderNodeMapRange")
    shade.location = (-280, -20)
    shade.inputs["To Min"].default_value = 0.78
    shade.inputs["To Max"].default_value = 1.0
    nt.links.new(mottling.outputs["Fac"], shade.inputs["Value"])
    nt.links.new(ramp.outputs["Color"], mix.inputs[1])
    nt.links.new(shade.outputs["Result"], mix.inputs[2])
    nt.links.new(mix.outputs["Color"], bsdf.inputs["Base Color"])

    ridges = nt.nodes.new("ShaderNodeTexWave")
    ridges.location = (-700, -240)
    ridges.wave_type = "BANDS"
    ridges.bands_direction = "X"
    ridges.inputs["Scale"].default_value = 7.0
    ridges.inputs["Distortion"].default_value = 1.5
    ridges.inputs["Detail"].default_value = 2.0
    nt.links.new(tc.outputs["UV"], ridges.inputs["Vector"])
    # Darken the grooves as well as bumping them.
    groove = nt.nodes.new("ShaderNodeMapRange")
    groove.location = (-420, -260)
    groove.inputs["To Min"].default_value = 0.72
    groove.inputs["To Max"].default_value = 1.0
    nt.links.new(ridges.outputs["Fac"], groove.inputs["Value"])
    mix2 = nt.nodes.new("ShaderNodeMixRGB")
    mix2.location = (80, -40)
    mix2.blend_type = "MULTIPLY"
    mix2.inputs[0].default_value = 1.0
    nt.links.new(mix.outputs["Color"], mix2.inputs[1])
    nt.links.new(groove.outputs["Result"], mix2.inputs[2])
    nt.links.new(mix2.outputs["Color"], bsdf.inputs["Base Color"])
    common._bump(nt, bsdf, ridges.outputs["Fac"], strength=0.35, distance=0.025)
    return m


def cavity_material():
    m, nt, bsdf = common._new_material("HornCavity")
    common._inp(bsdf, "Base Color", (0.045, 0.022, 0.010, 1.0))
    common._inp(bsdf, "Roughness", 0.48)
    common._inp(bsdf, "Coat Weight", 0.08)
    common._inp(bsdf, "Specular IOR Level", 0.3)
    return m


def build():
    common.reset_scene()
    common.setup_render()
    horn = build_horn()
    horn.data.materials.append(horn_material())
    horn.data.materials.append(cavity_material())
    mn, _mx = common._world_bbox([horn])
    horn.location.z -= mn.z - 0.01
    common.frame_object([horn], margin=0.12, focal=70.0, elevation=20.0, azimuth=32.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render("shofar", common.OUT_DIR)


build()
