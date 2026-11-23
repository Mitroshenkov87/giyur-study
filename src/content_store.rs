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
//! English lives in `content\en\` with the same file names, ids and JSON
//! shape (`check_parity`). Fixing a card is editing its JSON file and
//! restarting the app (or `shorashim-ctl reload-content`); see
//! docs/CONTENT-UPDATES.md.
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
use crate::i18n::{self, Lang};
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
    /// A short explanation, with the line and column for JSON errors.
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
            write!(
                f,
                "\n  {}: {}",
                i18n::tr_or("искали", "looked in"),
                p.display()
            )?;
        }
        Ok(())
    }
}

fn msg(ru: &str, en: &str) -> String {
    i18n::tr_or(ru, en).to_string()
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
        // macOS app bundle: Shorashim.app/Contents/MacOS/shorashim keeps
        // its course files in Contents/Resources/content.
        if let Some(res) = bundle_resources(&dir) {
            out.push(res.join(DIR_NAME));
        }
        // Linux AppImage (usr/bin/shorashim): usr/share/shorashim/content.
        if let Some(share) = share_dir(&dir) {
            out.push(share.join(DIR_NAME));
        }
    }
    out.push(Path::new(env!("CARGO_MANIFEST_DIR")).join(DIR_NAME));
    out
}

/// `Contents/Resources` of a macOS app bundle, given the folder of its
/// binary (`Contents/MacOS`); `None` outside a bundle.
pub fn bundle_resources(exe_dir: &Path) -> Option<PathBuf> {
    if exe_dir.file_name()? != "MacOS" {
        return None;
    }
    let res = exe_dir.parent()?.join("Resources");
    res.is_dir().then_some(res)
}

/// `../share/shorashim` beside a `bin/` folder (Linux AppImage or a
/// system install); `None` when it does not exist.
pub fn share_dir(exe_dir: &Path) -> Option<PathBuf> {
    if exe_dir.file_name()? != "bin" {
        return None;
    }
    let share = exe_dir.parent()?.join("share").join("shorashim");
    share.is_dir().then_some(share)
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
        reason: msg(
            "не найдена папка с материалами курса (нет файла manifest.json)",
            "course folder not found (no manifest.json)",
        ),
        searched: roots.iter().map(|r| r.join(lang)).collect(),
    })
}

// ------------------------------------------------------------ loading

