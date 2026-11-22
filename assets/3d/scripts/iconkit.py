"""Shared helpers for the icon_* batch (see SPEC.md 'Icon set').

Only generic, reusable building blocks live here; per-icon geometry stays in
the individual icon_<name>.py scripts. Requires common.py (imported by the
callers, which insert this directory on sys.path first).
"""

import math

import bpy

import common


# ---------------------------------------------------------------- geometry


def empty(name, location=(0, 0, 0), rotation=(0, 0, 0)):
    """Plain empty to parent groups of parts to (keeps pivots reusable)."""
    ob = bpy.data.objects.new(name, None)
    bpy.context.collection.objects.link(ob)
    ob.location = location
    ob.rotation_euler = rotation
    return ob


def parent_keep(child, root):
    """Parent keeping the child's current world transform."""
    mw = child.matrix_world.copy()
    child.parent = root
    child.matrix_parent_inverse = root.matrix_world.inverted()
    return child


def round_box(name, size, mat=None, location=(0, 0, 0), bevel=0.04,
              segments=3, rotation=None):
    """Bevelled box centred at `location`."""
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=location)
    ob = bpy.context.object
    ob.name = name
    ob.scale = (size[0] * 0.5, size[1] * 0.5, size[2] * 0.5)
    if rotation is not None:
        ob.rotation_euler = rotation
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    if mat is not None:
        common.assign(ob, mat)
    if bevel > 0:
        common.bevel(ob, bevel, segments)
    return ob


def prism(name, pts, thickness, mat=None, bevel_w=0.0):
    """Solid extrusion of a 2D polygon (list of (x, y)) along Z, centred.

    The polygon may be concave (crescent, gear teeth, easel triangles)."""
    n = len(pts)
    z0, z1 = -thickness * 0.5, thickness * 0.5
    verts = [(x, y, z0) for x, y in pts] + [(x, y, z1) for x, y in pts]
    faces = [tuple(range(n - 1, -1, -1)), tuple(range(n, 2 * n))]
    for i in range(n):
        j = (i + 1) % n
        faces.append((i, j, n + j, n + i))
    mesh = bpy.data.meshes.new(name + "_mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    ob = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(ob)
    if mat is not None:
        common.assign(ob, mat)
    if bevel_w > 0:
        common.bevel(ob, bevel_w, 2)
    return ob


def gear(name, teeth, r_root, r_tip, thickness, mat=None, bevel_w=0.0):
    """Cog silhouette: trapezoid teeth around a disc, extruded along Z."""
    pts = []
    step = 2.0 * math.pi / teeth
    for i in range(teeth):
        a = i * step
        for frac, r in ((0.00, r_root), (0.10, r_root), (0.18, r_tip),
                        (0.34, r_tip), (0.44, r_root), (0.50, r_root)):
            ang = a + frac * step
            pts.append((r * math.cos(ang), r * math.sin(ang)))
    return prism(name, pts, thickness, mat, bevel_w)


def crescent(name, radius, bite_radius, bite_dist, thickness, mat=None,
             bevel_w=0.0):
    """Crescent moon: disc of `radius` with a disc of `bite_radius` centred
    `bite_dist` toward +x removed; horns point +x in the polygon plane."""
    x0 = (radius ** 2 - bite_radius ** 2 + bite_dist ** 2) / (2.0 * bite_dist)
    y0 = math.sqrt(max(radius ** 2 - x0 ** 2, 1e-9))
    a_out = math.atan2(y0, x0)
    a_in = math.atan2(y0, x0 - bite_dist)
    pts = []
    steps = 24
    for i in range(steps + 1):
        t = a_out + (2.0 * math.pi - 2.0 * a_out) * i / steps
        pts.append((radius * math.cos(t), radius * math.sin(t)))
    for i in range(steps + 1):
        t = a_in + (2.0 * math.pi - 2.0 * a_in) * i / steps
        pts.append((bite_dist + bite_radius * math.cos(t),
                    bite_radius * math.sin(t)))
    return prism(name, pts, thickness, mat, bevel_w)


# ---------------------------------------------------------------- text


FONT_PATH = "/workspace/shorashim-wt-art/assets/fonts/NotoSerifHebrew-Regular.ttf"


def hebrew_letter(char, name, size, extrude, mat=None, location=(0, 0, 0),
                  rotation=(0, 0, 0)):
    """Raised Hebrew letter from the project's NotoSerifHebrew font."""
    font = bpy.data.fonts.get("NotoSerifHebrew") or bpy.data.fonts.load(FONT_PATH)
    cu = bpy.data.curves.new(name + "_curve", "FONT")
    cu.body = char
    cu.font = font
    cu.size = size
    cu.extrude = extrude
    cu.bevel_depth = extrude * 0.2
    cu.bevel_resolution = 2
    cu.align_x = "CENTER"
    cu.align_y = "BOTTOM"
    ob = bpy.data.objects.new(name, cu)
    bpy.context.collection.objects.link(ob)
    ob.location = location
    ob.rotation_euler = rotation
    if mat is not None:
        common.assign(ob, mat)
    return ob


# ---------------------------------------------------------------- materials


def simple_mat(name, color, roughness=0.5, metallic=0.0):
    """Plain Principled material (small accents not in the common library)."""
    m = bpy.data.materials.new(name)
    m.use_nodes = True
    bsdf = m.node_tree.nodes.get("Principled BSDF")
    bsdf.inputs["Base Color"].default_value = color
    bsdf.inputs["Roughness"].default_value = roughness
    bsdf.inputs["Metallic"].default_value = metallic
    return m


def mat_pomegranate():
    """Deep glossy red fruit skin with fine speckle bump."""
    m = bpy.data.materials.new("Pomegranate")
    m.use_nodes = True
    nt = m.node_tree
    nt.nodes.clear()
    out = nt.nodes.new("ShaderNodeOutputMaterial")
    out.location = (620, 0)
    bsdf = nt.nodes.new("ShaderNodeBsdfPrincipled")
    bsdf.location = (300, 0)
    nt.links.new(bsdf.outputs["BSDF"], out.inputs["Surface"])
    bsdf.inputs["Roughness"].default_value = 0.32
    if "Coat Weight" in bsdf.inputs:
        bsdf.inputs["Coat Weight"].default_value = 0.4
        bsdf.inputs["Coat Roughness"].default_value = 0.25
    tc = nt.nodes.new("ShaderNodeTexCoord")
    tc.location = (-820, -100)
    noise = nt.nodes.new("ShaderNodeTexNoise")
    noise.location = (-580, -100)
    noise.inputs["Scale"].default_value = 24.0
    noise.inputs["Detail"].default_value = 4.0
    noise.inputs["Roughness"].default_value = 0.7
    ramp = nt.nodes.new("ShaderNodeValToRGB")
    ramp.location = (-280, 140)
    ramp.color_ramp.elements[0].color = (0.30, 0.010, 0.020, 1.0)
    ramp.color_ramp.elements[1].color = (0.70, 0.045, 0.07, 1.0)
    nt.links.new(tc.outputs["Generated"], noise.inputs["Vector"])
    nt.links.new(noise.outputs["Fac"], ramp.inputs["Fac"])
    nt.links.new(ramp.outputs["Color"], bsdf.inputs["Base Color"])
    bump = nt.nodes.new("ShaderNodeBump")
    bump.location = (60, -160)
    bump.inputs["Strength"].default_value = 0.28
    bump.inputs["Distance"].default_value = 0.02
    nt.links.new(noise.outputs["Fac"], bump.inputs["Height"])
    nt.links.new(bump.outputs["Normal"], bsdf.inputs["Normal"])
    return m
