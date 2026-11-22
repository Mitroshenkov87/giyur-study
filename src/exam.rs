//! Exam engine ported from js/exam.js plus the timed «Кто хочет стать
//! миллионером»-style exam logic of js/ui/timed.js.

use chrono::NaiveDate;
use rand::Rng;

use crate::content::{Difficulty, Library, Question, SectionMeta};
use crate::game::{self, AwardOutcome};
use crate::storage::State;
use crate::util;

// ---- Modes ----

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Section,
    General,
    Beitdin,
    Retry,
    Timed,
    /// «Самопроверка» after a lesson (docs/STRUCTURE.md), built from ids.
    SelfCheck,
    /// «Репетиция бейт-дина» of one section: its open questions, from ids.
    Rehearsal,
}

impl Mode {
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Section => "section",
            Mode::General => "general",
            Mode::Beitdin => "beitdin",
            Mode::Retry => "retry",
            Mode::Timed => "timed",
            Mode::SelfCheck => "selfcheck",
            Mode::Rehearsal => "rehearsal",
        }
    }
}

/// js/exam.js E.MODE_TITLES.
pub fn mode_title(mode: Mode) -> &'static str {
    match mode {
        Mode::Section => "Тест раздела",
        Mode::General => "Общий экзамен",
        Mode::Beitdin => "Пробный бейт-дин",
        Mode::Retry => "Работа над ошибками",
        Mode::Timed => "Экзамен (на время)",
        Mode::SelfCheck => "Самопроверка",
        Mode::Rehearsal => "Репетиция бейт-дина",
    }
}

// ---- Weighting ----

/// Questions answered wrong come up more often; recent correct answers less
/// (js/exam.js E.weight).
pub fn weight(state: &State, qid: &str) -> f64 {
    let s = state.questions.get(qid);
    let miss = s.map(|x| x.miss).unwrap_or(0.0);
    let streak = s.map(|x| x.streak).unwrap_or(0);
    (1.0 + 2.0 * miss - 0.5 * streak.min(3) as f64).max(0.25)
}

// ---- Difficulty tiers (timed exam ramp) ----

/// js/exam.js E.DIFF_TITLES.
pub fn diff_title(d: Option<Difficulty>) -> &'static str {
    match d {
        Some(Difficulty::Easy) => "лёгкий",
        Some(Difficulty::Hard) => "трудный",
        _ => "средний",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TierCounts {
    pub easy: usize,
    pub medium: usize,
    pub hard: usize,
}

/// Quota per tier for n questions: 25% easy (warm-up), 40% medium, the rest
/// hard. For 20 questions: 5 easy, 8 medium, 7 hard (js/exam.js E.tierCounts).
pub fn tier_counts(n: usize) -> TierCounts {
    let easy = (n as f64 * 0.25).round() as usize;
    let medium = (n as f64 * 0.4).round() as usize;
    TierCounts {
        easy,
        medium,
        hard: n - easy - medium,
    }
}

pub struct Tiers<'q> {
    pub easy: Vec<&'q Question>,
    pub medium: Vec<&'q Question>,
    pub hard: Vec<&'q Question>,
}

impl<'q> Tiers<'q> {
    fn len(&self, t: Tier) -> usize {
        match t {
            Tier::Easy => self.easy.len(),
            Tier::Medium => self.medium.len(),
            Tier::Hard => self.hard.len(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tier {
    Easy,
    Medium,
    Hard,
}

impl Tier {
    const ALL: [Tier; 3] = [Tier::Easy, Tier::Medium, Tier::Hard];
}

/// Split a pool into difficulty tiers; missing difficulty = medium
/// (js/exam.js E.splitTiers).
pub fn split_tiers<'q>(pool: &[&'q Question]) -> Tiers<'q> {
    let mut tiers = Tiers {
        easy: Vec::new(),
        medium: Vec::new(),
        hard: Vec::new(),
    };
    for q in pool {
        match q.common().difficulty {
            Some(Difficulty::Easy) => tiers.easy.push(q),
            Some(Difficulty::Hard) => tiers.hard.push(q),
            _ => tiers.medium.push(q),
        }
    }
    tiers
}

/// Adjust tier quotas to what is actually available, borrowing from a
/// neighbouring tier so the total stays n when the pool allows
/// (js/exam.js E.planTiers).
fn tier_count_mut(counts: &mut TierCounts, t: Tier) -> &mut usize {
    match t {
        Tier::Easy => &mut counts.easy,
        Tier::Medium => &mut counts.medium,
        Tier::Hard => &mut counts.hard,
    }
}

pub fn plan_tiers(n: usize, tiers: &Tiers) -> TierCounts {
    let mut counts = tier_counts(n);
    for _ in 0..6 {
        let over = Tier::ALL
            .into_iter()
            .find(|&t| *tier_count_mut(&mut counts, t) > tiers.len(t));
        let Some(over) = over else { break };
        let mut deficit = *tier_count_mut(&mut counts, over) - tiers.len(over);
        *tier_count_mut(&mut counts, over) = tiers.len(over);
        let neighbours: &[Tier] = match over {
            Tier::Medium => &[Tier::Easy, Tier::Hard],
            _ => &[Tier::Medium],
        };
        for &nb in neighbours {
            if deficit == 0 {
                break;
            }
            let spare = tiers
                .len(nb)
                .saturating_sub(*tier_count_mut(&mut counts, nb));
            let add = spare.min(deficit);
            *tier_count_mut(&mut counts, nb) += add;
            deficit -= add;
        }
    }
    counts
}

/// Ids of questions from the last 3 timed runs (js/exam.js E.recentTimedIds).
pub fn recent_timed_ids(state: &State) -> Vec<&str> {
    state
        .exam_history
        .iter()
        .flat_map(|run| run.iter().map(|s| s.as_str()))
        .collect()
}

/// Log the ids of a timed run, keeping the last 3 (js/exam.js E.logTimed).
pub fn log_timed(state: &mut State, ids: Vec<String>) {
    state.exam_history.push(ids);
    if state.exam_history.len() > 3 {
        let keep = state.exam_history.split_off(state.exam_history.len() - 3);
        state.exam_history = keep;
    }
}

/// Timed-exam selection: tiered ramp (easy → medium → hard), weighted within
/// a tier, avoiding questions from the last 3 timed runs unless the pool is
/// too small (js/exam.js E.pickTimed).
pub fn pick_timed<'q>(
    pool: &[&'q Question],
    n: usize,
    state: &State,
    rng: &mut impl Rng,
) -> Vec<&'q Question> {
    let tiers = split_tiers(pool);
    let counts = plan_tiers(n, &tiers);
    let recent = recent_timed_ids(state);
    let mut picked: Vec<&Question> = Vec::new();
    for (tier, bank) in [
        (Tier::Easy, &tiers.easy),
        (Tier::Medium, &tiers.medium),
        (Tier::Hard, &tiers.hard),
    ] {
        let want = match tier {
            Tier::Easy => counts.easy,
            Tier::Medium => counts.medium,
            Tier::Hard => counts.hard,
        };
        if want == 0 {
            continue;
        }
        let fresh: Vec<&Question> = bank
            .iter()
            .copied()
            .filter(|q| !recent.contains(&q.id()))
            .collect();
        let bank2 = if fresh.len() < want {
            bank.clone()
        } else {
            fresh
        };
        picked.extend(util::weighted_sample(
            &bank2,
            |q| weight(state, q.id()),
            want,
            rng,
        ));
    }
    picked
}

