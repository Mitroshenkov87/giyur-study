"""Small shared helpers for the obj_* book/scroll batch.

Ink and dark-bronze materials, a bevelled-box primitive, and a batch box
mesh (used to lay out rows of "text" dashes without hundreds of objects).
"""

import math
import os
import sys

import bpy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common


def mat_ink(color=(0.09, 0.045, 0.02, 1.0)):
    m = bpy.data.materials.new("Ink")
    m.use_nodes = True
    bsdf = m.node_tree.nodes.get("Principled BSDF")
    bsdf.inputs["Base Color"].default_value = color
    bsdf.inputs["Roughness"].default_value = 0.55
    return m


def mat_dark_bronze(color=(0.16, 0.09, 0.04, 1.0), roughness=0.4):
    m = bpy.data.materials.new("DarkBronze")
    m.use_nodes = True
    bsdf = m.node_tree.nodes.get("Principled BSDF")
    bsdf.inputs["Base Color"].default_value = color
    bsdf.inputs["Metallic"].default_value = 1.0
    bsdf.inputs["Roughness"].default_value = roughness
    return m


def cube(name, dims, loc, mat, bevel=0.0, rot=None):
    bpy.ops.mesh.primitive_cube_add(location=loc, rotation=rot or (0, 0, 0))
    ob = bpy.context.object
    ob.name = name
    ob.scale = (dims[0] / 2.0, dims[1] / 2.0, dims[2] / 2.0)
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    if mat:
        common.assign(ob, mat)
    if bevel:
        common.bevel(ob, bevel, 3)
    return ob


def add_boxes(name, boxes, mat):
    """One mesh from many small boxes: (cx, cy, cz, sx, sy, sz, rot_y)."""
    verts = []
    faces = []
    for cx, cy, cz, sx, sy, sz, ry in boxes:
        c, s = math.cos(ry), math.sin(ry)
        base = len(verts)
        for dx, dy, dz in ((-1, -1, -1), (1, -1, -1), (1, 1, -1), (-1, 1, -1),
                           (-1, -1, 1), (1, -1, 1), (1, 1, 1), (-1, 1, 1)):
            lx, lz = dx * sx * 0.5, dz * sz * 0.5
            verts.append((cx + lx * c + lz * s,
                          cy + dy * sy * 0.5,
                          cz - lx * s + lz * c))
        faces += [(base + 0, base + 1, base + 2, base + 3),
                  (base + 4, base + 7, base + 6, base + 5),
                  (base + 0, base + 4, base + 5, base + 1),
                  (base + 1, base + 5, base + 6, base + 2),
                  (base + 2, base + 6, base + 7, base + 3),
                  (base + 3, base + 7, base + 4, base + 0)]
    mesh = bpy.data.meshes.new(name + "_mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    ob = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(ob)
    ob.data.materials.append(mat)
    return ob


def mat_dark_walnut(name="DarkWalnut", scale=5.0, axis="X"):
    """Fine-grained dark walnut in Object space (no blotchy Generated stretch).
    Added for the obj_* art pass; does not replace common.mat_walnut."""
    m, nt, bsdf = common._new_material(name)
    tc = common._texcoord(nt)
    mp = nt.nodes.new("ShaderNodeMapping")
    sc = {"X": (0.25, 1.0, 1.0), "Y": (1.0, 0.25, 1.0), "Z": (1.0, 1.0, 0.25)}[axis]
    mp.inputs["Scale"].default_value = sc
    nt.links.new(tc.outputs["Object"], mp.inputs["Vector"])
    wave = nt.nodes.new("ShaderNodeTexWave")
    wave.wave_type = "BANDS"
    wave.bands_direction = {"X": "Y", "Y": "X", "Z": "X"}[axis]
    wave.inputs["Scale"].default_value = scale
    wave.inputs["Distortion"].default_value = 5.0
    wave.inputs["Detail"].default_value = 4.0
    nt.links.new(mp.outputs["Vector"], wave.inputs["Vector"])
    ramp = common._ramp(nt, [(0.0, (0.035, 0.016, 0.008, 1.0)),
                             (0.45, (0.095, 0.045, 0.020, 1.0)),
                             (0.75, (0.13, 0.065, 0.030, 1.0)),
                             (1.0, (0.06, 0.028, 0.013, 1.0))])
    nt.links.new(wave.outputs["Fac"], ramp.inputs["Fac"])
    nt.links.new(ramp.outputs["Color"], bsdf.inputs["Base Color"])
    common._bump(nt, bsdf, wave.outputs["Fac"], strength=0.06, distance=0.01)
    common._inp(bsdf, "Roughness", 0.38)
    common._inp(bsdf, "Coat Weight", 0.25)
    common._inp(bsdf, "Coat Roughness", 0.2)
    return m
