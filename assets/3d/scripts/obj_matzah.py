"""A stack of three round hand-made (shmurah) matzot: irregular crisp
edges, toasted cream dough with brown/charred blisters and rows of dark
perforations."""

import math
import os
import sys

import bpy
from mathutils import Vector, noise

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common

R = 0.60
T = 0.026


def mat_shmurah(seed):
    m, nt, bs = common._new_material("Shmurah%d" % seed)
    L = nt.links
    tc = common._texcoord(nt)
    # --- toast colour: base cream, brown blisters, charred specks
    n1 = common._noise(nt, scale=9.0, detail=6.0, roughness=0.62)
    n1.noise_dimensions = "4D"
    n1.inputs["W"].default_value = seed * 3.1
    L.new(tc.outputs["Object"], n1.inputs["Vector"])
    ramp = common._ramp(nt, [(0.0, (0.46, 0.36, 0.21, 1.0)),
                             (0.38, (0.41, 0.30, 0.16, 1.0)),
                             (0.48, (0.31, 0.19, 0.08, 1.0)),
                             (0.56, (0.17, 0.085, 0.035, 1.0)),
                             (0.63, (0.06, 0.03, 0.015, 1.0)),
                             (1.0, (0.06, 0.03, 0.015, 1.0))])
    # radial toast: edges browner
    sep = nt.nodes.new("ShaderNodeSeparateXYZ")
    L.new(tc.outputs["Object"], sep.inputs["Vector"])
    rlen = nt.nodes.new("ShaderNodeVectorMath")
    rlen.operation = "LENGTH"
    comb = nt.nodes.new("ShaderNodeCombineXYZ")
    L.new(sep.outputs["X"], comb.inputs["X"])
    L.new(sep.outputs["Y"], comb.inputs["Y"])
    L.new(comb.outputs["Vector"], rlen.inputs[0])
    edge = nt.nodes.new("ShaderNodeMapRange")
    edge.inputs["From Min"].default_value = R * 0.70
    edge.inputs["From Max"].default_value = R * 1.05
    edge.inputs["To Min"].default_value = 0.0
    edge.inputs["To Max"].default_value = 0.13
    L.new(rlen.outputs["Value"], edge.inputs["Value"])
    add = nt.nodes.new("ShaderNodeMath")
    add.operation = "ADD"
    L.new(n1.outputs["Fac"], add.inputs[0])
    L.new(edge.outputs["Result"], add.inputs[1])
    L.new(add.outputs["Value"], ramp.inputs["Fac"])

    # --- perforation dots: rows across the disc (distorted grid)
    dn = common._noise(nt, scale=2.0, detail=2.0)
    L.new(tc.outputs["Object"], dn.inputs["Vector"])
    dmix = nt.nodes.new("ShaderNodeVectorMath")
    dmix.operation = "MULTIPLY_ADD"
    dmix.inputs[1].default_value = (0.035, 0.035, 0.0)
    L.new(dn.outputs["Color"], dmix.inputs[0])
    L.new(tc.outputs["Object"], dmix.inputs[2])
    fr, fd = 1.0 / 0.10, 1.0 / 0.042          # row pitch, dot pitch
    scl = nt.nodes.new("ShaderNodeVectorMath")
    scl.operation = "MULTIPLY"
    scl.inputs[1].default_value = (fd, fr, 0.0)
    L.new(dmix.outputs["Vector"], scl.inputs[0])
    fra = nt.nodes.new("ShaderNodeVectorMath")
    fra.operation = "FRACTION"
    L.new(scl.outputs["Vector"], fra.inputs[0])
    sub = nt.nodes.new("ShaderNodeVectorMath")
    sub.operation = "SUBTRACT"
    sub.inputs[1].default_value = (0.5, 0.5, 0.0)
    L.new(fra.outputs["Vector"], sub.inputs[0])
    back = nt.nodes.new("ShaderNodeVectorMath")
    back.operation = "MULTIPLY"
    back.inputs[1].default_value = (1.0 / fd, 1.0 / fr, 0.0)
    L.new(sub.outputs["Vector"], back.inputs[0])
    dl = nt.nodes.new("ShaderNodeVectorMath")
    dl.operation = "LENGTH"
    L.new(back.outputs["Vector"], dl.inputs[0])
    dot = nt.nodes.new("ShaderNodeMapRange")       # 1 inside a hole
    dot.inputs["From Min"].default_value = 0.0095
    dot.inputs["From Max"].default_value = 0.0135
    dot.inputs["To Min"].default_value = 1.0
    dot.inputs["To Max"].default_value = 0.0
    L.new(dl.outputs["Value"], dot.inputs["Value"])
    inner = nt.nodes.new("ShaderNodeMapRange")     # no holes near the rim
    inner.inputs["From Min"].default_value = R * 0.80
    inner.inputs["From Max"].default_value = R * 0.76
    L.new(rlen.outputs["Value"], inner.inputs["Value"])
    holes = nt.nodes.new("ShaderNodeMath")
    holes.operation = "MULTIPLY"
    L.new(dot.outputs["Result"], holes.inputs[0])
    L.new(inner.outputs["Result"], holes.inputs[1])
    # holes are dark-rimmed brown pits
    mix = nt.nodes.new("ShaderNodeMix")
    mix.data_type = "RGBA"
    L.new(holes.outputs["Value"], mix.inputs["Factor"])
    L.new(ramp.outputs["Color"], mix.inputs["A"])
    mix.inputs["B"].default_value = (0.035, 0.018, 0.008, 1.0)
    L.new(mix.outputs["Result"], bs.inputs["Base Color"])
    # bump: blisters raised, holes sunk
    bh = nt.nodes.new("ShaderNodeMath")
    bh.operation = "MULTIPLY_ADD"
    bh.inputs[1].default_value = -1.2
    L.new(holes.outputs["Value"], bh.inputs[0])
    L.new(n1.outputs["Fac"], bh.inputs[2])
    common._bump(nt, bs, bh.outputs["Value"], strength=0.55, distance=0.01)
    common._inp(bs, "Roughness", 0.78)
    return m


