"""Two plump braided challot on a walnut board: glossy egg-washed crust with
sesame, one partly under a deep-blue velvet cover with a gold embroidered
border and rosette emblem (no text)."""

import math
import os
import random
import sys

import bmesh
import bpy
from mathutils import Matrix, Vector, noise

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import common
import foodkit

BOARD_TOP = 0.12


def mat_glazed_bread():
    """Egg-washed crust: deep golden brown tops, paler dough in the crevices."""
    m, nt, bs = common._new_material("BreadGlazed")
    L = nt.links
    tc = common._texcoord(nt)
    n = common._noise(nt, scale=5.0, detail=5.0, roughness=0.6)
    L.new(tc.outputs["Object"], n.inputs["Vector"])
    crust = common._ramp(nt, [(0.0, (0.20, 0.075, 0.015, 1.0)),
                              (0.40, (0.34, 0.15, 0.028, 1.0)),
                              (0.70, (0.48, 0.24, 0.05, 1.0)),
                              (1.0, (0.58, 0.33, 0.09, 1.0))])
    L.new(n.outputs["Fac"], crust.inputs["Fac"])
    ao = nt.nodes.new("ShaderNodeAmbientOcclusion")
    ao.inputs["Distance"].default_value = 0.10
    ao.samples = 8
    aor = nt.nodes.new("ShaderNodeMapRange")
    aor.inputs["From Min"].default_value = 0.45
    aor.inputs["From Max"].default_value = 0.85
    aor.inputs["To Min"].default_value = 0.8
    aor.inputs["To Max"].default_value = 0.0
    L.new(ao.outputs["AO"], aor.inputs["Value"])
    mix = nt.nodes.new("ShaderNodeMix")
    mix.data_type = "RGBA"
    L.new(aor.outputs["Result"], mix.inputs["Factor"])
    L.new(crust.outputs["Color"], mix.inputs["A"])
    mix.inputs["B"].default_value = (0.60, 0.40, 0.18, 1.0)
    L.new(mix.outputs["Result"], bs.inputs["Base Color"])
    fine = common._noise(nt, scale=60.0, detail=3.0, y=-320)
    L.new(tc.outputs["Object"], fine.inputs["Vector"])
    common._bump(nt, bs, fine.outputs["Fac"], strength=0.10, distance=0.01)
    common._inp(bs, "Roughness", 0.45)
    common._inp(bs, "Subsurface Weight", 0.08)
    common._inp(bs, "Coat Weight", 0.35)
    common._inp(bs, "Coat Roughness", 0.18)
    return m


def batch_ellipsoids(name, items, mat, seg=32, rings=18):
    """items: (center, radii(x,y,z), rotation Matrix 3x3). One mesh."""
    bm = bmesh.new()
    for c, r, rot in items:
        mtx = (Matrix.Translation(c) @ rot.to_4x4()
               @ Matrix.Diagonal((r[0], r[1], r[2], 1.0)))
        bmesh.ops.create_uvsphere(bm, u_segments=seg, v_segments=rings,
                                  radius=1.0, matrix=mtx)
    mesh = bpy.data.meshes.new(name + "_mesh")
    bm.to_mesh(mesh)
    bm.free()
    ob = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(ob)
    ob.data.materials.append(mat)
    common.smooth(ob)
    return ob