fn read(dir: &Path, rel: &str) -> Result<String, LoadError> {
    let path = dir.join(rel);
    match std::fs::read(&path) {
        Ok(bytes) => {
            let s = String::from_utf8(bytes).map_err(|_| {
                LoadError::new(
                    &path,
                    msg(
                        "файл не в кодировке UTF-8 (сохраните его как UTF-8)",
                        "file is not UTF-8 (save it as UTF-8)",
                    ),
                )
            })?;
            // Notepad may save UTF-8 with a byte-order mark; JSON has none.
            Ok(s.strip_prefix('\u{feff}').map(str::to_string).unwrap_or(s))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(LoadError::new(
            &path,
            msg("файл не найден", "file not found"),
        )),
        Err(e) => Err(LoadError::new(
            &path,
            crate::trf!("не удалось прочитать: {e}", "could not read: {e}"),
        )),
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
        crate::trf!("ошибка в данных: {msg}", "data error: {msg}")
    } else {
        crate::trf!(
            "строка {}, позиция {}: {msg}",
            "line {}, column {}: {msg}",
            e.line(),
            e.column()
        )
    }
}

/// The usual JSON slips in plain Russian (others keep serde's wording).
fn ru_json_message(msg: &str) -> String {
    let known: &[(&str, &str, &str)] = &[
        (
            "expected `,` or `}`",
            "пропущена запятая или закрывающая скобка }",
            "missing comma or closing }",
        ),
        (
            "expected `,` or `]`",
            "пропущена запятая или закрывающая скобка ]",
            "missing comma or closing ]",
        ),
        (
            "trailing comma",
            "лишняя запятая перед закрывающей скобкой",
            "trailing comma before a closing bracket",
        ),
        (
            "EOF while parsing",
            "файл обрывается: не хватает закрывающих скобок или кавычки",
            "file ends too soon: a bracket or quote is missing",
        ),
        (
            "control character",
            "в строке перенос или табуляция: внутри кавычек пишите \\n",
            "a line break or tab inside a string: write \\n instead",
        ),
        (
            "expected `:`",
            "пропущено двоеточие после названия поля",
            "missing colon after a field name",
        ),
        (
            "key must be a string",
            "название поля должно быть в кавычках",
            "field names must be quoted",
        ),
        (
            "expected value",
            "здесь ожидалось значение (строка в кавычках, число, [ ] или { })",
            "a value was expected (quoted string, number, [ ] or { })",
        ),
        (
            "invalid escape",
            "неверная обратная косая черта \\ внутри строки",
            "invalid backslash \\ inside a string",
        ),
        (
            "trailing characters",
            "лишние символы после конца файла",
            "extra characters after the end of the file",
        ),
    ];
    for (en_key, ru, en) in known {
        if msg.starts_with(en_key) {
            return i18n::tr_or(ru, en).to_string();
        }
    }
    if let Some(rest) = msg.strip_prefix("missing field ") {
        return crate::trf!(
            "нет обязательного поля {rest}",
            "missing required field {rest}"
        );
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
            .ok_or_else(|| {
                LoadError::new(
                    dir.join(MANIFEST),
                    msg("нет поля contentVersion", "missing field contentVersion"),
                )
            })?,
        min_app_version: raw_meta.min_app_version.filter(|v| !v.trim().is_empty()),
        format: raw_meta.content_format.unwrap_or(1),
    };
    if meta.format > FORMAT {
        return Err(LoadError::new(
            dir.join(MANIFEST),
            crate::trf!(
                "материалы в формате {} — эта версия программы понимает только формат {FORMAT}; обновите программу",
                "content format {} — this app understands only format {FORMAT}; please update the app",
                meta.format
            ),
        ));
    }
    if let Some(min) = &meta.min_app_version {
        let app = crate::install::current_version();
        if crate::install::newer(min, app) {
            return Err(LoadError::new(
                dir.join(MANIFEST),
                crate::trf!(
                    "материалам нужна программа версии {min} или новее, а установлена {app}",
                    "this content needs app version {min} or newer; {app} is installed"
                ),
            ));
        }
    }
    if manifest.sections.is_empty() {
        return Err(LoadError::new(
            dir.join(MANIFEST),
            msg("список sections пуст", "sections list is empty"),
        ));
    }

    // Sections, each parsed on its own so an error names its file.
    let mut section_raws = Vec::new();
    let mut ids = HashSet::new();
    for m in &manifest.sections {
        let Some(rel) = m.file.as_deref() else {
            return Err(LoadError::new(
                dir.join(MANIFEST),
                crate::trf!(
                    "у раздела «{}» не указан file",
                    "section «{}» has no file",
                    m.id
                ),
            ));
        };
        if !safe_rel(rel) {
            return Err(LoadError::new(
                dir.join(MANIFEST),
                crate::trf!(
                    "путь «{rel}» должен вести внутрь папки материалов",
                    "path «{rel}» must stay inside the content folder"
                ),
            ));
        }
        let raw = read(dir, rel)?;
        let sec: Section = parse(dir, rel, &raw)?;
        if sec.id != m.id {
            return Err(LoadError::new(
                dir.join(rel),
                crate::trf!(
                    "id раздела «{}», а в manifest.json он записан как «{}»",
                    "section id is «{}», but manifest.json lists it as «{}»",
                    sec.id,
                    m.id
                ),
            ));
        }
        // Progress is keyed by ids: each must be unique.
        let mut seen = |id: &str, what_ru: &str, what_en: &str| -> Result<(), LoadError> {
            if id.trim().is_empty() {
                return Err(LoadError::new(
                    dir.join(rel),
                    crate::trf!("{what_ru} без id", "{what_en} with no id"),
                ));
            }
            if !ids.insert(id.to_string()) {
                return Err(LoadError::new(
                    dir.join(rel),
                    crate::trf!(
                        "id «{id}» ({what_ru}) встречается дважды",
                        "id «{id}» ({what_en}) appears twice"
                    ),
                ));
            }
            Ok(())
        };
        for l in &sec.lessons {
            seen(&l.id, "урок", "lesson")?;
            for c in &l.cards {
                seen(&c.id, "карточка", "card")?;
            }
        }
        for q in &sec.questions {
            seen(q.id(), "вопрос", "question")?;
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
                crate::trf!(
                    "id вопроса «{}» встречается дважды",
                    "question id «{}» appears twice",
                    q.id
                ),
            ));
        }
        if interview.topic(&q.topic).is_none() {
            return Err(LoadError::new(
                dir.join(INTERVIEW),
                crate::trf!(
                    "у вопроса «{}» неизвестная тема «{}»",
                    "question «{}» has unknown topic «{}»",
                    q.id,
                    q.topic
                ),
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
        Err(e) => panic!(
            "{}: {e}",
            msg("материалы курса не загружены", "course content not loaded")
        ),
    }
}

/// Load `lang`, and if that folder is missing or broken, load Russian
/// instead. The process interface language is left as the caller set it
/// (English UI can show Russian course files until `content/en` exists).
pub fn init_with_fallback(lang: Lang) -> Result<&'static Content, LoadError> {
    match init(lang.code()) {
        Ok(c) => Ok(c),
        Err(e) if lang != Lang::Ru => match init(Lang::Ru.code()) {
            Ok(c) => Ok(c),
            Err(_) => Err(e),
        },
        Err(e) => Err(e),
    }
}

/// Headless / ctl: read the profile (env path, else default), set the
/// process language (`boot`: env, else saved, else Russian) and load that
/// language's content, falling back to Russian files if needed. Does not
/// write the profile (`SHORASHIM_LANG` must not stick).
pub fn start(data: Option<&Path>) -> Result<&'static Content, LoadError> {
    let path = data
        .map(|p| p.to_path_buf())
        .or_else(crate::storage::data_path_from_env)
        .unwrap_or_else(crate::storage::default_path);
    let today = crate::util::today();
    let (state, _) = crate::storage::load_checked(&path, &today);
    let lang = i18n::boot(state.settings.lang);
    init_with_fallback(lang)
}

