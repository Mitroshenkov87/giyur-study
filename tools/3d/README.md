# 3D models

Tripo AI exports (~65–75 MB GLB, 8K PBR textures) are shrunk for embedding with Blender headless:

    blender -b --factory-startup --python tools/3d/optimize.py -- in.glb assets/3d/out.glb 200000 2048 2048

Decimates to ~200k triangles and re-exports with WebP textures (`EXT_texture_webp`):
base colour, normal and metallic-roughness at 2048 px (the optional 5th argument sets the metallic-roughness size; default is half). That gives about 6.5–7.6 MB per model.

The static fallback picture (tiny-skia renderer, or «Объёмные модели» off) is rendered with Cycles from the same
camera the app uses:

    blender -b --factory-startup --python tools/3d/still.py -- assets/3d/out.glb assets/3d/out.webp 512 0.7

still.py takes an optional 5th argument: camera pitch in radians (default 0.28; the Second Temple
uses 0.62 for a 3/4 aerial start view):

    blender -b --factory-startup --python tools/3d/still.py -- assets/3d/temple-complex.glb assets/3d/temple-complex.webp 512 0.7 0.62

The Second Temple complex (assets/3d/temple-complex.glb, ~293k triangles, 2048 px maps, ~12 MB) was
optimized with optimize.py at 300000 2048 2048, then the ragged base was trimmed with two Blender
bisect planes (x <= 0.40 and y >= -0.45 in Blender Z-up coordinates) so the base ends evenly beside
the walls (`tools/3d/trim_base.py -- in.glb out.glb`).

The altar (mizbeach) in the Court of the Priests was then rebuilt larger so it reads at card size
(`tools/3d/temple_altar.py -- in.glb out.glb`): the small Tripo altar is removed, the floor under it is
patched with a planar-UV quad, and a stylized stone altar (base, top step, four corner horns, ramp on
the south side) is added with UVs taken from the courtyard wall texture. Result: ~292k triangles,
same 2048 px maps. In the app the Temple is dimmed slightly (exposure −28%) on the dark themes only,
via a per-model uniform (`Model::dark_dim`); other models are unchanged.

Five more object models were added the same way (`200000 2048 2048`): `magen-david`,
`hanukkiah` (nine cups: eight lights plus the shamash), `seder-plate`, `lulav-etrog`
and `megillat-esther`. The seder plate is a single fused mesh (~2M triangles before
decimation, 560 loose islands). A napkin under the matzot is part of the mesh and
is not a separate object, so it was left in; the caption does not describe the
arrangement as exact. The still of the plate uses pitch 0.55 so the wells and the
foods are visible (the app start view matches).

Three more models were added the same way (`200000 2048 2048`): `sukkah`,
`tallit` and `mezuzah`. The sukkah still uses pitch 0.48 so the schach and the
table inside are visible (the app start view matches). The mezuzah raw export
had a second case on the back of the doorpost and a third on the right side;
those were removed (silver components not on the front face, then a wood cap
on the back) so the post is a solid block with one slanted case. Tefillin was
not placed: v2 is crisp from the front but the sides show stacked extra slabs
and a very long strap that could not be cleaned reliably, and v1 (a pair on a
bag) is recognisable but too melted.

Runtime: `src/ui/model3d` (GLB reader, wgpu shader widget). Debug hooks: `SHORASHIM_3D_STATS=1` (fps and 3D-pass
time to stderr), `SHORASHIM_3D_YAW=<rad>` (fixed angle, no idle spin), `SHORASHIM_3D_MSAA=1` (no MSAA),
`SHORASHIM_3D_SIZE=<px>` (fixed model size on all cards; by default 190–280 px, scaled with the reading column).
