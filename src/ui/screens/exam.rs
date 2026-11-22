//! «Тесты»: exam setup, runner and results (PARITY §7–§9).
//!
//! The PWA keeps the session in a module global and navigates
//! `#/exam` → `#/exam-run` → `#/exam-results`; here the three views are
//! phases of one screen, with `Route::Exam` / `Route::ExamRun` as the
//! entry points. All scoring, selection and XP rules live in
//! `crate::exam` / `crate::game`; this screen owns only view state.

use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use iced::keyboard::{self, key};
use iced::widget::canvas::{self, Canvas, Frame, Path, Stroke};
use iced::widget::text::Shaping;
use iced::widget::{
    button, center, checkbox, column, container, pick_list, row, space, stack, text, text_editor,
    text_input, Column, Row, Text,
};
use iced::{
    Alignment, Background, Border, Color, Element, Length, Point, Rectangle, Renderer, Shadow,
    Subscription, Theme,
};

use crate::content::{Differs, Library, Question, SectionMeta};
use crate::exam::{self, CheckResult, Mode, Response};
use crate::game::{self, AwardOutcome};
use crate::ui::app::{Action, Ctx, ExamSpec, Route, Toast};
use crate::ui::fonts::{SANS, SANS_BOLD, SERIF_BOLD};
use crate::ui::screens::map::icons;
use crate::ui::theme::{self, Palette};
use crate::ui::widgets;

// ---------------------------------------------------------------------------
// Shared view helpers
// ---------------------------------------------------------------------------

fn tp() -> Palette {
    theme::current()
}

fn txt<'a>(content: impl text::IntoFragment<'a>) -> Text<'a> {
    text(content).shaping(Shaping::Advanced)
}

fn muted<'a>(content: impl text::IntoFragment<'a>) -> Text<'a> {
    txt(content).color(tp().muted)
}

/// Serif prompt text (question wording, mistake headlines).
fn serif<'a>(content: impl text::IntoFragment<'a>, size: f32) -> Text<'a> {
    txt(content).font(SERIF_BOLD).size(size)
}

/// Small gold uppercase meta line («{раздел} · {формат} · {сложность}»).
fn gold_meta<'a>(content: impl text::IntoFragment<'a>) -> Text<'a> {
    txt(content)
        .font(SANS_BOLD)
        .size(theme::text::KICKER)
        .color(tp().gold)
}

/// «kbd» key hint chip (§T.4: small, outlined, muted).
fn kbd<'a, Message: 'a>(label: impl text::IntoFragment<'a>) -> Element<'a, Message> {
    let p = tp();
    container(txt(label).size(12.0).font(SANS_BOLD).color(p.muted))
        .padding([2.0, 7.0])
        .style(move |_| container::Style {
            background: Some(Background::Color(p.surface3)),
            border: Border {
                color: p.line2,
                width: 1.0,
                radius: 6.0.into(),
            },
            ..plain()
        })
        .into()
}

fn plain() -> container::Style {
    container::Style {
        text_color: None,
        background: None,
        border: Border::default(),
        shadow: Shadow::default(),
        snap: true,
    }
}

/// Tinted feedback / model box (`.fb.ok|part|bad`, `.model`).
fn tinted_box<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    bg: Color,
    border: Color,
) -> Element<'a, Message> {
    container(content)
        .padding([16.0, 18.0])
        .width(Length::Fill)
        .style(move |_| container::Style {
            background: Some(Background::Color(bg)),
            border: Border {
                color: border,
                width: 1.0,
                radius: theme::radius::SMALL.into(),
            },
            ..plain()
        })
        .into()
}

fn alpha(c: Color, a: f32) -> Color {
    Color { a, ..c }
}

/// Score ring (`size` px) with the percentage in the middle (§9).
struct ScoreRing {
    frac: f32,
}

impl<Message> canvas::Program<Message> for ScoreRing {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let p = tp();
        let mut frame = Frame::new(renderer, bounds.size());
        let size = bounds.size();
        let min = size.width.min(size.height);
        let stroke_width = min * 0.057; // CSS stroke 8 of a 140 px ring
        let radius = (min - stroke_width) / 2.0;
        let center = Point::new(size.width / 2.0, size.height / 2.0);
        frame.stroke(
            &Path::circle(center, radius),
            Stroke {
                style: canvas::Style::Solid(p.surface3),
                width: stroke_width,
                line_cap: canvas::LineCap::Round,
                ..Stroke::default()
            },
        );
        let frac = self.frac.clamp(0.0, 1.0);
        if frac > 0.0 {
            let start = -std::f32::consts::FRAC_PI_2;
            let end = start + frac * std::f32::consts::TAU;
            let arc = Path::new(|b| {
                b.move_to(Point::new(
                    center.x + radius * start.cos(),
                    center.y + radius * start.sin(),
                ));
                b.arc(canvas::path::Arc {
                    center,
                    radius,
                    start_angle: iced::Radians(start),
                    end_angle: iced::Radians(end),
                });
            });
            frame.stroke(
                &arc,
                Stroke {
                    style: canvas::Style::Solid(p.gold),
                    width: stroke_width,
                    line_cap: canvas::LineCap::Round,
                    ..Stroke::default()
                },
            );
        }
        vec![frame.into_geometry()]
    }
}

fn score_ring<'a, Message: 'a>(pct: i64, size: f32) -> Element<'a, Message> {
    let p = tp();
    let canvas = Canvas::new(ScoreRing {
        frac: pct as f32 / 100.0,
    })
    .width(Length::Fixed(size))
    .height(Length::Fixed(size));
    let label = container(
        column![
            txt(format!("{pct}%"))
                .size(size * 0.19)
                .font(SERIF_BOLD)
                .color(p.text),
            txt("Результат").size(12.0).color(p.muted),
        ]
        .spacing(2)
        .align_x(Alignment::Center),
    )
    .width(Length::Fixed(size))
    .height(Length::Fixed(size))
    .align_x(Alignment::Center)
    .align_y(Alignment::Center);
    stack![canvas, label].width(Length::Shrink).into()
}

