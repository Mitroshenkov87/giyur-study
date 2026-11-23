//! «Молитвы»: daily prayers and blessings for someone who does not (yet)
//! read Hebrew, embedded from `content/ru/prayers.json`. Every text has pointed
//! Hebrew, a Russian transliteration and a Russian translation, plus when /
//! how / why notes and a «До гиюра» note for candidates (marked «уточните у
//! раввина»). The whole file is listed in docs/RABBI-REVIEW.md.
//!
//! The reader's choices (what to show, text size, favourites) live in
//! `State::extra["prayers"]`, so the progress file schema does not change.

use serde::{Deserialize, Serialize};

use crate::i18n::tr;
use crate::storage::State;

/// One block of a prayer: the same words in three forms.
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Part {
    /// Optional heading («1. Авот — «Отцы»», «Шёпотом», …).
    pub label: Option<String>,
    /// Pointed Hebrew (the Name written as in siddurim: יְיָ).
    pub he: String,
    /// Russian-letter transliteration (Israeli pronunciation).
    pub translit: String,
    /// Russian translation (the app's own).
    pub ru: String,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Prayer {
    pub id: String,
    /// Group ids (`morning`, `day`, `evening`, `food`, `shabbat`, `special`);
    /// the first one is the prayer's home group.
    pub groups: Vec<String>,
    pub title: String,
    pub he_title: String,
    pub subtitle: String,
    pub when: String,
    pub how: String,
    pub about: String,
    pub parts: Vec<Part>,
    /// «До гиюра» note; always ends with «Уточните у раввина».
    pub candidate: String,
    /// Where the Hebrew comes from (licence included).
    pub source: String,
    /// Talmud / halacha references.
    pub r#ref: String,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Group {
    pub id: String,
    pub title: String,
    pub hint: String,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Note {
    pub id: String,
    pub title: String,
    pub text: String,
    /// Shown with the «уточните у раввина» mark.
    pub rabbi: bool,
    pub r#ref: String,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct SourceInfo {
    pub id: String,
    pub title: String,
    pub license: String,
    pub url: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Book {
    pub review_status: String,
    pub nusach: String,
    pub sources: Vec<SourceInfo>,
    pub groups: Vec<Group>,
    pub notes: Vec<Note>,
    pub prayers: Vec<Prayer>,
}

impl Book {
    pub fn prayer(&self, id: &str) -> Option<&Prayer> {
        self.prayers.iter().find(|p| p.id == id)
    }

    pub fn group(&self, id: &str) -> Option<&Group> {
        self.groups.iter().find(|g| g.id == id)
    }

    /// Prayers listed under a group, in file order.
    pub fn in_group<'a>(&'a self, group: &'a str) -> impl Iterator<Item = &'a Prayer> + 'a {
        self.prayers
            .iter()
            .filter(move |p| p.groups.iter().any(|g| g == group))
    }

    /// The prayer before / after `id` in file order (for «Назад / Далее»).
    pub fn neighbours(&self, id: &str) -> (Option<&Prayer>, Option<&Prayer>) {
        match self.prayers.iter().position(|p| p.id == id) {
            Some(i) => (
                i.checked_sub(1).and_then(|j| self.prayers.get(j)),
                self.prayers.get(i + 1),
            ),
            None => (None, None),
        }
    }
}

/// The embedded prayer book.
pub fn book() -> &'static Book {
    &crate::content_store::get().prayers
}

// ------------------------------------------------------------------ prefs

/// What the prayer texts show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Display {
    /// Hebrew, transliteration and translation together.
    #[default]
    All,
    /// Russian translation only (to pray in Russian).
    Ru,
    /// Hebrew only.
    He,
    /// Transliteration only (to pray in Hebrew without reading it).
    Translit,
}

impl Display {
    pub const ALL: [Display; 4] = [Display::All, Display::Ru, Display::He, Display::Translit];

    pub fn label(self) -> &'static str {
        match self {
            Display::All => tr("Всё вместе"),
            Display::Ru => tr("Русский"),
            Display::He => tr("Иврит"),
            Display::Translit => tr("Транслитерация"),
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            Display::All => "all",
            Display::Ru => "ru",
            Display::He => "he",
            Display::Translit => "translit",
        }
    }

    pub fn from_key(k: &str) -> Option<Display> {
        Display::ALL.into_iter().find(|d| d.key() == k)
    }

    pub fn shows_he(self) -> bool {
        matches!(self, Display::All | Display::He)
    }
    pub fn shows_translit(self) -> bool {
        matches!(self, Display::All | Display::Translit)
    }
    pub fn shows_ru(self) -> bool {
        matches!(self, Display::All | Display::Ru)
    }
}

