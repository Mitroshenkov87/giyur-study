//! Gamification ported from js/game.js: XP, daily goal, streak + freezes,
//! badges, celebrations (returned as values for the UI to display).

use chrono::NaiveDate;

use crate::content::Library;
use crate::storage::State;
use crate::util;

pub struct Goal {
    pub xp: i64,
    pub title: &'static str,
}

pub const GOALS: &[Goal] = &[
    Goal {
        xp: 10,
        title: "Лёгкая",
    },
    Goal {
        xp: 20,
        title: "Обычная",
    },
    Goal {
        xp: 30,
        title: "Серьёзная",
    },
    Goal {
        xp: 50,
        title: "Интенсив",
    },
];

pub const MAX_FREEZES: i64 = 2;
pub const FREEZE_EVERY: i64 = 7;

/// XP rules (js/game.js G.XP).
pub struct Xp {
    pub lesson: i64,
    pub card: i64,
    pub deck: i64,
    pub test: i64,
    pub perfect: i64,
    pub exam_right: i64,
    pub retry_right: i64,
    pub beitdin: i64,
}

pub const XP: Xp = Xp {
    lesson: 10,
    card: 1,
    deck: 5,
    test: 20,
    perfect: 10,
    exam_right: 2,
    retry_right: 1,
    beitdin: 15,
};

/// Daily XP goal from settings, falling back to 20 (js/game.js G.goal).
pub fn goal(state: &State) -> i64 {
    let g = state.settings.daily_goal;
    if GOALS.iter().any(|x| x.xp == g) {
        g
    } else {
        20
    }
}

pub fn goal_title(xp: Option<i64>, state: &State) -> &'static str {
    let x = xp.unwrap_or_else(|| goal(state));
    GOALS
        .iter()
        .find(|g| g.xp == x)
        .map(|g| g.title)
        .unwrap_or("")
}

pub fn today_xp(state: &State, today: NaiveDate) -> i64 {
    state.xp.get(&util::day_key(today)).copied().unwrap_or(0)
}

/// A day «counts» for the streak when the goal was reached or a freeze covered it.
pub fn kept(state: &State, day: &str) -> bool {
    state.goal_days.get(day).copied().unwrap_or(false)
        || state.freeze_days.get(day).copied().unwrap_or(false)
}

/// Streak: consecutive kept days ending today, or yesterday if today is not kept yet.
pub fn streak(state: &State, today: NaiveDate) -> i64 {
    let mut d = util::day_key(today);
    let mut n = 0;
    if !kept(state, &d) {
        d = util::add_days(&d, -1);
    }
    while kept(state, &d) && n < 5000 {
        n += 1;
        d = util::add_days(&d, -1);
    }
    n
}

/// Is the streak alive but today's goal not reached yet?
pub fn at_risk(state: &State, today: NaiveDate) -> bool {
    !kept(state, &util::day_key(today)) && streak(state, today) > 0
}

/// Spend freezes for missed days between the last kept day and today (today
/// itself is never «missed» — there is still time). Called on app start.
/// Returns the number of freezes spent.
pub fn apply_freezes(state: &mut State, today: NaiveDate) -> i64 {
    let t = util::day_key(today);
    if kept(state, &t) || kept(state, &util::add_days(&t, -1)) {
        return 0;
    }
    // last kept day within the last 60 days
    let mut last: Option<String> = None;
    for i in 2..=60 {
        let d = util::add_days(&t, -i);
        if kept(state, &d) {
            last = Some(d);
            break;
        }
    }
    let Some(last) = last else { return 0 };
    let gap = util::diff_days(&last, &t) - 1; // missed days strictly between
    if gap < 1 || gap > state.freezes {
        return 0;
    }
    for k in 1..=gap {
        state.freeze_days.insert(util::add_days(&last, k), true);
    }
    state.freezes -= gap;
    gap
}

/// Celebration event for the UI (js/game.js G.celebrate).
#[derive(Debug, Clone, PartialEq)]
pub struct Celebration {
    pub title: String,
    pub text: String,
    pub icon: String,
    pub is_badge: bool,
}

