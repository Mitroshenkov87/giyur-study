"""Tests icon: one idea — a brass check-mark seal on a short parchment scroll.

The seal is the whole read at 40 px. The scroll is only the sheet it sits on.
"""

import math
import os
import sys

from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import iconkit

# Seal face aims up toward the 3/4 camera, a little steeper than the lens
# so the check is the flat shape and the rim still catches the key light.
_EL = math.radians(42.0)
_AZ = math.radians(28.0)
_FACE = Vector((
    math.sin(_AZ) * math.cos(_EL),
    -math.cos(_AZ) * math.cos(_EL),
    math.sin(_EL),
))


def _check(metal):
    """One thick check. Arms overlap past the vertex so the joint is solid."""
    vertex = (0.02, -0.10)
    z = 0.18
    stroke = 0.20
    depth = 0.12
    # (angle from +X, length). Short arm up-left, long arm up-right.
    arms = ((math.radians(122.0), 0.36), (math.radians(50.0), 0.56))
    out = []
    overlap = stroke * 0.70
    for ang, length in arms:
        span = length + overlap
        center_dist = (length - overlap) * 0.5
        cx = vertex[0] + math.cos(ang) * center_dist
        cy = vertex[1] + math.sin(ang) * center_dist
        out.append(iconkit.round_box(
            "check", (span, stroke, depth), metal,
            location=(cx, cy, z),
            bevel=0.045, segments=2,
            rotation=(0.0, 0.0, ang)))
    return out


def _seal(metal, accent):
    """Raised brass disc, dark inset, check standing proud of both."""
    disc = common.add_cylinder(
        "seal_disc", 0.70, 0.14, location=(0.0, 0.0, 0.0), vertices=48, mat=metal)
    common.bevel(disc, 0.03, segments=2)
    inset = common.add_cylinder(
        "seal_inset", 0.52, 0.07, location=(0.0, 0.0, 0.08), vertices=40, mat=accent)
    common.bevel(inset, 0.016, segments=1)
    return [disc, inset] + _check(metal)


def _document(parchment):
    """One sheet with the top edge rolled. The seal sits in the middle of it."""
    sheet = iconkit.round_box(
        "sheet", (1.85, 2.05, 0.09), parchment,
        location=(0.0, -0.06, 0.0), bevel=0.04, segments=2)
    roll = common.add_cylinder(
        "roll", 0.20, 1.72, location=(0.0, 0.90, 0.06), vertices=28, mat=parchment)
    roll.rotation_euler[1] = math.radians(90)
    common.smooth(roll)
    return [sheet, roll]


def build():
    common.reset_scene()
    common.setup_render()
    metal = common.theme_metal()
    accent = iconkit.accent()
    parchment = iconkit.mat_parchment()

    # Sheet, roll and seal share one facing so the check is stamped on the page.
    root = iconkit.empty(
        "doc", rotation=_FACE.to_track_quat("Z", "Y").to_euler())
    parts = _document(parchment) + _seal(metal, accent)
    # Seal disc is centred; lift it off the sheet so the check stands proud.
    for ob in parts:
        if ob.name.startswith("seal") or ob.name.startswith("check"):
            ob.location.z += 0.10
            ob.location.y -= 0.08
        iconkit.parent_keep(ob, root)

    common.frame_object(parts, margin=0.12, focal=68.0, elevation=28.0, azimuth=30.0)
    common.setup_lights()
    common.render(common.variant_name("tests"), common.OUT_DIR)


build()
