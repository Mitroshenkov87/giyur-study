# PARITY-STATUS — Rust (iced 0.14) vs PWA 1.2.0

Read-only review against the checklist in `docs/PARITY.md` (2026-11-22). This is a code reading only; nothing was built or run.
Totals: **DONE 36 · PARTIAL 33 · MISSING 2** (out of 71).

| # | Item | Status | Note |
|---|---|---|---|
| 1 | Router, first-run guard, not-found | PARTIAL | Route enum and guard are in place (`main.rs:68-74`); not-found views exist for section, lesson and cards. Missing: window title `{h1} — Giyur Study` (fixed at `main.rs:240`) and scroll-to-top on navigation (bug 2). |
| 2 | Sidebar + tab bar, active group | PARTIAL | Sidebar has the brand, 7 items and active group (`main.rs:106-160,269-281`). Missing: the < 900 px bottom tab bar (sidebar always shown, min width 720) and the 2 px gold inset on the active item (`theme.rs:787`). |
| 3 | Tab bar hidden on bare/timed/exam-run | DONE | Bare mode at `screens/mod.rs:115-121`. There is no tab bar, so nothing else to hide. |
| 4 | Toasts 2.6 s + storage toast | PARTIAL | Toasts last 4.2 s, not 2.6 s; no ok/bad kinds; no fade (`app.rs:166-174`). The storage toast exists with desktop wording (`app.rs:124-130`). |
| 5 | Confirm dialog | PARTIAL | Each screen builds its own dialog. Timed has Esc and backdrop cancel. Level has neither Esc nor backdrop cancel (`level.rs:187-244`). No screen has a focus trap. |
| 6 | Celebration queue, confetti, «+N XP» pop | PARTIAL | Single-slot toast only (`main.rs:162-193`): no icon, no confetti, no XP pop, and the queue timing is broken (bug 3). |
| 7 | Welcome | DONE | `welcome.rs:37-185`, strings exact. Hero is a fixed 360 px, not 16:9, and has no glow. |
| 8 | Level choice | DONE | `level.rs:46-54,90-94,122-185`. |
| 9 | Level change confirm / toast | DONE | `level.rs:31-63,191-209`. |
| 10 | Map header | DONE | `map.rs:671-817`. Ring label is not centred (bug 8). |
| 11 | Reminder banners | DONE | `game.rs:426-463`, `map.rs:865-902`. |
| 12 | Timed CTA | DONE | `map.rs:912-971`. Uses a linear gradient instead of the radial one. |
| 13 | «Материалы ещё готовятся…» | DONE | `map.rs:596-602`. |
| 14 | Path of 8 sections | PARTIAL | States, tags, meta, rings and buttons are done. Missing: responsive node sizes, emblem hidden at ≤ 400 px, pulsing current node, grayscale and 0.58 opacity for «Скоро», and a pill for the locked tag (`map.rs:1037-1071`). |
| 15 | Unlock rule + footer hint | DONE | `content.rs:672-722`, `map.rs:642-650`. |
| 16 | Section page | DONE | `section.rs:43-306`. «Уроки» is drawn at h3 size instead of h2. |
| 17 | Lesson reader header | PARTIAL | Kicker is correct. The progress bar is static and does not follow scroll (`lesson.rs:146`). The review badge has no tooltip. |
| 18 | Lesson block types, filter, translit | PARTIAL | All 7 block types, level filter and translit are done. Missing: lesson-level `differs` is never rendered (PWA `lesson.js:51`). Hebrew `ru`/`ref` are plain text instead of markdown (`widgets.rs:244-261`). |
| 19 | Lesson sources footer | DONE | `lesson.rs:428-479`. |
| 20 | Mark read | PARTIAL | read, activity, +10 XP and toast are done; next-lesson link done (`lesson.rs:108-236`). Missing: focus on the new button after marking. |
| 21 | Flashcard explainer | DONE | `cards.rs:220,284-288,374-381`. |
| 22 | Deck contexts | DONE | `cards.rs:166-216,753,779`. |
| 23 | Card UI | PARTIAL | No flip animation (instant swap, `cards.rs:627`). No «Уровень l из 5» tooltip on the level dots. |
| 24 | Grade gating + keys | DONE | `cards.rs:442-447,530-548`. |
| 25 | SRS rules | DONE | `srs.rs:16-36`, requeue and repeat pass at `cards.rs:245,264-270`. |
| 26 | Grade notes | PARTIAL | Texts are exact, but the note never fades (PWA fades it after 1.4 s). |
| 27 | Per-grade and deck XP, done card | DONE | `cards.rs:274,301-302,466-527`. |
| 28 | Exam setup | PARTIAL | Mode cards show the letters «Т/Э/Б» instead of icons (`exam.rs:993-1012`). The count chips are ignored for section tests (bug 1). |
| 29 | Question selection | PARTIAL | Weighting, the beit-din 70% mix and retry match (`exam.rs:325-398`). Section mode ignores the count (bug 1). A retry loses its section (bug 6). |
| 30 | Exam runner header | DONE | `exam.rs:1204-1233`. A retry kicker has no section (bug 6). |
| 31 | Question formats | DONE | `exam.rs:1290-1556`. The typed input is not auto-focused. |
| 32 | Typed normalisation, match scoring | DONE | `util.rs:75`, `exam.rs:478-503`. |
| 33 | Feedback box | DONE | `exam.rs:1560-1673`, Enter at `:892-895`. |
| 34 | `record` for every answer | DONE | `exam.rs:466-484`. |
| 35 | Exam quit confirm | DONE | `exam.rs:933-937`. |
| 36 | Exam finish rules | DONE | `exam.rs:566-642` (core). |
| 37 | Exam results | PARTIAL | Score shows as «15.0» (bug 9). A retry shows «На карту» instead of «К разделу» (bug 6). Mistake explanations are not markdown, and «Эталон:»/«Правильно:» are not bold (`exam.rs:1833-1838`). |
| 38 | Timed setup | DONE | `timed.rs:861-1038`. |
| 39 | Timed pool and tiers | DONE | `exam.rs:77-232,671` (core). |
| 40 | Timed top bar + ladder | PARTIAL | Top bar is done. The ladder never shows at the default 1200 px window (bug 10). |
| 41 | Intro, countdown, +5 s, danger | PARTIAL | Timings are correct. The 90 ms option stagger is missing. |
| 42 | Answer input, lock, reveal, skip, timeout | PARTIAL | All work, except that A–D fail on a Russian layout (bug 4) and the 0.35 s reduced-motion lock is missing. |
| 43 | Verdict texts, Next keys | DONE | `timed.rs:785-793,1382-1385`. |
| 44 | Auto-advance + pause | DONE | `timed.rs:692-699,759-766,1390-1430`. |
| 45 | Sound beeps | **MISSING** | The setting is stored but no audio exists. |
| 46 | Timed quit confirm | PARTIAL | Texts and cancel work, but the run keeps ticking under the dialog (bug 5). |
| 47 | Timed finish | DONE | `exam.rs:903-967` (core). |
| 48 | Timed results | PARTIAL | Everything except the ring glow at ≥ 75. |
| 49 | Progress KPIs | PARTIAL | Values are correct. Icons are missing and the layout is always 4 columns. |
| 50 | Goal & streak card | PARTIAL | Complete, except the freeze-day bar label is a hollow circle instead of «❄», and «❄» is missing in the explainer text (`progress.rs:632`). |
| 51 | Badges grid | PARTIAL | All 20 conditions and the art are there. The grid is fixed at 4 columns (spec 2/3/5). The fallback is the first letter of the title, not the emoji or Hebrew letter (`progress.rs:1244`). |
| 52 | Heatmap 28 days | DONE | `progress.rs:552-557,986-1022`. |
| 53 | Per-section rows | DONE | `progress.rs:1024-1111`. |
| 54 | Exam history chart + list | PARTIAL | Point tooltips «DD.MM.YYYY: pct%» are missing. |
| 55 | Weak topics | DONE | `progress.rs:583-617`. Tie order is unstable (bug 12). |
| 56 | Settings | PARTIAL | All rows are present, but the font size has no effect (bug 7). |
| 57 | Daily goal | DONE | `settings.rs:123-129`. «❄» is missing in the note (`settings.rs:336`). |
| 58 | Export | DONE | `settings.rs:130-143`, `storage.rs:431-445`. |
| 59 | Import | PARTIAL | Confirm, 3 validation errors, read error and success toast are done. Missing: ok/bad toast kinds and the «(хранилище недоступно…)» suffix (`settings.rs:399`). |
| 60 | Reset | DONE | `storage.rs:453-459`, `settings.rs:176-181`. |
| 61 | About card | DONE | `settings.rs:408-487`. |
| 62 | Sources page | PARTIAL | Content is done, but the links are not clickable (bug 11). |
| 63 | Streak / freeze logic | DONE | `game.rs:90-180`, `app.rs:134`, migration at `storage.rs:369-380`. |
| 64 | Level filtering | DONE | `content.rs:569-640`. |
| 65 | Palettes T.1–T.3 | PARTIAL | Dark `info_soft` is 0.14, not 0.12 (`theme.rs:99`). Radial glows are approximated or missing. |
| 66 | Fonts + root size steps | PARTIAL | Noto Serif is used instead of PT Serif. The 15/17/19/21.5 steps are never applied (bug 7). |
| 67 | Radii, spacing, buttons | PARTIAL | Primary and tq-go buttons have radius 0 (bug 13). tq-go has 2 gradient stops, not 3. Disabled is not .45 opacity. Minimum heights are mostly not applied. |
| 68 | Hebrew RTL | PARTIAL | Blocks are not centred (`widgets.rs:224-232`). Inline runs are separate spans but not bidi-isolated. |
| 69 | differs notes | DONE | `lesson.rs:372-416`, `widgets.rs:371-410`. |
| 70 | Image fallbacks | PARTIAL | Welcome star, level rings and section letter are done. Badge fallback is the title letter. Fallback emblems are not centred (bug 8). |
| 71 | Reduced motion | **MISSING** | No setting. `LOCK_MS` is fixed at 1100 (`timed.rs:38`). |

