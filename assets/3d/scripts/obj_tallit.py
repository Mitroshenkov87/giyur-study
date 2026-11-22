"""Tallit: a softly folded wool shawl, off-white with navy stripes near the
ends, a silver atarah, and tzitzit that drape from the corners onto the
surface they rest on."""

import math
import os
import sys

import bpy
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common

HX = 1.12
HY = 0.70
CORNER_R = 0.18
WOOL = (0.74, 0.69, 0.60, 1.0)
NAVY = (0.008, 0.010, 0.020, 1.0)
TZ = (0.72, 0.68, 0.58, 1.0)


def height(x, y):
    """Top surface of the folded shawl. Soft rolls, not a stack of boxes."""
    z = 0.055
    z += 0.078 * math.exp(-((y - 0.08) ** 2) / (2.0 * 0.12 ** 2))
    z += 0.055 * math.exp(-((y + 0.30) ** 2) / (2.0 * 0.14 ** 2))
    z += 0.018 * math.sin(2.1 * x + 0.4) * math.cos(1.5 * y)
    z += 0.008 * math.sin(5.2 * x + 0.8) * math.sin(3.1 * y)
    xn = abs(x) / HX
    z -= 0.016 * xn * xn
    yn = y / HY
    if yn < -0.70:
        e = (-0.70 - yn) / 0.30
        z += 0.045 * e * e
    if yn > 0.76:
        e = (yn - 0.76) / 0.24
        z += 0.030 * e * e
    return max(0.048, z)


def _rounded(u, v):
    """Map a unit square onto a rounded rectangle so the outline is not a box."""
    x = -HX + 2.0 * HX * u
    y = -HY + 2.0 * HY * v
    ax, ay = abs(x), abs(y)
    limx, limy = HX - CORNER_R, HY - CORNER_R
    if ax > limx and ay > limy:
        cx = math.copysign(limx, x)
        cy = math.copysign(limy, y)
        dx, dy = x - cx, y - cy
        length = math.hypot(dx, dy) or 1.0
        if length > CORNER_R:
            dx *= CORNER_R / length
            dy *= CORNER_R / length
        x, y = cx + dx, cy + dy
    return x, y


def _band(nt, x_out, lo, hi):
    """1 inside [lo, hi], else 0."""
    gt = nt.nodes.new("ShaderNodeMath")
    gt.operation = "GREATER_THAN"
    gt.inputs[1].default_value = lo
    lt = nt.nodes.new("ShaderNodeMath")
    lt.operation = "LESS_THAN"
    lt.inputs[1].default_value = hi
    both = nt.nodes.new("ShaderNodeMath")
    both.operation = "MULTIPLY"
    nt.links.new(x_out, gt.inputs[0])
    nt.links.new(x_out, lt.inputs[0])
    nt.links.new(gt.outputs[0], both.inputs[0])
    nt.links.new(lt.outputs[0], both.inputs[1])
    return both


