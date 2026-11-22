"""Shared medallion builder for emblem_* scripts.

Each emblem is a round coin-like medallion (diameter 2, thickness ~0.18)
of theme_metal() with a raised bezel rim and a theme_accent() inner disc
inset slightly below the front face.  The medallion stands in the XZ plane
(like the other objects) with its front face toward the camera at -Y;
relief symbols are built on the inner disc starting at emblemkit.RELIEF_Y
and extruded ~0.08 toward the camera so they stay below the bezel top.
"""

import math

import bpy

import common

COIN_R = 1.0
COIN_T = 0.18
DISC_R = 0.80
DISC_FRONT = 0.025         # disc face, recessed behind the coin front (y=0)
RELIEF_Y = DISC_FRONT      # relief base plane (rearmost y)
RELIEF_D = 0.08            # nominal relief extrusion depth toward camera
R_Y = RELIEF_Y - RELIEF_D * 0.5   # y centre for relief solids


def medallion():
    """Build coin + bezel + inset accent disc.

    Returns (objs, metal, accent).  Relief z0/y helpers use module constants."""
    metal = common.theme_metal()
    accent = common.theme_accent()
    objs = []

    coin = common.add_cylinder("coin", COIN_R, COIN_T,
                               location=(0, -COIN_T * 0.5, 0),
                               vertices=96, mat=metal)
    coin.rotation_euler = (math.radians(90), 0, 0)
    common.bevel(coin, 0.025, segments=3)
    objs.append(coin)

    # raised bezel on the front face; back rim finishes the reverse
    bezel = common.add_torus("bezel", 0.905, 0.085,
                             location=(0, 0, 0), mat=metal,
                             major_segments=96, minor_segments=20)
    bezel.rotation_euler = (math.radians(90), 0, 0)
    objs.append(bezel)
    back = common.add_torus("back_rim", 0.93, 0.05,
                            location=(0, -COIN_T, 0), mat=metal,
                            major_segments=96, minor_segments=16)
    back.rotation_euler = (math.radians(90), 0, 0)
    objs.append(back)

    disc = common.add_cylinder("inner_disc", DISC_R, 0.10,
                               location=(0, DISC_FRONT - 0.05, 0),
                               vertices=96, mat=accent)
    disc.rotation_euler = (math.radians(90), 0, 0)
    common.bevel(disc, 0.015, segments=2)
    objs.append(disc)

    return objs, metal, accent


def extrude_polygon(name, pts, y_base, depth, mat, bevel_width=0.012):
    """Flat extruded polygon (list of (x, z), CCW as seen from the camera)
    from y_base - depth to y_base."""
    n = len(pts)
    verts = [(x, y_base, z) for x, z in pts] + [(x, y_base - depth, z)
                                                for x, z in pts]
    faces = [tuple(range(n - 1, -1, -1)), tuple(range(n, 2 * n))]
    for i in range(n):
        j = (i + 1) % n
        faces.append((i, j, n + j, n + i))
    mesh = bpy.data.meshes.new(name + "_mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    ob = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(ob)
    common.assign(ob, mat)
    if bevel_width:
        common.bevel(ob, bevel_width, segments=2)
    return ob


def ellipse_pts(cx, cz, rx, rz, n=48, a0=0.0, a1=2.0 * math.pi):
    """CCW (as seen from the camera) ellipse/arc outline points."""
    return [(cx + rx * math.cos(a0 + (a1 - a0) * i / n),
             cz + rz * math.sin(a0 + (a1 - a0) * i / n)) for i in range(n)]


def finish(objs, base_name, margin=0.07):
    """Frame the medallion nearly frontal, light it, render the variant."""
    common.frame_object(objs, margin=margin, focal=85.0,
                        elevation=10.0, azimuth=15.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render(common.variant_name(base_name), common.OUT_DIR)
