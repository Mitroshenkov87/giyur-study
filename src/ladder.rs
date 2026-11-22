//! «Маалот» — the ladder quiz (the «Экзамен» screen).
//!
//! In meaning it follows a TV quiz ladder: fifteen questions of rising
//! difficulty, two safe steps, three one-time lifelines, and the right to
//! stop and keep what you have. The steps are named after Пиркей Авот 5:21
//! («в пять лет — к Микре, в десять — к Мишне, в тринадцать — к мицвот,
//! в пятнадцать — к Гмаре»): step 5 «Микра» and step 10 «Мишна» are the
//! safe steps, step 13 «Мицвот», step 15 «Гмара» is the top. Score is the
//! number of steps kept, never money.
//!
//! Pure logic only (no UI): building a ladder, answering, lifelines,
//! walking away, and finishing (best score, history, XP).

use chrono::NaiveDate;
use rand::Rng;

use crate::content::{Difficulty, Library, Question, SectionMeta};
use crate::exam::{self, CheckResult};
use crate::game::{self, AwardOutcome};
use crate::storage::State;
use crate::util;

/// Number of steps on the ladder.
pub const STEPS: usize = 15;
/// Safe steps: falling from above keeps this many steps.
pub const SAFE: [usize; 2] = [5, 10];
/// Questions per difficulty tier (steps 1–5 easy, 6–10 medium, 11–15 hard).
pub const PER_TIER: usize = 5;
/// XP for every step kept, plus a bonus for the top.
pub const XP_PER_STEP: i64 = 3;
pub const XP_TOP_BONUS: i64 = 15;
/// `state.counters` keys.
pub const BEST_KEY: &str = "ladderBest";
pub const TOP_KEY: &str = "ladderTop";
pub const RUNS_KEY: &str = "ladderRuns";

/// One step of the ladder, for the ladder column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rung {
    /// 1..=15.
    pub n: usize,
    /// Hebrew numeral (gematria), «א» … «טו».
    pub he: &'static str,
    /// Named steps (Пиркей Авот 5:21), empty for the others.
    pub name: &'static str,
    pub safe: bool,
    pub tier: Difficulty,
}

const HE: [&str; STEPS] = [
    "א", "ב", "ג", "ד", "ה", "ו", "ז", "ח", "ט", "י", "יא", "יב", "יג", "יד", "טו",
];

pub fn rung(n: usize) -> Rung {
    let n = n.clamp(1, STEPS);
    Rung {
        n,
        he: HE[n - 1],
        name: match n {
            5 => "Микра",
            10 => "Мишна",
            13 => "Мицвот",
            15 => "Гмара",
            _ => "",
        },
        safe: SAFE.contains(&n),
        tier: tier_of(n),
    }
}

/// All rungs, bottom (1) to top (15).
pub fn rungs() -> Vec<Rung> {
    (1..=STEPS).map(rung).collect()
}

/// Difficulty of step n (1-based).
pub fn tier_of(n: usize) -> Difficulty {
    match n {
        0..=5 => Difficulty::Easy,
        6..=10 => Difficulty::Medium,
        _ => Difficulty::Hard,
    }
}

/// Title of a difficulty tier on the ladder.
pub fn tier_title(d: Difficulty) -> &'static str {
    match d {
        Difficulty::Easy => "Ступени 1–5 · лёгкие",
        Difficulty::Medium => "Ступени 6–10 · средние",
        Difficulty::Hard => "Ступени 11–15 · трудные",
    }
}

/// Timer choices on the setup screen: (seconds per question, label);
/// 0 = no timer. Stored in `settings.timed_seconds`.
pub const TIMER_CHOICES: [(i64, &str); 3] = [(0, "Без таймера"), (30, "30 с"), (20, "20 с")];

/// Normalise a stored timer value to one of [`TIMER_CHOICES`] (30 by default).
pub fn sanitize_seconds(v: i64) -> i64 {
    match v {
        0 => 0,
        20 => 20,
        _ => 30,
    }
}