/// Quit-confirm dialog (§0.3): dimmed backdrop (click = cancel) + modal card.
/// Rendered as a shell layer above the whole window (see `screens::mod.rs`).
fn confirm_dialog<'a>(title: &'a str, body: &'a str) -> Element<'a, Msg> {
    let p = tp();
    let dialog = container(
        column![
            serif(title, theme::text::H2).color(p.text),
            txt(body).size(theme::text::BODY).color(p.text),
            row![
                space::horizontal(),
                widgets::btn(
                    "Отмена",
                    widgets::Btn::Ghost,
                    widgets::BtnSize::Md,
                    Some(Msg::QuitCancel),
                ),
                widgets::btn(
                    "Выйти",
                    widgets::Btn::Primary,
                    widgets::BtnSize::Md,
                    Some(Msg::QuitConfirm),
                ),
            ]
            .spacing(10)
            .width(Length::Fill),
        ]
        .spacing(14)
        .width(Length::Fill),
    )
    .padding(24)
    .max_width(460)
    .style(move |_| container::Style {
        background: Some(Background::Color(p.surface)),
        border: Border {
            color: p.line2,
            width: 1.0,
            radius: theme::radius::MODAL.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba8(0, 0, 0, 0.5),
            offset: iced::Vector::new(0.0, 12.0),
            blur_radius: 34.0,
        },
        ..plain()
    });
    let back = Color::from_rgba8(3, 5, 12, 0.66);
    let backdrop = button(container(space()).style(move |_| container::Style {
        background: Some(Background::Color(back)),
        ..plain()
    }))
    .width(Length::Fill)
    .height(Length::Fill)
    .on_press(Msg::QuitCancel);
    stack![backdrop, center(dialog)]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Badge / goal toasts after a finished run (mirrors `Ctx::award` effects;
/// the XP itself was already added by `exam::finish` / `exam::finish_timed`).
fn celebrate_award(ctx: &mut Ctx, award: &AwardOutcome) {
    if let Some(c) = award.celebration.clone() {
        ctx.celebrate(c);
    }
    for b in &award.new_badges {
        let c = b.celebration();
        ctx.toasts.push(Toast {
            title: c.title,
            text: c.text,
            icon: c.icon,
            badge_id: Some(b.id.clone()),
            until: Instant::now() + Duration::from_millis(5200),
        });
    }
}

/// A body-size rich line from (text, bold) fragments (owned, no lifetime ties).
fn rich_line<'a, Message: 'a>(
    parts: Vec<(String, bool)>,
    size: f32,
    color: Color,
) -> Element<'a, Message> {
    let spans: Vec<iced::widget::text::Span<'a>> = parts
        .into_iter()
        .map(|(t, bold)| {
            let s = iced::widget::text::Span::new(t)
                .font(if bold { SANS_BOLD } else { SANS })
                .color(color)
                .size(size);
            s
        })
        .collect();
    iced::widget::rich_text(spans)
        .size(size)
        .line_height(1.6)
        .width(Length::Fill)
        .into()
}

fn format_name(q: &Question) -> &'static str {
    match q {
        Question::Mc { .. } => "выбор ответа",
        Question::Tf { .. } => "верно / неверно",
        Question::Match { .. } => "сопоставление",
        Question::Typed { .. } => "впишите ответ",
        Question::Open { .. } => "открытый вопрос",
    }
}

// ---------------------------------------------------------------------------
// Setup state (persists in memory across visits, like the PWA module var)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupMode {
    Section,
    General,
    Beitdin,
}

#[derive(Debug, Clone)]
struct Setup {
    mode: SetupMode,
    section: Option<String>,
    count: usize,
    only_unlocked: bool,
}

fn setup_store() -> &'static Mutex<Setup> {
    static SETUP: OnceLock<Mutex<Setup>> = OnceLock::new();
    SETUP.get_or_init(|| {
        Mutex::new(Setup {
            mode: SetupMode::Section,
            section: None,
            count: 20,
            only_unlocked: true,
        })
    })
}

impl Setup {
    /// «Доступно вопросов» for the current mode (js/ui/exam.js available()).
    fn available(&self, ctx: &Ctx) -> usize {
        let user = Library::user_level(&ctx.state);
        match self.mode {
            SetupMode::Section => self
                .section
                .as_deref()
                .and_then(|id| ctx.lib.section(id))
                .map(|s| ctx.lib.questions(s, user).len())
                .unwrap_or(0),
            SetupMode::General => {
                let secs: Vec<&SectionMeta> = if self.only_unlocked {
                    ctx.lib.unlocked_sections(&ctx.state)
                } else {
                    ctx.lib.sections.iter().filter(|s| s.available).collect()
                };
                secs.iter().map(|s| ctx.lib.questions(s, user).len()).sum()
            }
            SetupMode::Beitdin => ctx
                .lib
                .sections
                .iter()
                .filter(|s| s.available)
                .map(|s| {
                    ctx.lib
                        .questions(s, user)
                        .iter()
                        .filter(|q| {
                            matches!(q, Question::Open { .. } | Question::Typed { .. })
                                || s.id == "beit-din"
                        })
                        .count()
                })
                .sum(),
        }
    }

    fn spec(&self) -> ExamSpec {
        match self.mode {
            SetupMode::Section => ExamSpec::Section {
                id: self.section.clone().unwrap_or_default(),
                count: self.count,
            },
            SetupMode::General => ExamSpec::General { count: self.count },
            SetupMode::Beitdin => ExamSpec::BeitDin { count: self.count },
        }
    }
}

// ---------------------------------------------------------------------------
// Run state
// ---------------------------------------------------------------------------

/// A session question made owned (the core `exam::Item` borrows the library).
struct OwnItem {
    q: Question,
    sec_title: Option<String>,
    order: Option<Vec<usize>>,
    rights: Option<Vec<String>>,
}

struct Run {
    mode: Mode,
    section_id: Option<String>,
    items: Vec<OwnItem>,
    idx: usize,
    answers: Vec<Option<CheckResult>>,
    started_at: i64,
    // Transient, per-question view state.
    answered: Option<CheckResult>,
    typed: String,
    match_sel: Vec<Option<String>>,
    open_text: text_editor::Content,
    open_revealed: bool,
    kp: Vec<bool>,
    differs_open: bool,
    confirm_quit: bool,
}

impl Run {
    fn new(mode: Mode, section_id: Option<String>, items: Vec<OwnItem>, started_at: i64) -> Run {
        let mut run = Run {
            mode,
            section_id,
            answers: vec![None; items.len()],
            items,
            idx: 0,
            started_at,
            answered: None,
            typed: String::new(),
            match_sel: Vec::new(),
            open_text: text_editor::Content::new(),
            open_revealed: false,
            kp: Vec::new(),
            differs_open: false,
            confirm_quit: false,
        };
        run.reset_question();
        run
    }

    fn reset_question(&mut self) {
        self.answered = None;
        self.typed = String::new();
        self.open_text = text_editor::Content::new();
        self.open_revealed = false;
        self.differs_open = false;
        let cur = &self.items[self.idx].q;
        self.match_sel = match cur {
            Question::Match { pairs, .. } => vec![None; pairs.len()],
            _ => Vec::new(),
        };
        self.kp = match cur {
            Question::Open { key_points, .. } => vec![false; key_points.len()],
            _ => Vec::new(),
        };
    }

    fn section_title<'a>(&self, ctx: &'a Ctx) -> Option<&'a str> {
        self.section_id
            .as_deref()
            .and_then(|id| ctx.lib.section(id))
            .map(|s| s.title.as_str())
    }

    /// Check + record an answer (js/exam.js E.answer).
    fn answer(&mut self, ctx: &mut Ctx, response: Response) {
        if self.answered.is_some() {
            return;
        }
        let idx = self.idx;
        let it = &self.items[idx];
        let item = exam::Item {
            q: &it.q,
            sec: None,
            order: it.order.clone(),
            rights: it.rights.clone(),
        };
        let r = exam::check(&item, &response);
        let today = ctx.today();
        exam::record(&mut ctx.state, it.q.id(), &r, today);
        self.answers[idx] = Some(r.clone());
        self.answered = Some(r);
        ctx.save();
    }

    /// Finish the session: results, history, best score, XP (§8 «Finish»).
    fn finish(&self, ctx: &mut Ctx) -> Results {
        let items: Vec<exam::Item> = self
            .items
            .iter()
            .map(|it| {
                let sec = ctx.lib.question(it.q.id()).map(|(_, s)| s);
                exam::Item {
                    q: &it.q,
                    sec,
                    order: it.order.clone(),
                    rights: it.rights.clone(),
                }
            })
            .collect();
        let session = exam::Session {
            mode: self.mode,
            section_id: self.section_id.clone(),
            items,
            idx: self.idx,
            answers: self.answers.clone(),
            started_at: self.started_at,
        };
        let today = ctx.today();
        let res = exam::finish(&ctx.lib, &session, &mut ctx.state, today);
        celebrate_award(ctx, &res.award);
        ctx.save();
        let cards = res
            .mistakes
            .iter()
            .map(|&i| {
                let it = &self.items[i];
                let user = self.answers[i]
                    .as_ref()
                    .map(|a| a.user.clone())
                    .unwrap_or_else(|| "—".to_string());
                let (open, right) = match &it.q {
                    Question::Open { model_answer, .. } => (true, model_answer.clone()),
                    q => (false, exam::correct_text(q)),
                };
                let c = it.q.common();
                MistakeCard {
                    prompt: c.prompt.clone(),
                    user,
                    open,
                    right,
                    explanation: c.explanation.clone(),
                    source: c.source.clone(),
                }
            })
            .collect();
        let mistake_ids = res
            .mistakes
            .iter()
            .map(|&i| self.items[i].q.id().to_string())
            .collect();
        Results {
            res,
            cards,
            mistake_ids,
        }
    }
}

