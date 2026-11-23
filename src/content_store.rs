//! Course content read at run time from `content/<lang>/` beside the exe.
//!
//! Nothing of the course is built into the program: lessons, cards, quiz
//! questions, the interview trainer, prayers, quotes and calendar texts are
//! plain JSON files the installer puts next to `shorashim.exe`:
//!
//! ```text
//! <install>\shorashim.exe
//! <install>\content\ru\manifest.json
//! <install>\content\ru\sections\01-osnovy.json …
//! <install>\content\ru\interview.json, prayers.json, quotes.json
//! <install>\content\ru\calendar\cities.json, days.json, parashot.json
//! ```
//!
//! A later English version adds `content\en\` with the same file names.
//! Fixing a card is editing its JSON file and restarting the app (or
//! `shorashim-ctl reload-content`); see docs/CONTENT-UPDATES.md.
//!
//! Every file is read and checked once at start-up ([`init`]). A missing or
//! broken file is reported as a [`LoadError`] naming the file and the
//! place in it, and the app shows a calm error window instead of crashing.

use std::collections::HashSet;
use std::fmt;
use std::path::{Component, Path, PathBuf};
use std::sync::RwLock;

use serde::Deserialize;

use crate::calendar::geo::City;
use crate::calendar::texts::Texts;
use crate::content::{Library, Manifest, Section};
use crate::interview::Bank;
use crate::prayers::Book;
use crate::quotes::Quote;

/// The language used until the English version exists.
pub const DEFAULT_LANG: &str = "ru";
/// Folder name beside the exe.
pub const DIR_NAME: &str = "content";
/// Highest `contentFormat` this build understands.
pub const FORMAT: u32 = 1;

/// Files every language folder has besides the manifest and the sections
/// listed in it.
pub const INTERVIEW: &str = "interview.json";
pub const PRAYERS: &str = "prayers.json";
pub const QUOTES: &str = "quotes.json";
pub const CITIES: &str = "calendar/cities.json";
pub const DAYS: &str = "calendar/days.json";
pub const PARASHOT: &str = "calendar/parashot.json";
pub const MANIFEST: &str = "manifest.json";

/// Version fields of `manifest.json`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Meta {
    /// `contentVersion`: bumped with every content change («2026-11-23.2»).
    pub content_version: String,
    /// `minAppVersion`: the oldest program that can read these files.
    pub min_app_version: Option<String>,
    /// `contentFormat` (1 for now).
    pub format: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawMeta {
    #[serde(default)]
    content_version: Option<String>,
    #[serde(default)]
    min_app_version: Option<String>,
    #[serde(default)]
    content_format: Option<u32>,
}

/// One language's course content, parsed and checked.
pub struct Content {
    /// The language folder the files came from.
    pub dir: PathBuf,
    pub lang: String,
    pub meta: Meta,
    pub library: Library,
    pub interview: Bank,
    pub prayers: Book,
    pub quotes: Vec<Quote>,
    pub calendar: Texts,
    pub cities: Vec<City>,
}

impl fmt::Debug for Content {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Content")
            .field("dir", &self.dir)
            .field("lang", &self.lang)
            .field("meta", &self.meta)
            .finish_non_exhaustive()
    }
}

/// Why the content could not be used: which file, and what is wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadError {
    /// The file (or folder) at fault.
    pub file: PathBuf,
    /// A short Russian explanation, with the line and column for JSON errors.
    pub reason: String,
    /// Folders that were looked at (only when the folder itself is missing).
    pub searched: Vec<PathBuf>,
}

impl LoadError {
    fn new(file: impl Into<PathBuf>, reason: impl Into<String>) -> Self {
        LoadError {
            file: file.into(),
            reason: reason.into(),
            searched: Vec::new(),
        }
    }
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.file.display(), self.reason)?;
        for p in &self.searched {
            write!(f, "\n  искали: {}", p.display())?;
        }
        Ok(())
    }
}

impl std::error::Error for LoadError {}

// ------------------------------------------------------------ locating

