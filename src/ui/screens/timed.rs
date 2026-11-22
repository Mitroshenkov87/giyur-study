//! «Маалот» — the ladder quiz (sidebar «Маалот», map CTA «Подняться»).
//!
//! The meaning of a TV quiz ladder in the app's own library look: fifteen
//! questions from easy to hard, safe steps 5 «Микра» and 10 «Мишна»
//! (Пиркей Авот 5:21), three one-time lifelines and the right to stop with
//! the steps climbed. The score is steps, never money. All rules live in
//! `crate::ladder`; this screen owns only view state.
//!
//! Three phases of one screen: the setup (`Route::Timed`), the climb
//! (`Route::TimedRun`; its `retry_ids` is ignored) and the results. The
//! ladder stands beside the question as a column of fifteen brass-railed
//! rungs when the content column is at least [`WIDE`] px wide, and as a
//! strip of small rungs above the question on narrower windows.

use std::time::{Duration, Instant};

use iced::gradient::Linear;
use iced::keyboard::{self, key};
use iced::widget::canvas::{self, Canvas, Frame, Path, Stroke};
use iced::widget::text::Shaping;
use iced::widget::{
    button, center, column, container, responsive, row, space, stack, text, Column, Row, Text,
};
use iced::{
    Alignment, Background, Border, Color, Element, Gradient, Length, Point, Radians, Rectangle,
    Renderer, Shadow, Subscription, Theme,
};
use rand::SeedableRng;

use crate::content::{Difficulty, Question};
use crate::game::{self, AwardOutcome};
use crate::ladder::{self, Help, Ladder, LadderResult, Lifeline, Outcome, Step};
use crate::ui::app::{Action, Ctx, Route, Toast};
use crate::ui::fonts::{HEBREW, SANS, SANS_BOLD, SERIF_BOLD};
use crate::ui::screens::map::{self, icons};
use crate::ui::theme::{self, Palette};
use crate::ui::widgets;

/// Content width from which the ladder column stands beside the question.
const WIDE: f32 = 760.0;
/// Width of the ladder column on wide windows.
const LADDER_W: f32 = 200.0;
/// Gap between the question and the ladder column.
const GAP: f32 = 20.0;
/// Height of one rung of the ladder column, and the gap between rungs.
const RUNG_H: f32 = 30.0;
const RUNG_GAP: f32 = 4.0;
/// Width of the ladder rails.
const RAIL_W: f32 = 4.0;
/// Inner question-card width below which labels get shorter.
const COMPACT: f32 = 520.0;
const LETTERS: [&str; 4] = ["A", "B", "C", "D"];

// ---------------------------------------------------------------------------
// View helpers (app palette)
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

/// Small gold uppercase meta line (as in the «Тесты» runner).
fn gold_meta<'a>(content: impl text::IntoFragment<'a>) -> Text<'a> {
    txt(content)
        .font(SANS_BOLD)
        .size(theme::text::KICKER)
        .color(tp().gold)
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

fn alpha(c: Color, a: f32) -> Color {
    Color { a, ..c }
}

fn v_gradient(top: Color, bottom: Color) -> Background {
    Background::Gradient(Gradient::Linear(
        Linear::new(Radians(std::f32::consts::FRAC_PI_2))
            .add_stop(0.0, top)
            .add_stop(1.0, bottom),
    ))
}

/// «kbd» key hint chip.
fn kbd<'a>(label: &'a str) -> Element<'a, Msg> {
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

/// Tinted box (feedback, lifeline results).
fn tinted_box<'a>(
    content: impl Into<Element<'a, Msg>>,
    bg: Color,
    border: Color,
) -> Element<'a, Msg> {
    container(content)
        .padding([14.0, 16.0])
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

fn divider<'a>() -> Element<'a, Msg> {
    container(space())
        .width(Length::Fill)
        .height(1)
        .style(|_| theme::divider_v(&tp()))
        .into()
}

fn tier_word(d: Option<Difficulty>) -> &'static str {
    match d {
        Some(Difficulty::Easy) => "лёгкий",
        Some(Difficulty::Hard) => "трудный",
        _ => "средний",
    }
}

/// Ring: track + arc for `frac` (0–1) from 12 o'clock, round caps.
struct Ring {
    frac: f32,
    color: Color,
    track: Color,
}

impl<Message> canvas::Program<Message> for Ring {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let size = bounds.size();
        let min = size.width.min(size.height);
        let stroke_width = (min * 0.075).max(3.0);
        let radius = (min - stroke_width) / 2.0;
        let center = Point::new(size.width / 2.0, size.height / 2.0);
        frame.stroke(
            &Path::circle(center, radius),
            Stroke {
                style: canvas::Style::Solid(self.track),
                width: stroke_width,
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
                    start_angle: Radians(start),
                    end_angle: Radians(end),
                });
            });
            frame.stroke(
                &arc,
                Stroke {
                    style: canvas::Style::Solid(self.color),
                    width: stroke_width,
                    line_cap: canvas::LineCap::Round,
                    ..Stroke::default()
                },
            );
        }
        vec![frame.into_geometry()]
    }
}

fn ring<'a>(frac: f32, color: Color, size: f32, label: Element<'a, Msg>) -> Element<'a, Msg> {
    let p = tp();
    stack![
        Canvas::new(Ring {
            frac,
            color,
            track: p.surface3,
        })
        .width(Length::Fixed(size))
        .height(Length::Fixed(size)),
        container(label).center(Length::Fixed(size)),
    ]
    .width(Length::Shrink)
    .into()
}

/// Countdown ring: brass arc, the bad colour in the last five seconds.
fn timer_ring<'a>(remaining_ms: i64, secs: i64, size: f32) -> Element<'a, Msg> {
    let p = tp();
    let left = ((remaining_ms.max(0) as f64) / 1000.0).ceil() as i64;
    let danger = left <= 5;
    let color = if danger { p.bad } else { p.gold };
    let frac = if secs > 0 {
        remaining_ms as f32 / (secs * 1000) as f32
    } else {
        1.0
    };
    ring(
        frac,
        color,
        size,
        txt(left.to_string())
            .size(size * 0.36)
            .font(SANS_BOLD)
            .color(if danger { p.bad } else { p.text })
            .into(),
    )
}

/// Badge / goal toasts after a finished run (mirrors `Ctx::award` effects;
/// the XP itself was already added by `ladder::finish`).
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