def loaf(name, center, angle, length, width, height, n, mat, seed=0):
    """Six-strand look: two rows of fat diagonal lobes (alternating chevrons)
    plus a ridge row of smaller knots on top, tapering to the ends."""
    rng = random.Random(seed)
    items = []
    rz = Matrix.Rotation(angle, 3, "Z")
    c0 = Vector(center)
    pitch = length / n

    def env(u):                       # 0..1 along loaf -> fullness
        return math.sin(math.pi * (0.06 + 0.88 * u)) ** 0.55

    # bottom core so there are no gaps under the lobes
    items.append((c0 + Vector((0, 0, height * 0.22)),
                  (length * 0.47, width * 0.36, height * 0.26), rz))
    for k in range(n):
        u = (k + 0.5) / n  # noqa
        e = env(u)
        for side, shift, tilt in ((1, 0.25, 1), (-1, -0.25, -1)):
            x = -length / 2 + (k + 0.5 + shift) * pitch
            uu = min(max((x + length / 2) / length, 0.02), 0.98)
            e2 = env(uu)
            y = side * width * 0.21 * e2
            z = height * 0.38 * e2
            rot = rz @ Matrix.Rotation(tilt * math.radians(55), 3, "Z") \
                     @ Matrix.Rotation(-side * math.radians(18), 3, "X")
            items.append((c0 + rz @ Vector((x, y, z)),
                          (pitch * 1.45 * (0.8 + 0.2 * e2), width * 0.20 * e2 + 0.02,
                           height * 0.40 * e2 + 0.02), rot))
    # top ridge: smaller knots alternating left/right lean
    for k in range(n - 1):
        u = (k + 1.0) / n
        e = env(u)
        x = -length / 2 + (k + 1.0) * pitch
        lean = 1 if k % 2 == 0 else -1
        rot = rz @ Matrix.Rotation(lean * math.radians(35), 3, "Z")
        items.append((c0 + rz @ Vector((x, 0.0, height * 0.70 * e + 0.02)),
                      (pitch * 0.80, width * 0.17 * e + 0.015, height * 0.27 * e + 0.015),
                      rot))
    ob = batch_ellipsoids(name, items, mat)
    # slight lumpy irregularity
    for v in ob.data.vertices:
        p = v.co * 3.0 + Vector((seed, 2 * seed, 0))
        v.co += Vector((0, 0, 1)) * 0.012 * noise.noise(p)
    return ob


def sesame(target, mat, count, seed, footprint):
    """Scatter seeds onto the loaf by ray casting from above."""
    rng = random.Random(seed)
    dg = bpy.context.evaluated_depsgraph_get()
    cx, cy, ang, hl, hw = footprint
    ca, sa = math.cos(ang), math.sin(ang)
    bm = bmesh.new()
    placed = 0
    for _ in range(count * 6):
        if placed >= count:
            break
        lx, ly = rng.uniform(-hl, hl), rng.uniform(-hw, hw)
        o = Vector((cx + lx * ca - ly * sa, cy + lx * sa + ly * ca, 3.0))
        hit, loc, nor, _i, ob, _m = bpy.context.scene.ray_cast(dg, o, Vector((0, 0, -1)))
        if not hit or ob is not target or nor.z < 0.45:
            continue
        q = nor.to_track_quat("Z", "X") @ \
            Matrix.Rotation(rng.uniform(0, 6.28), 3, "Z").to_quaternion()
        mtx = (Matrix.Translation(loc + nor * 0.006) @ q.to_matrix().to_4x4()
               @ Matrix.Diagonal((0.034, 0.020, 0.010, 1.0)))
        bmesh.ops.create_uvsphere(bm, u_segments=8, v_segments=5, radius=1.0,
                                  matrix=mtx)
        placed += 1
    mesh = bpy.data.meshes.new("sesame_mesh")
    bm.to_mesh(mesh)
    bm.free()
    ob = bpy.data.objects.new("sesame", mesh)
    bpy.context.collection.objects.link(ob)
    ob.data.materials.append(mat)
    common.smooth(ob)
    return ob