/// Folders that may hold `content/`, in order:
/// `SHORASHIM_CONTENT_DIR` (only it, when set); the folder of the exe; the
/// source tree the program was built from (for `cargo run` and tests; it
/// does not exist on a user's computer).
pub fn candidate_roots() -> Vec<PathBuf> {
    if let Some(dir) = std::env::var_os("SHORASHIM_CONTENT_DIR").filter(|v| !v.is_empty()) {
        return vec![PathBuf::from(dir)];
    }
    let mut out = Vec::new();
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(Path::to_path_buf))
    {
        out.push(dir.join(DIR_NAME));
    }
    out.push(Path::new(env!("CARGO_MANIFEST_DIR")).join(DIR_NAME));
    out
}

/// The language folder inside a root: `<root>/<lang>/`, or the root itself
/// when it already is a language folder (has `manifest.json`).
pub fn lang_dir(root: &Path, lang: &str) -> Option<PathBuf> {
    let nested = root.join(lang);
    if nested.join(MANIFEST).is_file() {
        Some(nested)
    } else if root.join(MANIFEST).is_file() {
        Some(root.to_path_buf())
    } else {
        None
    }
}

/// The first language folder found among `roots`.
pub fn locate_in(roots: &[PathBuf], lang: &str) -> Result<PathBuf, LoadError> {
    for r in roots {
        if let Some(d) = lang_dir(r, lang) {
            return Ok(d);
        }
    }
    let first = roots
        .first()
        .cloned()
        .unwrap_or_else(|| PathBuf::from(DIR_NAME));
    Err(LoadError {
        file: first.join(lang),
        reason: "не найдена папка с материалами курса (нет файла manifest.json)".into(),
        searched: roots.iter().map(|r| r.join(lang)).collect(),
    })
}

// ------------------------------------------------------------ loading

fn read(dir: &Path, rel: &str) -> Result<String, LoadError> {
    let path = dir.join(rel);
    match std::fs::read(&path) {
        Ok(bytes) => {
            let s = String::from_utf8(bytes).map_err(|_| {
                LoadError::new(&path, "файл не в кодировке UTF-8 (сохраните его как UTF-8)")
            })?;
            // Notepad may save UTF-8 with a byte-order mark; JSON has none.
            Ok(s.strip_prefix('\u{feff}').map(str::to_string).unwrap_or(s))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Err(LoadError::new(&path, "файл не найден"))
        }
        Err(e) => Err(LoadError::new(&path, format!("не удалось прочитать: {e}"))),
    }
}

/// A JSON error as «строка 12, позиция 5: …».
fn json_reason(e: &serde_json::Error) -> String {
    let msg = e.to_string();
    let msg = match msg.rfind(" at line ") {
        Some(i) => msg[..i].to_string(),
        None => msg,
    };
    let msg = ru_json_message(&msg);
    if e.line() == 0 {
        format!("ошибка в данных: {msg}")
    } else {
        format!("строка {}, позиция {}: {msg}", e.line(), e.column())
    }
}

/// The usual JSON slips in plain Russian (others keep serde's wording).
fn ru_json_message(msg: &str) -> String {
    let known: &[(&str, &str)] = &[
        (
            "expected `,` or `}`",
            "пропущена запятая или закрывающая скобка }",
        ),
        (
            "expected `,` or `]`",
            "пропущена запятая или закрывающая скобка ]",
        ),
        ("trailing comma", "лишняя запятая перед закрывающей скобкой"),
        (
            "EOF while parsing",
            "файл обрывается: не хватает закрывающих скобок или кавычки",
        ),
        (
            "control character",
            "в строке перенос или табуляция: внутри кавычек пишите \\n",
        ),
        ("expected `:`", "пропущено двоеточие после названия поля"),
        (
            "key must be a string",
            "название поля должно быть в кавычках",
        ),
        (
            "expected value",
            "здесь ожидалось значение (строка в кавычках, число, [ ] или { })",
        ),
        (
            "invalid escape",
            "неверная обратная косая черта \\ внутри строки",
        ),
        ("trailing characters", "лишние символы после конца файла"),
    ];
    for (en, ru) in known {
        if msg.starts_with(en) {
            return ru.to_string();
        }
    }
    if let Some(rest) = msg.strip_prefix("missing field ") {
        return format!("нет обязательного поля {rest}");
    }
    msg.to_string()
}