/// Seconds per ladder question from settings (0 = no timer).
pub fn timer_seconds(state: &State) -> i64 {
    sanitize_seconds(state.settings.timed_seconds)
}

/// Plural of «ступень» after a number: «1 ступень», «3 ступени», «5 ступеней».
pub fn steps_word(n: usize) -> &'static str {
    util::plural(n as i64, &["ступень", "ступени", "ступеней"])
}

/// Steps kept after falling with `climbed` steps already answered.
pub fn safe_below(climbed: usize) -> usize {
    SAFE.iter()
        .copied()
        .filter(|&s| s <= climbed)
        .max()
        .unwrap_or(0)
}

// ---- Pool and building ----

fn difficulty(q: &Question) -> Difficulty {
    q.common().difficulty.unwrap_or(Difficulty::Medium)
}

/// Ladder pool: multiple-choice questions (4 options) and true/false from
/// unlocked sections at the user's level. Multiple choice is preferred;
/// true/false only fills a tier that lacks multiple choice.
pub fn pool<'a>(lib: &'a Library, state: &State) -> Vec<&'a Question> {
    let user = Library::user_level(state);
    let mut out = Vec::new();
    for sec in lib.unlocked_sections(state) {
        for q in lib.questions(sec, user) {
            match q {
                Question::Mc { options, .. } if options.len() >= 3 => out.push(q),
                Question::Tf { .. } => out.push(q),
                _ => {}
            }
        }
    }
    out
}

/// Per-difficulty counts of the ladder pool (mc, tf), for the setup screen.
pub fn pool_counts(lib: &Library, state: &State) -> [(Difficulty, usize, usize); 3] {
    let p = pool(lib, state);
    [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard].map(|d| {
        let mc = p
            .iter()
            .filter(|q| difficulty(q) == d && matches!(q, Question::Mc { .. }))
            .count();
        let tf = p
            .iter()
            .filter(|q| difficulty(q) == d && matches!(q, Question::Tf { .. }))
            .count();
        (d, mc, tf)
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct LOption {
    pub text: String,
    pub right: bool,
}

/// One step's question with its options in display order.
#[derive(Debug, Clone)]
pub struct Step {
    pub q: Question,
    pub section_title: Option<String>,
    pub opts: Vec<LOption>,
}

impl Step {
    pub fn right_index(&self) -> usize {
        self.opts.iter().position(|o| o.right).unwrap_or(0)
    }
    pub fn right_text(&self) -> &str {
        self.opts
            .iter()
            .find(|o| o.right)
            .map(|o| o.text.as_str())
            .unwrap_or("")
    }
}

fn prepare(lib: &Library, q: &Question, rng: &mut impl Rng) -> Step {
    let section_title = lib.question(q.id()).map(|(_, s)| s.title.clone());
    let opts = match q {
        Question::Mc {
            options, answer, ..
        } => {
            let idx: Vec<usize> = (0..options.len()).collect();
            util::shuffle(&idx, rng)
                .into_iter()
                .map(|i| LOption {
                    text: options[i].clone(),
                    right: i == *answer,
                })
                .collect()
        }
        Question::Tf { answer, .. } => vec![
            LOption {
                text: "Верно".to_string(),
                right: *answer,
            },
            LOption {
                text: "Неверно".to_string(),
                right: !*answer,
            },
        ],
        _ => Vec::new(),
    };
    Step {
        q: q.clone(),
        section_title,
        opts,
    }
}

/// Choose the 15 questions: 5 per tier in tier order, multiple choice first,
/// weighted toward questions answered wrong before, avoiding the last runs'
/// questions while the pool allows. A tier short of questions borrows from
/// its neighbour (easy ↔ medium ↔ hard) so the ladder still has 15 steps
/// when the pool has 15 questions at all.
pub fn pick<'q>(pool: &[&'q Question], state: &State, rng: &mut impl Rng) -> Vec<&'q Question> {
    let recent = exam::recent_timed_ids(state);
    let tiers = [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard];
    let mut picked: Vec<&Question> = Vec::new();
    let order_for = |d: Difficulty| -> Vec<Difficulty> {
        match d {
            Difficulty::Easy => vec![Difficulty::Easy, Difficulty::Medium, Difficulty::Hard],
            Difficulty::Medium => vec![Difficulty::Medium, Difficulty::Easy, Difficulty::Hard],
            Difficulty::Hard => vec![Difficulty::Hard, Difficulty::Medium, Difficulty::Easy],
        }
    };
    for d in tiers {
        let mut need = PER_TIER;
        let mut chunk: Vec<&Question> = Vec::new();
        // Preference ladder: own tier mc (fresh, then any), own tier tf,
        // then the neighbouring tiers the same way.
        'outer: for src in order_for(d) {
            for want_mc in [true, false] {
                for fresh_only in [true, false] {
                    if need == 0 {
                        break 'outer;
                    }
                    let bank: Vec<&Question> = pool
                        .iter()
                        .copied()
                        .filter(|q| difficulty(q) == src)
                        .filter(|q| matches!(q, Question::Mc { .. }) == want_mc)
                        .filter(|q| !fresh_only || !recent.contains(&q.id()))
                        .filter(|q| {
                            !picked.iter().any(|p| p.id() == q.id())
                                && !chunk.iter().any(|p| p.id() == q.id())
                        })
                        .collect();
                    let got =
                        util::weighted_sample(&bank, |q| exam::weight(state, q.id()), need, rng);
                    need -= got.len();
                    chunk.extend(got);
                }
            }
        }
        // Inside a tier: easier first when borrowing.
        chunk.sort_by_key(|q| match difficulty(q) {
            Difficulty::Easy => 0,
            Difficulty::Medium => 1,
            Difficulty::Hard => 2,
        });
        picked.extend(chunk);
    }
    picked
}

