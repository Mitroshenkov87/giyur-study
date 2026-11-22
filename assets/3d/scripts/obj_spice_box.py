"""Havdalah besamim box: antique-silver tower with real arched cut-outs,
filigree sitting in the openings, dark interior, and a small pennant.
No cross. Base colour stays well under blown-out white."""

import math
import os
import sys

import bpy
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common

N_BAYS = 6
BODY_R = 0.36
BODY_ZC = 0.70
BODY_H = 0.74


def mat_antique_silver():
    m, nt, bsdf = common._new_material("AntiqueSilver")
    geo = nt.nodes.new("ShaderNodeNewGeometry")
    geo.location = (-1100, 40)
    noise = common._noise(nt, scale=5.5, detail=4.0, roughness=0.55)
    nt.links.new(geo.outputs["Position"], noise.inputs["Vector"])
    ramp = common._ramp(nt, [
        (0.00, (0.07, 0.065, 0.055, 1.0)),
        (0.45, (0.16, 0.165, 0.17, 1.0)),
        (1.00, (0.30, 0.31, 0.32, 1.0)),
    ])
    nt.links.new(noise.outputs["Fac"], ramp.inputs["Fac"])

    ao = nt.nodes.new("ShaderNodeAmbientOcclusion")
    ao.location = (-700, -260)
    if "Distance" in ao.inputs:
        ao.inputs["Distance"].default_value = 0.09
    shade = nt.nodes.new("ShaderNodeMapRange")
    shade.location = (-460, -260)
    shade.inputs["To Min"].default_value = 0.28
    shade.inputs["To Max"].default_value = 1.0
    ao_out = "AO" if "AO" in ao.outputs else "Color"
    nt.links.new(ao.outputs[ao_out], shade.inputs["Value"])
    mul = nt.nodes.new("ShaderNodeMixRGB")
    mul.location = (-160, 20)
    mul.blend_type = "MULTIPLY"
    mul.inputs[0].default_value = 1.0
    nt.links.new(ramp.outputs["Color"], mul.inputs[1])
    nt.links.new(shade.outputs["Result"], mul.inputs[2])
    nt.links.new(mul.outputs["Color"], bsdf.inputs["Base Color"])

    common._bump(nt, bsdf, noise.outputs["Fac"], strength=0.12, distance=0.008)
    rough = nt.nodes.new("ShaderNodeMapRange")
    rough.location = (-160, -180)
    rough.inputs["To Min"].default_value = 0.38
    rough.inputs["To Max"].default_value = 0.72
    nt.links.new(noise.outputs["Fac"], rough.inputs["Value"])
    nt.links.new(rough.outputs["Result"], bsdf.inputs["Roughness"])
    common._inp(bsdf, "Metallic", 1.0)
    return m


def mat_void():
    m, nt, bsdf = common._new_material("SpiceDark")
    common._inp(bsdf, "Base Color", (0.015, 0.012, 0.010, 1.0))
    common._inp(bsdf, "Roughness", 0.95)
    return m


