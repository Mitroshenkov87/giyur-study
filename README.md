# Шорашим (Shorashim)

Основы иудаизма и подготовка к гиюру. Учебное приложение на русском языке.

Foundations of Judaism and preparation for giyur. A Russian-language study app.

**Скачать / Download:** [Releases](../../releases)

**Лицензия / License:** Copyright 2026 Aleksandr Mitroshenkov. Apache License 2.0 — [`LICENSE`](LICENSE), [`NOTICE`](NOTICE).

---

## Технические подробности / Technical details

**Запуск / Run.** Скачайте `Shorashim-vX.Y.Z-windows-x64.zip` со страницы [Releases](../../releases) и запустите `Shorashim.exe`; установка не нужна. В архиве есть папка `music` с фоновой музыкой. Один `.exe` без этой папки тоже запускается, но без музыки. В «Настройках» программу можно установить в систему — папка `music` копируется вместе с ней. / Download the portable `.zip` from Releases and run `Shorashim.exe`; no installer. The zip includes a `music` folder. A lone `.exe` still runs, but without background music.

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