def cloth_material():
    """Off-white wool. Navy stripes are their own diffuse shader so sheen
    on the wool cannot wash them out to gray."""
    m, nt, out = common._new_material("TallitWool")[0], None, None
    # _new_material already wired a Principled to the output. Rebuild the
    # stripe mix on top of that BSDF.
    nt = m.node_tree
    bsdf = None
    surface = None
    for node in nt.nodes:
        if node.type == "BSDF_PRINCIPLED":
            bsdf = node
        elif node.type == "OUTPUT_MATERIAL":
            surface = node
    wool = bsdf
    common._inp(wool, "Base Color", WOOL)
    common._inp(wool, "Roughness", 0.90)
    common._inp(wool, "Sheen Weight", 0.30)
    common._inp(wool, "Sheen Roughness", 0.55)
    tc = common._texcoord(nt)
    sep = nt.nodes.new("ShaderNodeSeparateXYZ")
    sep.location = (-900, 200)
    nt.links.new(tc.outputs["Generated"], sep.inputs["Vector"])
    x = sep.outputs["X"]
    # Two wide bands near each short end.
    bands = [
        _band(nt, x, 0.00, 0.09),
        _band(nt, x, 0.145, 0.245),
        _band(nt, x, 0.755, 0.855),
        _band(nt, x, 0.910, 1.001),
    ]
    acc = bands[0]
    for b in bands[1:]:
        add = nt.nodes.new("ShaderNodeMath")
        add.operation = "ADD"
        nt.links.new(acc.outputs[0], add.inputs[0])
        nt.links.new(b.outputs[0], add.inputs[1])
        acc = add
    stripe = nt.nodes.new("ShaderNodeBsdfPrincipled")
    stripe.location = (200, -220)
    common._inp(stripe, "Base Color", NAVY)
    common._inp(stripe, "Roughness", 1.0)
    common._inp(stripe, "Specular IOR Level", 0.0)
    common._inp(stripe, "Sheen Weight", 0.0)
    mix = nt.nodes.new("ShaderNodeMixShader")
    mix.location = (480, 40)
    nt.links.new(acc.outputs[0], mix.inputs[0])
    nt.links.new(wool.outputs[0], mix.inputs[1])
    nt.links.new(stripe.outputs[0], mix.inputs[2])
    # replace the wool->output link
    for link in list(nt.links):
        if link.to_node == surface and link.to_socket == surface.inputs[0]:
            nt.links.remove(link)
    nt.links.new(mix.outputs[0], surface.inputs[0])
    mottling = common._noise(nt, scale=7.0, detail=3.0, y=-40)
    nt.links.new(tc.outputs["Generated"], mottling.inputs["Vector"])
    shade = nt.nodes.new("ShaderNodeMapRange")
    shade.location = (-280, -40)
    shade.inputs["To Min"].default_value = 0.88
    shade.inputs["To Max"].default_value = 1.0
    nt.links.new(mottling.outputs["Fac"], shade.inputs["Value"])
    mul = nt.nodes.new("ShaderNodeMixRGB")
    mul.location = (-40, 80)
    mul.blend_type = "MULTIPLY"
    mul.inputs[0].default_value = 1.0
    mul.inputs[1].default_value = WOOL
    nt.links.new(shade.outputs["Result"], mul.inputs[2])
    nt.links.new(mul.outputs["Color"], wool.inputs["Base Color"])
    weave = common._noise(nt, scale=180.0, detail=2.0, y=-280)
    nt.links.new(tc.outputs["Generated"], weave.inputs["Vector"])
    common._bump(nt, bsdf, weave.outputs["Fac"], strength=0.16, distance=0.004)
    return m


def tzitzit_material():
    m, nt, bsdf = common._new_material("Tzitzit")
    common._inp(bsdf, "Base Color", TZ)
    common._inp(bsdf, "Roughness", 0.72)
    common._inp(bsdf, "Sheen Weight", 0.25)
    return m


def atarah_material():
    """Silver-thread collar. Only partly metallic, so the warm key stays gray."""
    m, nt, bsdf = common._new_material("AtarahSilver")
    common._inp(bsdf, "Base Color", (0.55, 0.57, 0.60, 1.0))
    common._inp(bsdf, "Metallic", 0.35)
    common._inp(bsdf, "Roughness", 0.42)
    common._inp(bsdf, "Specular IOR Level", 0.45)
    return m