// ---------------------------------------------------------------------------
// Results state
// ---------------------------------------------------------------------------

struct MistakeCard {
    prompt: String,
    user: String,
    open: bool,
    right: String,
    explanation: Option<String>,
    source: Option<String>,
}

struct Results {
    res: exam::ExamResult,
    cards: Vec<MistakeCard>,
    mistake_ids: Vec<String>,
}

// ---------------------------------------------------------------------------
// Screen
// ---------------------------------------------------------------------------

enum Phase {
    Setup(Setup),
    Run(Box<Run>),
    Results(Box<Results>),
}

pub struct Screen {
    phase: Phase,
}

#[derive(Debug, Clone)]
pub enum Msg {
    // Setup.
    Mode(SetupMode),
    SectionLabel(String),
    Count(usize),
    OnlyUnlocked(bool),
    Start,
    // Runner.
    PickMc(usize),
    PickTf(bool),
    Typed(String),
    TypedSubmit,
    MatchPick(usize, String),
    MatchSubmit,
    OpenEdit(text_editor::Action),
    OpenReveal,
    KeyPoint(usize, bool),
    SelfGrade(f64),
    Next,
    DiffersToggle,
    Quit,
    QuitConfirm,
    QuitCancel,
    // Results.
    Retry,
    NewExam,
    ToSection(String),
    ToMap,
    // Keyboard subscription.
    Key(keyboard::Key, keyboard::Modifiers),
}

fn default_setup(ctx: &Ctx) -> Setup {
    let mut setup = setup_store().lock().expect("setup lock").clone();
    normalize_setup(&mut setup, ctx, None);
    *setup_store().lock().expect("setup lock") = setup.clone();
    setup
}

/// `/exam/s/{id}` presets section mode; the stored section must be unlocked
/// (js/ui/exam.js lines 10–12).
fn normalize_setup(setup: &mut Setup, ctx: &Ctx, preset: Option<&String>) {
    let unlocked = ctx.lib.unlocked_sections(&ctx.state);
    if let Some(id) = preset {
        setup.mode = SetupMode::Section;
        setup.section = Some(id.clone());
    }
    let ok = setup
        .section
        .as_deref()
        .map(|id| unlocked.iter().any(|s| s.id == id))
        .unwrap_or(false);
    if !ok {
        setup.section = unlocked.first().map(|s| s.id.clone());
    }
}

/// Build an owned session from a spec (js/ui/exam.js start action).
fn build_run(ctx: &Ctx, spec: &ExamSpec) -> Option<Run> {
    let (mode, section_id, count, only_unlocked, ids) = match spec {
        ExamSpec::Section { id, count } => {
            (Mode::Section, Some(id.clone()), Some(*count), None, None)
        }
        ExamSpec::General { count } => (
            Mode::General,
            None,
            Some(*count),
            Some(setup_store().lock().expect("setup lock").only_unlocked),
            None,
        ),
        ExamSpec::BeitDin { count } => (Mode::Beitdin, None, Some(*count), None, None),
        ExamSpec::Retry { ids, section_id } => (
            Mode::Retry,
            section_id.clone(),
            None,
            None,
            Some(ids.clone()),
        ),
        ExamSpec::SelfCheck { section_id, ids } => (
            Mode::SelfCheck,
            Some(section_id.clone()),
            None,
            None,
            Some(ids.clone()),
        ),
        ExamSpec::Rehearsal { section_id, ids } => (
            Mode::Rehearsal,
            Some(section_id.clone()),
            None,
            None,
            Some(ids.clone()),
        ),
    };
    let mut rng = rand::thread_rng();
    let s = exam::build(
        &ctx.lib,
        &ctx.state,
        exam::BuildOpts {
            mode,
            section_id,
            count,
            only_unlocked,
            ids,
        },
        &mut rng,
    );
    if s.items.is_empty() {
        return None;
    }
    let items = s
        .items
        .iter()
        .map(|it| OwnItem {
            q: it.q.clone(),
            sec_title: it.sec.map(|m| m.title.clone()),
            order: it.order.clone(),
            rights: it.rights.clone(),
        })
        .collect();
    Some(Run::new(s.mode, s.section_id, items, s.started_at))
}

impl Screen {
    pub fn new(ctx: &mut Ctx, route: &Route) -> Self {
        match route {
            Route::ExamRun(spec) => {
                if let Some(run) = build_run(ctx, spec) {
                    return Screen {
                        phase: Phase::Run(Box::new(run)),
                    };
                }
                // No session → setup (PWA redirects to `#/exam`).
                Screen {
                    phase: Phase::Setup(default_setup(ctx)),
                }
            }
            _ => {
                let preset = match route {
                    Route::Exam(p) => p.as_ref(),
                    _ => None,
                };
                let mut setup = setup_store().lock().expect("setup lock").clone();
                normalize_setup(&mut setup, ctx, preset);
                *setup_store().lock().expect("setup lock") = setup.clone();
                Screen {
                    phase: Phase::Setup(setup),
                }
            }
        }
    }