/// GUI first launch: env, else the saved profile, else the OS language;
/// load that content, falling back to Russian files if needed.
pub fn start_os(data: Option<&Path>) -> Result<&'static Content, LoadError> {
    let path = data
        .map(|p| p.to_path_buf())
        .or_else(crate::storage::data_path_from_env)
        .unwrap_or_else(crate::storage::default_path);
    let today = crate::util::today();
    let (state, _) = crate::storage::load_checked(&path, &today);
    let lang = i18n::boot_os(state.settings.lang);
    init_with_fallback(lang)
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

/// Load another language's files and, only if they are valid, make them
/// the content in use. On an error the previous language and content stay.
pub fn switch_language(lang: &str) -> Result<&'static Content, LoadError> {
    let dir = locate_in(&candidate_roots(), lang)?;
    let content = load_dir(&dir, lang)?;
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
    crate::trf!(
        "версия {}: {} разделов, {lessons} уроков, {cards} карточек, {questions} вопросов, {} вопросов собеседования, {} молитв, {} цитат",
        "version {}: {} sections, {lessons} lessons, {cards} cards, {questions} questions, {} interview questions, {} prayers, {} quotes",
        c.meta.content_version,
        c.library.sections.len(),
        c.interview.questions.len(),
        c.prayers.prayers.len(),
        c.quotes.len(),
    )
}

// ------------------------------------------------------------ language parity

/// One difference between `content/ru` and `content/en`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParityIssue {
    /// Relative path inside the language folder, or `*` for a folder-level
    /// problem (a missing language, a missing file).
    pub file: String,
    pub reason: String,
}

/// Result of comparing two language folders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParityResult {
    /// Same files, same ids, same JSON shape.
    Ok,
    /// The other language folder is not there yet (English may land later).
    Skipped {
        missing: PathBuf,
    },
    Failed(Vec<ParityIssue>),
}