// ---- Lifelines ----

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lifeline {
    /// «50 на 50»: two wrong options disappear (multiple choice only).
    Fifty,
    /// «Подсказка из урока»: the lesson, its source and the closest key idea.
    Lesson,
    /// «Спросить раввина»: the direction of the explanation, the answer hidden.
    Rabbi,
}

impl Lifeline {
    pub const ALL: [Lifeline; 3] = [Lifeline::Fifty, Lifeline::Lesson, Lifeline::Rabbi];

    pub fn title(self) -> &'static str {
        match self {
            Lifeline::Fifty => "50 на 50",
            Lifeline::Lesson => "Подсказка из урока",
            Lifeline::Rabbi => "Спросить раввина",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Lifeline::Fifty => "fifty",
            Lifeline::Lesson => "lesson",
            Lifeline::Rabbi => "rabbi",
        }
    }

    fn idx(self) -> usize {
        match self {
            Lifeline::Fifty => 0,
            Lifeline::Lesson => 1,
            Lifeline::Rabbi => 2,
        }
    }
}

/// What a lifeline shows.
#[derive(Debug, Clone, PartialEq)]
pub enum Help {
    /// Option indices (display order) that are now hidden.
    Fifty(Vec<usize>),
    Lesson {
        section: Option<String>,
        lesson: Option<String>,
        source: Option<String>,
        idea: Option<String>,
    },
    Rabbi(String),
}

fn words(s: &str) -> std::collections::HashSet<String> {
    s.to_lowercase()
        .replace('ё', "е")
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() > 3)
        // Russian endings vary: compare stems of up to 6 letters.
        .map(|w| w.chars().take(6).collect())
        .collect()
}

