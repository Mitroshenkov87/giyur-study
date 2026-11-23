# Automation: driving Shorashim with commands

Shorashim can be driven by commands instead of mouse clicks, for testing
bots, agents and CI. Every command goes through the app's normal `update`
code: `press` publishes exactly the message a mouse click on that control
would publish, `navigate` is the sidebar's route change, `answer` sends the
quiz messages. Nothing here is on in normal use.

There are four ways in:

| Front end | Window | Use |
|---|---|---|
| `shorashim --control` + `shorashim ctl …` | yes | drive the real app from a shell |
| `shorashim mcp` | yes (attaches) | MCP tools for agents, talking to the running app |
| `shorashim mcp --headless` | no | MCP tools on an in-process app without a window |
| `shorashim --headless --script file.jsonl` | no | fast scripted tests (CI) |

All of them speak the same JSON commands.

## Quick start

```sh
# 1. Start the app with the control channel on
shorashim --control            # or: SHORASHIM_CONTROL=1 shorashim
#    (or let the CLI start it and wait until it answers)
shorashim ctl launch

# 2. Look at the screen: texts, clickable controls with ids, inputs
shorashim ctl state
shorashim ctl state texts=false

# 3. Act
shorashim ctl press Начать                       # by visible label
shorashim ctl press contains=Ортодоксальный      # by substring
shorashim ctl navigate section:osnovy
shorashim ctl press id='Section(Go(Lesson("osnovy-1")))'
shorashim ctl press id='Lesson(MarkRead)'
shorashim ctl navigate cards:osnovy-1
shorashim ctl answer knew=true                   # flips, then «Вспомнил»
shorashim ctl navigate examrun:section:osnovy:10
shorashim ctl answer choice=0 next=true          # first option as shown
shorashim ctl navigate calendar:2026-12-05
shorashim ctl navigate Карточки                  # a sidebar label (old ones like «Повторение» work too)
shorashim ctl set key=theme value=leather        # «Кожа»; "parchment" is accepted as its old name
shorashim ctl screenshot /tmp/shot.webp          # the real window, WebP
shorashim ctl quit

# Raw JSON, or many commands from stdin (one reply line each)
shorashim ctl --json '{"cmd":"navigate","to":"progress"}'
printf '%s\n' '{"cmd":"navigate","to":"map"}' '{"cmd":"state","texts":false}' | shorashim ctl -
```

`ctl` arguments are `key=value` (values are parsed as JSON when they can be:
`true`, `3`, `"text"`); a bare first word is the command's main argument
(`navigate <route>`, `press <label>`, `type <text>`, `screenshot <path>`).
The exit code is 0 when the reply has `"ok": true`.

## Commands

Requests are JSON objects with `cmd`; replies always have `ok` and, on
failure, `error`. Action commands reply with a short summary (`screen`,
`route`, `dialog`, `toasts`); add `"state": true` to get the full state.

| cmd | arguments | what it does |
|---|---|---|
| `help` | | the command list |
| `screens` | | sidebar items, route syntax, sections and lessons with ids |
| `state` | `texts` (default true) | screen, route, visible `texts`, `controls` (`id`, `label`), `inputs`, `choices` (drop-downs), `progress`, `settings`, `toasts` |
| `navigate` | `to` | go to a route (below) or a sidebar label (current or old, case-insensitive; see below) |
| `back` | | «Назад»: previous route, or the map |
| `press` | `id` \| `label` \| `contains`, `nth` | press a control from `state` |
| `answer` | `choice` \| `tf` \| `text` \| `knew` \| `self_grade` \| `auto`, `next` | answer the current question |
| `type` | `text`, `input` (index), `append`, `submit` | type into a field (replaces its text unless `append`) |
| `select` | `choice` (index) or `target` (shown text), `option`, `nth` | pick from a drop-down |
| `set` | `key`, `value` | `theme` (`dark` / `leather` = «Кожа»; `parchment`, its old name, is stored as `leather`), `font_step` (0–3), `translit`, `unlock_all`, `timed_sound`, `timed_auto`, `timed_seconds`, `daily_goal` |
| `wait` | `ms` (≤ 60000) | let time pass; timers run (the timed quiz reveals options after a pause) |
| `screenshot` | `path`, `full_page`, `width`, `height` | save a WebP (the extension is forced to `.webp`) |
| `scroll` | `to` = top / bottom / 0..1 | windowed app only |
| `quit` | | close the app (headless: end the session) |