impl ParityResult {
    pub fn issues(&self) -> &[ParityIssue] {
        match self {
            ParityResult::Failed(v) => v,
            _ => &[],
        }
    }

    pub fn is_ok(&self) -> bool {
        matches!(self, ParityResult::Ok | ParityResult::Skipped { .. })
    }
}

/// Compare `content/ru` with `content/en` under `root` (the `content/`
/// folder, or a scratch copy). Same JSON files, same ids, no field that
/// exists in Russian missing from English. Extra English keys are allowed.
pub fn check_parity(root: &Path) -> ParityResult {
    check_parity_between(root, "ru", "en")
}

/// Compare language folder `base` with `other` (ids and shape of `base`
/// must exist in `other`).
pub fn check_parity_between(root: &Path, base: &str, other: &str) -> ParityResult {
    let base_dir = match lang_dir(root, base) {
        Some(d) => d,
        None => {
            return ParityResult::Failed(vec![ParityIssue {
                file: "*".into(),
                reason: format!(
                    "no {base}/ folder with manifest.json under {}",
                    root.display()
                ),
            }]);
        }
    };
    let other_dir = match lang_dir(root, other) {
        Some(d) => d,
        None => {
            return ParityResult::Skipped {
                missing: root.join(other),
            };
        }
    };
    let mut issues = Vec::new();
    let base_files = json_rel_files(&base_dir);
    let other_files = json_rel_files(&other_dir);
    for rel in &base_files {
        if !other_files.contains(rel) {
            issues.push(ParityIssue {
                file: rel.clone(),
                reason: format!("missing in {other}/ (present in {base}/)"),
            });
        }
    }
    for rel in &base_files {
        if !other_files.contains(rel) {
            continue;
        }
        let a = match std::fs::read_to_string(base_dir.join(rel)) {
            Ok(s) => s,
            Err(e) => {
                issues.push(ParityIssue {
                    file: rel.clone(),
                    reason: format!("could not read {base}: {e}"),
                });
                continue;
            }
        };
        let b = match std::fs::read_to_string(other_dir.join(rel)) {
            Ok(s) => s,
            Err(e) => {
                issues.push(ParityIssue {
                    file: rel.clone(),
                    reason: format!("could not read {other}: {e}"),
                });
                continue;
            }
        };
        let av: serde_json::Value = match serde_json::from_str(&a) {
            Ok(v) => v,
            Err(e) => {
                issues.push(ParityIssue {
                    file: rel.clone(),
                    reason: format!("{base} is not JSON: {e}"),
                });
                continue;
            }
        };
        let bv: serde_json::Value = match serde_json::from_str(&b) {
            Ok(v) => v,
            Err(e) => {
                issues.push(ParityIssue {
                    file: rel.clone(),
                    reason: format!("{other} is not JSON: {e}"),
                });
                continue;
            }
        };
        walk_shape(rel, "", &av, &bv, &mut issues);
        let a_ids = collect_ids(&av);
        let b_ids = collect_ids(&bv);
        for id in &a_ids {
            if !b_ids.contains(id) {
                issues.push(ParityIssue {
                    file: rel.clone(),
                    reason: format!("id «{id}» is in {base} but not in {other}"),
                });
            }
        }
        for id in &b_ids {
            if !a_ids.contains(id) {
                issues.push(ParityIssue {
                    file: rel.clone(),
                    reason: format!("id «{id}» is in {other} but not in {base}"),
                });
            }
        }
    }
    if issues.is_empty() {
        ParityResult::Ok
    } else {
        ParityResult::Failed(issues)
    }
}

fn json_rel_files(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    walk_json(dir, dir, &mut out);
    out.sort();
    out
}

