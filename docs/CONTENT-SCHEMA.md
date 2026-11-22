# Content schema (Giyur Study)

All content lives in `content/` as JSON (UTF-8, no comments). The app loads `content/manifest.json`, then each section file. Code never hardcodes lesson text.

## Levels (movements) — nested strictness
`"reform"` (1) ⊂ `"conservative"` (2) ⊂ `"orthodox"` (3).
Every lesson, block, card and question has `minLevel`. A user at level L sees items with minLevel ≤ L.
- reform = база (общее для всех течений).
- conservative = добавляется то, что требуют Масорти (и ортодоксы).
- orthodox = только ортодоксальная галахическая детализация.
Where Reform (or Conservative) differs IN KIND (not just "less"), add `"differs": [{"level":"reform","text":"..."}]` — shown as a coloured note «В реформистском иудаизме иначе: …».

## Section file `content/sections/NN-id.json`
```json
{
  "id": "shabbat",
  "order": 3,
  "title": "Шаббат",
  "subtitle": "Святость седьмого дня",
  "icon": "🕯️",
  "intro": "1–2 предложения о разделе",
  "sources": [ {"id":"ksa","title":"Кицур Шульхан Арух, пер. А. Кутукова (КЕРООР, 1999)","url":"https://...","movement":"orthodox"} ],
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
- `"format":"open"` — beit-din style; `"modelAnswer":"..."` (3–6 sentences), `"keyPoints":["...","..."]` for self-check
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