**Routes:** `welcome`, `level`, `map`, `calendar`, `calendar:today|month|settings`,
`calendar:YYYY-MM-DD`, `section:<id>`, `lesson:<id>`, `cards:<lessonId>`,
`deck:<sectionId>`, `cards:review`, `exam`, `exam:<sectionId>`,
`examrun:section:<id>[:n]`, `examrun:general[:n]`, `examrun:beitdin[:n]`,
`timed`, `timedrun`, `progress`, `settings`, `sources`. Until the first-run
level choice is made, every route goes to the welcome screen, as in the app.

**Sidebar labels.** `navigate` takes the sidebar names from the app's own
list (`src/ui/nav.rs`), so commands and the sidebar never disagree. Labels
from before v0.5 still work as aliases. `screens` returns the list with
subtitles and tooltips.

| Label | Old label | Route |
|---|---|---|
| Уроки | Карта | `map` |
| Календарь | | `calendar` |
| Карточки | Повторение | `cards:review` |
| Викторина | Экзамен | `timed` |
| Тесты | | `exam` |
| Успехи | Прогресс | `progress` |
| Настройки | | `settings` |
| Источники | | `sources` |

**Control ids** are the Debug form of the message a click publishes, e.g.
`Map(Go(Section("osnovy")))`, `Exam(PickMc(2))`, `Calendar(NextDay)`. They are
stable across runs for the same content; very long ones are shortened to a
prefix plus a hash. `label` joins the visible texts inside the control.

