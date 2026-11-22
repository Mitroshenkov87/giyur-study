"""Two braided challot on a walnut board; one under a blue velvet cover
with gold trim."""

import math
import os
import sys

import bpy
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import foodkit

STRAND_R = 0.155
BOARD_TOP = 0.14


def mat_glazed_bread():
    """common.mat_bread plus an egg-wash sheen."""
    m, nt, bsdf = common._new_material("BreadGlazed")
    tc = common._texcoord(nt)
    noise = common._noise(nt, scale=3.0, detail=6.0, roughness=0.7, distortion=0.2)
    ramp = common._ramp(nt, [(0.0, (0.34, 0.16, 0.05, 1.0)),
                             (0.55, (0.66, 0.38, 0.12, 1.0)),
                             (1.0, (0.82, 0.56, 0.22, 1.0))])
    nt.links.new(tc.outputs["Generated"], noise.inputs["Vector"])
    nt.links.new(noise.outputs["Fac"], ramp.inputs["Fac"])
    nt.links.new(ramp.outputs["Color"], bsdf.inputs["Base Color"])
    common._bump(nt, bsdf, noise.outputs["Fac"], strength=0.28, distance=0.05)
    common._inp(bsdf, "Roughness", 0.42)
    common._inp(bsdf, "Subsurface Weight", 0.10)
    common._inp(bsdf, "Coat Weight", 0.22)
    common._inp(bsdf, "Coat Roughness", 0.25)
    return m


def strand_state(i, u, n_cycles):
    """3-strand plait: each strand cycles through tracks [0,1,2,1]."""
    return (u * 4.0 * n_cycles + i * 2.0) % 4.0


def braid(name, center, angle, length, n_cycles, mat, seed_mat=None):
    ca, sa = math.cos(angle), math.sin(angle)

    def point(i, u):
        pos = strand_state(i, u, n_cycles)
        k = int(pos)
        f = foodkit.smoothstep(pos - k)
        track_seq = (0.0, 1.0, 2.0, 1.0)
        t0 = track_seq[k % 4]
        t1 = track_seq[(k + 1) % 4]
        y = (t0 + (t1 - t0) * f - 1.0) * 0.82 * STRAND_R
        pinch = foodkit.smoothstep(min(u, 1.0 - u) / 0.10)
        lift = 0.55 * STRAND_R * math.cos(math.pi * pos * 0.5) * pinch
        z = STRAND_R + max(lift, -0.3 * STRAND_R)
        x = -0.5 * length + length * u
        wl = math.sin(9.1 * u + i * 2.4) * 0.02 * STRAND_R * pinch
        xr = x * ca - (y + wl) * sa
        yr = x * sa + (y + wl) * ca
        return Vector((center[0] + xr, center[1] + yr, center[2] + z))

    objs = []
    npts = 64
    for i in range(3):
        pts = [point(i, j / npts) for j in range(npts + 1)]
        objs.append(common.tube_curve("braid_strand", pts, STRAND_R, mat=mat))
        objs.append(common.add_uv_sphere("braid_tip", STRAND_R * 0.98,
                                         location=point(i, 0.0), mat=mat,
                                         segments=24, rings=12))
        objs.append(common.add_uv_sphere("braid_tip", STRAND_R * 0.98,
                                         location=point(i, 1.0), mat=mat,
                                         segments=24, rings=12))
    if seed_mat:
        idx = 0
        for i in range(3):
            for j in range(13):
                u = 0.08 + 0.84 * j / 12.0
                p = point(i, u)
                jx = 0.05 * STRAND_R * math.sin(idx * 12.9898)
                jy = 0.05 * STRAND_R * math.sin(idx * 78.233)
                seed = common.add_uv_sphere(
                    "sesame", 0.030,
                    location=(p.x + jx, p.y + jy, p.z + 0.82 * STRAND_R),
                    scale=(1.0, 0.72, 0.5), mat=seed_mat,
                    segments=12, rings=8)
                seed.rotation_euler = (0.5 * math.sin(idx * 3.7),
                                       0.5 * math.sin(idx * 5.1),
                                       idx * 0.7)
                objs.append(seed)
                idx += 1
    return objs