fn parse<T: for<'de> Deserialize<'de>>(dir: &Path, rel: &str, raw: &str) -> Result<T, LoadError> {
    serde_json::from_str(raw).map_err(|e| LoadError::new(dir.join(rel), json_reason(&e)))
}

/// A section path from the manifest must stay inside the language folder.
fn safe_rel(rel: &str) -> bool {
    let p = Path::new(rel);
    !rel.is_empty()
        && p.components()
            .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
}

/// Read and check every file of one language folder.
pub fn load_dir(dir: &Path, lang: &str) -> Result<Content, LoadError> {
    let manifest_raw = read(dir, MANIFEST)?;
    let manifest: Manifest = parse(dir, MANIFEST, &manifest_raw)?;
    let raw_meta: RawMeta = parse(dir, MANIFEST, &manifest_raw)?;
    let meta = Meta {
        content_version: raw_meta
            .content_version
            .filter(|v| !v.trim().is_empty())
            .ok_or_else(|| LoadError::new(dir.join(MANIFEST), "нет поля contentVersion"))?,
        min_app_version: raw_meta.min_app_version.filter(|v| !v.trim().is_empty()),
        format: raw_meta.content_format.unwrap_or(1),
    };
    if meta.format > FORMAT {
        return Err(LoadError::new(
            dir.join(MANIFEST),
            format!(
                "материалы в формате {} — эта версия программы понимает только формат {FORMAT}; обновите программу",
                meta.format
            ),
        ));
    }
    if let Some(min) = &meta.min_app_version {
        let app = crate::install::current_version();
        if crate::install::newer(min, app) {
            return Err(LoadError::new(
                dir.join(MANIFEST),
                format!("материалам нужна программа версии {min} или новее, а установлена {app}"),
            ));
        }
    }
    if manifest.sections.is_empty() {
        return Err(LoadError::new(dir.join(MANIFEST), "список sections пуст"));
    }

    // Sections, each parsed on its own so an error names its file.
    let mut section_raws = Vec::new();
    let mut ids = HashSet::new();
    for m in &manifest.sections {
        let Some(rel) = m.file.as_deref() else {
            return Err(LoadError::new(
                dir.join(MANIFEST),
                format!("у раздела «{}» не указан file", m.id),
            ));
        };
        if !safe_rel(rel) {
            return Err(LoadError::new(
                dir.join(MANIFEST),
                format!("путь «{rel}» должен вести внутрь папки материалов"),
            ));
        }
        let raw = read(dir, rel)?;
        let sec: Section = parse(dir, rel, &raw)?;
        if sec.id != m.id {
            return Err(LoadError::new(
                dir.join(rel),
                format!(
                    "id раздела «{}», а в manifest.json он записан как «{}»",
                    sec.id, m.id
                ),
            ));
        }
        // Progress is keyed by ids: each must be unique.
        let mut seen = |id: &str, what: &str| -> Result<(), LoadError> {
            if id.trim().is_empty() {
                return Err(LoadError::new(dir.join(rel), format!("{what} без id")));
            }
            if !ids.insert(id.to_string()) {
                return Err(LoadError::new(
                    dir.join(rel),
                    format!("id «{id}» ({what}) встречается дважды"),
                ));
            }
            Ok(())
        };
        for l in &sec.lessons {
            seen(&l.id, "урок")?;
            for c in &l.cards {
                seen(&c.id, "карточка")?;
            }
        }
        for q in &sec.questions {
            seen(q.id(), "вопрос")?;
        }
        section_raws.push(raw);
    }
    let refs: Vec<&str> = section_raws.iter().map(String::as_str).collect();
    let library = Library::from_json(&manifest_raw, &refs)
        .map_err(|e| LoadError::new(dir.join(MANIFEST), json_reason(&e)))?;

    let interview: Bank = parse(dir, INTERVIEW, &read(dir, INTERVIEW)?)?;
    let mut seen = HashSet::new();
    for q in &interview.questions {
        if !seen.insert(q.id.as_str()) {
            return Err(LoadError::new(
                dir.join(INTERVIEW),
                format!("id вопроса «{}» встречается дважды", q.id),
            ));
        }
        if interview.topic(&q.topic).is_none() {
            return Err(LoadError::new(
                dir.join(INTERVIEW),
                format!("у вопроса «{}» неизвестная тема «{}»", q.id, q.topic),
            ));
        }
    }
    let prayers: Book = parse(dir, PRAYERS, &read(dir, PRAYERS)?)?;
    let quotes = crate::quotes::parse(&read(dir, QUOTES)?)
        .map_err(|e| LoadError::new(dir.join(QUOTES), json_reason(&e)))?;
    let cities: Vec<City> = parse(dir, CITIES, &read(dir, CITIES)?)?;
    let parashot = read(dir, PARASHOT)?;
    let days = read(dir, DAYS)?;
    let calendar = Texts::try_parse(&parashot, &days).map_err(|(which, e)| {
        let rel = if which == 0 { PARASHOT } else { DAYS };
        LoadError::new(dir.join(rel), json_reason(&e))
    })?;

    Ok(Content {
        dir: dir.to_path_buf(),
        lang: lang.to_string(),
        meta,
        library,
        interview,
        prayers,
        quotes,
        calendar,
        cities,
    })
}