def _arch_cutter(name, width, height, depth):
    """Closed arch prism. Profile in local XZ, extruded along local Y."""
    hw = width * 0.5
    rad = hw * 0.92
    rect = max(0.02, height - rad)
    z0 = -height * 0.5
    profile = [(-hw, z0), (-hw, z0 + rect)]
    segs = 8
    for i in range(1, segs):
        a = math.pi * (1.0 - i / segs)
        profile.append((rad * math.cos(a), z0 + rect + rad * math.sin(a)))
    profile.append((hw, z0 + rect))
    profile.append((hw, z0))
    n = len(profile)
    verts = []
    for y in (-depth * 0.5, depth * 0.5):
        for x, z in profile:
            verts.append((x, y, z))
    faces = []
    for i in range(n):
        j = (i + 1) % n
        faces.append((i, j, n + j, n + i))
    faces.append(tuple(range(n - 1, -1, -1)))
    faces.append(tuple(range(n, 2 * n)))
    mesh = bpy.data.meshes.new(name + "_mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    ob = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(ob)
    return ob


def _point_y(obj, radial):
    obj.rotation_euler = Vector(radial).to_track_quat("Y", "Z").to_euler()


def _boolean_difference(target, cutters):
    bpy.ops.object.select_all(action="DESELECT")
    for c in cutters:
        c.select_set(True)
    bpy.context.view_layer.objects.active = cutters[0]
    if len(cutters) > 1:
        bpy.ops.object.join()
    cutter = bpy.context.view_layer.objects.active
    mod = target.modifiers.new("openings", "BOOLEAN")
    mod.operation = "DIFFERENCE"
    mod.object = cutter
    for solver in ("MANIFOLD", "EXACT", "FLOAT"):
        try:
            mod.solver = solver
            break
        except (TypeError, ValueError):
            continue
    bpy.ops.object.select_all(action="DESELECT")
    bpy.context.view_layer.objects.active = target
    target.select_set(True)
    bpy.ops.object.modifier_apply(modifier=mod.name)
    target.select_set(False)
    bpy.data.objects.remove(cutter, do_unlink=True)
    return target


def _filigree(objs, angle, silver):
    """Diamond and ring sitting in the opening, smaller than the arch."""
    radial = Vector((math.cos(angle), math.sin(angle), 0.0))
    tangent = Vector((-math.sin(angle), math.cos(angle), 0.0))
    up = Vector((0.0, 0.0, 1.0))
    center = radial * (BODY_R - 0.02) + Vector((0.0, 0.0, 0.70))

    def at(x, z):
        return center + tangent * x + up * z

    diamond = [(0.0, 0.09), (0.055, 0.0), (0.0, -0.07), (-0.055, 0.0)]
    for i in range(4):
        a = at(*diamond[i])
        b = at(*diamond[(i + 1) % 4])
        objs.append(common.tube_curve("fil_bar", [a, b], 0.008, mat=silver, resolution=2))
    ring = common.add_torus("fil_ring", 0.028, 0.006, location=center, mat=silver,
                            major_segments=28, minor_segments=8)
    ring.rotation_euler = radial.to_track_quat("Z", "Y").to_euler()
    objs.append(ring)


def _pennant(mat):
    nu, nv = 9, 4
    length, height = 0.34, 0.16
    verts = []
    for j in range(nv):
        v = j / (nv - 1)
        for i in range(nu):
            u = i / (nu - 1)
            z = (v - 0.5) * height * (1.0 - 0.82 * u)
            x = u * length
            y = 0.016 * math.sin(u * math.pi) 
            verts.append((x, y, z))
    faces = []
    for j in range(nv - 1):
        for i in range(nu - 1):
            a = j * nu + i
            faces.append((a, a + 1, a + nu + 1, a + nu))
    faces += [tuple(reversed(f)) for f in list(faces)]
    mesh = bpy.data.meshes.new("flag_mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    ob = bpy.data.objects.new("flag", mesh)
    bpy.context.collection.objects.link(ob)
    common.assign(ob, mat)
    common.smooth(ob)
    return ob


def build():
    common.reset_scene()
    common.setup_render()
    silver = mat_antique_silver()
    dark = mat_void()
    flag_mat = common.mat_velvet((0.42, 0.035, 0.04, 1.0))

    objs = []

    plinth1 = common.add_cylinder("plinth_low", 0.52, 0.09,
                                  location=(0, 0, 0.045), vertices=64, mat=silver)
    common.bevel(plinth1, 0.016, segments=2)
    plinth2 = common.add_cylinder("plinth_mid", 0.44, 0.12,
                                  location=(0, 0, 0.14), vertices=64, mat=silver)
    common.bevel(plinth2, 0.014, segments=2)
    plinth3 = common.add_cylinder("plinth_top", 0.39, 0.12,
                                  location=(0, 0, 0.25), vertices=64, mat=silver)
    common.bevel(plinth3, 0.012, segments=2)
    objs += [plinth1, plinth2, plinth3]

    # Boolean difference left a watertight shell, so the windows are built
    # as the gaps between posts and arches instead of cut out of a solid.
    R = 0.34
    sill = common.add_cylinder("sill", 0.38, 0.11, location=(0, 0, 0.385),
                               vertices=64, mat=silver)
    common.bevel(sill, 0.008, segments=1)
    capband = common.add_cylinder("capband", 0.38, 0.11, location=(0, 0, 0.97),
                                  vertices=64, mat=silver)
    common.bevel(capband, 0.008, segments=1)
    objs += [sill, capband]
    for k in range(N_BAYS):
        a = 2.0 * math.pi * k / N_BAYS
        objs.append(common.add_cylinder(
            "post", 0.042, 0.50,
            location=(R * math.cos(a), R * math.sin(a), 0.67),
            vertices=16, mat=silver))
        a2 = 2.0 * math.pi * (k + 1) / N_BAYS
        mid = (a + a2) * 0.5
        tangent = Vector((-math.sin(mid), math.cos(mid), 0.0))
        radial = Vector((math.cos(mid), math.sin(mid), 0.0))
        center = radial * R + Vector((0.0, 0.0, 0.76))
        pts = []
        for i in range(16):
            t = math.pi * i / 15.0
            pts.append(center + tangent * math.cos(t) * 0.15
                       + Vector((0.0, 0.0, math.sin(t) * 0.17)))
        objs.append(common.tube_curve("arch", pts, 0.016, mat=silver, resolution=3))
        # Small open ring above the arch.
        oculus = common.add_torus(
            "oculus", 0.042, 0.011,
            location=(radial * 0.33) + Vector((0.0, 0.0, 0.90)),
            mat=silver, major_segments=24, minor_segments=8)
        oculus.rotation_euler = radial.to_track_quat("Z", "Y").to_euler()
        objs.append(oculus)

    chamber = common.add_cylinder("chamber", 0.25, 0.58,
                                  location=(0, 0, 0.68), vertices=48, mat=dark)
    objs.append(chamber)

    for k in range(N_BAYS):
        a = 2.0 * math.pi * (k + 0.5) / N_BAYS
        _filigree(objs, a, silver)

    objs.append(common.add_torus("band_low", 0.375, 0.014, location=(0, 0, 0.36), mat=silver))
    objs.append(common.add_torus("band_high", 0.375, 0.016, location=(0, 0, 1.05), mat=silver))

    eaves = common.lathe("eaves", [
        (0.0, 1.02), (0.30, 1.03), (0.42, 1.06), (0.50, 1.10),
        (0.46, 1.15), (0.28, 1.18), (0.0, 1.18),
    ], segments=72, mat=silver)
    objs.append(eaves)
    spire = common.add_cone("spire", 0.26, 0.018, 0.48,
                            location=(0, 0, 1.40), vertices=48, mat=silver)
    common.smooth(spire)
    objs.append(spire)
    objs.append(common.add_uv_sphere("finial", 0.048, location=(0, 0, 1.67), mat=silver))
    objs.append(common.add_cylinder("flag_pole", 0.012, 0.28,
                                    location=(0, 0, 1.86), vertices=12, mat=silver))
    flag = _pennant(flag_mat)
    flag.location = (0.012, 0.0, 1.98)
    # Fly the pennant toward the camera so it reads against the spire.
    flag.rotation_euler = (0.0, 0.0, math.radians(-50.0))
    objs.append(flag)

    common.frame_object(objs, margin=0.12, elevation=18.0, azimuth=28.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render("spice_box", common.OUT_DIR)


build()
