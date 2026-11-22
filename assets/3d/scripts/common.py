"""Shared helpers for procedural ritual-object renders.

Blender 5.2 LTS, headless:  blender -b --factory-startup -P obj_<name>.py -- --out <dir> [--samples N]
Every object script imports this module, builds its geometry and calls
common.render(name, out_dir) which writes <out_dir>/<name>.png (RGBA, 1024x1024,
film transparent). A separate Pillow step converts PNGs to webp.
"""

import math
import os
import sys

import bpy
from mathutils import Vector
from bpy_extras.object_utils import world_to_camera_view

RESOLUTION = 1280  # rendered larger; tools/to_webp.py crops/centres to 1024
_DEFAULT_SAMPLES = 96


def _parse_cli():
    """Args after '--': --out DIR, --samples N, --res PX, --theme wood|leather,
    --hover (icon hover frame: turned ~15 deg and brighter key)."""
    argv = sys.argv
    argv = argv[argv.index("--") + 1:] if "--" in argv else []
    opts = {"out": None, "samples": None, "res": None, "theme": "wood", "hover": False}
    i = 0
    while i < len(argv):
        a = argv[i]
        if a == "--hover":
            opts["hover"] = True
            i += 1
        elif a.startswith("--") and a[2:] in opts and i + 1 < len(argv):
            opts[a[2:]] = argv[i + 1]
            i += 2
        else:
            i += 1
    return opts


_OPTS = _parse_cli()
OUT_DIR = _OPTS["out"]
SAMPLES = int(_OPTS["samples"]) if _OPTS["samples"] else _DEFAULT_SAMPLES
if _OPTS["res"]:
    RESOLUTION = int(_OPTS["res"])
THEME = _OPTS["theme"]          # "wood" (dark library) or "leather" (light hide)
HOVER = bool(_OPTS["hover"])    # hover frame for interactive icons
HOVER_TURN = 15.0               # degrees added to camera azimuth on hover
if OUT_DIR is None:
    OUT_DIR = os.path.abspath(
        os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "img", "objects")
    )


def variant_name(base):
    """'nav_lessons' -> 'nav_lessons-wood' / 'nav_lessons-leather-hover'."""
    return base + "-" + THEME + ("-hover" if HOVER else "")


def theme_metal():
    """Main metal of the theme: polished brass (wood) / dark antique bronze (leather)."""
    if THEME == "leather":
        return _metal("Bronze", (0.36, 0.20, 0.09, 1.0), 0.32,
                      noise_scale=9.0, bump_strength=0.05, bump_distance=0.02)
    return mat_brass()


def theme_accent():
    """Secondary surface: walnut (wood) / oxblood leather (leather)."""
    if THEME == "leather":
        return mat_leather((0.22, 0.06, 0.04, 1.0))
    return mat_walnut()


_RENDERABLE = {"MESH", "CURVE", "SURFACE", "FONT", "META"}


# ---------------------------------------------------------------- scene


def reset_scene():
    if bpy.context.object and bpy.context.object.mode != "OBJECT":
        try:
            bpy.ops.object.mode_set(mode="OBJECT")
        except RuntimeError:
            pass
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    for coll in (bpy.data.meshes, bpy.data.curves, bpy.data.materials,
                 bpy.data.cameras, bpy.data.lights):
        for block in list(coll):
            if block.users == 0:
                coll.remove(block)