def cover(name, center, angle, length, velvet, gold):
    nx, ny = 25, 19
    sx, sy = 2.25, 1.78
    hx = length * 0.5 + 0.32
    hy = 0.66
    peak = BOARD_TOP + 1.9 * STRAND_R + 0.17
    ca, sa = math.cos(angle), math.sin(angle)
    verts = []
    for iy in range(ny):
        v = iy / (ny - 1)
        y = -sy * 0.5 + sy * v
        for ix in range(nx):
            u = ix / (nx - 1)
            x = -sx * 0.5 + sx * u
            fx = foodkit.smoothstep((hx - abs(x)) / 0.35)
            fy = max(0.0, 1.0 - (abs(y) / hy) ** 2) ** 0.75
            zl = peak * fx * fy
            ripple = 0.012 * math.sin(7.0 * x + 1.7) * math.sin(9.0 * y - 0.6)
            z = BOARD_TOP + 0.012 + zl + ripple * (0.3 + 0.7 * fy)
            xr = x * ca - y * sa
            yr = x * sa + y * ca
            verts.append((center[0] + xr, center[1] + yr, z))
    faces = []
    for iy in range(ny - 1):
        for ix in range(nx - 1):
            a = iy * nx + ix
            faces.append((a, a + 1, a + 1 + nx, a + nx))
    mesh = bpy.data.meshes.new(name + "_mesh")
    mesh.from_pydata(verts, [], faces)
    mesh.update()
    cloth = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(cloth)
    cloth.data.materials.append(velvet)
    for poly in mesh.polygons:
        poly.use_smooth = True
    sub = cloth.modifiers.new("subsurf", "SUBSURF")
    sub.levels = 1
    sub.render_levels = 1
    sol = cloth.modifiers.new("solidify", "SOLIDIFY")
    sol.thickness = 0.016

    # gold embroidered trim along the hem (grid boundary)
    border = []
    for ix in range(nx):
        border.append(ix)
    for iy in range(1, ny):
        border.append(iy * nx + (nx - 1))
    for ix in range(nx - 2, -1, -1):
        border.append((ny - 1) * nx + ix)
    for iy in range(ny - 2, 0, -1):
        border.append(iy * nx)
    pts = [Vector(verts[i]) + Vector((0, 0, 0.004)) for i in border]
    pts.append(pts[0])
    trim = common.tube_curve(name + "_trim", pts, 0.017, mat=gold)

    corners = [verts[0], verts[nx - 1], verts[(ny - 1) * nx],
               verts[(ny - 1) * nx + (nx - 1)]]
    tassels = []
    for c in corners:
        t = common.add_cone(name + "_tassel", 0.022, 0.007, 0.08,
                            location=(c[0], c[1], c[2] - 0.035), mat=gold,
                            vertices=16)
        tassels.append(t)
    return [cloth, trim] + tassels


def build():
    common.reset_scene()
    common.setup_render()
    walnut = common.mat_walnut()
    gold = common.mat_gold()
    bread = mat_glazed_bread()
    sesame = foodkit.prin("Sesame", (0.88, 0.80, 0.58, 1.0), rough=0.55)
    velvet = common.mat_velvet((0.05, 0.07, 0.32, 1.0))

    board = foodkit.rounded_box("board", (3.05, 2.15, BOARD_TOP),
                                (0.0, 0.0, BOARD_TOP * 0.5), walnut,
                                bevel_w=0.05, segments=4)

    objs = [board]
    objs += braid("challah_bare", (0.55, -0.35, BOARD_TOP), math.radians(-18),
                  1.65, 5.0, bread, seed_mat=sesame)
    covered_center = (-0.72, 0.42, BOARD_TOP)
    covered_angle = math.radians(10)
    objs += braid("challah_covered", covered_center, covered_angle,
                  1.45, 4.5, bread)
    objs += cover("cover", covered_center, covered_angle, 1.45, velvet, gold)

    common.frame_object(objs, margin=0.16, elevation=27.0, azimuth=35.0,
                        focal=70.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render("challah", common.OUT_DIR)


build()