fn walk_json(root: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk_json(root, &p, out);
        } else if p.extension().and_then(|x| x.to_str()) == Some("json") {
            if let Ok(rel) = p.strip_prefix(root) {
                out.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
}

fn walk_shape(
    file: &str,
    path: &str,
    base: &serde_json::Value,
    other: &serde_json::Value,
    issues: &mut Vec<ParityIssue>,
) {
    use serde_json::Value;
    let here = |k: &str| {
        if path.is_empty() {
            k.to_string()
        } else if k.is_empty() {
            path.to_string()
        } else {
            format!("{path}.{k}")
        }
    };
    match (base, other) {
        (Value::Object(a), Value::Object(b)) => {
            for (k, v) in a {
                match b.get(k) {
                    None => issues.push(ParityIssue {
                        file: file.into(),
                        reason: format!("missing field {}", here(k)),
                    }),
                    Some(ov) => walk_shape(file, &here(k), v, ov, issues),
                }
            }
        }
        (Value::Array(a), Value::Array(b)) => {
            if a.len() != b.len() {
                issues.push(ParityIssue {
                    file: file.into(),
                    reason: format!(
                        "array {} has {} items in ru and {} in the other language",
                        if path.is_empty() { "<root>" } else { path },
                        a.len(),
                        b.len()
                    ),
                });
            }
            for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
                walk_shape(file, &format!("{path}[{i}]"), x, y, issues);
            }
        }
        (Value::String(_), Value::String(_))
        | (Value::Number(_), Value::Number(_))
        | (Value::Bool(_), Value::Bool(_))
        | (Value::Null, Value::Null) => {}
        _ => {
            if std::mem::discriminant(base) != std::mem::discriminant(other) {
                issues.push(ParityIssue {
                    file: file.into(),
                    reason: format!(
                        "type mismatch at {}",
                        if path.is_empty() { "<root>" } else { path }
                    ),
                });
            }
        }
    }
}

fn collect_ids(v: &serde_json::Value) -> std::collections::BTreeSet<String> {
    let mut out = std::collections::BTreeSet::new();
    collect_ids_into(v, &mut out);
    out
}

