//! Persistence ported from js/storage.js.
//! The on-disk JSON uses the exact same field names as the PWA export format
//! so progress files are interchangeable between the PWA and the native app.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::util;

/// Progress-file format id. Kept from the Giyur Study days on purpose:
/// files exported by every earlier version (PWA and native) stay importable.
pub const APP_ID: &str = "giyur-study";
pub const SCHEMA: i64 = 1;

/// Study level (js/storage.js settings.level). Levels are cumulative:
/// reform ⊂ conservative ⊂ orthodox (see js/content.js).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Reform,
    Conservative,
    Orthodox,
}

impl Level {
    /// Numeric rank used by the content level filter (reform=1, conservative=2, orthodox=3).
    pub fn rank(self) -> u8 {
        match self {
            Level::Reform => 1,
            Level::Conservative => 2,
            Level::Orthodox => 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// 'reform' | 'conservative' | 'orthodox' (null until onboarding);
    /// unknown strings fall back to None → orthodox (js/content.js userLevel)
    #[serde(default, deserialize_with = "de_level_opt")]
    pub level: Option<Level>,
    /// 'dark' | 'leather' (the light theme «Кожа»; the old value
    /// 'parchment' is mapped to 'leather' on load)
    #[serde(default = "default_theme")]
    pub theme: String,
    /// 0..3
    #[serde(default = "default_font_step")]
    pub font_step: i64,
    #[serde(default)]
    pub unlock_all: bool,
    #[serde(default = "default_true")]
    pub translit: bool,
    /// timed exam: 20 | 30 s per question
    #[serde(default = "default_timed_seconds")]
    pub timed_seconds: i64,
    /// timed exam: WebAudio beeps
    #[serde(default)]
    pub timed_sound: bool,
    /// timed exam: auto-advance after reveal
    #[serde(default = "default_true")]
    pub timed_auto: bool,
    /// daily XP goal: 10 | 20 | 30 | 50
    #[serde(default = "default_daily_goal")]
    pub daily_goal: i64,
    /// daily local notification time
    #[serde(default = "default_reminder_time")]
    pub reminder_time: String,
    /// Unknown keys kept so a PWA file → native → PWA round-trip loses nothing
    /// (js/storage.js fill keeps them).
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// Tolerant level parsing: unknown strings → None (orthodox fallback).
fn de_level_opt<'de, D>(deserializer: D) -> Result<Option<Level>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = Option::<String>::deserialize(deserializer)?;
    Ok(s.and_then(|s| match s.as_str() {
        "reform" => Some(Level::Reform),
        "conservative" => Some(Level::Conservative),
        "orthodox" => Some(Level::Orthodox),
        _ => None,
    }))
}

/// Stored value of the light theme «Кожа».
pub const LIGHT_THEME: &str = "leather";
/// Pre-0.5 (and PWA) value of the same theme, then called «Пергамент».
pub const LEGACY_LIGHT_THEME: &str = "parchment";

fn default_theme() -> String {
    "dark".to_string()
}
fn default_font_step() -> i64 {
    1
}
fn default_true() -> bool {
    true
}
fn default_timed_seconds() -> i64 {
    30
}
fn default_daily_goal() -> i64 {
    20
}
fn default_reminder_time() -> String {
    "19:00".to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            level: None,
            theme: default_theme(),
            font_step: default_font_step(),
            unlock_all: false,
            translit: true,
            timed_seconds: default_timed_seconds(),
            timed_sound: false,
            timed_auto: true,
            daily_goal: default_daily_goal(),
            reminder_time: default_reminder_time(),
            extra: serde_json::Map::new(),
        }
    }
}

/// SRS state of one card (js/storage.js cards: {l, due, n, miss, m, last}).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CardState {
    /// SRS level 0..=5
    #[serde(default)]
    pub l: i64,
    /// Day key when the card is due
    #[serde(default)]
    pub due: String,
    /// Successful grades
    #[serde(default)]
    pub n: i64,
    /// Misses
    #[serde(default)]
    pub miss: i64,
    /// Mastered flag (set only by «Вспомнил» arriving at level 5)
    #[serde(default)]
    pub m: bool,
    /// Last review day key
    #[serde(default)]
    pub last: String,
}