/// The lesson key idea closest to the question prompt (by shared stems).
fn closest_idea(lesson_ideas: &[String], prompt: &str) -> Option<String> {
    let pw = words(prompt);
    lesson_ideas
        .iter()
        .map(|i| (words(i).intersection(&pw).count(), i))
        .filter(|(n, _)| *n > 0)
        .max_by_key(|(n, _)| *n)
        .map(|(_, i)| i.clone())
}

/// Hide the right answer inside a text (case-insensitive), so the rabbi
/// points the way without saying the option out loud.
pub fn mask_answer(text: &str, answer: &str) -> String {
    let a = answer.trim().trim_end_matches('.');
    if a.chars().count() < 3 {
        return text.to_string();
    }
    let lower = text.to_lowercase();
    let al = a.to_lowercase();
    let mut out = String::new();
    let mut i = 0;
    while let Some(pos) = lower[i..].find(&al) {
        let start = i + pos;
        // Byte offsets from the lower-cased copy are only safe when lowering
        // kept byte lengths; otherwise fall back to the text unmasked.
        if !text.is_char_boundary(start) || !text.is_char_boundary(start + al.len()) {
            return text.to_string();
        }
        out.push_str(&text[i..start]);
        out.push('…');
        i = start + al.len();
    }
    out.push_str(&text[i..]);
    out
}

/// Prefix of the rabbi's answer.
pub const RABBI_LEAD: &str = "Раввин отвечает не прямо, а направлением: ";

pub fn help(lib: &Library, step: &Step, line: Lifeline, rng: &mut impl Rng) -> Help {
    match line {
        Lifeline::Fifty => {
            let wrong: Vec<usize> = (0..step.opts.len())
                .filter(|&i| !step.opts[i].right)
                .collect();
            if step.opts.len() < 3 {
                return Help::Fifty(Vec::new());
            }
            // Leave the right one and one random wrong option.
            let keep = wrong[rng.gen_range(0..wrong.len())];
            Help::Fifty(wrong.into_iter().filter(|&i| i != keep).collect())
        }
        Lifeline::Lesson => {
            let c = step.q.common();
            let found = c.lesson_id.as_deref().and_then(|l| lib.lesson(l));
            Help::Lesson {
                section: found
                    .map(|(_, s): (_, &SectionMeta)| s.title.clone())
                    .or(step.section_title.clone()),
                lesson: found.and_then(|(l, _)| l.title.clone()),
                source: c.source.clone(),
                idea: found.and_then(|(l, _)| closest_idea(&l.key_ideas, &c.prompt)),
            }
        }
        Lifeline::Rabbi => {
            let c = step.q.common();
            let base = c
                .explanation
                .clone()
                .filter(|e| !e.trim().is_empty())
                .unwrap_or_else(|| "Вспомните урок, из которого этот вопрос.".to_string());
            let masked = match &step.q {
                Question::Mc { .. } => mask_answer(&base, step.right_text()),
                _ => base,
            };
            Help::Rabbi(masked)
        }
    }
}

// ---- The run ----

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// All 15 steps.
    Top,
    /// Stopped and kept the steps climbed.
    WalkedAway,
    /// Wrong answer (or time out): back to the last safe step.
    Fell,
}

impl Outcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Outcome::Top => "top",
            Outcome::WalkedAway => "walked",
            Outcome::Fell => "fell",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Ladder {
    pub steps: Vec<Step>,
    /// Steps answered right so far (= index of the current step).
    pub climbed: usize,
    used: [bool; 3],
    /// Lifeline results shown on the current step.
    pub shown: Vec<(Lifeline, Help)>,
    /// Display indices hidden by 50/50 on the current step.
    pub hidden: Vec<usize>,
    pub outcome: Option<Outcome>,
    /// Chosen option on the step that ended the run with a wrong answer
    /// (None = time out).
    pub fell_pick: Option<usize>,
    pub started_at: i64,
}

