//! Interface language: Russian (the source text) or English.
//!
//! The app's own interface strings are written in Russian in the code and
//! wrapped in [`tr`] (plain text) or [`trf!`](crate::trf) (formatted text).
//! English comes from the catalog `src/i18n/en.json`, a flat map
//! «Russian original» → «English». A string missing from the catalog falls
//! back to Russian (a test, `catalog_covers_every_tr_call`, keeps the
//! catalog complete once it is filled). Course content is not here: it is
//! read from `content/<lang>/` (src/content_store.rs).
//!
//! The language is a process-wide setting: chosen on first launch from the
//! Windows interface language ([`detect_system`]), then from the profile
//! (`settings.lang`). `SHORASHIM_LANG=en|ru` overrides both (tests,
//! screenshots) without touching the profile.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::OnceLock;

/// The two interface and content languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Lang {
    #[default]
    Ru,
    En,
}

impl Lang {
    pub const ALL: [Lang; 2] = [Lang::Ru, Lang::En];

    /// Folder name under `content/` and the value stored in the profile.
    pub fn code(self) -> &'static str {
        match self {
            Lang::Ru => "ru",
            Lang::En => "en",
        }
    }

    pub fn from_code(code: &str) -> Option<Lang> {
        match code.trim().to_ascii_lowercase().as_str() {
            "ru" | "rus" | "russian" | "ru-ru" => Some(Lang::Ru),
            "en" | "eng" | "english" | "en-us" | "en-gb" => Some(Lang::En),
            _ => None,
        }
    }

    /// The language's own name, for the switch in «Настройки».
    pub fn native_name(self) -> &'static str {
        match self {
            Lang::Ru => "Русский",
            Lang::En => "English",
        }
    }
}

impl std::fmt::Display for Lang {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.native_name())
    }
}

impl serde::Serialize for Lang {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.code())
    }
}

impl<'de> serde::Deserialize<'de> for Lang {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Lang::from_code(&s).ok_or_else(|| serde::de::Error::unknown_variant(&s, &["ru", "en"]))
    }
}

static LANG: AtomicU8 = AtomicU8::new(0);

thread_local! {
    /// Per-thread override for tests (they run in parallel threads).
    static OVERRIDE: std::cell::Cell<Option<Lang>> = const { std::cell::Cell::new(None) };
}

/// The interface language in use.
pub fn lang() -> Lang {
    if let Some(l) = OVERRIDE.with(|o| o.get()) {
        return l;
    }
    match LANG.load(Ordering::Relaxed) {
        1 => Lang::En,
        _ => Lang::Ru,
    }
}

pub fn is_en() -> bool {
    lang() == Lang::En
}

/// Switch the interface language for the whole process. In the crate's own
/// unit tests it only switches the calling thread, so parallel tests never
/// see each other's language.
pub fn set_lang(l: Lang) {
    #[cfg(test)]
    {
        OVERRIDE.with(|o| o.set(Some(l)));
        return;
    }
    #[allow(unreachable_code)]
    LANG.store(
        match l {
            Lang::Ru => 0,
            Lang::En => 1,
        },
        Ordering::Relaxed,
    );
}

/// Run `f` with `l` as the language of this thread only (tests).
pub fn with_lang<T>(l: Lang, f: impl FnOnce() -> T) -> T {
    let prev = OVERRIDE.with(|o| o.replace(Some(l)));
    let out = f();
    OVERRIDE.with(|o| o.set(prev));
    out
}

/// `SHORASHIM_LANG=en|ru`, when set.
pub fn env_override() -> Option<Lang> {
    std::env::var("SHORASHIM_LANG")
        .ok()
        .and_then(|v| Lang::from_code(&v))
}

/// Pick the language: env override, else the saved profile, else `fallback`.
pub fn pick(saved: Option<Lang>, env: Option<Lang>, fallback: Lang) -> Lang {
    env.or(saved).unwrap_or(fallback)
}