**Christian framing:** none in UI strings. A case-insensitive search of `src/` for библи*, ветх*/нов* завет, христ*, евангел*, церк* and bible found nothing. Content hits: `content/sections/01-osnovy.json:77` «Библейские образы» is a Steinsaltz book title in a source list, so it can stay. Other «христиан…» hits in content are about converts' background, not framing.

## Bugs

1. **bug**: section tests ignore the chosen count. The run always has up to 20 questions, and «будет задано N» is wrong. `src/ui/screens/exam.rs:382` builds `ExamSpec::Section(id)` without a count, and `build_run` (`exam.rs:~644`) passes `count: None`, so the core falls back to 20.
   - Fix: change the variant to `ExamSpec::Section { id, count }` (in `app.rs`).
   - Return `ExamSpec::Section { id: self.section.clone().unwrap_or_default(), count: self.count }` from `spec()`.
   - Pass `Some(count)` in `build_run`.
   - Update `parse_route` in `src/main.rs:258`.
2. **bug**: `src/main.rs:217-221` returns `base` with no toasts and `stack![base, toasts]` with toasts. Changing the root widget type rebuilds the tree, so the scroll position and input focus reset every time a toast appears or expires. For example, marking a lesson read at the bottom jumps the page to the top, and does it again 4.2 s later. Also, nothing scrolls to the top on navigation.
   - Fix: always return `stack![base, self.toasts()]`.
   - Give the main `scrollable` an `Id`, and in `go()` return `scrollable::snap_to(id, RelativeOffset::START)`, except for ExamRun and TimedRun.
