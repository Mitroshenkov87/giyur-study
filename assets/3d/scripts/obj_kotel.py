"""Kotel vignette: Herodian ashlars in warm weathered limestone.
Each stone has a drafted margin (smooth recessed border) around a raised
rough boss, deep shadowed joints, crumpled notes, and a caper tuft."""

import math
import os
import sys

import bpy
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common

COURSE_H = 0.36
DEPTH = 0.52
GAP = 0.046
# Bottom to top. Joints staggered, the way the Herodian courses are laid.
COURSES = [
    [(-1.42, -0.18), (-0.18, 0.78), (0.78, 1.42)],
    [(-1.42, 0.36), (0.36, 1.42)],
    [(-1.42, -0.52), (-0.52, 0.28), (0.28, 1.42)],
    [(-1.42, 0.02), (0.02, 0.84), (0.84, 1.42)],
    [(-1.42, -0.30), (-0.30, 0.62), (0.62, 1.42)],
    [(-1.42, 0.22), (0.22, 1.42)],
]
Y_OFF = [0.000, 0.012, -0.006, 0.018, 0.004, -0.010, 0.008, 0.014, -0.004, 0.010]


def _box(name, dims, loc, mat, bevel_w=0.0):
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=loc)
    ob = bpy.context.object
    ob.name = name
    ob.scale = dims
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    if mat:
        common.assign(ob, mat)
    if bevel_w:
        common.bevel(ob, bevel_w, segments=2)
    return ob


def limestone(name, bump, distance, rough, lighten=0.0):
    """Cream-ochre stone. World-space stains so a boss matches its margin."""
    m, nt, bsdf = common._new_material(name)
    geo = nt.nodes.new("ShaderNodeNewGeometry")
    geo.location = (-1100, 0)
    noise = common._noise(nt, scale=1.7, detail=5.0, roughness=0.65, distortion=0.5)
    nt.links.new(geo.outputs["Position"], noise.inputs["Vector"])
    ramp = common._ramp(nt, [
        (0.00, (0.26 + lighten, 0.16 + lighten * 0.5, 0.08, 1.0)),
        (0.22, (0.55 + lighten * 0.5, 0.40 + lighten * 0.4, 0.22, 1.0)),
        (0.48, (0.74, 0.60, 0.42, 1.0)),
        (0.70, (0.42 + lighten, 0.30 + lighten * 0.4, 0.16, 1.0)),
        (1.00, (0.66, 0.52, 0.34, 1.0)),
    ])
    nt.links.new(noise.outputs["Fac"], ramp.inputs["Fac"])

    ao = nt.nodes.new("ShaderNodeAmbientOcclusion")
    ao.location = (-700, -280)
    if "Distance" in ao.inputs:
        ao.inputs["Distance"].default_value = 0.14
    shade = nt.nodes.new("ShaderNodeMapRange")
    shade.location = (-460, -280)
    shade.inputs["To Min"].default_value = 0.58
    shade.inputs["To Max"].default_value = 1.0
    ao_out = "AO" if "AO" in ao.outputs else "Color"
    nt.links.new(ao.outputs[ao_out], shade.inputs["Value"])
    mul = nt.nodes.new("ShaderNodeMixRGB")
    mul.location = (-180, -40)
    mul.blend_type = "MULTIPLY"
    mul.inputs[0].default_value = 1.0
    nt.links.new(ramp.outputs["Color"], mul.inputs[1])
    nt.links.new(shade.outputs["Result"], mul.inputs[2])
    nt.links.new(mul.outputs["Color"], bsdf.inputs["Base Color"])

    pits = common._noise(nt, scale=16.0, detail=8.0, roughness=0.65, y=-420)
    nt.links.new(geo.outputs["Position"], pits.inputs["Vector"])
    common._bump(nt, bsdf, pits.outputs["Fac"], strength=bump, distance=distance)
    common._inp(bsdf, "Roughness", rough)
    return m


def paper_material(seed):
    m, nt, bsdf = common._new_material("Note%d" % seed)
    warm = 0.03 * ((seed * 3) % 5)
    # Lighter than the limestone so a note does not read as another stone.
    common._inp(bsdf, "Base Color", (0.78 - warm * 0.3, 0.72 - warm * 0.35, 0.52 - warm * 0.25, 1.0))
    common._inp(bsdf, "Roughness", 0.96)
    common._inp(bsdf, "Specular IOR Level", 0.04)
    common._inp(bsdf, "Metallic", 0.0)
    n = common._noise(nt, scale=9.0, detail=2.0)
    common._bump(nt, bsdf, n.outputs["Fac"], strength=0.45, distance=0.012)
    return m


