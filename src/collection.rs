//! «Коллекция предметов» on «Успехи»: every live 3D ritual object is a
//! collectible that opens through real study, not by itself.
//!
//! Most objects open when the lesson that talks about them is marked read;
//! the Second Temple opens on the 10th step («Мишна») of «Маалот», whose
//! fifteen steps recall the fifteen steps of the Temple court. Unlocks are
//! recorded once in `State::extra["collection"]` (object key → day), so an
//! object stays in the collection after a level change hides its lesson;
//! a progress reset clears them with everything else.

use crate::content::Library;
use crate::storage::State;

/// `State::extra` key of the unlocked objects map.
pub const KEY: &str = "collection";

/// What opens an object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unlock {
    /// The lesson with this id is marked read.
    Lesson(&'static str),
    /// The best «Маалот» result reaches this many steps.
    Ladder(usize),
}

/// One collectible: the object key (the 3D asset name, see
/// `ui::model3d::Model::key`) and its unlock rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Item {
    pub key: &'static str,
    pub unlock: Unlock,
}

const fn lesson(key: &'static str, id: &'static str) -> Item {
    Item {
        key,
        unlock: Unlock::Lesson(id),
    }
}

/// All collectibles, in the order of the 3D model list.
pub const ITEMS: [Item; 22] = [
    lesson("kiddush-cup", "shabbat-3"),
    lesson("shabbat-candlesticks", "shabbat-2"),
    lesson("menorah", "osnovy-5"),
    lesson("torah-scroll", "osnovy-2"),
    lesson("shofar", "prazdniki-2"),
    Item {
        key: "temple-complex",
        unlock: Unlock::Ladder(10),
    },
    lesson("magen-david", "osnovy-1"),
    lesson("hanukkiah", "prazdniki-5"),
    lesson("seder-plate", "prazdniki-4"),
    lesson("lulav-etrog", "prazdniki-3"),
    lesson("megillat-esther", "prazdniki-5"),
    lesson("sukkah", "prazdniki-3"),
    lesson("tallit", "ivrit-6"),
    lesson("mezuzah", "semya-4"),
    lesson("siddur", "ivrit-4"),
    lesson("tefillin", "ivrit-3"),
    lesson("kotel", "prazdniki-6"),
    lesson("chuppah", "semya-3"),
    lesson("ketubah", "semya-3"),
    lesson("kosher-dishes", "kashrut-4"),
    lesson("wedding-glass", "semya-3"),
    lesson("kosher-fish", "kashrut-2"),
];

pub fn item(key: &str) -> Option<&'static Item> {
    ITEMS.iter().find(|i| i.key == key)
}

/// Whether the rule is met by the current progress.
pub fn met(state: &State, unlock: Unlock) -> bool {
    match unlock {
        Unlock::Lesson(id) => Library::is_read(state, id),
        Unlock::Ladder(n) => crate::ladder::best(state) >= n,
    }
}

/// The unlock rule in Russian, for a locked tile.
pub fn condition(lib: &Library, unlock: Unlock) -> String {
    match unlock {
        Unlock::Lesson(id) => match lib.lesson(id) {
            Some((l, sec)) => format!(
                "Прочитайте урок «{}» ({})",
                l.title.as_deref().unwrap_or(&sec.title),
                sec.title
            ),
            None => "Прочитайте урок, где рассказано об этом предмете".to_string(),
        },
        Unlock::Ladder(n) => {
            let r = crate::ladder::rung(n);
            if r.name.is_empty() {
                format!("Поднимитесь в «Маалот» на {n} ступеней")
            } else {
                format!("Поднимитесь в «Маалот» до {n}-й ступени «{}»", r.name)
            }
        }
    }
}

/// Day the object was added to the collection, if it was.
pub fn unlocked_on<'a>(state: &'a State, key: &str) -> Option<&'a str> {
    state
        .extra
        .get(KEY)
        .and_then(|m| m.get(key))
        .and_then(|d| d.as_str())
}

pub fn is_unlocked(state: &State, key: &str) -> bool {
    // «Открытый режим рецензента»: every object can be opened.
    crate::reviewer::active(state) || unlocked_on(state, key).is_some()
}

/// Objects in the collection (only known keys count).
pub fn count(state: &State) -> usize {
    ITEMS.iter().filter(|i| is_unlocked(state, i.key)).count()
}

