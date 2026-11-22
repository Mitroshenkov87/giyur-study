"""Shared medallion builder for emblem_* scripts.

The coin stands in the XZ plane. The camera sits on -Y (elevation ~10,
azimuth ~15), so smaller Y is closer to the camera:

    bezel crest  y ~ -0.06     raised rim, closest metal
    relief front y ~ -0.09     symbol stands proud of the field
    field face   y ~  0.006    recessed accent disc (walnut / oxblood)
    coin back    y ~  0.16

Relief meshes are extruded from RELIEF_Y toward the camera by `depth`
(see extrude_polygon). R_Y is the Y centre for relief solids.
"""

import math

import bpy
from mathutils import Vector

import common

COIN_R = 1.0
DISC_R = 0.805
DISC_FACE_Y = 0.006       # accent face, recessed behind the bezel crest
RELIEF_Y = 0.012          # relief root, just inside the accent face
RELIEF_D = 0.105          # nominal extrusion toward the camera
R_Y = RELIEF_Y - RELIEF_D * 0.5
FIELD_LIMIT = 0.72        # keep symbol geometry inside the accent disc


def _lerp(a, b, t):
    return tuple(a[i] + (b[i] - a[i]) * t for i in range(len(a)))


def _link(name, mesh):
    ob = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(ob)
    return ob


def _outward_normals(ob):
    bpy.context.view_layer.objects.active = ob
    ob.select_set(True)
    try:
        bpy.ops.object.mode_set(mode="EDIT")
        bpy.ops.mesh.select_all(action="SELECT")
        bpy.ops.mesh.normals_make_consistent(inside=False)
        bpy.ops.object.mode_set(mode="OBJECT")
    except RuntimeError:
        try:
            bpy.ops.object.mode_set(mode="OBJECT")
        except RuntimeError:
            pass
    ob.select_set(False)
    return ob


def _auto_smooth(ob, angle_deg=48.0):
    bpy.context.view_layer.objects.active = ob
    ob.select_set(True)
    try:
        bpy.ops.object.shade_auto_smooth(angle=math.radians(angle_deg))
    except (RuntimeError, TypeError, AttributeError):
        common.smooth(ob)
    ob.select_set(False)
    return ob


def _bsdf(name):
    m = bpy.data.materials.new(name)
    m.use_nodes = True
    nt = m.node_tree
    nt.nodes.clear()
    out = nt.nodes.new("ShaderNodeOutputMaterial")
    out.location = (480, 0)
    bsdf = nt.nodes.new("ShaderNodeBsdfPrincipled")
    bsdf.location = (160, 0)
    nt.links.new(bsdf.outputs["BSDF"], out.inputs["Surface"])
    return m, nt, bsdf


def _set(bsdf, name, value):
    if name in bsdf.inputs:
        bsdf.inputs[name].default_value = value


