//! Separate progress for each movement («течение»).
//!
//! Up to v0.5.5 the progress was one for all movements: lessons, answers,
//! tests, «Маалот» records, cards, XP, badges and the collection were keyed
//! by item ids only, and the movement only filtered what is shown (the
//! nested levels of the PWA: «Прогресс сохранится; уроки и вопросы другого
//! уровня появятся или скроются»). So what was done under the Orthodox
//! movement also counted under the Reform one — by design of the old
//! spec, but not what the owner wants (t161u).
//!
//! Now each movement has its own track. The progress of the movement in
//! use stays where it always was (the top-level fields of [`State`]), so
//! every screen and the PWA file format are unchanged; the other
//! movements' tracks are kept under `State::extra["tracks"]` (movement id →
//! the same fields). Switching the movement ([`switch`]) puts the current
//! progress away under its movement and brings back the new movement's own
//! progress (empty the first time). An existing file needs no migration:
//! its progress already belongs to the movement selected in it.
//!
//! Settings (theme, font, music, calendar, …), the prayers' display and
//! favourites, and the first-run flag are not progress and are shared.

use serde_json::{Map, Value};

use crate::storage::{Level, State};

/// `State::extra` key of the other movements' tracks.
pub const KEY: &str = "tracks";

/// Top-level progress fields of a track (the stored, camelCase names).
pub const FIELDS: &[&str] = &[
    "lessonsRead",
    "cards",
    "questions",
    "exams",
    "examHistory",
    "best",
    "activity",
    "xp",
    "xpTotal",
    "goalDays",
    "freezeDays",
    "freezes",
    "freezeAwardDay",
    "badges",
    "counters",
    "goalCelebrated",
    "studyMode",
    // Kept in `extra`: the collection objects and the interview trainer.
    crate::collection::KEY,
    crate::interview::KEY,
];

/// Stored id of a movement.
pub fn id(level: Level) -> &'static str {
    match level {
        Level::Reform => "reform",
        Level::Conservative => "conservative",
        Level::Orthodox => "orthodox",
    }
}

/// The movement the progress in use belongs to (orthodox until one is
/// chosen, as `content::Library::user_level`).
pub fn current(state: &State) -> Level {
    state.settings.level.unwrap_or(Level::Orthodox)
}

fn to_value(state: &State) -> Map<String, Value> {
    match serde_json::to_value(state) {
        Ok(Value::Object(m)) => m,
        _ => Map::new(),
    }
}

fn from_value(m: Map<String, Value>, fallback: &State) -> State {
    serde_json::from_value(Value::Object(m)).unwrap_or_else(|_| fallback.clone())
}

/// The progress in use, as a track object.
fn take(state: &State) -> Map<String, Value> {
    let all = to_value(state);
    FIELDS
        .iter()
        .filter_map(|k| all.get(*k).map(|v| (k.to_string(), v.clone())))
        .collect()
}

/// Replace the progress in use with `track` (missing fields: empty).
fn put(state: &mut State, track: &Map<String, Value>) {
    let mut all = to_value(state);
    let empty = to_value(&State::default());
    for k in FIELDS {
        match track.get(*k).or_else(|| empty.get(*k)) {
            Some(v) => {
                all.insert(k.to_string(), v.clone());
            }
            None => {
                all.remove(*k);
            }
        }
    }
    *state = from_value(all, state);
}

/// Whether the progress in use is empty (nothing done in this movement).
/// A fresh track has only the Torah scroll in its collection.
pub fn is_empty(state: &State) -> bool {
    let mut fresh = State::default();
    crate::collection::sync(&mut fresh, "");
    let mut mine = take(state);
    let mut theirs = take(&fresh);
    // The day the scroll was added does not matter.
    for t in [&mut mine, &mut theirs] {
        if let Some(Value::Object(c)) = t.get_mut(crate::collection::KEY) {
            for v in c.values_mut() {
                *v = Value::Null;
            }
        }
    }
    mine == theirs
}

/// Choose the movement `to`: the progress of the movement in use is put
/// away, `to`'s own progress comes back. First choice (no movement yet):
/// the progress in use simply becomes `to`'s. A new track starts with the
/// Torah scroll in its collection (`today` = its day key).
pub fn switch(state: &mut State, to: Level, today: &str) {
    let Some(from) = state.settings.level else {
        state.settings.level = Some(to);
        crate::collection::sync(state, today);
        return;
    };
    if from == to {
        return;
    }
    let mine = take(state);
    let mut tracks = match state.extra.remove(KEY) {
        Some(Value::Object(m)) => m,
        _ => Map::new(),
    };
    let theirs = match tracks.remove(id(to)) {
        Some(Value::Object(m)) => m,
        _ => Map::new(),
    };
    tracks.insert(id(from).to_string(), Value::Object(mine));
    put(state, &theirs);
    state.settings.level = Some(to);
    if !tracks.is_empty() {
        state.extra.insert(KEY.to_string(), Value::Object(tracks));
    }
    crate::collection::sync(state, today);
}