/// Record every object whose rule is met now; returns the newly added keys
/// in list order. The caller saves the state.
pub fn sync(state: &mut State, today: &str) -> Vec<&'static str> {
    if crate::reviewer::active(state) {
        return Vec::new();
    }
    let new: Vec<&'static str> = ITEMS
        .iter()
        .filter(|i| !is_unlocked(state, i.key) && met(state, i.unlock))
        .map(|i| i.key)
        .collect();
    if new.is_empty() {
        return new;
    }
    let entry = state
        .extra
        .entry(KEY.to_string())
        .or_insert_with(|| serde_json::Value::Object(Default::default()));
    if !entry.is_object() {
        *entry = serde_json::Value::Object(Default::default());
    }
    if let Some(map) = entry.as_object_mut() {
        for k in &new {
            map.insert(
                (*k).to_string(),
                serde_json::Value::String(today.to_string()),
            );
        }
    }
    new
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Level;

    #[test]
    fn every_rule_points_at_a_lesson_everyone_sees() {
        let lib = Library::embedded();
        let mut keys = std::collections::HashSet::new();
        for i in ITEMS {
            assert!(keys.insert(i.key), "duplicate {}", i.key);
            match i.unlock {
                Unlock::Lesson(id) => {
                    let (l, sec) = lib.lesson(id).unwrap_or_else(|| panic!("{id}"));
                    // Reachable at every level, so no object is locked forever.
                    assert!(
                        lib.lessons(sec, Level::Reform).iter().any(|x| x.id == l.id),
                        "{id} is hidden at the reform level"
                    );
                    let c = condition(&lib, i.unlock);
                    assert!(c.starts_with("Прочитайте урок «"), "{c}");
                }
                Unlock::Ladder(n) => assert!((1..=crate::ladder::STEPS).contains(&n)),
            }
        }
    }

    #[test]
    fn reading_a_lesson_unlocks_its_objects_once() {
        let mut s = State::default();
        assert!(sync(&mut s, "2026-11-23").is_empty());
        assert_eq!(count(&s), 0);
        s.lessons_read.insert("semya-3".into(), "2026-11-23".into());
        assert_eq!(
            sync(&mut s, "2026-11-23"),
            vec!["chuppah", "ketubah", "wedding-glass"]
        );
        assert_eq!(count(&s), 3);
        assert_eq!(unlocked_on(&s, "ketubah"), Some("2026-11-23"));
        // Nothing new the next time; the first day is kept.
        assert!(sync(&mut s, "2026-11-24").is_empty());
        assert_eq!(unlocked_on(&s, "ketubah"), Some("2026-11-23"));
        assert!(!is_unlocked(&s, "kiddush-cup"));
    }

    #[test]
    fn the_temple_opens_on_the_mishna_step() {
        let lib = Library::embedded();
        let mut s = State::default();
        s.counters.insert(crate::ladder::BEST_KEY.into(), 9);
        assert!(sync(&mut s, "2026-11-23").is_empty());
        s.counters.insert(crate::ladder::BEST_KEY.into(), 10);
        assert_eq!(sync(&mut s, "2026-11-23"), vec!["temple-complex"]);
        let c = condition(&lib, item("temple-complex").unwrap().unlock);
        assert!(c.contains("Мишна"), "{c}");
    }

    #[test]
    fn unlocks_survive_save_and_load_and_level_changes() {
        let dir = std::env::temp_dir().join(format!("shorashim-coll-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("progress.json");
        let mut s = State::default();
        s.lessons_read
            .insert("shabbat-3".into(), "2026-11-23".into());
        sync(&mut s, "2026-11-23");
        crate::storage::save(&path, &s).unwrap();
        let mut back = crate::storage::load(&path, "2026-11-24");
        assert!(is_unlocked(&back, "kiddush-cup"));
        // Losing the condition later (e.g. an imported file without the
        // lesson) does not take the object away.
        back.lessons_read.clear();
        back.settings.level = Some(Level::Reform);
        assert!(sync(&mut back, "2026-11-24").is_empty());
        assert_eq!(count(&back), 1);
        // Export / import keeps it too.
        let again = State::import_json(&back.export_json(), "2026-11-24").unwrap();
        assert!(is_unlocked(&again, "kiddush-cup"));
        // A reset clears the collection with the rest of the progress.
        back.reset();
        assert_eq!(count(&back), 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_broken_collection_value_is_repaired() {
        let mut s = State::default();
        s.extra.insert(KEY.into(), serde_json::json!("junk"));
        assert_eq!(count(&s), 0);
        s.lessons_read
            .insert("kashrut-2".into(), "2026-11-23".into());
        assert_eq!(sync(&mut s, "2026-11-23"), vec!["kosher-fish"]);
        assert_eq!(count(&s), 1);
    }
}