/// Result of awarding XP.
#[derive(Debug, Clone)]
pub struct AwardOutcome {
    pub amount: i64,
    pub goal_reached: bool,
    pub freeze_earned: bool,
    pub celebration: Option<Celebration>,
    /// Badges unlocked by this award.
    pub new_badges: Vec<Badge>,
}

/// award(amount) adds XP for today, records goal / streak / freezes, checks badges.
pub fn award(lib: &Library, state: &mut State, amount: i64, today: NaiveDate) -> AwardOutcome {
    let today_key = util::day_key(today);
    let amount = amount.max(0);
    if amount > 0 {
        *state.xp.entry(today_key.clone()).or_insert(0) += amount;
        state.xp_total += amount;
    }
    let mut freeze_earned = false;
    let mut reached = false;
    if !state.goal_days.contains_key(&today_key) && today_xp(state, today) >= goal(state) {
        state.goal_days.insert(today_key.clone(), true);
        reached = true;
        let n = streak(state, today);
        if n > 0
            && n % FREEZE_EVERY == 0
            && state.freeze_award_day.as_deref() != Some(today_key.as_str())
            && state.freezes < MAX_FREEZES
        {
            state.freezes += 1;
            state.freeze_award_day = Some(today_key.clone());
            freeze_earned = true;
        }
    }
    let mut celebration = None;
    if reached && state.goal_celebrated.as_deref() != Some(today_key.as_str()) {
        state.goal_celebrated = Some(today_key.clone());
        let n = streak(state, today);
        celebration = Some(Celebration {
            title: "Цель дня выполнена!".to_string(),
            text: format!(
                "{} XP сегодня · серия {} {}{}",
                today_xp(state, today),
                n,
                util::plural(n, &["день", "дня", "дней"]),
                if freeze_earned {
                    " · получена заморозка серии ❄"
                } else {
                    ""
                }
            ),
            icon: "🔥".to_string(),
            is_badge: false,
        });
    }
    let new_badges = check_badges(lib, state, today);
    AwardOutcome {
        amount,
        goal_reached: reached,
        freeze_earned,
        celebration,
        new_badges,
    }
}

/// js/game.js G.count.
pub fn count(state: &mut State, key: &str, by: i64) {
    *state.counters.entry(key.to_string()).or_insert(0) += by;
}

// ---- Badges ----

#[derive(Debug, Clone, PartialEq)]
pub struct Badge {
    pub id: String,
    pub icon: String,
    pub title: String,
    pub text: String,
    pub section: bool,
}

impl Badge {
    pub fn celebration(&self) -> Celebration {
        Celebration {
            title: format!("Новый значок: «{}»", self.title),
            text: self.text.clone(),
            icon: self.icon.clone(),
            is_badge: true,
        }
    }
}