def mat_cover(sx, sy):
    """Velvet with gold embroidery driven by UV (in metres)."""
    m = bpy.data.materials.new("ChallahCover")
    m.use_nodes = True
    nt = m.node_tree
    nt.nodes.clear()
    L = nt.links
    out = nt.nodes.new("ShaderNodeOutputMaterial")
    vel = nt.nodes.new("ShaderNodeBsdfPrincipled")
    vel.inputs["Base Color"].default_value = (0.012, 0.020, 0.13, 1.0)
    vel.inputs["Roughness"].default_value = 0.8
    vel.inputs["Sheen Weight"].default_value = 1.0
    vel.inputs["Sheen Roughness"].default_value = 0.35
    vel.inputs["Sheen Tint"].default_value = (0.45, 0.55, 1.0, 1.0)
    gold = nt.nodes.new("ShaderNodeBsdfPrincipled")
    gold.inputs["Base Color"].default_value = (0.70, 0.46, 0.14, 1.0)
    gold.inputs["Metallic"].default_value = 0.85
    gold.inputs["Roughness"].default_value = 0.38
    uv = nt.nodes.new("ShaderNodeUVMap")
    sep = nt.nodes.new("ShaderNodeSeparateXYZ")
    L.new(uv.outputs["UV"], sep.inputs["Vector"])

    def math_node(op, a=None, b=None):
        n = nt.nodes.new("ShaderNodeMath")
        n.operation = op
        for i, v in enumerate((a, b)):
            if v is None:
                continue
            if isinstance(v, (int, float)):
                n.inputs[i].default_value = v
            else:
                L.new(v, n.inputs[i])
        return n.outputs[0]

    def band(val, lo, hi):            # 1 inside [lo, hi]
        a = math_node("GREATER_THAN", val, lo)
        b = math_node("LESS_THAN", val, hi)
        return math_node("MULTIPLY", a, b)

    x, y = sep.outputs["X"], sep.outputs["Y"]
    dx = math_node("MINIMUM", x, math_node("SUBTRACT", sx, x))
    dy = math_node("MINIMUM", y, math_node("SUBTRACT", sy, y))
    de = math_node("MINIMUM", dx, dy)
    mask = math_node("MAXIMUM", band(de, 0.07, 0.13), band(de, 0.16, 0.175))
    # rosette emblem at cloth centre: ring + six petals + centre dot
    cxn = math_node("SUBTRACT", x, sx * 0.5)
    cyn = math_node("SUBTRACT", y, sy * 0.5)
    rr = math_node("SQRT", math_node("ADD", math_node("MULTIPLY", cxn, cxn),
                                     math_node("MULTIPLY", cyn, cyn)))
    th = math_node("ARCTAN2", cyn, cxn)
    petal = math_node("MULTIPLY", math_node("ABSOLUTE", math_node("COSINE",
                      math_node("MULTIPLY", th, 3.0))), 0.15)
    petals = math_node("MULTIPLY", math_node("LESS_THAN", rr, petal),
                       math_node("GREATER_THAN", rr, 0.045))
    emb = math_node("MAXIMUM", band(rr, 0.20, 0.228), petals)
    emb = math_node("MAXIMUM", emb, math_node("LESS_THAN", rr, 0.03))
    mask = math_node("MAXIMUM", mask, emb)
    mix = nt.nodes.new("ShaderNodeMixShader")
    L.new(mask, mix.inputs["Fac"])
    L.new(vel.outputs["BSDF"], mix.inputs[1])
    L.new(gold.outputs["BSDF"], mix.inputs[2])
    L.new(mix.outputs["Shader"], out.inputs["Surface"])
    # raised embroidery + velvet fuzz
    thread = nt.nodes.new("ShaderNodeTexWave")
    thread.inputs["Scale"].default_value = 220.0
    L.new(uv.outputs["UV"], thread.inputs["Vector"])
    hgt = math_node("ADD", math_node("MULTIPLY", mask, 1.0),
                    math_node("MULTIPLY", thread.outputs["Fac"], 0.15))
    bump = nt.nodes.new("ShaderNodeBump")
    bump.inputs["Strength"].default_value = 0.6
    bump.inputs["Distance"].default_value = 0.006
    L.new(hgt, bump.inputs["Height"])
    L.new(bump.outputs["Normal"], vel.inputs["Normal"])
    L.new(bump.outputs["Normal"], gold.inputs["Normal"])
    return m