def medal_metal(dark=0.0):
    """Satin-polished theme metal. Fine brush, no large cloudy patches.

    dark=0 is the emblem brass / bronze. dark=1 is a darker worked brass
    for flat pieces (corner brackets) that would otherwise blow out.
    """
    leather = common.THEME == "leather"
    if leather:
        base = _lerp((0.28, 0.15, 0.065), (0.055, 0.028, 0.012), dark)
        rough = 0.32 + 0.20 * dark
    else:
        # Linear brass. The dark end is dull antique brass for flat pieces
        # (corner brackets); a low roughness there clips the key light to white.
        base = _lerp((0.50, 0.32, 0.12), (0.085, 0.045, 0.016), dark)
        rough = 0.24 + 0.28 * dark
    m, nt, bsdf = _bsdf("MedalBronze" if leather else "MedalBrass")
    _set(bsdf, "Base Color", (*base, 1.0))
    _set(bsdf, "Metallic", 1.0)
    _set(bsdf, "Roughness", rough)
    _set(bsdf, "Anisotropic", 0.55 if not leather else 0.35)
    _set(bsdf, "Specular IOR Level", 0.45)

    tc = nt.nodes.new("ShaderNodeTexCoord")
    tc.location = (-900, 0)
    wave = nt.nodes.new("ShaderNodeTexWave")
    wave.location = (-680, 80)
    wave.wave_type = "BANDS"
    wave.bands_direction = "X"
    if "Scale" in wave.inputs:
        wave.inputs["Scale"].default_value = 90.0
    if "Distortion" in wave.inputs:
        wave.inputs["Distortion"].default_value = 1.2
    if "Detail" in wave.inputs:
        wave.inputs["Detail"].default_value = 2.0
    noise = nt.nodes.new("ShaderNodeTexNoise")
    noise.location = (-680, -180)
    if "Scale" in noise.inputs:
        noise.inputs["Scale"].default_value = 140.0
    if "Detail" in noise.inputs:
        noise.inputs["Detail"].default_value = 4.0
    if "Roughness" in noise.inputs:
        noise.inputs["Roughness"].default_value = 0.55
    mix = nt.nodes.new("ShaderNodeMixRGB")
    mix.location = (-440, -40)
    mix.inputs[0].default_value = 0.35
    nt.links.new(tc.outputs["Generated"], wave.inputs["Vector"])
    nt.links.new(tc.outputs["Generated"], noise.inputs["Vector"])
    nt.links.new(wave.outputs["Fac"], mix.inputs[1])
    nt.links.new(noise.outputs["Fac"], mix.inputs[2])
    bump = nt.nodes.new("ShaderNodeBump")
    bump.location = (-160, -80)
    if "Strength" in bump.inputs:
        bump.inputs["Strength"].default_value = 0.12
    if "Distance" in bump.inputs:
        bump.inputs["Distance"].default_value = 0.004
    nt.links.new(mix.outputs["Color"], bump.inputs["Height"])
    nt.links.new(bump.outputs["Normal"], bsdf.inputs["Normal"])

    rm = nt.nodes.new("ShaderNodeMapRange")
    rm.location = (-160, 140)
    rm.inputs["To Min"].default_value = max(rough - 0.04, 0.12)
    rm.inputs["To Max"].default_value = min(rough + 0.06, 0.55)
    nt.links.new(noise.outputs["Fac"], rm.inputs["Value"])
    nt.links.new(rm.outputs["Result"], bsdf.inputs["Roughness"])

    try:
        tan = nt.nodes.new("ShaderNodeTangent")
        tan.location = (-160, 300)
        tan.direction_type = "RADIAL"
        tan.axis = "Y"
        nt.links.new(tan.outputs["Tangent"], bsdf.inputs["Tangent"])
    except (TypeError, KeyError, AttributeError):
        pass
    return m