/// The 20 badges of js/game.js G.badgeList: 12 base + one per section.
pub fn badge_list(lib: &Library) -> Vec<Badge> {
    let mut list = vec![
        Badge {
            id: "first-lesson".into(),
            icon: "📖".into(),
            title: "Первый урок".into(),
            text: "Прочитать первый урок".into(),
            section: false,
        },
        Badge {
            id: "first-deck".into(),
            icon: "🃏".into(),
            title: "Первая колода".into(),
            text: "Пройти колоду карточек до конца".into(),
            section: false,
        },
        Badge {
            id: "goal-first".into(),
            icon: "🎯".into(),
            title: "Цель дня".into(),
            text: "Впервые выполнить дневную цель".into(),
            section: false,
        },
        Badge {
            id: "first-test".into(),
            icon: "✅".into(),
            title: "Тест сдан".into(),
            text: "Сдать тест раздела на 80% и выше".into(),
            section: false,
        },
        Badge {
            id: "perfect".into(),
            icon: "💯".into(),
            title: "Без ошибок".into(),
            text: "Сдать тест раздела на 100%".into(),
            section: false,
        },
        Badge {
            id: "streak-3".into(),
            icon: "🔥".into(),
            title: "Три дня подряд".into(),
            text: "Серия 3 дня".into(),
            section: false,
        },
        Badge {
            id: "streak-7".into(),
            icon: "🔥".into(),
            title: "Неделя".into(),
            text: "Серия 7 дней".into(),
            section: false,
        },
        Badge {
            id: "streak-30".into(),
            icon: "🏆".into(),
            title: "Месяц".into(),
            text: "Серия 30 дней".into(),
            section: false,
        },
        Badge {
            id: "cards-100".into(),
            icon: "🧠".into(),
            title: "100 карточек".into(),
            text: "Выучить 100 карточек (интервал 30 дней)".into(),
            section: false,
        },
        Badge {
            id: "millionaire".into(),
            icon: "⏱️".into(),
            title: "20 из 20".into(),
            text: "Экзамен на время: 20 верных из 20".into(),
            section: false,
        },
        Badge {
            id: "beit-din-mock".into(),
            icon: "🏛️".into(),
            title: "Пробный бейт-дин".into(),
            text: "Пройти пробное собеседование".into(),
            section: false,
        },
        Badge {
            id: "all-lessons".into(),
            icon: "📚".into(),
            title: "Все уроки".into(),
            text: "Прочитать все уроки всех разделов".into(),
            section: false,
        },
    ];
    for sec in &lib.sections {
        list.push(Badge {
            id: format!("section-{}", sec.id),
            icon: sec.icon.clone(),
            title: sec.title.clone(),
            text: format!("Пройти раздел «{}»", sec.title),
            section: true,
        });
    }
    list
}

fn mastered_count(state: &State) -> usize {
    state.cards.values().filter(|c| c.m).count()
}

fn all_lessons_read(lib: &Library, state: &State) -> bool {
    let user = Library::user_level(state);
    let mut any = false;
    let mut all = true;
    for sec in &lib.sections {
        if !sec.available {
            all = false;
            continue;
        }
        for l in lib.lessons(sec, user) {
            any = true;
            if !Library::is_read(state, &l.id) {
                all = false;
            }
        }
    }
    any && all
}

fn best_streak(state: &State, today: NaiveDate) -> i64 {
    streak(state, today).max(state.counters.get("bestStreak").copied().unwrap_or(0))
}

/// Evaluate one badge condition (js/game.js badge ok()).
pub fn badge_ok(b: &Badge, lib: &Library, state: &State, today: NaiveDate) -> bool {
    let cnt = &state.counters;
    match b.id.as_str() {
        "first-lesson" => !state.lessons_read.is_empty(),
        "first-deck" => cnt.get("decks").copied().unwrap_or(0) > 0,
        "goal-first" => !state.goal_days.is_empty(),
        "first-test" => state.best.values().any(|&v| v >= 80.0),
        "perfect" => state.best.values().any(|&v| v >= 100.0),
        "streak-3" => best_streak(state, today) >= 3,
        "streak-7" => best_streak(state, today) >= 7,
        "streak-30" => best_streak(state, today) >= 30,
        "cards-100" => mastered_count(state) >= 100,
        "millionaire" => cnt.get("perfectTimed").copied().unwrap_or(0) > 0,
        "beit-din-mock" => cnt.get("beitdin").copied().unwrap_or(0) > 0,
        "all-lessons" => all_lessons_read(lib, state),
        _ => {
            if b.section {
                if let Some(sec_id) = b.id.strip_prefix("section-") {
                    if let Some(sec) = lib.section(sec_id) {
                        return sec.available
                            && lib
                                .section_stats(sec, Library::user_level(state), state)
                                .complete;
                    }
                }
            }
            false
        }
    }
}

/// Unlock newly earned badges; returns the fresh ones (js/game.js G.checkBadges).
pub fn check_badges(lib: &Library, state: &mut State, today: NaiveDate) -> Vec<Badge> {
    let today_key = util::day_key(today);
    let bs = best_streak(state, today);
    state.counters.insert("bestStreak".to_string(), bs);
    let mut fresh = Vec::new();
    for b in badge_list(lib) {
        if state.badges.contains_key(&b.id) {
            continue;
        }
        if badge_ok(&b, lib, state, today) {
            state.badges.insert(b.id.clone(), today_key.clone());
            fresh.push(b);
        }
    }
    fresh
}