def setup_render(samples=None, resolution=RESOLUTION):
    sc = bpy.context.scene
    sc.render.engine = "CYCLES"
    sc.cycles.device = "CPU"
    sc.cycles.samples = samples if samples else SAMPLES
    sc.cycles.use_denoising = True
    try:
        sc.cycles.denoiser = "OPENIMAGEDENOISE"
    except (TypeError, ValueError):
        pass
    sc.render.resolution_x = resolution
    sc.render.resolution_y = resolution
    sc.render.resolution_percentage = 100
    img = sc.render.image_settings
    img.file_format = "PNG"
    img.color_mode = "RGBA"
    img.color_depth = "8"
    sc.render.film_transparent = True
    sc.view_settings.view_transform = "AgX"
    for look in ("AgX - Medium High Contrast", "Medium High Contrast", "None"):
        try:
            sc.view_settings.look = look
            break
        except (TypeError, ValueError):
            continue
    sc.view_settings.exposure = -0.35
    world = bpy.data.worlds.get("World") or bpy.data.worlds.new("World")
    sc.world = world
    world.use_nodes = True
    # Invisible (film transparent) "library" environment so metals have
    # something to reflect: dark floor, warm amber band, dim ceiling.
    nt = world.node_tree
    bg = nt.nodes.get("Background")
    tc = nt.nodes.new("ShaderNodeTexCoord")
    sep = nt.nodes.new("ShaderNodeSeparateXYZ")
    ramp = nt.nodes.new("ShaderNodeValToRGB")
    nt.links.new(tc.outputs["Generated"], sep.inputs["Vector"])
    nt.links.new(sep.outputs["Z"], ramp.inputs["Fac"])
    cr = ramp.color_ramp
    cr.elements[0].position = 0.35
    cr.elements[0].color = (0.010, 0.007, 0.005, 1.0)
    cr.elements[1].position = 0.56
    cr.elements[1].color = (0.55, 0.36, 0.18, 1.0)
    e = cr.elements.new(0.75)
    e.color = (0.12, 0.08, 0.05, 1.0)
    nt.links.new(ramp.outputs["Color"], bg.inputs["Color"])
    bg.inputs["Strength"].default_value = 0.6


def render(name, out_dir=None):
    out_dir = out_dir or OUT_DIR
    os.makedirs(out_dir, exist_ok=True)
    sc = bpy.context.scene
    sc.render.filepath = os.path.join(out_dir, name + ".png")
    bpy.ops.render.render(write_still=True)
    return sc.render.filepath


# ---------------------------------------------------------------- camera / lights


def _world_bbox(objs):
    deps = bpy.context.evaluated_depsgraph_get()
    mn = Vector((math.inf,) * 3)
    mx = Vector((-math.inf,) * 3)
    for ob in objs:
        ob_eval = ob.evaluated_get(deps)
        mesh = ob_eval.to_mesh()
        if mesh is None:
            continue
        mw = ob_eval.matrix_world
        for vert in mesh.vertices:
            p = mw @ vert.co
            mn = Vector(map(min, mn, p))
            mx = Vector(map(max, mx, p))
        ob_eval.to_mesh_clear()
    return mn, mx


def _point_at(obj, target):
    direction = Vector(target) - obj.location
    obj.rotation_euler = direction.to_track_quat("-Z", "Y").to_euler()


def _cam_direction(elevation_deg, azimuth_deg):
    el = math.radians(elevation_deg)
    az = math.radians(azimuth_deg)
    return Vector((math.sin(az) * math.cos(el),
                   -math.cos(az) * math.cos(el),
                   math.sin(el)))


def frame_object(objs, margin=0.1, focal=72.0, elevation=25.0, azimuth=38.0):
    """3/4 view from slightly above; bounding box fills ~80% of the frame."""
    objs = [o for o in objs if o.type in _RENDERABLE]
    if HOVER:
        azimuth += HOVER_TURN
    cam_data = bpy.data.cameras.new("Camera")
    cam = bpy.data.objects.new("Camera", cam_data)
    bpy.context.collection.objects.link(cam)
    cam_data.lens = focal
    cam_data.sensor_width = 36.0
    bpy.context.scene.camera = cam

    mn, mx = _world_bbox(objs)
    center = (mn + mx) * 0.5
    radius = max((mx - mn).length * 0.5, 1e-4)
    fov = 2.0 * math.atan(cam_data.sensor_width / (2.0 * cam_data.lens))
    direction = _cam_direction(elevation, azimuth)
    dist = radius / math.tan(fov * 0.5)
    cam.location = center + direction * dist
    _point_at(cam, center)
    bpy.context.view_layer.update()
    # second pass: project bbox corners, scale distance so bbox spans 1-2*margin
    dev = 1e-4
    corners = [Vector((x, y, z))
               for x in (mn.x, mx.x) for y in (mn.y, mx.y) for z in (mn.z, mx.z)]
    for p_world in corners:
        p = world_to_camera_view(bpy.context.scene, cam, p_world)
        dev = max(dev, abs(p.x - 0.5), abs(p.y - 0.5))
    dist *= dev / (0.5 - margin)
    cam.location = center + direction * dist
    _point_at(cam, center)
    bpy.context.view_layer.update()
    return center, radius