/// What «Сбросить прохождение» clears.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reset {
    /// Only the movement in use.
    Current,
    /// Every movement.
    All,
}

/// Clear the progress (settings, the movement choice and the first-run flag
/// are kept). The cleared track starts again with the Torah scroll.
pub fn reset(state: &mut State, what: Reset, today: &str) {
    put(state, &Map::new());
    if what == Reset::All {
        state.extra.remove(KEY);
    }
    crate::collection::sync(state, today);
}

/// Movements that have progress stored aside (not the one in use).
pub fn stored(state: &State) -> Vec<Level> {
    let Some(Value::Object(m)) = state.extra.get(KEY) else {
        return Vec::new();
    };
    [Level::Reform, Level::Conservative, Level::Orthodox]
        .into_iter()
        .filter(|l| *l != current(state) && m.contains_key(id(*l)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day() -> chrono::NaiveDate {
        chrono::NaiveDate::from_ymd_opt(2026, 11, 23).unwrap()
    }

    fn orthodox_with_progress() -> State {
        let lib = crate::content::Library::current();
        let mut s = State {
            onboarded: true,
            ..Default::default()
        };
        s.settings.level = Some(Level::Orthodox);
        s.settings.theme = "leather".into();
        s.lessons_read
            .insert("osnovy-1".into(), "2026-11-23".into());
        s.best.insert("osnovy".into(), 90.0);
        s.counters.insert(crate::ladder::BEST_KEY.into(), 7);
        crate::game::award(&lib, &mut s, 25, day());
        crate::collection::sync(&mut s, "2026-11-23");
        let mut prefs = crate::prayers::Prefs::load(&s);
        prefs.toggle_favorite("modeh-ani");
        prefs.store(&mut s);
        s
    }

    #[test]
    fn each_movement_has_its_own_progress() {
        let mut s = orthodox_with_progress();
        let before = take(&s);
        assert!(!is_empty(&s));
        switch(&mut s, Level::Reform, "2026-11-23");
        // Reform starts empty …
        assert!(is_empty(&s), "{:?}", take(&s));
        assert_eq!(s.xp_total, 0);
        assert!(s.lessons_read.is_empty() && s.best.is_empty() && s.badges.is_empty());
        assert_eq!(crate::ladder::best(&s), 0);
        assert_eq!(crate::collection::count(&s), 1, "only the Torah scroll");
        assert!(crate::collection::is_unlocked(&s, "torah-scroll"));
        // … settings, favourites and the first-run flag are shared.
        assert_eq!(s.settings.theme, "leather");
        assert!(s.onboarded);
        assert!(crate::prayers::Prefs::load(&s).is_favorite("modeh-ani"));
        assert_eq!(stored(&s), vec![Level::Orthodox]);
        // Reform progress of its own.
        s.lessons_read
            .insert("osnovy-2".into(), "2026-11-23".into());
        switch(&mut s, Level::Orthodox, "2026-11-23");
        assert_eq!(take(&s), before, "Orthodox comes back exactly");
        switch(&mut s, Level::Reform, "2026-11-23");
        assert!(s.lessons_read.contains_key("osnovy-2"));
        assert!(!s.lessons_read.contains_key("osnovy-1"));
    }

    #[test]
    fn existing_progress_belongs_to_the_selected_movement() {
        // A v0.5.5 file: no tracks; its progress is the selected movement's.
        let s = orthodox_with_progress();
        let text = serde_json::to_string(&s).unwrap();
        let mut back = State::import_json(&text, "2026-11-23").unwrap();
        assert_eq!(take(&back), take(&s));
        switch(&mut back, Level::Conservative, "2026-11-23");
        switch(&mut back, Level::Orthodox, "2026-11-23");
        assert_eq!(take(&back), take(&s));
        // Tracks survive save and load.
        switch(&mut back, Level::Reform, "2026-11-23");
        let text = serde_json::to_string(&back).unwrap();
        let mut again = State::import_json(&text, "2026-11-23").unwrap();
        switch(&mut again, Level::Orthodox, "2026-11-23");
        assert_eq!(take(&again), take(&s));
    }

    #[test]
    fn reset_current_or_all() {
        let mut s = orthodox_with_progress();
        switch(&mut s, Level::Reform, "2026-11-23");
        s.lessons_read
            .insert("osnovy-2".into(), "2026-11-23".into());
        reset(&mut s, Reset::Current, "2026-11-23");
        assert!(is_empty(&s));
        assert_eq!(s.settings.level, Some(Level::Reform));
        assert_eq!(s.settings.theme, "leather");
        assert!(s.onboarded);
        switch(&mut s, Level::Orthodox, "2026-11-23");
        assert!(
            s.lessons_read.contains_key("osnovy-1"),
            "other movement kept"
        );
        reset(&mut s, Reset::All, "2026-11-23");
        assert!(is_empty(&s));
        assert!(stored(&s).is_empty());
        assert!(crate::prayers::Prefs::load(&s).is_favorite("modeh-ani"));
        switch(&mut s, Level::Reform, "2026-11-23");
        assert!(is_empty(&s));
    }
}
