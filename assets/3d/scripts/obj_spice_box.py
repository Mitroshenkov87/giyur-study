"""Havdalah besamim (spice box): silver tower with arched filigree windows,
flared roof, ball finial and a small pennant flag (no cross)."""

import math
import os
import sys

import bpy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common

BODY_Z0 = 0.18
BODY_H = 0.85
N_WINDOWS = 6


def build():
    common.reset_scene()
    common.setup_render()
    silver = common.mat_silver()
    dark = common.mat_leather((0.03, 0.025, 0.02, 1.0))
    flag_mat = common.mat_velvet()

    objs = []

    # stepped plinth
    plinth1 = common.add_cylinder("plinth_low", 0.50, 0.08,
                                  location=(0, 0, 0.04), vertices=64, mat=silver)
    common.bevel(plinth1, 0.020, segments=3)
    plinth2 = common.add_cylinder("plinth_high", 0.42, 0.12,
                                  location=(0, 0, 0.12), vertices=64, mat=silver)
    common.bevel(plinth2, 0.020, segments=3)
    objs += [plinth1, plinth2]

    # tapered tower body
    body = common.add_cone("tower", 0.36, 0.30, BODY_H,
                           location=(0, 0, BODY_Z0 + BODY_H * 0.5),
                           vertices=64, mat=silver)
    common.bevel(body, 0.012, segments=2)
    common.smooth(body)
    objs.append(body)

    # decorative bands around the body
    for z, r in ((0.26, 0.352), (0.96, 0.314)):
        objs.append(common.add_torus("band", r, 0.014, location=(0, 0, z),
                                     mat=silver))

    # arched windows: dark recessed panels with rounded tops
    r_surf = 0.306   # panel outer face ends up ~flush with the wall
    for k in range(N_WINDOWS):
        a = 2.0 * math.pi * k / N_WINDOWS
        ca, sa = math.cos(a), math.sin(a)
        rot = a - math.pi / 2.0          # local Y (compressed) -> radial dir
        zc = BODY_Z0 + 0.42
        # tall thin panel: vertical-axis cylinder, squashed along its local Y
        win = common.add_cylinder("window", 0.058, 0.26,
                                  location=(r_surf * ca, r_surf * sa, zc),
                                  vertices=24, mat=dark)
        win.scale = (1.0, 0.45, 1.0)
        win.rotation_euler = (0.0, 0.0, rot)
        objs.append(win)
        # half-dome arch capping each panel
        dome = common.add_uv_sphere("window_arch", 0.058,
                                    location=(r_surf * ca, r_surf * sa,
                                              zc + 0.13),
                                    scale=(1.0, 0.45, 1.0), mat=dark,
                                    segments=24, rings=12)
        dome.rotation_euler = (0.0, 0.0, rot)
        objs.append(dome)
        # silver rosette between windows
        ra = a + math.pi / N_WINDOWS
        objs.append(common.add_uv_sphere(
            "rosette", 0.022,
            location=(0.348 * math.cos(ra), 0.348 * math.sin(ra), 0.34),
            mat=silver, segments=16, rings=8))

    # flared eaves + pointed spire
    eaves_prof = [
        (0.0, 1.02), (0.40, 1.03), (0.50, 1.06),
        (0.48, 1.11), (0.30, 1.15), (0.0, 1.15),
    ]
    eaves = common.lathe("eaves", eaves_prof, segments=72, mat=silver)
    common.bevel(eaves, 0.008, segments=2)
    objs.append(eaves)
    spire = common.add_cone("spire", 0.29, 0.02, 0.50,
                            location=(0, 0, 1.40), vertices=64, mat=silver)
    common.smooth(spire)
    objs.append(spire)

    # ball finial + small pennant flag
    objs.append(common.add_uv_sphere("finial_ball", 0.055,
                                     location=(0, 0, 1.685), mat=silver))
    objs.append(common.add_cylinder("flag_pole", 0.012, 0.20,
                                    location=(0, 0, 1.80), vertices=12,
                                    mat=silver))
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=(0.085, 0, 1.86))
    flag = bpy.context.object
    flag.name = "flag"
    flag.scale = (0.075, 0.009, 0.050)
    bpy.ops.object.transform_apply(scale=True)
    common.assign(flag, flag_mat)
    common.bevel(flag, 0.008, segments=2)
    objs.append(flag)

    common.frame_object(objs, margin=0.12, elevation=18.0, azimuth=30.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render("spice_box", common.OUT_DIR)


build()