/// Pick the language: env override, else the saved profile, else `fallback`.
/// Sets the process language. Does not write the profile.
pub fn boot_choice(saved: Option<Lang>, env: Option<Lang>, fallback: Lang) -> Lang {
    let l = pick(saved, env, fallback);
    set_lang(l);
    l
}

/// Headless, ctl and tests: env, else the saved profile, else Russian
/// (so parallel tests do not pick up the OS locale).
pub fn boot(saved: Option<Lang>) -> Lang {
    boot_choice(saved, env_override(), Lang::Ru)
}

/// GUI first launch: env, else the saved profile, else the OS language
/// (Russian only when Windows speaks Russian).
pub fn boot_os(saved: Option<Lang>) -> Lang {
    boot_choice(saved, env_override(), detect_system())
}

/// Installer / setup: env, else the OS language. Does not read the profile.
pub fn apply_os_lang() -> Lang {
    boot_choice(None, env_override(), detect_system())
}

/// True when this run should write the chosen language into an empty
/// profile: no saved choice, and `SHORASHIM_LANG` is not set.
pub fn persist_detected(saved: Option<Lang>) -> bool {
    saved.is_none() && env_override().is_none()
}

/// The language for a first launch: Russian only when Windows speaks
/// Russian, English for every other interface language.
pub fn detect_system() -> Lang {
    lang_for_ui_language(system_ui_language())
}

/// Windows LANGID → our language. Russian is primary language 0x19.
pub fn lang_for_ui_language(langid: Option<u16>) -> Lang {
    match langid {
        Some(id) if id & 0x3ff == 0x19 => Lang::Ru,
        Some(_) => Lang::En,
        // Not Windows: look at the usual locale variables.
        None => {
            let v = ["LC_ALL", "LC_MESSAGES", "LANG"]
                .iter()
                .find_map(|k| std::env::var(k).ok().filter(|v| !v.is_empty()))
                .unwrap_or_default();
            if v.to_ascii_lowercase().starts_with("ru") {
                Lang::Ru
            } else {
                Lang::En
            }
        }
    }
}

#[cfg(windows)]
fn system_ui_language() -> Option<u16> {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetUserDefaultUILanguage() -> u16;
    }
    // SAFETY: no arguments, returns a LANGID.
    Some(unsafe { GetUserDefaultUILanguage() })
}

#[cfg(not(windows))]
fn system_ui_language() -> Option<u16> {
    None
}

fn catalog() -> &'static HashMap<String, String> {
    static CAT: OnceLock<HashMap<String, String>> = OnceLock::new();
    CAT.get_or_init(|| serde_json::from_str(include_str!("en.json")).unwrap_or_default())
}

/// The interface string `ru` in the current language.
pub fn tr(ru: &str) -> &str {
    if !is_en() {
        return ru;
    }
    en(ru).unwrap_or(ru)
}

/// Like [`tr`], but when English is on and the catalog has no entry, use
/// `en` instead of leaving the Russian on screen (installer, load errors).
pub fn tr_or<'a>(ru: &'a str, en: &'a str) -> &'a str {
    if !is_en() {
        ru
    } else {
        self::en(ru).unwrap_or(en)
    }
}

/// The English of `ru` from the catalog, if there is one.
pub fn en(ru: &str) -> Option<&'static str> {
    catalog().get(ru).map(String::as_str)
}

/// The app's name in the current language.
pub fn app_name() -> &'static str {
    match lang() {
        Lang::Ru => "Шорашим",
        Lang::En => "Shorashim",
    }
}

/// Window title / shortcut description in the current language.
pub fn app_tagline() -> &'static str {
    tr_or(
        "основы иудаизма и подготовка к гиюру",
        "foundations of Judaism and preparation for giyur",
    )
}

/// `«Шорашим» — …` / `Shorashim — …` for the window title.
pub fn window_title() -> String {
    format!("{} — {}", app_name(), app_tagline())
}

