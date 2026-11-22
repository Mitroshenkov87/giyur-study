# Шорашим (Shorashim)

Основы иудаизма и подготовка к гиюру. Учебное приложение на русском языке.

Foundations of Judaism and preparation for giyur. A Russian-language study app.

**Скачать / Download:** [Releases](../../releases)

**Лицензия / License:** Copyright 2026 Aleksandr Mitroshenkov. Apache License 2.0 — [`LICENSE`](LICENSE), [`NOTICE`](NOTICE).

---

## Технические подробности / Technical details

**Запуск / Run.** Скачайте `Shorashim-vX.Y.Z-windows-x64.exe` со страницы [Releases](../../releases) и запустите; установка не нужна. / Download the `.exe` from Releases and run it; no installer.

**Прогресс / Progress.** `%APPDATA%\Shorashim\progress.json`.

**Сборка / Build from source.** Rust stable (<https://rustup.rs>):

```sh
cargo run --release
cargo test
# Windows .exe from Linux (needs mingw-w64):
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu
```

Материалы требуют проверки раввином ([`docs/RABBI-REVIEW.md`](docs/RABBI-REVIEW.md)). / Content must be checked with your rabbi. Шрифты / Fonts: Noto (SIL OFL 1.1), DejaVu Sans (DejaVu Fonts license).