def leaf_material(color):
    m, nt, bsdf = common._new_material("Caper")
    common._inp(bsdf, "Base Color", color)
    common._inp(bsdf, "Roughness", 0.55)
    common._inp(bsdf, "Sheen Weight", 0.2)
    return m


def build_wall(margin_mat, boss_mat):
    objs = []
    idx = 0
    for level, spans in enumerate(COURSES):
        z0 = level * COURSE_H
        for (x0, x1) in spans:
            w = (x1 - x0) - GAP
            h = COURSE_H - GAP
            yoff = Y_OFF[idx % len(Y_OFF)]
            cx = (x0 + x1) * 0.5
            cz = z0 + COURSE_H * 0.5
            cy = yoff
            body = _box("ashlar", (w, DEPTH, h), (cx, cy, cz), margin_mat, bevel_w=0.006)
            # Drafted margin is the body's flat front; the boss is a smaller
            # rough block standing proud of it.
            inset = 0.050 + 0.010 * ((idx * 5) % 4)
            inset_r = inset + 0.008 * ((idx * 3) % 3)
            boss_w = max(0.12, w - inset - inset_r)
            boss_h = max(0.10, h - inset * 1.7)
            boss_d = 0.034 + 0.010 * ((idx * 7) % 3) / 3.0
            front = cy - DEPTH * 0.5
            boss = _box(
                "boss", (boss_w, boss_d, boss_h),
                (cx + 0.01 * ((idx % 3) - 1), front - boss_d * 0.5 + 0.008, cz),
                boss_mat, bevel_w=0.008)
            objs += [body, boss]
            idx += 1
    return objs