// ---- Regular exam sessions ----

/// A question prepared for presentation (js/exam.js E.prepare).
pub struct Item<'a> {
    pub q: &'a Question,
    pub sec: Option<&'a SectionMeta>,
    /// mc: shuffled display order of option indices
    pub order: Option<Vec<usize>>,
    /// match: shuffled right-hand side
    pub rights: Option<Vec<String>>,
}

/// Response by format: mc → original option index; tf → bool; typed → string;
/// match → chosen right strings aligned with the pairs; open → 0 | 0.5 | 1.
#[derive(Debug, Clone, PartialEq)]
pub enum Response {
    Mc(usize),
    Tf(bool),
    Typed(String),
    Match(Vec<String>),
    Open(f64),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CheckResult {
    pub score: f64,
    pub correct: bool,
    pub user: String,
}

pub struct Session<'a> {
    pub mode: Mode,
    pub section_id: Option<String>,
    pub items: Vec<Item<'a>>,
    pub idx: usize,
    pub answers: Vec<Option<CheckResult>>,
    pub started_at: i64,
}

impl<'a> Session<'a> {
    /// Record an answer in the session and per-question stats (js/exam.js E.answer).
    pub fn answer(
        &mut self,
        state: &mut State,
        response: Response,
        today: NaiveDate,
    ) -> CheckResult {
        let item = &self.items[self.idx];
        let r = check(item, &response);
        if self.answers.len() <= self.idx {
            self.answers.resize(self.idx + 1, None);
        }
        self.answers[self.idx] = Some(r.clone());
        record(state, item.q.id(), &r, today);
        r
    }
}

/// Options for building an exam (js/exam.js E.build opts).
pub struct BuildOpts {
    pub mode: Mode,
    pub section_id: Option<String>,
    pub count: Option<usize>,
    pub only_unlocked: Option<bool>,
    pub ids: Option<Vec<String>>,
}

fn prepare<'a>(lib: &'a Library, q: &'a Question, rng: &mut impl Rng) -> Item<'a> {
    let sec = lib.question(q.id()).map(|(_, s)| s);
    let mut item = Item {
        q,
        sec,
        order: None,
        rights: None,
    };
    match q {
        Question::Mc { options, .. } => {
            let idx: Vec<usize> = (0..options.len()).collect();
            item.order = Some(util::shuffle(&idx, rng));
        }
        Question::Match { pairs, .. } => {
            let rights: Vec<String> = pairs.iter().map(|p| p.1.clone()).collect();
            item.rights = Some(util::shuffle(&rights, rng));
        }
        _ => {}
    }
    item
}