// ------------------------------------------------------------ the active content

/// The content in use. Each load is leaked on purpose: screens keep
/// `&'static` references into it, and a reload (rare, a developer tool)
/// must not pull them out from under a running screen.
static CURRENT: RwLock<Option<&'static Content>> = RwLock::new(None);

fn set(c: Content) -> &'static Content {
    let c: &'static Content = Box::leak(Box::new(c));
    *CURRENT.write().unwrap_or_else(|e| e.into_inner()) = Some(c);
    c
}

fn current() -> Option<&'static Content> {
    *CURRENT.read().unwrap_or_else(|e| e.into_inner())
}

/// Load the content for `lang` once (later calls return the same content).
pub fn init(lang: &str) -> Result<&'static Content, LoadError> {
    if let Some(c) = current() {
        return Ok(c);
    }
    let dir = locate_in(&candidate_roots(), lang)?;
    let content = load_dir(&dir, lang)?;
    // Another thread may have won the race; keep the first one.
    let mut slot = CURRENT.write().unwrap_or_else(|e| e.into_inner());
    if let Some(c) = *slot {
        return Ok(c);
    }
    let c: &'static Content = Box::leak(Box::new(content));
    *slot = Some(c);
    Ok(c)
}

/// The content in use, loading it on first use. The app calls [`init`]
/// first and shows the error window when that fails, so this never panics
/// in the app itself; tools and tests get a clear message.
pub fn get() -> &'static Content {
    match init(DEFAULT_LANG) {
        Ok(c) => c,
        Err(e) => panic!("материалы курса не загружены: {e}"),
    }
}

/// Read the files again from the same folder (`shorashim-ctl
/// reload-content`). On an error the content in use stays as it was.
pub fn reload() -> Result<&'static Content, LoadError> {
    let (dir, lang) = match current() {
        Some(c) => (c.dir.clone(), c.lang.clone()),
        None => {
            let lang = DEFAULT_LANG.to_string();
            (locate_in(&candidate_roots(), &lang)?, lang)
        }
    };
    let content = load_dir(&dir, &lang)?;
    Ok(set(content))
}

