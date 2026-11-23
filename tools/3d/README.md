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
on the back) so the post is a solid block with one slanted case.

`siddur` is the first of three Tripo attempts (v1 `83353323`): the front, the
start view and the three-quarter view show an open prayer book with a Magen
David and a blue bookmark; the sides and the underside of the covers are
flatter, so the start view uses pitch 0.40 to favour the open pages. Attempt A
was a flatter book with geometric marks on the pages; attempt B had a cover
standing like a wall, a ragged flap and a floating ribbon. Letters on the
pages are decorative; zoom is capped at 1.15× like the scrolls.

`tefillin` was rebuilt in Blender from the cleaned v3 bayit/titura as a
proportion reference. It is now a modelled shel-rosh: a square bayit on a
wider titura, a maabarta slot at the back, a raised three-headed shin
(Frank Ruhl Libre) on the wearer's right and a four-headed shin on the
left (a fourth prong added to the same letter), and glossy black leather
straps looping out of the maabarta with hanging tails. The knot is
stylized, not a precise dalet-kesher. Single leather material, roughness
about 0.25, 2K WebP maps, well under 199,999 triangles. The still uses
pitch 0.32 so the titura and strap loop are visible (the app start view
matches). v1–v3 of the scanned placeholder were replaced.

`kotel` and `chuppah` were added the same way (`199999 2048 2048`). The Kotel
is a short fragment of ashlar with plants in the joints and a paved stand;
the side already reads as solid stone blocks about one course deep, so the
slab was not thickened. The back is also stone with plants; there are no
kvitlach in the cracks. The chuppah is a tallit-like cloth on four wooden
poles with a small table and a cup; the cloth is thin and the fringes are
uneven, and the back hem is a little asymmetric. The chuppah still uses
pitch 0.40 so the table under the cloth is visible (the app start view
matches).


`ketubah` and `kosher-dishes` were added the same way (`199999 2048 2048`).
The ketubah is a framed decorative contract on a small easel; the letters
are not a real ketubah text, so zoom is capped at 1.15× like the scrolls.
The kosher dishes are a teaching set on one board: a red plate, bowl and
fork for meat (басар) and a blue plate, bowl and spoon for dairy (халав).
The still of the dishes uses pitch 0.55 so the bowls and utensils are
visible (the app start view matches). The colours are a conventional aid,
not a halakhic rule; the caption says the sets would be stored separately
in a home kitchen.

`wedding-glass` was added the same way (`199999 2048 2048`). The glass
arrived from Tripo as opaque matte white, with a blue ring on the tray
under the foot and a tan smudge on the stem. The app shader has no alpha
and uses only the first material, so the glass was retinted in Blender
to a glossy pale grey-blue (low roughness, a slight metallic boost) and
the ring and smudge were cleaned on the 2K maps. It is a convincing tint,
not real transparency. The still uses pitch 0.35 so the tray and napkin
are visible (the app start view matches). The object belongs in «Жизненный
цикл и семья» (breaking the glass in memory of the Temple) and in
«Предмет дня».

`kosher-fish` was added the same way (`199999 2048 2048`). It is a carp
on a rustic two-plank board, with loose silvery scales beside the belly:
the teaching point is that kosher fish have fins and scales that come off
without damaging the skin (Ваикра 11:9; Mishna Nida 6:9; Shulchan Aruch
Yoreh Deah 83). A first Blender pass that flattened a suspected “icy
clump” along the lower edge also flattened part of the belly and tail, so
that export was discarded; the shipped file is a plain optimize of the
raw Tripo mesh. Some loose scales along the lower edge are slightly
thick or fused, but they remain recognizable as scales that came off.
The still uses pitch 0.55 (the app start view matches). The object
belongs in «Кашрут» (signs of kosher fish) and in «Предмет дня».


Runtime: `src/ui/model3d` (GLB reader, wgpu shader widget). Debug hooks: `SHORASHIM_3D_STATS=1` (fps and 3D-pass
time to stderr), `SHORASHIM_3D_YAW=<rad>` (fixed angle, no idle spin), `SHORASHIM_3D_MSAA=1` (no MSAA),
`SHORASHIM_3D_SIZE=<px>` (fixed model size on all cards; by default 190–280 px, scaled with the reading column).