def field_material():
    """Recessed inner field: walnut grain across the face, or oxblood leather."""
    if common.THEME == "leather":
        m, nt, bsdf = _bsdf("Oxblood")
        # Deep oxblood. Low green/blue so AgX does not swing it pink.
        _set(bsdf, "Base Color", (0.10, 0.008, 0.008, 1.0))
        _set(bsdf, "Roughness", 0.52)
        _set(bsdf, "Coat Weight", 0.06)
        _set(bsdf, "Coat Roughness", 0.4)
        tc = nt.nodes.new("ShaderNodeTexCoord")
        n = nt.nodes.new("ShaderNodeTexNoise")
        n.location = (-520, -80)
        if "Scale" in n.inputs:
            n.inputs["Scale"].default_value = 70.0
        if "Detail" in n.inputs:
            n.inputs["Detail"].default_value = 3.0
        nt.links.new(tc.outputs["Generated"], n.inputs["Vector"])
        bump = nt.nodes.new("ShaderNodeBump")
        bump.location = (-180, -120)
        if "Strength" in bump.inputs:
            bump.inputs["Strength"].default_value = 0.22
        if "Distance" in bump.inputs:
            bump.inputs["Distance"].default_value = 0.008
        nt.links.new(n.outputs["Fac"], bump.inputs["Height"])
        nt.links.new(bump.outputs["Normal"], bsdf.inputs["Normal"])
        ramp = nt.nodes.new("ShaderNodeValToRGB")
        ramp.location = (-200, 80)
        cr = ramp.color_ramp
        cr.elements[0].position = 0.35
        cr.elements[0].color = (0.07, 0.005, 0.005, 1.0)
        cr.elements[1].position = 0.75
        cr.elements[1].color = (0.15, 0.014, 0.012, 1.0)
        nt.links.new(n.outputs["Fac"], ramp.inputs["Fac"])
        nt.links.new(ramp.outputs["Color"], bsdf.inputs["Base Color"])
        return m

    m, nt, bsdf = _bsdf("WalnutFace")
    tc = nt.nodes.new("ShaderNodeTexCoord")
    tc.location = (-980, 0)
    # Bands along local X: the disc cap is the local XY plane, so the grain
    # runs across the face instead of along the thickness.
    wave = nt.nodes.new("ShaderNodeTexWave")
    wave.location = (-740, 120)
    wave.wave_type = "BANDS"
    wave.bands_direction = "X"
    if "Scale" in wave.inputs:
        wave.inputs["Scale"].default_value = 4.2
    if "Distortion" in wave.inputs:
        wave.inputs["Distortion"].default_value = 2.4
    if "Detail" in wave.inputs:
        wave.inputs["Detail"].default_value = 2.0
    noise = nt.nodes.new("ShaderNodeTexNoise")
    noise.location = (-740, -140)
    if "Scale" in noise.inputs:
        noise.inputs["Scale"].default_value = 3.5
    if "Detail" in noise.inputs:
        noise.inputs["Detail"].default_value = 6.0
    mix = nt.nodes.new("ShaderNodeMixRGB")
    mix.location = (-500, 20)
    mix.blend_type = "MULTIPLY"
    mix.inputs[0].default_value = 0.65
    ramp = nt.nodes.new("ShaderNodeValToRGB")
    ramp.location = (-260, 80)
    cols = [(0.055, 0.024, 0.011, 1.0), (0.12, 0.055, 0.022, 1.0),
            (0.075, 0.032, 0.014, 1.0)]
    cr = ramp.color_ramp
    cr.elements[0].position = 0.0
    cr.elements[0].color = cols[0]
    cr.elements[1].position = 1.0
    cr.elements[1].color = cols[-1]
    mid = cr.elements.new(0.48)
    mid.color = cols[1]
    nt.links.new(tc.outputs["Generated"], wave.inputs["Vector"])
    nt.links.new(tc.outputs["Generated"], noise.inputs["Vector"])
    nt.links.new(wave.outputs["Color"], mix.inputs[1])
    nt.links.new(noise.outputs["Fac"], mix.inputs[2])
    nt.links.new(mix.outputs["Color"], ramp.inputs["Fac"])
    nt.links.new(ramp.outputs["Color"], bsdf.inputs["Base Color"])
    bump = nt.nodes.new("ShaderNodeBump")
    bump.location = (-40, -160)
    if "Strength" in bump.inputs:
        bump.inputs["Strength"].default_value = 0.16
    if "Distance" in bump.inputs:
        bump.inputs["Distance"].default_value = 0.012
    nt.links.new(wave.outputs["Fac"], bump.inputs["Height"])
    nt.links.new(bump.outputs["Normal"], bsdf.inputs["Normal"])
    _set(bsdf, "Roughness", 0.46)
    return m


def ink():
    m, nt, bsdf = _bsdf("Ink")
    _set(bsdf, "Base Color", (0.045, 0.022, 0.012, 1.0))
    _set(bsdf, "Roughness", 0.62)
    return m


