"""Decor: deep-red wax seal stamped with a Star of David (both themes)."""

import math
import os
import sys

import bpy

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import emblemkit

SEG = 64
# Plateau the stamp sits on, and how far the stamp stands proud of it.
PLATEAU_Y = -0.15
STAMP_FRONT = -0.225


def jitter(i, salt):
    v = math.sin(i * 12.9898 + salt * 78.233) * 43758.5453
    return 2.0 * (v - math.floor(v)) - 1.0


def lobe(i, seg, salt):
    a = 2.0 * math.pi * i / seg
    # Broad puddle lobes. Returned as a fraction of radius, not a 0-1 mask.
    return (0.16 * math.sin(3.0 * a + salt)
            + 0.09 * math.sin(5.0 * a + salt * 1.7)
            + 0.045 * jitter(i, salt))


def wax_blob(mat):
    """Irregular puddle. Centre plateau faces the camera; rim is lumpy."""
    # (y, radius scale). y decreases toward the camera.
    rings = [
        (0.06, 0.25, 0.02, 3.0),
        (-0.02, 0.85, 0.04, 5.0),
        (-0.08, 1.05, 0.07, 9.0),    # widest, most irregular
        (-0.12, 0.98, 0.06, 13.0),
        (-0.145, 0.72, 0.03, 17.0),
        (PLATEAU_Y, 0.55, 0.012, 19.0),
    ]
    verts = [(0.0, 0.10, 0.0)]  # back centre
    for y, rf, amp, salt in rings:
        for i in range(SEG):
            a = 2.0 * math.pi * i / SEG
            # Outer rings use the full puddle lobe; the plateau stays round
            # so the stamp has a flat seat.
            wobble = lobe(i, SEG, salt) if amp > 0.03 else amp * jitter(i, salt)
            r = 0.62 * rf * (1.0 + wobble)
            yj = y + (0.025 * math.sin(4.0 * a + salt) if amp > 0.03 else 0.0)
            verts.append((r * math.cos(a), yj, r * math.sin(a)))
    verts.append((0.0, PLATEAU_Y - 0.008, 0.0))  # shallow dome centre
    faces = []
    back = 0
    first = 1
    for i in range(SEG):
        i2 = (i + 1) % SEG
        faces.append((back, first + i, first + i2))
    for ring in range(len(rings) - 1):
        r0 = 1 + ring * SEG
        r1 = r0 + SEG
        for i in range(SEG):
            i2 = (i + 1) % SEG
            faces.append((r0 + i, r1 + i, r1 + i2, r0 + i2))
    front = len(verts) - 1
    last = 1 + (len(rings) - 1) * SEG
    for i in range(SEG):
        i2 = (i + 1) % SEG
        faces.append((front, last + i2, last + i))
    mesh = bpy.data.meshes.new("wax_mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    ob = bpy.data.objects.new("wax", mesh)
    bpy.context.collection.objects.link(ob)
    common.assign(ob, mat)
    common.smooth(ob)
    return ob


def triangle_frame(name, r_out, r_in, y_front, depth, rot, mat):
    """Bold equilateral triangle outline, extruded toward the camera."""
    verts = []
    for y in (y_front + depth, y_front):
        for r in (r_out, r_in):
            for k in range(3):
                a = math.radians(90.0 + 120.0 * k) + rot
                verts.append((r * math.cos(a), y, r * math.sin(a)))
    faces = []
    for k in range(3):
        k2 = (k + 1) % 3
        faces.append((6 + k, 9 + k, 9 + k2, 6 + k2))
        faces.append((k, k2, 3 + k2, 3 + k))
        faces.append((k, 6 + k, 6 + k2, k2))
        faces.append((3 + k2, 9 + k2, 9 + k, 3 + k))
    mesh = bpy.data.meshes.new(name + "_mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    ob = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(ob)
    common.assign(ob, mat)
    common.bevel(ob, 0.010, segments=2)
    return ob


def build():
    common.reset_scene()
    common.setup_render()
    wax = emblemkit.seal_wax((0.20, 0.008, 0.006, 1.0))
    # Darker than the puddle so the impression reads even nearly head-on.
    stamp = emblemkit.seal_wax((0.055, 0.003, 0.003, 1.0))
    rim_mat = emblemkit.seal_wax((0.09, 0.004, 0.004, 1.0))

    objs = [wax_blob(wax)]
    # Raised border of the stamp, then the star inside it.
    border = common.add_torus(
        "stamp_ring", 0.40, 0.028,
        location=(0.0, STAMP_FRONT + 0.03, 0.0),
        mat=rim_mat, major_segments=64, minor_segments=12)
    border.rotation_euler = (math.radians(90), 0, 0)
    objs.append(border)

    # Point-up and point-down. 180 deg, not 60, is the same vertex set for an
    # equilateral triangle; use pi so the intent is obvious.
    # Positive depth: back of the stamp embeds in the plateau, front stands proud.
    depth = (PLATEAU_Y + 0.02) - STAMP_FRONT
    objs.append(triangle_frame(
        "tri_up", 0.34, 0.22, STAMP_FRONT, depth, 0.0, stamp))
    objs.append(triangle_frame(
        "tri_down", 0.34, 0.22, STAMP_FRONT, depth, math.pi, stamp))

    common.frame_object(objs, margin=0.12, focal=78.0,
                        elevation=18.0, azimuth=20.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render(common.variant_name("seal"), common.OUT_DIR)


build()