/// Build an exam session (js/exam.js E.build).
pub fn build<'a>(
    lib: &'a Library,
    state: &State,
    opts: BuildOpts,
    rng: &mut impl Rng,
) -> Session<'a> {
    let n = opts.count.unwrap_or(20);
    let user = Library::user_level(state);
    let w = |q: &Question| weight(state, q.id());
    let picked: Vec<&Question> = match opts.mode {
        Mode::Section => {
            let sec = opts.section_id.as_deref().and_then(|id| lib.section(id));
            let pool: Vec<&Question> = sec.map(|s| lib.questions(s, user)).unwrap_or_default();
            util::weighted_sample(&pool, |q| w(q), n, rng)
        }
        Mode::General => {
            let only_unlocked = opts.only_unlocked.unwrap_or(true);
            let secs: Vec<&SectionMeta> = if only_unlocked {
                lib.unlocked_sections(state)
            } else {
                lib.sections.iter().filter(|s| s.available).collect()
            };
            let pool: Vec<&Question> = secs.iter().flat_map(|s| lib.questions(s, user)).collect();
            util::weighted_sample(&pool, |q| w(q), n, rng)
        }
        Mode::Beitdin => {
            // ~70% open questions from the beit-din section, the rest
            // open/typed from others.
            let bd = lib.section("beit-din");
            let bd_pool: Vec<&Question> = match bd {
                Some(s) if s.available => lib
                    .questions(s, user)
                    .into_iter()
                    .filter(|q| matches!(q, Question::Open { .. }))
                    .collect(),
                _ => Vec::new(),
            };
            let others: Vec<&Question> = lib
                .sections
                .iter()
                .filter(|s| s.available && s.id != "beit-din")
                .flat_map(|s| lib.questions(s, user))
                .filter(|q| matches!(q, Question::Open { .. } | Question::Typed { .. }))
                .collect();
            let nb = bd_pool.len().min((n as f64 * 0.7).round() as usize);
            let mut picked = util::weighted_sample(&bd_pool, |q| w(q), nb, rng);
            let need = n - picked.len();
            picked.extend(util::weighted_sample(&others, |q| w(q), need, rng));
            if picked.len() < n {
                // top up from remaining beit-din questions
                let rest: Vec<&Question> = match bd {
                    Some(s) if s.available => lib
                        .questions(s, user)
                        .into_iter()
                        .filter(|q| !picked.iter().any(|p| p.id() == q.id()))
                        .collect(),
                    _ => Vec::new(),
                };
                let need = n - picked.len();
                picked.extend(util::weighted_sample(&rest, |q| w(q), need, rng));
            }
            util::shuffle(&picked, rng)
        }
        Mode::Retry | Mode::SelfCheck | Mode::Rehearsal => {
            let ids = opts.ids.clone().unwrap_or_default();
            let pool: Vec<&Question> = ids
                .iter()
                .filter_map(|id| lib.question(id).map(|(q, _)| q))
                .collect();
            util::shuffle(&pool, rng)
        }
        // The timed exam is built by build_timed, not E.build.
        Mode::Timed => Vec::new(),
    };
    let items = picked.into_iter().map(|q| prepare(lib, q, rng)).collect();
    Session {
        mode: opts.mode,
        section_id: opts.section_id,
        items,
        idx: 0,
        answers: Vec::new(),
        started_at: chrono::Local::now().timestamp_millis(),
    }
}

/// js/exam.js E.correctText.
pub fn correct_text(q: &Question) -> String {
    match q {
        Question::Mc {
            options, answer, ..
        } => options.get(*answer).cloned().unwrap_or_default(),
        Question::Tf { answer, .. } => {
            if *answer {
                "Верно".to_string()
            } else {
                "Неверно".to_string()
            }
        }
        Question::Typed { answer, .. } => answer.clone(),
        Question::Match { pairs, .. } => pairs
            .iter()
            .map(|p| format!("{} — {}", p.0, p.1))
            .collect::<Vec<_>>()
            .join("; "),
        Question::Open { model_answer, .. } => model_answer.clone(),
    }
}

/// Check a response (js/exam.js E.check).
pub fn check(item: &Item, response: &Response) -> CheckResult {
    match (item.q, response) {
        (
            Question::Mc {
                options, answer, ..
            },
            Response::Mc(r),
        ) => {
            let score = if r == answer { 1.0 } else { 0.0 };
            CheckResult {
                score,
                correct: score == 1.0,
                user: options.get(*r).cloned().unwrap_or_else(|| "—".to_string()),
            }
        }
        (Question::Tf { answer, .. }, Response::Tf(r)) => {
            let score = if r == answer { 1.0 } else { 0.0 };
            CheckResult {
                score,
                correct: score == 1.0,
                user: if *r { "Верно" } else { "Неверно" }.to_string(),
            }
        }
        (Question::Typed { answer, accept, .. }, Response::Typed(r)) => {
            let a = util::normalize(r);
            let ok: Vec<String> = std::iter::once(answer.clone())
                .chain(accept.iter().cloned())
                .map(|x| util::normalize(&x))
                .collect();
            let score = if !a.is_empty() && ok.contains(&a) {
                1.0
            } else {
                0.0
            };
            CheckResult {
                score,
                correct: score == 1.0,
                user: if r.is_empty() {
                    "—".to_string()
                } else {
                    r.clone()
                },
            }
        }
        (Question::Match { pairs, .. }, Response::Match(r)) => {
            let mut good = 0usize;
            for (i, p) in pairs.iter().enumerate() {
                if r.get(i) == Some(&p.1) {
                    good += 1;
                }
            }
            // js/exam.js: good === q.pairs.length ? 1 : good / q.pairs.length,
            // so an empty pairs list scores 1 (0 === 0).
            let score = if good == pairs.len() {
                1.0
            } else {
                good as f64 / pairs.len() as f64
            };
            let user = pairs
                .iter()
                .enumerate()
                .map(|(i, p)| format!("{} — {}", p.0, r.get(i).map(|s| s.as_str()).unwrap_or("?")))
                .collect::<Vec<_>>()
                .join("; ");
            CheckResult {
                score,
                correct: score == 1.0,
                user,
            }
        }
        (Question::Open { .. }, Response::Open(r)) => {
            let score = *r;
            let user = if score == 1.0 {
                "Ответил верно"
            } else if score == 0.5 {
                "Частично"
            } else {
                "Не ответил"
            };
            CheckResult {
                score,
                correct: score == 1.0,
                user: user.to_string(),
            }
        }
        _ => CheckResult {
            score: 0.0,
            correct: false,
            user: "—".to_string(),
        },
    }
}

/// Per-question stats, shared by all modes (js/exam.js E.record).
pub fn record(state: &mut State, qid: &str, r: &CheckResult, today: NaiveDate) {
    let st = state.questions.entry(qid.to_string()).or_default();
    st.n += 1;
    if r.correct {
        st.streak += 1;
    } else {
        st.miss += if r.score > 0.0 { 0.5 } else { 1.0 };
        st.streak = 0;
    }
    st.last = util::day_key(today);
    state.touch(&util::day_key(today));
}

/// Append a finished run to the exam history, keeping the last 100
/// (js/exam.js E.logExam).
pub fn log_exam(state: &mut State, entry: serde_json::Value) {
    state.exams.push(entry);
    if state.exams.len() > 100 {
        let keep = state.exams.split_off(state.exams.len() - 100);
        state.exams = keep;
    }
}