def make_disc(name, idx, mat):
    """Polar-grid disc: irregular wobbly rim, wavy body, blister bumps."""
    nseg, nring = 160, 26
    off = Vector((idx * 7.3, idx * 2.1, idx * 4.7))

    def rim(a):
        p = Vector((math.cos(a), math.sin(a), 0.0)) * 1.3 + off
        r = R * (1.0 + 0.07 * noise.noise(p) + 0.02 * math.sin(5 * a + idx))
        # small brittle chips
        r *= 1.0 - 0.03 * max(0.0, noise.noise(p * 7.0)) - 0.012 * max(0.0, noise.noise(p * 23.0))
        return r

    def height(x, y, top):
        p = Vector((x * 1.4, y * 1.4, 0.0)) + off
        wave = 0.032 * noise.noise(p)
        if not top:
            return wave - T * 0.5
        q = Vector((x * 11.0, y * 11.0, 0.0)) + off
        bl = max(0.0, noise.noise(q)) ** 1.5 * 0.022
        return wave + T * 0.5 + bl

    verts, faces = [], []
    rims = [rim(2 * math.pi * k / nseg) for k in range(nseg)]

    def ring_verts(top):
        start = len(verts)
        verts.append((0.0, 0.0, height(0.0, 0.0, top)))
        for j in range(1, nring + 1):
            f = j / nring
            f = 1.0 - (1.0 - f) ** 1.3
            for k in range(nseg):
                a = 2 * math.pi * k / nseg
                r = rims[k] * f
                x, y = r * math.cos(a), r * math.sin(a)
                z = height(x, y, top)
                if j == nring:
                    z = (z + height(x, y, not top)) * 0.5 + (0.006 if top else -0.006)
                verts.append((x, y, z))
        return start

    def ring_faces(start, flip):
        for k in range(nseg):
            kn = (k + 1) % nseg
            f = (start, start + 1 + k, start + 1 + kn)
            faces.append(f if not flip else f[::-1])
        for j in range(nring - 1):
            b0 = start + 1 + j * nseg
            b1 = b0 + nseg
            for k in range(nseg):
                kn = (k + 1) % nseg
                f = (b0 + k, b1 + k, b1 + kn, b0 + kn)
                faces.append(f if not flip else f[::-1])

    st = ring_verts(True)
    sb = ring_verts(False)
    ring_faces(st, False)
    ring_faces(sb, True)
    ot = st + 1 + (nring - 1) * nseg
    ob_ = sb + 1 + (nring - 1) * nseg
    for k in range(nseg):
        kn = (k + 1) % nseg
        faces.append((ot + k, ot + kn, ob_ + kn, ob_ + k))
    mesh = bpy.data.meshes.new(name + "_mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    ob = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(ob)
    ob.data.materials.append(mat)
    common.smooth(ob)
    return ob


def build():
    common.reset_scene()
    common.setup_render()
    objs = []
    placements = [  # z, rot, offset, tilt
        (0.035, 0.4, (0.0, 0.0), (0.0, 0.0)),
        (0.100, 2.6, (0.05, -0.03), (0.02, -0.015)),
        (0.165, 1.2, (-0.04, 0.035), (-0.018, 0.02)),
    ]
    for idx, (z, rot, off, tilt) in enumerate(placements):
        ob = make_disc("matzah", idx, mat_shmurah(idx))
        ob.location = (off[0], off[1], z)
        ob.rotation_euler = (tilt[0], tilt[1], rot)
        objs.append(ob)

    common.frame_object(objs, margin=0.0, elevation=34.0, azimuth=30.0,
                        focal=72.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render("matzah", common.OUT_DIR)


build()
