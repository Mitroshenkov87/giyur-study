# Giyur Study — нативное приложение (Rust)

**Русский** · [English](#english)

Офлайн-учебник на русском языке для подготовки к гиюру и собеседованию в бейт-дине. Это полностью нативная версия на Rust (без браузера и webview) приложения Giyur Study: 8 разделов с уроками по уровням, карточки «Вспомнил / Не вспомнил» с интервальным повторением (1 → 3 → 7 → 14 → 30 дней), тесты по разделам, общий экзамен, пробное собеседование в бейт-дине и экзамен на время из 20 вопросов — от лёгких к трудным, каждый раз новый.

Три течения — реформистское, консервативное (Масорти) и ортодоксальное — устроены как вложенные уровни строгости: каждый следующий включает всё из предыдущего. Есть XP, дневная цель, серия дней с «заморозкой», значки и карта пути.

> **Важно.** Материалы требуют проверки раввином. Программы гиюра различаются в зависимости от течения и конкретного бейт-дина — сверяйте всё со своим раввином. Список вопросов для проверки: [`docs/RABBI-REVIEW.md`](docs/RABBI-REVIEW.md).

## Скачать
Готовый `.exe` для Windows (x64) — на странице [Releases](../../releases). Установка не нужна: скачайте и запустите. Прогресс хранится в `%APPDATA%\GiyurStudy\progress.json`.

## Сборка
Нужен Rust (stable) — <https://rustup.rs>.

```sh
cargo run --release          # запустить
cargo test                   # тесты
```

Windows-сборка из Linux (MinGW):
```sh
rustup target add x86_64-pc-windows-gnu
sudo apt install gcc-mingw-w64-x86-64
cargo build --release --target x86_64-pc-windows-gnu
# → target/x86_64-pc-windows-gnu/release/giyur-study.exe
```
На Linux нужны обычные библиотеки окна: `libxkbcommon`, `libwayland`/`libx11`, драйвер Vulkan или OpenGL (если GPU нет, включается программная отрисовка).

Материалы (`content/`) встраиваются в программу при сборке; картинки — только WebP (`assets/img/`); значок `.ico` для Windows делается из WebP во время сборки.

## Лицензия
Apache License 2.0 — см. [`LICENSE`](LICENSE) и [`NOTICE`](NOTICE). Шрифты Noto — SIL Open Font License 1.1.

---

## English

**Giyur Study** is an offline, Russian-language study app for people preparing for giyur (conversion to Judaism) and the beit din interview. This repository is the full native Rust rewrite (no browser, no webview) of the Giyur Study PWA. The UI and content are in Russian.

Features: 8 sections of leveled lessons; flashcards with spaced repetition (1 → 3 → 7 → 14 → 30 days); section tests, a general exam, a mock beit-din interview and a timed 20-question exam that ramps from easy to hard and changes every run; three movements (Reform ⊂ Conservative/Masorti ⊂ Orthodox) as nested levels of strictness; XP, a daily goal, a streak with freezes, badges and a path map; a navy-and-gold dark theme and a light «Parchment» theme.

> **Note.** The content must be checked with your rabbi. Conversion programs differ by movement and by beit din.

### Download
Get the Windows x64 `.exe` from [Releases](../../releases). No installer; progress is stored in `%APPDATA%\GiyurStudy\progress.json`.

### Build
```sh
cargo run --release
cargo test
cargo build --release --target x86_64-pc-windows-gnu   # cross-compile from Linux (needs mingw-w64)
```

### Why iced
GUI_CHOICE_PLACEHOLDER

### License
Apache License 2.0 — see [`LICENSE`](LICENSE) and [`NOTICE`](NOTICE). Noto fonts are under the SIL Open Font License 1.1.