pub struct ExamResult {
    pub mode: Mode,
    pub section_id: Option<String>,
    pub total: usize,
    pub score: f64,
    pub pct: i64,
    pub passed: bool,
    /// Indices of items answered incorrectly.
    pub mistakes: Vec<usize>,
    pub at: i64,
    pub xp: i64,
    pub award: AwardOutcome,
}

/// Finish a session: results, history, best score, XP (js/exam.js E.finish).
pub fn finish(lib: &Library, session: &Session, state: &mut State, today: NaiveDate) -> ExamResult {
    let total = session.items.len();
    let mut sum = 0.0;
    let mut mistakes = Vec::new();
    for (i, _) in session.items.iter().enumerate() {
        let a = session
            .answers
            .get(i)
            .and_then(|a| a.clone())
            .unwrap_or(CheckResult {
                score: 0.0,
                correct: false,
                user: "—".to_string(),
            });
        sum += a.score;
        if !a.correct {
            mistakes.push(i);
        }
    }
    let pct = if total > 0 {
        (sum / total as f64 * 100.0).round() as i64
    } else {
        0
    };
    let passed = pct as f64 >= crate::content::PASS;
    let at = chrono::Local::now().timestamp_millis();
    log_exam(
        state,
        serde_json::json!({
            "at": at,
            "mode": session.mode.as_str(),
            "sectionId": session.section_id,
            "total": total,
            "score": sum,
            "pct": pct,
        }),
    );
    if session.mode == Mode::Section {
        if let Some(sec_id) = &session.section_id {
            let best = state.best.get(sec_id).copied();
            if best.is_none() || pct as f64 > best.unwrap() {
                state.best.insert(sec_id.clone(), pct as f64);
            }
        }
    }
    // XP: section test passed +20 (+10 for 100%), mock beit-din +15,
    // general exam +2 per correct answer, mistakes retry +1 per correct answer.
    let right = (total - mistakes.len()) as i64;
    let xp = match session.mode {
        Mode::Section => {
            if passed {
                game::XP.test + if pct == 100 { game::XP.perfect } else { 0 }
            } else {
                0
            }
        }
        Mode::Beitdin => {
            game::count(state, "beitdin", 1);
            game::XP.beitdin
        }
        Mode::General => game::XP.exam_right * right,
        _ => game::XP.retry_right * right,
    };
    let award = game::award(lib, state, xp, today);
    ExamResult {
        mode: session.mode,
        section_id: session.section_id.clone(),
        total,
        score: sum,
        pct,
        passed,
        mistakes,
        at,
        xp,
        award,
    }
}

// ---- Timed exam («Режим телевикторины», js/ui/timed.js) ----

/// Question counts offered on the setup screen; the default is 20.
pub const TIMED_COUNTS: [usize; 4] = [10, 15, 20, 30];
pub const TIMED_DEFAULT_COUNT: usize = 20;

/// Base seconds per question from settings (20 | 30).
pub fn base_seconds(state: &State) -> i64 {
    if state.settings.timed_seconds == 20 {
        20
    } else {
        30
    }
}

/// Easy questions get +5 s on top of the base (js/ui/timed.js qSeconds).
pub fn q_seconds(q: &Question, state: &State) -> i64 {
    let b = base_seconds(state);
    b + if q.common().difficulty == Some(Difficulty::Easy) {
        5
    } else {
        0
    }
}

/// Question pool: mc (with ≥2 options) + tf from unlocked sections,
/// filtered by level (js/ui/timed.js pool).
pub fn timed_pool<'a>(lib: &'a Library, state: &State) -> Vec<&'a Question> {
    let user = Library::user_level(state);
    let mut out = Vec::new();
    for sec in lib.unlocked_sections(state) {
        for q in lib.questions(sec, user) {
            match q {
                Question::Mc { options, .. } if options.len() >= 2 => out.push(q),
                Question::Tf { .. } => out.push(q),
                _ => {}
            }
        }
    }
    out
}

#[derive(Debug, Clone, PartialEq)]
pub struct TimedOption {
    pub text: String,
    pub right: bool,
}

pub struct TimedItem<'a> {
    pub q: &'a Question,
    pub sec: Option<&'a SectionMeta>,
    pub opts: Vec<TimedOption>,
    pub seconds: i64,
}

fn timed_prepare<'a>(
    lib: &'a Library,
    q: &'a Question,
    state: &State,
    rng: &mut impl Rng,
) -> TimedItem<'a> {
    let sec = lib.question(q.id()).map(|(_, s)| s);
    let opts = match q {
        Question::Mc {
            options, answer, ..
        } => {
            let idx: Vec<usize> = (0..options.len()).collect();
            util::shuffle(&idx, rng)
                .into_iter()
                .map(|i| TimedOption {
                    text: options[i].clone(),
                    right: i == *answer,
                })
                .collect()
        }
        Question::Tf { answer, .. } => vec![
            TimedOption {
                text: "Верно".to_string(),
                right: *answer,
            },
            TimedOption {
                text: "Неверно".to_string(),
                right: !*answer,
            },
        ],
        _ => Vec::new(),
    };
    TimedItem {
        q,
        sec,
        opts,
        seconds: q_seconds(q, state),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimedStatus {
    Right,
    Wrong,
    Skip,
    Timeout,
}

impl TimedStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            TimedStatus::Right => "right",
            TimedStatus::Wrong => "wrong",
            TimedStatus::Skip => "skip",
            TimedStatus::Timeout => "timeout",
        }
    }

    /// Verdict text shown on reveal (js/ui/timed.js reveal msg).
    pub fn message(self) -> &'static str {
        match self {
            TimedStatus::Right => "Верно!",
            TimedStatus::Wrong => "Неверно",
            TimedStatus::Skip => "Не знаю",
            TimedStatus::Timeout => "Время вышло",
        }
    }
}

#[derive(Debug, Clone)]
pub struct TimedRes {
    pub status: TimedStatus,
    /// Chosen option index, or -1 for «не знаю»/timeout.
    pub k: i64,
    pub user: Option<String>,
}