fn collect_ids_into(v: &serde_json::Value, out: &mut std::collections::BTreeSet<String>) {
    match v {
        serde_json::Value::Object(m) => {
            if let Some(serde_json::Value::String(id)) = m.get("id") {
                if !id.is_empty() {
                    out.insert(id.clone());
                }
            }
            for val in m.values() {
                collect_ids_into(val, out);
            }
        }
        serde_json::Value::Array(a) => {
            for val in a {
                collect_ids_into(val, out);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn macos_bundle_resources_are_found_beside_contents_macos() {
        let tmp = std::env::temp_dir().join(format!("shorashim-bundle-{}", std::process::id()));
        let macos = tmp.join("Shorashim.app/Contents/MacOS");
        std::fs::create_dir_all(&macos).unwrap();
        assert_eq!(bundle_resources(&macos), None, "no Resources yet");
        let res = tmp.join("Shorashim.app/Contents/Resources");
        std::fs::create_dir_all(&res).unwrap();
        assert_eq!(bundle_resources(&macos), Some(res));
        assert_eq!(bundle_resources(&tmp), None, "not a bundle");
        let bin = tmp.join("AppDir/usr/bin");
        std::fs::create_dir_all(&bin).unwrap();
        assert_eq!(share_dir(&bin), None, "no share yet");
        let share = tmp.join("AppDir/usr/share/shorashim");
        std::fs::create_dir_all(&share).unwrap();
        assert_eq!(share_dir(&bin), Some(share));
        let _ = std::fs::remove_dir_all(&tmp);
    }

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

    #[test]
    fn repo_en_is_optional_and_matches_when_present() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(DIR_NAME);
        match check_parity(&root) {
            ParityResult::Ok | ParityResult::Skipped { .. } => {}
            ParityResult::Failed(issues) => {
                panic!("content/en does not match content/ru: {issues:?}")
            }
        }
    }

    #[test]
    fn parity_reports_missing_file_id_and_field() {
        let root = scratch("parity");
        // A copy of ru as en: equal → Ok.
        crate::music::copy_dir(&root.join("ru"), &root.join("en")).unwrap();
        assert!(matches!(check_parity(&root), ParityResult::Ok));

        // Missing file.
        std::fs::remove_file(root.join("en").join(QUOTES)).unwrap();
        match check_parity(&root) {
            ParityResult::Failed(issues) => {
                assert!(
                    issues
                        .iter()
                        .any(|i| i.file == QUOTES && i.reason.contains("missing")),
                    "{issues:?}"
                );
            }
            other => panic!("{other:?}"),
        }
        crate::music::copy_dir(&root.join("ru"), &root.join("en")).unwrap();

        // Missing field in a section.
        let f = root.join("en/sections/01-osnovy.json");
        let mut sec: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&f).unwrap()).unwrap();
        sec.as_object_mut().unwrap().remove("lessons");
        std::fs::write(&f, sec.to_string()).unwrap();
        match check_parity(&root) {
            ParityResult::Failed(issues) => {
                assert!(
                    issues.iter().any(|i| i.reason.contains("missing field")
                        && i.reason.contains("lessons")),
                    "{issues:?}"
                );
            }
            other => panic!("{other:?}"),
        }
        crate::music::copy_dir(&root.join("ru"), &root.join("en")).unwrap();

        // Missing id: drop the first lesson from English.
        let f = root.join("en/sections/01-osnovy.json");
        let mut sec: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&f).unwrap()).unwrap();
        let dropped = sec["lessons"][0]["id"].as_str().unwrap().to_string();
        sec["lessons"].as_array_mut().unwrap().remove(0);
        std::fs::write(&f, sec.to_string()).unwrap();
        match check_parity(&root) {
            ParityResult::Failed(issues) => {
                assert!(
                    issues.iter().any(|i| i.reason.contains(&dropped)),
                    "{issues:?}"
                );
            }
            other => panic!("{other:?}"),
        }

        // No en folder: skipped, not a failure (English lands gradually).
        let _ = std::fs::remove_dir_all(root.join("en"));
        assert!(matches!(check_parity(&root), ParityResult::Skipped { .. }));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn switch_language_keeps_previous_when_target_is_invalid() {
        let root = scratch("switch");
        // Broken English: missing manifest.
        std::fs::create_dir_all(root.join("en")).unwrap();
        std::fs::write(root.join("en/notes.txt"), "no manifest").unwrap();
        let ru = load_dir(&root.join("ru"), "ru").unwrap();
        let err = load_dir(&root.join("en"), "en").unwrap_err();
        assert!(
            err.reason.contains("manifest")
                || err.reason.contains("не найден")
                || err.reason.contains("not found"),
            "{}",
            err.reason
        );
        // The Russian load is still valid: callers only replace CURRENT on Ok.
        assert_eq!(ru.lang, "ru");
        assert!(!ru.library.sections.is_empty());
        let _ = std::fs::remove_dir_all(root);
    }

    /// English course files: present, loadable, the same ids as Russian,
    /// and Jewish framing only (Torah / Tanakh, never the other names).
    #[test]
    fn repo_en_loads_matches_ru_and_keeps_jewish_framing() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(DIR_NAME);
        let en = load_dir(&root.join("en"), "en").expect("content/en must load");
        assert!(matches!(check_parity(&root), ParityResult::Ok));
        assert_eq!(en.lang, "en");
        let mut files = Vec::new();
        fn walk(d: &Path, out: &mut Vec<PathBuf>) {
            for e in std::fs::read_dir(d).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, out);
                } else if p.extension().is_some_and(|x| x == "json") {
                    out.push(p);
                }
            }
        }
        walk(&root.join("en"), &mut files);
        let banned = [
            "bible",
            "biblical",
            "old testament",
            "new testament",
            "christian",
            "christianity",
            "church",
            "jesus",
            "heresy",
            "heretic",
            "heretical",
            "west bank",
        ];
        for f in files {
            let low = std::fs::read_to_string(&f).unwrap().to_lowercase();
            let words: Vec<&str> = low
                .split(|c: char| !(c.is_alphanumeric() || c == '\''))
                .filter(|w| !w.is_empty())
                .collect();
            for b in banned {
                let parts: Vec<&str> = b.split(' ').collect();
                let hit = words.windows(parts.len()).any(|w| w == parts.as_slice());
                assert!(!hit, "{}: «{b}»", f.display());
            }
        }
    }
}
