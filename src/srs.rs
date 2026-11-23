//! Spaced repetition ported from js/srs.js.
//! Card level 0..=5. «Вспомнил» raises the level and schedules the next review
//! after INTERVALS[level-1] days (1, 3, 7, 14, 30). «Не вспомнил» resets to
//! level 0, due tomorrow. Mastered («выучена») only after «Вспомнил» at level 5.

use chrono::NaiveDate;

use crate::storage::{CardState, State};
use crate::util;

pub const INTERVALS: [i64; 5] = [1, 3, 7, 14, 30];
pub const MAX: i64 = 5;

/// Grade a card: `knew` = «Вспомнил». Returns the updated card state.
/// Mirrors js/srs.js grade() including the activity touch.
pub fn grade(state: &mut State, id: &str, knew: bool, today: NaiveDate) -> CardState {
    let today_key = util::day_key(today);
    let mut c = state.cards.get(id).cloned().unwrap_or_default();
    step(&mut c, knew, &today_key);
    state.cards.insert(id.to_string(), c.clone());
    state.touch(&today_key);
    c
}

/// One review of a card state (the scheduling part of [`grade`]), for other
/// decks that keep their own card states (the interview trainer).
pub fn step(c: &mut CardState, knew: bool, today_key: &str) {
    c.n += 1;
    if knew {
        if c.l >= MAX {
            c.m = true; // recalled at the 30-day interval → выучена
        }
        c.l = (c.l + 1).min(MAX);
        c.due = util::add_days(today_key, INTERVALS[(c.l - 1) as usize]);
    } else {
        c.l = 0;
        c.m = false;
        c.miss += 1;
        c.due = util::add_days(today_key, 1);
    }
    c.last = today_key.to_string();
}

pub fn is_due(state: &State, id: &str, today: NaiveDate) -> bool {
    match state.cards.get(id) {
        Some(c) => !c.due.is_empty() && c.due <= util::day_key(today),
        None => false,
    }
}

pub fn is_new(state: &State, id: &str) -> bool {
    !state.cards.contains_key(id)
}

/// Old stored progress has no `m` field → not mastered (level alone is not enough).
pub fn is_mastered(state: &State, id: &str) -> bool {
    state.cards.get(id).map(|c| c.m).unwrap_or(false)
}

/// Feedback message shown right after grading (js/ui/cards.js).
/// `word` is the session word: "уроке" / "повторении".
pub fn grade_message(c: &CardState, knew: bool, word: &str, today: NaiveDate) -> String {
    if knew {
        let d = util::diff_days(&util::day_key(today), &c.due);
        let days = util::plural(d, &["день", "дня", "дней"]);
        if c.m {
            format!("Карточка выучена! Следующий контроль через {d} {days}")
        } else {
            format!("Следующий показ: через {d} {days}")
        }
    } else {
        format!("Покажем ещё раз в этом {word}, а затем завтра")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn interval_sequence_1_3_7_14_30() {
        let t = day(2026, 11, 22);
        let mut s = State::default();
        let mut expected_due = vec![];
        for &i in INTERVALS.iter() {
            expected_due.push(util::add_days("2026-11-22", i));
        }
        for (i, due) in expected_due.iter().enumerate() {
            let c = grade(&mut s, "c1", true, t);
            assert_eq!(c.l, i as i64 + 1);
            assert_eq!(&c.due, due);
            assert!(!c.m || c.l == MAX && c.n > 5);
        }
    }

    #[test]
    fn mastered_only_on_recall_at_level_5() {
        let t = day(2026, 11, 22);
        let mut s = State::default();
        // Reach level 5: five recalls, not yet mastered.
        for _ in 0..5 {
            let c = grade(&mut s, "c1", true, t);
            assert!(!c.m, "not mastered while merely reaching level 5");
        }
        assert_eq!(s.cards["c1"].l, 5);
        assert!(!is_mastered(&s, "c1"));
        // Sixth recall arrives at level 5 again → mastered.
        let c = grade(&mut s, "c1", true, t);
        assert!(c.m);
        assert!(is_mastered(&s, "c1"));
        assert_eq!(c.due, util::add_days("2026-11-22", 30));
    }

    #[test]
    fn miss_resets_to_level_0_due_tomorrow() {
        let t = day(2026, 11, 22);
        let mut s = State::default();
        grade(&mut s, "c1", true, t);
        grade(&mut s, "c1", true, t);
        let c = grade(&mut s, "c1", false, t);
        assert_eq!(c.l, 0);
        assert!(!c.m);
        assert_eq!(c.miss, 1);
        assert_eq!(c.due, "2026-11-23");
        assert_eq!(c.last, "2026-11-22");
        assert_eq!(c.n, 3);
    }

    #[test]
    fn due_and_new_logic() {
        let t = day(2026, 11, 22);
        let mut s = State::default();
        assert!(is_new(&s, "c1"));
        assert!(!is_due(&s, "c1", t));
        grade(&mut s, "c1", false, t); // due tomorrow
        assert!(!is_new(&s, "c1"));
        assert!(!is_due(&s, "c1", t));
        assert!(is_due(&s, "c1", day(2026, 11, 23)));
        // Card due today counts as due.
        s.cards.get_mut("c1").unwrap().due = "2026-11-22".into();
        assert!(is_due(&s, "c1", t));
    }

    #[test]
    fn old_cards_without_m_not_mastered() {
        let mut s = State::default();
        // Simulate legacy progress: level 5, no m flag.
        s.cards.insert(
            "old".to_string(),
            CardState {
                l: 5,
                due: "2026-11-22".into(),
                n: 5,
                miss: 0,
                m: false,
                last: "2026-11-01".into(),
            },
        );
        assert!(!is_mastered(&s, "old"));
        assert!(is_due(&s, "old", day(2026, 11, 22)));
    }

    #[test]
    fn grade_touches_activity() {
        let t = day(2026, 11, 22);
        let mut s = State::default();
        grade(&mut s, "c1", true, t);
        assert_eq!(s.activity["2026-11-22"], 1);
    }

    #[test]
    fn grade_messages() {
        let t = day(2026, 11, 22);
        let mut s = State::default();
        let c = grade(&mut s, "c1", true, t);
        assert_eq!(
            grade_message(&c, true, "уроке", t),
            "Следующий показ: через 1 день"
        );
        let c = grade(&mut s, "c1", true, t);
        assert_eq!(
            grade_message(&c, true, "уроке", t),
            "Следующий показ: через 3 дня"
        );
        let c = grade(&mut s, "c1", false, t);
        assert_eq!(
            grade_message(&c, false, "повторении", t),
            "Покажем ещё раз в этом повторении, а затем завтра"
        );
        for _ in 0..6 {
            grade(&mut s, "c1", true, t);
        }
        let c = s.cards["c1"].clone();
        assert!(c.m);
        assert_eq!(
            grade_message(&c, true, "уроке", t),
            "Карточка выучена! Следующий контроль через 30 дней"
        );
    }
}