impl Ladder {
    pub fn new(steps: Vec<Step>) -> Self {
        Ladder {
            steps,
            climbed: 0,
            used: [false; 3],
            shown: Vec::new(),
            hidden: Vec::new(),
            outcome: None,
            fell_pick: None,
            started_at: chrono::Local::now().timestamp_millis(),
        }
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    pub fn current(&self) -> Option<&Step> {
        if self.outcome.is_some() {
            None
        } else {
            self.steps.get(self.climbed)
        }
    }

    /// 1-based number of the step being played.
    pub fn step_no(&self) -> usize {
        (self.climbed + 1).min(self.len().max(1))
    }

    pub fn used(&self, l: Lifeline) -> bool {
        self.used[l.idx()]
    }

    /// Whether a lifeline can be used now (unused, run going, and 50/50
    /// only where there are at least three options).
    pub fn can_use(&self, l: Lifeline) -> bool {
        match self.current() {
            None => false,
            Some(s) => !self.used(l) && (l != Lifeline::Fifty || s.opts.len() >= 3),
        }
    }

    pub fn use_lifeline(&mut self, lib: &Library, l: Lifeline, rng: &mut impl Rng) -> Option<Help> {
        if !self.can_use(l) {
            return None;
        }
        let step = self.current()?.clone();
        let h = help(lib, &step, l, rng);
        if let Help::Fifty(ref hide) = h {
            self.hidden = hide.clone();
        }
        self.used[l.idx()] = true;
        self.shown.push((l, h.clone()));
        Some(h)
    }

    /// Steps the player keeps if the run ended now by walking away.
    pub fn kept(&self) -> usize {
        match self.outcome {
            Some(Outcome::Fell) => safe_below(self.climbed),
            _ => self.climbed,
        }
    }

    /// Answer the current step with display option `k` (None = time out).
    /// Records the question stats. Returns whether it was right.
    pub fn answer(&mut self, state: &mut State, k: Option<usize>, today: NaiveDate) -> bool {
        let Some(step) = self.current() else {
            return false;
        };
        let correct = k
            .and_then(|k| step.opts.get(k))
            .map(|o| o.right)
            .unwrap_or(false);
        let user = k
            .and_then(|k| step.opts.get(k))
            .map(|o| o.text.clone())
            .unwrap_or_default();
        exam::record(
            state,
            step.q.id(),
            &CheckResult {
                score: if correct { 1.0 } else { 0.0 },
                correct,
                user,
            },
            today,
        );
        if correct {
            self.climbed += 1;
            self.shown.clear();
            self.hidden.clear();
            if self.climbed >= self.len() {
                self.outcome = Some(Outcome::Top);
            }
        } else {
            self.fell_pick = k;
            self.outcome = Some(Outcome::Fell);
        }
        correct
    }

    /// Stop and keep the steps climbed.
    pub fn walk_away(&mut self) {
        if self.outcome.is_none() {
            self.outcome = Some(Outcome::WalkedAway);
        }
    }
}

/// Build a fresh ladder (None when the pool is empty).
pub fn build(lib: &Library, state: &State, rng: &mut impl Rng) -> Option<Ladder> {
    let p = pool(lib, state);
    let picked = pick(&p, state, rng);
    if picked.is_empty() {
        return None;
    }
    Some(Ladder::new(
        picked.into_iter().map(|q| prepare(lib, q, rng)).collect(),
    ))
}

/// Best steps kept in any run.
pub fn best(state: &State) -> usize {
    state.counters.get(BEST_KEY).copied().unwrap_or(0).max(0) as usize
}

#[derive(Debug, Clone)]
pub struct LadderResult {
    pub outcome: Outcome,
    pub kept: usize,
    pub climbed: usize,
    pub total: usize,
    pub best_before: usize,
    pub new_best: bool,
    pub lifelines_used: usize,
    pub xp: i64,
    pub award: AwardOutcome,
}

/// Headline and line under it for the result screen.
pub fn result_title(r: &LadderResult) -> (&'static str, &'static str) {
    match r.outcome {
        Outcome::Top => (
            "Вся лестница пройдена",
            "Пятнадцать ступеней: от Микры до Гмары.",
        ),
        Outcome::WalkedAway if r.kept >= 10 => {
            ("Мудрое решение", "Знать, где остановиться, — тоже знание.")
        }
        Outcome::WalkedAway => (
            "Вы остановились",
            "Ступени, которые вы прошли, остаются за вами.",
        ),
        Outcome::Fell if r.kept > 0 => (
            "Вы удержались на надёжной ступени",
            "Разберите вопрос ниже — в следующий раз он не застанет врасплох.",
        ),
        Outcome::Fell => (
            "Начало пути",
            "Разберите вопрос ниже и попробуйте снова: каждая попытка учит.",
        ),
    }
}

/// Finish a run: history, best score, counters and XP. Call once.
pub fn finish(lib: &Library, ladder: &Ladder, state: &mut State, today: NaiveDate) -> LadderResult {
    let outcome = ladder.outcome.unwrap_or(Outcome::WalkedAway);
    let kept = match outcome {
        Outcome::Fell => safe_below(ladder.climbed),
        _ => ladder.climbed,
    };
    let total = ladder.len();
    let best_before = best(state);
    let new_best = kept > best_before;
    if new_best {
        state.counters.insert(BEST_KEY.to_string(), kept as i64);
    }
    game::count(state, RUNS_KEY, 1);
    if outcome == Outcome::Top && total >= STEPS {
        game::count(state, TOP_KEY, 1);
    }
    let at = chrono::Local::now().timestamp_millis();
    let pct = if total > 0 {
        (kept as f64 / total as f64 * 100.0).round() as i64
    } else {
        0
    };
    exam::log_exam(
        state,
        serde_json::json!({
            "at": at,
            "mode": "ladder",
            "sectionId": serde_json::Value::Null,
            "total": total,
            "score": kept,
            "pct": pct,
            "outcome": outcome.as_str(),
        }),
    );
    // Asked questions (up to the last one played) are kept out of the next runs.
    let asked = (ladder.climbed + usize::from(outcome == Outcome::Fell)).min(total);
    exam::log_timed(
        state,
        ladder.steps[..asked]
            .iter()
            .map(|s| s.q.id().to_string())
            .collect(),
    );
    let xp = XP_PER_STEP * kept as i64
        + if outcome == Outcome::Top {
            XP_TOP_BONUS
        } else {
            0
        };
    let award = game::award(lib, state, xp, today);
    LadderResult {
        outcome,
        kept,
        climbed: ladder.climbed,
        total,
        best_before,
        new_best,
        lifelines_used: ladder.used.iter().filter(|u| **u).count(),
        xp,
        award,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    fn rng() -> rand::rngs::StdRng {
        rand::rngs::StdRng::seed_from_u64(11)
    }

    fn day() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 22).unwrap()
    }