pub struct TimedRun<'a> {
    pub items: Vec<TimedItem<'a>>,
    pub idx: usize,
    pub res: Vec<Option<TimedRes>>,
    pub seconds: i64,
    pub retry: bool,
    pub started_at: i64,
}

impl<'a> TimedRun<'a> {
    /// k: chosen option index, or -1 for «не знаю»/timeout
    /// (js/ui/timed.js resolve).
    pub fn resolve(
        &mut self,
        state: &mut State,
        k: i64,
        why: Option<TimedStatus>,
        today: NaiveDate,
    ) -> TimedRes {
        let it = &self.items[self.idx];
        let correct = k >= 0 && it.opts.get(k as usize).map(|o| o.right).unwrap_or(false);
        let status = if k < 0 {
            why.unwrap_or(TimedStatus::Skip)
        } else if correct {
            TimedStatus::Right
        } else {
            TimedStatus::Wrong
        };
        let res = TimedRes {
            status,
            k,
            user: if k >= 0 {
                it.opts.get(k as usize).map(|o| o.text.clone())
            } else {
                None
            },
        };
        if self.res.len() <= self.idx {
            self.res.resize(self.idx + 1, None);
        }
        self.res[self.idx] = Some(res.clone());
        record(
            state,
            it.q.id(),
            &CheckResult {
                score: if correct { 1.0 } else { 0.0 },
                correct,
                user: res.user.clone().unwrap_or_default(),
            },
            today,
        );
        res
    }
}

/// Build a timed run: either a retry of specific ids or a fresh weighted,
/// tiered selection (js/ui/timed.js T.build).
pub fn build_timed<'a>(
    lib: &'a Library,
    state: &State,
    ids: Option<&[String]>,
    count: Option<usize>,
    rng: &mut impl Rng,
) -> TimedRun<'a> {
    let picked: Vec<&Question> = match ids {
        Some(ids) => {
            let pool: Vec<&Question> = ids
                .iter()
                .filter_map(|id| lib.question(id).map(|(q, _)| q))
                .collect();
            util::shuffle(&pool, rng)
        }
        None => {
            let pool = timed_pool(lib, state);
            pick_timed(&pool, count.unwrap_or(TIMED_DEFAULT_COUNT), state, rng)
        }
    };
    TimedRun {
        items: picked
            .into_iter()
            .map(|q| timed_prepare(lib, q, state, rng))
            .collect(),
        idx: 0,
        res: Vec::new(),
        seconds: base_seconds(state),
        retry: ids.is_some(),
        started_at: chrono::Local::now().timestamp_millis(),
    }
}

pub struct TimedResult {
    pub total: usize,
    pub right: usize,
    pub wrong: usize,
    pub skip: usize,
    pub pct: i64,
    /// Indices of items not answered right.
    pub misses: Vec<usize>,
    pub seconds: i64,
    pub at: i64,
    pub xp: i64,
    pub award: AwardOutcome,
}

/// Results headline and sub-line (js/ui/timed.js timedResults).
pub fn timed_result_title(pct: i64) -> (&'static str, &'static str) {
    if pct >= 90 {
        ("Блестяще!", "Вы уверенно держитесь под давлением таймера.")
    } else if pct >= 75 {
        (
            "Отличная игра",
            "Ещё немного — и ни один вопрос не застанет врасплох.",
        )
    } else if pct >= 50 {
        (
            "Достойный результат",
            "Половина пути пройдена: разберите ошибки ниже.",
        )
    } else {
        (
            "Есть над чем поработать",
            "Разберите ошибки и повторите уроки — следующая игра пройдёт лучше.",
        )
    }
}

