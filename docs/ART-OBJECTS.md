# Procedural 3D art (Blender → WebP)

All art here is generated from code: Blender 5.2 LTS scripts in
`assets/3d/scripts/`, rendered headless with Cycles and converted to WebP with
alpha (1024 / 512 / 256 px). No PNGs are kept.

```
cd assets/3d
SAMPLES=48 JOBS=2 ./render_all.sh                 # everything
RES=768 SAMPLES=32 ./render_all.sh icon_quiz      # one script (all theme/hover variants)
python3 tools/contact_sheet.py ../img /tmp/sheet.webp
```

| Folder | Files | Variants |
|---|---|---|
| `assets/img/objects/` | 20 ritual objects | one look (works on both themes) |
| `assets/img/icons/` | 8 sidebar icons | `-wood`, `-leather`, each `-hover` |
| `assets/img/emblems/` | 8 section medallions | `-wood`, `-leather` |
| `assets/img/decor/` | divider, corner, seal, bookmark | `-wood`, `-leather` |

Hover = the same icon turned 15° with a brighter key light. Swapping rest → hover
on mouse-over gives a cheap "tilt and catch the light" effect without real 3D.

## Proposed placement (not wired yet)

| Asset | Where |
|---|---|
| icons/lessons, calendar, cards, quiz, tests, progress, settings, sources | sidebar items Уроки, Календарь, Карточки, Викторина, Тесты, Успехи, Настройки, Источники (48 px, hover frame on mouse-over) |
| emblems/osnovy … beit_din | section cards on «Уроки» (64–80 px) and section page headers (128–160 px) |
| objects/torah_open | Основы header; empty state «Нет уроков» |
| objects/siddur | Иврит и молитва header |
| objects/shabbat_candles, kiddush_cup, challah | Шаббат header; calendar Friday/Shabbat cells; candle-lighting time card |
| objects/havdalah_candle, spice_box | calendar Motzei Shabbat / havdalah time |
| objects/seder_plate, matzah | calendar Pesach; Кашрут header (matzah) |
| objects/hanukkiah | calendar Hanukkah (8 days) |
| objects/shofar, pomegranate | calendar Rosh Hashana / Yom Kippur; Праздники header (shofar) |
| objects/lulav_etrog | calendar Sukkot |
| objects/menorah_temple, kotel | calendar Tisha b'Av, 17 Tammuz; quiz ladder top prize step (menorah) |
| objects/tefillin, tallit, mezuzah | Жизненный цикл и семья (bar mitzvah, home); flashcard deck covers |
| objects/pirkei_avot | Этика header; quote-of-the-day panel |
| objects/magen_david | app splash / «Мои успехи» big badge |
| objects/torah_rolled | Бейт-дин header; quiz ladder milestone steps |
| decor/divider | between page sections, under page titles |
| decor/corner | corners of the quote panel and featured cards |
| decor/seal | «пройдено» stamp on finished lessons / passed tests |
| decor/bookmark | «продолжить» marker on the last opened lesson |

## Tool split
- **Kimi** (text and code only) wrote the pipeline (`common.py`, converter,
  runner) and the first version of every script.
- **Cursor agent CLI** reviewed renders visually and fixed the icons, the
  emblems/decor, and the b/d object batch (shofar, tallit, kotel, havdalah,
  tefillin, spice box, hanukkiah).
- **Claude** reviewed and reworked the a/c object batch (Torah open/rolled,
  siddur, Pirkei Avot, mezuzah, challah, matzah, seder plate, pomegranate,
  lulav & etrog).
