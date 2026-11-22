"""Shofar: ram's horn, narrow mouthpiece curling up to a wide flared bell.
Custom swept tube (parallel rings along a spiral centreline) with UVs so the
cream->amber gradient and growth ridges run along the horn's length."""

import math
import os
import sys

import bpy
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common

CENTER = Vector((0.0, 0.0, 0.95))
SPIRAL_R = 0.62
A0 = -math.pi / 2.0
SWEEP = math.radians(315.0)
R_TIP = 0.045
R_BELL = 0.34
LIP = 0.06
N_RINGS = 150
N_SIDES = 24


def centerline(t):
    """Spiral in XZ with a gentle Y drift for a natural 3D curl."""
    a = A0 + SWEEP * t
    return CENTER + Vector((SPIRAL_R * math.cos(a),
                            0.16 * math.sin(math.pi * t),
                            SPIRAL_R * math.sin(a)))


def radius(t):
    r = R_TIP + (R_BELL - R_TIP) * (t ** 2.8)
    # flared lip near the bell
    s = max(0.0, (t - 0.90) / 0.10)
    return r + LIP * s * s


def sweep_horn():
    # ring centres: mouthpiece -> bell, then extra rings turning the lip inward
    rings = []           # list of (center, radius)
    for i in range(N_RINGS):
        t = i / (N_RINGS - 1)
        rings.append((centerline(t), radius(t)))
    end_c = centerline(1.0)
    end_r = radius(1.0)
    # end tangent (forward direction of travel)
    t_end = (centerline(1.0) - centerline(0.985)).normalized()
    rings.append((end_c + t_end * 0.030, end_r * 0.97))   # rolled rim
    rings.append((end_c + t_end * 0.100, end_r * 0.55))   # throat turns in
    rings.append((end_c + t_end * 0.130, end_r * 0.18))

    up = Vector((0.0, 1.0, 0.0))
    verts, uv, faces = [], [], []
    n = len(rings)
    for i, (c, r) in enumerate(rings):
        i0, i1 = max(i - 1, 0), min(i + 1, n - 1)
        tang = (rings[i1][0] - rings[i0][0]).normalized()
        n1 = tang.cross(up)
        if n1.length < 1e-6:
            n1 = Vector((1.0, 0.0, 0.0))
        n1.normalize()
        n2 = tang.cross(n1).normalized()
        u = min(i / (N_RINGS - 1), 1.0)
        for j in range(N_SIDES):
            a = 2.0 * math.pi * j / N_SIDES
            p = c + r * (math.cos(a) * n1 + math.sin(a) * n2)
            verts.append(tuple(p))
            uv.append((u, j / N_SIDES))
    for i in range(n - 1):
        for j in range(N_SIDES):
            nj = (j + 1) % N_SIDES
            faces.append((i * N_SIDES + j, i * N_SIDES + nj,
                          (i + 1) * N_SIDES + nj, (i + 1) * N_SIDES + j))
    # caps
    c0, _r0 = rings[0]
    tang0 = (rings[1][0] - rings[0][0]).normalized()
    ci = len(verts)
    verts.append(tuple(c0 - tang0 * 0.02))
    uv.append((0.0, 0.5))
    for j in range(N_SIDES):
        nj = (j + 1) % N_SIDES
        faces.append((ci, nj, j))
    cN, _rN = rings[-1]
    tangN = (rings[-1][0] - rings[-2][0]).normalized()
    ci = len(verts)
    verts.append(tuple(cN + tangN * 0.02))
    uv.append((1.0, 0.5))
    base = (n - 1) * N_SIDES
    for j in range(N_SIDES):
        nj = (j + 1) % N_SIDES
        faces.append((ci, base + j, base + nj))

    mesh = bpy.data.meshes.new("shofar_mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    uvl = mesh.uv_layers.new(name="UVMap")
    for poly in mesh.polygons:
        for li in poly.loop_indices:
            vi = mesh.loops[li].vertex_index
            uvl.data[li].uv = uv[vi]
    ob = bpy.data.objects.new("shofar", mesh)
    bpy.context.collection.objects.link(ob)
    common.smooth(ob)
    return ob


def horn_material():
    """Cream -> amber along the horn (UV u), ring ridges, subtle roughness."""
    m = bpy.data.materials.new("HornGrad")
    m.use_nodes = True
    nt = m.node_tree
    nt.nodes.clear()
    out = nt.nodes.new("ShaderNodeOutputMaterial")
    out.location = (600, 0)
    bsdf = nt.nodes.new("ShaderNodeBsdfPrincipled")
    bsdf.location = (260, 0)
    nt.links.new(bsdf.outputs["BSDF"], out.inputs["Surface"])
    bsdf.inputs["Roughness"].default_value = 0.38
    bsdf.inputs["Subsurface Weight"].default_value = 0.05

    tc = nt.nodes.new("ShaderNodeTexCoord")
    tc.location = (-900, 0)
    sep = nt.nodes.new("ShaderNodeSeparateXYZ")
    sep.location = (-700, 160)
    nt.links.new(tc.outputs["UV"], sep.inputs["Vector"])
    ramp = nt.nodes.new("ShaderNodeValToRGB")
    ramp.location = (-420, 160)
    cr = ramp.color_ramp
    cr.elements[0].position = 0.0
    cr.elements[0].color = (0.90, 0.84, 0.70, 1.0)
    e = cr.elements.new(0.55)
    e.color = (0.80, 0.62, 0.38, 1.0)
    cr.elements[1].position = 1.0
    cr.elements[1].color = (0.46, 0.26, 0.09, 1.0)
    nt.links.new(sep.outputs["X"], ramp.inputs["Fac"])
    nt.links.new(ramp.outputs["Color"], bsdf.inputs["Base Color"])

    wave = nt.nodes.new("ShaderNodeTexWave")
    wave.location = (-700, -180)
    wave.wave_type = "BANDS"
    wave.bands_direction = "X"
    wave.inputs["Scale"].default_value = 10.0
    wave.inputs["Distortion"].default_value = 6.0
    wave.inputs["Detail"].default_value = 4.0
    nt.links.new(tc.outputs["UV"], wave.inputs["Vector"])
    bump = nt.nodes.new("ShaderNodeBump")
    bump.location = (-60, -160)
    bump.inputs["Strength"].default_value = 0.25
    bump.inputs["Distance"].default_value = 0.03
    nt.links.new(wave.outputs["Color"], bump.inputs["Height"])
    nt.links.new(bump.outputs["Normal"], bsdf.inputs["Normal"])
    return m


def build():
    common.reset_scene()
    common.setup_render()
    horn = sweep_horn()
    common.assign(horn, horn_material())
    common.frame_object([horn], margin=0.12, elevation=18.0, azimuth=35.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render("shofar", common.OUT_DIR)


build()