// ---- Reminders ----

#[derive(Debug, Clone, PartialEq)]
pub struct Reminder {
    pub kind: &'static str,
    pub text: String,
    pub href: Option<&'static str>,
}

/// Text of the in-app reminders (js/game.js G.reminders).
pub fn reminders(lib: &Library, state: &State, today: NaiveDate) -> Vec<Reminder> {
    let mut out = Vec::new();
    let left = (goal(state) - today_xp(state, today)).max(0);
    let n = streak(state, today);
    if left > 0 {
        if at_risk(state, today) {
            out.push(Reminder {
                kind: "streak",
                text: format!(
                    "Серия {} {} под угрозой — наберите ещё {} XP сегодня",
                    n,
                    util::plural(n, &["день", "дня", "дней"]),
                    left
                ),
                href: None,
            });
        } else {
            out.push(Reminder {
                kind: "goal",
                text: format!("До цели дня — {} XP. Один урок даёт 10 XP", left),
                href: None,
            });
        }
    }
    let due = lib.due_cards(state, today).len() as i64;
    if due > 0 {
        out.push(Reminder {
            kind: "due",
            text: format!(
                "{} {} повторения",
                due,
                util::plural(due, &["карточка ждёт", "карточки ждут", "карточек ждут"])
            ),
            href: Some("#/review"),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::srs;

    fn day(d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, d).unwrap()
    }

    fn lib() -> Library {
        Library::embedded()
    }

    fn goal_day(state: &mut State, d: NaiveDate) {
        let key = util::day_key(d);
        state.xp.insert(key.clone(), 20);
        state.goal_days.insert(key, true);
    }

    #[test]
    fn goals_and_titles() {
        let mut s = State::default();
        assert_eq!(goal(&s), 20);
        assert_eq!(goal_title(None, &s), "Обычная");
        s.settings.daily_goal = 10;
        assert_eq!(goal(&s), 10);
        assert_eq!(goal_title(None, &s), "Лёгкая");
        s.settings.daily_goal = 30;
        assert_eq!(goal_title(None, &s), "Серьёзная");
        s.settings.daily_goal = 50;
        assert_eq!(goal_title(None, &s), "Интенсив");
        s.settings.daily_goal = 99;
        assert_eq!(goal(&s), 20);
    }

    #[test]
    fn award_adds_xp_and_reaches_goal() {
        let lib = lib();
        let mut s = State::default();
        let t = day(22);
        let out = award(&lib, &mut s, XP.lesson, t);
        assert_eq!(out.amount, 10);
        assert!(!out.goal_reached);
        assert_eq!(today_xp(&s, t), 10);
        assert_eq!(s.xp_total, 10);
        let out = award(&lib, &mut s, XP.lesson, t);
        assert!(out.goal_reached);
        let cel = out.celebration.unwrap();
        assert_eq!(cel.title, "Цель дня выполнена!");
        assert!(cel.text.starts_with("20 XP сегодня · серия 1 день"));
        assert_eq!(cel.icon, "🔥");
        // Second goal reach on the same day does not re-celebrate.
        let out = award(&lib, &mut s, XP.card, t);
        assert!(!out.goal_reached);
        assert!(out.celebration.is_none());
        assert!(s.goal_days[&util::day_key(t)]);
    }

    #[test]
    fn streak_counts_kept_days() {
        let mut s = State::default();
        let t = day(22);
        assert_eq!(streak(&s, t), 0);
        goal_day(&mut s, day(20));
        goal_day(&mut s, day(21));
        assert_eq!(streak(&s, t), 2); // today not kept yet, streak alive from yesterday
        assert!(at_risk(&s, t));
        goal_day(&mut s, t);
        assert_eq!(streak(&s, t), 3);
        assert!(!at_risk(&s, t));
        // Gap breaks the streak.
        let mut s2 = State::default();
        goal_day(&mut s2, day(19));
        goal_day(&mut s2, day(21));
        assert_eq!(streak(&s2, t), 1);
    }

    #[test]
    fn freeze_earned_every_7_days() {
        let lib = lib();
        let mut s = State::default();
        for i in 1..=6u32 {
            goal_day(&mut s, day(15 + i)); // 16..21
        }
        let t = day(22);
        let out = award(&lib, &mut s, 20, t); // streak becomes 7
        assert!(out.goal_reached);
        assert_eq!(streak(&s, t), 7);
        assert!(out.freeze_earned);
        assert_eq!(s.freezes, 1);
        assert_eq!(s.freeze_award_day.as_deref(), Some("2026-11-22"));
        assert!(out.celebration.unwrap().text.contains("заморозка серии"));
    }

    #[test]
    fn apply_freezes_covers_missed_days() {
        let mut s = State::default();
        let t = day(22);
        goal_day(&mut s, day(19));
        s.freezes = 2;
        // Missed 20 and 21 (two days) → both covered.
        assert_eq!(apply_freezes(&mut s, t), 2);
        assert_eq!(s.freezes, 0);
        assert!(s.freeze_days["2026-11-20"]);
        assert!(s.freeze_days["2026-11-21"]);
        assert_eq!(streak(&s, t), 3);
        // Nothing left to spend.
        assert_eq!(apply_freezes(&mut s, t), 0);
    }

    #[test]
    fn apply_freezes_respects_limits() {
        let mut s = State::default();
        let t = day(22);
        // Today or yesterday kept → nothing to do.
        goal_day(&mut s, day(21));
        s.freezes = 2;
        assert_eq!(apply_freezes(&mut s, t), 0);
        // Gap bigger than available freezes → no partial cover.
        let mut s2 = State::default();
        goal_day(&mut s2, day(17));
        s2.freezes = 2;
        assert_eq!(apply_freezes(&mut s2, t), 0);
        assert!(s2.freeze_days.is_empty());
        assert_eq!(s2.freezes, 2);
    }

    #[test]
    fn badges_key_cases() {
        let lib = lib();
        let mut s = State::default();
        let t = day(22);
        assert!(check_badges(&lib, &mut s, t).is_empty());
        // first-lesson
        s.lessons_read.insert("l1".to_string(), util::day_key(t));
        let fresh = check_badges(&lib, &mut s, t);
        assert!(fresh.iter().any(|b| b.id == "first-lesson"));
        assert_eq!(s.badges["first-lesson"], "2026-11-22");
        // Already unlocked → not returned again.
        assert!(!check_badges(&lib, &mut s, t)
            .iter()
            .any(|b| b.id == "first-lesson"));
        // first-test (80%) and perfect (100%).
        s.best.insert("osnovy".to_string(), 85.0);
        let fresh = check_badges(&lib, &mut s, t);
        assert!(fresh.iter().any(|b| b.id == "first-test"));
        assert!(!fresh.iter().any(|b| b.id == "perfect"));
        s.best.insert("shabbat".to_string(), 100.0);
        let fresh = check_badges(&lib, &mut s, t);
        assert!(fresh.iter().any(|b| b.id == "perfect"));
        // streak badges via bestStreak counter.
        s.counters.insert("bestStreak".to_string(), 7);
        let fresh = check_badges(&lib, &mut s, t);
        assert!(fresh.iter().any(|b| b.id == "streak-3"));
        assert!(fresh.iter().any(|b| b.id == "streak-7"));
        assert!(!fresh.iter().any(|b| b.id == "streak-30"));
        // millionaire / beitdin counters.
        s.counters.insert("perfectTimed".to_string(), 1);
        s.counters.insert("beitdin".to_string(), 1);
        let fresh = check_badges(&lib, &mut s, t);
        assert!(fresh.iter().any(|b| b.id == "millionaire"));
        assert!(fresh.iter().any(|b| b.id == "beit-din-mock"));
    }

    #[test]
    fn badge_list_has_20_with_exact_ids() {
        let lib = lib();
        let list = badge_list(&lib);
        assert_eq!(list.len(), 20);
        let ids: Vec<&str> = list.iter().map(|b| b.id.as_str()).collect();
        for want in [
            "first-lesson",
            "first-deck",
            "goal-first",
            "first-test",
            "perfect",
            "streak-3",
            "streak-7",
            "streak-30",
            "cards-100",
            "millionaire",
            "beit-din-mock",
            "all-lessons",
            "section-osnovy",
            "section-ivrit",
            "section-shabbat",
            "section-kashrut",
            "section-prazdniki",
            "section-semya",
            "section-etika",
            "section-beit-din",
        ] {
            assert!(ids.contains(&want), "missing badge {want}");
        }
    }

    #[test]
    fn section_badge_when_complete() {
        let lib = lib();
        let mut s = State::default();
        let t = day(22);
        let sec = lib.section("osnovy").unwrap();
        let user = Library::user_level(&s);
        for l in lib.lessons(sec, user) {
            s.lessons_read.insert(l.id.clone(), util::day_key(t));
        }
        s.best.insert("osnovy".to_string(), 80.0);
        let fresh = check_badges(&lib, &mut s, t);
        assert!(fresh.iter().any(|b| b.id == "section-osnovy"));
        assert!(!fresh.iter().any(|b| b.id == "section-ivrit"));
    }

    #[test]
    fn cards_100_badge() {
        let lib = lib();
        let mut s = State::default();
        let t = day(22);
        // Master 100 cards via repeated recalls.
        let sec = lib.section("osnovy").unwrap();
        let ids: Vec<String> = lib
            .section_cards(sec, Level::Orthodox)
            .iter()
            .take(10)
            .map(|c| c.id.clone())
            .collect();
        for (i, id) in ids.iter().enumerate() {
            for j in 0..90 {
                let fake = format!("x-{}-{}", i, j);
                let c = crate::storage::CardState {
                    m: true,
                    ..Default::default()
                };
                s.cards.insert(fake, c);
            }
            let _ = id;
        }
        assert_eq!(s.cards.values().filter(|c| c.m).count(), 900);
        let fresh = check_badges(&lib, &mut s, t);
        assert!(fresh.iter().any(|b| b.id == "cards-100"));
    }

    #[test]
    fn reminders_text() {
        let lib = lib();
        let mut s = State::default();
        let t = day(22);
        // No streak, nothing today → plain goal reminder.
        let r = reminders(&lib, &s, t);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].kind, "goal");
        assert_eq!(r[0].text, "До цели дня — 20 XP. Один урок даёт 10 XP");
        // Streak at risk.
        goal_day(&mut s, day(21));
        let r = reminders(&lib, &s, t);
        assert_eq!(r[0].kind, "streak");
        assert_eq!(
            r[0].text,
            "Серия 1 день под угрозой — наберите ещё 20 XP сегодня"
        );
        // Goal reached → no goal/streak reminder.
        goal_day(&mut s, t);
        assert!(reminders(&lib, &s, t).is_empty());
        // Due cards reminder.
        let sec = lib.section("osnovy").unwrap();
        let card = lib.section_cards(sec, Level::Orthodox)[0];
        let cid = card.id.clone();
        srs::grade(&mut s, &cid, false, t);
        let tomorrow = NaiveDate::from_ymd_opt(2026, 11, 23).unwrap();
        goal_day(&mut s, tomorrow);
        let r = reminders(&lib, &s, tomorrow);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].kind, "due");
        assert_eq!(r[0].text, "1 карточка ждёт повторения");
        assert_eq!(r[0].href, Some("#/review"));
    }

    use crate::storage::Level;

    #[test]
    fn count_accumulates() {
        let mut s = State::default();
        count(&mut s, "decks", 1);
        count(&mut s, "decks", 1);
        count(&mut s, "beitdin", 1);
        assert_eq!(s.counters["decks"], 2);
        assert_eq!(s.counters["beitdin"], 1);
    }
}