def parchment():
    """Cream klaf, held down so the key light does not clip it to white."""
    m, nt, bsdf = _bsdf("Klaf")
    tc = nt.nodes.new("ShaderNodeTexCoord")
    n = nt.nodes.new("ShaderNodeTexNoise")
    n.location = (-560, 40)
    if "Scale" in n.inputs:
        n.inputs["Scale"].default_value = 5.0
    if "Detail" in n.inputs:
        n.inputs["Detail"].default_value = 3.0
    ramp = nt.nodes.new("ShaderNodeValToRGB")
    ramp.location = (-260, 40)
    cr = ramp.color_ramp
    cr.elements[0].position = 0.0
    cr.elements[0].color = (0.40, 0.31, 0.19, 1.0)
    cr.elements[1].position = 1.0
    cr.elements[1].color = (0.55, 0.44, 0.28, 1.0)
    nt.links.new(tc.outputs["Generated"], n.inputs["Vector"])
    nt.links.new(n.outputs["Fac"], ramp.inputs["Fac"])
    nt.links.new(ramp.outputs["Color"], bsdf.inputs["Base Color"])
    fibres = nt.nodes.new("ShaderNodeTexNoise")
    fibres.location = (-560, -200)
    if "Scale" in fibres.inputs:
        fibres.inputs["Scale"].default_value = 50.0
    bump = nt.nodes.new("ShaderNodeBump")
    bump.location = (-40, -180)
    if "Strength" in bump.inputs:
        bump.inputs["Strength"].default_value = 0.12
    if "Distance" in bump.inputs:
        bump.inputs["Distance"].default_value = 0.006
    nt.links.new(fibres.outputs["Fac"], bump.inputs["Height"])
    nt.links.new(bump.outputs["Normal"], bsdf.inputs["Normal"])
    _set(bsdf, "Roughness", 0.68)
    return m


def silk(color=(0.075, 0.004, 0.007, 1.0)):
    """Deep burgundy silk. Sheen is tinted so it cannot wash out to pink."""
    m, nt, bsdf = _bsdf("Silk")
    _set(bsdf, "Base Color", color)
    _set(bsdf, "Roughness", 0.48)
    _set(bsdf, "Sheen Weight", 0.16)
    _set(bsdf, "Sheen Roughness", 0.45)
    _set(bsdf, "Sheen Tint", (0.14, 0.012, 0.016, 1.0))
    _set(bsdf, "Specular IOR Level", 0.35)
    tc = nt.nodes.new("ShaderNodeTexCoord")
    wave = nt.nodes.new("ShaderNodeTexWave")
    wave.location = (-620, 40)
    wave.wave_type = "BANDS"
    wave.bands_direction = "Z"
    if "Scale" in wave.inputs:
        wave.inputs["Scale"].default_value = 40.0
    if "Distortion" in wave.inputs:
        wave.inputs["Distortion"].default_value = 0.6
    n = nt.nodes.new("ShaderNodeTexNoise")
    n.location = (-620, -200)
    if "Scale" in n.inputs:
        n.inputs["Scale"].default_value = 80.0
    nt.links.new(tc.outputs["Object"], wave.inputs["Vector"])
    nt.links.new(tc.outputs["Object"], n.inputs["Vector"])
    mix = nt.nodes.new("ShaderNodeMixRGB")
    mix.location = (-360, -40)
    mix.inputs[0].default_value = 0.5
    nt.links.new(wave.outputs["Fac"], mix.inputs[1])
    nt.links.new(n.outputs["Fac"], mix.inputs[2])
    bump = nt.nodes.new("ShaderNodeBump")
    bump.location = (-80, -160)
    if "Strength" in bump.inputs:
        bump.inputs["Strength"].default_value = 0.10
    if "Distance" in bump.inputs:
        bump.inputs["Distance"].default_value = 0.004
    nt.links.new(mix.outputs["Color"], bump.inputs["Height"])
    nt.links.new(bump.outputs["Normal"], bsdf.inputs["Normal"])
    return m


def seal_wax(color=(0.16, 0.006, 0.005, 1.0)):
    """Glossy deep-red sealing wax. No subsurface (that blooms pink under AgX)."""
    m, nt, bsdf = _bsdf("SealWax")
    _set(bsdf, "Base Color", color)
    _set(bsdf, "Roughness", 0.22)
    _set(bsdf, "Coat Weight", 0.28)
    _set(bsdf, "Coat Roughness", 0.12)
    _set(bsdf, "Specular IOR Level", 0.5)
    tc = nt.nodes.new("ShaderNodeTexCoord")
    n = nt.nodes.new("ShaderNodeTexNoise")
    n.location = (-560, -40)
    if "Scale" in n.inputs:
        n.inputs["Scale"].default_value = 18.0
    if "Detail" in n.inputs:
        n.inputs["Detail"].default_value = 4.0
    nt.links.new(tc.outputs["Object"], n.inputs["Vector"])
    bump = nt.nodes.new("ShaderNodeBump")
    bump.location = (-180, -80)
    if "Strength" in bump.inputs:
        bump.inputs["Strength"].default_value = 0.18
    if "Distance" in bump.inputs:
        bump.inputs["Distance"].default_value = 0.01
    nt.links.new(n.outputs["Fac"], bump.inputs["Height"])
    nt.links.new(bump.outputs["Normal"], bsdf.inputs["Normal"])
    return m