/// Text size steps of the prayer reader (on top of the app's font setting).
pub const SIZES: [f32; 5] = [0.85, 1.0, 1.2, 1.45, 1.75];
pub const DEFAULT_SIZE: usize = 1;

/// `State::extra` key.
pub const KEY: &str = "prayers";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    pub display: Display,
    /// Index into [`SIZES`].
    pub size: usize,
    /// Favourite prayer ids, in the order they were starred.
    pub favorites: Vec<String>,
}

impl Default for Prefs {
    fn default() -> Self {
        Prefs {
            display: Display::All,
            size: DEFAULT_SIZE,
            favorites: Vec::new(),
        }
    }
}

impl Prefs {
    /// Read from the progress state (anything malformed → defaults).
    pub fn load(state: &State) -> Prefs {
        let mut p: Prefs = state
            .extra
            .get(KEY)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        p.size = p.size.min(SIZES.len() - 1);
        let book = book();
        p.favorites.retain(|id| book.prayer(id).is_some());
        p.favorites.dedup();
        p
    }

    pub fn store(&self, state: &mut State) {
        if let Ok(v) = serde_json::to_value(self) {
            state.extra.insert(KEY.into(), v);
        }
    }

    pub fn scale(&self) -> f32 {
        SIZES[self.size.min(SIZES.len() - 1)]
    }

    pub fn is_favorite(&self, id: &str) -> bool {
        self.favorites.iter().any(|f| f == id)
    }

    /// Star / unstar; returns the new state.
    pub fn toggle_favorite(&mut self, id: &str) -> bool {
        if let Some(i) = self.favorites.iter().position(|f| f == id) {
            self.favorites.remove(i);
            false
        } else {
            self.favorites.push(id.to_string());
            true
        }
    }

    pub fn bigger(&mut self) {
        self.size = (self.size + 1).min(SIZES.len() - 1);
    }

