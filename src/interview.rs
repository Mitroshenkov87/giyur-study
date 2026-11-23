//! «Тренажёр собеседования с бейт-дином»: the question bank
//! (content/interview.json, built by tools/interview/build_content.py) and
//! the practice rules. The screen is `ui::screens::interview`.
//!
//! «Практика» and «Репетиция» are multiple-choice (3–4 options, one of
//! them right); a right or wrong pick drives the same SRS ladder as the
//! flashcards. «Темы» is a reading mode (question + model answer, no
//! typing). «Репетиция» also has an optional «Ответить вслух»: speak,
//! reveal the model answer, self-mark «знал» / «не знал» — not scored.
//!
//! Progress lives in the progress file under the top-level key
//! `"interview"` (kept by `State::extra`, so older builds round-trip it):
//! one SRS card per question (the same 1 → 3 → 7 → 14 → 30 day ladder as
//! the flashcards, `srs::step`), the last self-rating of each question, and
//! the last mock interviews («Репетиция»).

use std::collections::HashMap;
use std::sync::OnceLock;

use chrono::NaiveDate;
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::storage::{CardState, Level, State};
use crate::{srs, util};

/// Key of the trainer's progress in the progress file.
pub const KEY: &str = "interview";
/// Questions in one «Репетиция».
pub const REHEARSAL_LEN: usize = 10;
/// Mock interviews kept in the history.
pub const MAX_RUNS: usize = 30;
/// XP for an answer rated «Готов» (practice and rehearsal alike).
pub const XP_READY: i64 = 1;

#[derive(Debug, Clone, Deserialize)]
pub struct Topic {
    pub id: String,
    pub title: String,
    pub blurb: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: String,
    pub topic: String,
    pub question: String,
    /// What the rabbis really want to hear or check.
    pub purpose: String,
    pub key_points: Vec<String>,
    pub mistakes: Vec<String>,
    /// Lesson ids in the embedded library.
    pub lessons: Vec<String>,
    /// How the movements differ on this question, when they do.
    #[serde(default)]
    pub movements: Option<String>,
    /// Default options (used when `options_by_level` has no entry for the learner).
    pub options: Vec<String>,
    /// Index of the right option in `options`.
    pub answer: usize,
    /// Movement-specific options, keyed `orthodox` / `conservative` / `reform`.
    #[serde(default)]
    pub options_by_level: Option<HashMap<String, ChoiceSet>>,
}

/// One multiple-choice set (the default options, or one movement's variant).
#[derive(Debug, Clone, Deserialize)]
pub struct ChoiceSet {
    pub options: Vec<String>,
    pub answer: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Bank {
    pub title: String,
    pub intro: String,
    pub sincerity: String,
    pub topics: Vec<Topic>,
    pub questions: Vec<Item>,
}

impl Bank {
    pub fn item(&self, id: &str) -> Option<&Item> {
        self.questions.iter().find(|q| q.id == id)
    }

    pub fn topic(&self, id: &str) -> Option<&Topic> {
        self.topics.iter().find(|t| t.id == id)
    }

    pub fn in_topic<'a>(&'a self, topic: &'a str) -> impl Iterator<Item = &'a Item> + 'a {
        self.questions.iter().filter(move |q| q.topic == topic)
    }
}

impl Item {
    /// Options and the index of the right one for this learner's movement.
    pub fn choices(&self, level: Level) -> (&[String], usize) {
        if let Some(map) = &self.options_by_level {
            let key = match level {
                Level::Reform => "reform",
                Level::Conservative => "conservative",
                Level::Orthodox => "orthodox",
            };
            if let Some(c) = map.get(key) {
                return (&c.options, c.answer);
            }
        }
        (&self.options, self.answer)
    }
}

/// Movement used for movement-specific options (unset → orthodox, as elsewhere).
pub fn user_level(state: &State) -> Level {
    state.settings.level.unwrap_or(Level::Orthodox)
}

/// One-line explanation of the current tab, shown under the mode switch.
pub fn mode_hint(mode: Mode) -> &'static str {
    match mode {
        Mode::Browse => {
            "Темы: читайте вопросы и образцы ответов, чтобы спокойно изучить материал."
        }
        Mode::Practice => {
            "Практика: выберите верный ответ из нескольких вариантов. Результат проверяется сразу и учитывается в повторениях."
        }
        Mode::Rehearsal => {
            "Репетиция: десять вопросов с вариантами ответа, как пробная беседа. В конце — счёт. По желанию можно сначала ответить вслух."
        }
    }
}