/// Per-question stats (js/storage.js questions: {n, miss, streak, last}).
/// `miss` is fractional: a partially correct match answer adds 0.5.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QuestionStat {
    #[serde(default)]
    pub n: i64,
    #[serde(default)]
    pub miss: f64,
    #[serde(default)]
    pub streak: i64,
    #[serde(default)]
    pub last: String,
}

fn default_schema() -> i64 {
    SCHEMA
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    #[serde(default = "default_app")]
    pub app: String,
    #[serde(default = "default_schema")]
    pub schema: i64,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub onboarded: bool,
    #[serde(default)]
    pub settings: Settings,
    /// lessonId -> dayKey
    #[serde(default)]
    pub lessons_read: HashMap<String, String>,
    /// cardId -> SRS state
    #[serde(default)]
    pub cards: HashMap<String, CardState>,
    #[serde(default)]
    pub cards_help_seen: bool,
    /// questionId -> stats
    #[serde(default)]
    pub questions: HashMap<String, QuestionStat>,
    /// Exam history, newest last (records produced by js/exam.js)
    #[serde(default)]
    pub exams: Vec<serde_json::Value>,
    /// Timed exam: question ids of the last 3 runs
    #[serde(default)]
    pub exam_history: Vec<Vec<String>>,
    /// sectionId -> best section test %
    #[serde(default)]
    pub best: HashMap<String, f64>,
    /// dayKey -> number of actions
    #[serde(default)]
    pub activity: HashMap<String, i64>,
    /// dayKey -> XP earned that day
    #[serde(default)]
    pub xp: HashMap<String, i64>,
    #[serde(default)]
    pub xp_total: i64,
    /// dayKey -> true: daily goal reached
    #[serde(default)]
    pub goal_days: HashMap<String, bool>,
    /// dayKey -> true: missed day covered by a streak freeze
    #[serde(default)]
    pub freeze_days: HashMap<String, bool>,
    /// stored streak freezes (0..2)
    #[serde(default)]
    pub freezes: i64,
    #[serde(default)]
    pub freeze_award_day: Option<String>,
    /// badgeId -> dayKey unlocked
    #[serde(default)]
    pub badges: HashMap<String, String>,
    /// decks, beitdin, perfectTimed, bestStreak
    #[serde(default)]
    pub counters: HashMap<String, i64>,
    #[serde(default)]
    pub goal_celebrated: Option<String>,
    /// sectionId -> "read" | "prep": last study mode chosen in that section.
    #[serde(default)]
    pub study_mode: HashMap<String, String>,
    /// Unknown top-level keys kept for round-trip fidelity (js/storage.js fill).
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

fn default_app() -> String {
    APP_ID.to_string()
}

impl Default for State {
    fn default() -> Self {
        State {
            app: default_app(),
            schema: SCHEMA,
            created_at: chrono::Local::now().timestamp_millis(),
            onboarded: false,
            settings: Settings::default(),
            lessons_read: HashMap::new(),
            cards: HashMap::new(),
            cards_help_seen: false,
            questions: HashMap::new(),
            exams: Vec::new(),
            exam_history: Vec::new(),
            best: HashMap::new(),
            activity: HashMap::new(),
            xp: HashMap::new(),
            xp_total: 0,
            goal_days: HashMap::new(),
            freeze_days: HashMap::new(),
            freezes: 0,
            freeze_award_day: None,
            badges: HashMap::new(),
            counters: HashMap::new(),
            goal_celebrated: None,
            study_mode: HashMap::new(),
            extra: serde_json::Map::new(),
        }
    }
}

/// js/storage.js looksValid
fn looks_valid(v: &serde_json::Value) -> bool {
    let Some(obj) = v.as_object() else {
        return false;
    };
    obj.get("app").and_then(|a| a.as_str()) == Some(APP_ID)
        || obj.get("settings").map(|s| s.is_object()).unwrap_or(false)
}

/// Repair/clamp fields after loading, mirroring js/storage.js fill().
fn fill(s: &mut State) {
    // «Пергамент» was renamed «Кожа» (v0.5); keep old files and PWA exports.
    if s.settings.theme == LEGACY_LIGHT_THEME {
        s.settings.theme = LIGHT_THEME.to_string();
    }
    s.settings.font_step = util::clamp(s.settings.font_step, 0, 3);
    if ![10, 20, 30, 50].contains(&s.settings.daily_goal) {
        s.settings.daily_goal = 20;
    }
    s.freezes = util::clamp(s.freezes, 0, 2);
    s.xp_total = s.xp_total.max(0);
    s.schema = SCHEMA;
}

/// Map fields that must be JSON objects (js/storage.js importJSON list).
const OBJECT_FIELDS: &[&str] = &[
    "lessonsRead",
    "studyMode",
    "cards",
    "questions",
    "best",
    "activity",
    "xp",
    "goalDays",
    "freezeDays",
    "badges",
    "counters",
];

/// Expected JSON shape of a field for the fill()-style type check.
#[derive(Clone, Copy)]
enum Kind {
    Bool,
    Int,
    Str,
    Arr,
    NullableStr,
}

fn kind_ok(v: &serde_json::Value, k: Kind) -> bool {
    match k {
        Kind::Bool => v.is_boolean(),
        Kind::Int => v.as_i64().is_some(),
        Kind::Str => v.is_string(),
        Kind::Arr => v.is_array(),
        Kind::NullableStr => v.is_null() || v.is_string(),
    }
}

/// Top-level fields with their expected types (js/storage.js defaults()).
const TOP_KINDS: &[(&str, Kind)] = &[
    ("app", Kind::Str),
    ("schema", Kind::Int),
    ("createdAt", Kind::Int),
    ("onboarded", Kind::Bool),
    ("cardsHelpSeen", Kind::Bool),
    ("exams", Kind::Arr),
    ("examHistory", Kind::Arr),
    ("xpTotal", Kind::Int),
    ("freezes", Kind::Int),
    ("freezeAwardDay", Kind::NullableStr),
    ("goalCelebrated", Kind::NullableStr),
];

/// settings fields with their expected types (js/storage.js defaults().settings).
const SETTINGS_KINDS: &[(&str, Kind)] = &[
    ("level", Kind::NullableStr),
    ("theme", Kind::Str),
    ("fontStep", Kind::Int),
    ("unlockAll", Kind::Bool),
    ("translit", Kind::Bool),
    ("timedSeconds", Kind::Int),
    ("timedSound", Kind::Bool),
    ("timedAuto", Kind::Bool),
    ("dailyGoal", Kind::Int),
    ("reminderTime", Kind::Str),
];

/// Sanitise an imported JSON value like js/storage.js importJSON, then
/// deserialise with defaults. Returns the Russian error strings of the PWA.
pub fn import_value(v: &mut serde_json::Value, today: &str) -> Result<(), String> {
    if !looks_valid(v) {
        return Err("Это не файл прогресса «Шорашим».".to_string());
    }
    if let Some(schema) = v.get("schema").and_then(|s| s.as_i64()) {
        if schema > SCHEMA {
            return Err("Файл создан более новой версией приложения.".to_string());
        }
    }
    let obj = v.as_object_mut().expect("looks_valid checked object");
    for k in OBJECT_FIELDS {
        if let Some(f) = obj.get(*k) {
            if !f.is_object() {
                obj.remove(*k);
            }
        }
    }
    obj.remove("exportedAt");
    // Saves from before gamification: treat past active days as goal days.
    // js/storage.js migrates only when xp is absent (null is not absent).
    let xp_missing = !obj.contains_key("xp");
    if xp_missing {
        if let Some(activity) = obj.get("activity").and_then(|a| a.as_object()) {
            let mut goal_days = serde_json::Map::new();
            for (k, val) in activity {
                if !val.is_null() && val != &serde_json::Value::from(0) && k.as_str() < today {
                    goal_days.insert(k.clone(), serde_json::Value::Bool(true));
                }
            }
            obj.insert("goalDays".to_string(), serde_json::Value::Object(goal_days));
        }
    }
    // js/storage.js fill(): a null or wrongly typed field falls back to its
    // default instead of failing the whole import. Null-stripping happens
    // after the migration check above.
    if let Some(f) = obj.get("settings") {
        if !f.is_object() {
            obj.remove("settings");
        }
    }
    for (k, kind) in TOP_KINDS {
        if let Some(f) = obj.get(*k) {
            if f.is_null() || !kind_ok(f, *kind) {
                obj.remove(*k);
            }
        }
    }
    if let Some(settings) = obj.get_mut("settings").and_then(|s| s.as_object_mut()) {
        for (k, kind) in SETTINGS_KINDS {
            if let Some(f) = settings.get(*k) {
                if f.is_null() || !kind_ok(f, *kind) {
                    settings.remove(*k);
                }
            }
        }
    }
    // examHistory runs must be string arrays (as recentTimedIds assumes).
    if let Some(serde_json::Value::Array(runs)) = obj.get_mut("examHistory") {
        runs.retain(|r| {
            r.as_array()
                .is_some_and(|a| a.iter().all(|x| x.is_string()))
        });
    }
    Ok(())
}

impl State {
    /// Parse an exported/imported JSON string (js/storage.js importJSON).
    pub fn import_json(text: &str, today: &str) -> Result<State, String> {
        // A UTF-8 BOM (e.g. a file re-saved in Windows Notepad) breaks
        // serde_json but not the PWA's FileReader → JSON.parse path.
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let mut v: serde_json::Value = serde_json::from_str(text)
            .map_err(|_| "Файл не является корректным JSON.".to_string())?;
        import_value(&mut v, today)?;
        let mut s: State = serde_json::from_value(v)
            .map_err(|_| "Это не файл прогресса «Шорашим».".to_string())?;
        fill(&mut s);
        Ok(s)
    }

    /// Export string identical to the PWA format (js/storage.js exportJSON).
    pub fn export_json(&self) -> String {
        let mut v = serde_json::to_value(self).unwrap_or(serde_json::Value::Null);
        if let Some(obj) = v.as_object_mut() {
            let mut out = serde_json::Map::new();
            out.insert(
                "exportedAt".to_string(),
                serde_json::Value::String(chrono::Local::now().to_rfc3339()),
            );
            for (k, val) in std::mem::take(obj) {
                out.insert(k, val);
            }
            v = serde_json::Value::Object(out);
        }
        serde_json::to_string_pretty(&v).unwrap_or_default()
    }

    /// Record any learning activity for the streak (js/storage.js touch).
    pub fn touch(&mut self, today: &str) {
        *self.activity.entry(today.to_string()).or_insert(0) += 1;
    }

    /// Reset progress, keeping theme and fontStep (js/storage.js reset).
    pub fn reset(&mut self) {
        let theme = self.settings.theme.clone();
        let font_step = self.settings.font_step;
        *self = State::default();
        self.settings.theme = theme;
        self.settings.font_step = font_step;
    }
}

/// Folder name under config_dir() (%APPDATA% on Windows).
pub const DATA_DIR: &str = "Shorashim";
/// Folder used before the rename to Shorashim (v0.1.x).
pub const LEGACY_DATA_DIR: &str = "GiyurStudy";

/// Default progress file path: config_dir()/Shorashim/progress.json
/// (%APPDATA%\Shorashim\progress.json on Windows). Progress saved by
/// v0.1.x under GiyurStudy is copied over on first start.
pub fn default_path() -> PathBuf {
    let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    let path = base.join(DATA_DIR).join("progress.json");
    migrate_legacy(&base.join(LEGACY_DATA_DIR).join("progress.json"), &path);
    path
}

/// Copy the pre-rename progress file to the new location if the new one does
/// not exist yet. The old file is left in place, so nothing can be lost.
/// Returns true when a copy was made.
pub fn migrate_legacy(old: &Path, new: &Path) -> bool {
    if new.exists() || !old.is_file() {
        return false;
    }
    if let Some(dir) = new.parent() {
        if std::fs::create_dir_all(dir).is_err() {
            return false;
        }
    }
    std::fs::copy(old, new).is_ok()
}

/// Load state from a file. A corrupt file is kept as <path>.bak and a fresh
/// state is returned.
pub fn load(path: &Path, today: &str) -> State {
    load_checked(path, today).0
}

/// Load state from a file, reporting whether it is safe for the caller to
/// overwrite the file afterwards. The bool is false when an existing file
/// could not be read at all (permission denied, invalid UTF-8, locked): the
/// real progress may still be there, so the caller must not save over it.
/// A missing file or a parseable-but-corrupt file (kept as <path>.bak) is
/// safe to overwrite.
pub fn load_checked(path: &Path, today: &str) -> (State, bool) {
    let raw = match std::fs::read_to_string(path) {
        Ok(r) => r,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return (State::default(), true);
        }
        Err(_) => {
            let bak = path.with_extension("json.bak");
            let _ = std::fs::copy(path, &bak);
            return (State::default(), false);
        }
    };
    match State::import_json(&raw, today) {
        Ok(s) => (s, true),
        Err(_) => {
            let bak = path.with_extension("json.bak");
            let _ = std::fs::copy(path, &bak);
            (State::default(), true)
        }
    }
}