def _area_light(name, color, energy, size, location, target):
    data = bpy.data.lights.new(name, "AREA")
    data.energy = energy
    data.color = color
    data.shape = "DISK"
    data.size = size
    ob = bpy.data.objects.new(name, data)
    bpy.context.collection.objects.link(ob)
    ob.location = location
    _point_at(ob, target)
    return ob


def setup_lights(center=None, radius=None):
    """Warm 3-light rig scaled to the current scene content."""
    objs = [o for o in bpy.context.scene.objects if o.type in _RENDERABLE]
    if center is None or radius is None:
        mn, mx = _world_bbox(objs)
        center = (mn + mx) * 0.5
        radius = max((mx - mn).length * 0.5, 0.1)
    s = radius
    target = Vector(center)
    _area_light("Key", (1.0, 0.78, 0.55), 600.0 * (1.45 if HOVER else 1.0), 3.2 * s,
                target + Vector((-1.6 * s, -1.9 * s, 2.4 * s)), target)
    _area_light("Fill", (0.72, 0.80, 1.0), 250.0, 4.0 * s,
                target + Vector((1.8 * s, -1.0 * s, 0.9 * s)), target)
    _area_light("Rim", (1.0, 0.72, 0.45), 800.0, 2.5 * s,
                target + Vector((0.4 * s, 2.1 * s, 1.9 * s)), target)


def add_shadow_catcher():
    """Invisible plane under the object that keeps a soft contact shadow in alpha."""
    objs = [o for o in bpy.context.scene.objects
            if o.type in _RENDERABLE and not o.is_shadow_catcher]
    mn, mx = _world_bbox(objs)
    center = (mn + mx) * 0.5
    radius = max((mx - mn).length * 0.5, 0.1)
    bpy.ops.mesh.primitive_plane_add(
        size=radius * 10.0,
        location=(center.x, center.y, mn.z - 0.001))
    plane = bpy.context.object
    plane.name = "ShadowCatcher"
    plane.is_shadow_catcher = True
    mat = bpy.data.materials.new("ShadowCatcher")
    mat.use_nodes = True
    plane.data.materials.append(mat)
    return plane


# ---------------------------------------------------------------- geometry helpers


def assign(obj, mat):
    obj.data.materials.append(mat)
    return obj


def smooth(obj):
    if hasattr(obj.data, "polygons"):
        for poly in obj.data.polygons:
            poly.use_smooth = True
    return obj


def bevel(obj, width=0.02, segments=3):
    mod = obj.modifiers.new("bevel", "BEVEL")
    mod.width = width
    mod.segments = segments
    mod.limit_method = "ANGLE"
    bpy.context.view_layer.objects.active = obj
    obj.select_set(True)
    try:
        bpy.ops.object.modifier_apply(modifier=mod.name)
    except RuntimeError:
        pass
    obj.select_set(False)
    return obj


def add_cylinder(name, radius, depth, location=(0, 0, 0), vertices=48, mat=None):
    bpy.ops.mesh.primitive_cylinder_add(vertices=vertices, radius=radius, depth=depth,
                                       location=location)
    ob = bpy.context.object
    ob.name = name
    if mat:
        assign(ob, mat)
    return ob


def add_uv_sphere(name, radius, location=(0, 0, 0), scale=(1, 1, 1), mat=None,
                  segments=48, rings=24):
    bpy.ops.mesh.primitive_uv_sphere_add(segments=segments, ring_count=rings,
                                        radius=radius, location=location)
    ob = bpy.context.object
    ob.name = name
    ob.scale = scale
    if mat:
        assign(ob, mat)
    smooth(ob)
    return ob


def add_cone(name, radius1, radius2, depth, location=(0, 0, 0), vertices=48, mat=None):
    bpy.ops.mesh.primitive_cone_add(vertices=vertices, radius1=radius1, radius2=radius2,
                                   depth=depth, location=location)
    ob = bpy.context.object
    ob.name = name
    if mat:
        assign(ob, mat)
    return ob


def add_torus(name, major_radius, minor_radius, location=(0, 0, 0), mat=None,
              major_segments=64, minor_segments=16):
    bpy.ops.mesh.primitive_torus_add(major_segments=major_segments,
                                    minor_segments=minor_segments,
                                    major_radius=major_radius,
                                    minor_radius=minor_radius,
                                    location=location)
    ob = bpy.context.object
    ob.name = name
    if mat:
        assign(ob, mat)
    smooth(ob)
    return ob