def _note_mesh(name, w, h, seed, mat):
    """Sheet in local XZ. Hangs downward from v=0; local -Y points at the camera."""
    nu, nv = 9, 14
    verts = []
    for j in range(nv):
        v = j / (nv - 1)
        for i in range(nu):
            u = i / (nu - 1)
            x = (u - 0.5) * w * (1.0 - 0.06 * math.sin(seed + v * 2.0))
            z = -v * h
            # Gentle crumple only. A second skin plus solidify turned into a tangle.
            crumple = (0.012 * math.sin(u * 5.0 + seed) * math.sin(v * 3.2 + seed)
                       + 0.006 * math.sin(u * 11.0 + v * 7.0))
            # Peel the lower half off the stone; a soft horizontal crease.
            curl = -0.07 * (v ** 1.5)
            fold = -0.02 * math.sin(v * math.pi) * (0.4 + 0.6 * math.sin(u * math.pi))
            verts.append((x, crumple + curl + fold, z))
    faces = []
    for j in range(nv - 1):
        for i in range(nu - 1):
            a = j * nu + i
            faces.append((a, a + 1, a + nu + 1, a + nu))
    mesh = bpy.data.meshes.new(name + "_mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    ob = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(ob)
    common.assign(ob, mat)
    solid = ob.modifiers.new("solid", "SOLIDIFY")
    solid.thickness = 0.004
    bpy.context.view_layer.objects.active = ob
    ob.select_set(True)
    try:
        bpy.ops.object.modifier_apply(modifier=solid.name)
    except RuntimeError:
        pass
    ob.select_set(False)
    common.smooth(ob)
    return ob


def build_notes():
    """Notes tucked into horizontal joints, leaning out of the wall."""
    specs = [
        (0.35, 3, 0.26, 0.34, 0.18, 0.4),
        (-0.85, 4, 0.22, 0.30, -0.12, 1.7),
        (-0.15, 2, 0.24, 0.36, 0.22, 2.4),
        (0.95, 2, 0.20, 0.28, -0.08, 3.1),
        (-0.45, 5, 0.22, 0.26, 0.10, 4.2),
    ]
    objs = []
    front = -DEPTH * 0.5
    for n, (x, level, w, h, tilt, seed) in enumerate(specs):
        mat = paper_material(n + 1)
        ob = _note_mesh("note%d" % n, w, h, seed, mat)
        # Top edge sits in the joint; the sheet hangs down the stone face.
        ob.location = (x, front - 0.07, level * COURSE_H + 0.012)
        ob.rotation_euler = (0.08, 0.05 * math.sin(seed), tilt * 0.4)
        objs.append(ob)
    return objs


def build_plant(green, green_dark, stem_mat):
    """Caper tuft: arcing stems with flat paired leaves spilling from a joint."""
    joint_x = 0.15
    joint_z = 3 * COURSE_H
    base = Vector((joint_x, -DEPTH * 0.5 - 0.01, joint_z))
    objs = []
    # (sideways, out toward camera, up, length)
    stems = [
        (0.02, 0.55, 0.85, 0.48),
        (-0.55, 0.40, 0.70, 0.40),
        (0.60, 0.35, 0.55, 0.38),
        (-0.20, 0.70, 0.40, 0.34),
        (0.30, 0.25, 0.95, 0.44),
        (-0.80, 0.30, 0.50, 0.30),
    ]
    leaf_i = 0
    for sx, sy, sz, length in stems:
        direction = Vector((sx, -sy, sz))
        direction.normalize()
        tip = base + direction * length + Vector((0.0, -0.04, 0.02))
        mid = base + direction * length * 0.5 + Vector((sx * 0.04, -0.08, 0.05))
        pts = [base, mid, tip]
        objs.append(common.tube_curve("stem", pts, 0.011, mat=stem_mat, resolution=3))
        side = Vector((direction.z, 0.0, -direction.x))
        if side.length < 1e-6:
            side = Vector((1.0, 0.0, 0.0))
        side.normalize()
        for t, sc in ((0.32, 1.0), (0.52, 0.92), (0.72, 0.82), (0.90, 0.68)):
            p = base * (1.0 - t) + tip * t
            p = Vector(p) + Vector((0.0, -0.02, 0.01))
            for sign in (-1.0, 1.0):
                mat = green if (leaf_i + int(sign > 0)) % 2 == 0 else green_dark
                center = p + side * (0.045 * sc * sign) + Vector((0.0, -0.012, 0.0))
                leaf = common.add_uv_sphere(
                    "leaf", 0.070 * sc, location=center,
                    scale=(1.7, 0.22, 0.72), mat=mat, segments=12, rings=8)
                # Flat face toward the camera, tipped along the stem.
                leaf.rotation_euler = (
                    0.4 + 0.25 * math.sin(leaf_i),
                    0.5 * sign,
                    math.atan2(direction.x, direction.z) + sign * 0.6)
                objs.append(leaf)
                leaf_i += 1
        bud = common.add_uv_sphere(
            "bud", 0.016, location=tip + Vector((0.0, -0.01, 0.012)),
            scale=(0.8, 0.8, 1.25), mat=green, segments=12, rings=8)
        objs.append(bud)
    return objs


def build():
    common.reset_scene()
    common.setup_render()
    margin_mat = limestone("LimestoneMargin", bump=0.18, distance=0.012,
                           rough=0.72, lighten=0.06)
    boss_mat = limestone("LimestoneBoss", bump=0.85, distance=0.07, rough=0.98)
    void, _nt, void_bsdf = common._new_material("JointShadow")
    common._inp(void_bsdf, "Base Color", (0.06, 0.045, 0.030, 1.0))
    common._inp(void_bsdf, "Roughness", 1.0)

    objs = build_wall(margin_mat, boss_mat)
    # Backing so the joints read as deep shadow rather than holes to nothing.
    total_h = len(COURSES) * COURSE_H
    back = _box("joint_back", (2.78, 0.08, total_h - 0.04),
                (0.0, DEPTH * 0.5 - 0.02, total_h * 0.5), void, bevel_w=0.0)
    objs.append(back)
    objs += build_notes()
    green = leaf_material((0.16, 0.48, 0.10, 1.0))
    green_dark = leaf_material((0.08, 0.32, 0.06, 1.0))
    stem = leaf_material((0.20, 0.24, 0.08, 1.0))
    objs += build_plant(green, green_dark, stem)

    common.frame_object(objs, margin=0.10, focal=68.0, elevation=20.0, azimuth=28.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render("kotel", common.OUT_DIR)


build()