/// Formatted text in both languages, picked at run time:
/// `trf!("Урок {n} из {total}", "Lesson {n} of {total}")`. Arguments may be
/// captured from scope or passed after the two formats, as in `format!`.
#[macro_export]
macro_rules! trf {
    ($ru:literal, $en:literal $(, $($arg:tt)*)?) => {
        if $crate::i18n::is_en() {
            format!($en $(, $($arg)*)?)
        } else {
            format!($ru $(, $($arg)*)?)
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn russian_windows_gives_russian_everything_else_english() {
        assert_eq!(lang_for_ui_language(Some(0x0419)), Lang::Ru); // ru-RU
        assert_eq!(lang_for_ui_language(Some(0x0819)), Lang::Ru); // ru-MD
        assert_eq!(lang_for_ui_language(Some(0x0409)), Lang::En); // en-US
        assert_eq!(lang_for_ui_language(Some(0x040D)), Lang::En); // he-IL
        assert_eq!(lang_for_ui_language(Some(0x0422)), Lang::En); // uk-UA
        assert_eq!(lang_for_ui_language(Some(0x0407)), Lang::En); // de-DE
        assert_eq!(lang_for_ui_language(Some(0x0419)), Lang::Ru);
        // Primary-language mask: any sublanguage of Russian is Russian.
        assert_eq!(lang_for_ui_language(Some(0x0019)), Lang::Ru);
    }

    #[test]
    fn codes_round_trip() {
        for l in Lang::ALL {
            assert_eq!(Lang::from_code(l.code()), Some(l));
            assert_eq!(format!("{l}"), l.native_name());
        }
        assert_eq!(Lang::from_code("fr"), None);
        assert_eq!(Lang::from_code("RU"), Some(Lang::Ru));
        assert_eq!(Lang::from_code("en-US"), Some(Lang::En));
        assert_eq!(Lang::from_code(" russian "), Some(Lang::Ru));
    }

    #[test]
    fn serde_uses_folder_codes() {
        assert_eq!(serde_json::to_string(&Lang::Ru).unwrap(), "\"ru\"");
        assert_eq!(serde_json::to_string(&Lang::En).unwrap(), "\"en\"");
        let l: Lang = serde_json::from_str("\"en\"").unwrap();
        assert_eq!(l, Lang::En);
        assert!(serde_json::from_str::<Lang>("\"fr\"").is_err());
    }

    #[test]
    fn tr_falls_back_to_russian_and_follows_the_thread_language() {
        with_lang(Lang::Ru, || assert_eq!(tr("Настройки"), "Настройки"));
        with_lang(Lang::En, || {
            let unknown = "нет такой строки в каталоге";
            assert_eq!(tr(unknown), unknown);
            assert_eq!(
                tr_or("нет такой строки в каталоге", "no such string"),
                "no such string"
            );
            assert_eq!(app_name(), "Shorashim");
            assert!(window_title().starts_with("Shorashim"));
            let n = 3;
            assert_eq!(trf!("Урок {n}", "Lesson {n}"), "Lesson 3");
            assert_eq!(trf!("{} из {}", "{} of {}", 1, 2), "1 of 2");
        });
        with_lang(Lang::Ru, || {
            assert_eq!(app_name(), "Шорашим");
            assert_eq!(tr_or("Настройки", "Settings"), "Настройки");
        });
    }

    #[test]
    fn saved_preference_wins_over_os_fallback() {
        // Pure pick(): never touches the process language (parallel tests).
        assert_eq!(
            pick(Some(Lang::Ru), None, Lang::En),
            Lang::Ru,
            "saved Russian wins over an English OS"
        );
        assert_eq!(
            pick(Some(Lang::En), None, Lang::Ru),
            Lang::En,
            "saved English wins over a Russian OS"
        );
        assert_eq!(
            pick(None, Some(Lang::En), Lang::Ru),
            Lang::En,
            "SHORASHIM_LANG wins over both"
        );
        assert_eq!(
            pick(Some(Lang::Ru), Some(Lang::En), Lang::Ru),
            Lang::En,
            "SHORASHIM_LANG wins over a saved profile too"
        );
        assert_eq!(
            pick(None, None, Lang::En),
            Lang::En,
            "first launch with no profile uses the OS fallback"
        );
        assert!(!persist_detected(Some(Lang::En)), "saved choice is kept");
        assert!(!persist_detected(Some(Lang::Ru)));
    }

    #[test]
    fn env_override_parses_the_same_codes() {
        assert_eq!(Lang::from_code("en"), Some(Lang::En));
        assert_eq!(Lang::from_code("ru"), Some(Lang::Ru));
        assert_eq!(Lang::from_code(""), None);
        assert_eq!(Lang::from_code("de"), None);
    }

    #[test]
    fn catalog_is_a_string_map() {
        // Empty catalog (UI agent fills it) is valid; a broken file would
        // have failed to parse in catalog().
        for (k, v) in catalog() {
            assert!(!k.is_empty(), "empty Russian key in en.json");
            assert!(!v.is_empty(), "empty English for «{k}»");
        }
    }

    #[test]
    fn catalog_covers_every_tr_call() {
        let cat = catalog();
        if cat.is_empty() {
            // UI agent owns en.json; nothing to check until it is filled.
            return;
        }
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut missing = Vec::new();
        visit_rs(&root, &mut |text| {
            for lit in tr_literals(&strip_test_blocks(text)) {
                if !cat.contains_key(&lit) {
                    missing.push(lit);
                }
            }
        });
        missing.sort();
        missing.dedup();
        assert!(
            missing.is_empty(),
            "src/i18n/en.json is missing English for: {missing:?}"
        );
    }

    fn visit_rs(dir: &std::path::Path, f: &mut impl FnMut(&str)) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                visit_rs(&p, f);
            } else if p.extension().and_then(|x| x.to_str()) == Some("rs") {
                if let Ok(text) = std::fs::read_to_string(&p) {
                    f(&text);
                }
            }
        }
    }

    fn strip_test_blocks(src: &str) -> String {
        let mut out = String::with_capacity(src.len());
        let mut rest = src;
        while let Some(at) = rest.find("#[cfg(test)]") {
            out.push_str(&rest[..at]);
            rest = &rest[at + "#[cfg(test)]".len()..];
            if let Some(brace) = rest.find('{') {
                let mut depth = 0u32;
                let mut i = brace;
                for (off, c) in rest[brace..].char_indices() {
                    match c {
                        '{' => depth += 1,
                        '}' => {
                            depth -= 1;
                            if depth == 0 {
                                i = brace + off + 1;
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                rest = &rest[i..];
            }
        }
        out.push_str(rest);
        out
    }

    fn tr_literals(src: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut rest = src;
        while let Some(at) = rest.find("tr(\"") {
            // `tr(` itself, not `str(` / `from_str(`.
            let word = rest[..at]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric() || c == '_');
            rest = &rest[at + 4..];
            let mut s = String::new();
            let mut chars = rest.chars();
            while let Some(c) = chars.next() {
                if c == '\\' {
                    match chars.next() {
                        Some('n') => s.push('\n'),
                        Some('t') => s.push('\t'),
                        Some('\n') | Some('\r') => {
                            // Line continuation: skip the next line's indent.
                            let t = chars.as_str().trim_start();
                            chars = t.chars();
                        }
                        Some(n) => s.push(n),
                        None => {}
                    }
                    continue;
                }
                if c == '"' {
                    break;
                }
                s.push(c);
            }
            if !word
                && s.chars().any(|c| {
                    ('а'..='я').contains(&c.to_lowercase().next().unwrap_or(c))
                        || c == 'ё'
                        || c == 'Ё'
                })
            {
                out.push(s);
            }
            rest = chars.as_str();
        }
        out
    }
}
