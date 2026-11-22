//! Study structure (docs/STRUCTURE.md): every section has two modes —
//! «Читать» (understand) and «Готовиться» (prepare for the beit din) — and a
//! per-section exam-readiness score built from the prep activities.
//!
//! Pure logic, no UI: the section and lesson screens render what this
//! module computes.

use crate::content::{Library, Question, SectionMeta, PASS};
use crate::storage::{Level, State};

/// The two ways to work with a section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StudyMode {
    /// «Читать»: overview, lessons in order, key ideas, «Углубиться».
    Read,
    /// «Готовиться»: readiness, self-checks, beit-din answers, cards, test.
    Prep,
}

impl StudyMode {
    pub fn as_str(self) -> &'static str {
        match self {
            StudyMode::Read => "read",
            StudyMode::Prep => "prep",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            StudyMode::Read => "Читать",
            StudyMode::Prep => "Готовиться",
        }
    }

    fn parse(s: &str) -> Option<StudyMode> {
        match s {
            "read" => Some(StudyMode::Read),
            "prep" => Some(StudyMode::Prep),
            _ => None,
        }
    }
}

/// Mode to open a section in: the last one the user chose there; otherwise
/// «Читать» until every lesson is read, then «Готовиться».
pub fn mode_for(lib: &Library, sec: &SectionMeta, state: &State) -> StudyMode {
    if let Some(m) = state
        .study_mode
        .get(&sec.id)
        .and_then(|s| StudyMode::parse(s))
    {
        return m;
    }
    let user = Library::user_level(state);
    if lib.section_stats(sec, user, state).all_read {
        StudyMode::Prep
    } else {
        StudyMode::Read
    }
}

pub fn set_mode(state: &mut State, sec_id: &str, mode: StudyMode) {
    state
        .study_mode
        .insert(sec_id.to_string(), mode.as_str().to_string());
}

// ---- Question progress ----

/// Answered at least once and the last answer was right.
pub fn is_known(state: &State, qid: &str) -> bool {
    state
        .questions
        .get(qid)
        .map(|s| s.streak >= 1)
        .unwrap_or(false)
}

/// Answered (or rehearsed) at least once.
pub fn is_practiced(state: &State, qid: &str) -> bool {
    state.questions.get(qid).map(|s| s.n > 0).unwrap_or(false)
}

/// Card progress 0..=1: mastered = 1, otherwise SRS level / 5.
pub fn card_progress(state: &State, card_id: &str) -> f64 {
    match state.cards.get(card_id) {
        None => 0.0,
        Some(c) if c.m => 1.0,
        Some(c) => (c.l.clamp(0, crate::srs::MAX) as f64 / crate::srs::MAX as f64) * 0.9,
    }
}

fn difficulty_rank(q: &Question) -> u8 {
    use crate::content::Difficulty::*;
    match q.common().difficulty {
        Some(Easy) => 0,
        Some(Medium) | None => 1,
        Some(Hard) => 2,
    }
}

/// Pick up to `n` question ids: not yet `done` first, then by difficulty
/// (easy → hard), keeping bank order otherwise. Deterministic.
fn pick(
    qs: Vec<&Question>,
    state: &State,
    n: usize,
    done: fn(&State, &str) -> bool,
) -> Vec<String> {
    let mut qs: Vec<(usize, &Question)> = qs.into_iter().enumerate().collect();
    qs.sort_by_key(|(i, q)| (done(state, q.id()), difficulty_rank(q), *i));
    qs.into_iter()
        .take(n)
        .map(|(_, q)| q.id().to_string())
        .collect()
}

/// Questions per lesson self-check («Самопроверка»).
pub const SELF_CHECK_SIZE: usize = 6;
/// Questions per beit-din rehearsal of a section.
pub const REHEARSAL_SIZE: usize = 8;

/// «Самопроверка» after a lesson: its closed-format questions (mc, tf,
/// match, typed); open questions live in the beit-din box instead.
pub fn self_check_ids(
    lib: &Library,
    sec: &SectionMeta,
    lesson_id: &str,
    state: &State,
) -> Vec<String> {
    let user = Library::user_level(state);
    let qs = lib
        .lesson_questions(sec, lesson_id, user)
        .into_iter()
        .filter(|q| !q.is_open())
        .collect();
    pick(qs, state, SELF_CHECK_SIZE, is_known)
}