**`answer`:**
- `choice`: 0-based, in the order the options are shown (the app shuffles
  them; the control id carries the option's own index). Works for
  multiple-choice and true/false questions in tests, and for the timed quiz.
- `tf`: true/false question. `text`: typed answer (typed and submitted).
- `knew`: flashcard; flips the card first when needed, then «Вспомнил» /
  «Не вспомнил».
- `self_grade`: open (beit-din) questions; reveals the model answer and grades.
- `auto`: any answer to whatever question is shown (smoke tests).
- `next`: then press «Далее» / «Дальше».

**Inputs:** `state.inputs[i].text` is the field's value, or its placeholder
when empty.

**Links, file dialogs and the installer.** Controls that open a browser or a
system file dialog (`OpenUrl`, `Export`, `ImportPick`), or install, update or
remove the app on the system (`Settings(Install)`, `Settings(AskUninstall)`),
are refused unless
`allow_external=true` is given, and are never pressed headless. A native
file dialog blocks the app until a person closes it.

## Headless scripts

```sh
shorashim --headless --script tests/scripts/02-lesson.jsonl --today 2026-11-22
shorashim --headless < commands.jsonl          # stdin, keeps going on errors
```

- One JSON command per line; blank lines and `#` / `//` comments are skipped.
- One reply line per command on stdout. With `--script`, the run stops at the
  first failure (`--keep-going` to continue); the exit code is 1 if any
  command failed.
- `--data FILE` uses that progress file; by default a fresh throw-away one is
  used, never your own progress. `--today YYYY-MM-DD` fixes the date.
- `expect` makes a line an assertion, checked after the command:
  `{"screen": "lesson", "route": "lesson:", "text": "гиюр", "control": "Exam(Start)", "min_xp": 10, "reply": {"dialog": true}}`
  (`route` is a prefix, `text` / `control` are case-insensitive substrings, `reply` objects match as subsets).
- Headless screenshots render the page column (no sidebar) with the
  software renderer.
- Screenshot hooks for the 3D parts (environment variables, read on
  start): `SHORASHIM_COLLECTION_OPEN=<key>` opens that unlocked object of
  the «Коллекция предметов» on «Успехи» in the live viewer (keys are the
  asset names, e.g. `kiddush-cup`); `SHORASHIM_LADDER_DEMO=mid` with
  `SHORASHIM_LADDER_CLIMB=<steps>` (e.g. `6.5`) freezes the lamp of the 3D
  «Маалот» stair mid-climb. Live 3D needs the wgpu renderer (a real window);
  the software renderer shows the stills.

**Start-up splash.** The windowed app opens with a 5-second splash (the
menorah picture and «Шорашим», a click or any key skips it). It is never
shown with `--control` / `SHORASHIM_CONTROL=1`, with the
`SHORASHIM_START_ROUTE` hook, with `--no-splash` or `SHORASHIM_NO_SPLASH=1`,
and headless runs have no window at all, so tests and the night test are not
delayed. `SHORASHIM_SPLASH=1` shows it anyway; `SHORASHIM_SPLASH_HOLD=1`
(or `0.5` for mid-fade) freezes it for a screenshot.

CI runs `tests/automation.rs`, which plays `tests/scripts/*.jsonl` (first
run and navigation, a lesson, flashcards, a section test, the timed quiz («Викторина») and
a mock beit-din, the calendar, settings, the «Тесты» mode tiles) and checks the MCP server.

## MCP

```jsonc
// e.g. an MCP client config
{ "mcpServers": {
    "shorashim":          { "command": "shorashim", "args": ["mcp"] },   // Windows: "shorashim-ctl.exe"
    "shorashim-headless": { "command": "shorashim", "args": ["mcp", "--headless", "--today", "2026-11-22"] }
} }
```

JSON-RPC 2.0 over stdio, one message per line (`initialize`, `tools/list`,
`tools/call`, `ping`). Each command above is a tool with the same arguments
(`state`, `screens`, `navigate`, `back`, `press`, `answer`, `type`, `select`,
`set`, `wait`, `screenshot`, `quit`; attached mode adds `scroll` and
`launch`). `screenshot` with `inline: true` also returns the WebP image.
Attached mode reconnects when the app restarts; `launch` starts it.

## Security model

- **Off by default.** The control channel starts only with `--control` or
  `SHORASHIM_CONTROL=1`. A normal launch opens no port and writes no file.
- **Loopback only.** It listens on `127.0.0.1` (random port, or
  `SHORASHIM_CONTROL_PORT`); connections from other addresses are dropped.
  The client refuses a non-loopback address in the control file.
- **Token.** A random 256-bit token (OS randomness) is written with the port
  to `control.json` next to the progress file (`%APPDATA%\Shorashim\` on
  Windows, which only the user can read; mode 0600 on Linux). If
  `SHORASHIM_DATA` points elsewhere on Windows, the file gets that folder's
  permissions. Every request must carry it; a bad
  token gets one error after a short delay and the connection is closed.
  The token is new for each start. `quit` removes the file.
- **No browser access.** Requests must be newline-delimited JSON objects; an
  HTTP request (e.g. from a web page) is not JSON and is dropped.
- **Same powers as the user's clicks.** Commands can only do what the UI can
  do; links, file dialogs and the installer need `allow_external`. Headless runs use a
  throw-away progress file unless `--data` is given.

## Windows notes

The release `.exe` is a GUI program, so shells don't wait for it. Use the
console companion **`shorashim-ctl.exe`** (built next to it, shipped in the CI
artifact) for commands: it waits, prints normally and sets `%ERRORLEVEL%` /
`$LASTEXITCODE`. A bare command means `ctl`:

```powershell
.\shorashim-ctl.exe launch                 # starts the app found next to it
.\shorashim-ctl.exe state
.\shorashim-ctl.exe press label=Начать
.\shorashim-ctl.exe mcp                    # MCP server
.\shorashim-ctl.exe --headless --script s.jsonl
```

The main `.exe` also accepts `ctl`, `mcp` and `--headless`: it attaches to the
console of the shell that started it, or uses the pipes it was given (MCP
clients work fine). In an interactive `cmd.exe` the prompt may come back
before the output and the exit code is lost; pipe it (`| more`, `| Out-String`)
or use `shorashim-ctl.exe`.

## How it works (for developers)

- `src/automation/probe.rs` lays out the current screen's real `view()` with
  the tiny-skia renderer, headless, inside the same padded scrollable column
  as the window. It lists texts (`Operation::text`), inputs, and drop-downs,
  and finds controls by simulating a click on each text and recording the
  messages the widgets publish.
- `iced::widget::rich_text` and `pick_list` do not report themselves to
  widget operations, so `automation::reported` / `reported_choice` wrap them
  (lesson paragraphs, exam lines, the section and match drop-downs).
- `src/automation/mod.rs`: the commands (`execute`) against a `Host` (the
  windowed `App` in `src/main.rs`, or the headless `Session`).
- `server.rs` (control channel as an iced subscription), `client.rs`,
  `cli.rs`, `mcp.rs`, `session.rs`.