def fruit_red():
    m, nt, bsdf = _bsdf("Pomegranate")
    _set(bsdf, "Base Color", (0.18, 0.008, 0.008, 1.0))
    _set(bsdf, "Roughness", 0.38)
    _set(bsdf, "Coat Weight", 0.12)
    _set(bsdf, "Coat Roughness", 0.25)
    tc = nt.nodes.new("ShaderNodeTexCoord")
    n = nt.nodes.new("ShaderNodeTexNoise")
    n.location = (-520, -40)
    if "Scale" in n.inputs:
        n.inputs["Scale"].default_value = 22.0
    if "Detail" in n.inputs:
        n.inputs["Detail"].default_value = 5.0
    nt.links.new(tc.outputs["Object"], n.inputs["Vector"])
    bump = nt.nodes.new("ShaderNodeBump")
    bump.location = (-160, -80)
    if "Strength" in bump.inputs:
        bump.inputs["Strength"].default_value = 0.35
    if "Distance" in bump.inputs:
        bump.inputs["Distance"].default_value = 0.02
    nt.links.new(n.outputs["Fac"], bump.inputs["Height"])
    nt.links.new(bump.outputs["Normal"], bsdf.inputs["Normal"])
    return m


def leaf_green():
    m, nt, bsdf = _bsdf("Calyx")
    _set(bsdf, "Base Color", (0.04, 0.10, 0.035, 1.0))
    _set(bsdf, "Roughness", 0.5)
    return m


def flame_mat(strength=2.4, color=(1.0, 0.38, 0.05, 1.0)):
    m, nt, bsdf = _bsdf("Flame")
    _set(bsdf, "Base Color", (0.18, 0.04, 0.0, 1.0))
    _set(bsdf, "Roughness", 1.0)
    _set(bsdf, "Emission Color", color)
    _set(bsdf, "Emission Strength", strength)
    return m


