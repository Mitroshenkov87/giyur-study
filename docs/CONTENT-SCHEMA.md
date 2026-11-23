# Content schema (Шорашим)

All content lives in `content/<lang>/` as JSON (UTF-8, no comments); Russian is `content/ru/`, a later English version adds `content/en/` with the same file names. Nothing is built into the program: the app reads `content/<lang>/manifest.json` and every file it lists at start-up, from the `content\` folder beside `shorashim.exe` (src/content_store.rs). Code never hardcodes lesson text. Editing and shipping content without rebuilding: docs/CONTENT-UPDATES.md.

`manifest.json` also carries `contentVersion` (bump it with every content change, e.g. `2026-11-24` or `2026-11-24.2`), `minAppVersion` (the oldest program that can read these files; raise it when content needs a new app feature) and `contentFormat` (1). Lesson, card and question ids are what progress is stored under: never reuse or renumber them; a removed id is simply ignored by old progress.

## Levels (movements) — nested strictness
`"reform"` (1) ⊂ `"conservative"` (2) ⊂ `"orthodox"` (3).
Every lesson, block, card and question has `minLevel`. A user at level L sees items with minLevel ≤ L.
- reform = база (общее для всех течений).
- conservative = добавляется то, что требуют Масорти (и ортодоксы).
- orthodox = только ортодоксальная галахическая детализация.
Where Reform (or Conservative) differs IN KIND (not just "less"), add `"differs": [{"level":"reform","text":"..."}]` — shown as a coloured note «В реформистском иудаизме иначе: …».

Equal depth (owner's rule, 2026-11-23): Reform Judaism must be written as fully and respectfully as Orthodox. Do not treat `minLevel: reform` as a residual “common core” and leave Reform practice to short footnotes. Where Reform has its own process, theology or custom (гиюр, бейт-дин, Shabbat, kashrut, prayer, siddurim such as Mishkan T'filah, interview questions), give it its own substantial blocks, lessons, sources and questions. Neutral shared text stays neutral; Orthodox-only detail stays behind `minLevel: orthodox`. Never describe Reform as “softer Orthodox” or “optional leftovers”.

## Section file `content/ru/sections/NN-id.json`
```json
{
  "id": "shabbat",
  "order": 3,
  "title": "Шаббат",
  "subtitle": "Святость седьмого дня",
  "icon": "🕯️",
  "intro": "1–2 предложения о разделе",
  "overview": {                        // «О чём раздел» in the «Читать» mode (docs/STRUCTURE.md)
    "why": "2–3 предложения: зачем раздел для понимания иудаизма",
    "examFocus": "2–3 предложения: что по теме проверяет бейт-дин",
    "goals": ["После раздела вы сможете …"]   // 3–5
  },
  "sources": [ {"id":"ksa","title":"Кицур Шульхан Арух, пер. А. Кутукова (КЕРООР, 1999)","url":"https://...","movement":"orthodox"} ],
  "deeper": [                          // «Углубиться», 3–6 items
    {"title":"Кицур Шульхан Арух о Шаббате","sourceRef":"ksa","text":"что там почитать и зачем","minLevel":"orthodox"}
  ],
  "lessons": [ LESSON, ... ],          // 4–8 lessons
  "questions": [ QUESTION, ... ]       // section exam bank, 40–60 questions
}
```

## LESSON
```json
{
  "id": "shabbat-1",                 // "<sectionId>-<n>"
  "title": "Что такое Шаббат",
  "minLevel": "reform",
  "minutes": 7,                      // 5–10 min reading = ~700–1300 Russian words
  "sourceRefs": ["ksa","tanakh"],    // ids from section.sources (at least one)
  "citations": ["Шмот 20:8–11", "Кицур Шульхан Арух 72:1"],   // precise references
  "keyIdeas": ["3–5 главных мыслей урока (≤ 160 символов), только пересказ текста"],
  "blocks": [ BLOCK, ... ],
  "cards": [ CARD, ... ]             // 10–20 per lesson
}
```

## BLOCK (lesson body, rendered in order)
- `{"type":"h","text":"Подзаголовок"}`
- `{"type":"p","text":"Абзац. Допускается **жирный** и *курсив*."}`
- `{"type":"list","items":["...","..."]}` (optional `"ordered": true`)
- `{"type":"hebrew","he":"שְׁמַע יִשְׂרָאֵל","translit":"Шма Исраэль","ru":"Слушай, Израиль","ref":"Дварим 6:4"}` — Hebrew with nikkud, Russian transliteration, translation
- `{"type":"quote","text":"...","ref":"Пиркей Авот 1:2"}`
- `{"type":"note","text":"..."}` — practical tip
- `{"type":"term","term":"Мицва","he":"מִצְוָה","text":"заповедь"}` — glossary term
Any block may have `"minLevel"` (default = lesson minLevel) and `"differs"`.

## CARD
```json
{"id":"shabbat-1-c1","front":"Когда начинается Шаббат?","back":"В пятницу с заходом солнца (свечи зажигают за ~18 минут до заката)","minLevel":"reform","differs":[]}
```
Short: front ≤ 120 chars, back ≤ 220 chars.

## QUESTION (exam bank)
Common fields: `difficulty` ("easy"|"medium"|"hard", used by the timed exam ramp), `id` ("shabbat-q1"), `lessonId`, `minLevel`, `format`, `prompt`, `explanation` (1–2 sentences), `source` (precise reference string), optional `differs`.
- `"format":"mc"` — `"options":["A","B","C","D"]` (exactly 4), `"answer": 0..3`
- `"format":"tf"` — `"answer": true|false`
- `"format":"match"` — `"pairs":[["Левая","Правая"], ...]` (3–5 pairs)
- `"format":"typed"` — `"answer":"Кидуш"`, `"accept":["кидуш","киддуш"]` (lower-case variants; app compares case-insensitively, ignores ё/е and punctuation)
- `"format":"open"` — beit-din style; `"modelAnswer":"..."` (3–6 sentences), `"keyPoints":["...","..."]` for self-check. Open questions with a `lessonId` form the lesson's «Что спрашивают на бейт-дине» box and the section's «Репетиция бейт-дина»: at least 2 per lesson (3 in the «Бейт-дин» section). Closed formats (mc, tf, match, typed) of a lesson form its «Самопроверка».
Per section mix target: ~45% mc, 20% tf, 10% match, 10% typed, 15% open. At least 40 questions per section.

## manifest.json
```json
{"version":"1.0.0","contentVersion":"2026-11-15","reviewStatus":"требует проверки раввином",
 "sections":[{"id":"osnovy","file":"sections/01-osnovy.json"}, ...]}
```

## Language and accuracy rules
- Natural literate Russian (not calque). Use common Russian-Jewish spelling: Тора, Танах, Шаббат, кашрут, бейт-дин, миква, Рош а-Шана, Йом Кипур, Песах, Шавуот, Суккот, Ханука, Пурим, брит-мила, бар-мицва, хупа, кидушин, тфилин, талит, мезуза, сидур, Амида, Шма, кидуш, авдала, галаха, мицва (мн. мицвот).
- Tanakh book names in Hebrew form with Russian in brackets on first use: Берешит (Бытие), Шмот (Исход), Ваикра (Левит), Бемидбар (Числа), Дварим (Второзаконие).
- Facts must match mainstream sources. If unsure, keep it general and add the point to `docs/RABBI-REVIEW.md`-style list in your report; never invent quotes or references.
- God's name: write «Всевышний», «Б-г» or «Бог»; in Hebrew blocks use the standard printed forms (ה׳ / יְהֹוָה as in siddurim) — prefer ה׳ or אֲדֹנָי.
