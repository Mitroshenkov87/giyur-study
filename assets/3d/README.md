# Procedural 3D art (Blender → WebP)

Everything here is generated from code: `scripts/*.py` model each item in
Blender 5.2 (headless, Cycles CPU), `render_all.sh` renders them and
`tools/to_webp.py` centres/crops and writes WebP at 1024, 512 and 256 px
with alpha. No PNG is kept. See `SPEC.md` for the look and the rules.

    SAMPLES=48 JOBS=2 ./render_all.sh                 # everything
    RES=768 SAMPLES=32 ./render_all.sh icon_quiz      # one script (all its variants)

| Folder | Files | Variants |
|---|---|---|
| `assets/img/objects/` | 20 ritual objects + kiddush cup = 21 | one look (works on both themes) |
| `assets/img/icons/` | 8 nav icons | `-wood`, `-leather`, each with `-hover` (turned 15°, brighter) |
| `assets/img/emblems/` | 8 section medallions | `-wood` (brass on walnut), `-leather` (bronze on oxblood leather) |
| `assets/img/decor/` | divider, corner, seal, bookmark | `-wood`, `-leather` |

## Proposed placement (not wired yet)
- **Sidebar nav** (Уроки, Календарь, Карточки, Викторина, Тесты, Успехи, Настройки, Источники):
  `icons/<name>-<theme>.webp` at 40–48 px left of the label; swap to `-hover` on
  mouse-over for a cheap "tilt + glint" effect (two pre-rendered frames, no 3D at runtime).
- **Section cards / headers**: `emblems/<section>-<theme>.webp` replaces the flat
  section tiles (osnovy, ivrit, shabbat, kashrut, prazdniki, semya, etika, beit_din).
  Large header art per section: osnovy → torah_rolled / torah_open, ivrit → siddur,
  shabbat → shabbat_candles / kiddush_cup / challah, kashrut → matzah or seder_plate,
  prazdniki → hanukkiah / menorah_temple / shofar / lulav_etrog, semya → tallit / mezuzah,
  etika → pirkei_avot, beit_din → magen_david / tefillin.
- **Quiz ladder** (Викторина): pomegranate (safe steps), shofar (final question /
  win), magen_david (record), objects as question illustrations by topic.
- **Empty states**: siddur ("no cards today"), pirkei_avot ("no results yet"),
  kotel ("nothing here yet"), spice_box (end of a session).
- **Calendar holidays**: Rosh Hashana → shofar + pomegranate; Sukkot → lulav_etrog;
  Hanukkah → hanukkiah; Pesach → seder_plate + matzah; Shabbat → shabbat_candles +
  challah; Havdalah → havdalah_candle + spice_box; Shavuot / Simchat Torah → torah_open
  / torah_rolled; Tisha b'Av and fasts → kotel; Tu bi-Shvat → pomegranate.
- **Decor**: `divider` between page sections, `corner` on featured cards,
  `seal` on passed tests / completed sections, `bookmark` on the current lesson.
