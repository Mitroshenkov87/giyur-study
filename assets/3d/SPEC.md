# Shorashim object art — procedural 3D spec

Shorashim («Шорашим») is a study app on the basics of Judaism and giyur prep.
Visual world: a dark, expensive library — walnut wood, dark leather, brass/gold
details, warm lamp light; plus a light theme of matte leather / kosher hide.
We need a coherent SET of Jewish ritual objects, modelled procedurally in
Blender Python, rendered to flat images with transparent background.

Hard rules
- Jewish ritual items, accurate in form. No human figures. No Christian symbols
  (no crosses, no halos, nothing church-like). Hebrew letters only where noted.
- Blender 5.2 LTS, run headless: `blender -b --factory-startup -P <script> -- --out <dir>`
  Binary: /home/box/.local/bin/blender
- Each object = one script `assets/3d/scripts/obj_<name>.py` that does
  `import common` (sys.path insert of its own dir) and calls common helpers.
- `common.py` holds: scene reset, render settings, camera, lighting rig,
  material library, and `render(name, out_dir)`.
- Output: render to `<out>/<name>.png` (temporary, RGBA, 1024x1024,
  film_transparent=True). A separate Python step (Pillow) converts to
  `<name>.webp` (lossless or quality 92 with alpha) + `<name>-512.webp` +
  `<name>-256.webp` and deletes the PNG. Never leave PNGs as deliverables.

Look (same for every object, so the set is coherent)
- Camera: 3/4 view from slightly above (~20-30 deg elevation), focal ~60-85mm,
  object auto-framed so its bounding box fills ~80% of the frame, centered.
  Provide `common.frame_object(objs, margin=0.1)` that computes this.
- Lighting: warm key area light (~3200K, colour ~ (1.0, 0.78, 0.55)) from
  upper-left front, soft fill from right (dimmer, slightly cooler), a rim light
  from behind for silhouette edge against dark backgrounds. No world
  background visible (film transparent); world colour very dark warm gray at
  low strength for subtle reflections. Shadow catcher plane under the object
  (is_shadow_catcher=True) so a soft contact shadow is kept in alpha.
- Render: Cycles CPU, 96-128 samples, denoise (OpenImageDenoise), Filmic/AgX
  view transform "AgX" with look "Medium High Contrast" or similar, 1024x1024.
- Materials (Principled BSDF, procedural textures only, no image files):
  brass (warm gold, metallic 1, roughness 0.25-0.35, slight noise bump),
  gold, silver (roughness 0.2), dark walnut wood (wave/noise grain),
  light wood, parchment/klaf (cream-beige, roughness 0.7, subtle noise
  colour variation, faint fibres), wax (ivory, subsurface), leather (dark
  brown or black, roughness 0.5, fine noise bump), cloth/wool (white with
  blue or black stripes for tallit; sheen), velvet (deep burgundy/navy),
  bread crust (challah, golden-brown), matzah (pale toasted with darker
  spots), horn (shofar: cream to amber gradient, ridges), stone (Jerusalem
  limestone, warm cream, noise bump), glass, fruit skins (pomegranate red,
  etrog yellow-green bumpy), palm leaf green, flame (emission, warm).
- Ink/text: where Hebrew script is needed (Torah columns, siddur pages, Pirkei
  Avot), do NOT try to render real letters; use procedural dark rows of short
  dashes/blocks that read as columns of text from a distance. Exception: the
  mezuzah case may show the letter ש (shin) as a simple geometric emblem;
  may build it from curves/text object with font
  /workspace/shorashim-wt-art/assets/fonts/NotoSerifHebrew-Regular.ttf.

Objects (name -> description)
- torah_open: Sefer Torah open: two wooden rollers (atzei chaim) with turned
  handles and round discs, parchment scroll spread between them showing 3-4
  columns of text rows. Lying at a slight angle, as on a bimah.
- torah_rolled: Sefer Torah closed, standing upright: both rollers together,
  covered by a velvet mantle (deep burgundy) with gold embroidery band (no
  crosses; a simple crown or Star-of-David-free band is fine), brass crown or
  two rimonim finials on top.
