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