    pub fn update(&mut self, ctx: &mut Ctx, msg: Msg) -> Action {
        match msg {
            // ---- Setup ----
            Msg::Mode(mode) => {
                if let Phase::Setup(s) = &mut self.phase {
                    s.mode = mode;
                    *setup_store().lock().expect("setup lock") = s.clone();
                }
                Action::None
            }
            Msg::SectionLabel(label) => {
                if let Phase::Setup(s) = &mut self.phase {
                    if let Some(id) = section_id_for_label(ctx, &label) {
                        s.section = Some(id);
                        *setup_store().lock().expect("setup lock") = s.clone();
                    }
                }
                Action::None
            }
            Msg::Count(n) => {
                if let Phase::Setup(s) = &mut self.phase {
                    s.count = n;
                    *setup_store().lock().expect("setup lock") = s.clone();
                }
                Action::None
            }
            Msg::OnlyUnlocked(v) => {
                if let Phase::Setup(s) = &mut self.phase {
                    s.only_unlocked = v;
                    *setup_store().lock().expect("setup lock") = s.clone();
                }
                Action::None
            }
            Msg::Start => {
                if let Phase::Setup(s) = &self.phase {
                    if s.available(ctx) > 0 {
                        return Action::Go(Route::ExamRun(s.spec()));
                    }
                }
                Action::None
            }
            // ---- Runner ----
            Msg::PickMc(orig) => {
                if let Phase::Run(run) = &mut self.phase {
                    run.answer(ctx, Response::Mc(orig));
                }
                Action::None
            }
            Msg::PickTf(v) => {
                if let Phase::Run(run) = &mut self.phase {
                    run.answer(ctx, Response::Tf(v));
                }
                Action::None
            }
            Msg::Typed(v) => {
                if let Phase::Run(run) = &mut self.phase {
                    run.typed = v;
                }
                Action::None
            }
            Msg::TypedSubmit => {
                if let Phase::Run(run) = &mut self.phase {
                    let v = std::mem::take(&mut run.typed);
                    run.answer(ctx, Response::Typed(v));
                }
                Action::None
            }
            Msg::MatchPick(i, v) => {
                if let Phase::Run(run) = &mut self.phase {
                    if let Some(slot) = run.match_sel.get_mut(i) {
                        *slot = Some(v);
                    }
                }
                Action::None
            }
            Msg::MatchSubmit => {
                if let Phase::Run(run) = &mut self.phase {
                    let sel: Vec<String> = run
                        .match_sel
                        .iter()
                        .map(|s| s.clone().unwrap_or_default())
                        .collect();
                    run.answer(ctx, Response::Match(sel));
                }
                Action::None
            }
            Msg::OpenEdit(action) => {
                if let Phase::Run(run) = &mut self.phase {
                    if !run.open_revealed {
                        run.open_text.perform(action);
                    }
                }
                Action::None
            }
            Msg::OpenReveal => {
                if let Phase::Run(run) = &mut self.phase {
                    run.open_revealed = true;
                }
                Action::None
            }
            Msg::KeyPoint(i, v) => {
                if let Phase::Run(run) = &mut self.phase {
                    if let Some(slot) = run.kp.get_mut(i) {
                        *slot = v;
                    }
                }
                Action::None
            }
            Msg::SelfGrade(v) => {
                if let Phase::Run(run) = &mut self.phase {
                    run.answer(ctx, Response::Open(v));
                }
                Action::None
            }
            Msg::Next => {
                let finished = match &self.phase {
                    Phase::Run(run) => run.answered.is_some() && run.idx + 1 >= run.items.len(),
                    _ => false,
                };
                if finished {
                    if let Phase::Run(run) = &self.phase {
                        let results = run.finish(ctx);
                        self.phase = Phase::Results(Box::new(results));
                    }
                } else if let Phase::Run(run) = &mut self.phase {
                    if run.answered.is_some() {
                        run.idx += 1;
                        run.reset_question();
                    }
                }
                Action::None
            }
            Msg::DiffersToggle => {
                if let Phase::Run(run) = &mut self.phase {
                    run.differs_open = !run.differs_open;
                }
                Action::None
            }
            Msg::Quit => {
                if let Phase::Run(run) = &mut self.phase {
                    run.confirm_quit = true;
                }
                Action::None
            }
            Msg::QuitConfirm => match &self.phase {
                // Self-check and rehearsal start from a section: go back there.
                Phase::Run(run) if matches!(run.mode, Mode::SelfCheck | Mode::Rehearsal) => {
                    match &run.section_id {
                        Some(id) => Action::Go(Route::Section(id.clone())),
                        None => Action::Go(Route::Exam(None)),
                    }
                }
                _ => Action::Go(Route::Exam(None)),
            },
            Msg::QuitCancel => {
                if let Phase::Run(run) = &mut self.phase {
                    run.confirm_quit = false;
                }
                Action::None
            }
            // ---- Results ----
            Msg::Retry => {
                if let Phase::Results(r) = &self.phase {
                    if !r.mistake_ids.is_empty() {
                        let ids = r.mistake_ids.clone();
                        // Practice runs retry as the same practice (interview
                        // chrome, back to the section).
                        let spec = match (r.res.mode, r.res.section_id.clone()) {
                            (Mode::SelfCheck, Some(section_id)) => {
                                ExamSpec::SelfCheck { section_id, ids }
                            }
                            (Mode::Rehearsal, Some(section_id)) => {
                                ExamSpec::Rehearsal { section_id, ids }
                            }
                            (_, section_id) => ExamSpec::Retry { ids, section_id },
                        };
                        return Action::Go(Route::ExamRun(spec));
                    }
                }
                Action::None
            }
            Msg::NewExam => Action::Go(Route::Exam(None)),
            Msg::ToSection(id) => Action::Go(Route::Section(id)),
            Msg::ToMap => Action::Go(Route::Map),
            // ---- Keyboard ----
            Msg::Key(key, modifiers) => {
                if modifiers.command() || modifiers.alt() {
                    return Action::None;
                }
                let Phase::Run(run) = &mut self.phase else {
                    return Action::None;
                };
                if run.confirm_quit {
                    if matches!(key, keyboard::Key::Named(key::Named::Escape)) {
                        run.confirm_quit = false;
                    }
                    return Action::None;
                }
                if run.answered.is_some() {
                    if matches!(key, keyboard::Key::Named(key::Named::Enter)) {
                        return self.update(ctx, Msg::Next);
                    }
                    return Action::None;
                }
                // Digits 1…n pick an mc/tf option (§8 «Keys»).
                if let keyboard::Key::Character(c) = &key {
                    if let Ok(n) = c.parse::<usize>() {
                        if n >= 1 {
                            match &run.items[run.idx].q {
                                Question::Mc { options, .. } => {
                                    if n <= options.len() {
                                        if let Some(order) = &run.items[run.idx].order {
                                            if let Some(&orig) = order.get(n - 1) {
                                                return self.update(ctx, Msg::PickMc(orig));
                                            }
                                        }
                                    }
                                }
                                Question::Tf { .. } => match n {
                                    1 => return self.update(ctx, Msg::PickTf(true)),
                                    2 => return self.update(ctx, Msg::PickTf(false)),
                                    _ => {}
                                },
                                _ => {}
                            }
                        }
                    }
                }
                Action::None
            }
        }
    }

    pub fn view<'a>(&'a self, ctx: &'a Ctx) -> Element<'a, Msg> {
        match &self.phase {
            Phase::Setup(setup) => self.view_setup(ctx, setup),
            Phase::Run(run) => self.view_run(ctx, run),
            Phase::Results(r) => self.view_results(ctx, r),
        }
    }

    /// The quit-confirm dialog, rendered as a shell layer above the window.
    pub fn dialog<'a>(&'a self, _ctx: &'a Ctx) -> Option<Element<'a, Msg>> {
        match &self.phase {
            Phase::Run(run) if run.confirm_quit => Some(confirm_dialog(
                "Прервать экзамен?",
                "Ответы на текущие вопросы сохранятся в статистике, но результат не будет засчитан.",
            )),
            _ => None,
        }
    }

    pub fn subscription(&self) -> Subscription<Msg> {
        match &self.phase {
            Phase::Run(_) => keyboard::listen().filter_map(|ev| match ev {
                keyboard::Event::KeyPressed { key, modifiers, .. } => {
                    Some(Msg::Key(key, modifiers))
                }
                _ => None,
            }),
            _ => Subscription::none(),
        }
    }
}

/// Map a section pick-list label back to the section id.
fn section_id_for_label(ctx: &Ctx, label: &str) -> Option<String> {
    ctx.lib
        .unlocked_sections(&ctx.state)
        .iter()
        .find(|s| section_label(s, ctx) == label)
        .map(|s| s.id.clone())
}

/// «{n}. {title}» + « — лучший {best}%» (§7).
fn section_label(sec: &SectionMeta, ctx: &Ctx) -> String {
    let base = format!("{}. {}", sec.index + 1, sec.title);
    match Library::best(&ctx.state, &sec.id) {
        Some(b) => format!("{base} — лучший {}", b.round() as i64) + "%",
        None => base,
    }
}

// ---------------------------------------------------------------------------
// Setup view (§7)
// ---------------------------------------------------------------------------