    pub fn smaller(&mut self) {
        self.size = self.size.saturating_sub(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GROUPS: [&str; 6] = ["morning", "day", "evening", "food", "shabbat", "special"];

    #[test]
    fn book_loads_with_every_group_filled() {
        let b = book();
        assert!(b.prayers.len() >= 20, "{} prayers", b.prayers.len());
        let ids: Vec<_> = b.groups.iter().map(|g| g.id.as_str()).collect();
        assert_eq!(ids, GROUPS);
        for g in GROUPS {
            assert!(b.in_group(g).count() > 0, "empty group {g}");
        }
    }

    #[test]
    fn requested_prayers_are_present() {
        let b = book();
        for id in [
            "modeh-ani",
            "netilat-yadayim",
            "birkot-hashachar",
            "shma",
            "amida-short",
            "hamotzi",
            "mezonot",
            "hagafen",
            "haetz",
            "haadama",
            "shehakol",
            "birkat-hamazon",
            "al-hamichya",
            "bore-nefashot",
            "tefilat-haderech",
            "shma-al-hamita",
            "shehecheyanu",
            "asher-yatzar",
            "shabbat-candles",
        ] {
            assert!(b.prayer(id).is_some(), "missing {id}");
        }
    }

    fn is_hebrew_letter(c: char) -> bool {
        ('\u{05D0}'..='\u{05EA}').contains(&c)
    }
    fn is_nikud(c: char) -> bool {
        ('\u{05B0}'..='\u{05BC}').contains(&c) || matches!(c, '\u{05C1}' | '\u{05C2}' | '\u{05C7}')
    }

    #[test]
    fn every_part_has_pointed_hebrew_translit_and_russian() {
        for p in &book().prayers {
            assert!(!p.parts.is_empty(), "{}", p.id);
            assert!(
                !p.title.is_empty()
                    && !p.when.is_empty()
                    && !p.how.is_empty()
                    && !p.about.is_empty(),
                "{}",
                p.id
            );
            assert!(!p.source.is_empty(), "{} has no source", p.id);
            assert!(GROUPS.contains(&p.groups[0].as_str()), "{}", p.id);
            for part in &p.parts {
                assert!(part.he.chars().any(is_hebrew_letter), "{}: no Hebrew", p.id);
                assert!(
                    part.he.chars().any(is_nikud),
                    "{}: Hebrew without nikud",
                    p.id
                );
                // No cantillation marks or meteg left over from the sources
                // (Noto Serif Hebrew draws them as stray dots).
                assert!(
                    !part
                        .he
                        .chars()
                        .any(|c| ('\u{0591}'..='\u{05AF}').contains(&c) || c == '\u{05BD}'),
                    "{}: cantillation left in",
                    p.id
                );
                // Every pointed Hebrew word, not just some of them.
                for w in part
                    .he
                    .split(|c: char| !(is_hebrew_letter(c) || is_nikud(c) || c == '\u{05BE}'))
                {
                    if w.chars().filter(|c| is_hebrew_letter(*c)).count() > 1 {
                        assert!(w.chars().any(is_nikud), "{}: unpointed word «{w}»", p.id);
                    }
                }
                assert!(
                    part.translit.chars().any(|c| ('а'..='я').contains(&c)),
                    "{}: translit",
                    p.id
                );
                assert!(
                    !part.translit.chars().any(is_hebrew_letter),
                    "{}: Hebrew in translit",
                    p.id
                );
                assert!(
                    part.ru.chars().any(|c| ('а'..='я').contains(&c)),
                    "{}: ru",
                    p.id
                );
            }
        }
    }

    #[test]
    fn the_name_is_written_as_in_siddurim() {
        for p in &book().prayers {
            for part in &p.parts {
                assert!(
                    !part.he.contains("יְהֹוָה") && !part.he.contains("יְהוָה"),
                    "{}",
                    p.id
                );
            }
        }
    }

    #[test]
    fn candidate_notes_send_to_the_rabbi() {
        let b = book();
        for p in &b.prayers {
            assert!(
                p.candidate.contains("Уточните у раввина")
                    || p.candidate.contains("уточните у раввина"),
                "{}",
                p.id
            );
        }
        let lang = b
            .notes
            .iter()
            .find(|n| n.id == "language")
            .expect("language note");
        for w in ["Сота 7:1", "Шма", "Амид", "Биркат а-мазон"] {
            assert!(lang.text.contains(w), "language note lacks {w}");
        }
        assert!(b
            .notes
            .iter()
            .any(|n| n.rabbi && n.title.contains("уточните у раввина")));
    }

    #[test]
    fn jewish_framing_only() {
        const BANNED: [&str; 8] = [
            "библи",
            "ветхий завет",
            "ветхого завета",
            "ветхом завете",
            "старый завет",
            "новый завет",
            "христиан",
            "церк",
        ];
        let raw = include_str!("../content/ru/prayers.json").to_lowercase();
        for w in BANNED {
            assert!(!raw.contains(w), "banned word «{w}»");
        }
    }

    #[test]
    fn ids_unique_and_neighbours_walk_the_list() {
        let b = book();
        let mut ids: Vec<_> = b.prayers.iter().map(|p| p.id.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), b.prayers.len());
        let first = &b.prayers[0].id;
        assert_eq!(b.neighbours(first).0, None);
        assert_eq!(b.neighbours(first).1.map(|p| &p.id), Some(&b.prayers[1].id));
        let last = &b.prayers.last().unwrap().id;
        assert_eq!(b.neighbours(last).1, None);
    }

    #[test]
    fn prefs_round_trip_through_state_extra() {
        let mut s = State::default();
        assert_eq!(Prefs::load(&s), Prefs::default());
        let mut p = Prefs {
            display: Display::Ru,
            ..Default::default()
        };
        p.bigger();
        p.bigger();
        assert!(p.toggle_favorite("shma"));
        assert!(p.toggle_favorite("hamotzi"));
        assert!(!p.toggle_favorite("shma"));
        p.store(&mut s);
        let back = Prefs::load(&s);
        assert_eq!(back.display, Display::Ru);
        assert_eq!(back.size, 3);
        assert_eq!(back.favorites, vec!["hamotzi".to_string()]);
        // Saved JSON keeps the readable keys.
        let v = &s.extra[KEY];
        assert_eq!(v["display"], "ru");
        // Junk and unknown ids are dropped, sizes clamped.
        s.extra
            .insert(KEY.into(), serde_json::json!({"display": "xx", "size": 99}));
        assert_eq!(Prefs::load(&s), Prefs::default());
        s.extra.insert(
            KEY.into(),
            serde_json::json!({"size": 99, "favorites": ["nope", "shma"]}),
        );
        let p = Prefs::load(&s);
        assert_eq!(p.size, SIZES.len() - 1);
        assert_eq!(p.favorites, vec!["shma".to_string()]);
        let mut p = Prefs::default();
        for _ in 0..10 {
            p.smaller();
        }
        assert_eq!(p.size, 0);
    }

    #[test]
    fn display_modes() {
        for d in Display::ALL {
            assert_eq!(Display::from_key(d.key()), Some(d));
        }
        assert!(
            Display::All.shows_he() && Display::All.shows_ru() && Display::All.shows_translit()
        );
        assert!(Display::Ru.shows_ru() && !Display::Ru.shows_he() && !Display::Ru.shows_translit());
        assert!(Display::He.shows_he() && !Display::He.shows_ru());
        assert!(Display::Translit.shows_translit() && !Display::Translit.shows_he());
    }
}