/// Neighbouring topic in bank order. `id = None` (the topic list): next is the
/// first topic, previous is none.
pub fn neighbor_topic(id: Option<&str>, dir: i32) -> Option<&'static str> {
    let topics = &bank().topics;
    if topics.is_empty() {
        return None;
    }
    let i = match id {
        None => {
            return if dir > 0 {
                Some(topics[0].id.as_str())
            } else {
                None
            };
        }
        Some(cur) => topics.iter().position(|t| t.id == cur)?,
    };
    let j = i as i32 + dir;
    if j >= 0 && (j as usize) < topics.len() {
        Some(topics[j as usize].id.as_str())
    } else {
        None
    }
}

pub const RAW: &str = include_str!("../content/interview.json");

/// The embedded question bank.
pub fn bank() -> &'static Bank {
    static BANK: OnceLock<Bank> = OnceLock::new();
    BANK.get_or_init(|| serde_json::from_str(RAW).expect("content/interview.json"))
}

/// Which part of the trainer a route opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// «Темы»: all questions by topic with the guidance.
    #[default]
    Browse,
    /// «Практика»: one question at a time, weak ones more often.
    Practice,
    /// «Репетиция»: a mock interview of [`REHEARSAL_LEN`] questions.
    Rehearsal,
}

impl Mode {
    /// Route argument (`interview`, `interview:practice`, `interview:rehearsal`).
    pub fn key(self) -> &'static str {
        match self {
            Mode::Browse => "",
            Mode::Practice => "practice",
            Mode::Rehearsal => "rehearsal",
        }
    }

    pub fn parse(s: &str) -> Option<Mode> {
        Some(match s {
            "" | "topics" | "browse" => Mode::Browse,
            "practice" => Mode::Practice,
            "rehearsal" => Mode::Rehearsal,
            _ => return None,
        })
    }
}

/// The learner's own rating of an answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Rating {
    /// «Готов»: answered sincerely and covered the main points.
    Ready,
    /// «Частично»: some points missing or unsure.
    Partly,
    /// «Ещё не готов».
    NotYet,
}

impl Rating {
    pub fn label(self) -> &'static str {
        match self {
            Rating::Ready => "Готов",
            Rating::Partly => "Частично",
            Rating::NotYet => "Ещё не готов",
        }
    }

    /// Share of a full answer (summaries and readiness).
    pub fn score(self) -> f64 {
        match self {
            Rating::Ready => 1.0,
            Rating::Partly => 0.5,
            Rating::NotYet => 0.0,
        }
    }
}

/// One finished «Репетиция».
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Run {
    pub day: String,
    pub ids: Vec<String>,
    pub ratings: Vec<Rating>,
}

impl Run {
    pub fn pct(&self) -> i64 {
        if self.ratings.is_empty() {
            return 0;
        }
        let s: f64 = self.ratings.iter().map(|r| r.score()).sum();
        util::pct(s / self.ratings.len() as f64)
    }

    pub fn count(&self, r: Rating) -> usize {
        self.ratings.iter().filter(|x| **x == r).count()
    }
}

/// The trainer's progress (`state.extra["interview"]`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Progress {
    #[serde(default)]
    pub cards: HashMap<String, CardState>,
    #[serde(default)]
    pub last: HashMap<String, Rating>,
    #[serde(default)]
    pub runs: Vec<Run>,
}

pub fn load(state: &State) -> Progress {
    state
        .extra
        .get(KEY)
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default()
}

pub fn store(state: &mut State, p: &Progress) {
    if let Ok(v) = serde_json::to_value(p) {
        state.extra.insert(KEY.to_string(), v);
    }
}

/// Record a self-rating: schedules the question like a flashcard
/// («Готов» = recalled; «Частично» = one level down, back tomorrow;
/// «Ещё не готов» = level 0, back tomorrow) and counts as activity.
pub fn rate(state: &mut State, id: &str, r: Rating, today: NaiveDate) -> CardState {
    let key = util::day_key(today);
    let mut p = load(state);
    let mut c = p.cards.get(id).cloned().unwrap_or_default();
    match r {
        Rating::Ready => srs::step(&mut c, true, &key),
        Rating::NotYet => srs::step(&mut c, false, &key),
        Rating::Partly => {
            c.n += 1;
            c.l = (c.l - 1).max(0);
            c.m = false;
            c.due = util::add_days(&key, 1);
            c.last = key.clone();
        }
    }
    p.cards.insert(id.to_string(), c.clone());
    p.last.insert(id.to_string(), r);
    store(state, &p);
    state.touch(&key);
    c
}

