# PARITY.md — PWA → iced 0.14 feature inventory

> Written against the earlier PWA, then called Giyur Study. The app is now «Шорашим» (Shorashim); product-name strings below are given with the new name. The progress-file id `giyur-study` is kept for import compatibility.

Source of truth: the earlier PWA (`/workspace/giyur-study`) (index.html, css/app.css, js/*.js, js/ui/*.js, docs/APP.md), app version `1.2.0`.
All strings below are quoted exactly from the source. `{x}` = interpolated value; `plural(n, a|b|c)` = Russian plural (`U.plural`: 11–14 → c; ends 1 → a; ends 2–4 → b; else c).
Markup in content strings: `**bold**`, `*italic*`; Hebrew runs (U+0590–05FF, FB1D–FB4F) are auto-wrapped as inline RTL Hebrew (`he-inline`).

> Note: the PWA has **no** screen called «Кто хочет стать миллионером» and **no lifelines** (no 50/50, hint, etc.). The quiz-show timed mode is titled «Экзамен» / «Экзамен (на время)»; only the badge id `millionaire` (title «20 из 20») refers to it. The only "help" controls are «Не знаю — дальше» and «Пауза» of auto-advance.

---

## 0. Shell, navigation, global behaviour

### 0.1 Routes (hash router, `js/app.js`)
| Route | View | Nav group |
|---|---|---|
| `welcome` | Welcome | — |
| `level` | Movement choice | settings |
| `map` (default, empty hash) | Map/home | map |
| `section/{id}` | Section page | map |
| `lesson/{id}` | Lesson reader | map |
| `cards/lesson/{lessonId}`, `cards/section/{secId}` | Flashcard deck | map |
| `review` | Daily review deck | review |
| `exam`, `exam/s/{secId}` (preset section) | Exam setup | exam |
| `exam-run` | Exam runner | exam |
| `exam-results` | Exam results | exam |
| `timed`, `timed-run`, `timed-results` | Timed quiz-show exam | timed |
| `progress` | Progress | progress |
| `settings` | Settings | settings |
| `sources` | Sources list | sources |
| anything else | Not found | — |

- First launch guard: if `!state.onboarded` and route ∉ {welcome, level} → force `welcome`.
- "Bare" mode (no sidebar, no tab bar): `welcome`, and `level` while not onboarded.
- "No tab bar" mode: `timed`, `timed-run`, `timed-results`, `exam-run` (sidebar stays on desktop).
- On each navigation: teardown of key handler + timers/listeners; scroll to top (except `exam-run`, `timed-run`); window title = «Шорашим — основы иудаизма и подготовка к гиюру» (fixed).

### 0.2 Navigation items (icon name → label)
Order: `map` «Карта» · `review` «Повторение» · `timer` «Экзамен» (→ `#/timed`) · `exam` «Тесты» (→ `#/exam`) · `progress` «Прогресс» · `settings` «Настройки»; sidebar adds `sources` «Источники».
- Bottom tab bar (< 900 px): first 6 items. Sidebar (≥ 900 px, 230 px wide, sticky): brand + 7 items.
- Brand: `assets/img/app-icon.webp` (36×36) + «Шорашим» (serif bold) / «основы иудаизма» (small muted); links to map.
- Active item: gold2 text (+ gold-soft bg and inset 2px gold left bar in sidebar).
- Skip link: «К содержимому». App title: «Шорашим — основы иудаизма и подготовка к гиюру». (Desktop app: skip link optional.)

### 0.3 Global toasts / dialogs / overlays
- **Toast** (`UI.toast(text, kind)`): bottom-centre, above tab bar (desktop 1.5rem from bottom); visible 2.6 s, fade out 0.5 s; kinds: default, `ok` (ok border), `bad` (bad border).
- **Startup toast** (storage unavailable): «Хранилище браузера недоступно: прогресс не сохранится» (bad).
- **Confirm dialog** (`UI.confirm(title, text, okLabel, danger)` → bool): backdrop `rgba(3,5,12,.66)` + blur; modal ≤ 460 px, radius 20. Buttons right-aligned: «Отмена» (ghost) and `{okLabel}` (primary or danger). Esc / backdrop click = cancel; focus starts on OK; Tab cycles within dialog. While a dialog is open, exam/timed key handlers are inert.
- **Celebration toast** (queued, one at a time, 3.2 s + 0.4 s fade, then next): icon (emoji, 1.9rem) + bold gold serif title + small muted text; gold border. Fires confetti (28 pieces, 2.4 s; none with reduced motion). No sound.
  - Goal reached: title «Цель дня выполнена!», text `{todayXP} XP сегодня · серия {n} plural(n, день|дня|дней)` + optional « · получена заморозка серии ❄», icon «🔥».
  - Badge: title «Новый значок: «{title}»», text = badge text, icon = badge icon.
- **XP pop**: pill «+{n} XP» (gold bg, on-gold text), 1.3 s rise-and-fade. Suppressed for per-card XP (`quiet`).
- **Not found** view: header «Не найдено» (or «Раздел закрыт» when locked) with back «Карта» → `#/map`; empty card with Star-of-David SVG + message (default «Страница не найдена.») + primary «На карту».

### 0.4 Keyboard (global rule)
View key handler ignored when Alt/Ctrl/Meta is held or focus is in a text input/textarea/select.

### 0.5 Page header pattern (`UI.header`)
Optional back link (chevron + label, default «Назад»; muted, hover surface bg) → kicker (uppercase gold, letter-spacing .12em, .72rem) → h1 (serif) → sub (muted).

---

## 1. Welcome (`#/welcome`)
**Layout:** hero (16:9, radius 24, overlapping body by 3rem) with `assets/img/welcome.webp` (fallback: gold Star-of-David SVG on `linear-gradient(160deg,#1b2547,#0b1020)` + gold radial glow) and a bottom dark gradient `rgba(8,11,22,.92)`; text over hero; then a card.
**Strings:**
- Kicker «Добро пожаловать»; h1 «Шорашим»; lead «Спокойная, последовательная подготовка к гиюру и собеседованию в бейт-дине.»
- Feature list (gold icons book / cards / exam):
  - «**8 разделов** — от основ веры до бейт-дина, уроки по 5–10 минут.»
  - «**Карточки** с интервальным повторением: 1 → 3 → 7 → 14 → 30 дней.»
  - «**Тесты и пробный бейт-дин** с разбором ошибок.»
- Notice (gold-soft box, sources icon): «Материалы требуют проверки раввином вашей общины. Приложение — помощник в учёбе, а не замена учителю.»
- Small muted: «Всё работает без интернета; прогресс хранится только на этом устройстве.»
- Button (primary lg, right): «Начать» → `#/level`.

## 2. Movement / level choice (`#/level`)
**Header:** back → `#/welcome` (first run) or `levelReturn` (`#/map` or `#/settings`, default `#/settings`), label «Назад». Kicker «Шаг 2 из 2» (first run) / «Течение». h1 «Выберите течение». Sub «Уровни вложены друг в друга: консервативный включает всё реформистское, ортодоксальный — всё. Прогресс сохранится; уроки и вопросы другого уровня появятся или скроются.»
**Cards** (3, grid 3-col ≥ 720 px, centered; selected = gold border + 2px gold-soft ring; hover lift −3px):
image `assets/img/level-{reform|conservative|orthodox}.webp` in 104 px circle (fallback: nested-rings SVG, 1/2/3 rings filled) → «Уровень {1|2|3}» (gold uppercase) → title → text:
| id | title | short | text |
|---|---|---|---|
| reform | «Реформистский» | «Реформа» | «База: общее для всех течений — вера, история, праздники, этика и смысл заповедей.» |
| conservative | «Консервативный (Масорти)» | «Масорти» | «Всё из базового уровня плюс то, что требуют Масорти: соблюдение Шаббата, кашрута, молитвы.» |
| orthodox | «Ортодоксальный» | «Ортодоксия» | «Полный уровень: включает всё предыдущее и подробную галахическую практику.» |

**Interactions:** pick → if onboarded and a different level is already set: confirm title «Сменить течение на «{title}»?», text «Прогресс сохранится; уроки и вопросы другого уровня появятся или скроются.», OK «Сменить». On apply: `settings.level = id`, `onboarded = true`, save, toast «Выбрано: {title}», go `#/map` (first run) or back target. Same level / first run: apply without a dialog. Progress is never reset.
**Edge:** level null → content treated as orthodox (level 3) everywhere.

## 3. Map / home (`#/map`)
**Header row** (wraps; on ≤ 520 px stats become a 3-col grid):
- Level chip (pill, gold2, uppercase .72rem): «{title} уровень» + chevron + small lowercase «сменить» → sets `levelReturn='#/map'`, go `#/level`.
- h1 «Карта разделов».
- Stats: **goal ring** (48 px; links to `#/progress`): ring text = today XP or «✓» when done; «{min(x,g)} / {g} XP» + small «цель дня» / «цель дня выполнена»; done → ring stroke ok colour. **Streak stat** (flame icon; lit orange `#f0a24a` when today is kept) → `#/progress`: bold `{n}`, small «plural(n, день|дня|дней) подряд» + « · ❄ {freezes}» if freezes > 0. **Due stat** (review icon) → `#/review`: bold `{due}`, small «plural(due, карточка|карточки|карточек) на сегодня».

**Reminder banners** (in-app only, below header):
- If goal not yet reached: streak at risk (today not kept and streak > 0) → (flame, orange tint) «Серия {n} plural(день|дня|дней) под угрозой — наберите ещё {left} XP сегодня»; else (star, gold tint) «До цели дня — {left} XP. Один урок даёт 10 XP».
- If due cards > 0: link banner (review icon, info tint) «{due} plural(due, карточка ждёт|карточки ждут|карточек ждут) повторения» + right label «Повторить» → `#/review`.

**Timed CTA** (indigo radial gradient `#3a2f8a → #15134a → #08081d`, gold border `rgba(212,175,55,.55)`, radius 20) → `#/timed`: timer icon (`#f0cf6a`), bold «Экзамен», small «Вопрос, четыре варианта и таймер — как в телевикторине», gold pill «Играть».

**No content:** notice (book icon) «Материалы ещё готовятся. Разделы появятся после сборки содержания.»

**Path** (vertical list of 8 sections, connector line between nodes; gold line under done nodes). Section order = manifest or defaults:
| # | id | title | subtitle | fallback icon |
|---|---|---|---|---|
| 1 | osnovy | Основы | Кто такой еврей, Тора и вера | 📜 |
| 2 | ivrit | Иврит и молитва | Алфавит, Шма, Амида, сидур | א (Hebrew font) |
| 3 | shabbat | Шаббат | Святость седьмого дня | 🕯️ |
| 4 | kashrut | Кашрут | Законы пищи | 🍞 |
| 5 | prazdniki | Праздники | Еврейский календарь | 🌙 |
| 6 | semya | Жизненный цикл и семья | От брит-милы до траура | 🏠 |
| 7 | etika | Этика | Пиркей Авот, цдака, речь | ⚖️ |
| 8 | beit-din | Бейт-дин | Как проходит собеседование | 🏛️ |
(Bundle data may override title/subtitle/icon; unknown ids get icon ✡.)

Per item: node circle (52 px; 64 px ≥ 720; 44 px ≤ 400) + card with emblem `assets/img/section-{id}.webp` (64 px, radius 16; hidden ≤ 400 px) + title (serif) + subtitle (muted) + status line + right side.
- State `soon` (not in bundle): node lock, tag «Скоро», right lock icon, opacity .58, emblem grayscale .8, not clickable.
- State `locked`: node lock, tag «🔒 Закрыт: пройдите раздел {N}» (N = number of previous available section), lock icon, not clickable.
- State `open`/`done`: meta «{read}/{lessons} уроков · тест {best}%» (or «—»), right progress ring (54 px) with section %; node = number (open) or check (done, gold fill). Links to `#/section/{id}`.
- Current = first `open` section: bigger pulsing gold node (60/72/50 px), gold card border, pill «Продолжить» (if any lesson read or test taken) else «Начать».
- Footer (when `unlockAll` off), small muted centred: «Следующий раздел открывается, когда прочитаны все уроки и тест сдан на 80% и выше. Свободный режим — в настройках.»

**Core logic:** `sectionStats` pct = round(100·(0.6·read/lessons + 0.4·min(best/80,1))), 0 if no lessons; `complete = allRead && best ≥ 80`. `isUnlocked`: available && (unlockAll || previous *available* section complete || it is the first available).

## 4. Section page (`#/section/{id}`)
**Guards:** missing/not available → Not found «Раздел ещё не готов.»; locked → Not found (locked) «Этот раздел пока закрыт.»
**Header:** emblem lg (96 px, radius 22; `section-{id}.webp`) · back «Карта» → `#/map` · kicker «Раздел {n}» · h1 title · sub subtitle · ring 76 px (section %).
Intro (if present): lead paragraph (markdown).
**Action cards** (2-col ≥ 600 px):
- «Карточки раздела» / «{mastered} из {cards} выучено» → `#/cards/section/{id}` (cards icon).
- Gold variant: «Тест раздела» / «Ещё не сдавали» or «Лучший результат: {best}%» + « · порог 80%» → `#/exam/s/{id}`.
**Lessons:** h2 «Уроки»; rows → `#/lesson/{id}`: circle (number, or gold-filled check if read) · bold title · small «🕒 {minutes|5} мин · карточки {m}/{n}» · thin bar (mastered %) · tag with level short name («Масорти»/«Ортодоксия») if `minLevel` ≠ reform.
**Empty:** «Для вашего уровня в этом разделе пока нет уроков.»

## 5. Lesson reader (`#/lesson/{id}`)
**Guards:** unknown or invisible for level → «Урок не найден.»; section locked → locked «Этот раздел пока закрыт.»
**Layout:** fixed top reading-progress bar (5 px, surface3 track, gold gradient fill with glow; % of window scroll, 100% if nothing to scroll) → header (back = section title → `#/section/{id}`; kicker «Урок {pos} из {count} · {minutes|5} мин»; h1 lesson title) → article card (padding 1.6/1.5rem; 2.2/2.6rem ≥ 720; font 1.04rem, line-height 1.75):
- Review badge, floated right (gold-soft pill): «Требует проверки раввином» (tooltip «Материалы требуют проверки раввином вашей общины»).
- Blocks (filtered by level, inheriting lesson minLevel), each followed by its `differs` notes:
  - `h` → serif h2 1.3rem with 36×2 px gold bar above.
  - `p` → paragraph. `list` → ul/ol, gold markers.
  - `hebrew` → centred figure (gold-soft gradient on surface2, gold 25% border, radius 12): Hebrew line (RTL, Hebrew font 1.85em, gold2; parchment `#5a400a`) → transliteration (italic muted; only if setting `translit` on) → Russian translation → caption ref (.8rem gold).
  - `quote` → blockquote, 3px gold left border, serif italic 1.08em, cite small muted sans.
  - `note` → info box: label «Совет» (uppercase info) + text.
  - `term` → dashed box: dt serif bold term + inline Hebrew; dd text.
- Lesson-level `differs`.
- Sources footer (if any): h3 «Источники» (gold); list of sources (title; external link icon if `http(s)` URL — opens in browser; tag movement short name); «**Ссылки:** {c1}; {c2}…».
**Actions** (centred): unread → primary lg «Отметить прочитанным»; read → «✓ Прочитано» (ok colour) + primary lg «К карточкам урока» → `#/cards/lesson/{id}`.
**Next link** (if a next visible lesson): «Следующий урок: {title} →».
**Mark read:** `lessonsRead[id] = today`, `touch()` (activity +1), toast «Урок отмечен прочитанным», `award(10)` (XP pop), actions swap in place and focus the button. Re-reading never re-awards (button gone).

## 6. Flashcards (`#/cards/...`, `#/review`)
**Contexts:**
| Route | h1 | kicker | back (label) | "word" | done button | deck order |
|---|---|---|---|---|---|---|
| `cards/lesson/{id}` | «Карточки урока» | lesson title | section (section title) | «уроке» | «К разделу» | lesson cards, level-filtered |
| `cards/section/{id}` | «Карточки раздела» | section title | section (section title) | «разделе» | «К разделу» | due (shuffled) → new → rest (shuffled) |
| `review` | «Повторение дня» | today `DD.MM.YYYY` | `#/map` («Карта») | «повторении» | «На карту» | all due cards of unlocked sections, shuffled |
Guards: lesson unknown or section locked → «Урок не найден.»; section missing/locked → locked «Раздел закрыт или не найден.»

**Empty deck:** card with star SVG + text (default «Карточек нет.»; review: «На сегодня повторять нечего. Изучайте новые карточки в разделах — они появятся здесь в нужный день.») + primary «На карту».

**First-run explainer** (shown instead of the first card while `cardsHelpSeen` false; reopenable): h3 «Как это работает?», ordered list:
1. «Вспомните ответ про себя и нажмите на карточку, чтобы перевернуть её.»
2. «Оцените себя честно: «Вспомнил» или «Не вспомнил».»
3. «„Вспомнил" — карточка вернётся через 1, затем 3, 7, 14 и 30 дней.» (source: «Вспомнил» — карточка вернётся через 1, затем 3, 7, 14 и 30 дней.)
4. «Не вспомнил» — покажем её ещё раз в этом уроке, а расписание снова начнётся с 1 дня.
5. «Карточка считается «выученной», когда вы вспомните её на 30-дневном интервале.»
Button primary lg «Понятно, начнём» → `cardsHelpSeen = true`, save, show current card.
(Items 3–4 exactly: `«Вспомнил» — карточка вернётся через 1, затем 3, 7, 14 и 30 дней.` / `«Не вспомнил» — покажем её ещё раз в этом уроке, а расписание снова начнётся с 1 дня.`)

**Card screen:**
- Top row: «{pos} / {queueLen}» · step bar (segmented if ≤ 20) · tag «повтор» for re-queued entries, else 5 level dots (filled = SRS level, tooltip «Уровень {l} из 5») · pill button «Как это работает?» → explainer.
- Flip card (min-height 300, radius 22, 3D Y-flip .65 s): front = kicker lesson title, card front (serif 1.35rem), hint «Вспомните ответ и нажмите, чтобы перевернуть»; inner gold hairline frame. Back = kicker «Ответ», back text (sans 1.12rem), `differs` notes; gold-tinted bg.
- Buttons (disabled until flipped): primary lg «Вспомнил» `[1]`, secondary lg «Не вспомнил» `[2]` (min-width 150).
- Grade note (above card, 1.4 s then fade): first pass knew → `c.m` ? «Карточка выучена! Следующий контроль через {d} plural(день|дня|дней)» : «Следующий показ: через {d} plural(день|дня|дней)» (ok); missed → «Покажем ещё раз в этом {word}, а затем завтра» (bad). Repeat pass: «Принято» (ok) / «Завтра покажем снова» (bad).

**Logic:** grade on a card not yet flipped = flip instead. First pass calls `srs.grade(id, knew)` (knew: level+1 max 5, due = today + [1,3,7,14,30][level−1], mastered `m=true` if level was already 5; miss: level 0, m=false, miss+1, due tomorrow; n+1, last=today; `touch()`). Repeat pass does **not** call srs. A missed card is re-queued once at the end (`repeat:true`). Each grade (both passes, both answers) → `award(1, quiet)`.
**Keys:** Space/Enter = flip (unless focus on another button/link); `1` or ArrowLeft = «Вспомнил»; `2` or ArrowRight = «Не вспомнил».
**Finish:** `count('decks')`, `award(5)`. Done card: ring 110 px (`knew/total` %, label «Вспомнил»), h2 doneText (default «Колода пройдена!», review «Повторение на сегодня завершено!»), «Вспомнил: **{knew}** · Не вспомнил: **{missed}**», xp line «+{gained} XP за колоду · сегодня {today} / {goal} XP», small muted «Карточки, которые вы не вспомнили, покажем ещё раз в этом {word}, а завтра они снова в работе. Остальные — через 1, 3, 7, 14 и 30 дней.», buttons primary `{doneLabel}` → back, ghost «На карту».

## 7. Exam setup «Тесты» (`#/exam`, `#/exam/s/{id}`)
Header kicker «Проверка знаний», h1 «Экзамен», sub «Набор вопросов каждый раз новый; чаще попадаются те, в которых были ошибки.»
**Mode cards** (radio group, 3-col ≥ 720): «По разделу» / «Проверочный тест: 80% и выше открывает следующий раздел.» (exam icon); «Общий экзамен» / «Вопросы из всех разделов вперемешку.» (map icon); «Пробный бейт-дин» / «Открытые вопросы как на собеседовании; оцениваете себя сами.» (sources icon). Setup state persists in memory across visits (default section mode, count 20, onlyUnlocked true); `/exam/s/{id}` presets section mode + section.
**Setup card:**
- section: select «Раздел», options «{n}. {title}» + « — лучший {best}%» (unlocked sections only; default first unlocked).
- general: switch «Только открытые разделы».
- beitdin: muted «Вопросы задаются по одному, как на настоящем собеседовании. Подумайте или ответьте вслух, затем сравните с эталоном.»
- «Количество вопросов» chips 10 / 20 / 30 / 50.
- Small muted «Доступно вопросов: {n}» + « — будет задано {n}» if n < count.
- primary lg «Начать» (disabled when 0 available) → `exam.build(...)`, go `#/exam-run`.
Available count: section = questions of section; general = all questions in (unlocked | all available) sections; beitdin = open/typed questions from available sections + all from beit-din.
**Selection (`exam.build`):** weighted sampling without replacement, weight = max(0.25, 1 + 2·miss − 0.5·min(streak,3)). beitdin: ~70% open questions from beit-din, rest open/typed from other sections, top-up from beit-din, shuffled. retry: given ids, shuffled. mc options and match right-hand values shuffled.

## 8. Exam runner (`#/exam-run`) — section test, general exam, mock beit-din, mistakes retry
No session → go `#/exam`. Tab bar hidden.
**Top:** «✕ Выйти» · kicker `{mode title}` + « · {section}» · progress «Вопрос {i} из {n}» (beitdin: «Вопрос раввина {i} из {n}») + step bar.
Mode titles: section «Тест раздела», general «Общий экзамен», beitdin «Пробный бейт-дин», retry «Работа над ошибками», timed «Экзамен (на время)».
**Question card:** meta (gold uppercase) «{section} · {format} · {difficulty}»; formats: mc «выбор ответа», tf «верно / неверно», match «сопоставление», typed «впишите ответ», open «открытый вопрос»; difficulty «лёгкий»/«средний»/«трудный». Prompt h2 (1.28rem) — or in beitdin a chat bubble (radius 18 18 18 4) labelled «Раввин» with serif 1.22rem prompt.
**Bodies:**
- mc: option buttons with `[k]` key hint, keys 1…n.
- tf: two buttons «Верно» `[1]`, «Неверно» `[2]`.
- typed: text input placeholder «Ваш ответ» + primary «Проверить» (Enter submits). Check: normalise (lowercase, ё→е, strip nikkud, punctuation → space, collapse) vs answer + `accept[]`.
- match: rows «left» + select «— выберите —» (Hebrew options RTL) + primary «Проверить». Score = good/pairs (1 only if all).
- open: textarea (4 rows) placeholder «Сформулируйте ответ письменно или вслух (необязательно записывать)» + primary «Показать эталонный ответ» → textarea read-only; model box (gold-soft): h3 «Эталонный ответ», model text, h4 «Проверьте себя — упомянуты ли ключевые мысли:» + key-point checkboxes (not scored); self-grade buttons primary «Ответил верно» (1), secondary «Частично» (0.5), ghost «Не ответил» (0).
**Feedback** (inputs disabled): mc/tf — correct option green, chosen wrong red; match — rows green/red, wrong rows add «→ {correct}». Box class ok/part/bad with title «Верно!» / «Частично» / «Неверно»; if wrong & format ∉ {open, match}: «Правильный ответ: **{text}**»; explanation; «Источник: {source}»; differs notes. Button primary lg «Далее» or «Результаты» `[Enter]`, auto-focused.
**Core:** each answer → `exam.record(qid, r)`: n+1; correct → streak+1; else miss += 0.5 (partial) or 1, streak 0; last=today; `touch()`.
**Keys:** digits 1…n pick mc/tf option; Enter after answering = next.
**Quit:** confirm «Прервать экзамен?» / «Ответы на текущие вопросы сохранятся в статистике, но результат не будет засчитан.» / «Выйти» → clear session, go `#/exam`.
**Finish (`exam.finish`):** pct = round(Σscore/total·100); log to `exams` (max 100); section mode: update `best` if higher. XP: section passed (≥ 80) 20 (+10 if 100), failed 0; beitdin 15 + `count('beitdin')` (regardless of score); general 2 × correct; retry 1 × correct. `award(xp)`.

## 9. Exam results (`#/exam-results`)
No result → `#/exam`. Header back «Экзамен» → `#/exam`, kicker «{mode title}» + « · {section}», h1 «Результаты».
Result card (section mode: green border if passed): ring 140 px (pct, label «Результат»), h2: section → «Тест сдан!» (ok) / «Пока не сдан» (bad); other modes → «Отличный результат!» (≥ 80) / «Есть над чем поработать». «Набрано **{score, 1 decimal}** из **{total}**» + « · порог 80%» (section). XP line: «+{xp} XP» or «Без XP: тест не сдан — попробуйте ещё раз», + « · сегодня {today} / {goal} XP». Section passed: muted «Если все уроки раздела прочитаны, следующий раздел открыт.»; failed: «Повторите уроки и карточки, затем попробуйте снова.»
Buttons: primary «Повторить ошибки» (only if mistakes) → build retry with mistake ids, `#/exam-run`; secondary «Новый экзамен» → `#/exam`; ghost «К разделу» (if section) else «На карту».
Mistakes: h2 «Ошибки ({n})»; each card: prompt (serif bold), «Ваш ответ: {user}» (bad colour; open → «Ответил верно»/«Частично»/«Не ответил»), «**Эталон:** {model}» (open) or «**Правильно:** {correct}» (ok colour), explanation (muted), «Источник: {source}». None: centred muted «Ошибок нет — прекрасно!»
(Note: XP 0 for general/retry with 0 correct also shows the «Без XP…» line — PWA quirk.)

## 10. Timed quiz-show exam «Экзамен»
Distinct "studio" palette scoped to `.tq` container (see Theme §T.3). Tab bar hidden on all three screens.

### 10.1 Intro / setup (`#/timed`)
- Back «На главную» → `#/map`.
- Hero: static timer ring (96 px, full, shows base seconds) + kicker «Режим телевикторины», h1 «Экзамен» (gold gradient text `#fff2c4 → #d4af37`, 2.2rem), sub «Сначала — вопрос. Через пару секунд — четыре варианта ответа и таймер. Один выбор, без права на исправление.»
- Card: «Количество вопросов» chips **10 / 15 / 20 / 30** (default 20, in-memory); «Время на вопрос» segmented «20 с» / «30 с» (writes `settings.timedSeconds`, saved); switch «Звуковые сигналы» (`timedSound`; turning on plays the "right" chime); switch «Автопереход к следующему вопросу» (`timedAuto`).
- Rules list:
  - «Ответ — клик или клавиши [A]–[D] / [1]–[4].»
  - «Порядок — от лёгких к трудным: сначала разминка (на лёгкие +5 с), затем средние и трудные вопросы.»
  - «„Не знаю" и истёкшее время считаются ошибкой — такие вопросы потом попадаются чаще.» (exact: `«Не знаю» и истёкшее время считаются ошибкой — такие вопросы потом попадаются чаще.`)
  - «Вопросы — из открытых разделов, по вашему уровню ({level title lowercased}); каждый раз — новый набор.»
- «Доступно вопросов: {n}» + « — будет задано {n}»; if 0: «В открытых разделах пока нет вопросов с выбором ответа.»
- Gold button lg «Начать игру» (disabled if 0) → build, `#/timed-run`.
**Pool:** mc (≥ 2 options) and tf questions from unlocked sections, level-filtered. **Selection:** tiers 25% easy / 40% medium / rest hard (20 → 5/8/7; missing difficulty = medium), quotas borrow from neighbour tiers if short; weighted within tier; avoid ids from last 3 timed runs (`examHistory`) unless tier too small; order easy → medium → hard. mc options shuffled; tf options fixed «Верно», «Неверно». Seconds per question = base (20/30) + 5 for easy.

### 10.2 Run (`#/timed-run`)
No run → `#/timed`. Wide page (max 1060 px).
- Top bar: pill «✕ Выйти» · «Вопрос **{i}** из {n}» (serif, number gold2) · score pills «✓ {r} верно» (ok), «✕ {w} неверно» (bad), «? {s} пропущено» (muted) — word labels hidden ≤ 520 px.
- Ladder (≥ 1024 px only, right column 96 px): numbers n…1 top-down; right = gold bar, wrong = red bar, skip/timeout = grey bar, current = orange-gold pill.
- Timer ring 84 px (track `rgba(5,6,25,.85)`, stroke gold2 7px, number 1.7rem bold tabular); `waiting` (opacity .4) during intro; `danger` ≤ 5 s (stroke `#ff4d3d`, number `#ff6a5c` pulsing); `stopped` (opacity .6) after answer. aria «Осталось секунд: {n}».
- Question panel: hexagonal gold-framed pill (cut 34px; inner indigo gradient), reveal animation; meta «{section}» + « · верно или неверно?» (tf) + difficulty tag «лёгкий» (ok) / «средний» (info) / «трудный» (bad); prompt serif 1.32rem 600. Horizontal gold line through panel.
- Phases: **intro** 2.5 s (only question + «Не знаю — дальше» button) → **active**: options appear staggered (90 ms each), 2-col ≥ 700 px, hex buttons «A: …» … «D: …» (letter gold2 800); options chime; timer starts (100 ms ticks; tick beep each second ≤ 5).
- Answer (click, A–D / 1–4 / Numpad1–4): option turns **locked** orange, lock tone, reveal after 1.1 s (0.35 s reduced motion). «Не знаю — дальше» (allowed in intro or active) or timeout → reveal immediately (intro skip renders options disabled first).
- Reveal: correct option green (flash ×3), chosen wrong red, others dimmed .5; sounds right/wrong/skip. Verdict (serif 1.15rem): «Верно!» (ok) / «Неверно» (bad) / «Не знаю» / «Время вышло» (gold2) + for non-right « · правильный ответ выделен зелёным». Row: (if auto) ghost «Пауза» + gold «Дальше» or «Итоги» `[Enter]` (focused); (if auto) 3 px progress bar filling over 2.2 s → auto-next. «Пауза» → cancels auto-advance, label «Автопереход на паузе», disabled, bar dimmed/paused.
- Every resolution → `exam.record(qid, {correct, score: 0|1})` (skip/timeout = miss 1).
- Keys: in revealed phase Enter/Space = next (unless focus on Пауза).
- Quit: confirm «Выйти из экзамена?» / «Уже данные ответы сохранятся в статистике, но результат игры не будет засчитан.» / «Выйти» → stop timers, drop run, `#/timed`.
- Finish: pct = right/total; `logExam({mode:'timed', score: right, …})`; `logTimed(ids)` (keep last 3 runs); if total ≥ 20 and all right → `count('perfectTimed')` (badge «20 из 20»); XP = 2 × right, `award`. Go `#/timed-results`. Retry runs are also logged and count toward history.
- Sounds (only if `timedSound`; sine/square/triangle beeps): options 523+784 Hz; tick 880 square; lock 330 triangle; right 660+990; wrong 196+147 triangle; skip 262.

### 10.3 Results (`#/timed-results`)
No result → `#/timed`.
- Kicker «Экзамен (на время) · {seconds} с на вопрос»; ring 150 px (pct, gold2; glow if ≥ 75); h1 by pct: ≥ 90 «Блестяще!», ≥ 75 «Отличная игра», ≥ 50 «Достойный результат», else «Есть над чем поработать». Sub: ≥ 90 «Вы уверенно держитесь под давлением таймера.»; ≥ 75 «Ещё немного — и ни один вопрос не застанет врасплох.»; ≥ 50 «Половина пути пройдена: разберите ошибки ниже.»; else «Разберите ошибки и повторите уроки — следующая игра пройдёт лучше.»
- Tiles (3): `{right}` plural(верный|верных|верных) (ok); `{wrong}` plural(ошибка|ошибки|ошибок) (bad); `{skip}` «пропущено» (gold2; skip counts skip + timeout).
- Small «Всего вопросов: {total} · верно {pct}%»; xp line «+{xp} XP · сегодня {today} / {goal} XP».
- Buttons: gold «Ещё раз» (new run with current count) · secondary «Повторить ошибки ({n})» (if misses; run of those ids, shuffled) · ghost «На главную» → `#/map`.
- Review: h2 «Разбор ошибок и пропусков»; cards: prompt, «Ваш ответ: {text}» / «не знаю (время вышло)» / «не знаю», «**Правильно:** {text}», explanation, «Источник: {source}». None: «Ни одной ошибки — превосходно!»

## 11. Progress (`#/progress`)
Header kicker «Ваш путь», h1 «Прогресс».
1. **KPIs** (2-col, 4-col ≥ 720; icon gold2, value serif 1.6rem): `{streak}` «plural(день|дня|дней) подряд» (flame); `{read}/{lessons}` «уроков прочитано» (book); `{mastered}` «карточек выучено (интервал 30 дней)» (cards); `{exams}` «plural(экзамен|экзамена|экзаменов) пройдено» (exam). Totals exclude «Скоро» sections.
2. **«Цель и серия»** card: goal ring 76 px (non-link) · streak block (flame lit if today kept) `{n}` «plural подряд» · freezes «❄❄» (on/off) + «{fr} из 2 заморозок» · «{xpTotal}» «XP всего». XP week: 7 bars (last 7 days, height = xp/max(goal, max xp); green goal line; kept day = solid gold; label = weekday «вс пн вт ср чт пт сб» or «❄» if freeze day; today label gold bold; tooltip «{day}: {xp} XP» + « (заморозка)»). Collapsible «Как работают цель, серия и заморозки»:
   - «**XP** начисляются за учёбу: урок прочитан — 10, каждая оценённая карточка — 1 («Не вспомнил» тоже: важна работа), колода пройдена — 5, тест раздела сдан — 20 (+10 за 100%), экзамен — 2 за каждый верный ответ, пробный бейт-дин — 15.»
   - «**Цель дня** — {g} XP ({goalTitle}). Её можно изменить в настройках: 10, 20, 30 или 50 XP. Сегодня набрано {x} XP.»
   - «**Серия** — сколько дней подряд выполнена цель дня. Если сегодня цель ещё не выполнена, серия не сгорает до конца дня.»
   - «**Заморозка серии** ❄ — за каждые 7 дней серии вы получаете одну (хранится не больше двух). Если вы пропустите день, заморозка спишется автоматически и серия сохранится.»
3. **Badges grid** «Значки · {got} из {total}» (2 / 3 ≥ 600 / 5 ≥ 1100 cols). Item: emblem 64 px circle `assets/img/badge-{id}.webp` (fallback emoji / Hebrew letter), bold title, small text; locked = grayscale .4 opacity + lock icon top-right, muted title; earned = gold border + glow. 20 badges:
   | id | icon | title | text |
   |---|---|---|---|
   | first-lesson | 📖 | Первый урок | Прочитать первый урок |
   | first-deck | 🃏 | Первая колода | Пройти колоду карточек до конца |
   | goal-first | 🎯 | Цель дня | Впервые выполнить дневную цель |
   | first-test | ✅ | Тест сдан | Сдать тест раздела на 80% и выше |
   | perfect | 💯 | Без ошибок | Сдать тест раздела на 100% |
   | streak-3 | 🔥 | Три дня подряд | Серия 3 дня |
   | streak-7 | 🔥 | Неделя | Серия 7 дней |
   | streak-30 | 🏆 | Месяц | Серия 30 дней |
   | cards-100 | 🧠 | 100 карточек | Выучить 100 карточек (интервал 30 дней) |
   | millionaire | ⏱️ | 20 из 20 | Экзамен на время: 20 верных из 20 |
   | beit-din-mock | 🏛️ | Пробный бейт-дин | Пройти пробное собеседование |
   | all-lessons | 📚 | Все уроки | Прочитать все уроки всех разделов |
   | section-{id} ×8 | section icon | section title | «Пройти раздел «{title}»» |
   Conditions: lessonsRead non-empty; counters.decks > 0; goalDays non-empty; any best ≥ 80; any best ≥ 100; bestStreak ≥ 3/7/30 (bestStreak = max(current, stored)); mastered (`m`) ≥ 100; counters.perfectTimed; counters.beitdin > 0; all sections available & every visible lesson read; section available & complete. Checked after every `award` (celebrate) and silently at start. Badges are never revoked.
4. **«Активность за 4 недели»**: 28 squares (14 cols, radius 5), level by `activity[day]`: 0 surface3; < 10 gold 35%; < 30 gold 65%; else gold2 + glow; tooltip «{day}: {v}».
5. **«По разделам · лучший тест»**: rows «{n}. {title}» + «Уроки {r}/{l}» gold bar + «Карточки {m}/{c}» blue bar (`#6f8fd0 → info`) + best «{best}%» / «—» (gold2). «Скоро» sections: tag «Скоро», opacity .55.
6. **«История экзаменов»**: none → «Вы ещё не проходили экзамены.»; else (last 20) SVG line chart (if ≥ 2; gold line, gold area gradient, dashed green 80% line labelled «80%», points with tooltip «DD.MM.YYYY: {pct}%») + list newest first: date · «{mode title}» + « · {section}» · pill «{pct}%» (ok if ≥ 80 else bad).
7. **«Слабые темы»**: per lesson, (card+question misses)/(attempts), only n ≥ 3 and rate > 0, top 5: link → lesson, bold lesson title, small section, red bar, «{pct}% ошибок». None: «Пока недостаточно данных: решайте тесты и повторяйте карточки.»

## 12. Settings (`#/settings`)
h1 «Настройки». Rows: bold label + small muted sub, control right (wrap).
**Card 1:**
- «Течение» / «{level title} · прогресс сохранится при смене» + secondary «Сменить течение» → `levelReturn='#/settings'`, `#/level`.
- «Тема» / «Тёмная — основная»: segmented «Тёмная» | «Пергамент» → apply immediately.
- «Размер шрифта» / «Иврит масштабируется тоже»: «A−» | «A» | «A+» | «A++» (root 15 / 17 / 19 / 21.5 px; default step 1).
- Switch «Открыть все разделы» / «Свободное изучение без последовательной разблокировки» (`unlockAll`, default off).
- Switch «Показывать транслитерацию» / «Русская транскрипция под текстом на иврите» (`translit`, default on).
- «Экзамен на время» / «Секунд на вопрос»: «20 с» | «30 с» (default 30).
- Switch «Звук в экзамене на время» / «Короткие сигналы: таймер, верно/неверно» (default off).
- Switch «Автопереход в экзамене на время» / «Следующий вопрос через 2 секунды (можно поставить на паузу)» (default on).
- Hebrew sample, centred, RTL, Hebrew font 1.85em gold2: «שְׁמַע יִשְׂרָאֵל».
- Each switch change → save + toast «Сохранено».
**Card 2 «Цель и серия»:** «Цель дня» / «Сколько XP набирать в день. Серия растёт, когда цель выполнена.» — 4 stacked segment buttons «Лёгкая / 10 XP», «Обычная / 20 XP», «Серьёзная / 30 XP», «Интенсив / 50 XP» (default 20). Change → save, `award(0)` (may complete today's goal and celebrate), toast «Цель дня: {n} XP». (Reminder time row «Время напоминания» / «Ежедневное уведомление на телефоне» only exists with native notifications — skip on desktop or implement as optional; toast «Напоминание: {HH:MM}».) Small muted: «XP даются за уроки, карточки, тесты и экзамены. За каждые 7 дней серии — заморозка ❄ (до двух), она сохранит серию, если вы пропустите день. Подробнее в разделе «Прогресс».» (last sentence part is a link → `#/progress`).
**Card 3 «Данные»:** small «Прогресс хранится только в этом браузере.» (+ « (хранилище недоступно — прогресс не сохранится после закрытия)» if storage failed). Desktop wording may say «на этом устройстве». Buttons:
- secondary «Экспорт прогресса» → write pretty JSON of state + `exportedAt` ISO, file name `shorashim-progress-YYYY-MM-DD.json`; toast «Файл прогресса сохранён».
- secondary «Импорт прогресса» (file picker, .json) → confirm «Импортировать прогресс?» / «Текущий прогресс на этом устройстве будет заменён данными из файла.» / «Импортировать» → success: apply settings, toast «Прогресс импортирован» (ok), re-render; errors (toast bad): «Файл не является корректным JSON.», «Это не файл прогресса «Шорашим».», «Файл создан более новой версией приложения.», read failure «Не удалось прочитать файл».
- danger «Сбросить прогресс» → confirm (danger) «Сбросить весь прогресс?» / «Будут удалены прочитанные уроки, карточки, результаты экзаменов и серия дней. Это действие нельзя отменить (кроме импорта ранее сохранённого файла).» / «Сбросить» → defaults kept only theme + fontStep (level and onboarded reset), toast «Прогресс сброшен», go `#/welcome`.
**Card 4 «О приложении»:** key/values «Версия приложения» `1.2.0`; «Версия содержания» `{contentVersion|—}` + « ({version})»; «Статус материалов» `{reviewStatus|требует проверки раввином}`; «Разделов готово» «{available} из {total}». Notice «Материалы требуют проверки раввином вашей общины. Окончательные решения о гиюре принимает бейт-дин.» Small «Документация в папке приложения: docs/ROADMAP.md, docs/CONTENT-SCHEMA.md, docs/SOURCES.md, docs/IMAGE-SLOTS.md.» Secondary button «Источники ({count})» → `#/sources`.

## 13. Sources (`#/sources`)
Header back «Настройки» → `#/settings`, h1 «Источники», sub «Книги и сайты, на которые опираются материалы. Ссылки открываются во внешнем браузере (нужен интернет).» One card per section with sources: h3 «{n}. {title}», list (title, external link if URL, movement tag). Empty: «Список источников появится вместе с материалами разделов.»

## 14. Core state & rules referenced by UI (storage `giyur-study:v1`, schema 1)
- Settings defaults: level null, theme dark, fontStep 1, unlockAll false, translit true, timedSeconds 30, timedSound false, timedAuto true, dailyGoal 20 (∈ 10/20/30/50 else 20), reminderTime «19:00».
- XP: lesson 10, card 1, deck 5, test 20, perfect +10, exam/timed 2 per right, retry 1 per right, beit-din 15. XP per local day in `xp`, `xpTotal`.
- Goal reached → `goalDays[today]`; streak = consecutive kept days (goalDays or freezeDays) ending today, or yesterday if today not kept. On goal reach, if streak % 7 == 0 and freezes < 2 and not awarded today → +1 freeze. App start: if today and yesterday not kept, find last kept day within 60 days; if gap ≤ freezes, fill gap with freezeDays and spend freezes.
- Old saves without `xp`: past active days become goal days.
- `touch()` increments `activity[today]` (lesson read, every SRS grade, every recorded answer).
- Level filtering: item visible if `minLevel` (inherited from lesson/block) ≤ user level; questions tied to hidden lessons are hidden.

---

## T. Theme

### T.1 Dark (default) — `:root`
| token | value | | token | value |
|---|---|---|---|---|
| bg0 | `#0b1020` | | gold | `#d4af37` |
| bg1 | `#141a2e` | | gold2 | `#e6c36a` |
| surface | `#151c32` | | gold-soft | `rgba(212,175,55,.14)` |
| surface2 | `#1b2340` | | on-gold | `#1b1404` |
| surface3 | `#232c4d` | | ok / ok-soft | `#6cc49a` / `rgba(108,196,154,.14)` |
| line | `rgba(255,255,255,.06)` | | bad / bad-soft | `#e88b7a` / `rgba(232,139,122,.14)` |
| line2 | `rgba(255,255,255,.12)` | | info / info-soft | `#8fb1ea` / `rgba(143,177,234,.12)` |
| text | `#ece7da` | | violet / violet-soft | `#b9a3ec` / `rgba(185,163,236,.12)` |
| muted | `#a9a496` | | streak flame | `#f0a24a` (tint `rgba(240,162,74,.12)`, border `.4`) |
- Shadows: `0 14px 34px rgba(0,0,0,.38), 0 2px 6px rgba(0,0,0,.28)`; sm `0 4px 14px rgba(0,0,0,.28)`.
- Card gradient overlay: `linear-gradient(180deg, rgba(255,255,255,.035), transparent 60%)` on surface.
- Body bg: radial gold glow `rgba(212,175,55,.07)` (1200×700 at 80% −10%) over `linear-gradient(180deg, bg1, bg0 70%)`.
- Emblem bg: `radial-gradient(circle at 30% 25%, #26305a, #0f1428 75%)`, border gold 40%; badge/level circles `radial-gradient(circle at 35% 30%, #27325c, #0e1327 75%)`.
- Window theme colour: dark `#0b1020`, parchment `#f4ecd8`.

### T.2 Light «Пергамент» — `[data-theme="parchment"]`
bg0 `#f3ead6`, bg1 `#ebdfc4`, surface `#fbf6ea`, surface2 `#f4ecda`, surface3 `#ebdfc6`, line `rgba(70,50,15,.10)`, line2 `rgba(70,50,15,.20)`, text `#2b2316`, muted `#6d604a`, gold `#9c7514`, gold2 `#7f5f0d`, gold-soft `rgba(156,117,20,.13)`, on-gold `#fffaf0`, ok `#2f7d55` / `rgba(47,125,85,.12)`, bad `#a4402e` / `rgba(164,64,46,.10)`, info `#335f9e` / `rgba(51,95,158,.10)`, violet `#6449a8` / `rgba(100,73,168,.10)`; shadow `0 12px 28px rgba(80,55,15,.14), 0 2px 6px rgba(80,55,15,.10)`, sm `0 3px 10px rgba(80,55,15,.10)`; card grad `rgba(255,255,255,.6) → 0`; inputs bg `#fffdf7`; Hebrew block text `#5a400a`; badge emblem bg `radial-gradient(#fffaf0, #efe3c4)`. Timed `.tq` screens keep their own dark studio palette in both themes.

### T.3 Timed studio palette (`.tq` scope)
text `#f4ecd4`, muted `#b6aecb`, surface `#16184a`, surface2 `#1c1f55`, surface3 `#272b6a`, line `rgba(255,255,255,.07)`, line2 `rgba(240,207,106,.22)`, gold `#d4af37`, gold2 `#f0cf6a`, gold-soft `rgba(212,175,55,.16)`, on-gold `#1d1402`, ok `#3fd17f`, bad `#ff6a5c`. Container: radius 26, border `rgba(212,175,55,.28)`, bg `radial-gradient(120% 70% at 50% -10%, #3a2f8a 0%, #1a1650 38%, #0a0a24 72%, #05050f 100%)`, faint conic spotlights. Gold button `tq-go`: `linear-gradient(180deg,#f6dc86,#d4af37 55%,#a8821c)`, text `#1d1402`, min-width 150. Hex frame: outer `linear-gradient(180deg,#f6dc86,#b48a22 50%,#f0cf6a)` (2px), inner `linear-gradient(180deg,#23268a,#11134a 48%,#0b0c33 52%,#191b66)`; hover inner `#3236a8 → #1b1e6e → #2a2e95`; locked outer `#ffe09a→#e08a00`, inner `#ffb53c,#e08a00,#c87400,#f2a51f` text `#1e1100`; right outer `#b6ffd2→#1f9e55`, inner `#34d07a,#16934a,#0f7d3d,#2bbd6b` text white; wrong outer `#ffc1b8→#c0302a`, inner `#e8453b,#b3241d,#9b1c16,#d9372d`. Ladder current `#f2a51f → #d4af37`.

### T.4 Typography
- Serif (headings h1–h4, card titles, prompts, flashcard front, quotes, KPI numbers): `"PT Serif", Georgia, "Times New Roman", serif`, weight 700, line-height 1.25.
- Sans (body, buttons, back of card): `system-ui, -apple-system, "Segoe UI", Roboto, "Noto Sans", "PT Sans", Ubuntu, Cantarell, "Helvetica Neue", Arial, sans-serif`; body line-height 1.6.
- Hebrew: `"SBL Hebrew", "Ezra SIL", "David", "Noto Serif Hebrew", "Frank Ruehl", "Taamey Frank CLM", serif`. Inline Hebrew 1.25em gold2; Hebrew blocks 1.85em, line-height 1.6, gold2.
- Sizes: root 17 px (steps 15/17/19/21.5); h1 2rem (1.65 ≤ 400 px); h2 1.4rem; `.h2` 1.3rem; `.h3` 1.12rem; small .85rem; kicker .72rem uppercase, letter-spacing .12em, gold, 700; kbd .7rem 600 outlined, opacity .55.
- Desktop: bundle PT Serif + Noto Sans + Noto Serif Hebrew (or SBL Hebrew if licensed) via iced font loading.

### T.5 Radii, spacing, layout
- Radius: card 18; small 12; buttons 14; chips/tags/pills 999; segment buttons 10; modal 20; flip card 22; emblem 16 (lg 22); hero 24; timed container 26.
- Card padding 1.35rem (1.1 ≤ 400), margin-bottom 1rem; row gap .6rem; content max width 820 px (timed run 1060); shell max 1100; sidebar 230 px at ≥ 900 px.
- Min touch heights: btn 46 (lg 52), chip 44, inputs 46, back/link 40.
- Page enter animation .38 s fade-up 10 px; easing `cubic-bezier(.2,.7,.2,1)`; ring fill 1.1 s; bars .9 s. All animation off under reduced motion.

### T.6 Buttons & controls
- Base `.btn`: surface2 bg, 1px line2 border, text colour, 600 .95rem; hover surface3; active scale .98; disabled opacity .45.
- primary: `linear-gradient(180deg, gold2, gold)`, on-gold text, no border, shadow `0 6px 18px rgba(212,175,55,.25)` + inner top highlight.
- secondary: surface2. ghost: transparent (with border). danger: bad-soft bg, bad text, bad-40% border.
- Chip/segment selected: gold-soft bg, gold2 text, gold border.
- Switch: 50×28 track surface3 → on gold-soft + gold border; knob 20 px muted → gold2, slides 22 px.
- Focus ring: 2px gold2, offset 3.
- Progress ring: track surface3, stroke gold, width 8 (of 100 viewBox), round cap; step bar 14 px, gold gradient fill, segment dividers in bg0.
- Tags: surface3 bg, muted text, .75rem 600.

### T.7 Hebrew RTL blocks
Right-to-left direction, Hebrew font, gold2 colour (parchment `#5a400a`). Lesson `hebrew` blocks are **centred** in a gold-tinted framed box; RTL paragraphs elsewhere (match options, inline runs) should be right-aligned / bidi-isolated. Inline Hebrew inside Russian text must be isolated (no reordering of surrounding Russian). Transliteration italic muted below; translation below that; reference caption gold .8rem.

### T.8 «differs» notes
- Box: padding .8rem 1rem, radius 12, font .95em, 3px left border; label bold in accent colour followed by text.
- Labels: reform «В реформистском иудаизме иначе:» (violet / violet-soft), conservative «В консервативном иудаизме иначе:» (ok / ok-soft), orthodox «В ортодоксальном иудаизме иначе:» (info / info-soft); unknown «В других течениях иначе:».
- Notes for the user's own movement shown expanded; the rest inside collapsible «Как в других течениях» (muted summary, min-height 44). Shown after lesson blocks, at lesson end, on flashcard back and in exam feedback.

---

## Parity checklist

1. [ ] Router with all routes in §0.1, first-run guard → welcome, not-found view.
2. [ ] Sidebar (≥ 900 px) with brand + 7 items; bottom tab bar with 6 items; active highlighting by nav group.
3. [ ] Tab bar hidden on welcome/first-run level (bare) and on timed/timed-run/timed-results/exam-run.
4. [ ] Toasts (default/ok/bad, 2.6 s) and storage-unavailable startup toast.
5. [ ] Confirm dialog (Отмена + OK/danger, Esc/backdrop cancel, focus trap).
6. [ ] Celebration toast queue + confetti (off with reduced motion) + «+N XP» pop.
7. [ ] Welcome screen: hero welcome.webp with star fallback, 3 features, notice, offline note, «Начать».
8. [ ] Level choice: 3 cards with level-*.webp / ring fallback, kicker «Шаг 2 из 2»/«Течение», back target.
9. [ ] Level change confirm «Сменить течение на «…»?»; toast «Выбрано: …»; progress kept.
10. [ ] Map header: level chip, «Карта разделов», goal ring, streak stat (lit, ❄ count), due stat.
11. [ ] Map reminder banners (streak at risk / goal left / due cards with «Повторить»).
12. [ ] Map timed CTA «Экзамен … Играть».
13. [ ] Map «Материалы ещё готовятся…» notice when no sections.
14. [ ] Path of 8 sections: emblems section-*.webp, states soon/locked/open/done/current, tags «Скоро», «Закрыт: пройдите раздел N», meta, rings, «Начать»/«Продолжить».
15. [ ] Unlock rule (all lessons read + best ≥ 80 of previous available section; unlockAll) and footer hint.
16. [ ] Section page: header emblem lg + ring, intro, two action cards, lessons list with read checks, minutes, mastered bar, level tag, empty text.
17. [ ] Lesson reader: reading progress bar, kicker «Урок i из n · m мин», review badge.
18. [ ] Lesson block types h/p/list/hebrew/quote/note(«Совет»)/term, level filtering, translit toggle.
19. [ ] Lesson sources footer with external links and «Ссылки:».
20. [ ] «Отметить прочитанным» → lessonsRead, activity, +10 XP, toast; «Прочитано» + «К карточкам урока»; next-lesson link.
21. [ ] Flashcard explainer (once; «Как это работает?» reopen; «Понятно, начнём»).
22. [ ] Deck contexts: lesson / section (due→new→rest) / daily review, with titles, kickers, back targets, empty texts.
23. [ ] Card UI: counter, step bar, level dots / «повтор» tag, flip animation, front hint, back «Ответ» + differs.
24. [ ] Grade buttons disabled until flip; keys Space/Enter, 1/←, 2/→.
25. [ ] SRS grade 1/3/7/14/30, mastered flag at level 5, miss reset; requeue once; repeat pass not graded.
26. [ ] Grade notes texts (next show / выучена / ещё раз / Принято / Завтра покажем снова).
27. [ ] +1 XP per grade (quiet), deck finish +5 and decks counter; done card with ring, counts, XP line, explanation, buttons.
28. [ ] Exam setup: 3 mode cards, section select with best %, «Только открытые разделы», beit-din note, counts 10/20/30/50, available count, «Начать».
29. [ ] Weighted question selection; beit-din 70% open mix; retry by ids.
30. [ ] Exam runner header (Выйти, mode kicker, «Вопрос»/«Вопрос раввина» i из n, steps).
31. [ ] Formats mc/tf/typed/match/open incl. rabbi bubble, model answer, key points, self-grade 1/0.5/0.
32. [ ] Typed answer normalisation + accept list; match partial scoring.
33. [ ] Feedback box Верно!/Частично/Неверно, correct answer, explanation, source, differs; «Далее»/«Результаты» Enter.
34. [ ] Per-question stats `record` for every answer in all modes.
35. [ ] Exam quit confirm «Прервать экзамен?».
36. [ ] Exam finish: history (100), section best, XP rules (20/+10, 15, 2×, 1×), beitdin counter.
37. [ ] Exam results: ring, pass/fail texts, score line, XP line, hints, buttons «Повторить ошибки»/«Новый экзамен»/«К разделу»|«На карту», mistakes list, «Ошибок нет — прекрасно!».
38. [ ] Timed setup: hero, counts 10/15/20/30, 20/30 s, sound + auto switches, 4 rules, availability, «Начать игру».
39. [ ] Timed pool (mc ≥ 2 opts + tf, unlocked, level) and tiered selection 25/40/35 with recent-run avoidance (last 3).
40. [ ] Timed run top bar (Выйти, Вопрос i из n, score pills) and ladder ≥ 1024 px.
41. [ ] Intro 2.5 s → staggered A–D options → countdown ring; +5 s for easy; danger ≤ 5 s.
42. [ ] Answer by click/A–D/1–4; lock orange 1.1 s → reveal green/red/dim; «Не знаю — дальше»; timeout «Время вышло».
43. [ ] Verdict texts + «· правильный ответ выделен зелёным»; «Дальше»/«Итоги» Enter/Space.
44. [ ] Auto-advance 2.2 s with progress bar and «Пауза» → «Автопереход на паузе».
45. [ ] Optional sound beeps (options/tick/lock/right/wrong/skip), off by default.
46. [ ] Timed quit confirm «Выйти из экзамена?».
47. [ ] Timed finish: log exam (mode timed), log ids, perfectTimed counter (≥ 20 all right), XP 2×right.
48. [ ] Timed results: kicker, ring, 4-tier title/sub, 3 tiles, totals, XP, «Ещё раз»/«Повторить ошибки (n)»/«На главную», review list, «Ни одной ошибки — превосходно!».
49. [ ] Progress KPIs (4).
50. [ ] «Цель и серия» card: goal ring, streak, freezes «x из 2 заморозок», XP total, 7-day XP bars with goal line/❄, explainer details.
51. [ ] Badges grid «Значки · x из 20», badge-*.webp art, locked styling, all 20 conditions, celebrations on unlock.
52. [ ] Activity heatmap 28 days (4 levels).
53. [ ] Per-section progress rows (lessons bar gold, cards bar blue, best %), «Скоро» rows.
54. [ ] Exam history chart (80% line) + list with pills; empty text.
55. [ ] Weak topics (≥ 3 attempts, top 5) linking to lessons; empty text.
56. [ ] Settings: change movement, theme dark/parchment, font size 4 steps, unlockAll, translit, timed seconds, timed sound, timed auto, Hebrew sample, «Сохранено» toast.
57. [ ] Daily goal 10/20/30/50 with titles; award(0) re-check; toast «Цель дня: N XP».
58. [ ] Export JSON file `shorashim-progress-YYYY-MM-DD.json`; toast.
59. [ ] Import with confirm and the 3 validation errors + read error; success toast; re-render.
60. [ ] Reset with danger confirm; keeps theme + font; toast; → welcome.
61. [ ] About card (versions, status, sections ready, notice, docs line, «Источники (n)»).
62. [ ] Sources page by section; empty text.
63. [ ] Streak/freeze logic: goal days, streak to today/yesterday, freeze every 7 (max 2), auto-spend on start; migration of old activity → goal days.
64. [ ] Level filtering of lessons/cards/questions/blocks (minLevel inheritance; null level = orthodox).
65. [ ] Dark palette tokens exactly as T.1; parchment as T.2; timed studio palette as T.3.
66. [ ] Fonts: serif headings, sans body, Hebrew stack; root size steps 15/17/19/21.5.
67. [ ] Radii/spacing/min heights as T.5; button variants primary/secondary/ghost/danger/tq-go as T.6.
68. [ ] Hebrew RTL rendering: centred gold blocks, isolated inline runs, RTL options.
69. [ ] differs notes: colour per movement, own movement expanded, others under «Как в других течениях».
70. [ ] Image fallbacks for every slot (welcome star, level rings, section/badge emoji or Hebrew letter).
71. [ ] Reduced-motion mode disables animations/confetti; shortened timed lock delay (0.35 s).
