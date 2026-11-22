"""Lessons icon: a chunky open book, seen from three-quarter above.

Thick leather covers sit under two page blocks that rise from the spine.
The fore-edge shows the page stack, with a ribbon and brass corner caps.
"""

import math
import os
import sys

import bpy
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import iconkit

COVER_T = 0.26
OPEN = math.radians(16.0)
# Page block in the hinge frame: x out from the spine, y along it, z up.
X0, X1 = 0.07, 0.78
Y0, Y1 = -0.54, 0.50
NU, NV = 22, 8


def _ztop(u):
    """Page-top height above the cover. Low in the gutter, tall at the fore-edge."""
    lift = math.sin(min(max(u, 0.0), 1.0) * math.pi * 0.50) ** 0.75
    return COVER_T + 0.10 + 0.52 * lift


def _page_edge_mat():
    """Fore-edge and head/tail: a few thick page bands, not a hairline texture."""
    m, nt, bsdf = common._new_material("PageEdge")
    tc = common._texcoord(nt)
    sep = nt.nodes.new("ShaderNodeSeparateXYZ")
    sep.location = (-640, 80)
    nt.links.new(tc.outputs["Generated"], sep.inputs["Vector"])
    ramp = nt.nodes.new("ShaderNodeValToRGB")
    ramp.location = (-360, 80)
    cr = ramp.color_ramp
    cr.interpolation = "CONSTANT"
    dark = (0.08, 0.045, 0.022, 1.0)
    light = (0.42, 0.31, 0.17, 1.0)
    # Three equal page bands. The dark half of each band is wide enough
    # to survive a 48 px downsample.
    stops = []
    n = 3
    for i in range(n):
        a = i / n
        # Dark seam is wide enough to survive 48 px, narrow enough to stay a line.
        stops.append((a, dark))
        stops.append((a + 0.22 / n, light))
    cr.elements[0].position = stops[0][0]
    cr.elements[0].color = stops[0][1]
    cr.elements[1].position = stops[1][0]
    cr.elements[1].color = stops[1][1]
    for pos, col in stops[2:]:
        elt = cr.elements.new(pos)
        elt.color = col
    nt.links.new(sep.outputs["Z"], ramp.inputs["Fac"])
    nt.links.new(ramp.outputs["Color"], bsdf.inputs["Base Color"])
    common._inp(bsdf, "Roughness", 0.88)
    return m


def _ribbon_mat():
    m, nt, bsdf = common._new_material("Bookmark")
    common._inp(bsdf, "Base Color", (0.22, 0.012, 0.016, 1.0))
    common._inp(bsdf, "Roughness", 0.40)
    common._inp(bsdf, "Sheen Weight", 0.22)
    common._inp(bsdf, "Specular IOR Level", 0.32)
    return m