/// Save a finished mock interview (newest last, at most [`MAX_RUNS`]).
pub fn finish_run(state: &mut State, run: Run) {
    let mut p = load(state);
    p.runs.push(run);
    let extra = p.runs.len().saturating_sub(MAX_RUNS);
    p.runs.drain(..extra);
    store(state, &p);
}

/// How much a question needs practice now: never practised and due
/// questions first, then the ones rated low or missed often; mastered
/// ones rarely.
pub fn weight(p: &Progress, id: &str, today: NaiveDate) -> f64 {
    let Some(c) = p.cards.get(id) else {
        return 4.0;
    };
    let due = !c.due.is_empty() && c.due <= util::day_key(today);
    let last = p.last.get(id).copied();
    let mut w = match last {
        Some(Rating::NotYet) => 5.0,
        Some(Rating::Partly) => 3.5,
        _ => 1.0,
    };
    if due {
        w += 2.0;
    }
    w += c.miss.min(5) as f64 * 0.5;
    w += (srs::MAX - c.l).max(0) as f64 * 0.2;
    if c.m {
        w = 0.3;
    }
    w
}

fn pool(topic: Option<&str>) -> Vec<&'static Item> {
    bank()
        .questions
        .iter()
        .filter(|q| topic.is_none_or(|t| q.topic == t))
        .collect()
}

/// Next practice question (weighted toward weak ones), not `avoid` when
/// there is another choice.
pub fn pick(
    state: &State,
    topic: Option<&str>,
    avoid: Option<&str>,
    today: NaiveDate,
    rng: &mut impl Rng,
) -> Option<String> {
    let p = load(state);
    let mut items: Vec<&Item> = pool(topic);
    if items.len() > 1 {
        items.retain(|q| Some(q.id.as_str()) != avoid);
    }
    util::weighted_sample(&items, |q| weight(&p, &q.id, today), 1, rng)
        .first()
        .map(|q| q.id.clone())
}

/// Questions for a «Репетиция»: [`REHEARSAL_LEN`] weighted picks, at most
/// two from one topic so the mock interview ranges over the whole life,
/// then ordered the way an interview goes (motivation first, knowledge last).
pub fn rehearsal(state: &State, today: NaiveDate, rng: &mut impl Rng) -> Vec<String> {
    let p = load(state);
    let b = bank();
    let all: Vec<&Item> = b.questions.iter().collect();
    let ranked = util::weighted_sample(&all, |q| weight(&p, &q.id, today), all.len(), rng);
    let mut per: HashMap<&str, usize> = HashMap::new();
    let mut out: Vec<&Item> = Vec::new();
    for q in &ranked {
        let n = per.entry(q.topic.as_str()).or_insert(0);
        if *n < 2 {
            *n += 1;
            out.push(q);
        }
        if out.len() == REHEARSAL_LEN {
            break;
        }
    }
    for q in &ranked {
        if out.len() == REHEARSAL_LEN {
            break;
        }
        if !out.iter().any(|x| x.id == q.id) {
            out.push(q);
        }
    }
    let order = |t: &str| b.topics.iter().position(|x| x.id == t).unwrap_or(99);
    out.sort_by_key(|q| order(&q.topic));
    out.into_iter().map(|q| q.id.clone()).collect()
}

/// Totals for «Успехи» and the trainer's header.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Stats {
    pub total: usize,
    /// Practised at least once.
    pub practiced: usize,
    /// Last rating «Готов».
    pub ready: usize,
    /// Due for another look today.
    pub due: usize,
    pub runs: usize,
    pub best_run: Option<i64>,
    pub last_run: Option<i64>,
    /// Mean of the last ratings over all questions (0..=100).
    pub readiness: i64,
}

pub fn stats(state: &State, today: NaiveDate) -> Stats {
    let p = load(state);
    let b = bank();
    let total = b.questions.len();
    let today_key = util::day_key(today);
    let mut s = Stats {
        total,
        runs: p.runs.len(),
        best_run: p.runs.iter().map(Run::pct).max(),
        last_run: p.runs.last().map(Run::pct),
        ..Stats::default()
    };
    let mut sum = 0.0;
    for q in &b.questions {
        if let Some(c) = p.cards.get(&q.id) {
            s.practiced += 1;
            if !c.due.is_empty() && c.due <= today_key {
                s.due += 1;
            }
        }
        if let Some(r) = p.last.get(&q.id) {
            sum += r.score();
            if *r == Rating::Ready {
                s.ready += 1;
            }
        }
    }
    if total > 0 {
        s.readiness = util::pct(sum / total as f64);
    }
    s
}

