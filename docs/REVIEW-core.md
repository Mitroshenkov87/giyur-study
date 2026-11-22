# Core review: Rust vs PWA (util, storage, srs, content, game, exam, timed)

Read-only review against `/workspace/giyur-study/js/{util,storage,srs,content,game,exam}.js` and `js/ui/timed.js`.

## Findings

1. **bug**, `src/storage.rs:365-369` + `src/ui/app.rs:109-123`. If reading the file fails for any reason other than "not found" (invalid UTF-8, permission denied, file locked by AV/sync tool), `load` returns `State::default()` and makes no `.bak`. `Ctx::load` then calls `ctx.save()` right away and overwrites the real progress file with an empty state. The PWA cannot hit this: a failed `localStorage` read sets `available=false`, and `save()` then does nothing.
   Fix: in `load`, match `Err(e) if e.kind() == NotFound => default`. For any other `Err`, copy to `<path>.json.bak` (as the corrupt-JSON branch does) and return default. Better still, also return a flag that stops `Ctx::load` from saving on start-up in that case.

2. **bug**, `src/storage.rs:316-317` (with `fill`, `:250`). Strict serde typing makes the whole import fail where JS `fill()` repairs field by field. The PWA replaces `null` or a wrongly typed top-level field with its default (`storage.js:172-176`), and an odd `settings.level` falls back to orthodox. Rust rejects the whole file in these cases: `"settings": null`, `"xp": null`, `"onboarded": null`, `"exams": {}`, `"examHistory": [[1,2]]`, `"level": ""`, or a float in an `i64` field. On start-up that means `.bak` plus a reset. `import_value` only removes non-object values for the 10 map keys.
   Fix: in `import_value`, before deserialising, remove every top-level key whose value is `null` or whose JSON type differs from the default. Do the same for each `settings` key. Make `settings.level` tolerant: `#[serde(default, deserialize_with = …)]` that maps unknown strings to `None`. Drop non-array `exams`/`examHistory`, and drop `examHistory` runs that are not string arrays (as `recentTimedIds` does).

3. **bug**, `src/ui/app.rs:120-123` (game start-up). JS `G.init` runs `applyFreezes()`, then `checkBadges(true)`. Rust only calls `apply_freezes`. So an imported or old save gets no badges and keeps a stale `counters.bestStreak` until the next XP award, and then those badges celebrate loudly instead of unlocking quietly.
   Fix: after `apply_freezes`, call `game::check_badges(&ctx.lib, &mut ctx.state, today)` and throw away the result (quiet), then save. Do the same after a successful «Импорт».

4. **minor**, `src/storage.rs:381-389`. The save is not durable: the temp file is never fsynced before `rename`. After a power loss, `progress.json` can end up empty or truncated, which then goes to `.bak` and a reset. Also, `unwrap_or_default()` would write an empty file if serialisation ever failed.
   Fix: `let mut f = File::create(&tmp)?; f.write_all(data.as_bytes())?; f.sync_all()?; drop(f); rename(&tmp, path)?;`. Use `serde_json::to_string_pretty(state).map_err(io::Error::other)?` instead of `unwrap_or_default()`.

5. **minor**, `src/storage.rs:313`. A UTF-8 BOM (for example a progress or export file re-saved in Windows Notepad) makes `serde_json` fail with «Файл не является корректным JSON.». On start-up that means `.bak` plus a reset. JS `JSON.parse` of a `FileReader` text has no BOM in practice.
   Fix: `let text = text.strip_prefix('\u{feff}').unwrap_or(text);` before parsing.

6. **minor**, `src/storage.rs:295`. The legacy migration condition differs. JS migrates only when `s.xp === undefined` (`storage.js:168`). Rust also migrates when `xp` is `null`.
   Fix: `let xp_missing = !obj.contains_key("xp");`. With item 2 in place, null-stripping happens after this check.