- menorah_temple: seven-branched Temple menorah, gold, branches as smooth
  semicircular arcs rising from the central stem (three on each side), cups at
  the tops all at the same height, decorative knops along the stem, tripod or
  solid base. Small oil-lamp flames optional (emission).
- hanukkiah: Hanukkah menorah with 9 lights: 8 in a straight row at equal
  height + the shamash set apart (higher or in front). Brass. Small candles or
  oil cups, warm flames.
- kiddush_cup: silver goblet with stem and foot, engraved band (procedural
  ring pattern), filled with red wine (dark red glass-like liquid surface).
- shabbat_candles: two white candles in two silver/brass candlesticks, lit.
- challah: two braided challot (at least one visible braid, 4-6 strands),
  golden crust with sesame dots optional, partly under an embroidered cloth
  cover (deep blue or white with gold border) draped over one; on a wooden board.
- shofar: ram's horn, curved spiral-ish horn, wide flared bell tapering to a
  mouthpiece, cream-to-amber gradient with growth ridges. No metal decorations.
- mezuzah: slim rectangular case (brass or olive wood), slightly rounded,
  with letter ש on the front, small screw holes top and bottom.
- tefillin: black leather cube box (bayit) on a wider square base (titura),
  with black leather straps (retzuot) coiled beside it. Add a second box
  (shel rosh) if possible. Cubes must be sharp-edged and black matte.
- tallit: folded white wool tallit with black (or blue) stripes near the ends,
  an atarah collar band (silver/gold), and tzitzit (white knotted fringes,
  4 strings doubled, with visible windings) hanging from the corners.
- siddur: closed prayer book, dark leather cover with gold-embossed border,
  gilt page edges, a ribbon bookmark.
- magen_david: Star of David emblem — two interlocked equilateral triangles,
  hexagram, gold/brass, bevelled, slightly tilted toward the camera.
- pomegranate: ripe pomegranate with crown (calyx), plus one half opened
  showing seeds (red translucent arils).
- spice_box: havdalah besamim box: silver tower-shaped box with filigree-ish
  windows, small flag or pointed roof finial (NO cross on top — a ball or
  small flag only).
- havdalah_candle: braided candle of 2-4 twisted strands (blue and white
  wax), multiple wicks lit with one larger flame, in a small holder.
- kotel: section of the Western Wall — large Herodian ashlar stones with
  recessed margins (drafted edges), a few small paper notes tucked in cracks,
  a tuft of plant growing from a joint. As a small block/vignette, not a
  whole scene.
- matzah: a stack of 3 round hand-made (shmurah) matzot or square ones,
  perforation rows, toasted brown spots.
- seder_plate: round ceramic/silver plate with 6 places (zeroa shankbone,
  beitzah roasted egg, maror, charoset, karpas, chazeret) as simple shapes,
  plus three matzot under a cloth optional.
- lulav_etrog: lulav (palm frond spine) bound with hadassim (myrtle) and
  aravot (willow), plus etrog (yellow, bumpy, lemon-like with pitam) next to
  it, maybe in an open silver etrog box.
- pirkei_avot: open book (thick, two curved page spreads) with text rows,
  dark leather cover, resting on a small wooden reading stand (shtender style)
  or flat.

Quality bar: these go into a premium-looking app as section header art and
quiz illustrations. Silhouettes must read clearly at 256 px. Prefer bevels,
subdivision, smooth shading; avoid default gray materials and z-fighting.

## Status / conventions (added after pilot)
- `common.py` is FROZEN for parallel work: do NOT edit it. If you need extra
  helpers, put them in your own script or in a new helper module named for your
  batch (e.g. `iconkit.py`). Read common.py first and reuse its helpers:
  reset_scene, setup_render, frame_object(objs, margin, focal, elevation,
  azimuth), setup_lights, add_shadow_catcher, render(name, out_dir),
  add_cylinder/add_uv_sphere/add_cone/add_torus/tube_curve/lathe, bevel, smooth,
  assign, and the mat_* library, plus THEME, HOVER, variant_name(), theme_metal(),
  theme_accent().