/// Atomic save: write to a temp file, fsync, then rename.
pub fn save(path: &Path, state: &State) -> std::io::Result<()> {
    use std::io::Write;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    let data = serde_json::to_string_pretty(state).map_err(std::io::Error::other)?;
    {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(data.as_bytes())?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TODAY: &str = "2026-11-22";

    #[test]
    fn defaults_match_pwa() {
        let s = State::default();
        assert_eq!(s.app, "giyur-study");
        assert_eq!(s.schema, 1);
        assert!(!s.onboarded);
        assert_eq!(s.settings.theme, "dark");
        assert_eq!(s.settings.font_step, 1);
        assert!(s.settings.translit);
        assert_eq!(s.settings.timed_seconds, 30);
        assert!(s.settings.timed_auto);
        assert_eq!(s.settings.daily_goal, 20);
        assert_eq!(s.settings.reminder_time, "19:00");
        assert_eq!(s.freezes, 0);
    }

    #[test]
    fn roundtrip() {
        let mut s = State {
            onboarded: true,
            ..State::default()
        };
        s.settings.level = Some(Level::Orthodox);
        s.xp_total = 123;
        s.cards.insert(
            "c1".to_string(),
            CardState {
                l: 3,
                due: "2026-11-25".into(),
                n: 4,
                miss: 1,
                m: false,
                last: TODAY.into(),
            },
        );
        s.goal_days.insert("2026-11-21".to_string(), true);
        s.badges.insert("first-steps".to_string(), TODAY.into());
        let text = s.export_json();
        let back = State::import_json(&text, TODAY).unwrap();
        assert!(back.onboarded);
        assert_eq!(back.settings.level, Some(Level::Orthodox));
        assert_eq!(back.xp_total, 123);
        assert_eq!(back.cards["c1"].l, 3);
        assert_eq!(back.cards["c1"].due, "2026-11-25");
        assert!(back.goal_days["2026-11-21"]);
        assert_eq!(back.badges["first-steps"], TODAY);
    }

    #[test]
    fn export_has_pwa_field_names() {
        let s = State::default();
        let text = s.export_json();
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        let obj = v.as_object().unwrap();
        assert!(obj.contains_key("exportedAt"));
        assert!(obj.contains_key("createdAt"));
        assert!(obj.contains_key("lessonsRead"));
        assert!(obj.contains_key("cardsHelpSeen"));
        assert!(obj.contains_key("examHistory"));
        assert!(obj.contains_key("xpTotal"));
        assert!(obj.contains_key("goalDays"));
        assert!(obj.contains_key("freezeDays"));
        assert!(obj.contains_key("freezeAwardDay"));
        assert!(obj.contains_key("goalCelebrated"));
        let settings = obj["settings"].as_object().unwrap();
        assert!(settings.contains_key("fontStep"));
        assert!(settings.contains_key("unlockAll"));
        assert!(settings.contains_key("timedSeconds"));
        assert!(settings.contains_key("timedSound"));
        assert!(settings.contains_key("timedAuto"));
        assert!(settings.contains_key("dailyGoal"));
        assert!(settings.contains_key("reminderTime"));
    }

    #[test]
    fn import_pwa_sample() {
        // A small sample like the PWA «Экспорт» file.
        let sample = r#"{
          "exportedAt": "2026-11-20T10:00:00.000Z",
          "app": "giyur-study",
          "schema": 1,
          "createdAt": 1763000000000,
          "onboarded": true,
          "settings": {
            "level": "conservative",
            "theme": "parchment",
            "fontStep": 2,
            "dailyGoal": 30,
            "timedSeconds": 20
          },
          "lessonsRead": {"s01-l01": "2026-11-19"},
          "cards": {"c9": {"l": 5, "due": "2026-11-30", "n": 6, "miss": 0, "m": true, "last": "2026-11-19"}},
          "questions": {"q1": {"n": 3, "miss": 1, "streak": 2, "last": "2026-11-19"}},
          "xp": {"2026-11-19": 25},
          "xpTotal": 25,
          "goalDays": {"2026-11-19": true},
          "freezes": 1,
          "badges": {"first-steps": "2026-11-19"},
          "counters": {"decks": 2}
        }"#;
        let s = State::import_json(sample, TODAY).unwrap();
        assert_eq!(s.settings.level, Some(Level::Conservative));
        // The old «Пергамент» value opens as «Кожа».
        assert_eq!(s.settings.theme, "leather");
        assert_eq!(s.settings.font_step, 2);
        assert_eq!(s.settings.daily_goal, 30);
        assert_eq!(s.settings.timed_seconds, 20);
        // Missing settings get PWA defaults.
        assert!(s.settings.translit);
        assert!(s.cards["c9"].m);
        assert_eq!(s.questions["q1"].streak, 2);
        assert_eq!(s.freezes, 1);
        assert_eq!(s.counters["decks"], 2);
    }

    #[test]
    fn import_rejects_garbage_and_foreign() {
        assert!(State::import_json("not json", TODAY).is_err());
        assert!(State::import_json("{\"foo\": 1}", TODAY).is_err());
        assert!(State::import_json("[1,2]", TODAY).is_err());
        let newer = r#"{"app":"giyur-study","schema":99}"#;
        assert!(State::import_json(newer, TODAY).is_err());
    }

    #[test]
    fn import_repairs_and_clamps() {
        let sample = r#"{
          "app": "giyur-study",
          "settings": {"fontStep": 9, "dailyGoal": 17},
          "cards": [1,2,3],
          "freezes": 7,
          "xpTotal": -5
        }"#;
        let s = State::import_json(sample, TODAY).unwrap();
        assert_eq!(s.settings.font_step, 3);
        assert_eq!(s.settings.daily_goal, 20);
        assert!(s.cards.is_empty()); // non-object field dropped
        assert_eq!(s.freezes, 2);
        assert_eq!(s.xp_total, 0);
    }

    #[test]
    fn legacy_activity_becomes_goal_days() {
        // Pre-gamification save: has activity but no xp.
        let sample = r#"{
          "app": "giyur-study",
          "settings": {},
          "activity": {"2026-11-20": 3, "2026-11-22": 1, "2026-11-21": 0}
        }"#;
        let s = State::import_json(sample, TODAY).unwrap();
        assert!(s.goal_days["2026-11-20"]); // past active day counts
        assert!(!s.goal_days.get("2026-11-22").unwrap_or(&false)); // today not counted
        assert!(!s.goal_days.get("2026-11-21").unwrap_or(&false)); // zero activity not counted
    }

    #[test]
    fn touch_increments_activity() {
        let mut s = State::default();
        s.touch(TODAY);
        s.touch(TODAY);
        assert_eq!(s.activity[TODAY], 2);
    }

    #[test]
    fn reset_keeps_theme_and_font() {
        let mut s = State::default();
        s.settings.theme = "leather".into();
        s.settings.font_step = 3;
        s.xp_total = 500;
        s.reset();
        assert_eq!(s.settings.theme, "leather");
        assert_eq!(s.settings.font_step, 3);
        assert_eq!(s.xp_total, 0);
    }

    #[test]
    fn corrupt_file_kept_as_bak() {
        let dir = std::env::temp_dir().join(format!("giyur-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("progress.json");
        std::fs::write(&path, "{corrupt!!!").unwrap();
        let s = load(&path, TODAY);
        assert_eq!(s.xp_total, 0);
        let bak = path.with_extension("json.bak");
        assert!(bak.exists());
        // Now save and reload.
        let s2 = State {
            xp_total: 42,
            ..State::default()
        };
        save(&path, &s2).unwrap();
        let s3 = load(&path, TODAY);
        assert_eq!(s3.xp_total, 42);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn level_serde_names() {
        let s = serde_json::to_string(&Level::Reform).unwrap();
        assert_eq!(s, "\"reform\"");
        let l: Level = serde_json::from_str("\"orthodox\"").unwrap();
        assert_eq!(l, Level::Orthodox);
        assert_eq!(Level::Reform.rank(), 1);
        assert_eq!(Level::Conservative.rank(), 2);
        assert_eq!(Level::Orthodox.rank(), 3);
    }

    #[test]
    fn import_repairs_null_and_wrongly_typed_fields() {
        // js/storage.js fill() repairs field by field instead of failing.
        let sample = r#"{
          "app": "giyur-study",
          "settings": null,
          "xp": null,
          "onboarded": null,
          "exams": {},
          "examHistory": [[1,2], ["q1","q2"], "oops"]
        }"#;
        let s = State::import_json(sample, TODAY).unwrap();
        assert_eq!(s.settings.theme, "dark");
        assert!(s.xp.is_empty());
        assert!(!s.onboarded);
        assert!(s.exams.is_empty());
        assert_eq!(
            s.exam_history,
            vec![vec!["q1".to_string(), "q2".to_string()]]
        );
        // Null xp is not "missing": no legacy migration (js: xp === undefined).
        assert!(s.goal_days.is_empty());

        let odd = r#"{
          "app": "giyur-study",
          "settings": {"level": "", "fontStep": "big", "theme": 5},
          "xpTotal": 1.5
        }"#;
        let s = State::import_json(odd, TODAY).unwrap();
        assert_eq!(s.settings.level, None); // odd level → orthodox fallback
        assert_eq!(s.settings.font_step, 1);
        assert_eq!(s.settings.theme, "dark");
        assert_eq!(s.xp_total, 0);

        let unknown_level = r#"{"app":"giyur-study","settings":{"level":"reconstructionist"}}"#;
        let s = State::import_json(unknown_level, TODAY).unwrap();
        assert_eq!(s.settings.level, None);
    }

    #[test]
    fn import_tolerates_bom() {
        let sample = "\u{feff}{\"app\":\"giyur-study\",\"xpTotal\":7}";
        let s = State::import_json(sample, TODAY).unwrap();
        assert_eq!(s.xp_total, 7);
    }

    #[test]
    fn unknown_keys_round_trip() {
        // js/storage.js fill() keeps unknown keys; the export must too.
        let sample = r#"{
          "app": "giyur-study",
          "futureField": {"a": 1},
          "settings": {"futureSetting": 42, "level": "reform"}
        }"#;
        let s = State::import_json(sample, TODAY).unwrap();
        assert_eq!(s.settings.level, Some(Level::Reform));
        let text = s.export_json();
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(v["futureField"], serde_json::json!({"a": 1}));
        assert_eq!(v["settings"]["futureSetting"], serde_json::json!(42));
    }

    #[test]
    fn load_checked_flags_unreadable_file() {
        let dir = std::env::temp_dir().join(format!("giyur-test-checked-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // Missing file: safe to create/overwrite.
        let missing = dir.join("progress.json");
        let (s, ok) = load_checked(&missing, TODAY);
        assert!(ok);
        assert_eq!(s.xp_total, 0);
        // A directory is not a readable file: exists but unreadable → false.
        let (s, ok) = load_checked(&dir, TODAY);
        assert!(!ok);
        assert_eq!(s.xp_total, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn migrates_legacy_progress_once() {
        let dir = std::env::temp_dir().join(format!("shorashim-migrate-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let old = dir.join("GiyurStudy").join("progress.json");
        let new = dir.join("Shorashim").join("progress.json");
        std::fs::create_dir_all(old.parent().unwrap()).unwrap();
        std::fs::write(&old, r#"{"app":"giyur-study","xpTotal":42}"#).unwrap();
        assert!(migrate_legacy(&old, &new));
        assert_eq!(load(&new, "2026-11-22").xp_total, 42);
        assert!(old.exists(), "old file is kept");
        std::fs::write(&old, r#"{"app":"giyur-study","xpTotal":1}"#).unwrap();
        assert!(!migrate_legacy(&old, &new), "never overwrites the new file");
        assert_eq!(load(&new, "2026-11-22").xp_total, 42);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