7. **minor**, `src/storage.rs:145-205`. Unknown top-level and settings keys are silently dropped on load/save. JS keeps them through `fill()`, so a PWA file → native → PWA round-trip loses any field the Rust struct lacks.
   Fix: add `#[serde(flatten)] pub extra: serde_json::Map<String, serde_json::Value>` to `State` and to `Settings`.

8. **minor**, `src/content.rs:459-463`. Sections that are bundled but missing from the manifest are appended in `HashMap` order, which is random on every run. That randomises `index`, and with it the unlock chain and badge order. JS appends them in bundle order (`content.js:47`).
   Fix: keep the order of the input `sections` slice, e.g. collect ids from the slice before building `by_id`, and push the missing ones in that order.

9. **minor**, `src/content.rs:153-171`. `Lesson` has no `differs` field, but `js/ui/lesson.js:51` renders lesson-level `L.differs`. No lesson in the current content has it (all 15 `differs` at that depth are on questions), so this is latent.
   Fix: add `#[serde(default)] pub differs: Vec<Differs>` to `Lesson` and render it after the blocks.

10. **minor**, `src/content.rs:476-487`. The title/subtitle/icon fallback uses `Option`, so an empty string `""` in section JSON wins. JS `(data && data.title) || d.title` treats `""` as missing.
    Fix: `.and_then(|d| d.title.clone()).filter(|s| !s.is_empty())`, and the same for subtitle and icon.

11. **minor**, `src/exam.rs:485-491`. A `match` question with empty `pairs` scores 0 in Rust; JS scores 1 (`good === q.pairs.length` → `0 === 0`). No such content exists, so this is only a parity note.
    Fix: either keep it (safer) or mirror JS with `if good == pairs.len() { 1.0 }`. Better: reject empty `pairs` when loading content.

## Areas with no difference

- **SRS intervals 1/3/7/14/30, level cap, `m` set only on «Вспомнил» at level 5, miss → l=0, m=false, due tomorrow, touch:** ok.
- **Unlock rules (previous *available* section complete = all visible lessons read + best ≥ 80; unlockAll; first available is open):** ok.
- **Level filtering/inheritance (null level → orthodox; cards and blocks inherit lesson minLevel; lessons and questions don't inherit; question hidden only if its lessonId exists and that lesson is hidden):** ok.
- **Timed ramp (5/8/7 via round(0.25n)/round(0.4n), neighbour borrowing, easy → medium → hard order, weighted within tier, avoid ids of the last 3 runs with whole-tier fallback, +5 s on easy, base 20|30, retry shuffled, history capped at 3, perfectTimed only when total ≥ 20, XP 2/right):** ok.
- **Weighting `max(0.25, 1 + 2·miss − 0.5·min(streak,3))`, Efraimidis–Spirakis with w ≥ 0.01:** ok.
- **Answer checking/normalisation (lower-case, ё→е, U+0591–U+05C7 removed, non-letter/digit runs → one space, trim; typed answer + accept list; partial match score; miss +0.5 when score > 0):** ok.
- **XP values, goal fallback 20, goal/celebration once per day, streak (today or yesterday), freeze every 7 days, max 2, `freezeAwardDay`, applyFreezes 2..60-day lookback and gap ≤ freezes, 12 + 8 badge conditions, reminders text:** ok.
- **Export/import field names (camelCase, `exportedAt` stripped, schema > 1 rejected, the 10 object-field sanitisation, clamps for fontStep/dailyGoal/freezes/xpTotal), reset keeps theme + fontStep:** ok apart from items 2, 5, 6 and 7.
- **Christian framing («Библия», «Ветхий Завет», «библейский»):** ok. None in `src/` or `content/`. The only Christian mentions are factual: `content/sections/01-osnovy.json:1940` («при христианском делении», a book count) and the beit-din answers about growing up in a Christian family (`08-beit-din.json:600,727,1898-1899`).