3. **bug**: `src/ui/app.rs:172,182,201` set each toast's `until` when it is pushed, but only `toasts[0]` is shown (`main.rs:164`), and `Tick` drops every expired toast. Queued toasts count down while hidden. A goal toast plus a badge toast means the badge shows for about 1 s; two toasts pushed together means the second never shows.
   - Fix: in the push helpers, set `until = max(last.until, now) + dur`. Or store `dur` and set `toasts[0].until = now + dur` when the front toast is removed.
4. **bug**: `src/ui/screens/timed.rs:795-806` matches A–D on the logical `Key::Character`. On a Russian layout the A key sends «ф», so A–D do nothing; only digits work. The PWA uses `e.code`.
   - Fix: also pass `physical_key` from `KeyPressed` (`timed.rs:845`) in `Msg::Key`.
   - Map `key::Physical::Code(Code::KeyA..KeyD | Digit1..Digit4 | Numpad1..Numpad4)` to 0..3 first, then fall back to the character match.
5. **bug**: `src/ui/screens/timed.rs:734` `Msg::Tick` ignores `run.confirm_quit`. While «Выйти из экзамена?» is open, the countdown runs, a timeout records a miss, and auto-advance calls `Next` (`:764`). On the last question that calls `finish()`, which logs the run and awards XP, although the dialog says the result will not count. The dialog then vanishes into results.
   - Fix: at the top of `Tick`, `if run.confirm_quit { return Action::None; }`.
   - Record `Instant::now()` in `Msg::Quit`. On `QuitCancel`, add the elapsed time to `timer_end` and `phase_deadline`.