/// «Репетиция бейт-дина» for a section: its open questions, never
/// rehearsed first (a «частично» grade must not pin the same few forever).
pub fn rehearsal_ids(lib: &Library, sec: &SectionMeta, state: &State) -> Vec<String> {
    let user = Library::user_level(state);
    pick(
        lib.section_interview(sec, user),
        state,
        REHEARSAL_SIZE,
        is_practiced,
    )
}

// ---- Per-lesson prep ----

#[derive(Debug, Clone, Default, PartialEq)]
pub struct LessonPrep {
    pub read: bool,
    /// Closed-format questions of the lesson / answered right last time.
    pub checks: usize,
    pub checks_known: usize,
    /// Open (beit-din) questions of the lesson / rehearsed with a good self-grade.
    pub interview: usize,
    pub interview_known: usize,
}

impl LessonPrep {
    /// 0..=1 share of the lesson's questions answered right.
    pub fn ratio(&self) -> f64 {
        let total = self.checks + self.interview;
        if total == 0 {
            return 0.0;
        }
        (self.checks_known + self.interview_known) as f64 / total as f64
    }
}

pub fn lesson_prep(lib: &Library, sec: &SectionMeta, lesson_id: &str, state: &State) -> LessonPrep {
    let user = Library::user_level(state);
    let mut p = LessonPrep {
        read: Library::is_read(state, lesson_id),
        ..LessonPrep::default()
    };
    for q in lib.lesson_questions(sec, lesson_id, user) {
        let known = is_known(state, q.id());
        if q.is_open() {
            p.interview += 1;
            p.interview_known += known as usize;
        } else {
            p.checks += 1;
            p.checks_known += known as usize;
        }
    }
    p
}

// ---- Section readiness ----

/// Readiness bands shown as «Готовность к экзамену».
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Band {
    NotStarted,
    FirstSteps,
    InProgress,
    Almost,
    Ready,
}

impl Band {
    pub fn title(self) -> &'static str {
        match self {
            Band::NotStarted => "Ещё не начато",
            Band::FirstSteps => "Первые шаги",
            Band::InProgress => "В процессе",
            Band::Almost => "Почти готовы",
            Band::Ready => "Готовы к бейт-дину",
        }
    }
}