def _page_block(name, sign, top_mat, edge_mat):
    """Solid page block. Top arches up from the spine; sides show the stack."""
    verts = []
    for i in range(NU + 1):
        u = i / NU
        x = sign * (X0 + (X1 - X0) * u)
        z = _ztop(u)
        for j in range(NV + 1):
            y = Y0 + (Y1 - Y0) * j / NV
            verts.append((x, y, z))
    off = (NU + 1) * (NV + 1)
    for i in range(NU + 1):
        u = i / NU
        x = sign * (X0 + (X1 - X0) * u)
        for j in range(NV + 1):
            y = Y0 + (Y1 - Y0) * j / NV
            verts.append((x, y, COVER_T))
    W = NV + 1
    faces, fmat = [], []

    def add(face, mat):
        faces.append(face)
        fmat.append(mat)

    for i in range(NU):
        for j in range(NV):
            a = i * W + j
            b = (i + 1) * W + j
            add((a, b, b + 1, a + 1), 0)
            add((off + a, off + a + 1, off + b + 1, off + b), 1)
    for j in range(NV):
        a = NU * W + j
        add((a, a + 1, off + a + 1, off + a), 1)
        a = j
        add((a, off + a, off + a + 1, a + 1), 1)
    for i in range(NU):
        a = i * W
        b = (i + 1) * W
        add((a, off + a, off + b, b), 1)
        a = i * W + NV
        b = (i + 1) * W + NV
        add((a, b, off + b, off + a), 1)

    mesh = bpy.data.meshes.new(name + "_mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    ob = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(ob)
    ob.data.materials.append(top_mat)
    ob.data.materials.append(edge_mat)
    for poly, mi in zip(mesh.polygons, fmat):
        poly.material_index = mi
        poly.use_smooth = mi == 0
    bpy.context.view_layer.objects.active = ob
    ob.select_set(True)
    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_all(action="SELECT")
    bpy.ops.mesh.normals_make_consistent(inside=False)
    bpy.ops.object.mode_set(mode="OBJECT")
    ob.select_set(False)
    return ob


def _corner(sign, oy, metal):
    """Brass cap wrapping one fore-edge corner of the cover."""
    ox = sign * 1.02
    in_x = -sign
    in_y = 1.0 if oy < 0.0 else -1.0
    out_y = -in_y
    parts = []
    parts.append(iconkit.round_box(
        "cap_top", (0.30, 0.30, 0.07), metal,
        location=(ox + in_x * 0.11, oy + in_y * 0.11, COVER_T + 0.02),
        bevel=0.02, segments=2))
    parts.append(iconkit.round_box(
        "cap_fore", (0.08, 0.32, COVER_T + 0.08), metal,
        location=(ox + sign * 0.012, oy + in_y * 0.10, (COVER_T + 0.06) * 0.5),
        bevel=0.016, segments=1))
    parts.append(iconkit.round_box(
        "cap_end", (0.32, 0.08, COVER_T + 0.08), metal,
        location=(ox + in_x * 0.10, oy + out_y * 0.012, (COVER_T + 0.06) * 0.5),
        bevel=0.016, segments=1))
    return parts


def _ribbon(mat, tip_mat):
    """Burgundy bookmark on the right page, over the front cover, brass tip."""
    x = 0.24
    u = (x - X0) / (X1 - X0)
    z_on = _ztop(u) + 0.02
    pts = [
        Vector((x, 0.18, z_on)),
        Vector((x, -0.15, z_on - 0.01)),
        Vector((x, Y0 + 0.02, z_on - 0.02)),
        Vector((x - 0.01, Y0 - 0.16, COVER_T * 0.35)),
        Vector((x - 0.02, Y0 - 0.34, -0.28)),
    ]
    half_w = 0.075
    verts = []
    for i, p in enumerate(pts):
        prev = pts[max(i - 1, 0)]
        nxt = pts[min(i + 1, len(pts) - 1)]
        tangent = nxt - prev
        if tangent.length < 1e-6:
            tangent = Vector((0.0, -1.0, 0.0))
        tangent.normalize()
        side = tangent.cross(Vector((0.0, 0.0, 1.0)))
        if side.length < 1e-4:
            side = Vector((1.0, 0.0, 0.0))
        side.normalize()
        verts.append(tuple(p - side * half_w))
        verts.append(tuple(p + side * half_w))
    n = len(pts)
    faces = []
    for i in range(n - 1):
        l0, r0, l1, r1 = 2 * i, 2 * i + 1, 2 * i + 2, 2 * i + 3
        faces.append((l0, l1, r1, r0))
    mesh = bpy.data.meshes.new("ribbon_mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    ob = bpy.data.objects.new("ribbon", mesh)
    bpy.context.collection.objects.link(ob)
    common.assign(ob, mat)
    sol = ob.modifiers.new("thickness", "SOLIDIFY")
    sol.thickness = 0.022
    bpy.context.view_layer.objects.active = ob
    ob.select_set(True)
    bpy.ops.object.modifier_apply(modifier=sol.name)
    ob.select_set(False)
    end = pts[-1]
    tip = iconkit.round_box(
        "ribbon_tip", (0.20, 0.09, 0.05), tip_mat,
        location=(end.x, end.y - 0.02, end.z - 0.02), bevel=0.014, segments=1)
    return [ob, tip]


def _side(sign, metal, accent, parchment, edge):
    hinge = iconkit.empty("side", rotation=(0.0, -OPEN * sign, 0.0))
    cover = iconkit.round_box(
        "cover", (1.04, 1.56, COVER_T), accent,
        location=(sign * 0.54, -0.06, COVER_T * 0.5), bevel=0.028, segments=2)
    pages = _page_block("pages", sign, parchment, edge)
    caps = []
    for oy in (-0.74, 0.60):
        caps += _corner(sign, oy, metal)
    parts = [cover, pages] + caps
    for ob in parts:
        iconkit.parent_keep(ob, hinge)
    return hinge, parts


def build():
    common.reset_scene()
    common.setup_render()
    metal = common.theme_metal()
    accent = iconkit.accent()
    parchment = iconkit.mat_parchment()
    edge = _page_edge_mat()

    objs = []
    hinges = {}
    for sign in (-1, 1):
        hinge, parts = _side(sign, metal, accent, parchment, edge)
        hinges[sign] = hinge
        objs += parts

    # Leather spine fills the gutter so the valley is a binding, not a hole.
    spine = common.add_cylinder(
        "spine", 0.15, 1.22, location=(0.0, -0.02, 0.16), mat=accent, vertices=28)
    spine.rotation_euler[0] = math.radians(90)
    common.smooth(spine)
    objs.append(spine)

    ribbon = _ribbon(_ribbon_mat(), metal)
    for ob in ribbon:
        iconkit.parent_keep(ob, hinges[1])
    objs += ribbon

    common.frame_object(objs, margin=0.11, focal=64.0, elevation=38.0, azimuth=34.0)
    common.setup_lights()
    common.render(common.variant_name("lessons"), common.OUT_DIR)


build()