def make_cloth(mat):
    nu, nv = 64, 42
    verts = []
    for j in range(nv):
        v = j / (nv - 1)
        for i in range(nu):
            u = i / (nu - 1)
            x, y = _rounded(u, v)
            verts.append((x, y, height(x, y)))
    faces = []
    for j in range(nv - 1):
        for i in range(nu - 1):
            a = j * nu + i
            faces.append((a, a + 1, a + nu + 1, a + nu))
    mesh = bpy.data.meshes.new("tallit_mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    ob = bpy.data.objects.new("tallit", mesh)
    bpy.context.collection.objects.link(ob)
    common.assign(ob, mat)
    solid = ob.modifiers.new("solid", "SOLIDIFY")
    solid.thickness = 0.030
    solid.offset = -1.0
    sub = ob.modifiers.new("sub", "SUBSURF")
    sub.levels = 1
    sub.render_levels = 1
    bpy.context.view_layer.objects.active = ob
    ob.select_set(True)
    for name in (solid.name, sub.name):
        try:
            bpy.ops.object.modifier_apply(modifier=name)
        except RuntimeError:
            pass
    ob.select_set(False)
    common.smooth(ob)
    return ob


def _ribbon(name, x0, x1, y0, y1, mat, lift, ny=36):
    verts = []
    xs = (x0, x1)
    for j in range(ny):
        y = y0 + (y1 - y0) * j / (ny - 1)
        for x in xs:
            # Follow the cloth, but damped so the band stays a band.
            verts.append((x, y, 0.045 + 0.45 * height(x, y) + lift))
    n = len(verts)
    for j in range(ny):
        y = y0 + (y1 - y0) * j / (ny - 1)
        for x in xs:
            verts.append((x, y, 0.045 + 0.45 * height(x, y) + lift + 0.008))
    faces = []
    row = 2
    for j in range(ny - 1):
        a = j * row
        b = (j + 1) * row
        faces.append((a, b, b + 1, a + 1))                  # bottom
        faces.append((n + a + 1, n + b + 1, n + b, n + a))  # top
        faces.append((a, n + a, n + b, b))                  # left edge
        faces.append((a + 1, b + 1, n + b + 1, n + a + 1))  # right edge
    faces.append((0, 1, n + 1, n))
    last = (ny - 1) * row
    faces.append((last, n + last, n + last + 1, last + 1))
    mesh = bpy.data.meshes.new(name + "_mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    ob = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(ob)
    common.assign(ob, mat)
    common.smooth(ob)
    return ob


def _strand(name, pts, radius, mat):
    return common.tube_curve(name, pts, radius, mat=mat, resolution=2)


def add_tzitzit(objs, cx, cy, mat):
    """Wound gid with knots, then loose strings that lie on the cloth's surface."""
    ox = 1.0 if cx >= 0.0 else -1.0
    oy = 1.0 if cy >= 0.0 else -1.0
    origin = Vector((cx, cy, height(cx, cy) + 0.006))
    outward = Vector((ox * 0.78, oy * 0.62, 0.0)).normalized()
    side = Vector((-outward.y, outward.x, 0.0))
    drop = origin.z - 0.012

    wind_n = 22
    core = []
    helix = []
    for i in range(wind_n):
        t = i / (wind_n - 1)
        dist = 0.15 * t
        c = origin + outward * dist
        c.z = max(origin.z - drop * (t ** 0.65), 0.012)
        core.append(Vector(c))
        ang = t * 5.0 * 2.0 * math.pi
        radial = side * math.cos(ang) + Vector((0.0, 0.0, 1.0)) * math.sin(ang) * 0.35
        helix.append(c + radial * 0.013)
    objs.append(_strand("tz_core", core, 0.008, mat))
    objs.append(_strand("tz_wind", helix, 0.0042, mat))
    for kt in (0.08, 0.40, 0.72, 0.98):
        c = origin + outward * (0.15 * kt)
        c.z = max(origin.z - drop * (kt ** 0.65), 0.016)
        objs.append(common.add_uv_sphere(
            "tz_knot", 0.018, location=c, scale=(1.1, 1.1, 0.8),
            mat=mat, segments=16, rings=8))

    end = core[-1]
    for i in range(7):
        spread = (i - 3) * 0.011
        phase = i * 1.17 + (1.0 if ox > 0 else 0.4)
        length = 0.40 + (i % 3) * 0.055
        pts = []
        steps = 9
        for k in range(steps):
            t = k / (steps - 1)
            lateral = side * (spread * (0.15 + 0.85 * t)
                              + 0.032 * math.sin(t * 4.2 + phase))
            # A loose S so the strings don't read as straight sticks.
            sway = outward * math.sin(t * 2.6 + phase) * 0.03 * t
            p = end + outward * (t * length) + lateral + sway
            p = Vector(p)
            p.z = 0.012 + 0.008 * math.sin(t * math.pi) * (1.0 - t)
            pts.append(p)
        objs.append(_strand("tz_str", pts, 0.0055, mat))


def build():
    common.reset_scene()
    common.setup_render()
    cloth = cloth_material()
    white = tzitzit_material()
    silver = atarah_material()
    objs = [make_cloth(cloth)]
    # Silver collar just inside the striped end, parallel to the stripes.
    objs.append(_ribbon("atarah", -0.52, -0.22, -0.55, 0.55, silver, 0.016))

    for sx in (-1.0, 1.0):
        for sy in (-1.0, 1.0):
            add_tzitzit(objs, sx * 0.96, sy * 0.52, white)

    common.frame_object(objs, margin=0.10, focal=68.0, elevation=30.0, azimuth=34.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render("tallit", common.OUT_DIR)


build()