/// What to do next in a section.
#[derive(Debug, Clone, PartialEq)]
pub enum NextStep {
    /// Read this lesson (first unread one).
    Read(String),
    /// Run the self-check of this lesson.
    SelfCheck(String),
    /// Study the section's flashcards.
    Cards,
    /// Rehearse the beit-din questions of the section.
    Rehearse,
    /// Take the section test.
    Test,
    /// Everything done: keep repeating cards.
    Done,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Readiness {
    pub lessons: usize,
    pub read: usize,
    pub cards: usize,
    pub cards_mastered: usize,
    /// 0..=1 average card progress (SRS level based).
    pub cards_score: f64,
    pub checks: usize,
    pub checks_known: usize,
    pub interview: usize,
    pub interview_known: usize,
    pub best: Option<f64>,
    /// 0..=100 exam readiness.
    pub pct: i64,
    pub band: Band,
    pub next: NextStep,
}

/// Weights of the readiness score (sum = 1). Reading is tracked separately
/// («Прочитано»), but «Готовы» also requires every lesson read.
pub const W_CARDS: f64 = 0.2;
pub const W_CHECKS: f64 = 0.25;
pub const W_INTERVIEW: f64 = 0.2;
pub const W_TEST: f64 = 0.35;

fn ratio(a: usize, b: usize) -> f64 {
    if b == 0 {
        0.0
    } else {
        a as f64 / b as f64
    }
}

/// Score of one readiness part: a part with nothing in it (no cards, no
/// questions at this level) counts as done, so its weight is not lost.
fn part(a: usize, b: usize) -> f64 {
    if b == 0 {
        1.0
    } else {
        a as f64 / b as f64
    }
}

/// Section exam readiness (docs/STRUCTURE.md §Готовность).
pub fn readiness(lib: &Library, sec: &SectionMeta, state: &State) -> Readiness {
    let user: Level = Library::user_level(state);
    let lessons = lib.lessons(sec, user);
    let read = lessons
        .iter()
        .filter(|l| Library::is_read(state, &l.id))
        .count();
    let cards = lib.section_cards(sec, user);
    let cards_mastered = cards
        .iter()
        .filter(|c| crate::srs::is_mastered(state, &c.id))
        .count();
    let cards_score = if cards.is_empty() {
        1.0
    } else {
        cards
            .iter()
            .map(|c| card_progress(state, &c.id))
            .sum::<f64>()
            / cards.len() as f64
    };
    // Only questions a lesson self-check can reach (linked to a visible
    // lesson), so the per-lesson rows add up to the section total.
    let (mut checks, mut checks_known) = (0, 0);
    for q in lib.questions(sec, user).into_iter().filter(|q| {
        !q.is_open()
            && q.common()
                .lesson_id
                .as_deref()
                .is_some_and(|l| lessons.iter().any(|x| x.id == l))
    }) {
        checks += 1;
        checks_known += is_known(state, q.id()) as usize;
    }
    let interview_qs = lib.section_interview(sec, user);
    let interview = interview_qs.len();
    let interview_known = interview_qs
        .iter()
        .filter(|q| is_known(state, q.id()))
        .count();
    let best = Library::best(state, &sec.id);
    let test_score = (best.unwrap_or(0.0) / 100.0).clamp(0.0, 1.0);
    let score = W_CARDS * cards_score
        + W_CHECKS * part(checks_known, checks)
        + W_INTERVIEW * part(interview_known, interview)
        + W_TEST * test_score;
    let pct = (score * 100.0).round() as i64;
    let passed = best.map(|b| b >= PASS).unwrap_or(false);
    let all_read = read == lessons.len();
    let started = read > 0
        || best.is_some()
        || cards.iter().any(|c| state.cards.contains_key(&c.id))
        || lib
            .questions(sec, user)
            .iter()
            .any(|q| is_practiced(state, q.id()));

    let band = if !started {
        Band::NotStarted
    } else if pct < 35 {
        Band::FirstSteps
    } else if pct < 65 {
        Band::InProgress
    } else if pct < 80 || !passed || !all_read {
        Band::Almost
    } else {
        Band::Ready
    };

    // Next step: read in order; then close the weakest read lesson's
    // self-check; then cards, beit-din answers, the test; after that the
    // part with the biggest weighted gap until «Готовы».
    let weakest_lesson = || {
        lessons
            .iter()
            .map(|l| (l, lesson_prep(lib, sec, &l.id, state)))
            .filter(|(_, p)| p.checks > 0 && p.checks_known < p.checks)
            .min_by(|a, b| {
                ratio(a.1.checks_known, a.1.checks)
                    .partial_cmp(&ratio(b.1.checks_known, b.1.checks))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(l, p)| (l.id.clone(), ratio(p.checks_known, p.checks)))
    };
    let next = if let Some(l) = lessons.iter().find(|l| !Library::is_read(state, &l.id)) {
        NextStep::Read(l.id.clone())
    } else if let Some((id, _)) = weakest_lesson().filter(|(_, r)| *r < 0.6) {
        NextStep::SelfCheck(id)
    } else if !cards.is_empty() && cards_score < 0.5 {
        NextStep::Cards
    } else if interview > 0 && ratio(interview_known, interview) < 0.6 {
        NextStep::Rehearse
    } else if !passed {
        NextStep::Test
    } else if band != Band::Ready {
        let gaps = [
            (W_CARDS * (1.0 - cards_score), 0),
            (W_CHECKS * (1.0 - part(checks_known, checks)), 1),
            (W_INTERVIEW * (1.0 - part(interview_known, interview)), 2),
            (W_TEST * (1.0 - test_score), 3),
        ];
        let (_, which) = gaps
            .iter()
            .copied()
            .fold((-1.0, 3), |acc, g| if g.0 > acc.0 { g } else { acc });
        match which {
            0 => NextStep::Cards,
            1 => weakest_lesson()
                .map(|(id, _)| NextStep::SelfCheck(id))
                .unwrap_or(NextStep::Test),
            2 => NextStep::Rehearse,
            _ => NextStep::Test,
        }
    } else if cards_mastered < cards.len() {
        NextStep::Cards
    } else {
        NextStep::Done
    };

    Readiness {
        lessons: lessons.len(),
        read,
        cards: cards.len(),
        cards_mastered,
        cards_score,
        checks,
        checks_known,
        interview,
        interview_known,
        best,
        pct,
        band,
        next,
    }
}

/// Reading progress 0..=100 for the «Читать» tab.
pub fn read_pct(r: &Readiness) -> i64 {
    (ratio(r.read, r.lessons) * 100.0).round() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::QuestionStat;

    fn lib() -> Library {
        Library::embedded()
    }

    fn know(state: &mut State, id: &str) {
        state.questions.insert(
            id.to_string(),
            QuestionStat {
                n: 1,
                miss: 0.0,
                streak: 1,
                last: "2026-11-22".into(),
            },
        );
    }

    #[test]
    fn fresh_state_is_not_started_and_reads_first_lesson() {
        let lib = lib();
        let state = State::default();
        let sec = lib.section("shabbat").unwrap();
        let r = readiness(&lib, sec, &state);
        assert_eq!(r.pct, 0);
        assert_eq!(r.band, Band::NotStarted);
        assert_eq!(r.next, NextStep::Read("shabbat-1".into()));
        assert_eq!(read_pct(&r), 0);
        assert!(r.checks > 0 && r.interview > 0 && r.cards > 0);
    }

    #[test]
    fn default_mode_follows_reading_then_remembers_choice() {
        let lib = lib();
        let mut state = State::default();
        let sec = lib.section("osnovy").unwrap();
        assert_eq!(mode_for(&lib, sec, &state), StudyMode::Read);
        for l in lib.lessons(sec, Library::user_level(&state)) {
            state.lessons_read.insert(l.id.clone(), "2026-11-22".into());
        }
        assert_eq!(mode_for(&lib, sec, &state), StudyMode::Prep);
        set_mode(&mut state, "osnovy", StudyMode::Read);
        assert_eq!(mode_for(&lib, sec, &state), StudyMode::Read);
        // The choice survives an export/import round trip.
        let back = State::import_json(&state.export_json(), "2026-11-22").unwrap();
        assert_eq!(mode_for(&lib, sec, &back), StudyMode::Read);
    }

    #[test]
    fn everything_done_is_ready() {
        let lib = lib();
        let mut state = State::default();
        let sec = lib.section("kashrut").unwrap();
        let user = Library::user_level(&state);
        for l in lib.lessons(sec, user) {
            state.lessons_read.insert(l.id.clone(), "2026-11-22".into());
        }
        for q in lib.questions(sec, user) {
            know(&mut state, q.id());
        }
        for c in lib.section_cards(sec, user) {
            let cs = state.cards.entry(c.id.clone()).or_default();
            cs.l = 5;
            cs.m = true;
        }
        state.best.insert("kashrut".into(), 100.0);
        let r = readiness(&lib, sec, &state);
        assert_eq!(r.pct, 100);
        assert_eq!(r.band, Band::Ready);
        assert_eq!(r.next, NextStep::Done);
        assert_eq!(read_pct(&r), 100);
    }

    #[test]
    fn high_score_without_passing_test_is_only_almost() {
        let lib = lib();
        let mut state = State::default();
        let sec = lib.section("kashrut").unwrap();
        let user = Library::user_level(&state);
        for l in lib.lessons(sec, user) {
            state.lessons_read.insert(l.id.clone(), "2026-11-22".into());
        }
        for q in lib.questions(sec, user) {
            know(&mut state, q.id());
        }
        for c in lib.section_cards(sec, user) {
            let cs = state.cards.entry(c.id.clone()).or_default();
            cs.m = true;
        }
        state.best.insert("kashrut".into(), 70.0);
        let r = readiness(&lib, sec, &state);
        assert!(r.pct >= 80, "pct {}", r.pct);
        assert_eq!(r.band, Band::Almost);
        assert_eq!(r.next, NextStep::Test);
    }

    #[test]
    fn next_step_order_after_reading() {
        let lib = lib();
        let mut state = State::default();
        let sec = lib.section("etika").unwrap();
        let user = Library::user_level(&state);
        let lessons = lib.lessons(sec, user);
        for l in &lessons {
            state.lessons_read.insert(l.id.clone(), "2026-11-22".into());
        }
        // Nothing practised yet → self-check of the first lesson.
        let r = readiness(&lib, sec, &state);
        assert_eq!(r.next, NextStep::SelfCheck(lessons[0].id.clone()));
        // All closed questions known → cards next.
        for q in lib
            .questions(sec, user)
            .into_iter()
            .filter(|q| !q.is_open())
        {
            know(&mut state, q.id());
        }
        assert_eq!(readiness(&lib, sec, &state).next, NextStep::Cards);
        for c in lib.section_cards(sec, user) {
            state.cards.entry(c.id.clone()).or_default().l = 4;
        }
        assert_eq!(readiness(&lib, sec, &state).next, NextStep::Rehearse);
        for q in lib.section_interview(sec, user) {
            know(&mut state, q.id());
        }
        assert_eq!(readiness(&lib, sec, &state).next, NextStep::Test);
    }

    #[test]
    fn self_check_takes_closed_questions_unknown_first() {
        let lib = lib();
        let mut state = State::default();
        let sec = lib.section("shabbat").unwrap();
        let ids = self_check_ids(&lib, sec, "shabbat-1", &state);
        assert!(!ids.is_empty() && ids.len() <= SELF_CHECK_SIZE);
        for id in &ids {
            let (q, _) = lib.question(id).unwrap();
            assert!(!q.is_open());
            assert_eq!(q.common().lesson_id.as_deref(), Some("shabbat-1"));
        }
        // Once the first pick is known it moves behind the unknown ones.
        know(&mut state, &ids[0]);
        let again = self_check_ids(&lib, sec, "shabbat-1", &state);
        let all = lib
            .lesson_questions(sec, "shabbat-1", Library::user_level(&state))
            .into_iter()
            .filter(|q| !q.is_open())
            .count();
        if all > SELF_CHECK_SIZE {
            assert!(!again.contains(&ids[0]));
        } else {
            assert_eq!(again.last(), Some(&ids[0]));
        }
    }

    #[test]
    fn rehearsal_uses_open_questions_of_the_section() {
        let lib = lib();
        let state = State::default();
        let sec = lib.section("beit-din").unwrap();
        let ids = rehearsal_ids(&lib, sec, &state);
        assert_eq!(ids.len(), REHEARSAL_SIZE);
        for id in &ids {
            let (q, s) = lib.question(id).unwrap();
            assert!(q.is_open());
            assert_eq!(s.id, "beit-din");
        }
    }

    #[test]
    fn lesson_prep_counts_split_by_format() {
        let lib = lib();
        let mut state = State::default();
        let sec = lib.section("osnovy").unwrap();
        let p = lesson_prep(&lib, sec, "osnovy-1", &state);
        assert!(!p.read && p.checks > 0 && p.interview >= 1);
        assert_eq!(p.ratio(), 0.0);
        let first_open = lib.lesson_interview(sec, "osnovy-1", Level::Orthodox)[0]
            .id()
            .to_string();
        know(&mut state, &first_open);
        state
            .lessons_read
            .insert("osnovy-1".into(), "2026-11-22".into());
        let p = lesson_prep(&lib, sec, "osnovy-1", &state);
        assert!(p.read);
        assert_eq!(p.interview_known, 1);
        assert!(p.ratio() > 0.0);
    }

    #[test]
    fn card_progress_scale() {
        let mut state = State::default();
        assert_eq!(card_progress(&state, "x"), 0.0);
        state.cards.entry("x".into()).or_default().l = 5;
        assert!(card_progress(&state, "x") < 1.0);
        state.cards.get_mut("x").unwrap().m = true;
        assert_eq!(card_progress(&state, "x"), 1.0);
    }

    #[test]
    fn weights_sum_to_one() {
        assert!((W_CARDS + W_CHECKS + W_INTERVIEW + W_TEST - 1.0).abs() < 1e-9);
    }

    #[test]
    fn done_only_when_ready() {
        // Everything read and mastered, 60% of both question kinds, test 80%:
        // 0.20 + 0.15 + 0.12 + 0.28 = 75% → «Почти готовы», not «пройден».
        let lib = lib();
        let mut state = State::default();
        let sec = lib.section("kashrut").unwrap();
        let user = Library::user_level(&state);
        for l in lib.lessons(sec, user) {
            state.lessons_read.insert(l.id.clone(), "2026-11-22".into());
        }
        for c in lib.section_cards(sec, user) {
            state.cards.entry(c.id.clone()).or_default().m = true;
        }
        let closed: Vec<_> = lib
            .questions(sec, user)
            .into_iter()
            .filter(|q| !q.is_open())
            .collect();
        for q in closed.iter().take((closed.len() * 6).div_ceil(10)) {
            know(&mut state, q.id());
        }
        let open = lib.section_interview(sec, user);
        for q in open.iter().take((open.len() * 6).div_ceil(10)) {
            know(&mut state, q.id());
        }
        state.best.insert("kashrut".into(), 80.0);
        let r = readiness(&lib, sec, &state);
        assert!(r.pct < 80, "pct {}", r.pct);
        assert_eq!(r.band, Band::Almost);
        assert_ne!(r.next, NextStep::Done);
    }

    fn tiny_lib(cards: bool) -> Library {
        let manifest = r#"{"sections":[{"id":"t","file":"t.json"}]}"#;
        let section = format!(
            r#"{{"id":"t","lessons":[{{"id":"t-1","cards":[{}]}}],
               "questions":[{{"id":"t-q1","lessonId":"t-1","format":"tf","difficulty":"easy","answer":true}}]}}"#,
            if cards { r#"{"id":"t-1-c1"}"# } else { "" }
        );
        Library::from_json(manifest, &[section.as_str()]).unwrap()
    }

    #[test]
    fn empty_parts_count_as_done() {
        // No cards and no open questions: the remaining parts can still
        // reach 100% and «Готовы».
        let lib = tiny_lib(false);
        let sec = &lib.sections[0];
        let mut state = State::default();
        let r0 = readiness(&lib, sec, &state);
        assert_eq!(r0.band, Band::NotStarted);
        assert_eq!(r0.next, NextStep::Read("t-1".into()));
        state.lessons_read.insert("t-1".into(), "2026-11-22".into());
        know(&mut state, "t-q1");
        state.best.insert("t".into(), 100.0);
        let r = readiness(&lib, sec, &state);
        assert_eq!(r.pct, 100);
        assert_eq!(r.band, Band::Ready);
        assert_eq!(r.next, NextStep::Done);
        // With a card that is not mastered yet: ready by score, cards next.
        let lib = tiny_lib(true);
        let r = readiness(&lib, &lib.sections[0], &state);
        assert_eq!(r.pct, 80);
        assert_eq!(r.next, NextStep::Cards);
    }

    #[test]
    fn band_edges() {
        let lib = tiny_lib(true);
        let sec = &lib.sections[0];
        let mut state = State::default();
        state.lessons_read.insert("t-1".into(), "2026-11-22".into());
        // One unpractised card: read only = the empty interview part (20%).
        assert_eq!(readiness(&lib, sec, &state).band, Band::FirstSteps);
        know(&mut state, "t-q1"); // +25 → 45%
        assert_eq!(readiness(&lib, sec, &state).band, Band::InProgress);
        state.best.insert("t".into(), 70.0); // +24.5 → 70%, test not passed
        assert_eq!(readiness(&lib, sec, &state).band, Band::Almost);
        state.best.insert("t".into(), 80.0); // 73% < 80
        assert_eq!(readiness(&lib, sec, &state).band, Band::Almost);
        state.best.insert("t".into(), 100.0); // 80%, passed, all read
        assert_eq!(readiness(&lib, sec, &state).band, Band::Ready);
    }

    #[test]
    fn rehearsal_moves_on_after_a_partial_grade() {
        let lib = lib();
        let mut state = State::default();
        let sec = lib.section("beit-din").unwrap();
        let first = rehearsal_ids(&lib, sec, &state);
        // Rehearsed but graded «частично» (streak 0): still not known…
        for id in &first {
            state.questions.insert(
                id.clone(),
                QuestionStat {
                    n: 1,
                    miss: 0.5,
                    streak: 0,
                    last: "2026-11-22".into(),
                },
            );
        }
        // …yet the next rehearsal brings new questions.
        let second = rehearsal_ids(&lib, sec, &state);
        assert!(second.iter().all(|id| !first.contains(id)));
    }

    #[test]
    fn practice_runs_keep_the_picked_order_and_do_not_touch_best() {
        use crate::exam::{self, BuildOpts, Mode};
        let lib = lib();
        let mut state = State::default();
        let sec = lib.section("shabbat").unwrap();
        let ids = self_check_ids(&lib, sec, "shabbat-2", &state);
        let mut rng = rand::thread_rng();
        let session = exam::build(
            &lib,
            &state,
            BuildOpts {
                mode: Mode::SelfCheck,
                section_id: Some("shabbat".into()),
                count: None,
                only_unlocked: None,
                ids: Some(ids.clone()),
            },
            &mut rng,
        );
        let got: Vec<String> = session.items.iter().map(|i| i.q.id().to_string()).collect();
        assert_eq!(got, ids);
        let today = chrono::NaiveDate::from_ymd_opt(2026, 11, 22).unwrap();
        let res = exam::finish(&lib, &session, &mut state, today);
        assert_eq!(res.mode, Mode::SelfCheck);
        assert!(!state.best.contains_key("shabbat"));
        assert_eq!(
            state.exams.last().and_then(|e| e["mode"].as_str()),
            Some("selfcheck")
        );
    }

    #[test]
    fn null_study_mode_does_not_lose_progress() {
        let text = r#"{"app":"giyur-study","settings":{"level":"orthodox"},"xpTotal":42,"studyMode":null}"#;
        let s = State::import_json(text, "2026-11-22").unwrap();
        assert_eq!(s.xp_total, 42);
        assert!(s.study_mode.is_empty());
    }

    #[test]
    fn reading_alone_is_first_steps_not_not_started() {
        let lib = lib();
        let mut state = State::default();
        let sec = lib.section("kashrut").unwrap();
        for l in lib.lessons(sec, Library::user_level(&state)) {
            state.lessons_read.insert(l.id.clone(), "2026-11-22".into());
        }
        let r = readiness(&lib, sec, &state);
        assert_eq!(r.pct, 0);
        assert_eq!(r.band, Band::FirstSteps);
    }

    #[test]
    fn practice_does_not_push_exams_out_of_history() {
        let mut state = State::default();
        crate::exam::log_exam(
            &mut state,
            serde_json::json!({"mode": "section", "pct": 90}),
        );
        for _ in 0..(crate::exam::PRACTICE_LOG_CAP + 30) {
            crate::exam::log_exam(&mut state, serde_json::json!({"mode": "selfcheck"}));
        }
        assert_eq!(state.exams[0]["mode"], "section");
        assert_eq!(state.exams.len(), crate::exam::PRACTICE_LOG_CAP + 1);
        for _ in 0..(crate::exam::EXAM_LOG_CAP + 5) {
            crate::exam::log_exam(&mut state, serde_json::json!({"mode": "general"}));
        }
        let exams = state
            .exams
            .iter()
            .filter(|e| e["mode"] != "selfcheck")
            .count();
        assert_eq!(exams, crate::exam::EXAM_LOG_CAP);
        assert!(state.exams.iter().all(|e| e["mode"] != "section"));
    }
}