def revolve_y(name, profile, segments, mat):
    """Revolve (radius, y) around Y. Profile runs along the surface.
    Smaller y faces the camera. Close the solid with radius 0 at both ends."""
    n = len(profile)
    verts = []
    for i in range(segments):
        a = 2.0 * math.pi * i / segments
        ca, sa = math.cos(a), math.sin(a)
        for r, y in profile:
            verts.append((r * ca, y, r * sa))
    faces = []
    for i in range(segments):
        ni = (i + 1) % segments
        for j in range(n - 1):
            a = i * n + j
            b = ni * n + j
            faces.append((a, a + 1, b + 1, b))
    mesh = bpy.data.meshes.new(name + "_mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    ob = _link(name, mesh)
    common.assign(ob, mat)
    _outward_normals(ob)
    _auto_smooth(ob, 55.0)
    return ob


def _coin_profile():
    """Cross-section of the medallion: back plate, raised bezel, recessed floor."""
    crest_y = -0.058
    pts = [
        (0.00, 0.16),
        (0.90, 0.16),
        (0.97, 0.148),
        (1.005, 0.11),
        (1.012, 0.035),
        (0.99, -0.008),
        (0.955, -0.040),
        (0.915, crest_y),
        (0.875, -0.050),
        (0.835, -0.022),
        (0.798, 0.010),
        (0.78, 0.020),
        (0.00, 0.020),
    ]
    return pts


def medallion():
    """Coin + raised bezel + recessed accent disc.

    Returns (objs, metal, accent). Build relief on RELIEF_Y / R_Y.
    """
    metal = medal_metal(0.0)
    accent = field_material()
    objs = []

    coin = revolve_y("coin", _coin_profile(), 96, metal)
    objs.append(coin)

    # Cylinder axis is local Z. Rx(90) maps local +Z onto world -Y, so the
    # cap nearest the camera lands at location.y - depth/2.
    depth = 0.07
    disc = common.add_cylinder(
        "inner_disc", DISC_R, depth,
        location=(0.0, DISC_FACE_Y + depth * 0.5, 0.0),
        vertices=96, mat=accent)
    disc.rotation_euler = (math.radians(90.0), 0.0, 0.0)
    common.bevel(disc, 0.008, segments=2)
    _auto_smooth(disc, 60.0)
    objs.append(disc)
    return objs, metal, accent


def _ensure_ccw(pts):
    area = 0.0
    for i in range(len(pts)):
        x1, z1 = pts[i]
        x2, z2 = pts[(i + 1) % len(pts)]
        area += x1 * z2 - x2 * z1
    if area < 0.0:
        return list(reversed(pts))
    return list(pts)


def extrude_polygon(name, pts, y_base, depth, mat, bevel_width=0.012):
    """Extrude an (x, z) polygon from y_base toward the camera by `depth`.

    pts are CCW as seen from the camera (auto-corrected if not).
    """
    pts = _ensure_ccw(pts)
    n = len(pts)
    verts = [(x, y_base, z) for x, z in pts] + [
        (x, y_base - depth, z) for x, z in pts]
    faces = [tuple(range(n - 1, -1, -1)), tuple(range(n, 2 * n))]
    for i in range(n):
        j = (i + 1) % n
        faces.append((i, j, n + j, n + i))
    mesh = bpy.data.meshes.new(name + "_mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    ob = _link(name, mesh)
    common.assign(ob, mat)
    _outward_normals(ob)
    if bevel_width:
        common.bevel(ob, bevel_width, segments=2)
    _auto_smooth(ob, 46.0)
    return ob


def ellipse_pts(cx, cz, rx, rz, n=48, a0=0.0, a1=None):
    if a1 is None:
        a1 = 2.0 * math.pi
    return [(cx + rx * math.cos(a0 + (a1 - a0) * i / n),
             cz + rz * math.sin(a0 + (a1 - a0) * i / n)) for i in range(n)]


def _xz_bounds(objs):
    bpy.context.view_layer.update()
    xs, zs = [], []
    for ob in objs:
        for corner in ob.bound_box:
            p = ob.matrix_world @ Vector(corner)
            xs.append(p.x)
            zs.append(p.z)
    return xs, zs


def recenter_xz(objs):
    """Shift objects so their combined XZ bounds sit on the medallion centre."""
    if not objs:
        return
    xs, zs = _xz_bounds(objs)
    cx = (min(xs) + max(xs)) * 0.5
    cz = (min(zs) + max(zs)) * 0.5
    for ob in objs:
        ob.location.x -= cx
        ob.location.z -= cz
    bpy.context.view_layer.update()


def fit_xz(objs, half):
    """Centre symbols and scale XZ so the longer half-extent equals `half`."""
    if not objs:
        return
    xs, zs = _xz_bounds(objs)
    cx = (min(xs) + max(xs)) * 0.5
    cz = (min(zs) + max(zs)) * 0.5
    cur = max((max(xs) - min(xs)) * 0.5, (max(zs) - min(zs)) * 0.5, 1e-6)
    s = half / cur
    empty = bpy.data.objects.new("fit_xz", None)
    bpy.context.collection.objects.link(empty)
    empty.location = (cx, 0.0, cz)
    bpy.context.view_layer.update()
    for ob in objs:
        ob.parent = empty
        ob.matrix_parent_inverse = empty.matrix_world.inverted()
    empty.location = (0.0, 0.0, 0.0)
    empty.scale = (s, 1.0, s)
    bpy.context.view_layer.update()


def finish(objs, base_name, margin=0.08):
    """Nearly frontal frame, lights, contact shadow, render the theme variant."""
    common.frame_object(objs, margin=margin, focal=85.0,
                        elevation=10.0, azimuth=15.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render(common.variant_name(base_name), common.OUT_DIR)