/// Confirm dialog: dimmed backdrop (click = cancel) + modal card. Rendered
/// as a shell layer above the whole window (see `screens::mod.rs`).
fn confirm_dialog<'a>(title: String, body: String, yes: String) -> Element<'a, Msg> {
    let p = tp();
    let dialog = container(
        column![
            txt(title)
                .font(SERIF_BOLD)
                .size(theme::text::H2)
                .color(p.text),
            txt(body).size(theme::text::BODY).color(p.text),
            row![
                space::horizontal(),
                widgets::btn(
                    "Продолжить",
                    widgets::Btn::Ghost,
                    widgets::BtnSize::Md,
                    Some(Msg::ConfirmNo),
                ),
                widgets::btn(
                    yes,
                    widgets::Btn::Primary,
                    widgets::BtnSize::Md,
                    Some(Msg::ConfirmYes),
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
    .on_press(Msg::ConfirmNo);
    stack![backdrop, center(dialog)]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

// ---------------------------------------------------------------------------
// The ladder graphic
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RungState {
    /// Climbed (and kept).
    Done,
    /// Climbed, but lost in the fall below the last safe step.
    Lost,
    /// The step being played.
    Current,
    /// The step answered wrong (or timed out).
    Fell,
    /// Not reached yet.
    Ahead,
}

/// Rung states (index 0 = step 1) during a run.
fn run_states(l: &Ladder, revealing: bool) -> Vec<RungState> {
    let fell = l.outcome == Some(Outcome::Fell);
    (1..=l.len())
        .map(|n| {
            if n <= l.climbed {
                RungState::Done
            } else if n == l.climbed + 1 && fell {
                RungState::Fell
            } else if n == l.climbed + 1 && !revealing && l.outcome.is_none() {
                RungState::Current
            } else {
                RungState::Ahead
            }
        })
        .collect()
}

/// Rung states for the results screen.
fn result_states(r: &LadderResult) -> Vec<RungState> {
    (1..=r.total)
        .map(|n| {
            if n <= r.kept {
                RungState::Done
            } else if n <= r.climbed {
                RungState::Lost
            } else if n == r.climbed + 1 && r.outcome == Outcome::Fell {
                RungState::Fell
            } else {
                RungState::Ahead
            }
        })
        .collect()
}

/// (text, accent) ink of a rung.
fn rung_ink(p: &Palette, st: RungState) -> (Color, Color) {
    match st {
        RungState::Current => (p.on_gold, p.on_gold),
        RungState::Done => (p.text, p.gold2),
        RungState::Lost => (p.muted, p.muted),
        RungState::Fell => (p.bad, p.bad),
        RungState::Ahead => (alpha(p.muted, 0.85), alpha(p.muted, 0.85)),
    }
}

/// Rung box: brass for the current step, a soft brass fill for climbed
/// ones, dim wood for the rest; safe steps carry a brass edge.
fn rung_style(p: &Palette, st: RungState, safe: bool, radius: f32) -> container::Style {
    let safe_edge = |c: Color| if safe { (p.gold, 1.5) } else { (c, 1.0) };
    let (bg, (border, width)) = match st {
        RungState::Current => (v_gradient(p.brass_hi, p.brass_lo), (p.brass_edge, 1.0)),
        RungState::Done => (
            Background::Color(alpha(p.gold, 0.18)),
            safe_edge(alpha(p.gold, 0.35)),
        ),
        RungState::Lost => (Background::Color(alpha(p.gold, 0.06)), safe_edge(p.line)),
        RungState::Fell => (Background::Color(p.bad_soft), (alpha(p.bad, 0.7), 1.0)),
        RungState::Ahead => (Background::Color(alpha(p.surface2, 0.6)), safe_edge(p.line)),
    };
    container::Style {
        background: Some(bg),
        border: Border {
            color: border,
            width,
            radius: radius.into(),
        },
        ..plain()
    }
}

fn rail<'a>(h: f32) -> Element<'a, Msg> {
    container(space())
        .width(RAIL_W)
        .height(h)
        .style(|_| {
            let p = tp();
            container::Style {
                background: Some(Background::Color(alpha(p.brass_lo, 0.85))),
                ..plain()
            }
        })
        .into()
}

/// The ladder column (wide windows): rungs 15 at the top to 1 at the
/// bottom between two brass rails, grouped by difficulty tier.
fn ladder_column<'a>(states: &[RungState]) -> Element<'a, Msg> {
    let p = tp();
    let mut col = Column::new().width(Length::Fill);
    let total = states.len();
    for n in (1..=total).rev() {
        let r = ladder::rung(n);
        // A tier label above the top rung of each tier.
        if n == total || ladder::tier_of(n) != ladder::tier_of(n + 1) {
            col = col.push(
                row![
                    rail(24.0),
                    container(muted(ladder::tier_title(r.tier)).size(11.5))
                        .width(Length::Fill)
                        .height(24.0)
                        .align_x(Alignment::Center)
                        .align_y(Alignment::Center),
                    rail(24.0),
                ]
                .width(Length::Fill),
            );
        }
        let st = states[n - 1];
        let (ink, accent) = rung_ink(&p, st);
        let marker: Element<'a, Msg> = if r.safe {
            icons::icon(icons::Kind::Shield, 14.0, accent)
        } else if st == RungState::Done {
            icons::icon(icons::Kind::Check, 13.0, accent)
        } else {
            space().width(14).into()
        };
        let bold = matches!(st, RungState::Current | RungState::Fell);
        let inner = row![
            container(txt(r.he).font(HEBREW).size(16.0).color(accent))
                .width(Length::Fixed(26.0))
                .align_x(Alignment::Center),
            container(txt(n.to_string()).size(13.5).font(SANS_BOLD).color(ink))
                .width(Length::Fixed(20.0)),
            txt(r.name)
                .size(13.0)
                .font(if bold { SANS_BOLD } else { SANS })
                .color(ink)
                .width(Length::Fill),
            marker,
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .padding([0.0, 8.0]);
        let rung = container(inner)
            .width(Length::Fill)
            .center_y(Length::Fixed(RUNG_H))
            .style(move |_| rung_style(&tp(), st, r.safe, 6.0));
        col = col.push(
            row![
                rail(RUNG_H + RUNG_GAP),
                container(rung)
                    .padding([RUNG_GAP / 2.0, 0.0])
                    .width(Length::Fill),
                rail(RUNG_H + RUNG_GAP),
            ]
            .width(Length::Fill),
        );
    }
    col = col.push(space().height(6));
    col = col.push(
        container(
            row![
                icons::icon(icons::Kind::Shield, 13.0, p.gold),
                muted("надёжная ступень").size(12.5),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .center_x(Length::Fill),
    );
    container(col)
        .padding([12.0, 12.0])
        .width(Length::Fixed(LADDER_W))
        .style(|_| theme::card(&tp()))
        .into()
}

/// The compact ladder (narrow windows, results): 1 on the left to 15 on
/// the right, in three tier groups, with a legend line under it.
fn ladder_strip<'a>(states: &[RungState]) -> Element<'a, Msg> {
    let p = tp();
    let mut groups = Row::new().spacing(8).width(Length::Fill);
    let mut group = Row::new().spacing(3).width(Length::Fill);
    let mut in_group = 0usize;
    for (i, &st) in states.iter().enumerate() {
        let n = i + 1;
        let r = ladder::rung(n);
        let (ink, accent) = rung_ink(&p, st);
        let cell = container(
            column![
                txt(r.he).font(HEBREW).size(12.5).color(accent),
                txt(n.to_string()).size(11.0).font(SANS_BOLD).color(ink),
            ]
            .spacing(0)
            .align_x(Alignment::Center),
        )
        .width(Length::FillPortion(1))
        .height(42.0)
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .style(move |_| rung_style(&tp(), st, r.safe, 6.0));
        group = group.push(cell);
        in_group += 1;
        let last = n == states.len();
        if last || ladder::tier_of(n) != ladder::tier_of(n + 1) {
            let done = std::mem::replace(&mut group, Row::new().spacing(3).width(Length::Fill));
            groups = groups.push(container(done).width(Length::FillPortion(in_group as u16)));
            in_group = 0;
        }
    }
    column![
        groups,
        row![
            icons::icon(icons::Kind::Shield, 13.0, p.gold),
            muted("Надёжные ступени — 5 «Микра» и 10 «Мишна»").size(12.5),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    ]
    .spacing(8)
    .width(Length::Fill)
    .into()
}

// ---------------------------------------------------------------------------
// Run state
// ---------------------------------------------------------------------------

/// The answered step, shown until «Следующая ступень» / «Итоги».
struct Reveal {
    step: Step,
    /// 1-based step number.
    no: usize,
    pick: Option<usize>,
    right: bool,
    /// Options hidden by 50/50 on that step.
    hidden: Vec<usize>,
}

struct Run {
    ladder: Ladder,
    /// Seconds per question; 0 = no timer.
    secs: i64,
    timer_end: Instant,
    remaining_ms: i64,
    reveal: Option<Reveal>,
    /// The walk-away (or leave) dialog is open; the clock stands still.
    confirm: bool,
    confirm_at: Option<Instant>,
    /// Screenshot hook: the clock does not run.
    frozen: bool,
}

impl Run {
    fn new(ladder: Ladder, secs: i64) -> Self {
        let mut run = Run {
            ladder,
            secs: secs.max(0),
            timer_end: Instant::now(),
            remaining_ms: 0,
            reveal: None,
            confirm: false,
            confirm_at: None,
            frozen: false,
        };
        run.start_clock();
        run
    }

    fn start_clock(&mut self) {
        self.remaining_ms = self.secs * 1000;
        self.timer_end = Instant::now() + Duration::from_millis(self.remaining_ms as u64);
    }

    fn playing(&self) -> bool {
        self.reveal.is_none() && !self.confirm && self.ladder.current().is_some()
    }

    fn can_pick(&self, k: usize) -> bool {
        self.playing()
            && !self.ladder.hidden.contains(&k)
            && self
                .ladder
                .current()
                .map(|s| k < s.opts.len())
                .unwrap_or(false)
    }

    /// Answer the current step (None = time out) and show the reveal.
    fn answer(&mut self, ctx: &mut Ctx, k: Option<usize>) {
        if self.reveal.is_some() {
            return;
        }
        let Some(step) = self.ladder.current().cloned() else {
            return;
        };
        let no = self.ladder.step_no();
        let hidden = self.ladder.hidden.clone();
        let today = ctx.today();
        let right = self.ladder.answer(&mut ctx.state, k, today);
        ctx.save();
        self.reveal = Some(Reveal {
            step,
            no,
            pick: k,
            right,
            hidden,
        });
    }

    /// From a right answer's reveal to the next step.
    fn advance(&mut self) {
        self.reveal = None;
        self.start_clock();
    }

    fn tick(&mut self, ctx: &mut Ctx, now: Instant) {
        if self.frozen || self.secs <= 0 || !self.playing() {
            return;
        }
        let left = self.timer_end.saturating_duration_since(now);
        self.remaining_ms = left.as_millis() as i64;
        if left.is_zero() {
            self.answer(ctx, None);
        }
    }
}

/// The failed question on the results screen.
struct Review {
    no: usize,
    prompt: String,
    /// None = the time ran out.
    yours: Option<String>,
    right: String,
    explanation: Option<String>,
    source: Option<String>,
}

struct Results {
    r: LadderResult,
    review: Option<Review>,
}

/// Finish a run (`ladder::finish` — called once per run: the caller
/// replaces the run phase with the results).
fn finish_run(ctx: &mut Ctx, run: &Run) -> Results {
    let today = ctx.today();
    let r = ladder::finish(&ctx.lib, &run.ladder, &mut ctx.state, today);
    celebrate_award(ctx, &r.award);
    ctx.save();
    let l = &run.ladder;
    let review = if r.outcome == Outcome::Fell {
        l.steps.get(l.climbed).map(|s| {
            let c = s.q.common();
            Review {
                no: l.climbed + 1,
                prompt: c.prompt.clone(),
                yours: l
                    .fell_pick
                    .and_then(|k| s.opts.get(k))
                    .map(|o| o.text.clone()),
                right: s.right_text().to_string(),
                explanation: c.explanation.clone().filter(|e| !e.trim().is_empty()),
                source: c.source.clone(),
            }
        })
    } else {
        None
    };
    Results { r, review }
}

// ---------------------------------------------------------------------------
// Screen
// ---------------------------------------------------------------------------

enum Phase {
    Setup,
    Run(Box<Run>),
    Results(Box<Results>),
}

pub struct Screen {
    phase: Phase,
}

#[derive(Debug, Clone)]
pub enum Msg {
    // Setup.
    Secs(i64),
    Start,
    // Run.
    Pick(usize),
    Help(Lifeline),
    WalkAway,
    ConfirmYes,
    ConfirmNo,
    Next,
    // Results.
    Again,
    ToMap,
    // Subscriptions.
    Tick(Instant),
    Key(keyboard::Key, key::Physical, keyboard::Modifiers),
}

/// Seconds per question for a new run (the `SHORASHIM_TEST_TIMER` hook wins).
fn run_seconds(ctx: &Ctx) -> i64 {
    ctx.test_timer
        .map(i64::from)
        .unwrap_or_else(|| ladder::timer_seconds(&ctx.state))
}

/// Screenshot hook `SHORASHIM_LADDER_DEMO`, read when the run screen opens.
/// The ladder is built with a seeded `StdRng` (seed 7), the clock is frozen,
/// and the answers are recorded like real ones (use a scratch
/// `SHORASHIM_DATA` file):
/// - `mid` (and any unknown value): the first 6 steps answered right, then
///   «50 на 50» and «Спросить раввина» used on step 7, which stays open;
/// - `reveal`: like `mid`, then step 7 answered right, the reveal stays open;
/// - `result`: 7 steps climbed, step 8 answered wrong → the results screen
///   with the review card;
/// - `top`: all 15 steps right → the results screen.
fn demo_phase(ctx: &mut Ctx, mode: &str) -> Phase {
    let mut rng = rand::rngs::StdRng::seed_from_u64(7);
    let Some(l) = ladder::build(&ctx.lib, &ctx.state, &mut rng) else {
        return Phase::Setup;
    };
    let mut run = Run::new(l, run_seconds(ctx));
    run.frozen = true;
    let climb = |run: &mut Run, ctx: &mut Ctx, n: usize| {
        for _ in 0..n {
            if let Some(k) = run.ladder.current().map(|s| s.right_index()) {
                run.answer(ctx, Some(k));
                run.advance();
            }
        }
    };
    match mode {
        "top" => {
            climb(&mut run, ctx, ladder::STEPS);
            Phase::Results(Box::new(finish_run(ctx, &run)))
        }
        "result" => {
            climb(&mut run, ctx, 7);
            let wrong = run
                .ladder
                .current()
                .and_then(|s| (0..s.opts.len()).find(|&i| !s.opts[i].right));
            run.answer(ctx, wrong);
            Phase::Results(Box::new(finish_run(ctx, &run)))
        }
        _ => {
            climb(&mut run, ctx, 6);
            run.ladder.use_lifeline(&ctx.lib, Lifeline::Fifty, &mut rng);
            run.ladder.use_lifeline(&ctx.lib, Lifeline::Rabbi, &mut rng);
            if mode == "reveal" {
                if let Some(k) = run.ladder.current().map(|s| s.right_index()) {
                    run.answer(ctx, Some(k));
                }
            }
            Phase::Run(Box::new(run))
        }
    }
}

/// Answer index for a key: physical A–D / 1–4 first (on a Cyrillic layout
/// the A key sends «ф»), then the character.
fn key_index(k: &keyboard::Key, physical: &key::Physical) -> Option<usize> {
    let by_code = match physical {
        key::Physical::Code(code) => match code {
            key::Code::KeyA | key::Code::Digit1 | key::Code::Numpad1 => Some(0),
            key::Code::KeyB | key::Code::Digit2 | key::Code::Numpad2 => Some(1),
            key::Code::KeyC | key::Code::Digit3 | key::Code::Numpad3 => Some(2),
            key::Code::KeyD | key::Code::Digit4 | key::Code::Numpad4 => Some(3),
            _ => None,
        },
        key::Physical::Unidentified(_) => None,
    };
    by_code.or_else(|| match k {
        keyboard::Key::Character(c) => match c.to_lowercase().as_str() {
            "a" | "1" => Some(0),
            "b" | "2" => Some(1),
            "c" | "3" => Some(2),
            "d" | "4" => Some(3),
            _ => None,
        },
        _ => None,
    })
}

impl Screen {
    pub fn new(ctx: &mut Ctx, route: &Route) -> Self {
        let phase = match route {
            Route::TimedRun { .. } => match crate::util::env_hook("LADDER_DEMO") {
                Some(mode) => demo_phase(ctx, mode.trim()),
                None => match ladder::build(&ctx.lib, &ctx.state, &mut rand::thread_rng()) {
                    Some(l) => Phase::Run(Box::new(Run::new(l, run_seconds(ctx)))),
                    // Nothing to ask: back to the setup with its notice.
                    None => Phase::Setup,
                },
            },
            _ => Phase::Setup,
        };
        Screen { phase }
    }

    /// Run → results (the only place `ladder::finish` is called in play).
    fn end(&mut self, ctx: &mut Ctx) {
        if let Phase::Run(run) = &self.phase {
            let res = finish_run(ctx, run);
            self.phase = Phase::Results(Box::new(res));
        }
    }

    pub fn update(&mut self, ctx: &mut Ctx, msg: Msg) -> Action {
        match msg {
            Msg::Secs(v) => {
                ctx.state.settings.timed_seconds = ladder::sanitize_seconds(v);
                ctx.save();
            }
            Msg::Start | Msg::Again => return Action::Go(Route::TimedRun { retry_ids: None }),
            Msg::ToMap => return Action::Go(Route::Map),
            Msg::Pick(k) => {
                if let Phase::Run(run) = &mut self.phase {
                    if run.can_pick(k) {
                        run.answer(ctx, Some(k));
                    }
                }
            }
            Msg::Help(l) => {
                if let Phase::Run(run) = &mut self.phase {
                    if run.playing() {
                        run.ladder
                            .use_lifeline(&ctx.lib, l, &mut rand::thread_rng());
                    }
                }
            }
            Msg::WalkAway => {
                if let Phase::Run(run) = &mut self.phase {
                    if run.ladder.outcome.is_none() && !run.confirm {
                        run.confirm = true;
                        run.confirm_at = Some(Instant::now());
                    }
                }
            }
            Msg::ConfirmNo => {
                if let Phase::Run(run) = &mut self.phase {
                    run.confirm = false;
                    // Give back the time spent in the dialog.
                    if let Some(at) = run.confirm_at.take() {
                        run.timer_end += at.elapsed();
                    }
                }
            }
            Msg::ConfirmYes => {
                let walk = match &mut self.phase {
                    Phase::Run(run) if run.confirm => {
                        run.confirm = false;
                        run.confirm_at = None;
                        if run.ladder.climbed == 0 {
                            // No steps yet: leave without a result.
                            return Action::Go(Route::Timed);
                        }
                        run.ladder.walk_away();
                        true
                    }
                    _ => false,
                };
                if walk {
                    self.end(ctx);
                }
            }
            Msg::Next => {
                let finished = match &mut self.phase {
                    Phase::Run(run) if run.reveal.is_some() && !run.confirm => {
                        if run.ladder.outcome.is_some() {
                            true
                        } else {
                            run.advance();
                            false
                        }
                    }
                    _ => false,
                };
                if finished {
                    self.end(ctx);
                }
            }
            Msg::Tick(now) => {
                if let Phase::Run(run) = &mut self.phase {
                    run.tick(ctx, now);
                }
            }
            Msg::Key(k, physical, modifiers) => {
                if modifiers.command() || modifiers.alt() {
                    return Action::None;
                }
                let Phase::Run(run) = &self.phase else {
                    return Action::None;
                };
                let named = |n: key::Named| matches!(&k, keyboard::Key::Named(x) if *x == n);
                if run.confirm {
                    if named(key::Named::Escape) {
                        return self.update(ctx, Msg::ConfirmNo);
                    }
                } else if run.reveal.is_some() {
                    if named(key::Named::Enter) || named(key::Named::Space) {
                        return self.update(ctx, Msg::Next);
                    }
                } else if let Some(i) = key_index(&k, &physical) {
                    return self.update(ctx, Msg::Pick(i));
                }
            }
        }
        Action::None
    }

    pub fn view<'a>(&'a self, ctx: &'a Ctx) -> Element<'a, Msg> {
        responsive(move |size| self.view_at(ctx, size.width))
            .height(Length::Shrink)
            .into()
    }

    /// The page for a content width `w`.
    fn view_at<'a>(&'a self, ctx: &'a Ctx, w: f32) -> Element<'a, Msg> {
        match &self.phase {
            Phase::Setup => view_setup(ctx, w),
            Phase::Run(run) => view_run(run, w),
            Phase::Results(r) => view_results(ctx, r, w),
        }
    }

    /// The walk-away / leave dialog, rendered as a shell layer above the window.
    pub fn dialog<'a>(&'a self, _ctx: &'a Ctx) -> Option<Element<'a, Msg>> {
        match &self.phase {
            Phase::Run(run) if run.confirm => {
                let n = run.ladder.climbed;
                Some(if n == 0 {
                    confirm_dialog(
                        "Выйти из игры?".to_string(),
                        "Ступеней пока нет, поэтому игра не будет засчитана. \
                         Данные ответы уже сохранены в статистике."
                            .to_string(),
                        "Выйти".to_string(),
                    )
                } else {
                    confirm_dialog(
                        format!("Забрать {n} {}?", ladder::steps_word(n)),
                        "Игра на этом закончится, а пройденные ступени останутся за вами."
                            .to_string(),
                        "Забрать".to_string(),
                    )
                })
            }
            _ => None,
        }
    }

    pub fn subscription(&self) -> Subscription<Msg> {
        match &self.phase {
            Phase::Run(run) => {
                let keys = keyboard::listen().filter_map(|ev| match ev {
                    keyboard::Event::KeyPressed {
                        key,
                        physical_key,
                        modifiers,
                        ..
                    } => Some(Msg::Key(key, physical_key, modifiers)),
                    _ => None,
                });
                if run.secs > 0 && !run.frozen && run.reveal.is_none() {
                    Subscription::batch([
                        iced::time::every(Duration::from_millis(100)).map(Msg::Tick),
                        keys,
                    ])
                } else {
                    keys
                }
            }
            _ => Subscription::none(),
        }
    }
}

// ---------------------------------------------------------------------------
// Setup view
// ---------------------------------------------------------------------------

fn view_setup<'a>(ctx: &'a Ctx, w: f32) -> Element<'a, Msg> {
    let p = tp();
    let mut col = Column::new().spacing(18).width(Length::Fill);
    col = col.push(widgets::kicker("Лестница знаний"));
    col = col.push(
        row![
            widgets::heading("Маалот", 1),
            txt("מַעֲלוֹת")
                .font(HEBREW)
                .size(theme::text::H1 * 0.9)
                .color(p.hebrew),
        ]
        .spacing(14)
        .align_y(Alignment::Center),
    );
    col = col.push(
        column![
            muted(
                "Пятнадцать ступеней — от лёгких вопросов к трудным. Ступени 5 «Микра» \
                 и 10 «Мишна» надёжные: после ошибки вы спускаетесь к последней \
                 надёжной ступени, которую прошли.",
            )
            .size(theme::text::BODY),
            muted(
                "Три подсказки — «50 на 50», «Подсказка из урока» и «Спросить раввина» — \
                 по одному разу за игру. Остановиться можно в любой момент: пройденные \
                 ступени останутся за вами.",
            )
            .size(theme::text::BODY),
        ]
        .spacing(10)
        .width(Length::Fill),
    );
    col = col.push(widgets::quote_block(
        "В пять лет — к Микре, в десять — к Мишне, в тринадцать — к мицвот, … \
         в пятнадцать — к Гмаре",
        Some("Пиркей Авот 5:21"),
    ));

    let counts = ladder::pool_counts(&ctx.lib, &ctx.state);
    let total: usize = counts.iter().map(|(_, mc, tf)| mc + tf).sum();
    if total == 0 {
        col = col.push(map::notice(
            icons::Kind::Book,
            "Для лестницы пока нет вопросов. Откройте первый раздел и прочитайте \
             его уроки — тогда здесь можно будет начать восхождение.",
            p,
        ));
        return col.into();
    }

    let mut card = Column::new().spacing(16).width(Length::Fill);

    // Available questions per tier.
    let mut tiles = Row::new().spacing(10).width(Length::Fill);
    for (d, mc, tf) in counts {
        let (title, steps) = match d {
            Difficulty::Easy => ("Лёгкие", "ступени 1–5"),
            Difficulty::Medium => ("Средние", "ступени 6–10"),
            Difficulty::Hard => ("Трудные", "ступени 11–15"),
        };
        tiles = tiles.push(
            container(
                column![
                    txt((mc + tf).to_string())
                        .size(theme::text::BODY * 1.6)
                        .font(SERIF_BOLD)
                        .color(p.gold2),
                    txt(title)
                        .size(theme::text::SMALL)
                        .font(SANS_BOLD)
                        .color(p.text),
                    muted(steps).size(12.5),
                ]
                .spacing(2)
                .align_x(Alignment::Center),
            )
            .padding([12.0, 8.0])
            .width(Length::FillPortion(1))
            .align_x(Alignment::Center)
            .style(move |_| container::Style {
                background: Some(Background::Color(alpha(p.surface2, 0.7))),
                border: Border {
                    color: p.line,
                    width: 1.0,
                    radius: theme::radius::SMALL.into(),
                },
                ..plain()
            }),
        );
    }
    card = card.push(
        column![
            txt("Вопросы для лестницы")
                .size(theme::text::BODY)
                .color(p.text),
            muted("Из открытых разделов, по вашему уровню").size(theme::text::SMALL),
        ]
        .spacing(2),
    );
    card = card.push(tiles);
    card = card.push(divider());

    // Timer.
    let secs = ladder::timer_seconds(&ctx.state);
    let seg = widgets::segmented(
        ladder::TIMER_CHOICES
            .iter()
            .map(|&(v, label)| (label.to_string(), Msg::Secs(v)))
            .collect(),
        ladder::TIMER_CHOICES.iter().position(|&(v, _)| v == secs),
    );
    card = card.push(
        column![
            row![
                icons::icon(icons::Kind::Timer, 18.0, p.gold),
                txt("Таймер на вопрос")
                    .size(theme::text::BODY)
                    .color(p.text),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            muted("Если время выйдет, ответ считается неверным.").size(theme::text::SMALL),
            container(seg).max_width(420),
        ]
        .spacing(8)
        .width(Length::Fill),
    );
    card = card.push(divider());

    // Record + start.
    let best = ladder::best(&ctx.state);
    let record: Element<'a, Msg> = if best > 0 {
        row![
            icons::icon(icons::Kind::Star, 18.0, p.gold),
            txt(format!("Рекорд: {best} из {}", ladder::STEPS))
                .size(theme::text::BODY)
                .font(SANS_BOLD)
                .color(p.gold2),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .into()
    } else {
        muted("Рекорда пока нет").size(theme::text::BODY).into()
    };
    let start = widgets::btn_icon(
        icons::icon(icons::Kind::Ladder, 18.0, p.on_gold),
        "Начать восхождение",
        widgets::Btn::Primary,
        widgets::BtnSize::Lg,
        Some(Msg::Start),
    );
    if w >= 560.0 {
        card = card.push(
            row![container(record).width(Length::Fill), start]
                .spacing(16)
                .align_y(Alignment::Center)
                .width(Length::Fill),
        );
    } else {
        card = card.push(column![record, start].spacing(14).width(Length::Fill));
    }
    col = col.push(widgets::card(card));
    col.into()
}

// ---------------------------------------------------------------------------
// Run view
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OptLook {
    Normal,
    Hidden,
    Right,
    Wrong,
    Dim,
}

fn option_button<'a>(
    k: usize,
    label: &'a str,
    look: OptLook,
    on_press: Option<Msg>,
    fill_portion: bool,
) -> Element<'a, Msg> {
    let p = tp();
    let ink = match look {
        OptLook::Hidden => alpha(p.text, 0.28),
        OptLook::Dim => alpha(p.text, 0.6),
        _ => p.text,
    };
    let (disc_bg, disc_border, letter_ink) = match look {
        OptLook::Right => (p.ok_soft, p.ok, p.ok),
        OptLook::Wrong => (p.bad_soft, p.bad, p.bad),
        OptLook::Hidden => (Color::TRANSPARENT, alpha(p.line2, 0.4), alpha(p.gold2, 0.3)),
        OptLook::Dim => (p.surface3, p.line, alpha(p.gold2, 0.6)),
        OptLook::Normal => (p.surface3, p.line2, p.gold2),
    };
    let letter = LETTERS.get(k).copied().unwrap_or("?");
    let content = row![
        widgets::disc(
            txt(letter)
                .size(14.0)
                .font(SANS_BOLD)
                .color(letter_ink)
                .into(),
            30.0,
            disc_bg,
            disc_border,
        ),
        txt(label)
            .size(theme::text::BODY)
            .color(ink)
            .width(Length::Fill),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .width(Length::Fill);
    button(content)
        .width(if fill_portion {
            Length::FillPortion(1)
        } else {
            Length::Fill
        })
        .padding([12.0, 14.0])
        .style(move |_, s| {
            let p = tp();
            match look {
                OptLook::Normal => theme::option_normal(&p, s),
                OptLook::Right => theme::option_correct(&p, s),
                OptLook::Wrong => theme::option_wrong(&p, s),
                OptLook::Dim => theme::option_normal(&p, button::Status::Active),
                OptLook::Hidden => button::Style {
                    background: None,
                    text_color: alpha(p.text, 0.28),
                    border: Border {
                        color: alpha(p.line2, 0.35),
                        width: 1.0,
                        radius: theme::radius::BUTTON.into(),
                    },
                    shadow: Shadow::default(),
                    snap: true,
                },
            }
        })
        .on_press_maybe(on_press)
        .into()
}

fn options<'a>(
    step: &'a Step,
    hidden: &[usize],
    reveal: Option<(Option<usize>, bool)>,
    active: bool,
) -> Element<'a, Msg> {
    let tf = matches!(step.q, Question::Tf { .. });
    let mut items: Vec<Element<'a, Msg>> = Vec::new();
    for (k, opt) in step.opts.iter().enumerate() {
        let is_hidden = hidden.contains(&k);
        let look = match reveal {
            Some((pick, _)) => {
                if opt.right {
                    OptLook::Right
                } else if pick == Some(k) {
                    OptLook::Wrong
                } else if is_hidden {
                    OptLook::Hidden
                } else {
                    OptLook::Dim
                }
            }
            None if is_hidden => OptLook::Hidden,
            None => OptLook::Normal,
        };
        let on = (active && look == OptLook::Normal).then_some(Msg::Pick(k));
        items.push(option_button(k, opt.text.as_str(), look, on, tf));
    }
    if tf {
        Row::with_children(items)
            .spacing(10)
            .width(Length::Fill)
            .into()
    } else {
        Column::with_children(items)
            .spacing(10)
            .width(Length::Fill)
            .into()
    }
}

fn lifeline_button<'a>(l: Lifeline, usable: bool, short: bool) -> Element<'a, Msg> {
    let p = tp();
    let kind = match l {
        Lifeline::Fifty => icons::Kind::Half,
        Lifeline::Lesson => icons::Kind::Book,
        Lifeline::Rabbi => icons::Kind::Speech,
    };
    let label = if short {
        match l {
            Lifeline::Fifty => "50/50",
            Lifeline::Lesson => "Урок",
            Lifeline::Rabbi => "Раввин",
        }
    } else {
        l.title()
    };
    let color = if usable { p.gold2 } else { alpha(p.muted, 0.5) };
    button(
        container(
            row![icons::icon(kind, 18.0, color), txt(label).size(15.0)]
                .spacing(8)
                .align_y(Alignment::Center),
        )
        .center(Length::Fill),
    )
    .width(Length::FillPortion(1))
    .height(40.0)
    .padding([0.0, 8.0])
    .style(|_, s| theme::button_secondary(&tp(), s))
    .on_press_maybe(usable.then_some(Msg::Help(l)))
    .into()
}

/// A lifeline result panel: icon + title, then the body.
fn help_panel<'a>(
    kind: icons::Kind,
    title: &'a str,
    body: impl Into<Element<'a, Msg>>,
) -> Element<'a, Msg> {
    let p = tp();
    tinted_box(
        column![
            row![
                icons::icon(kind, 16.0, p.gold2),
                txt(title)
                    .size(theme::text::SMALL)
                    .font(SANS_BOLD)
                    .color(p.gold2),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            body.into(),
        ]
        .spacing(8)
        .width(Length::Fill),
        p.gold_soft,
        alpha(p.gold, 0.3),
    )
}

fn help_view<'a>(l: Lifeline, h: &'a Help) -> Element<'a, Msg> {
    let p = tp();
    match h {
        Help::Fifty(_) => help_panel(
            icons::Kind::Half,
            l.title(),
            muted("Два неверных варианта убраны.").size(theme::text::SMALL),
        ),
        Help::Lesson {
            section,
            lesson,
            source,
            idea,
        } => {
            let mut c = Column::new().spacing(8).width(Length::Fill);
            let path: Vec<&str> = [section, lesson]
                .into_iter()
                .flatten()
                .map(|s| s.as_str())
                .collect();
            if !path.is_empty() {
                c = c.push(
                    txt(path.join(" › "))
                        .size(theme::text::BODY)
                        .font(SANS_BOLD)
                        .color(p.text),
                );
            }
            match idea {
                Some(i) => c = c.push(widgets::quote_block(i.as_str(), None)),
                None => {
                    c = c.push(
                        muted("Ответ — в этом уроке: вспомните его главные мысли.")
                            .size(theme::text::SMALL),
                    )
                }
            }
            if let Some(s) = source {
                c = c.push(muted(format!("Источник: {s}")).size(theme::text::SMALL));
            }
            help_panel(icons::Kind::Book, l.title(), c)
        }
        Help::Rabbi(t) => help_panel(
            icons::Kind::Speech,
            l.title(),
            txt(format!("{}{t}", ladder::RABBI_LEAD))
                .size(theme::text::BODY)
                .color(p.text)
                .width(Length::Fill),
        ),
    }
}

/// Verdict box after an answer.
fn reveal_view<'a>(run: &'a Run, rv: &'a Reveal) -> Element<'a, Msg> {
    let p = tp();
    let c = rv.step.q.common();
    let (title, color, bg) = if rv.right {
        ("Верно!", p.ok, p.ok_soft)
    } else if rv.pick.is_none() {
        ("Время вышло", p.bad, p.bad_soft)
    } else {
        ("Неверно", p.bad, p.bad_soft)
    };
    let mut col = Column::new().spacing(8).width(Length::Fill);
    col = col.push(
        txt(title)
            .font(SERIF_BOLD)
            .size(theme::text::H3)
            .color(color),
    );
    if !rv.right {
        col = col.push(
            iced::widget::rich_text::<(), Msg, Theme, Renderer>(vec![
                iced::widget::text::Span::new("Правильный ответ: ")
                    .font(SANS)
                    .color(p.text),
                iced::widget::text::Span::new(rv.step.right_text())
                    .font(SANS_BOLD)
                    .color(p.text),
            ])
            .size(theme::text::BODY)
            .width(Length::Fill),
        );
    }
    if let Some(e) = c.explanation.as_deref().filter(|e| !e.trim().is_empty()) {
        col = col.push(widgets::rich_paragraph(e));
    }
    if let Some(s) = &c.source {
        col = col.push(muted(format!("Источник: {s}")).size(theme::text::SMALL));
    }
    // What the answer means on the ladder.
    let note: Option<(icons::Kind, String, Color)> = if rv.right {
        if run.ladder.outcome == Some(Outcome::Top) {
            Some((
                icons::Kind::Star,
                format!("Вершина: все {} ступеней пройдены.", run.ladder.len()),
                p.gold2,
            ))
        } else if ladder::SAFE.contains(&rv.no) {
            Some((
                icons::Kind::Shield,
                format!(
                    "Надёжная ступень: {} {} уже ваши.",
                    rv.no,
                    ladder::steps_word(rv.no)
                ),
                p.gold2,
            ))
        } else {
            None
        }
    } else {
        let kept = run.ladder.kept();
        Some((
            icons::Kind::Shield,
            if kept > 0 {
                format!(
                    "Вы спускаетесь к надёжной ступени {kept}: {kept} {} остаются за вами.",
                    ladder::steps_word(kept)
                )
            } else {
                "Надёжная ступень ещё не пройдена, поэтому в этот раз ступеней нет.".to_string()
            },
            p.muted,
        ))
    };
    if let Some((kind, line, ink)) = note {
        col = col.push(
            row![
                icons::icon(kind, 16.0, ink),
                txt(line)
                    .size(theme::text::BODY)
                    .font(SANS_BOLD)
                    .color(ink)
                    .width(Length::Fill),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        );
    }
    tinted_box(col, bg, alpha(color, 0.45))
}

fn view_run<'a>(run: &'a Run, w: f32) -> Element<'a, Msg> {
    let p = tp();
    let l = &run.ladder;
    let wide = w >= WIDE;
    let states = run_states(l, run.reveal.is_some());
    let card_w = (if wide { w - LADDER_W - GAP } else { w }) - 2.0 * theme::spacing::CARD_PAD;
    let compact = card_w < COMPACT;

    // Top bar: kicker, what is already safe, the timer.
    let safe = ladder::safe_below(l.climbed);
    let mut top = Row::new()
        .spacing(12)
        .align_y(Alignment::Center)
        .width(Length::Fill);
    top = top.push(widgets::kicker(if wide {
        "Маалот · лестница знаний"
    } else {
        "Маалот"
    }));
    top = top.push(space::horizontal());
    if safe > 0 {
        top = top.push(widgets::pill(
            format!("Надёжно: {safe} {}", ladder::steps_word(safe)),
            widgets::Pill::Chip,
        ));
    }
    if run.secs > 0 {
        top = top.push(timer_ring(run.remaining_ms, run.secs, 46.0));
    }

    // The step being shown: the answered one during the reveal.
    let (step, no, hidden, reveal): (&Step, usize, &[usize], Option<(Option<usize>, bool)>) =
        match (&run.reveal, l.current()) {
            (Some(rv), _) => (&rv.step, rv.no, &rv.hidden, Some((rv.pick, rv.right))),
            (None, Some(s)) => (s, l.step_no(), &l.hidden, None),
            (None, None) => return top.into(),
        };
    let c = step.q.common();
    let r = ladder::rung(no);

    let mut card = Column::new().spacing(14).width(Length::Fill);
    let mut meta = format!("Ступень {no} из {}", l.len());
    if !r.name.is_empty() {
        meta.push_str(&format!(" · «{}»", r.name));
    }
    meta.push_str(" · ");
    meta.push_str(tier_word(c.difficulty));
    if matches!(step.q, Question::Tf { .. }) {
        meta.push_str(" · верно или неверно?");
    }
    card = card.push(
        row![
            txt(r.he).font(HEBREW).size(20.0).color(p.hebrew),
            gold_meta(meta.to_uppercase()),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    );
    if let Some(t) = &step.section_title {
        card = card.push(muted(t.as_str()).size(theme::text::SMALL));
    }
    card = card.push(
        txt(c.prompt.as_str())
            .font(SERIF_BOLD)
            .size(if compact { 21.0 } else { theme::text::H2 })
            .color(p.text),
    );
    card = card.push(options(step, hidden, reveal, run.playing()));

    match &run.reveal {
        None => {
            let mut lifelines = Row::new().spacing(8).width(Length::Fill);
            for line in Lifeline::ALL {
                lifelines = lifelines.push(lifeline_button(
                    line,
                    l.can_use(line) && !run.confirm,
                    compact,
                ));
            }
            card = card.push(lifelines);
            for (line, h) in &l.shown {
                card = card.push(help_view(*line, h));
            }
        }
        Some(rv) => card = card.push(reveal_view(run, rv)),
    }

    // Footer: walk away on the left, the next step (or the key hint) on the
    // right; on a narrow card the next step goes on its own line above.
    card = card.push(divider());
    let walk: Option<Element<'a, Msg>> = l.outcome.is_none().then(|| {
        let label = if l.climbed == 0 {
            "Выйти".to_string()
        } else {
            format!("Забрать {} {}", l.climbed, ladder::steps_word(l.climbed))
        };
        widgets::btn(
            label,
            widgets::Btn::Ghost,
            widgets::BtnSize::Sm,
            (!run.confirm).then_some(Msg::WalkAway),
        )
        .into()
    });
    let trailing: Option<Element<'a, Msg>> = match &run.reveal {
        None if compact => None,
        None => Some(
            row![
                muted("Ответ — клавиши").size(theme::text::SMALL),
                kbd("A"),
                muted("–").size(theme::text::SMALL),
                kbd("D"),
                muted("или").size(theme::text::SMALL),
                kbd("1"),
                muted("–").size(theme::text::SMALL),
                kbd("4"),
            ]
            .spacing(5)
            .align_y(Alignment::Center)
            .into(),
        ),
        Some(_) => {
            let label = if l.outcome.is_some() {
                "Итоги"
            } else {
                "Следующая ступень"
            };
            Some(if compact {
                widgets::btn(
                    label,
                    widgets::Btn::Primary,
                    widgets::BtnSize::Md,
                    Some(Msg::Next),
                )
                .into()
            } else {
                widgets::btn_icon(
                    kbd("Enter"),
                    label,
                    widgets::Btn::Primary,
                    widgets::BtnSize::Md,
                    Some(Msg::Next),
                )
                .into()
            })
        }
    };
    if compact {
        if let Some(t) = trailing {
            card = card.push(container(t).align_right(Length::Fill));
        }
        if let Some(w) = walk {
            card = card.push(w);
        }
    } else {
        let mut footer = Row::new()
            .spacing(12)
            .align_y(Alignment::Center)
            .width(Length::Fill);
        if let Some(w) = walk {
            footer = footer.push(w);
        }
        footer = footer.push(space::horizontal());
        if let Some(t) = trailing {
            footer = footer.push(t);
        }
        card = card.push(footer);
    }
    let card = widgets::card(card);

    let body: Element<'a, Msg> = if wide {
        row![container(card).width(Length::Fill), ladder_column(&states)]
            .spacing(GAP)
            .align_y(Alignment::Start)
            .width(Length::Fill)
            .into()
    } else {
        column![ladder_strip(&states), card]
            .spacing(14)
            .width(Length::Fill)
            .into()
    };
    column![top, body].spacing(16).width(Length::Fill).into()
}