6. **minor**: `src/ui/screens/exam.rs:~653,866-870` builds `ExamSpec::Retry` with no section. A retry of a section test shows «На карту» instead of «К разделу», the kicker has no section, and the history entry gets `section_id = null`, so it does not count toward the section best.
   - Fix: add `section_id: Option<String>` to `Retry`, fill it from `r.res.section_id` in `Msg::Retry`, and pass it to `build_run`.
7. **minor**: `font_step` is saved (`settings.rs:89-93`) but never read, so «A− / A / A+ / A++» do nothing.
   - Fix: in `main()` add `.scale_factor(|app: &App| [15.0, 17.0, 19.0, 21.5][app.ctx.state.settings.font_step.clamp(0, 3) as usize] / 17.0)`.
8. **minor**: ring and emblem labels sit in `container(..).width(N).height(N)` with no alignment, which in iced defaults to top-left. Affected: `map.rs:731` (goal ring), `map.rs:1110-1117` (path ring %), `map.rs:1208-1210` (node number/check/lock), `map.rs:497-499` (emblem fallback) and `section.rs:62-68` (section ring).
   - Fix: add `.center_x(N).center_y(N)` (or `.align_x(Center).align_y(Center)`), as `section.rs:239-240` already does.
9. **minor**: `src/ui/screens/exam.rs:1720` uses `format!("{:.1}", res.score)`, which prints «15.0» where the PWA shows «15» or «14.5».
   - Fix: `let s = (res.score * 10.0).round() / 10.0; if s.fract() == 0.0 { format!("{}", s as i64) } else { format!("{s}") }`.
10. **minor**: `src/ui/screens/timed.rs:1139` shows the ladder only when the inner width is ≥ 860. With a 230 px sidebar and the 980 px content cap (`main.rs:201`), the default 1200 px window gives about 850 px, so the ladder never appears.
    - Fix: lower the threshold to about 800, or test the window width (≥ 1024). Raise `max_width` to 1060 for TimedRun.
11. **minor**: `src/ui/screens/sources.rs:93-114` `source_row` draws the external-link icon but has no press handler, so links do nothing.
    - Fix: wrap the row in `button(..).on_press(Msg::OpenUrl(url))` for http(s) URLs.
    - Move `open_url` from `lesson.rs:514` into a shared module.
12. **minor**: `src/ui/screens/progress.rs:603-617` sorts weak topics by the rounded integer percent, collected from a `HashMap`. Ties come out in random order, so the top-5 cut can change between visits.
    - Fix: sort by the raw `miss as f64 / n as f64` descending, then by lesson id. Round only for display.
13. **minor**: `src/ui/theme.rs:332-339` (`button_primary`) and `button_tq_go` use `Border::default()`, so these buttons have square corners.
    - Fix: `border: Border { radius: radius::BUTTON.into(), ..Default::default() }`.
14. **minor**: `src/ui/screens/settings.rs:149-151` reads the import with `read_to_string`. A non-UTF-8 file shows «Не удалось прочитать файл» instead of «Файл не является корректным JSON.».
    - Fix: use `std::fs::read`, then `String::from_utf8`; on a UTF-8 error show the JSON error.
15. **minor**: `src/ui/screens/level.rs:116` stacks the confirm inside the scrollable page. It covers only the page and scrolls with it, the sidebar stays clickable, and Esc does nothing.
    - Fix: render it as a window-level overlay and add an Esc listener that sends `ConfirmNo`.

Verified clean: no double XP (lesson mark-read button is replaced once read; card grades unflip; exam and timed `finish` run once; typed Enter is captured by the input). No panics on empty decks or pools (all guarded). State is saved after every grade, answer and finish. Timed subscriptions stop outside `Phase::Run`.
