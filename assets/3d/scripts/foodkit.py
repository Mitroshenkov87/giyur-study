"""Local helpers for the food/object batch (challah, matzah, seder plate,
pomegranate, lulav+etrog). common.py is frozen; extra helpers live here."""

import math

import bpy
from mathutils import Vector

import common


def smoothstep(t):
    t = max(0.0, min(1.0, t))
    return t * t * (3.0 - 2.0 * t)


def clamp(t, lo=0.0, hi=1.0):
    return max(lo, min(hi, t))


def prin(name, base, rough=0.5, metallic=0.0, coat=0.0, coat_rough=0.1,
         subsurface=0.0, transmission=0.0, ior=1.45,
         ramp=None, ramp_scale=4.0, ramp_detail=5.0,
         bump=0.0, bump_scale=50.0, emission=None):
    """Generic Principled material. ramp: [(pos, (r,g,b,a)), ...] over a noise."""
    m = bpy.data.materials.new(name)
    m.use_nodes = True
    nt = m.node_tree
    nt.nodes.clear()
    out = nt.nodes.new("ShaderNodeOutputMaterial")
    out.location = (620, 0)
    bs = nt.nodes.new("ShaderNodeBsdfPrincipled")
    bs.location = (260, 0)
    nt.links.new(bs.outputs["BSDF"], out.inputs["Surface"])

    def I(n, v):
        if n in bs.inputs:
            bs.inputs[n].default_value = v

    I("Base Color", base)
    I("Metallic", metallic)
    I("Roughness", rough)
    I("Coat Weight", coat)
    I("Coat Roughness", coat_rough)
    I("Subsurface Weight", subsurface)
    I("Transmission Weight", transmission)
    I("IOR", ior)
    if emission is not None:
        I("Emission Color", emission)
        I("Emission Strength", 4.0)
    if ramp:
        tc = nt.nodes.new("ShaderNodeTexCoord")
        tc.location = (-940, 0)
        n = nt.nodes.new("ShaderNodeTexNoise")
        n.location = (-720, -40)
        n.inputs["Scale"].default_value = ramp_scale
        n.inputs["Detail"].default_value = ramp_detail
        r = nt.nodes.new("ShaderNodeValToRGB")
        r.location = (-400, 90)
        cr = r.color_ramp
        cr.elements[0].position = ramp[0][0]
        cr.elements[0].color = ramp[0][1]
        cr.elements[1].position = ramp[-1][0]
        cr.elements[1].color = ramp[-1][1]
        for pos, col in ramp[1:-1]:
            e = cr.elements.new(pos)
            e.color = col
        nt.links.new(tc.outputs["Generated"], n.inputs["Vector"])
        nt.links.new(n.outputs["Fac"], r.inputs["Fac"])
        nt.links.new(r.outputs["Color"], bs.inputs["Base Color"])
    if bump > 0.0:
        tc2 = nt.nodes.new("ShaderNodeTexCoord")
        tc2.location = (-940, -380)
        n2 = nt.nodes.new("ShaderNodeTexNoise")
        n2.location = (-720, -400)
        n2.inputs["Scale"].default_value = bump_scale
        n2.inputs["Detail"].default_value = 3.0
        b = nt.nodes.new("ShaderNodeBump")
        b.location = (-80, -220)
        b.inputs["Strength"].default_value = bump
        b.inputs["Distance"].default_value = 0.02
        nt.links.new(tc2.outputs["Generated"], n2.inputs["Vector"])
        nt.links.new(n2.outputs["Fac"], b.inputs["Height"])
        nt.links.new(b.outputs["Normal"], bs.inputs["Normal"])
    return m


def rounded_box(name, dims, loc, mat, bevel_w=0.03, segments=3):
    bpy.ops.mesh.primitive_cube_add(location=loc)
    ob = bpy.context.object
    ob.name = name
    ob.dimensions = dims
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    common.bevel(ob, bevel_w, segments)
    common.assign(ob, mat)
    return ob


def tapered_tube(name, points, radii, sides=12, mat=None):
    """Tube mesh through points with a per-station radius. Ends are capped."""
    n = len(points)
    up = Vector((0.0, 1.0, 0.0))
    verts = []
    for i in range(n):
        p = points[i]
        if i == 0:
            tangent = (points[1] - points[0]).normalized()
        elif i == n - 1:
            tangent = (points[-1] - points[-2]).normalized()
        else:
            tangent = (points[i + 1] - points[i - 1]).normalized()
        normal = up.cross(tangent)
        if normal.length < 1e-6:
            normal = Vector((1.0, 0.0, 0.0))
        normal.normalize()
        binormal = tangent.cross(normal).normalized()
        r = radii[i]
        for k in range(sides):
            a = 2.0 * math.pi * k / sides
            d = normal * math.cos(a) + binormal * math.sin(a)
            verts.append(tuple(p + d * r))
    faces = []
    for i in range(n - 1):
        for k in range(sides):
            kn = (k + 1) % sides
            a = i * sides + k
            b = i * sides + kn
            c = (i + 1) * sides + kn
            d = (i + 1) * sides + k
            faces.append((a, b, c, d))
    c0 = len(verts)
    verts.append(tuple(points[0]))
    c1 = len(verts)
    verts.append(tuple(points[-1]))
    for k in range(sides):
        kn = (k + 1) % sides
        faces.append((c0, kn, k))
        a = (n - 1) * sides + k
        b = (n - 1) * sides + kn
        faces.append((c1, a, b))
    mesh = bpy.data.meshes.new(name + "_mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    ob = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(ob)
    if mat:
        ob.data.materials.append(mat)
    for poly in mesh.polygons:
        poly.use_smooth = True
    return ob


def ribbon(name, length, width, curl=0.5, fold=0.3, mat=None, stations=9,
           thickness=0.006):
    """Curled ribbon starting at the origin, extending +X, curling up +Z,
    with a V-fold across its width. Used for leaves/leaflets."""
    verts = []
    faces = []
    for i in range(stations):
        t = i / (stations - 1)
        x = length * t
        z = curl * length * (t ** 1.7)
        w = 0.5 * width * max(0.0, 1.0 - t ** 1.5) ** 0.7
        drop = fold * w
        verts.append((x, -w, z - drop))
        verts.append((x, 0.0, z))
        verts.append((x, w, z - drop))
    for i in range(stations - 1):
        a = i * 3
        b = (i + 1) * 3
        faces.append((a, b, b + 1, a + 1))
        faces.append((a + 1, b + 1, b + 2, a + 2))
    mesh = bpy.data.meshes.new(name + "_mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    ob = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(ob)
    if mat:
        ob.data.materials.append(mat)
    sol = ob.modifiers.new("solidify", "SOLIDIFY")
    sol.thickness = thickness
    return ob