// ---------------------------------------------------------------------------
// Results view
// ---------------------------------------------------------------------------

fn view_results<'a>(ctx: &'a Ctx, res: &'a Results, w: f32) -> Element<'a, Msg> {
    let p = tp();
    let r = &res.r;
    let (title, sub) = ladder::result_title(r);
    let mut col = Column::new().spacing(18).width(Length::Fill);
    col = col.push(widgets::kicker("Маалот · итоги"));
    col = col.push(widgets::heading(title, 1));

    let frac = if r.total > 0 {
        r.kept as f32 / r.total as f32
    } else {
        0.0
    };
    let ring_el = ring(
        frac,
        p.gold,
        136.0,
        column![
            txt(r.kept.to_string())
                .size(44.0)
                .font(SERIF_BOLD)
                .color(p.gold2),
            muted(format!("из {}", r.total)).size(theme::text::SMALL),
        ]
        .spacing(0)
        .align_x(Alignment::Center)
        .into(),
    );

    let mut info = Column::new().spacing(8).width(Length::Fill);
    info = info.push(muted(sub).size(theme::text::BODY));
    if r.outcome == Outcome::Fell && r.climbed > r.kept {
        info = info.push(
            txt(format!(
                "Вы поднялись до ступени {}; за вами остаются {} {}.",
                r.climbed,
                r.kept,
                ladder::steps_word(r.kept)
            ))
            .size(theme::text::BODY)
            .color(p.text),
        );
    }
    let record: Element<'a, Msg> = if r.new_best {
        row![
            icons::icon(icons::Kind::Star, 18.0, p.gold2),
            txt("Новый рекорд!")
                .size(theme::text::BODY)
                .font(SANS_BOLD)
                .color(p.gold2),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .into()
    } else if r.best_before > 0 {
        txt(format!("Рекорд: {} из {}", r.best_before, ladder::STEPS))
            .size(theme::text::BODY)
            .color(p.text)
            .into()
    } else {
        muted("Рекорда пока нет").size(theme::text::BODY).into()
    };
    info = info.push(record);
    let today_xp = game::today_xp(&ctx.state, ctx.today());
    let goal = game::goal(&ctx.state);
    let xp = if r.xp > 0 {
        format!("+{} XP · сегодня {today_xp} / {goal} XP", r.xp)
    } else {
        format!("Без XP: ступеней нет · сегодня {today_xp} / {goal} XP")
    };
    info = info.push(txt(xp).size(theme::text::BODY).color(p.gold2));
    info = info.push(
        muted(format!(
            "Подсказок использовано: {} из {}",
            r.lifelines_used,
            Lifeline::ALL.len()
        ))
        .size(theme::text::SMALL),
    );
    info = info.push(
        row![
            widgets::btn(
                "Ещё раз",
                widgets::Btn::Primary,
                widgets::BtnSize::Md,
                Some(Msg::Again),
            ),
            widgets::btn(
                "На карту",
                widgets::Btn::Ghost,
                widgets::BtnSize::Md,
                Some(Msg::ToMap),
            ),
        ]
        .spacing(10),
    );

    let head: Element<'a, Msg> = if w >= 560.0 {
        row![ring_el, info]
            .spacing(24)
            .align_y(Alignment::Center)
            .width(Length::Fill)
            .into()
    } else {
        column![widgets::centered(ring_el), info]
            .spacing(16)
            .width(Length::Fill)
            .into()
    };
    col = col.push(widgets::card(
        column![head, divider(), ladder_strip(&result_states(r))]
            .spacing(16)
            .width(Length::Fill),
    ));

    if let Some(rv) = &res.review {
        let mut c = Column::new().spacing(8).width(Length::Fill);
        c = c.push(gold_meta(format!("РАЗБОР · СТУПЕНЬ {}", rv.no)));
        c = c.push(
            txt(rv.prompt.as_str())
                .font(SERIF_BOLD)
                .size(theme::text::H3)
                .color(p.text),
        );
        c = c.push(
            txt(match &rv.yours {
                Some(y) => format!("Ваш ответ: {y}"),
                None => "Ваш ответ: время вышло".to_string(),
            })
            .size(theme::text::BODY)
            .color(p.bad),
        );
        c = c.push(
            txt(format!("Правильно: {}", rv.right))
                .size(theme::text::BODY)
                .color(p.ok),
        );
        if let Some(e) = &rv.explanation {
            c = c.push(widgets::rich_paragraph(e.as_str()));
        }
        if let Some(s) = &rv.source {
            c = c.push(muted(format!("Источник: {s}")).size(theme::text::SMALL));
        }
        col = col.push(widgets::card(c));
    }
    col.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::Library;
    use crate::storage::State;

    fn ctx() -> Ctx {
        let mut state = State::default();
        state.settings.unlock_all = true;
        Ctx {
            lib: Library::embedded(),
            state,
            path: std::env::temp_dir().join("shorashim-ladder-ui-test.json"),
            toasts: Vec::new(),
            today_override: chrono::NaiveDate::from_ymd_opt(2026, 11, 22),
            test_timer: None,
            writable: false,
        }
    }

    fn run_screen(ctx: &mut Ctx) -> Screen {
        let mut rng = rand::rngs::StdRng::seed_from_u64(7);
        let l = ladder::build(&ctx.lib, &ctx.state, &mut rng).unwrap();
        Screen {
            phase: Phase::Run(Box::new(Run::new(l, 30))),
        }
    }

    fn run(s: &Screen) -> &Run {
        match &s.phase {
            Phase::Run(r) => r,
            _ => panic!("not running"),
        }
    }

    fn views(s: &Screen, ctx: &Ctx) {
        for w in [425.0, 600.0, 820.0] {
            let _ = s.view_at(ctx, w);
            let _ = s.dialog(ctx);
        }
    }

    fn right(s: &Screen) -> usize {
        run(s).ladder.current().unwrap().right_index()
    }

    #[test]
    fn setup_builds_with_and_without_a_record() {
        let mut c = ctx();
        let s = Screen {
            phase: Phase::Setup,
        };
        views(&s, &c);
        c.state.counters.insert(ladder::BEST_KEY.into(), 9);
        c.state.settings.timed_seconds = 0;
        views(&s, &c);
    }

    #[test]
    fn climb_with_lifelines_fall_and_results() {
        let mut c = ctx();
        let mut s = run_screen(&mut c);
        views(&s, &c);
        for _ in 0..6 {
            let k = right(&s);
            s.update(&mut c, Msg::Pick(k));
            assert!(run(&s).reveal.as_ref().unwrap().right);
            views(&s, &c);
            s.update(&mut c, Msg::Next);
        }
        for l in Lifeline::ALL {
            s.update(&mut c, Msg::Help(l));
        }
        assert_eq!(run(&s).ladder.shown.len(), 3);
        views(&s, &c);
        // A hidden option cannot be picked.
        let h = run(&s).ladder.hidden[0];
        s.update(&mut c, Msg::Pick(h));
        assert!(run(&s).reveal.is_none());
        // Keyboard: the physical key of a wrong, visible option.
        let st = run(&s).ladder.current().unwrap();
        let wrong = (0..st.opts.len())
            .find(|i| !st.opts[*i].right && !run(&s).ladder.hidden.contains(i))
            .unwrap();
        let code = [
            key::Code::KeyA,
            key::Code::KeyB,
            key::Code::KeyC,
            key::Code::KeyD,
        ][wrong];
        s.update(
            &mut c,
            Msg::Key(
                keyboard::Key::Character("ф".into()),
                key::Physical::Code(code),
                keyboard::Modifiers::empty(),
            ),
        );
        assert!(!run(&s).reveal.as_ref().unwrap().right);
        views(&s, &c);
        s.update(
            &mut c,
            Msg::Key(
                keyboard::Key::Named(key::Named::Enter),
                key::Physical::Code(key::Code::Enter),
                keyboard::Modifiers::empty(),
            ),
        );
        let Phase::Results(r) = &s.phase else {
            panic!("no results");
        };
        assert_eq!((r.r.kept, r.r.climbed), (5, 6));
        assert_eq!(r.r.lifelines_used, 3);
        assert!(r.review.is_some());
        views(&s, &c);
        // Finished exactly once.
        assert_eq!(c.state.counters.get(ladder::RUNS_KEY), Some(&1));
        s.update(&mut c, Msg::Next);
        assert_eq!(c.state.counters.get(ladder::RUNS_KEY), Some(&1));
    }

    #[test]
    fn walk_away_with_confirm() {
        let mut c = ctx();
        let mut s = run_screen(&mut c);
        // At step 1 the dialog only leaves, without a result.
        s.update(&mut c, Msg::WalkAway);
        assert!(s.dialog(&c).is_some());
        assert!(matches!(
            s.update(&mut c, Msg::ConfirmYes),
            Action::Go(Route::Timed)
        ));
        let mut s = run_screen(&mut c);
        for _ in 0..3 {
            let k = right(&s);
            s.update(&mut c, Msg::Pick(k));
            s.update(&mut c, Msg::Next);
        }
        s.update(&mut c, Msg::WalkAway);
        views(&s, &c);
        s.update(&mut c, Msg::ConfirmNo);
        assert!(!run(&s).confirm);
        s.update(&mut c, Msg::WalkAway);
        s.update(&mut c, Msg::ConfirmYes);
        let Phase::Results(r) = &s.phase else {
            panic!("no results");
        };
        assert_eq!(r.r.outcome, Outcome::WalkedAway);
        assert_eq!(r.r.kept, 3);
        assert!(r.review.is_none());
        views(&s, &c);
    }

    #[test]
    fn time_out_is_a_wrong_answer() {
        let mut c = ctx();
        let mut s = run_screen(&mut c);
        if let Phase::Run(r) = &mut s.phase {
            r.timer_end = Instant::now() - Duration::from_millis(1);
        }
        s.update(&mut c, Msg::Tick(Instant::now()));
        let rv = run(&s).reveal.as_ref().unwrap();
        assert!(!rv.right && rv.pick.is_none());
        views(&s, &c);
    }

    #[test]
    fn top_of_the_ladder() {
        let mut c = ctx();
        let mut s = run_screen(&mut c);
        for _ in 0..ladder::STEPS {
            let k = right(&s);
            s.update(&mut c, Msg::Pick(k));
            s.update(&mut c, Msg::Next);
        }
        let Phase::Results(r) = &s.phase else {
            panic!("no results");
        };
        assert_eq!(r.r.outcome, Outcome::Top);
        assert!(r.r.new_best);
        views(&s, &c);
    }

    #[test]
    fn demo_modes_build() {
        for (mode, results) in [
            ("mid", false),
            ("reveal", false),
            ("result", true),
            ("top", true),
        ] {
            let mut c = ctx();
            let s = Screen {
                phase: demo_phase(&mut c, mode),
            };
            assert_eq!(matches!(s.phase, Phase::Results(_)), results, "{mode}");
            if mode == "mid" {
                let r = run(&s);
                assert_eq!(r.ladder.climbed, 6);
                assert_eq!(r.ladder.shown.len(), 2);
                assert!(r.reveal.is_none());
            }
            if mode == "reveal" {
                assert!(run(&s).reveal.as_ref().unwrap().right);
            }
            views(&s, &c);
        }
    }
}