impl Screen {
    fn view_setup<'a>(&'a self, ctx: &'a Ctx, setup: &'a Setup) -> Element<'a, Msg> {
        let p = tp();
        let mut col = Column::new().spacing(18).width(Length::Fill);
        col = col.push(widgets::kicker("Проверка знаний"));
        col = col.push(widgets::heading("Экзамен", 1));
        col = col.push(
            muted("Набор вопросов каждый раз новый; чаще попадаются те, в которых были ошибки.")
                .size(theme::text::BODY),
        );

        // Mode tiles (radio group): an engraved emblem, title and one line
        // of description; the selected tile gets a brass edge, a brass wash
        // and a check mark in the corner.
        let modes: [(SetupMode, icons::Kind, &str, &str); 3] = [
            (
                SetupMode::Section,
                icons::Kind::Book,
                "По разделу",
                "Проверочный тест: 80% и выше открывает следующий раздел.",
            ),
            (
                SetupMode::General,
                icons::Kind::Shuffle,
                "Общий экзамен",
                "Вопросы из всех разделов вперемешку.",
            ),
            (
                SetupMode::Beitdin,
                icons::Kind::Scales,
                "Пробный бейт-дин",
                "Открытые вопросы как на собеседовании; оцениваете себя сами.",
            ),
        ];
        // Equal-height tiles inside the page scrollable: a `Fill` height
        // collapses to zero there, so an invisible copy of the tiles at
        // their natural height (the stack's base layer) sets the row height
        // and the real tiles fill it.
        let tile = move |kind: icons::Kind, title: &'a str, desc: &'a str, selected: bool| {
            let emblem = widgets::disc(
                icons::icon(kind, 22.0, p.gold2),
                44.0,
                if selected { p.gold_soft } else { p.surface2 },
                if selected { p.gold } else { p.line2 },
            );
            let mark: Element<'a, Msg> = if selected {
                widgets::disc(
                    icons::icon(icons::Kind::Check, 13.0, p.on_gold),
                    22.0,
                    p.brass_mid,
                    p.brass_edge,
                )
            } else {
                iced::widget::space().width(22).height(22).into()
            };
            column![
                row![emblem, space::horizontal(), mark].align_y(Alignment::Start),
                txt(title)
                    .font(SANS_BOLD)
                    .size(theme::text::BODY)
                    .color(if selected { p.gold2 } else { p.text }),
                txt(desc).size(theme::text::SMALL).color(p.muted),
            ]
            .spacing(10)
            .width(Length::Fill)
        };
        let mut sizer = Row::new().spacing(GAP).width(Length::Fill);
        let mut cards = Row::new()
            .spacing(GAP)
            .width(Length::Fill)
            .height(Length::Fill);
        for (mode, kind, title, desc) in modes {
            let selected = setup.mode == mode;
            // Same padding and 1 px border as the button, so the ghost
            // measures exactly the real tile.
            sizer = sizer.push(
                container(tile(kind, title, desc, selected))
                    .width(Length::FillPortion(1))
                    .padding(TILE_PAD)
                    .style(|_| iced::widget::container::Style {
                        border: Border {
                            width: 1.0,
                            color: Color::TRANSPARENT,
                            ..Border::default()
                        },
                        ..Default::default()
                    }),
            );
            let b = button(tile(kind, title, desc, selected))
                .width(Length::FillPortion(1))
                .height(Length::Fill)
                .padding(TILE_PAD)
                .style(move |_, status| {
                    let hovered =
                        matches!(status, button::Status::Hovered | button::Status::Pressed);
                    container_button_style(
                        &p,
                        if selected && hovered {
                            Some(Background::Color(crate::ui::screens::map::alpha(
                                p.gold, 0.22,
                            )))
                        } else if selected {
                            Some(Background::Color(p.gold_soft))
                        } else if hovered {
                            Some(Background::Color(p.surface3))
                        } else {
                            Some(Background::Color(p.surface))
                        },
                        if selected {
                            p.gold
                        } else if hovered {
                            crate::ui::screens::map::alpha(p.gold, 0.5)
                        } else {
                            p.line2
                        },
                    )
                })
                .on_press(Msg::Mode(mode));
            cards = cards.push(b);
        }
        col = col.push(
            iced::widget::stack![widgets::ghost(sizer), cards]
                .width(Length::Fill)
                .height(Length::Shrink),
        );

        // Options card: a two-column form (label + hint on the left, the
        // control on the right, every control starting at the same x), then
        // a footer with the available count and the start button.
        let form_row = |label: &'a str, hint: Option<&'a str>, control: Element<'a, Msg>| {
            let mut l = column![txt(label).size(theme::text::BODY).color(p.text)].spacing(2);
            if let Some(h) = hint {
                l = l.push(txt(h).size(theme::text::SMALL).color(p.muted));
            }
            row![
                container(l).width(Length::Fixed(LABEL_W)),
                container(control).width(Length::Fill),
            ]
            .spacing(GAP)
            .align_y(Alignment::Center)
            .width(Length::Fill)
        };
        let divider = move || {
            container(iced::widget::space())
                .width(Length::Fill)
                .height(1)
                .style(move |_| theme::divider_v(&p))
        };
        let mut card_col = Column::new().spacing(16).width(Length::Fill);
        match setup.mode {
            SetupMode::Section => {
                let unlocked = ctx.lib.unlocked_sections(&ctx.state);
                let labels: Vec<String> = unlocked.iter().map(|s| section_label(s, ctx)).collect();
                let selected = setup.section.as_deref().and_then(|id| {
                    unlocked
                        .iter()
                        .position(|s| s.id == id)
                        .map(|i| labels[i].clone())
                });
                let control: Element<'a, Msg> = if labels.is_empty() {
                    muted("Нет открытых разделов")
                        .size(theme::text::SMALL)
                        .into()
                } else {
                    pick_list(labels, selected, Msg::SectionLabel)
                        .placeholder("— выберите —")
                        .text_shaping(Shaping::Advanced)
                        .padding([8, 12])
                        .width(Length::Fill)
                        .style(move |_, st| theme::pick_list(&p, st))
                        .menu_style(move |_| theme::pick_menu(&p))
                        .into()
                };
                card_col = card_col.push(form_row(
                    "Раздел",
                    Some("Доступны открытые разделы"),
                    control,
                ));
            }
            SetupMode::General => {
                // The whole row toggles, not only the small switch.
                card_col = card_col.push(
                    button(form_row(
                        "Только открытые разделы",
                        Some("Иначе — из всех разделов"),
                        container(
                            iced::widget::toggler(setup.only_unlocked).on_toggle(Msg::OnlyUnlocked),
                        )
                        .align_right(Length::Fill)
                        .into(),
                    ))
                    .padding(0)
                    .width(Length::Fill)
                    .style(|_, _| button::Style {
                        background: None,
                        ..button::Style::default()
                    })
                    .on_press(Msg::OnlyUnlocked(!setup.only_unlocked)),
                );
            }
            SetupMode::Beitdin => {
                card_col = card_col.push(
                    column![
                        txt("Как это проходит")
                            .size(theme::text::BODY)
                            .color(p.text),
                        muted(
                            "Вопросы задаются по одному, как на настоящем собеседовании. \
                             Подумайте или ответьте вслух, затем сравните с эталоном.",
                        )
                        .size(theme::text::SMALL),
                    ]
                    .spacing(4)
                    .width(Length::Fill),
                );
            }
        }
        card_col = card_col.push(divider());

        let counts = [10usize, 20, 30, 50];
        let seg = widgets::segmented(
            counts
                .iter()
                .map(|&n| (n.to_string(), Msg::Count(n)))
                .collect(),
            counts.iter().position(|&n| n == setup.count),
        );
        card_col = card_col.push(form_row("Количество вопросов", None, seg));
        card_col = card_col.push(divider());

        let avail = setup.available(ctx);
        let mut avail_line = format!("Доступно вопросов: {avail}");
        if avail < setup.count && avail > 0 {
            avail_line.push_str(&format!(" — будет задано {avail}"));
        }
        let start = widgets::btn(
            "Начать",
            widgets::Btn::Primary,
            widgets::BtnSize::Lg,
            (avail > 0).then_some(Msg::Start),
        );
        card_col = card_col.push(
            row![
                muted(avail_line)
                    .size(theme::text::SMALL)
                    .width(Length::Fill),
                start,
            ]
            .spacing(GAP)
            .align_y(Alignment::Center)
            .width(Length::Fill),
        );

        col = col.push(widgets::card(card_col));
        col.into()
    }
}

