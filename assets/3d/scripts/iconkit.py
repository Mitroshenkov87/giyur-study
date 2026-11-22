"""Shared helpers for the icon_* batch (see SPEC.md 'Icon set').

Only generic, reusable building blocks live here; per-icon geometry stays in
the individual icon_<name>.py scripts. Requires common.py (imported by the
callers, which insert this directory on sys.path first).
"""

import math

import bpy
from mathutils import Matrix

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
    """Parent `child` to `root`, leaving the child's local matrix as built.

    Icon parts are modelled in the parent's local frame (the empty carries the
    hinge, tilt, fan or placement). The parent transform must move them.

    An earlier version set matrix_parent_inverse to the inverse parent world
    matrix, which cancelled that transform: hinges stayed flat, the seal stayed
    at the origin, and yaw on a stack never landed. Identity parent-inverse
    keeps the local location/rotation and lets the empty do the assembly.
    """
    loc = child.location.copy()
    rot = child.rotation_euler.copy()
    scl = child.scale.copy()
    child.parent = root
    child.matrix_parent_inverse = Matrix.Identity(4)
    child.location = loc
    child.rotation_euler = rot
    child.scale = scl
    return child


def round_box(name, size, mat=None, location=(0, 0, 0), bevel=0.04,
              segments=3, rotation=None):
    """Bevelled box centred at `location`."""
    # primitive_cube_add(size=1) spans [-0.5, 0.5] (extent 1). Scale must equal
    # the requested size; a 0.5 factor baked every box at half size and left
    # parts floating in the gaps between their intended positions.
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=location)
    ob = bpy.context.object
    ob.name = name
    ob.scale = (size[0], size[1], size[2])
    if rotation is not None:
        ob.rotation_euler = rotation
    bpy.ops.object.select_all(action="DESELECT")
    ob.select_set(True)
    bpy.context.view_layer.objects.active = ob
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


def _recalc_normals(mesh):
    import bmesh
    bm = bmesh.new()
    bm.from_mesh(mesh)
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    bm.to_mesh(mesh)
    bm.free()