def tube_curve(name, points, radius, mat=None, resolution=3):
    """Smooth tube through 3D points (Vector or tuple)."""
    cu = bpy.data.curves.new(name + "_curve", "CURVE")
    cu.dimensions = "3D"
    cu.resolution_u = 2
    cu.bevel_depth = radius
    cu.bevel_resolution = resolution
    sp = cu.splines.new("NURBS")
    sp.points.add(len(points) - 1)
    for p, co in zip(sp.points, points):
        p.co = (co[0], co[1], co[2], 1.0)
    sp.order_u = min(3, len(points))
    sp.use_endpoint_u = True
    ob = bpy.data.objects.new(name, cu)
    bpy.context.collection.objects.link(ob)
    if mat:
        assign(ob, mat)
    return ob


def lathe(name, profile, segments=64, mat=None):
    """Surface of revolution. profile: [(radius, z), ...] bottom-to-top;
    start/end the list with radius 0 to close the solid."""
    n = len(profile)
    verts = []
    for i in range(segments):
        a = 2.0 * math.pi * i / segments
        ca, sa = math.cos(a), math.sin(a)
        for r, z in profile:
            verts.append((r * ca, r * sa, z))
    faces = []
    for i in range(segments):
        ni = (i + 1) % segments
        for j in range(n - 1):
            a = i * n + j
            b = ni * n + j
            faces.append((a, b, b + 1, a + 1))
    mesh = bpy.data.meshes.new(name + "_mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    ob = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(ob)
    if mat:
        assign(ob, mat)
    smooth(ob)
    return ob


# ---------------------------------------------------------------- materials


def _new_material(name):
    m = bpy.data.materials.new(name)
    m.use_nodes = True
    nt = m.node_tree
    nt.nodes.clear()
    out = nt.nodes.new("ShaderNodeOutputMaterial")
    out.location = (640, 0)
    bsdf = nt.nodes.new("ShaderNodeBsdfPrincipled")
    bsdf.location = (280, 0)
    nt.links.new(bsdf.outputs["BSDF"], out.inputs["Surface"])
    return m, nt, bsdf


def _inp(bsdf, name, value):
    if name in bsdf.inputs:
        bsdf.inputs[name].default_value = value
        return True
    return False


def _texcoord(nt, x=-1000, y=0):
    tc = nt.nodes.new("ShaderNodeTexCoord")
    tc.location = (x, y)
    return tc


def _noise(nt, scale=5.0, detail=3.0, roughness=0.6, distortion=0.0, x=-700, y=-200):
    n = nt.nodes.new("ShaderNodeTexNoise")
    n.location = (x, y)
    if "Scale" in n.inputs:
        n.inputs["Scale"].default_value = scale
    if "Detail" in n.inputs:
        n.inputs["Detail"].default_value = detail
    if "Roughness" in n.inputs:
        n.inputs["Roughness"].default_value = roughness
    if "Distortion" in n.inputs:
        n.inputs["Distortion"].default_value = distortion
    return n


def _bump(nt, bsdf, tex_out, strength=0.2, distance=0.05, x=-60, y=-200):
    b = nt.nodes.new("ShaderNodeBump")
    b.location = (x, y)
    if "Strength" in b.inputs:
        b.inputs["Strength"].default_value = strength
    if "Distance" in b.inputs:
        b.inputs["Distance"].default_value = distance
    nt.links.new(tex_out, b.inputs["Height"])
    nt.links.new(b.outputs["Normal"], bsdf.inputs["Normal"])
    return b


def _ramp(nt, colors, x=-320, y=120):
    """colors: [(pos, (r,g,b,a)), ...] sorted by pos."""
    ramp = nt.nodes.new("ShaderNodeValToRGB")
    ramp.location = (x, y)
    cr = ramp.color_ramp
    while len(cr.elements) > 2:
        cr.elements.remove(cr.elements[-1])
    cr.elements[0].position = colors[0][0]
    cr.elements[0].color = colors[0][1]
    cr.elements[1].position = colors[-1][0]
    cr.elements[1].color = colors[-1][1]
    for pos, col in colors[1:-1]:
        e = cr.elements.new(pos)
        e.color = col
    return ramp


def _metal(name, color, roughness, noise_scale=None, bump_strength=0.06, bump_distance=0.02):
    m, nt, bsdf = _new_material(name)
    _inp(bsdf, "Base Color", color)
    _inp(bsdf, "Metallic", 1.0)
    _inp(bsdf, "Roughness", roughness)
    if noise_scale:
        n = _noise(nt, scale=noise_scale, detail=3.0)
        _bump(nt, bsdf, n.outputs["Fac"], bump_strength, bump_distance)
        rm = nt.nodes.new("ShaderNodeMapRange")
        rm.location = (-40, 120)
        rm.inputs["To Min"].default_value = max(roughness - 0.08, 0.05)
        rm.inputs["To Max"].default_value = min(roughness + 0.12, 1.0)
        nt.links.new(n.outputs["Fac"], rm.inputs["Value"])
        nt.links.new(rm.outputs["Result"], bsdf.inputs["Roughness"])
    return m


def mat_brass():
    return _metal("Brass", (0.62, 0.40, 0.14, 1.0), 0.30,
                  noise_scale=7.0, bump_strength=0.05, bump_distance=0.02)


def mat_gold():
    return _metal("Gold", (0.85, 0.58, 0.16, 1.0), 0.22)


def mat_silver():
    return _metal("Silver", (0.80, 0.81, 0.84, 1.0), 0.14,
                  noise_scale=45.0, bump_strength=0.04, bump_distance=0.01)


def _wood(name, colors, roughness, grain_scale):
    m, nt, bsdf = _new_material(name)
    tc = _texcoord(nt)
    wave = nt.nodes.new("ShaderNodeTexWave")
    wave.location = (-780, 140)
    wave.wave_type = "BANDS"
    wave.bands_direction = "Z"
    if "Scale" in wave.inputs:
        wave.inputs["Scale"].default_value = grain_scale
    if "Distortion" in wave.inputs:
        wave.inputs["Distortion"].default_value = 8.0
    if "Detail" in wave.inputs:
        wave.inputs["Detail"].default_value = 4.0
    noise = _noise(nt, scale=3.0, detail=6.0, roughness=0.7, y=-160)
    mix = nt.nodes.new("ShaderNodeMixRGB")
    mix.blend_type = "MULTIPLY"
    mix.location = (-540, 60)
    mix.inputs[0].default_value = 0.7
    ramp = _ramp(nt, [(i / (len(colors) - 1), c) for i, c in enumerate(colors)])
    nt.links.new(tc.outputs["Generated"], wave.inputs["Vector"])
    nt.links.new(tc.outputs["Generated"], noise.inputs["Vector"])
    nt.links.new(wave.outputs["Color"], mix.inputs[1])
    nt.links.new(noise.outputs["Fac"], mix.inputs[2])
    nt.links.new(mix.outputs["Color"], ramp.inputs["Fac"])
    nt.links.new(ramp.outputs["Color"], bsdf.inputs["Base Color"])
    _bump(nt, bsdf, wave.outputs["Fac"], strength=0.25, distance=0.08)
    _inp(bsdf, "Roughness", roughness)
    return m


def mat_walnut():
    return _wood("Walnut",
                 [(0.10, 0.045, 0.020, 1.0), (0.18, 0.085, 0.035, 1.0),
                  (0.065, 0.028, 0.012, 1.0)],
                 0.40, 4.0)


def mat_light_wood():
    return _wood("LightWood",
                 [(0.42, 0.26, 0.12, 1.0), (0.58, 0.38, 0.18, 1.0),
                  (0.34, 0.20, 0.095, 1.0)],
                 0.45, 4.0)


def mat_parchment():
    m, nt, bsdf = _new_material("Parchment")
    tc = _texcoord(nt)
    noise = _noise(nt, scale=4.0, detail=3.0)
    ramp = _ramp(nt, [(0.0, (0.80, 0.71, 0.54, 1.0)),
                      (1.0, (0.91, 0.84, 0.67, 1.0))])
    nt.links.new(tc.outputs["Generated"], noise.inputs["Vector"])
    nt.links.new(noise.outputs["Fac"], ramp.inputs["Fac"])
    nt.links.new(ramp.outputs["Color"], bsdf.inputs["Base Color"])
    fibres = _noise(nt, scale=60.0, detail=2.0, y=-320)
    _bump(nt, bsdf, fibres.outputs["Fac"], strength=0.10, distance=0.015)
    _inp(bsdf, "Roughness", 0.70)
    return m


def mat_wax(color=(0.93, 0.89, 0.78, 1.0)):
    m, nt, bsdf = _new_material("Wax")
    _inp(bsdf, "Base Color", color)
    _inp(bsdf, "Roughness", 0.32)
    _inp(bsdf, "Subsurface Weight", 0.12)
    _inp(bsdf, "Subsurface Radius", (1.0, 0.55, 0.30))
    _inp(bsdf, "Coat Weight", 0.12)
    return m


def mat_leather(color=(0.13, 0.07, 0.035, 1.0)):
    m, nt, bsdf = _new_material("Leather")
    _inp(bsdf, "Base Color", color)
    _inp(bsdf, "Roughness", 0.50)
    _inp(bsdf, "Coat Weight", 0.08)
    n = _noise(nt, scale=110.0, detail=2.0)
    _bump(nt, bsdf, n.outputs["Fac"], strength=0.18, distance=0.012)
    return m


def mat_cloth_striped(base=(0.93, 0.93, 0.91, 1.0), stripe=(0.07, 0.08, 0.12, 1.0),
                      stripes=7, stripe_width=0.4, axis="X"):
    m, nt, bsdf = _new_material("ClothStriped")
    tc = _texcoord(nt)
    wave = nt.nodes.new("ShaderNodeTexWave")
    wave.location = (-760, 140)
    wave.wave_type = "BANDS"
    wave.bands_direction = axis
    if "Scale" in wave.inputs:
        wave.inputs["Scale"].default_value = max(stripes, 1)
    if "Distortion" in wave.inputs:
        wave.inputs["Distortion"].default_value = 1.5
    if "Detail" in wave.inputs:
        wave.inputs["Detail"].default_value = 2.0
    ramp = _ramp(nt, [(0.0, stripe), (stripe_width, base), (1.0, base)])
    ramp.color_ramp.interpolation = "CONSTANT"
    nt.links.new(tc.outputs["Generated"], wave.inputs["Vector"])
    nt.links.new(wave.outputs["Fac"], ramp.inputs["Fac"])
    nt.links.new(ramp.outputs["Color"], bsdf.inputs["Base Color"])
    weave = _noise(nt, scale=200.0, detail=2.0, y=-320)
    _bump(nt, bsdf, weave.outputs["Fac"], strength=0.15, distance=0.006)
    _inp(bsdf, "Roughness", 0.85)
    _inp(bsdf, "Sheen Weight", 0.50)
    return m


def mat_velvet(color=(0.28, 0.03, 0.07, 1.0)):
    m, nt, bsdf = _new_material("Velvet")
    _inp(bsdf, "Base Color", color)
    _inp(bsdf, "Roughness", 0.75)
    _inp(bsdf, "Sheen Weight", 1.0)
    _inp(bsdf, "Sheen Roughness", 0.35)
    fuzz = _noise(nt, scale=250.0, detail=2.0)
    _bump(nt, bsdf, fuzz.outputs["Fac"], strength=0.25, distance=0.008)
    return m


def mat_horn():
    m, nt, bsdf = _new_material("Horn")
    tc = _texcoord(nt)
    sep = nt.nodes.new("ShaderNodeSeparateXYZ")
    sep.location = (-760, 140)
    ramp = _ramp(nt, [(0.0, (0.86, 0.79, 0.63, 1.0)),
                      (1.0, (0.55, 0.32, 0.12, 1.0))])
    nt.links.new(tc.outputs["Generated"], sep.inputs["Vector"])
    nt.links.new(sep.outputs["Z"], ramp.inputs["Fac"])
    nt.links.new(ramp.outputs["Color"], bsdf.inputs["Base Color"])
    ridges = nt.nodes.new("ShaderNodeTexWave")
    ridges.location = (-760, -160)
    ridges.wave_type = "BANDS"
    ridges.bands_direction = "Z"
    if "Scale" in ridges.inputs:
        ridges.inputs["Scale"].default_value = 8.0
    if "Distortion" in ridges.inputs:
        ridges.inputs["Distortion"].default_value = 4.0
    nt.links.new(tc.outputs["Generated"], ridges.inputs["Vector"])
    _bump(nt, bsdf, ridges.outputs["Fac"], strength=0.18, distance=0.04)
    _inp(bsdf, "Roughness", 0.35)
    _inp(bsdf, "Subsurface Weight", 0.05)
    return m


def mat_stone():
    m, nt, bsdf = _new_material("Stone")
    tc = _texcoord(nt)
    noise = _noise(nt, scale=6.0, detail=8.0, roughness=0.8)
    ramp = _ramp(nt, [(0.0, (0.62, 0.56, 0.46, 1.0)),
                      (1.0, (0.80, 0.74, 0.63, 1.0))])
    nt.links.new(tc.outputs["Generated"], noise.inputs["Vector"])
    nt.links.new(noise.outputs["Fac"], ramp.inputs["Fac"])
    nt.links.new(ramp.outputs["Color"], bsdf.inputs["Base Color"])
    vor = nt.nodes.new("ShaderNodeTexVoronoi")
    vor.location = (-700, -300)
    vor.distance = "EUCLIDEAN"
    vor.feature = "DISTANCE_TO_EDGE"
    if "Scale" in vor.inputs:
        vor.inputs["Scale"].default_value = 4.0
    nt.links.new(tc.outputs["Generated"], vor.inputs["Vector"])
    _bump(nt, bsdf, vor.outputs["Distance"], strength=0.35, distance=0.12)
    _inp(bsdf, "Roughness", 0.90)
    return m


def mat_bread():
    m, nt, bsdf = _new_material("Bread")
    tc = _texcoord(nt)
    noise = _noise(nt, scale=3.0, detail=6.0, roughness=0.7, distortion=0.2)
    ramp = _ramp(nt, [(0.0, (0.32, 0.14, 0.04, 1.0)),
                      (0.55, (0.62, 0.34, 0.10, 1.0)),
                      (1.0, (0.78, 0.52, 0.20, 1.0))])
    nt.links.new(tc.outputs["Generated"], noise.inputs["Vector"])
    nt.links.new(noise.outputs["Fac"], ramp.inputs["Fac"])
    nt.links.new(ramp.outputs["Color"], bsdf.inputs["Base Color"])
    _bump(nt, bsdf, noise.outputs["Fac"], strength=0.30, distance=0.05)
    _inp(bsdf, "Roughness", 0.55)
    _inp(bsdf, "Subsurface Weight", 0.10)
    return m


def mat_matzah():
    m, nt, bsdf = _new_material("Matzah")
    tc = _texcoord(nt)
    noise = _noise(nt, scale=3.5, detail=4.0)
    ramp = _ramp(nt, [(0.0, (0.82, 0.74, 0.58, 1.0)),
                      (0.55, (0.72, 0.60, 0.42, 1.0)),
                      (0.80, (0.55, 0.38, 0.20, 1.0)),
                      (1.0, (0.45, 0.28, 0.13, 1.0))])
    nt.links.new(tc.outputs["Generated"], noise.inputs["Vector"])
    nt.links.new(noise.outputs["Fac"], ramp.inputs["Fac"])
    nt.links.new(ramp.outputs["Color"], bsdf.inputs["Base Color"])
    _bump(nt, bsdf, noise.outputs["Fac"], strength=0.40, distance=0.05)
    _inp(bsdf, "Roughness", 0.80)
    return m


def mat_flame(color=(1.0, 0.62, 0.15, 1.0)):
    m, nt, bsdf = _new_material("Flame")
    _inp(bsdf, "Base Color", (0.25, 0.06, 0.0, 1.0))
    _inp(bsdf, "Roughness", 1.0)
    _inp(bsdf, "Emission Color", color)
    _inp(bsdf, "Emission Strength", 6.0)
    return m


def mat_glass(color=(0.96, 1.0, 0.98, 1.0), roughness=0.06):
    m, nt, bsdf = _new_material("Glass")
    _inp(bsdf, "Base Color", color)
    _inp(bsdf, "Roughness", roughness)
    _inp(bsdf, "IOR", 1.45)
    _inp(bsdf, "Transmission Weight", 1.0)
    return m


def mat_wine(color=(0.16, 0.004, 0.012, 1.0)):
    m, nt, bsdf = _new_material("Wine")
    _inp(bsdf, "Base Color", color)
    _inp(bsdf, "Roughness", 0.06)
    _inp(bsdf, "IOR", 1.35)
    _inp(bsdf, "Transmission Weight", 0.2)
    _inp(bsdf, "Coat Weight", 0.5)
    _inp(bsdf, "Coat Roughness", 0.08)
    return m
