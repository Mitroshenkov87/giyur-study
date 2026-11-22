Read-only analysis task. The PWA "Giyur Study" source is at /workspace/giyur-study (index.html, css/app.css, js/*.js, js/ui/*.js, docs/APP.md). It is being rewritten as a native Rust desktop app (iced 0.14) in /workspace/giyur-study-rs. Write ONE file: /workspace/giyur-study-rs/docs/PARITY.md — an exhaustive, implementation-ready feature inventory for the UI rewrite. Do not modify any other file.

Include, per screen/route (welcome, level choice, map/home, section page, lesson reader, flashcards deck + first-run explainer, section test, general exam, mock beit-din, mistakes retry, timed «Кто хочет стать миллионером» intro/run/results, progress (stats, badges grid), settings (daily goal, timer, theme, change movement, export/import, reset), any dialogs/toasts/banners):
1. Purpose and layout (what is shown, in what order; which images from assets/img/ are used where).
2. Every user-visible Russian string, quoted exactly (buttons, headings, hints, empty states, confirmations, result texts, «Скоро» etc.).
3. Interactions and state changes (which core function it calls: srs grade, exam record, game XP, unlock), navigation targets, keyboard shortcuts if any.
4. Edge cases (locked sections, no due cards, timer expiry, lifelines if any).
Then a "Theme" section: the dark navy-and-gold palette (exact hex/rgba from css :root and the light theme if present), font roles (serif headings, sans body, Hebrew), radii, spacing, button styles, Hebrew RTL blocks (right-aligned, gold), 'differs' notes styling.
Then a numbered "Parity checklist" (one line per feature) that a reviewer can tick off.
Be precise and concise; no prose padding. Jewish perspective only in any wording you add (never «Библия»/«Ветхий Завет»).