/// Gap between the mode tiles and between form columns (the page grid).
const GAP: f32 = 16.0;
/// Width of the label column in the options card.
/// (224 px fits «Только открытые разделы» on one line and leaves the
/// controls ≈ 145 px at the 720 px minimum window.)
const LABEL_W: f32 = 224.0;
/// Inner padding of a mode tile (shared by the ghost sizer and the button).
const TILE_PAD: f32 = 18.0;

fn container_button_style(
    p: &Palette,
    background: Option<Background>,
    border_color: Color,
) -> button::Style {
    button::Style {
        background,
        text_color: p.text,
        border: Border {
            color: border_color,
            width: 1.0,
            radius: theme::radius::CARD.into(),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

// ---------------------------------------------------------------------------
// Runner view (§8)
// ---------------------------------------------------------------------------

impl Screen {
    fn view_run<'a>(&'a self, ctx: &'a Ctx, run: &'a Run) -> Element<'a, Msg> {
        let p = tp();
        let interview = matches!(run.mode, Mode::Beitdin | Mode::Rehearsal);
        let it = &run.items[run.idx];
        let q = &it.q;
        let total = run.items.len();

        let mut col = Column::new().spacing(16).width(Length::Fill);

        // Top: quit, progress, kicker.
        let quit = widgets::exit_button("Выйти", Msg::Quit);
        let mut kicker = exam::mode_title(run.mode).to_string();
        if let Some(t) = run.section_title(ctx) {
            kicker.push_str(" · ");
            kicker.push_str(t);
        }
        let progress_label = if interview {
            format!("Вопрос раввина {} из {total}", run.idx + 1)
        } else {
            format!("Вопрос {} из {total}", run.idx + 1)
        };
        col = col.push(
            row![
                quit,
                txt(progress_label).size(theme::text::SMALL).color(p.muted),
                space::horizontal(),
                gold_meta(kicker.to_uppercase()),
            ]
            .spacing(14)
            .align_y(Alignment::Center)
            .width(Length::Fill),
        );
        col = col.push(widgets::step_bar(run.idx, total));

        // Question card.
        let mut card_col = Column::new().spacing(14).width(Length::Fill);
        let mut meta = String::new();
        if let Some(t) = &it.sec_title {
            meta.push_str(t);
            meta.push_str(" · ");
        }
        meta.push_str(format_name(q));
        if q.common().difficulty.is_some() {
            meta.push_str(" · ");
            meta.push_str(exam::diff_title(q.common().difficulty));
        }
        card_col = card_col.push(gold_meta(meta.to_uppercase()));

        if interview {
            card_col = card_col.push(
                container(
                    column![
                        gold_meta("РАВВИН"),
                        serif(q.common().prompt.as_str(), theme::text::BODY * 1.22).color(p.text),
                    ]
                    .spacing(6)
                    .width(Length::Fill),
                )
                .padding([18.0, 20.0])
                .width(Length::Fill)
                .style(move |_| container::Style {
                    background: Some(Background::Color(p.surface2)),
                    border: Border {
                        color: p.line2,
                        width: 1.0,
                        radius: iced::border::Radius {
                            top_left: 18.0,
                            top_right: 18.0,
                            bottom_right: 18.0,
                            bottom_left: 4.0,
                        },
                    },
                    ..plain()
                }),
            );
        } else {
            card_col =
                card_col.push(serif(q.common().prompt.as_str(), theme::text::H2).color(p.text));
        }

        card_col = card_col.push(self.view_body(run, it));
        if run.answered.is_some() {
            card_col = card_col.push(self.view_feedback(ctx, run, it));
        }
        col = col.push(widgets::card(card_col));
        col.into()
    }

    /// The interactive part (mc / tf / typed / match / open).
    fn view_body<'a>(&'a self, run: &'a Run, it: &'a OwnItem) -> Element<'a, Msg> {
        let p = tp();
        let answered = run.answered.as_ref();
        match &it.q {
            Question::Mc {
                options, answer, ..
            } => {
                let order = it
                    .order
                    .clone()
                    .unwrap_or_else(|| (0..options.len()).collect::<Vec<usize>>());
                let mut list = Column::new().spacing(10).width(Length::Fill);
                for (k, oi) in order.iter().enumerate() {
                    let is_right = answered.is_some() && *oi == *answer;
                    let is_chosen_wrong = !is_right
                        && answered
                            .map(|a| !a.correct && a.user == options[*oi])
                            .unwrap_or(false);
                    let label = row![
                        kbd((k + 1).to_string()),
                        txt(options[*oi].as_str())
                            .size(theme::text::BODY)
                            .color(p.text),
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center)
                    .width(Length::Fill);
                    let b = button(label)
                        .width(Length::Fill)
                        .padding([10.0, 14.0])
                        .style(move |_, s| {
                            if is_right {
                                theme::option_correct(&p, s)
                            } else if is_chosen_wrong {
                                theme::option_wrong(&p, s)
                            } else {
                                theme::option_normal(&p, s)
                            }
                        });
                    let b = if answered.is_some() {
                        b
                    } else {
                        b.on_press(Msg::PickMc(*oi))
                    };
                    list = list.push(b);
                }
                list.into()
            }
            Question::Tf { answer, .. } => {
                let mut row = Row::new().spacing(10).width(Length::Fill);
                for (v, label, key_hint) in [(true, "Верно", "1"), (false, "Неверно", "2")]
                {
                    let is_right = answered.is_some() && v == *answer;
                    let is_chosen_wrong = !is_right
                        && answered
                            .map(|a| !a.correct && a.user == label)
                            .unwrap_or(false);
                    let content = iced::widget::row![
                        kbd(key_hint),
                        txt(label).size(theme::text::BODY).color(p.text),
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center);
                    let b = button(content)
                        .width(Length::FillPortion(1))
                        .padding([10.0, 14.0])
                        .style(move |_, s| {
                            if is_right {
                                theme::option_correct(&p, s)
                            } else if is_chosen_wrong {
                                theme::option_wrong(&p, s)
                            } else {
                                theme::option_normal(&p, s)
                            }
                        });
                    let b = if answered.is_some() {
                        b
                    } else {
                        b.on_press(Msg::PickTf(v))
                    };
                    row = row.push(b);
                }
                row.into()
            }
            Question::Typed { .. } => {
                if let Some(r) = answered {
                    // Disabled input echo.
                    column![txt(format!("Ваш ответ: {}", r.user))
                        .size(theme::text::BODY)
                        .color(p.muted),]
                    .spacing(10)
                    .into()
                } else {
                    row![
                        text_input("Ваш ответ", &run.typed)
                            .on_input(Msg::Typed)
                            .on_submit(Msg::TypedSubmit)
                            .padding(12.0)
                            .size(theme::text::BODY)
                            .width(Length::Fill),
                        widgets::btn(
                            "Проверить",
                            widgets::Btn::Primary,
                            widgets::BtnSize::Md,
                            Some(Msg::TypedSubmit),
                        ),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center)
                    .width(Length::Fill)
                    .into()
                }
            }
            Question::Match { pairs, .. } => {
                let rights = it.rights.clone().unwrap_or_default();
                let mut list = Column::new().spacing(10).width(Length::Fill);
                for (i, (left, correct)) in pairs.iter().enumerate() {
                    let chosen = run.match_sel.get(i).cloned().flatten();
                    let mut row_content = Row::new().spacing(14).align_y(Alignment::Center);
                    row_content = row_content.push(
                        container(txt(left.as_str()).size(theme::text::BODY).color(p.text))
                            .width(Length::FillPortion(1)),
                    );
                    if answered.is_some() {
                        let good = chosen.as_deref() == Some(correct.as_str());
                        let colr = if good { p.ok } else { p.bad };
                        row_content = row_content.push(
                            container(
                                txt(chosen.clone().unwrap_or_else(|| "—".to_string()))
                                    .size(theme::text::BODY)
                                    .color(colr),
                            )
                            .width(Length::FillPortion(1)),
                        );
                        if !good {
                            row_content = row_content.push(
                                txt(format!("→ {correct}"))
                                    .size(theme::text::SMALL + 1.0)
                                    .color(p.ok),
                            );
                        }
                    } else {
                        let pl = pick_list(rights.clone(), chosen, move |v| Msg::MatchPick(i, v))
                            .placeholder("— выберите —")
                            .text_shaping(Shaping::Advanced)
                            .width(Length::FillPortion(1))
                            .style(move |_, st| theme::pick_list(&p, st))
                            .menu_style(move |_| theme::pick_menu(&p));
                        row_content = row_content.push(pl);
                    }
                    list = list.push(row_content.width(Length::Fill));
                }
                if answered.is_none() {
                    list = list.push(
                        row![
                            space::horizontal(),
                            widgets::btn(
                                "Проверить",
                                widgets::Btn::Primary,
                                widgets::BtnSize::Md,
                                Some(Msg::MatchSubmit),
                            ),
                        ]
                        .width(Length::Fill),
                    );
                }
                list.into()
            }
            Question::Open {
                model_answer,
                key_points,
                ..
            } => {
                let mut col = Column::new().spacing(14).width(Length::Fill);
                if run.open_revealed {
                    let written = run.open_text.text();
                    if !written.trim().is_empty() {
                        col = col.push(
                            column![
                                muted("Ваш ответ").size(theme::text::SMALL),
                                txt(written).size(theme::text::BODY).color(p.text),
                            ]
                            .spacing(4),
                        );
                    }
                    // Model box (gold-soft).
                    let mut model = Column::new().spacing(10).width(Length::Fill);
                    model = model.push(
                        txt("Эталонный ответ")
                            .font(SERIF_BOLD)
                            .size(theme::text::BODY)
                            .color(p.gold2),
                    );
                    model = model.push(
                        txt(model_answer.as_str())
                            .size(theme::text::BODY)
                            .color(p.text),
                    );
                    if !key_points.is_empty() {
                        model = model.push(
                            txt("Проверьте себя — упомянуты ли ключевые мысли:")
                                .size(theme::text::SMALL)
                                .color(p.text),
                        );
                        for (i, kp) in key_points.iter().enumerate() {
                            let checked = run.kp.get(i).copied().unwrap_or(false);
                            model = model.push(
                                checkbox(checked)
                                    .label(kp.as_str())
                                    .on_toggle(move |v| Msg::KeyPoint(i, v))
                                    .text_shaping(Shaping::Advanced)
                                    .text_size(theme::text::BODY)
                                    .size(20),
                            );
                        }
                    }
                    col = col.push(tinted_box(model, p.gold_soft, alpha(p.gold, 0.3)));
                    if answered.is_none() {
                        col = col.push(
                            row![
                                widgets::btn(
                                    "Ответил верно",
                                    widgets::Btn::Primary,
                                    widgets::BtnSize::Md,
                                    Some(Msg::SelfGrade(1.0)),
                                ),
                                widgets::btn(
                                    "Частично",
                                    widgets::Btn::Secondary,
                                    widgets::BtnSize::Md,
                                    Some(Msg::SelfGrade(0.5)),
                                ),
                                widgets::btn(
                                    "Не ответил",
                                    widgets::Btn::Ghost,
                                    widgets::BtnSize::Md,
                                    Some(Msg::SelfGrade(0.0)),
                                ),
                            ]
                            .spacing(10),
                        );
                    }
                } else {
                    col = col.push(
                        text_editor(&run.open_text)
                            .placeholder(
                                "Сформулируйте ответ письменно или вслух (необязательно записывать)",
                            )
                            .on_action(Msg::OpenEdit)
                            .height(Length::Fixed(4.0 * 26.0))
                            .padding(12.0)
                            .size(theme::text::BODY),
                    );
                    col = col.push(
                        row![
                            space::horizontal(),
                            widgets::btn(
                                "Показать эталонный ответ",
                                widgets::Btn::Primary,
                                widgets::BtnSize::Md,
                                Some(Msg::OpenReveal),
                            ),
                        ]
                        .width(Length::Fill),
                    );
                }
                col.into()
            }
        }
    }

    /// Feedback box + «Далее» / «Результаты» (§8 «Feedback»).
    fn view_feedback<'a>(
        &'a self,
        ctx: &'a Ctx,
        run: &'a Run,
        it: &'a OwnItem,
    ) -> Element<'a, Msg> {
        let p = tp();
        let Some(r) = &run.answered else {
            return space().into();
        };
        let q = &it.q;
        let (bg, border, title_color) = if r.correct {
            (p.ok_soft, alpha(p.ok, 0.45), p.ok)
        } else if r.score > 0.0 {
            (p.gold_soft, alpha(p.gold, 0.45), p.gold2)
        } else {
            (p.bad_soft, alpha(p.bad, 0.45), p.bad)
        };
        let title = if r.correct {
            "Верно!"
        } else if r.score > 0.0 {
            "Частично"
        } else {
            "Неверно"
        };
        let mut box_col = Column::new().spacing(8).width(Length::Fill);
        box_col = box_col.push(
            txt(title)
                .font(SERIF_BOLD)
                .size(theme::text::H3)
                .color(title_color),
        );
        if !r.correct && !matches!(q, Question::Open { .. } | Question::Match { .. }) {
            box_col = box_col.push(rich_line(
                vec![
                    ("Правильный ответ: ".to_string(), false),
                    (exam::correct_text(q), true),
                ],
                theme::text::BODY,
                p.text,
            ));
        }
        if let Some(expl) = &q.common().explanation {
            box_col = box_col.push(widgets::rich_paragraph(expl));
        }
        if let Some(src) = &q.common().source {
            box_col = box_col.push(
                txt(format!("Источник: {src}"))
                    .size(theme::text::SMALL)
                    .color(p.muted),
            );
        }
        // differs notes: own movement expanded, the rest collapsible (§T.8).
        let differs = &q.common().differs;
        if !differs.is_empty() {
            let user = Library::user_level(&ctx.state);
            let mine: Vec<&Differs> = differs.iter().filter(|d| d.level == user).collect();
            let other: Vec<&Differs> = differs.iter().filter(|d| d.level != user).collect();
            for d in mine {
                box_col = box_col.push(widgets::differs_note(
                    crate::content::level_info(d.level).differs_label,
                    &d.text,
                ));
            }
            if !other.is_empty() {
                let label = if run.differs_open {
                    "Как в других течениях ▴"
                } else {
                    "Как в других течениях ▾"
                };
                box_col = box_col.push(
                    button(txt(label).size(theme::text::SMALL).color(p.muted))
                        .padding([8.0, 4.0])
                        .style(move |_, s| theme::button_ghost(&p, s))
                        .on_press(Msg::DiffersToggle),
                );
                if run.differs_open {
                    for d in other {
                        box_col = box_col.push(widgets::differs_note(
                            crate::content::level_info(d.level).differs_label,
                            &d.text,
                        ));
                    }
                }
            }
        }

        let last = run.idx + 1 >= run.items.len();
        let next_label = if last {
            "Результаты"
        } else {
            "Далее"
        };
        let next = widgets::btn_icon(
            kbd("Enter"),
            next_label,
            widgets::Btn::Primary,
            widgets::BtnSize::Lg,
            Some(Msg::Next),
        );

        column![
            tinted_box(box_col, bg, border),
            row![space::horizontal(), next].width(Length::Fill),
        ]
        .spacing(14)
        .width(Length::Fill)
        .into()
    }

    // -----------------------------------------------------------------------
    // Results view (§9)
    // -----------------------------------------------------------------------

    fn view_results<'a>(&'a self, ctx: &'a Ctx, r: &'a Results) -> Element<'a, Msg> {
        let p = tp();
        let res = &r.res;
        let is_section = res.mode == Mode::Section;
        let sec = res.section_id.as_deref().and_then(|id| ctx.lib.section(id));

        let practice = matches!(res.mode, Mode::SelfCheck | Mode::Rehearsal);
        let (back_label, back_msg) = match (practice, sec) {
            (true, Some(s)) => (s.title.as_str(), Msg::ToSection(s.id.clone())),
            _ => ("Экзамен", Msg::NewExam),
        };
        let mut col = Column::new().spacing(18).width(Length::Fill);
        col = col.push(widgets::back_button(back_label, back_msg));
        let mut kicker = exam::mode_title(res.mode).to_string();
        if let Some(s) = sec {
            kicker.push_str(" · ");
            kicker.push_str(&s.title);
        }
        col = col.push(gold_meta(kicker.to_uppercase()));
        col = col.push(widgets::heading("Результаты", 1));

        // Result card.
        let headline = if is_section {
            if res.passed {
                ("Тест сдан!", p.ok)
            } else {
                ("Пока не сдан", p.bad)
            }
        } else if res.pct >= 80 {
            ("Отличный результат!", p.ok)
        } else {
            ("Есть над чем поработать", p.bad)
        };
        let mut right_col = Column::new().spacing(8).width(Length::Fill);
        right_col = right_col.push(serif(headline.0, theme::text::H2).color(headline.1));
        // «15» / «14.5», never «15.0» (PWA drops the trailing zero).
        let s = (res.score * 10.0).round() / 10.0;
        let score_text = if s.fract() == 0.0 {
            format!("{}", s as i64)
        } else {
            format!("{s}")
        };
        let mut score_parts = vec![
            ("Набрано ".to_string(), false),
            (score_text, true),
            (" из ".to_string(), false),
            (format!("{}", res.total), true),
        ];
        if is_section {
            score_parts.push((" · порог 80%".to_string(), false));
        }
        right_col = right_col.push(rich_line(score_parts, theme::text::BODY, p.text));
        let today_xp = game::today_xp(&ctx.state, ctx.today());
        let goal = game::goal(&ctx.state);
        let xp_text = if res.xp > 0 {
            format!("+{} XP", res.xp)
        } else if is_section {
            "Без XP: тест не сдан — попробуйте ещё раз".to_string()
        } else {
            "Без XP: верных ответов пока нет".to_string()
        };
        right_col = right_col.push(
            txt(format!("{xp_text} · сегодня {today_xp} / {goal} XP"))
                .size(theme::text::BODY)
                .color(p.gold2),
        );
        if is_section && res.passed && sec.is_some() {
            right_col = right_col.push(
                muted("Если все уроки раздела прочитаны, следующий раздел открыт.")
                    .size(theme::text::BODY),
            );
        }
        if is_section && !res.passed {
            right_col = right_col.push(
                muted("Повторите уроки и карточки, затем попробуйте снова.")
                    .size(theme::text::BODY),
            );
        }
        let mut buttons = Row::new().spacing(10);
        if !r.mistake_ids.is_empty() {
            buttons = buttons.push(widgets::btn(
                "Повторить ошибки",
                widgets::Btn::Primary,
                widgets::BtnSize::Md,
                Some(Msg::Retry),
            ));
        }
        if !practice {
            buttons = buttons.push(widgets::btn(
                "Новый экзамен",
                widgets::Btn::Secondary,
                widgets::BtnSize::Md,
                Some(Msg::NewExam),
            ));
        }
        let ghost = match sec {
            Some(s) => widgets::btn(
                "К разделу",
                widgets::Btn::Ghost,
                widgets::BtnSize::Md,
                Some(Msg::ToSection(s.id.clone())),
            ),
            None => widgets::btn(
                "На карту",
                widgets::Btn::Ghost,
                widgets::BtnSize::Md,
                Some(Msg::ToMap),
            ),
        };
        buttons = buttons.push(ghost);
        right_col = right_col.push(buttons);

        let result_card = container(
            row![score_ring(res.pct, 140.0), right_col]
                .spacing(24)
                .align_y(Alignment::Center)
                .width(Length::Fill),
        )
        .padding(theme::spacing::CARD_PAD)
        .width(Length::Fill)
        .style(move |_| {
            let mut st = theme::card(&p);
            if is_section {
                st.border.color = if res.passed { p.ok } else { p.bad };
            }
            st
        });
        col = col.push(result_card);

        // Mistakes.
        if r.cards.is_empty() {
            col = col.push(
                container(
                    muted("Ошибок нет — прекрасно!")
                        .size(theme::text::BODY)
                        .align_x(iced::alignment::Horizontal::Center)
                        .width(Length::Fill),
                )
                .padding(12),
            );
        } else {
            col = col
                .push(serif(format!("Ошибки ({})", r.cards.len()), theme::text::H3).color(p.text));
            for card in &r.cards {
                let mut c = Column::new().spacing(8).width(Length::Fill);
                c = c.push(serif(card.prompt.as_str(), theme::text::BODY * 1.05).color(p.text));
                c = c.push(
                    txt(format!("Ваш ответ: {}", card.user))
                        .size(theme::text::BODY)
                        .color(p.bad),
                );
                let right_label = if card.open {
                    "Эталон"
                } else {
                    "Правильно"
                };
                c = c.push(
                    txt(format!("{right_label}: {}", card.right))
                        .size(theme::text::BODY)
                        .color(p.ok),
                );
                if let Some(expl) = &card.explanation {
                    c = c.push(muted(expl.as_str()).size(theme::text::BODY));
                }
                if let Some(src) = &card.source {
                    c = c.push(
                        txt(format!("Источник: {src}"))
                            .size(theme::text::SMALL)
                            .color(p.muted),
                    );
                }
                col = col.push(widgets::card(c));
            }
        }
        col.into()
    }
}