- Reference pattern: obj_menorah_temple.py and obj_kiddush_cup.py.
  Call order: reset_scene(); setup_render(); build geometry; frame_object(objs,...);
  setup_lights(); add_shadow_catcher(); render(name, common.OUT_DIR).
- Quick test (fast, low quality):
  `blender -b --factory-startup -P scripts/<file>.py -- --out /tmp/kt --samples 8 --res 384`
  Then check with Pillow that the PNG's alpha bbox is non-empty and the object
  isn't clipped (bbox should not touch the image border). Never open or look at
  the image itself; numbers only. Delete test PNGs afterwards.
- Final renders are done by the maintainer with render_all.sh; you only write
  and smoke-test scripts. Max 3 test renders per script; stop when it runs
  cleanly and the geometry is sound.
- Output naming: obj_<name>.py renders "<name>". icon_/emblem_/decor_ scripts
  must render common.variant_name("<name>") so theme/hover variants get
  separate files.

## Icon set (icon_*.py) — sidebar navigation, both themes, rest + hover
Small, bold, chunky 3D icons that read at 48-64 px. One object per icon, simple
silhouette, few parts, thick features, no tiny details. Main material:
common.theme_metal() (brass on wood theme, dark antique bronze on leather
theme) with common.theme_accent() for secondary parts (walnut / oxblood
leather); parchment or wax where natural. Camera: 3/4 view, elevation ~25,
azimuth ~30 (HOVER adds +15 automatically and brightens the key light).
No shadow catcher needed for icons (skip add_shadow_catcher) so they sit
cleanly in a nav row.
- icon_lessons (Уроки): an open book, thick pages fanned, brass corner caps.
- icon_calendar (Календарь): a small standing desk calendar/luach page block
  in a brass frame with a crescent moon emblem on top (Jewish lunar calendar).
- icon_cards (Карточки): a fanned stack of 3 thick index cards, top one with a
  brass corner and a raised א.
- icon_quiz (Викторина): a short brass ladder of 5 rungs climbing to a small
  crown-like finial (the quiz is a ladder of rising questions). No money, no TV.
- icon_tests (Проверка знаний): a rolled parchment sheet with a brass
  check-mark seal / quill pen and inkwell.
- icon_progress (Мои успехи): a brass goblet-trophy or a pomegranate on a
  small plinth (pomegranate = 613 mitzvot, fullness). Pick pomegranate on plinth.
- icon_settings (Настройки): a brass gear/cog with a walnut/leather hub.
- icon_sources (Источники): a stack of 3 thick leather-bound books (seforim)
  with brass spine bands, lying flat.

## Section emblems (emblem_*.py) — round medallions, both themes
Each = a round coin-like medallion (diameter 2, thickness ~0.18, raised bezel
rim, bevelled) made of theme_metal(), with a theme_accent() inner disc inset,
and a raised relief symbol (theme_metal, extruded ~0.08) on the inner disc,
viewed nearly frontal (elevation ~10, azimuth ~15) so the symbol reads.
- emblem_osnovy: rolled Torah scroll (two rollers with parchment between).
- emblem_ivrit: the letter א (use text object with font
  /workspace/shorashim-wt-art/assets/fonts/NotoSerifHebrew-Regular.ttf, extrude).
- emblem_shabbat: two candles with flames (flames emissive).
- emblem_kashrut: a fish with clearly visible fins and scales (kosher sign).
- emblem_prazdniki: a shofar.
- emblem_semya: a chuppah (canopy cloth on four poles), no people.
- emblem_etika: balance scales.
- emblem_beit_din: three high-backed chairs side by side (three judges), no people.

## Decor (decor_*.py) — ornaments for screens, both themes
- decor_divider: a long thin horizontal brass rule with a pomegranate finial in
  the centre and small knobs at the ends (aspect ~8:1; still render square, the
  converter centres it).
- decor_corner: an L-shaped brass corner bracket with a small engraved scroll
  curl, for card corners.
- decor_seal: a round wax seal (deep red wax, irregular rim) stamped with a
  Star of David, for "passed" / completion states.
- decor_bookmark: a silk ribbon bookmark (burgundy) with a brass tip, hanging.