def crescent(name, radius, bite_radius, bite_dist, thickness, mat=None,
             bevel_w=0.0):
    """Crescent in the XY plane. Horns point +Y.

    Built as a quad strip between the outer arc and the bite so the concave
    bowl is not handed to a single n-gon (that tessellates into a filled D).
    Rotate X by +90° afterwards to stand it with the horns up.
    """
    d = float(bite_dist)
    y_line = (radius ** 2 - bite_radius ** 2 + d * d) / (2.0 * d)
    x_half = math.sqrt(max(radius ** 2 - y_line ** 2, 1e-8))
    a_r = math.atan2(y_line, x_half)
    a_l = math.atan2(y_line, -x_half)
    b_r = math.atan2(y_line - d, x_half)
    b_l = math.atan2(y_line - d, -x_half)
    steps = 28

    def arc(a0, a1, cx, cy, r):
        if a1 < a0:
            a1 += 2.0 * math.pi
        pts = []
        for i in range(steps + 1):
            t = a0 + (a1 - a0) * i / steps
            pts.append((cx + r * math.cos(t), cy + r * math.sin(t)))
        return pts

    # Both polylines run left horn -> right horn. Outer takes the long way
    # around -Y; inner takes the dent (the bite arc inside the disc).
    outer = arc(a_l, a_r + 2.0 * math.pi, 0.0, 0.0, radius)
    inner = arc(b_l, b_r + 2.0 * math.pi, 0.0, d, bite_radius)
    n = len(outer)
    z0, z1 = -thickness * 0.5, thickness * 0.5
    verts = ([(x, y, z0) for x, y in outer] + [(x, y, z1) for x, y in outer]
             + [(x, y, z0) for x, y in inner] + [(x, y, z1) for x, y in inner])
    o0, o1, i0, i1 = 0, n, 2 * n, 3 * n
    faces = []
    for i in range(n - 1):
        faces.append((o1 + i, o1 + i + 1, i1 + i + 1, i1 + i))
        faces.append((o0 + i + 1, o0 + i, i0 + i, i0 + i + 1))
        faces.append((o0 + i, o0 + i + 1, o1 + i + 1, o1 + i))
        faces.append((i0 + i + 1, i0 + i, i1 + i, i1 + i + 1))
    mesh = bpy.data.meshes.new(name + "_mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    _recalc_normals(mesh)
    ob = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(ob)
    if mat is not None:
        common.assign(ob, mat)
    if bevel_w > 0:
        common.bevel(ob, bevel_w, 1)
    return ob


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
    # A heavy bevel fills the counters of א and the letter reads as a blob.
    cu.bevel_depth = min(extrude * 0.12, size * 0.012)
    cu.bevel_resolution = 1
    cu.resolution_u = 12
    cu.align_x = "CENTER"
    cu.align_y = "CENTER"
    ob = bpy.data.objects.new(name, cu)
    bpy.context.collection.objects.link(ob)
    ob.location = location
    ob.rotation_euler = rotation
    if mat is not None:
        common.assign(ob, mat)
    return ob


# ---------------------------------------------------------------- materials


def _set(bsdf, name, value):
    if name in bsdf.inputs:
        bsdf.inputs[name].default_value = value


def mat_parchment():
    """Warm tan page. Dark enough that AgX plus the icon key does not clip
    to white; hover only lifts it via the shared light rig."""
    m = bpy.data.materials.new("IconParchment")
    m.use_nodes = True
    nt = m.node_tree
    bsdf = nt.nodes.get("Principled BSDF")
    tc = nt.nodes.new("ShaderNodeTexCoord")
    noise = nt.nodes.new("ShaderNodeTexNoise")
    noise.inputs["Scale"].default_value = 3.5
    noise.inputs["Detail"].default_value = 2.0
    ramp = nt.nodes.new("ShaderNodeValToRGB")
    # AgX + the icon key lifts a 0.62 albedo to near-white pixels. These
    # values are darker than a swatch of parchment so the render lands on tan.
    ramp.color_ramp.elements[0].position = 0.30
    ramp.color_ramp.elements[0].color = (0.22, 0.15, 0.08, 1.0)
    ramp.color_ramp.elements[1].position = 0.72
    ramp.color_ramp.elements[1].color = (0.36, 0.26, 0.14, 1.0)
    nt.links.new(tc.outputs["Object"], noise.inputs["Vector"])
    nt.links.new(noise.outputs["Fac"], ramp.inputs["Fac"])
    nt.links.new(ramp.outputs["Color"], bsdf.inputs["Base Color"])
    _set(bsdf, "Roughness", 0.92)
    _set(bsdf, "Specular IOR Level", 0.08)
    fibres = nt.nodes.new("ShaderNodeTexNoise")
    fibres.inputs["Scale"].default_value = 40.0
    fibres.inputs["Detail"].default_value = 2.0
    bump = nt.nodes.new("ShaderNodeBump")
    bump.inputs["Strength"].default_value = 0.08
    bump.inputs["Distance"].default_value = 0.01
    nt.links.new(fibres.outputs["Fac"], bump.inputs["Height"])
    nt.links.new(bump.outputs["Normal"], bsdf.inputs["Normal"])
    return m


def mat_leather_color(name, color, roughness=0.52):
    """Dyed leather. `color` is a linear RGBA tuple; keep it dark so the
    leather theme does not render as salmon."""
    m = bpy.data.materials.new(name)
    m.use_nodes = True
    nt = m.node_tree
    bsdf = nt.nodes.get("Principled BSDF")
    _set(bsdf, "Base Color", color)
    _set(bsdf, "Roughness", roughness)
    _set(bsdf, "Coat Weight", 0.05)
    _set(bsdf, "Specular IOR Level", 0.35)
    n = nt.nodes.new("ShaderNodeTexNoise")
    n.inputs["Scale"].default_value = 70.0
    n.inputs["Detail"].default_value = 2.0
    bump = nt.nodes.new("ShaderNodeBump")
    bump.inputs["Strength"].default_value = 0.16
    bump.inputs["Distance"].default_value = 0.008
    nt.links.new(n.outputs["Fac"], bump.inputs["Height"])
    nt.links.new(bump.outputs["Normal"], bsdf.inputs["Normal"])
    return m


def mat_oxblood():
    return mat_leather_color("Oxblood", (0.12, 0.025, 0.02, 1.0), 0.50)


def accent():
    """Secondary surface for icons: walnut, or oxblood that stays oxblood."""
    if common.THEME == "leather":
        return mat_oxblood()
    return common.mat_walnut()


def mat_ivory():
    """Quill shaft. Dimmer than wax so the feather does not blow out."""
    m = bpy.data.materials.new("Ivory")
    m.use_nodes = True
    bsdf = m.node_tree.nodes.get("Principled BSDF")
    _set(bsdf, "Base Color", (0.40, 0.31, 0.18, 1.0))
    _set(bsdf, "Roughness", 0.55)
    _set(bsdf, "Specular IOR Level", 0.15)
    _set(bsdf, "Subsurface Weight", 0.08)
    return m


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
    ramp.color_ramp.elements[0].color = (0.12, 0.004, 0.008, 1.0)
    ramp.color_ramp.elements[1].color = (0.32, 0.012, 0.018, 1.0)
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
