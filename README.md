# Шорашим (Shorashim)

Основы иудаизма и подготовка к гиюру. Учебное приложение на русском и английском языках: язык выбирается по Windows при первом запуске и меняется в «Настройках».

Foundations of Judaism and preparation for giyur. A study app in Russian and English: the language follows Windows on first launch and can be changed in Settings.

**Скачать / Download:** [Releases](../../releases)

**Лицензия / License:** Copyright 2026 Aleksandr Mitroshenkov. Apache License 2.0 — [`LICENSE`](LICENSE), [`NOTICE`](NOTICE).

---

## Технические подробности / Technical details

**Установка / Install.** Скачайте `Shorashim-Setup.exe` со страницы [Releases](../../releases) и запустите: он установит программу для текущего пользователя (без прав администратора) вместе с папкой `content` (уроки, карточки, вопросы, молитвы, календарь) и музыкой, создаст ярлыки и запустит «Шорашим». Прогресс хранится отдельно (`%APPDATA%\Shorashim`) и при переустановке не меняется. / Download `Shorashim-Setup.exe` from Releases and run it: a per-user install of the app, its `content` folder and music. Course content is plain JSON read at run time — see [docs/CONTENT-UPDATES.md](docs/CONTENT-UPDATES.md).

**Прогресс / Progress.** `%APPDATA%\Shorashim\progress.json`.

**Музыка / Music.** Тихие инструментальные мелодии (по умолчанию выключены; включаются в «Настройках»). Треки созданы в Grok Imagine, лежат рядом с программой (`music/`, OGG Vorbis) и описаны в `music/manifest.json` — новый файл можно просто положить в папку и дописать в манифест.

**Сборка / Build from source.** Rust stable (<https://rustup.rs>):

```sh
cargo run --release
cargo test
# Windows .exe from Linux (needs mingw-w64):
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu
```

**Автоматизация / Automation.** Управление командами для тестов и ботов: `shorashim --control`, `shorashim ctl …`, `shorashim mcp`, `shorashim --headless` — [`docs/AUTOMATION.md`](docs/AUTOMATION.md). / Command control for tests and agents (off by default, localhost only).

Материалы требуют проверки раввином ([`docs/RABBI-REVIEW.md`](docs/RABBI-REVIEW.md)). / Content must be checked with your rabbi. Шрифты / Fonts: Noto (SIL OFL 1.1), DejaVu Sans (DejaVu Fonts license).