/// Per-topic share of questions whose last rating is «Готов» (0..=100).
pub fn topic_ready(state: &State, topic: &str) -> (usize, usize) {
    let p = load(state);
    let items: Vec<&Item> = bank().in_topic(topic).collect();
    let ready = items
        .iter()
        .filter(|q| p.last.get(&q.id) == Some(&Rating::Ready))
        .count();
    (ready, items.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn bank_is_complete() {
        let b = bank();
        assert!(
            (40..=80).contains(&b.questions.len()),
            "{} questions",
            b.questions.len()
        );
        assert_eq!(b.topics.len(), 9);
        let lib = crate::content::Library::embedded();
        let mut ids = std::collections::HashSet::new();
        for q in &b.questions {
            assert!(ids.insert(q.id.as_str()), "duplicate {}", q.id);
            assert!(b.topic(&q.topic).is_some(), "{}: topic {}", q.id, q.topic);
            assert!(q.question.ends_with(['?', '.']), "{}", q.id);
            assert!(!q.purpose.is_empty(), "{}", q.id);
            assert!(q.key_points.len() >= 2, "{}: key points", q.id);
            assert!(!q.mistakes.is_empty(), "{}: mistakes", q.id);
            assert!(!q.lessons.is_empty(), "{}: lessons", q.id);
            for l in &q.lessons {
                assert!(lib.lesson(l).is_some(), "{}: no lesson {l}", q.id);
            }
            for level in [Level::Orthodox, Level::Conservative, Level::Reform] {
                let (opts, ans) = q.choices(level);
                assert!(
                    (3..=4).contains(&opts.len()),
                    "{} {:?}: {} options",
                    q.id,
                    level,
                    opts.len()
                );
                assert!(ans < opts.len(), "{} {:?}: answer", q.id, level);
                let uniq: std::collections::HashSet<_> = opts.iter().map(|s| s.as_str()).collect();
                assert_eq!(
                    uniq.len(),
                    opts.len(),
                    "{} {:?}: duplicate option",
                    q.id,
                    level
                );
                assert!(
                    opts.iter().all(|s| !s.trim().is_empty()),
                    "{} empty option",
                    q.id
                );
            }
            if q.movements.is_some() {
                let map = q
                    .options_by_level
                    .as_ref()
                    .unwrap_or_else(|| panic!("{}: movements without optionsByLevel", q.id));
                for key in ["orthodox", "conservative", "reform"] {
                    assert!(map.contains_key(key), "{}: missing {key} options", q.id);
                }
            }
        }
        for t in &b.topics {
            assert!(b.in_topic(&t.id).count() >= 4, "topic {} too small", t.id);
        }
    }

    #[test]
    fn no_christian_framing() {
        let low = RAW.to_lowercase();
        for banned in [
            "библи",
            "ветхий завет",
            "ветхого завета",
            "новый завет",
            "нового завета",
            "старый завет",
            "старого завета",
            "христи",
        ] {
            assert!(!low.contains(banned), "interview.json contains «{banned}»");
        }
    }

    #[test]
    fn sincerity_is_stated() {
        let b = bank();
        assert!(b.intro.contains("заучив") || b.intro.contains("Заучен"));
        assert!(b.sincerity.contains("честн"));
    }

    #[test]
    fn rating_schedules_like_cards() {
        let t = day(2026, 11, 22);
        let mut s = State::default();
        let c = rate(&mut s, "mot-why", Rating::Ready, t);
        assert_eq!((c.l, c.due.as_str()), (1, "2026-11-23"));
        let c = rate(&mut s, "mot-why", Rating::Ready, t);
        assert_eq!((c.l, c.due.as_str()), (2, "2026-11-25"));
        let c = rate(&mut s, "mot-why", Rating::Partly, t);
        assert_eq!((c.l, c.due.as_str()), (1, "2026-11-23"));
        let c = rate(&mut s, "mot-why", Rating::NotYet, t);
        assert_eq!((c.l, c.miss), (0, 1));
        assert_eq!(s.activity["2026-11-22"], 4);
        // Stored under its own key, not mixed into the flashcards.
        assert!(s.cards.is_empty());
        assert!(s.extra.contains_key(KEY));
        // Round-trips through the progress file format.
        let json = serde_json::to_string(&s).unwrap();
        let back: State = serde_json::from_str(&json).unwrap();
        assert_eq!(load(&back).last["mot-why"], Rating::NotYet);
    }

    #[test]
    fn weak_questions_come_back_more_often() {
        let t = day(2026, 11, 22);
        let mut s = State::default();
        let ids: Vec<String> = bank().questions.iter().map(|q| q.id.clone()).collect();
        for id in &ids {
            rate(&mut s, id, Rating::Ready, t);
        }
        rate(&mut s, "shab-how", Rating::NotYet, t);
        let p = load(&s);
        assert!(weight(&p, "shab-how", t) > 3.0 * weight(&p, "mot-why", t));
        let mut rng = StdRng::seed_from_u64(1);
        let hits = (0..400)
            .filter(|_| pick(&s, None, None, t, &mut rng).as_deref() == Some("shab-how"))
            .count();
        // Uniform would be 400 / bank_len; the weak item must still stand out.
        let n = ids.len().max(1);
        let floor = (400 / n).saturating_mul(2).max(12);
        assert!(hits > floor, "weak question picked {hits} times (bank={n}, floor={floor})");
    }

    #[test]
    fn pick_by_topic_and_avoid() {
        let t = day(2026, 11, 22);
        let s = State::default();
        let mut rng = StdRng::seed_from_u64(3);
        for _ in 0..50 {
            let id = pick(&s, Some("kashrut"), Some("kosh-home"), t, &mut rng).unwrap();
            assert_eq!(bank().item(&id).unwrap().topic, "kashrut");
            assert_ne!(id, "kosh-home");
        }
    }

    #[test]
    fn rehearsal_ranges_over_topics() {
        let t = day(2026, 11, 22);
        let s = State::default();
        let mut rng = StdRng::seed_from_u64(7);
        let ids = rehearsal(&s, t, &mut rng);
        assert_eq!(ids.len(), REHEARSAL_LEN);
        let uniq: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(uniq.len(), ids.len());
        let b = bank();
        let mut per = HashMap::new();
        for id in &ids {
            *per.entry(b.item(id).unwrap().topic.clone()).or_insert(0) += 1;
        }
        assert!(per.values().all(|n| *n <= 2), "{per:?}");
        assert!(per.len() >= 5);
        // Interview order: topics never go backwards.
        let pos: Vec<usize> = ids
            .iter()
            .map(|id| {
                let tp = &b.item(id).unwrap().topic;
                b.topics.iter().position(|x| &x.id == tp).unwrap()
            })
            .collect();
        assert!(pos.windows(2).all(|w| w[0] <= w[1]));
    }

    #[test]
    fn stats_and_runs() {
        let t = day(2026, 11, 22);
        let mut s = State::default();
        assert_eq!(stats(&s, t).practiced, 0);
        rate(&mut s, "mot-why", Rating::Ready, t);
        rate(&mut s, "faith-god", Rating::Partly, t);
        for i in 0..(MAX_RUNS + 3) {
            finish_run(
                &mut s,
                Run {
                    day: "2026-11-22".into(),
                    ids: vec!["mot-why".into(), "faith-god".into()],
                    ratings: if i == 0 {
                        vec![Rating::Ready, Rating::Ready]
                    } else {
                        vec![Rating::Ready, Rating::Partly]
                    },
                },
            );
        }
        let st = stats(&s, t);
        assert_eq!(st.practiced, 2);
        assert_eq!(st.ready, 1);
        assert_eq!(st.runs, MAX_RUNS);
        assert_eq!(st.last_run, Some(75));
        // The 100% run was the oldest and fell out of the history.
        assert_eq!(st.best_run, Some(75));
        assert_eq!(st.readiness, util::pct(1.5 / st.total as f64));
        assert_eq!(topic_ready(&s, "motivation").0, 1);
    }

    #[test]
    fn neighbor_topic_walks_the_list() {
        let b = bank();
        assert_eq!(neighbor_topic(None, -1), None);
        assert_eq!(neighbor_topic(None, 1), Some(b.topics[0].id.as_str()));
        let first = b.topics[0].id.as_str();
        let second = b.topics[1].id.as_str();
        let last = b.topics[b.topics.len() - 1].id.as_str();
        assert_eq!(neighbor_topic(Some(first), -1), None);
        assert_eq!(neighbor_topic(Some(first), 1), Some(second));
        assert_eq!(neighbor_topic(Some(last), 1), None);
        assert_eq!(
            neighbor_topic(Some(last), -1),
            Some(b.topics[b.topics.len() - 2].id.as_str())
        );
    }

    #[test]
    fn movement_options_differ_where_expected() {
        let b = bank();
        let q = b.item("mitz-accept").unwrap();
        let (o, _) = q.choices(Level::Orthodox);
        let (r, _) = q.choices(Level::Reform);
        assert_ne!(o[0], r[0]);
        assert!(o[0].contains("все заповеди") || o[0].contains("кабалат"));
        assert!(r[0].to_lowercase().contains("реформ"));
    }
}