    fn open_state() -> State {
        let mut s = State::default();
        s.settings.unlock_all = true;
        s
    }

    fn right(l: &Ladder) -> usize {
        l.current().unwrap().right_index()
    }

    fn wrong(l: &Ladder) -> usize {
        let s = l.current().unwrap();
        (0..s.opts.len()).find(|&i| !s.opts[i].right).unwrap()
    }

    #[test]
    fn rungs_are_named_after_avot_5_21() {
        let r = rungs();
        assert_eq!(r.len(), 15);
        assert_eq!(r[4].name, "Микра");
        assert!(r[4].safe && r[9].safe);
        assert_eq!(r[9].name, "Мишна");
        assert_eq!(r[12].name, "Мицвот");
        assert_eq!(r[14].name, "Гмара");
        assert_eq!(r[14].he, "טו");
        assert_eq!(r.iter().filter(|x| x.safe).count(), 2);
    }

    #[test]
    fn safe_steps() {
        assert_eq!(safe_below(0), 0);
        assert_eq!(safe_below(4), 0);
        assert_eq!(safe_below(5), 5);
        assert_eq!(safe_below(9), 5);
        assert_eq!(safe_below(14), 10);
    }

    #[test]
    fn ladder_has_15_steps_rising_in_difficulty_mc_first() {
        let lib = Library::embedded();
        let state = open_state();
        let l = build(&lib, &state, &mut rng()).unwrap();
        assert_eq!(l.len(), STEPS);
        for (i, s) in l.steps.iter().enumerate() {
            assert_eq!(difficulty(&s.q), tier_of(i + 1), "step {}", i + 1);
            assert!(
                matches!(s.q, Question::Mc { .. }),
                "step {} should be mc",
                i + 1
            );
            assert_eq!(s.opts.iter().filter(|o| o.right).count(), 1);
        }
        let mut ids: Vec<&str> = l.steps.iter().map(|s| s.q.id()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), STEPS, "no repeats in one ladder");
    }

    #[test]
    fn every_tier_has_enough_mc_for_several_ladders() {
        // With everything open the pool must fill many different ladders.
        let lib = Library::embedded();
        for (d, mc, _tf) in pool_counts(&lib, &open_state()) {
            assert!(
                mc >= PER_TIER * 6,
                "{d:?}: only {mc} multiple-choice questions"
            );
        }
    }

    #[test]
    fn small_pool_borrows_from_neighbour_tiers() {
        let lib = Library::embedded();
        // The first section alone at the reform level.
        let state = State::default();
        let p = pool(&lib, &state);
        let picked = pick(&p, &state, &mut rng());
        assert_eq!(picked.len(), STEPS.min(p.len()));
    }

    #[test]
    fn climbing_to_the_top_and_finishing() {
        let lib = Library::embedded();
        let mut state = open_state();
        let mut l = build(&lib, &state, &mut rng()).unwrap();
        for _ in 0..STEPS {
            let k = right(&l);
            assert!(l.answer(&mut state, Some(k), day()));
        }
        assert_eq!(l.outcome, Some(Outcome::Top));
        let r = finish(&lib, &l, &mut state, day());
        assert_eq!(r.kept, 15);
        assert!(r.new_best);
        assert_eq!(r.xp, XP_PER_STEP * 15 + XP_TOP_BONUS);
        assert_eq!(best(&state), 15);
        assert_eq!(state.counters.get(TOP_KEY), Some(&1));
        assert_eq!(state.exam_history.last().unwrap().len(), 15);
        // The «15 ступеней» badge (id «millionaire») is earned.
        assert!(game::badge_list(&lib)
            .iter()
            .any(|b| b.id == "millionaire" && game::badge_ok(b, &lib, &state, day())));
    }

    #[test]
    fn falling_keeps_the_last_safe_step() {
        let lib = Library::embedded();
        let mut state = open_state();
        let mut l = build(&lib, &state, &mut rng()).unwrap();
        for _ in 0..7 {
            let k = right(&l);
            l.answer(&mut state, Some(k), day());
        }
        let w = wrong(&l);
        let qid = l.current().unwrap().q.id().to_string();
        assert!(!l.answer(&mut state, Some(w), day()));
        assert_eq!(l.outcome, Some(Outcome::Fell));
        assert_eq!(l.kept(), 5);
        assert!(state.questions.get(&qid).unwrap().miss > 0.0);
        let r = finish(&lib, &l, &mut state, day());
        assert_eq!((r.kept, r.climbed), (5, 7));
        assert_eq!(r.xp, XP_PER_STEP * 5);
        // 7 climbed + the one that failed are kept out of the next runs.
        assert_eq!(state.exam_history.last().unwrap().len(), 8);
    }

    #[test]
    fn time_out_counts_as_a_fall_below_the_first_safe_step() {
        let lib = Library::embedded();
        let mut state = open_state();
        let mut l = build(&lib, &state, &mut rng()).unwrap();
        for _ in 0..3 {
            let k = right(&l);
            l.answer(&mut state, Some(k), day());
        }
        l.answer(&mut state, None, day());
        assert_eq!(l.kept(), 0);
        assert_eq!(l.fell_pick, None);
    }

    #[test]
    fn walking_away_keeps_everything_and_best_is_kept() {
        let lib = Library::embedded();
        let mut state = open_state();
        state.counters.insert(BEST_KEY.into(), 12);
        let mut l = build(&lib, &state, &mut rng()).unwrap();
        for _ in 0..8 {
            let k = right(&l);
            l.answer(&mut state, Some(k), day());
        }
        l.walk_away();
        assert_eq!(l.kept(), 8);
        let r = finish(&lib, &l, &mut state, day());
        assert!(!r.new_best);
        assert_eq!(r.best_before, 12);
        assert_eq!(best(&state), 12);
        assert!(l.current().is_none());
    }

    #[test]
    fn lifelines_once_each() {
        let lib = Library::embedded();
        let mut state = open_state();
        let mut l = build(&lib, &state, &mut rng()).unwrap();
        let mut r = rng();
        match l.use_lifeline(&lib, Lifeline::Fifty, &mut r) {
            Some(Help::Fifty(hide)) => {
                assert_eq!(hide.len(), 2);
                let s = l.current().unwrap();
                assert!(hide.iter().all(|&i| !s.opts[i].right));
            }
            other => panic!("{other:?}"),
        }
        assert!(l.use_lifeline(&lib, Lifeline::Fifty, &mut r).is_none());
        assert!(matches!(
            l.use_lifeline(&lib, Lifeline::Lesson, &mut r),
            Some(Help::Lesson {
                lesson: Some(_),
                ..
            })
        ));
        let rabbi = l.use_lifeline(&lib, Lifeline::Rabbi, &mut r);
        let Some(Help::Rabbi(text)) = rabbi else {
            panic!("{rabbi:?}")
        };
        let ans = l.current().unwrap().right_text().to_lowercase();
        if ans.chars().count() >= 3 {
            assert!(
                !text.to_lowercase().contains(&ans),
                "rabbi gave the answer away: {text}"
            );
        }
        assert_eq!(l.shown.len(), 3);
        // Next step: shown help is cleared, used lifelines stay used.
        let k = right(&l);
        l.answer(&mut state, Some(k), day());
        assert!(l.shown.is_empty() && l.hidden.is_empty());
        assert!(Lifeline::ALL.iter().all(|x| !l.can_use(*x)));
    }

    #[test]
    fn timer_setting_allows_no_timer_and_defaults_to_30() {
        let mut s = State::default();
        assert_eq!(timer_seconds(&s), 30);
        s.settings.timed_seconds = 0;
        assert_eq!(timer_seconds(&s), 0);
        // The old timed exam still sees 30 for «no timer».
        assert_eq!(exam::base_seconds(&s), 30);
        s.settings.timed_seconds = 20;
        assert_eq!(timer_seconds(&s), 20);
        s.settings.timed_seconds = 45;
        assert_eq!(timer_seconds(&s), 30);
        assert_eq!(sanitize_seconds(-1), 30);
    }

    #[test]
    fn steps_word_forms() {
        assert_eq!(steps_word(1), "ступень");
        assert_eq!(steps_word(3), "ступени");
        assert_eq!(steps_word(5), "ступеней");
        assert_eq!(steps_word(11), "ступеней");
        assert_eq!(steps_word(12), "ступеней");
    }

    #[test]
    fn mask_answer_hides_every_occurrence() {
        assert_eq!(
            mask_answer("Кидуш говорят над вином. Кидуш — освящение.", "кидуш"),
            "… говорят над вином. … — освящение."
        );
        assert_eq!(mask_answer("Нет ответа", "ab"), "Нет ответа");
    }
}