/// One line about the content in use, for «О приложении» and `ctl`.
pub fn summary(c: &Content) -> String {
    let mut lessons = 0;
    let mut cards = 0;
    let mut questions = 0;
    for s in &c.library.sections {
        if let Some(d) = &s.data {
            lessons += d.lessons.len();
            cards += d.lessons.iter().map(|l| l.cards.len()).sum::<usize>();
            questions += d.questions.len();
        }
    }
    format!(
        "версия {}: {} разделов, {lessons} уроков, {cards} карточек, {questions} вопросов, {} вопросов собеседования, {} молитв, {} цитат",
        c.meta.content_version,
        c.library.sections.len(),
        c.interview.questions.len(),
        c.prayers.prayers.len(),
        c.quotes.len(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn repo_ru() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(DIR_NAME)
            .join(DEFAULT_LANG)
    }

    /// A scratch copy of the repo's `content/ru`.
    fn scratch(name: &str) -> PathBuf {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("shorashim-content-{name}-{n}"));
        crate::music::copy_dir(&repo_ru(), &root.join(DEFAULT_LANG)).unwrap();
        root
    }

    #[test]
    fn repo_content_loads_and_has_a_version() {
        let c = load_dir(&repo_ru(), "ru").unwrap();
        assert!(!c.meta.content_version.is_empty());
        assert!(
            c.meta.min_app_version.is_some(),
            "manifest needs minAppVersion"
        );
        assert!(c.library.sections.iter().all(|s| s.available));
        assert!(!c.interview.questions.is_empty());
        assert!(!c.prayers.prayers.is_empty());
        assert!(!c.quotes.is_empty());
        assert!(!c.cities.is_empty());
    }

    #[test]
    fn finds_nested_and_flat_language_folders() {
        let root = scratch("locate");
        assert_eq!(
            locate_in(std::slice::from_ref(&root), "ru").unwrap(),
            root.join("ru")
        );
        // Pointing straight at a language folder works too.
        let ru = root.join("ru");
        assert_eq!(locate_in(std::slice::from_ref(&ru), "ru").unwrap(), ru);
        let err = locate_in(&[root.join("nope")], "ru").unwrap_err();
        assert_eq!(err.searched, vec![root.join("nope").join("ru")]);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn broken_section_names_the_file_and_line() {
        let root = scratch("broken");
        let f = root.join("ru/sections/03-shabbat.json");
        let raw = std::fs::read_to_string(&f).unwrap();
        // Drop the first comma after the second line: invalid JSON.
        // (Windows checkouts may have CRLF line ends.)
        let at = raw.find(",\r\n").or_else(|| raw.find(",\n")).unwrap();
        std::fs::write(&f, format!("{}{}", &raw[..at], &raw[at + 1..])).unwrap();
        let err = load_dir(&root.join("ru"), "ru").unwrap_err();
        assert_eq!(err.file, f);
        assert!(err.reason.starts_with("строка "), "{}", err.reason);
        assert!(err.reason.contains("запятая"), "{}", err.reason);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn missing_file_and_bom_and_versions() {
        let root = scratch("missing");
        let ru = root.join("ru");
        // A byte-order mark (Notepad) is accepted.
        let q = ru.join(QUOTES);
        let raw = std::fs::read_to_string(&q).unwrap();
        std::fs::write(&q, format!("\u{feff}{raw}")).unwrap();
        assert!(load_dir(&ru, "ru").is_ok());

        std::fs::remove_file(ru.join(DAYS)).unwrap();
        let err = load_dir(&ru, "ru").unwrap_err();
        assert_eq!(err.file, ru.join(DAYS));
        assert_eq!(err.reason, "файл не найден");

        // Content that needs a newer program is refused, not misread.
        let m = ru.join(MANIFEST);
        let raw = std::fs::read_to_string(&m).unwrap();
        let newer = raw.replacen("\"minAppVersion\":\"", "\"minAppVersion\":\"99.", 1);
        assert_ne!(raw, newer);
        std::fs::write(&m, newer).unwrap();
        let err = load_dir(&ru, "ru").unwrap_err();
        assert_eq!(err.file, m);
        assert!(err.reason.contains("99."), "{}", err.reason);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn duplicate_ids_and_escaping_paths_are_refused() {
        let root = scratch("dup");
        let ru = root.join("ru");
        let f = ru.join("sections/02-ivrit.json");
        let sec: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&f).unwrap()).unwrap();
        let first = sec["lessons"][0]["id"].as_str().unwrap().to_string();
        let second = sec["lessons"][1]["id"].as_str().unwrap().to_string();
        let raw = std::fs::read_to_string(&f).unwrap().replacen(
            &format!("\"{second}\""),
            &format!("\"{first}\""),
            1,
        );
        std::fs::write(&f, raw).unwrap();
        let err = load_dir(&ru, "ru").unwrap_err();
        assert_eq!(err.file, f);
        assert!(err.reason.contains(&first), "{}", err.reason);

        assert!(!safe_rel("../x.json"));
        assert!(!safe_rel("/etc/passwd"));
        assert!(safe_rel("sections/01-osnovy.json"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn edited_card_shows_after_reload_without_rebuild() {
        let root = scratch("edit");
        let ru = root.join("ru");
        let f = ru.join("sections/01-osnovy.json");
        let sec: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&f).unwrap()).unwrap();
        let card = sec["lessons"][0]["cards"][0]["id"]
            .as_str()
            .unwrap()
            .to_string();
        let before = load_dir(&ru, "ru").unwrap();
        let (c, _, _) = before.library.card(&card).unwrap();
        let old_front = c.front.clone();
        let raw = std::fs::read_to_string(&f).unwrap().replacen(
            &serde_json::to_string(&old_front).unwrap(),
            "\"Исправлено\"",
            1,
        );
        std::fs::write(&f, raw).unwrap();
        let after = load_dir(&ru, "ru").unwrap();
        assert_eq!(after.library.card(&card).unwrap().0.front, "Исправлено");
        let _ = std::fs::remove_dir_all(root);
    }

    /// Progress written against older content keeps working when lessons,
    /// cards or questions are removed: unknown ids are ignored, not fatal.
    #[test]
    fn progress_with_removed_items_still_opens_every_screen() {
        let dir = std::env::temp_dir().join(format!(
            "shorashim-ghost-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let data = dir.join("progress.json");
        let lib = Library::current();
        let real_lesson = lib.sections[0].data.as_ref().unwrap().lessons[0].id.clone();
        let progress = serde_json::json!({
            "app": "giyur-study", "schema": 1, "onboarded": true,
            "settings": {"level": "reform"},
            "lessonsRead": {"ghost-lesson": "2026-11-01", real_lesson: "2026-11-01"},
            "cards": {"ghost-card": {"l": 3, "due": "2026-11-01", "n": 2}},
            "questions": {"ghost-q": {"n": 3, "miss": 1.0, "streak": 0, "last": "2026-11-01"}},
            "best": {"ghost-section": 90.0},
            "studyMode": {"ghost-section": "prep"},
            "examHistory": [["ghost-q", "ghost-q2"]],
        });
        std::fs::write(&data, progress.to_string()).unwrap();
        let mut s = crate::automation::session::Session::new(crate::automation::session::Options {
            data: Some(data),
            today: chrono::NaiveDate::from_ymd_opt(2026, 11, 23),
            reviewer: false,
        });
        let script = [
            "map",
            "cards:review",
            "progress",
            "exam",
            "timed",
            "interview:practice",
            "prayers",
            "calendar",
            "settings",
        ]
        .iter()
        .map(|r| format!("{{\"cmd\":\"navigate\",\"to\":\"{r}\"}}"))
        .chain([r#"{"cmd":"reload_content"}"#.to_string()])
        .collect::<Vec<_>>()
        .join("\n");
        // The file was read as is (not replaced by a fresh state).
        assert!(s.ctx.state.cards.contains_key("ghost-card"));
        assert!(s.ctx.state.lessons_read.contains_key("ghost-lesson"));
        let mut out = Vec::new();
        let failed = s.run_lines(script.as_bytes(), &mut out, false);
        assert_eq!(failed, 0, "{}", String::from_utf8_lossy(&out));
        let _ = std::fs::remove_dir_all(dir);
    }
}