def cover(center, angle, sx, sy, under):
    """Cloth heightfield draped over `under` objects via ray casts."""
    nx, ny = 64, 52
    dg = bpy.context.evaluated_depsgraph_get()
    ca, sa = math.cos(angle), math.sin(angle)
    raw = []
    world = []
    for j in range(ny):
        for i in range(nx):
            lx = -sx / 2 + sx * i / (nx - 1)
            ly = -sy / 2 + sy * j / (ny - 1)
            wx = center[0] + lx * ca - ly * sa
            wy = center[1] + lx * sa + ly * ca
            o = Vector((wx, wy, 3.0))
            z = BOARD_TOP + 0.012
            hit, loc, _n, _i, ob, _m = bpy.context.scene.ray_cast(dg, o, Vector((0, 0, -1)))
            if hit and ob in under:
                z = max(z, loc.z + 0.05)
            raw.append(z)
            world.append((wx, wy, lx, ly))
    h = list(raw)
    for _ in range(260):
        nh = list(h)
        for j in range(ny):
            for i in range(nx):
                k = j * nx + i
                nb = [h[jj * nx + ii] for ii, jj in ((i - 1, j), (i + 1, j), (i, j - 1), (i, j + 1))
                      if 0 <= ii < nx and 0 <= jj < ny]
                avg = sum(nb) / len(nb)
                nh[k] = max(raw[k], 0.3 * h[k] + 0.7 * avg - 0.0005)
        h = nh
    verts, uvs = [], []
    for k, (wx, wy, lx, ly) in enumerate(world):
        z = h[k]
        lift = z - BOARD_TOP
        if lift > 0.04:
            z += 0.014 * math.sin(9.0 * lx + 3.0 * ly) * min(1.0, lift / 0.2)
        verts.append((wx, wy, z))
        uvs.append((lx + sx / 2, ly + sy / 2))
    faces = []
    for j in range(ny - 1):
        for i in range(nx - 1):
            a = j * nx + i
            faces.append((a, a + 1, a + 1 + nx, a + nx))
    mesh = bpy.data.meshes.new("cover_mesh")
    mesh.from_pydata(verts, [], faces)
    uvl = mesh.uv_layers.new(name="UVMap")
    for poly in mesh.polygons:
        for li in poly.loop_indices:
            vi = mesh.loops[li].vertex_index
            uvl.data[li].uv = uvs[vi]
    mesh.update()
    ob = bpy.data.objects.new("cover", mesh)
    bpy.context.collection.objects.link(ob)
    ob.data.materials.append(mat_cover(sx, sy))
    common.smooth(ob)
    sol = ob.modifiers.new("solidify", "SOLIDIFY")
    sol.thickness = 0.02
    sol.offset = 1.0
    sub = ob.modifiers.new("subsurf", "SUBSURF")
    sub.levels = 1
    sub.render_levels = 1
    return ob


def build():
    common.reset_scene()
    common.setup_render()
    walnut = common.mat_walnut()
    gold = common.mat_gold()
    bread = mat_glazed_bread()
    seed_mat = foodkit.prin("Sesame", (0.78, 0.70, 0.50, 1.0), rough=0.5)

    board = foodkit.rounded_box("board", (2.75, 2.15, BOARD_TOP),
                                (0.0, 0.0, BOARD_TOP * 0.5), walnut,
                                bevel_w=0.05, segments=4)
    objs = [board]

    a1 = math.radians(-12)
    l1 = loaf("challah_front", (0.12, -0.45, BOARD_TOP), a1, 2.15, 0.92, 0.56, 7,
              bread, seed=1)
    a2 = math.radians(-8)
    c2 = (-0.05, 0.55, BOARD_TOP)
    l2 = loaf("challah_back", c2, a2, 2.05, 0.88, 0.54, 7, bread, seed=2)
    objs += [l1, l2]
    bpy.context.view_layer.update()
    objs.append(sesame(l1, seed_mat, 170, 5, (0.12, -0.45, a1, 1.0, 0.45)))
    objs.append(sesame(l2, seed_mat, 120, 6, (-0.05, 0.55, a2, 1.0, 0.45)))
    bpy.context.view_layer.update()
    # cover draped over the left ~60% of the back loaf
    objs.append(cover((-0.55, 0.62, 0), math.radians(-6), 1.70, 1.55, [l2]))

    common.frame_object(objs, margin=0.04, elevation=32.0, azimuth=24.0,
                        focal=70.0)
    common.setup_lights()
    common.add_shadow_catcher()
    common.render("challah", common.OUT_DIR)


build()