/// Finish a timed run (js/ui/timed.js finish).
pub fn finish_timed(
    lib: &Library,
    run: &TimedRun,
    state: &mut State,
    today: NaiveDate,
) -> TimedResult {
    let mut right = 0usize;
    let mut wrong = 0usize;
    let mut skip = 0usize;
    let mut misses = Vec::new();
    for (i, _) in run.items.iter().enumerate() {
        let r = run.res.get(i).and_then(|r| r.clone()).unwrap_or(TimedRes {
            status: TimedStatus::Skip,
            k: -1,
            user: None,
        });
        match r.status {
            TimedStatus::Right => right += 1,
            TimedStatus::Wrong => wrong += 1,
            _ => skip += 1,
        }
        if r.status != TimedStatus::Right {
            misses.push(i);
        }
    }
    let total = run.items.len();
    let pct = if total > 0 {
        (right as f64 / total as f64 * 100.0).round() as i64
    } else {
        0
    };
    let at = chrono::Local::now().timestamp_millis();
    log_exam(
        state,
        serde_json::json!({
            "at": at,
            "mode": "timed",
            "sectionId": serde_json::Value::Null,
            "total": total,
            "score": right,
            "pct": pct,
        }),
    );
    log_timed(
        state,
        run.items.iter().map(|it| it.q.id().to_string()).collect(),
    );
    if total >= 20 && right == total {
        game::count(state, "perfectTimed", 1);
    }
    let xp = game::XP.exam_right * right as i64;
    let award = game::award(lib, state, xp, today);
    TimedResult {
        total,
        right,
        wrong,
        skip,
        pct,
        misses,
        seconds: run.seconds,
        at,
        xp,
        award,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use std::collections::HashSet;

    fn lib() -> Library {
        Library::embedded()
    }

    fn day() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 22).unwrap()
    }

    fn rng() -> impl Rng {
        rand::rngs::StdRng::seed_from_u64(7)
    }

    fn unlocked_state() -> State {
        let mut s = State::default();
        s.settings.unlock_all = true;
        s
    }

    #[test]
    fn tier_counts_for_20() {
        let c = tier_counts(20);
        assert_eq!((c.easy, c.medium, c.hard), (5, 8, 7));
    }

    #[test]
    fn timed_20_has_5_easy_8_medium_7_hard_in_order() {
        let lib = lib();
        let state = unlocked_state();
        let pool = timed_pool(&lib, &state);
        assert!(pool.len() >= 20, "pool too small: {}", pool.len());
        let run = build_timed(&lib, &state, None, None, &mut rng());
        assert_eq!(run.items.len(), 20);
        let diffs: Vec<Option<Difficulty>> = run
            .items
            .iter()
            .map(|it| it.q.common().difficulty)
            .collect();
        for (i, d) in diffs.iter().enumerate() {
            let want = if i < 5 {
                Some(Difficulty::Easy)
            } else if i < 13 {
                Some(Difficulty::Medium)
            } else {
                Some(Difficulty::Hard)
            };
            assert_eq!(*d, want, "position {i}");
        }
    }

    #[test]
    fn timed_easy_questions_get_plus_5_seconds() {
        let lib = lib();
        let mut state = unlocked_state();
        state.settings.timed_seconds = 20;
        let run = build_timed(&lib, &state, None, None, &mut rng());
        assert_eq!(run.seconds, 20);
        for (i, it) in run.items.iter().enumerate() {
            let want = if i < 5 { 25 } else { 20 };
            assert_eq!(it.seconds, want, "position {i}");
        }
        state.settings.timed_seconds = 30;
        assert_eq!(base_seconds(&state), 30);
    }

    #[test]
    fn timed_avoids_last_3_runs_when_pool_allows() {
        let lib = lib();
        let mut state = unlocked_state();
        let pool = timed_pool(&lib, &state);
        let tiers = split_tiers(&pool);
        // Verify the invariant tier by tier: whenever a tier's fresh pool is
        // big enough, no picked id may come from the last 3 runs.
        for _ in 0..3 {
            let run = build_timed(&lib, &state, None, None, &mut rng());
            log_timed(
                &mut state,
                run.items.iter().map(|it| it.q.id().to_string()).collect(),
            );
        }
        let recent: HashSet<&str> = recent_timed_ids(&state).into_iter().collect();
        let counts = plan_tiers(20, &tiers);
        let run = build_timed(&lib, &state, None, None, &mut rng());
        let mut i = 0;
        for (want, bank) in [
            (counts.easy, &tiers.easy),
            (counts.medium, &tiers.medium),
            (counts.hard, &tiers.hard),
        ] {
            let fresh = bank.iter().filter(|q| !recent.contains(q.id())).count();
            for it in &run.items[i..i + want] {
                if fresh >= want {
                    assert!(!recent.contains(it.q.id()), "repeated {}", it.q.id());
                }
            }
            i += want;
        }
    }

    #[test]
    fn two_runs_differ() {
        let lib = lib();
        let state = unlocked_state();
        let mut rng = rng();
        let a = build_timed(&lib, &state, None, None, &mut rng);
        let b = build_timed(&lib, &state, None, None, &mut rng);
        let ids_a: Vec<&str> = a.items.iter().map(|it| it.q.id()).collect();
        let ids_b: Vec<&str> = b.items.iter().map(|it| it.q.id()).collect();
        assert_ne!(ids_a, ids_b);
    }

    #[test]
    fn level_filtering_respected() {
        let lib = Library::embedded();
        let mut state = unlocked_state();
        state.settings.level = Some(crate::storage::Level::Reform);
        let pool = timed_pool(&lib, &state);
        assert!(!pool.is_empty());
        for q in &pool {
            let min = q
                .common()
                .min_level
                .unwrap_or(crate::storage::Level::Reform);
            assert!(min.rank() <= 1, "{} at {:?}", q.id(), min);
        }
        // Only unlocked sections contribute.
        let mut locked = State::default(); // nothing completed → only osnovy unlocked
        locked.settings.level = Some(crate::storage::Level::Reform);
        let pool2 = timed_pool(&lib, &locked);
        for q in &pool2 {
            let (_, sec) = lib.question(q.id()).unwrap();
            assert_eq!(sec.id, "osnovy");
        }
    }

    #[test]
    fn typed_normalisation() {
        let q = Question::Typed {
            common: Default::default(),
            answer: "Кидуш".to_string(),
            accept: vec!["киддуш".to_string()],
        };
        let item = Item {
            q: &q,
            sec: None,
            order: None,
            rights: None,
        };
        for good in ["кидуш", "КИДУШ", "  кидуш  ", "кідуш?", "киддуш"] {
            let r = check(&item, &Response::Typed(good.to_string()));
            // «кідуш» has a Cyrillic і — still accepted because punctuation
            // (the «?») is stripped and letters compared case-insensitively.
            if good == "кідуш?" {
                assert!(!r.correct, "{good} must not pass (Ukrainian і)");
            } else {
                assert!(r.correct, "{good} must pass");
                assert_eq!(r.score, 1.0);
            }
        }
        let r = check(&item, &Response::Typed("кіддуш?".to_string()));
        assert!(!r.correct);
        let r = check(&item, &Response::Typed("".to_string()));
        assert!(!r.correct);
        assert_eq!(r.user, "—");
        // ё/е equivalence
        let q2 = Question::Typed {
            common: Default::default(),
            answer: "ёрш".to_string(),
            accept: vec![],
        };
        let item2 = Item {
            q: &q2,
            sec: None,
            order: None,
            rights: None,
        };
        assert!(check(&item2, &Response::Typed("ерш".to_string())).correct);
    }

    #[test]
    fn mc_option_shuffle_keeps_correct_answer() {
        let lib = lib();
        let state = unlocked_state();
        let pool = timed_pool(&lib, &state);
        let mc = pool
            .iter()
            .find(|q| matches!(q, Question::Mc { options, .. } if options.len() == 4))
            .unwrap();
        let run_item = timed_prepare(&lib, mc, &state, &mut rng());
        assert_eq!(run_item.opts.len(), 4);
        let right: Vec<&TimedOption> = run_item.opts.iter().filter(|o| o.right).collect();
        assert_eq!(right.len(), 1);
        if let Question::Mc {
            options, answer, ..
        } = mc
        {
            assert_eq!(right[0].text, options[*answer]);
            // All options present exactly once.
            let mut texts: Vec<&str> = run_item.opts.iter().map(|o| o.text.as_str()).collect();
            texts.sort();
            let mut orig: Vec<&str> = options.iter().map(|s| s.as_str()).collect();
            orig.sort();
            assert_eq!(texts, orig);
        }
    }

    #[test]
    fn check_all_formats() {
        // tf
        let q = Question::Tf {
            common: Default::default(),
            answer: true,
        };
        let item = Item {
            q: &q,
            sec: None,
            order: None,
            rights: None,
        };
        let r = check(&item, &Response::Tf(true));
        assert!(r.correct);
        assert_eq!(r.user, "Верно");
        assert!(!check(&item, &Response::Tf(false)).correct);
        assert_eq!(correct_text(&q), "Верно");
        // match: partial credit
        let q = Question::Match {
            common: Default::default(),
            pairs: vec![
                ("А".to_string(), "а".to_string()),
                ("Б".to_string(), "б".to_string()),
                ("В".to_string(), "в".to_string()),
            ],
        };
        let item = Item {
            q: &q,
            sec: None,
            order: None,
            rights: None,
        };
        let r = check(
            &item,
            &Response::Match(vec!["а".to_string(), "б".to_string(), "г".to_string()]),
        );
        assert!(!r.correct);
        assert!((r.score - 2.0 / 3.0).abs() < 1e-9);
        assert_eq!(r.user, "А — а; Б — б; В — г");
        let r = check(
            &item,
            &Response::Match(vec!["а".to_string(), "б".to_string(), "в".to_string()]),
        );
        assert!(r.correct);
        assert_eq!(correct_text(&q), "А — а; Б — б; В — в");
        // open: self-check
        let q = Question::Open {
            common: Default::default(),
            model_answer: "Модельный ответ".to_string(),
            key_points: vec!["точка".to_string()],
        };
        let item = Item {
            q: &q,
            sec: None,
            order: None,
            rights: None,
        };
        let r = check(&item, &Response::Open(0.5));
        assert!(!r.correct);
        assert_eq!(r.score, 0.5);
        assert_eq!(r.user, "Частично");
        assert_eq!(check(&item, &Response::Open(1.0)).user, "Ответил верно");
        assert_eq!(check(&item, &Response::Open(0.0)).user, "Не ответил");
        assert_eq!(correct_text(&q), "Модельный ответ");
    }

    #[test]
    fn match_empty_pairs_scores_like_js() {
        // js/exam.js: good === q.pairs.length → 0 === 0 → score 1.
        let q = Question::Match {
            common: Default::default(),
            pairs: vec![],
        };
        let item = Item {
            q: &q,
            sec: None,
            order: None,
            rights: None,
        };
        let r = check(&item, &Response::Match(vec![]));
        assert_eq!(r.score, 1.0);
        assert!(r.correct);
    }

    #[test]
    fn record_and_weight() {
        let mut state = State::default();
        let t = day();
        let wrong = CheckResult {
            score: 0.0,
            correct: false,
            user: "x".into(),
        };
        record(&mut state, "q1", &wrong, t);
        record(&mut state, "q1", &wrong, t);
        let st = &state.questions["q1"];
        assert_eq!(st.n, 2);
        assert_eq!(st.miss, 2.0);
        assert_eq!(st.streak, 0);
        assert_eq!(st.last, "2026-11-22");
        assert_eq!(weight(&state, "q1"), 5.0);
        // Partial: +0.5 miss.
        let partial = CheckResult {
            score: 2.0 / 3.0,
            correct: false,
            user: "x".into(),
        };
        record(&mut state, "q1", &partial, t);
        assert_eq!(state.questions["q1"].miss, 2.5);
        // Correct streak lowers the weight, floor 0.25.
        let right = CheckResult {
            score: 1.0,
            correct: true,
            user: "x".into(),
        };
        for _ in 0..3 {
            record(&mut state, "q2", &right, t);
        }
        assert_eq!(state.questions["q2"].streak, 3);
        assert_eq!(weight(&state, "q2"), 0.25); // 1 - 1.5 → clamped
        assert_eq!(weight(&state, "unknown"), 1.0);
    }

    #[test]
    fn build_modes() {
        let lib = lib();
        let mut state = unlocked_state();
        // Section mode.
        let s = build(
            &lib,
            &state,
            BuildOpts {
                mode: Mode::Section,
                section_id: Some("osnovy".to_string()),
                count: Some(10),
                only_unlocked: None,
                ids: None,
            },
            &mut rng(),
        );
        assert_eq!(s.items.len(), 10);
        for it in &s.items {
            let (_, sec) = lib.question(it.q.id()).unwrap();
            assert_eq!(sec.id, "osnovy");
            assert!(it.sec.is_some());
        }
        // General mode over unlocked sections.
        let g = build(
            &lib,
            &state,
            BuildOpts {
                mode: Mode::General,
                section_id: None,
                count: Some(20),
                only_unlocked: None,
                ids: None,
            },
            &mut rng(),
        );
        assert_eq!(g.items.len(), 20);
        // Beitdin mode: mostly open from beit-din + open/typed elsewhere.
        let b = build(
            &lib,
            &state,
            BuildOpts {
                mode: Mode::Beitdin,
                section_id: None,
                count: Some(10),
                only_unlocked: None,
                ids: None,
            },
            &mut rng(),
        );
        assert_eq!(b.items.len(), 10);
        for it in &b.items {
            assert!(matches!(
                it.q,
                Question::Open { .. } | Question::Typed { .. }
            ));
        }
        // Retry mode by ids, shuffled but complete.
        state.settings.level = None;
        let pool = timed_pool(&lib, &state);
        let ids: Vec<String> = pool.iter().take(5).map(|q| q.id().to_string()).collect();
        let r = build(
            &lib,
            &state,
            BuildOpts {
                mode: Mode::Retry,
                section_id: None,
                count: None,
                only_unlocked: None,
                ids: Some(ids.clone()),
            },
            &mut rng(),
        );
        assert_eq!(r.items.len(), 5);
        let mut got: Vec<&str> = r.items.iter().map(|it| it.q.id()).collect();
        got.sort();
        let mut want: Vec<&str> = ids.iter().map(|s| s.as_str()).collect();
        want.sort();
        assert_eq!(got, want);
    }

    #[test]
    fn finish_section_exam_updates_best_and_xp() {
        let lib = lib();
        let mut state = unlocked_state();
        let t = day();
        let mut s = build(
            &lib,
            &state,
            BuildOpts {
                mode: Mode::Section,
                section_id: Some("osnovy".to_string()),
                count: Some(4),
                only_unlocked: None,
                ids: None,
            },
            &mut rng(),
        );
        // Answer everything correctly.
        for i in 0..s.items.len() {
            s.idx = i;
            let resp = match s.items[i].q {
                Question::Mc { answer, .. } => Response::Mc(*answer),
                Question::Tf { answer, .. } => Response::Tf(*answer),
                Question::Typed { answer, .. } => Response::Typed(answer.clone()),
                Question::Match { pairs, .. } => {
                    Response::Match(pairs.iter().map(|p| p.1.clone()).collect())
                }
                Question::Open { .. } => Response::Open(1.0),
            };
            s.answer(&mut state, resp, t);
        }
        let res = finish(&lib, &s, &mut state, t);
        assert_eq!(res.total, 4);
        assert_eq!(res.pct, 100);
        assert!(res.passed);
        assert!(res.mistakes.is_empty());
        assert_eq!(res.xp, game::XP.test + game::XP.perfect); // 20 + 10
        assert_eq!(state.best["osnovy"], 100.0);
        assert_eq!(state.exams.len(), 1);
        assert_eq!(state.exams[0]["mode"], "section");
        assert_eq!(state.xp_total, 30);
        // A worse run does not lower best.
        let mut s2 = build(
            &lib,
            &state,
            BuildOpts {
                mode: Mode::Section,
                section_id: Some("osnovy".to_string()),
                count: Some(4),
                only_unlocked: None,
                ids: None,
            },
            &mut rng(),
        );
        s2.idx = 0;
        let resp = match s2.items[0].q {
            Question::Mc { answer, .. } => Response::Mc((answer + 1) % 4),
            Question::Tf { answer, .. } => Response::Tf(!answer),
            Question::Typed { .. } => Response::Typed("не знаю".to_string()),
            Question::Match { .. } => Response::Match(vec!["?".to_string(); 9]),
            Question::Open { .. } => Response::Open(0.0),
        };
        s2.answer(&mut state, resp, t);
        let res2 = finish(&lib, &s2, &mut state, t);
        assert!(res2.pct < 100);
        assert_eq!(state.best["osnovy"], 100.0);
    }

    #[test]
    fn finish_timed_counts_and_logs() {
        let lib = lib();
        let mut state = unlocked_state();
        let t = day();
        let mut run = build_timed(&lib, &state, None, None, &mut rng());
        assert_eq!(run.items.len(), 20);
        // Right, wrong, skip across the run.
        for i in 0..run.items.len() {
            run.idx = i;
            match i % 3 {
                0 => {
                    let k = run.items[i].opts.iter().position(|o| o.right).unwrap() as i64;
                    let r = run.resolve(&mut state, k, None, t);
                    assert_eq!(r.status, TimedStatus::Right);
                }
                1 => {
                    let k = run.items[i].opts.iter().position(|o| !o.right).unwrap() as i64;
                    let r = run.resolve(&mut state, k, None, t);
                    assert_eq!(r.status, TimedStatus::Wrong);
                }
                _ => {
                    let r = run.resolve(&mut state, -1, Some(TimedStatus::Skip), t);
                    assert_eq!(r.status, TimedStatus::Skip);
                }
            }
        }
        let res = finish_timed(&lib, &run, &mut state, t);
        assert_eq!(res.total, 20);
        assert_eq!(res.right + res.wrong + res.skip, 20);
        assert_eq!(res.misses.len(), res.wrong + res.skip);
        assert_eq!(res.xp, game::XP.exam_right * res.right as i64);
        assert_eq!(state.exam_history.len(), 1);
        assert_eq!(state.exam_history[0].len(), 20);
        assert_eq!(state.exams[0]["mode"], "timed");
        // perfectTimed only for 20/20.
        assert!(!state.counters.contains_key("perfectTimed"));
        // Status messages are the JS verdict texts.
        assert_eq!(TimedStatus::Right.message(), "Верно!");
        assert_eq!(TimedStatus::Timeout.message(), "Время вышло");
        assert_eq!(timed_result_title(95).0, "Блестяще!");
        assert_eq!(timed_result_title(80).0, "Отличная игра");
        assert_eq!(timed_result_title(55).0, "Достойный результат");
        assert_eq!(timed_result_title(10).0, "Есть над чем поработать");
    }

    #[test]
    fn perfect_timed_counts() {
        let lib = lib();
        let mut state = unlocked_state();
        let t = day();
        let mut run = build_timed(&lib, &state, None, None, &mut rng());
        for i in 0..run.items.len() {
            run.idx = i;
            let k = run.items[i].opts.iter().position(|o| o.right).unwrap() as i64;
            run.resolve(&mut state, k, None, t);
        }
        let res = finish_timed(&lib, &run, &mut state, t);
        assert_eq!(res.right, 20);
        assert_eq!(state.counters["perfectTimed"], 1);
    }
}
