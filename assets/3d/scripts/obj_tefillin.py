"""Tefillin: matte black leather batim on titura bases, glossier black
retzuot coiled beside them. Shel rosh carries a lightly embossed shin."""

import math
import os
import sys

import bpy
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common

FONT = "/workspace/shorashim-wt-art/assets/fonts/NotoSerifHebrew-Regular.ttf"


def mat_box():
    """True black, nearly matte. No coat, so the warm rig cannot turn it brown."""
    m, nt, bsdf = common._new_material("TefillinBox")
    common._inp(bsdf, "Base Color", (0.012, 0.012, 0.013, 1.0))
    common._inp(bsdf, "Roughness", 0.93)
    common._inp(bsdf, "Specular IOR Level", 0.15)
    common._inp(bsdf, "Coat Weight", 0.0)
    n = common._noise(nt, scale=90.0, detail=2.0)
    common._bump(nt, bsdf, n.outputs["Fac"], strength=0.10, distance=0.003)
    return m


def mat_strap():
    """Black leather strap, smoother and a little glossy."""
    m, nt, bsdf = common._new_material("TefillinStrap")
    common._inp(bsdf, "Base Color", (0.018, 0.018, 0.020, 1.0))
    common._inp(bsdf, "Roughness", 0.26)
    common._inp(bsdf, "Coat Weight", 0.28)
    common._inp(bsdf, "Coat Roughness", 0.16)
    common._inp(bsdf, "Specular IOR Level", 0.45)
    n = common._noise(nt, scale=50.0, detail=2.0)
    common._bump(nt, bsdf, n.outputs["Fac"], strength=0.06, distance=0.002)
    return m


def box_on_titura(name, center_x, center_y, titura_w, bayit_s, leather, h=0.46):
    """Sharp-edged square titura with a sharp bayit cube. Bevel is only a hair."""
    titura = common.add_cylinder(
        name + "_titura", titura_w * 0.5 * math.sqrt(2.0), 0.065,
        location=(center_x, center_y, 0.032), vertices=4, mat=leather)
    titura.rotation_euler.z = math.radians(45.0)
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    common.bevel(titura, 0.004, segments=1)
    bayit = common.add_cylinder(
        name + "_bayit", bayit_s * 0.5 * math.sqrt(2.0), h,
        location=(center_x, center_y, 0.065 + h * 0.5), vertices=4, mat=leather)
    bayit.rotation_euler.z = math.radians(45.0)
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    common.bevel(bayit, 0.004, segments=1)
    return titura, bayit


def coil(center, r0, r1, turns, z0, rise, leather, radius=0.055):
    n = 100
    pts = []
    for i in range(n + 1):
        t = i / n
        ang = 2.0 * math.pi * turns * t
        r = r0 + (r1 - r0) * t
        pts.append(Vector((center[0] + r * math.cos(ang),
                           center[1] + r * math.sin(ang),
                           z0 + rise * t)))
    ob = common.tube_curve("coil", pts, radius, mat=leather, resolution=3)
    ob.scale.z = 0.30
    return ob


def strap_segment(name, pts, leather, radius=0.050):
    ob = common.tube_curve(name, pts, radius, mat=leather, resolution=3)
    ob.scale.z = 0.30
    return ob


def embossed_shin(box_mat, center, bayit_s, h):
    """Lightly raised shin on the face toward the camera (-Y)."""
    cu = bpy.data.curves.new("shin_curve", "FONT")
    cu.body = "\u05e9"
    cu.font = bpy.data.fonts.load(FONT)
    cu.size = 1.0
    cu.extrude = 0.010
    cu.bevel_depth = 0.0015
    cu.bevel_resolution = 1
    cu.resolution_u = 8
    cu.align_x = "CENTER"
    cu.align_y = "CENTER"
    shin = bpy.data.objects.new("shin", cu)
    bpy.context.collection.objects.link(shin)
    shin.rotation_euler = (math.radians(90.0), 0.0, 0.0)
    bpy.context.view_layer.update()
    # Height lands on Y or Z depending on whether the depsgraph has applied
    # the rotation; the thin axis is the extrude. Use the long axis.
    dims = shin.dimensions
    span = max(dims.x, dims.y, dims.z)
    scale = (bayit_s * 0.52) / max(span, 1e-4)
    shin.scale = (scale, scale, scale)
    print("shin dims", tuple(round(v, 4) for v in dims), "scale", round(scale, 4))
    # Face is the -Y side of the cube; sit just proud of it.
    shin.location = (center[0],
                     center[1] - bayit_s * 0.5 - 0.006,
                     0.065 + h * 0.52)
    common.assign(shin, box_mat)
    return shin


def build():
    common.reset_scene()
    common.setup_render()
    box = mat_box()
    strap = mat_strap()
    objs = []

    rosh_h = 0.48
    rosh_s = 0.50
    rosh_c = (-0.62, 0.10)
    rosh_t, rosh_b = box_on_titura("rosh", rosh_c[0], rosh_c[1], 0.70, rosh_s, box, h=rosh_h)
    objs += [rosh_t, rosh_b]
    objs.append(embossed_shin(box, rosh_c, rosh_s, rosh_h))

    yad_t, yad_b = box_on_titura("yad", 0.46, -0.20, 0.60, 0.40, box, h=0.40)
    objs += [yad_t, yad_b]

    # ma'abarta: the strap loop on the shel rosh
    loop_pts = []
    top = 0.065 + rosh_h
    for i in range(16):
        a = math.pi * i / 15.0
        loop_pts.append(Vector((rosh_c[0] + 0.06 * math.cos(a), rosh_c[1],
                                top + 0.09 * math.sin(a))))
    loop = strap_segment("rosh_loop", loop_pts, strap, radius=0.022)
    loop.scale.y = 0.45
    objs.append(loop)

    objs.append(strap_segment(
        "yad_tail",
        [Vector((0.46, -0.50, 0.055)), Vector((0.62, -0.66, 0.04)),
         Vector((0.95, -0.70, 0.035)), Vector((1.15, -0.58, 0.035))],
        strap))

    objs.append(coil((1.35, -0.22), 0.08, 0.32, 4.2, 0.045, 0.07, strap))
    objs.append(strap_segment(
        "coilA_tail",
        [Vector((1.35 + 0.32, -0.22, 0.10)),
         Vector((1.05, 0.08, 0.04)), Vector((0.78, 0.22, 0.035))],
        strap))
    objs.append(coil((-1.38, -0.52), 0.07, 0.26, 3.8, 0.04, 0.06, strap, radius=0.048))
    objs.append(strap_segment(
        "coilB_tail",
        [Vector((-1.38 - 0.22, -0.52, 0.09)),
         Vector((-1.05, -0.18, 0.04)), Vector((-0.82, -0.02, 0.035))],
        strap, radius=0.048))

    common.frame_object(objs, margin=0.10, focal=72.0, elevation=28.0, azimuth=34.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render("tefillin", common.OUT_DIR)


build()
