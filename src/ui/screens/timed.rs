//! «Викторина» (was «Экзамен») — timed quiz-show (PARITY §10): intro/setup, run, results.
//!
//! Three PWA routes (`#/timed`, `#/timed-run`, `#/timed-results`) are phases
//! of one screen; `Route::Timed` / `Route::TimedRun { retry_ids }` are the
//! entry points. Selection, timing seconds, statuses, history and XP live in
//! `crate::exam` / `crate::game`. Sounds (WebAudio beeps) have no native
//! counterpart; the «Звуковые сигналы» switch is kept and persisted.

use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use iced::keyboard::{self, key};
use iced::widget::canvas::{self, Canvas, Frame, Path, Stroke};
use iced::widget::text::Shaping;
use iced::widget::{
    button, center, column, container, progress_bar, responsive, row, space, stack, text, Column,
    Row, Text,
};
use iced::{
    Alignment, Background, Border, Color, Element, Length, Point, Rectangle, Renderer, Shadow,
    Subscription, Theme,
};

use crate::content::{Difficulty, Library, Question};
use crate::exam::{self, TimedOption, TimedRes, TimedStatus};
use crate::game::{self, AwardOutcome};
use crate::ui::app::{Action, Ctx, Route, Toast};
use crate::ui::fonts::{SANS, SANS_BOLD, SERIF_BOLD};
use crate::ui::screens::map::icons;
use crate::ui::theme::{self, Palette};

/// The studio palette (§T.3), in both app themes.
fn tp() -> Palette {
    theme::TIMED
}

const INTRO_MS: u64 = 2500;
const INTRO_MS_TEST: u64 = 500;
const LOCK_MS: u64 = 1100;
const AUTO_MS: u64 = 2200;
const LETTERS: [&str; 4] = ["A", "B", "C", "D"];

// ---------------------------------------------------------------------------
// View helpers (all studio-palette)
// ---------------------------------------------------------------------------

fn txt<'a>(content: impl text::IntoFragment<'a>) -> Text<'a> {
    text(content).shaping(Shaping::Advanced)
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

fn kicker<'a>(content: impl text::IntoFragment<'a>) -> Text<'a> {
    txt(content)
        .size(theme::text::KICKER)
        .font(SANS_BOLD)
        .color(tp().gold)
}

fn muted<'a>(content: impl text::IntoFragment<'a>) -> Text<'a> {
    txt(content).color(tp().muted)
}

fn kbd<'a, Message: 'a>(label: impl text::IntoFragment<'a>) -> Element<'a, Message> {
    let p = tp();
    container(txt(label).size(12.0).font(SANS_BOLD).color(p.gold2))
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

/// Countdown ring (§10.2): dark track, gold2 stroke, red when ≤ 5 s.
struct TimerRing {
    frac: f32,
    danger: bool,
    dim: bool,
}

impl<Message> canvas::Program<Message> for TimerRing {
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
        let stroke_width = min * 7.0 / 84.0;
        let radius = (min - stroke_width) / 2.0;
        let center = Point::new(size.width / 2.0, size.height / 2.0);
        let dim = if self.dim { 0.5 } else { 1.0 };
        frame.stroke(
            &Path::circle(center, radius),
            Stroke {
                style: canvas::Style::Solid(Color::from_rgba8(5, 6, 25, 0.85 * dim)),
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
                    start_angle: iced::Radians(start),
                    end_angle: iced::Radians(end),
                });
            });
            let color = if self.danger {
                Color::from_rgb8(0xff, 0x4d, 0x3d)
            } else {
                p.gold2
            };
            frame.stroke(
                &arc,
                Stroke {
                    style: canvas::Style::Solid(alpha(color, dim)),
                    width: stroke_width,
                    line_cap: canvas::LineCap::Round,
                    ..Stroke::default()
                },
            );
        }
        vec![frame.into_geometry()]
    }
}

fn timer_ring<'a, Message: 'a>(
    frac: f32,
    secs: i64,
    size: f32,
    danger: bool,
    dim: bool,
) -> Element<'a, Message> {
    let p = tp();
    let canvas = Canvas::new(TimerRing { frac, danger, dim })
        .width(Length::Fixed(size))
        .height(Length::Fixed(size));
    let num_color = if danger {
        Color::from_rgb8(0xff, 0x6a, 0x5c)
    } else {
        p.text
    };
    let label = container(
        txt(secs.to_string())
            .size(size * 0.3)
            .font(SANS_BOLD)
            .color(alpha(num_color, if dim { 0.5 } else { 1.0 })),
    )
    .width(Length::Fixed(size))
    .height(Length::Fixed(size))
    .align_x(Alignment::Center)
    .align_y(Alignment::Center);
    stack![canvas, label].width(Length::Shrink).into()
}

/// Score ring for the results screen (gold2, §10.3).
fn result_ring<'a, Message: 'a>(pct: i64, size: f32) -> Element<'a, Message> {
    let p = tp();
    let canvas = Canvas::new(TimerRing {
        frac: pct as f32 / 100.0,
        danger: false,
        dim: false,
    })
    .width(Length::Fixed(size))
    .height(Length::Fixed(size));
    let label = container(
        txt(format!("{pct}%"))
            .size(size * 0.19)
            .font(SERIF_BOLD)
            .color(p.text),
    )
    .width(Length::Fixed(size))
    .height(Length::Fixed(size))
    .align_x(Alignment::Center)
    .align_y(Alignment::Center);
    stack![canvas, label].width(Length::Shrink).into()
}

/// Badge / goal toasts after a finished run (mirrors `Ctx::award` effects).
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

/// Quit-confirm dialog (§0.3): dimmed backdrop (click = cancel) + modal card.
/// Rendered as a shell layer above the whole window (see `screens::mod.rs`).
fn confirm_dialog<'a>(title: &'a str, body: &'a str) -> Element<'a, Msg> {
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
                tbutton("Отмена", TBtn::Ghost, Some(Msg::QuitCancel)),
                tbutton("Выйти", TBtn::Primary, Some(Msg::QuitConfirm)),
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

// ---------------------------------------------------------------------------
// TIMED-palette controls (mirrors of `widgets` helpers scoped to the studio)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
enum TBtn {
    Primary,
    Ghost,
}

/// Fixed-height (BTN_SM) centred-label button in the studio palette.
fn tbutton<'a>(label: &'a str, kind: TBtn, on_press: Option<Msg>) -> Element<'a, Msg> {
    let bold = matches!(kind, TBtn::Primary);
    button(
        container(
            txt(label)
                .size(15.0)
                .font(if bold { SANS_BOLD } else { SANS }),
        )
        .center_y(Length::Fill),
    )
    .height(theme::size::BTN_SM)
    .padding([0.0, 18.0])
    .style(move |_, s| match kind {
        TBtn::Primary => theme::button_primary(&tp(), s),
        TBtn::Ghost => theme::button_ghost(&tp(), s),
    })
    .on_press_maybe(on_press)
    .into()
}

/// «‹ label» back button in the studio palette (metrics of `widgets::back_button`).
fn tback_button<'a>(label: &'a str, msg: Msg) -> Element<'a, Msg> {
    button(
        container(
            row![
                icons::icon(icons::Kind::ChevronLeft, 14.0, tp().muted),
                txt(label).size(15.0),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .center_y(Length::Fill),
    )
    .height(theme::size::BTN_SM)
    .padding([0.0, 12.0])
    .style(move |_, s| theme::button_ghost(&tp(), s))
    .on_press(msg)
    .into()
}

/// «× Выйти» in the studio palette (metrics of `widgets::exit_button`).
fn texit_button<'a>(label: &'a str, msg: Msg) -> Element<'a, Msg> {
    button(
        container(
            row![txt("×").size(16.0).color(tp().muted), txt(label).size(15.0),]
                .spacing(6)
                .align_y(Alignment::Center),
        )
        .center_y(Length::Fill),
    )
    .height(theme::size::BTN_SM)
    .padding([0.0, 12.0])
    .style(move |_, s| theme::button_ghost(&tp(), s))
    .on_press(msg)
    .into()
}

/// Selectable chip / segment in the studio palette (see `widgets::choice`).
fn tchoice<'a>(label: String, selected: bool, round: bool, msg: Msg) -> Element<'a, Msg> {
    button(
        container(txt(label).size(theme::text::SMALL).font(if selected {
            SANS_BOLD
        } else {
            SANS
        }))
        .center(Length::Fill),
    )
    .height(theme::size::BTN_SM)
    .padding([0.0, 16.0])
    .style(move |_, status| theme::choice(&tp(), selected, round, status))
    .on_press(msg)
    .into()
}

// ---------------------------------------------------------------------------
// Setup state (count persists in memory, like the PWA module var)
// ---------------------------------------------------------------------------

fn count_store() -> &'static Mutex<usize> {
    static COUNT: OnceLock<Mutex<usize>> = OnceLock::new();
    COUNT.get_or_init(|| Mutex::new(exam::TIMED_DEFAULT_COUNT))
}

// ---------------------------------------------------------------------------
// Run state
// ---------------------------------------------------------------------------

/// A timed question made owned (the core `exam::TimedItem` borrows the library).
struct TimedOwn {
    q: Question,
    sec_title: Option<String>,
    opts: Vec<TimedOption>,
    seconds: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QPhase {
    /// Question alone on screen (2.5 s, 0.5 s under the test hook).
    Intro,
    /// Options visible, countdown running.
    Active,
    /// Chosen option locked (1.1 s) before the reveal.
    Locked,
    /// Answer revealed; waiting for «Дальше» or auto-advance.
    Revealed,
}

struct TimedRun {
    items: Vec<TimedOwn>,
    idx: usize,
    res: Vec<Option<TimedRes>>,
    base_seconds: i64,
    retry: bool,
    started_at: i64,
    intro_ms: u64,
    auto_setting: bool,
    phase: QPhase,
    /// End of the current intro / lock / auto-advance window.
    phase_deadline: Instant,
    /// Countdown end while `Active`.
    timer_end: Instant,
    /// Last computed milliseconds left (drives the ring).
    remaining_ms: i64,
    picked: Option<usize>,
    status: Option<TimedStatus>,
    auto_active: bool,
    auto_paused: bool,
    auto_frac: f32,
    confirm_quit: bool,
    /// When the quit dialog was opened (the clock is paused while it shows).
    quit_at: Option<Instant>,
}

impl TimedRun {
    fn new(
        items: Vec<TimedOwn>,
        base_seconds: i64,
        retry: bool,
        started_at: i64,
        ctx: &Ctx,
    ) -> Self {
        let intro_ms = if ctx.test_timer.is_some() {
            INTRO_MS_TEST
        } else {
            INTRO_MS
        };
        TimedRun {
            res: vec![None; items.len()],
            items,
            idx: 0,
            base_seconds,
            retry,
            started_at,
            intro_ms,
            auto_setting: ctx.state.settings.timed_auto,
            phase: QPhase::Intro,
            phase_deadline: Instant::now() + Duration::from_millis(intro_ms),
            timer_end: Instant::now(),
            remaining_ms: 0,
            picked: None,
            status: None,
            auto_active: false,
            auto_paused: false,
            auto_frac: 0.0,
            confirm_quit: false,
            quit_at: None,
        }
        .with_fresh_remaining()
    }

    fn with_fresh_remaining(mut self) -> Self {
        self.remaining_ms = self.items[self.idx].seconds * 1000;
        self
    }

    fn start_question(&mut self) {
        self.phase = QPhase::Intro;
        self.phase_deadline = Instant::now() + Duration::from_millis(self.intro_ms);
        self.remaining_ms = self.items[self.idx].seconds * 1000;
        self.picked = None;
        self.status = None;
        self.auto_active = false;
        self.auto_paused = false;
        self.auto_frac = 0.0;
    }

    fn tally(&self) -> (usize, usize, usize) {
        let mut t = (0, 0, 0);
        for r in self.res.iter().flatten() {
            match r.status {
                TimedStatus::Right => t.0 += 1,
                TimedStatus::Wrong => t.1 += 1,
                _ => t.2 += 1,
            }
        }
        t
    }

    /// k: chosen option index, or -1 for «не знаю»/timeout
    /// (js/ui/timed.js resolve).
    fn resolve(&mut self, ctx: &mut Ctx, k: i64, why: Option<TimedStatus>) {
        let now = Instant::now();
        let idx = self.idx;
        let it = &self.items[idx];
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
        self.res[idx] = Some(res.clone());
        let cr = exam::CheckResult {
            score: if correct { 1.0 } else { 0.0 },
            correct,
            user: res.user.clone().unwrap_or_default(),
        };
        let today = ctx.today();
        exam::record(&mut ctx.state, it.q.id(), &cr, today);
        ctx.save();
        self.picked = if k >= 0 { Some(k as usize) } else { None };
        self.status = Some(status);
        if k >= 0 {
            self.phase = QPhase::Locked;
            self.phase_deadline = now + Duration::from_millis(LOCK_MS);
        } else {
            self.reveal(now);
        }
    }

    fn reveal(&mut self, now: Instant) {
        self.phase = QPhase::Revealed;
        if self.auto_setting {
            self.auto_active = true;
            self.auto_paused = false;
            self.auto_frac = 0.0;
            self.phase_deadline = now + Duration::from_millis(AUTO_MS);
        }
    }

    /// Finish the run: results, history, perfect counter, XP (§10.2 «Finish»).
    fn finish(&self, ctx: &mut Ctx) -> TimedResults {
        let items: Vec<exam::TimedItem> = self
            .items
            .iter()
            .map(|it| {
                let sec = ctx.lib.question(it.q.id()).map(|(_, s)| s);
                exam::TimedItem {
                    q: &it.q,
                    sec,
                    opts: it.opts.clone(),
                    seconds: it.seconds,
                }
            })
            .collect();
        let run = exam::TimedRun {
            items,
            idx: self.idx,
            res: self.res.clone(),
            seconds: self.base_seconds,
            retry: self.retry,
            started_at: self.started_at,
        };
        let today = ctx.today();
        let res = exam::finish_timed(&ctx.lib, &run, &mut ctx.state, today);
        celebrate_award(ctx, &res.award);
        ctx.save();
        let cards = res
            .misses
            .iter()
            .map(|&i| {
                let it = &self.items[i];
                let r = self.res[i].clone().unwrap_or(TimedRes {
                    status: TimedStatus::Skip,
                    k: -1,
                    user: None,
                });
                let yours = match r.status {
                    TimedStatus::Wrong => r.user.unwrap_or_else(|| "—".to_string()),
                    TimedStatus::Timeout => "не знаю (время вышло)".to_string(),
                    _ => "не знаю".to_string(),
                };
                let right = it
                    .opts
                    .iter()
                    .find(|o| o.right)
                    .map(|o| o.text.clone())
                    .unwrap_or_default();
                ReviewCard {
                    prompt: it.q.common().prompt.clone(),
                    yours,
                    right,
                    explanation: it.q.common().explanation.clone(),
                    source: it.q.common().source.clone(),
                }
            })
            .collect();
        let miss_ids = res
            .misses
            .iter()
            .map(|&i| self.items[i].q.id().to_string())
            .collect();
        TimedResults {
            res,
            cards,
            miss_ids,
        }
    }
}

struct ReviewCard {
    prompt: String,
    yours: String,
    right: String,
    explanation: Option<String>,
    source: Option<String>,
}

struct TimedResults {
    res: exam::TimedResult,
    cards: Vec<ReviewCard>,
    miss_ids: Vec<String>,
}

// ---------------------------------------------------------------------------
// Screen
// ---------------------------------------------------------------------------

enum Phase {
    Intro,
    Run(Box<TimedRun>),
    Results(Box<TimedResults>),
}

pub struct Screen {
    phase: Phase,
}

#[derive(Debug, Clone)]
pub enum Msg {
    // Setup.
    Back,
    Count(usize),
    Secs(i64),
    Sound(bool),
    Auto(bool),
    Start,
    // Runner.
    Quit,
    QuitConfirm,
    QuitCancel,
    Pick(usize),
    Skip,
    Pause,
    Next,
    // Results.
    Again,
    RetryMisses,
    ToMap,
    // Subscriptions.
    Tick(Instant),
    Key(keyboard::Key, key::Physical, keyboard::Modifiers),
}

/// Build an owned timed run (js/ui/timed.js T.build).
fn build_run(ctx: &Ctx, retry_ids: Option<&Vec<String>>) -> Option<TimedRun> {
    let count = *count_store().lock().expect("count lock");
    let mut rng = rand::thread_rng();
    let run = exam::build_timed(
        &ctx.lib,
        &ctx.state,
        retry_ids.map(|v| v.as_slice()),
        Some(count),
        &mut rng,
    );
    if run.items.is_empty() {
        return None;
    }
    let test = ctx.test_timer.map(i64::from);
    let items = run
        .items
        .iter()
        .map(|it| TimedOwn {
            q: it.q.clone(),
            sec_title: it.sec.map(|s| s.title.clone()),
            opts: it.opts.clone(),
            // The test hook keeps full control (no +5 s for easy).
            seconds: test.unwrap_or(it.seconds),
        })
        .collect();
    let base = test.unwrap_or(run.seconds);
    Some(TimedRun::new(items, base, run.retry, run.started_at, ctx))
}

impl Screen {
    pub fn new(ctx: &mut Ctx, route: &Route) -> Self {
        match route {
            Route::TimedRun { retry_ids } => match build_run(ctx, retry_ids.as_ref()) {
                Some(run) => Screen {
                    phase: Phase::Run(Box::new(run)),
                },
                // No run → intro (PWA redirects to `#/timed`).
                None => Screen {
                    phase: Phase::Intro,
                },
            },
            _ => Screen {
                phase: Phase::Intro,
            },
        }
    }

    pub fn update(&mut self, ctx: &mut Ctx, msg: Msg) -> Action {
        match msg {
            // ---- Setup ----
            Msg::Back | Msg::ToMap => Action::Go(Route::Map),
            Msg::Count(n) => {
                *count_store().lock().expect("count lock") = n;
                Action::None
            }
            Msg::Secs(v) => {
                ctx.state.settings.timed_seconds = v;
                ctx.save();
                Action::None
            }
            Msg::Sound(v) => {
                // The PWA plays the «right» chime when enabled; no audio here.
                ctx.state.settings.timed_sound = v;
                ctx.save();
                Action::None
            }
            Msg::Auto(v) => {
                ctx.state.settings.timed_auto = v;
                ctx.save();
                Action::None
            }
            Msg::Start => Action::Go(Route::TimedRun { retry_ids: None }),
            // ---- Runner ----
            Msg::Quit => {
                if let Phase::Run(run) = &mut self.phase {
                    if !run.confirm_quit {
                        run.confirm_quit = true;
                        run.quit_at = Some(Instant::now());
                    }
                }
                Action::None
            }
            Msg::QuitConfirm => Action::Go(Route::Timed),
            Msg::QuitCancel => {
                if let Phase::Run(run) = &mut self.phase {
                    run.confirm_quit = false;
                    // Give back the time spent in the dialog.
                    if let Some(at) = run.quit_at.take() {
                        let paused = at.elapsed();
                        run.timer_end += paused;
                        run.phase_deadline += paused;
                    }
                }
                Action::None
            }
            Msg::Pick(k) => {
                if let Phase::Run(run) = &mut self.phase {
                    if run.phase == QPhase::Active {
                        run.resolve(ctx, k as i64, None);
                    }
                }
                Action::None
            }
            Msg::Skip => {
                if let Phase::Run(run) = &mut self.phase {
                    if matches!(run.phase, QPhase::Intro | QPhase::Active) {
                        run.resolve(ctx, -1, Some(TimedStatus::Skip));
                    }
                }
                Action::None
            }
            Msg::Pause => {
                if let Phase::Run(run) = &mut self.phase {
                    if run.phase == QPhase::Revealed && run.auto_active {
                        run.auto_active = false;
                        run.auto_paused = true;
                    }
                }
                Action::None
            }
            Msg::Next => {
                let finished = match &self.phase {
                    Phase::Run(run) => {
                        run.phase == QPhase::Revealed && run.idx + 1 >= run.items.len()
                    }
                    _ => false,
                };
                if finished {
                    if let Phase::Run(run) = &self.phase {
                        let results = run.finish(ctx);
                        self.phase = Phase::Results(Box::new(results));
                    }
                } else if let Phase::Run(run) = &mut self.phase {
                    if run.phase == QPhase::Revealed {
                        run.idx += 1;
                        run.start_question();
                    }
                }
                Action::None
            }
            // ---- Results ----
            Msg::Again => Action::Go(Route::TimedRun { retry_ids: None }),
            Msg::RetryMisses => {
                if let Phase::Results(r) = &self.phase {
                    if !r.miss_ids.is_empty() {
                        return Action::Go(Route::TimedRun {
                            retry_ids: Some(r.miss_ids.clone()),
                        });
                    }
                }
                Action::None
            }
            // ---- Subscriptions ----
            Msg::Tick(now) => {
                let Phase::Run(run) = &mut self.phase else {
                    return Action::None;
                };
                // The countdown, lock and auto-advance pause while the quit
                // dialog is open (its text promises the run won't count yet).
                if run.confirm_quit {
                    return Action::None;
                }
                match run.phase {
                    QPhase::Intro => {
                        if now >= run.phase_deadline {
                            run.phase = QPhase::Active;
                            run.timer_end = now
                                + Duration::from_millis((run.items[run.idx].seconds * 1000) as u64);
                            run.remaining_ms = run.items[run.idx].seconds * 1000;
                        }
                    }
                    QPhase::Active => {
                        let left = run.timer_end.saturating_duration_since(now);
                        run.remaining_ms = left.as_millis() as i64;
                        if left.is_zero() {
                            run.resolve(ctx, -1, Some(TimedStatus::Timeout));
                        }
                    }
                    QPhase::Locked => {
                        if now >= run.phase_deadline {
                            run.reveal(now);
                        }
                    }
                    QPhase::Revealed => {
                        if run.auto_active {
                            let left = run.phase_deadline.saturating_duration_since(now);
                            run.auto_frac = 1.0 - left.as_millis() as f32 / AUTO_MS as f32;
                            if left.is_zero() {
                                return self.update(ctx, Msg::Next);
                            }
                        }
                    }
                }
                Action::None
            }
            Msg::Key(k, physical, modifiers) => {
                if modifiers.command() || modifiers.alt() {
                    return Action::None;
                }
                let Phase::Run(run) = &mut self.phase else {
                    return Action::None;
                };
                if run.confirm_quit {
                    if matches!(k, keyboard::Key::Named(key::Named::Escape)) {
                        return self.update(ctx, Msg::QuitCancel);
                    }
                    return Action::None;
                }
                match run.phase {
                    QPhase::Revealed => {
                        if matches!(
                            k,
                            keyboard::Key::Named(key::Named::Enter)
                                | keyboard::Key::Named(key::Named::Space)
                        ) {
                            return self.update(ctx, Msg::Next);
                        }
                    }
                    QPhase::Active => {
                        // Physical codes first: on a non-Latin layout the A
                        // key sends «ф», so Key::Character alone misses A–D
                        // (the PWA uses e.code).
                        let idx = match physical {
                            key::Physical::Code(code) => match code {
                                key::Code::KeyA | key::Code::Digit1 | key::Code::Numpad1 => Some(0),
                                key::Code::KeyB | key::Code::Digit2 | key::Code::Numpad2 => Some(1),
                                key::Code::KeyC | key::Code::Digit3 | key::Code::Numpad3 => Some(2),
                                key::Code::KeyD | key::Code::Digit4 | key::Code::Numpad4 => Some(3),
                                _ => None,
                            },
                            key::Physical::Unidentified(_) => None,
                        };
                        let idx = idx.or_else(|| {
                            if let keyboard::Key::Character(c) = &k {
                                match c.to_ascii_lowercase().as_str() {
                                    "a" | "1" => Some(0),
                                    "b" | "2" => Some(1),
                                    "c" | "3" => Some(2),
                                    "d" | "4" => Some(3),
                                    _ => None,
                                }
                            } else {
                                None
                            }
                        });
                        if let Some(i) = idx {
                            if i < run.items[run.idx].opts.len() {
                                return self.update(ctx, Msg::Pick(i));
                            }
                        }
                    }
                    _ => {}
                }
                Action::None
            }
        }
    }

    pub fn view<'a>(&'a self, ctx: &'a Ctx) -> Element<'a, Msg> {
        match &self.phase {
            Phase::Intro => self.view_intro(ctx),
            Phase::Run(run) => self.view_run(run),
            Phase::Results(r) => self.view_results(ctx, r),
        }
    }

    /// The quit-confirm dialog, rendered as a shell layer above the window.
    pub fn dialog<'a>(&'a self, _ctx: &'a Ctx) -> Option<Element<'a, Msg>> {
        match &self.phase {
            Phase::Run(run) if run.confirm_quit => Some(confirm_dialog(
                "Выйти из викторины?",
                "Уже данные ответы сохранятся в статистике, но результат игры не будет засчитан.",
            )),
            _ => None,
        }
    }

    pub fn subscription(&self) -> Subscription<Msg> {
        match &self.phase {
            Phase::Run(_) => Subscription::batch([
                iced::time::every(Duration::from_millis(100)).map(Msg::Tick),
                keyboard::listen().filter_map(|ev| match ev {
                    keyboard::Event::KeyPressed {
                        key,
                        physical_key,
                        modifiers,
                        ..
                    } => Some(Msg::Key(key, physical_key, modifiers)),
                    _ => None,
                }),
            ]),
            _ => Subscription::none(),
        }
    }
}

// ---------------------------------------------------------------------------
// Intro / setup view (§10.1)
// ---------------------------------------------------------------------------

impl Screen {
    fn view_intro<'a>(&'a self, ctx: &'a Ctx) -> Element<'a, Msg> {
        let p = tp();
        let count = *count_store().lock().expect("count lock");
        let secs = exam::base_seconds(&ctx.state);
        let avail = exam::timed_pool(&ctx.lib, &ctx.state).len();

        let mut col = Column::new().spacing(20).width(Length::Fill);
        col = col.push(tback_button("На главную", Msg::Back));

        // Hero: static timer ring + title.
        col = col.push(
            row![
                timer_ring(1.0, secs, 96.0, false, false),
                column![
                    kicker("РЕЖИМ ТЕЛЕВИКТОРИНЫ"),
                    txt("Викторина").size(37.0).font(SERIF_BOLD).color(p.gold2),
                    muted(
                        "Сначала — вопрос. Через пару секунд — четыре варианта ответа и таймер. \
                         Один выбор, без права на исправление.",
                    )
                    .size(theme::text::BODY),
                ]
                .spacing(8)
                .width(Length::Fill),
            ]
            .spacing(24)
            .align_y(Alignment::Center)
            .width(Length::Fill),
        );

        // Setup card.
        let mut card_col = Column::new().spacing(18).width(Length::Fill);

        let mut chips = Row::new().spacing(8);
        for n in exam::TIMED_COUNTS {
            let selected = count == n;
            chips = chips.push(tchoice(format!("{n}"), selected, true, Msg::Count(n)));
        }
        card_col = card_col.push(
            column![muted("Количество вопросов").size(theme::text::SMALL), chips,].spacing(6),
        );

        let mut seg = Row::new().spacing(8);
        for v in [20i64, 30] {
            let selected = secs == v;
            seg = seg.push(tchoice(format!("{v} с"), selected, false, Msg::Secs(v)));
        }
        card_col = card_col
            .push(column![muted("Время на вопрос").size(theme::text::SMALL), seg].spacing(6));

        card_col = card_col.push(
            iced::widget::toggler(ctx.state.settings.timed_sound)
                .label("Звуковые сигналы")
                .on_toggle(Msg::Sound)
                .text_shaping(Shaping::Advanced)
                .text_size(theme::text::BODY),
        );
        card_col = card_col.push(
            iced::widget::toggler(ctx.state.settings.timed_auto)
                .label("Автопереход к следующему вопросу")
                .on_toggle(Msg::Auto)
                .text_shaping(Shaping::Advanced)
                .text_size(theme::text::BODY),
        );

        // Rules.
        let level_title = crate::content::level_info(Library::user_level(&ctx.state))
            .title
            .to_lowercase();
        let rules = Column::new()
            .spacing(8)
            .width(Length::Fill)
            .push(
                row![
                    muted("• Ответ — клик или клавиши ").size(theme::text::BODY),
                    kbd("A"),
                    muted("–").size(theme::text::BODY),
                    kbd("D"),
                    muted(" / ").size(theme::text::BODY),
                    kbd("1"),
                    muted("–").size(theme::text::BODY),
                    kbd("4"),
                    muted(".").size(theme::text::BODY),
                ]
                .align_y(Alignment::Center),
            )
            .push(
                muted(
                    "• Порядок — от лёгких к трудным: сначала разминка (на лёгкие +5 с), \
                     затем средние и трудные вопросы.",
                )
                .size(theme::text::BODY),
            )
            .push(
                muted(
                    "• «Не знаю» и истёкшее время считаются ошибкой — такие вопросы потом \
                     попадаются чаще.",
                )
                .size(theme::text::BODY),
            )
            .push(
                muted(format!(
                    "• Вопросы — из открытых разделов, по вашему уровню ({level_title}); \
                     каждый раз — новый набор."
                ))
                .size(theme::text::BODY),
            );
        card_col = card_col.push(rules);

        let mut avail_line = format!("Доступно вопросов: {avail}");
        if avail > 0 && avail < count {
            avail_line.push_str(&format!(" — будет задано {avail}"));
        }
        card_col = card_col.push(muted(avail_line).size(theme::text::SMALL));
        if avail == 0 {
            card_col = card_col.push(
                muted("В открытых разделах пока нет вопросов с выбором ответа.")
                    .size(theme::text::BODY),
            );
        }

        let go = button(
            container(txt("Начать игру").size(theme::text::BODY).font(SANS_BOLD))
                .center_y(Length::Fill),
        )
        .padding([0.0, 26.0])
        .height(theme::size::BTN_LG)
        .style(move |_, s| theme::button_tq_go(&p, s));
        let go = if avail > 0 {
            go.on_press(Msg::Start)
        } else {
            go
        };
        card_col = card_col.push(container(go).width(Length::Fill).align_x(Alignment::Center));

        col = col.push(
            container(card_col)
                .padding(theme::spacing::CARD_PAD)
                .width(Length::Fill)
                .style(move |_| container::Style {
                    background: Some(Background::Color(p.surface)),
                    border: Border {
                        color: p.line2,
                        width: 1.0,
                        radius: theme::radius::CARD.into(),
                    },
                    ..plain()
                }),
        );

        container(col)
            .padding(28)
            .width(Length::Fill)
            .style(move |_| theme::tq_container(&tp()))
            .into()
    }
}

/// A fixed-height BTN_MIN button with a centred label (studio palette).
fn mid_button<'a>(
    label: Element<'a, Msg>,
    style: for<'t> fn(&'t Palette, button::Status) -> button::Style,
    on_press: Option<Msg>,
) -> Element<'a, Msg> {
    button(container(label).center_y(Length::Fill))
        .padding([0.0, 20.0])
        .height(theme::size::BTN_MD)
        .style(move |_, s| style(&tp(), s))
        .on_press_maybe(on_press)
        .into()
}

// ---------------------------------------------------------------------------
// Run view (§10.2)
// ---------------------------------------------------------------------------

impl Screen {
    fn view_run<'a>(&'a self, run: &'a TimedRun) -> Element<'a, Msg> {
        let p = tp();
        let it = &run.items[run.idx];
        let total = run.items.len();
        let (right, wrong, skip) = run.tally();

        let mut col = Column::new().spacing(16).width(Length::Fill);

        // Top bar: quit, counter, score pills.
        let quit = texit_button("Выйти", Msg::Quit);
        let counter = iced::widget::rich_text::<(), Msg, Theme, Renderer>(vec![
            iced::widget::text::Span::new("Вопрос ")
                .font(SERIF_BOLD)
                .color(p.text),
            iced::widget::text::Span::new(format!("{}", run.idx + 1))
                .font(SERIF_BOLD)
                .color(p.gold2),
            iced::widget::text::Span::new(format!(" из {total}"))
                .font(SERIF_BOLD)
                .color(p.text),
        ])
        .size(theme::text::BODY);
        let pills = row![
            txt(format!("✓ {right} верно"))
                .size(theme::text::SMALL)
                .color(p.ok),
            txt(format!("× {wrong} неверно"))
                .size(theme::text::SMALL)
                .color(p.bad),
            txt(format!("? {skip} пропущено"))
                .size(theme::text::SMALL)
                .color(p.muted),
        ]
        .spacing(14)
        .align_y(Alignment::Center);
        col = col.push(
            row![quit, counter, space::horizontal(), pills]
                .spacing(14)
                .align_y(Alignment::Center)
                .width(Length::Fill),
        );

        // Main area (ladder on wide screens, §10.2 «Ladder»).
        let area = responsive(move |size| {
            let main: Element<'a, Msg> = self.view_play(run, it);
            if size.width >= 800.0 {
                row![main, self.view_ladder(run)]
                    .spacing(20)
                    .width(Length::Fill)
                    .into()
            } else {
                main
            }
        });
        col = col.push(area);

        container(col)
            .padding(28)
            .width(Length::Fill)
            .style(move |_| theme::tq_container(&tp()))
            .into()
    }

    /// Question panel + options + controls.
    fn view_play<'a>(&'a self, run: &'a TimedRun, it: &'a TimedOwn) -> Element<'a, Msg> {
        let p = tp();
        let mut col = Column::new().spacing(16).width(Length::Fill);

        // Timer ring.
        let secs_left = (run.remaining_ms as f64 / 1000.0).ceil() as i64;
        let total_ms = it.seconds * 1000;
        let frac = if total_ms > 0 {
            run.remaining_ms as f32 / total_ms as f32
        } else {
            1.0
        };
        let danger = run.phase == QPhase::Active && run.remaining_ms <= 5000;
        let dim = matches!(run.phase, QPhase::Intro | QPhase::Revealed);
        let shown = match run.phase {
            QPhase::Intro => it.seconds,
            _ => secs_left,
        };
        col = col.push(timer_ring(frac, shown.max(0), 84.0, danger, dim));

        // Question panel (hexagonal gold frame approximated).
        let mut meta = it.sec_title.clone().unwrap_or_default();
        if matches!(it.q, Question::Tf { .. }) {
            if !meta.is_empty() {
                meta.push_str(" · ");
            }
            meta.push_str("верно или неверно?");
        }
        let (diff_label, diff_color) = match it.q.common().difficulty {
            Some(Difficulty::Easy) => ("лёгкий", p.ok),
            Some(Difficulty::Hard) => ("трудный", p.bad),
            _ => ("средний", p.info),
        };
        let meta_row = row![
            muted(meta).size(theme::text::SMALL),
            container(
                txt(diff_label)
                    .size(theme::text::SMALL)
                    .font(SANS_BOLD)
                    .color(diff_color)
            )
            .padding([0.0, 12.0])
            .center_y(Length::Fixed(theme::size::TAG))
            .style(move |_| container::Style {
                background: Some(Background::Color(alpha(diff_color, 0.14))),
                border: Border {
                    radius: theme::radius::PILL.into(),
                    ..Border::default()
                },
                ..plain()
            }),
        ]
        .spacing(10)
        .align_y(Alignment::Center);
        col = col.push(
            container(
                column![
                    meta_row,
                    txt(it.q.common().prompt.as_str())
                        .size(22.4)
                        .font(SERIF_BOLD)
                        .color(p.text),
                ]
                .spacing(12)
                .width(Length::Fill),
            )
            .padding([20.0, 24.0])
            .width(Length::Fill)
            .style(move |_| container::Style {
                background: Some(Background::Color(p.surface2)),
                border: Border {
                    color: p.line2,
                    width: 1.5,
                    radius: theme::radius::TIMED.into(),
                },
                ..plain()
            }),
        );

        // Options (hidden during the intro unless skipping from it).
        if run.phase != QPhase::Intro {
            col = col.push(self.view_options(run, it));
        }

        // Controls.
        match run.phase {
            QPhase::Intro | QPhase::Active => {
                let skip = button(
                    container(
                        txt("Не знаю — дальше")
                            .size(theme::text::BODY)
                            .color(p.gold2),
                    )
                    .center(Length::Fill),
                )
                .width(Length::Fixed(220.0))
                .padding([0.0, 20.0])
                .height(theme::size::BTN_MD)
                .style(move |_, status| {
                    let hovered =
                        matches!(status, button::Status::Hovered | button::Status::Pressed);
                    button::Style {
                        background: Some(Background::Color(if hovered {
                            p.surface3
                        } else {
                            p.surface2
                        })),
                        text_color: p.gold2,
                        border: Border {
                            color: p.line2,
                            width: 1.0,
                            radius: theme::radius::BUTTON.into(),
                        },
                        shadow: Shadow::default(),
                        snap: true,
                    }
                })
                .on_press(Msg::Skip);
                col = col.push(
                    container(skip)
                        .width(Length::Fill)
                        .align_x(Alignment::Center),
                );
            }
            QPhase::Locked => {}
            QPhase::Revealed => {
                col = col.push(self.view_reveal_controls(run));
            }
        }
        col.into()
    }

    /// Option buttons «A: …» with the reveal colouring.
    fn view_options<'a>(&'a self, run: &'a TimedRun, it: &'a TimedOwn) -> Element<'a, Msg> {
        let p = tp();
        let n = it.opts.len();
        let mut col = Column::new().spacing(10).width(Length::Fill);
        let mut row_acc = Row::new().spacing(10).width(Length::Fill);
        for (k, opt) in it.opts.iter().enumerate() {
            let letter = LETTERS.get(k).copied().unwrap_or("?");
            let content = row![
                txt(format!("{letter}:"))
                    .size(theme::text::BODY)
                    .font(SANS_BOLD)
                    .color(p.gold2),
                txt(opt.text.as_str()).size(theme::text::BODY).color(p.text),
            ]
            .spacing(10)
            .align_y(Alignment::Center)
            .width(Length::Fill);
            let is_picked = run.picked == Some(k);
            let revealed = run.phase == QPhase::Revealed;
            let locked = run.phase == QPhase::Locked && is_picked;
            let b = button(content)
                .width(Length::FillPortion(1))
                .padding([12.0, 16.0])
                .style(move |_, status| {
                    if revealed {
                        if opt.right {
                            return option_filled(&p, p.ok_soft, p.ok);
                        }
                        if is_picked {
                            return option_filled(&p, p.bad_soft, p.bad);
                        }
                        // Dimmed.
                        return option_filled(&p, alpha(p.surface2, 0.5), alpha(p.line2, 0.5));
                    }
                    if locked {
                        // Orange «locked» flash (§T.3).
                        return button::Style {
                            background: Some(Background::Color(p.flame)),
                            text_color: Color::from_rgb8(0x1e, 0x11, 0x00),
                            border: Border {
                                color: p.gold2,
                                width: 1.0,
                                radius: theme::radius::TIMED.into(),
                            },
                            shadow: Shadow::default(),
                            snap: true,
                        };
                    }
                    let hovered =
                        matches!(status, button::Status::Hovered | button::Status::Pressed);
                    button::Style {
                        background: Some(Background::Color(if hovered {
                            p.surface3
                        } else {
                            p.surface2
                        })),
                        text_color: p.text,
                        border: Border {
                            color: p.line2,
                            width: 1.0,
                            radius: theme::radius::TIMED.into(),
                        },
                        shadow: Shadow::default(),
                        snap: true,
                    }
                });
            let b = if run.phase == QPhase::Active {
                b.on_press(Msg::Pick(k))
            } else {
                b
            };
            row_acc = row_acc.push(b);
            // 2 columns; tf (2 options) stays in one row.
            if n > 2 && k % 2 == 1 {
                col = col.push(std::mem::replace(
                    &mut row_acc,
                    Row::new().spacing(10).width(Length::Fill),
                ));
            }
        }
        if n % 2 == 1 || n <= 2 {
            col = col.push(row_acc);
        }
        col.into()
    }

    /// Verdict + next/auto controls after the reveal (§10.2 «Reveal»).
    fn view_reveal_controls<'a>(&'a self, run: &'a TimedRun) -> Element<'a, Msg> {
        let p = tp();
        let status = run.status.unwrap_or(TimedStatus::Skip);
        let color = match status {
            TimedStatus::Right => p.ok,
            TimedStatus::Wrong => p.bad,
            _ => p.gold2,
        };
        let mut verdict = status.message().to_string();
        if status != TimedStatus::Right {
            verdict.push_str(" · правильный ответ выделен зелёным");
        }

        let last = run.idx + 1 >= run.items.len();
        let next_label = if last { "Итоги" } else { "Дальше" };
        let mut controls = Row::new().spacing(10).align_y(Alignment::Center);
        if run.auto_setting {
            let pause = if run.auto_paused {
                mid_button(
                    txt("Автопереход на паузе").size(theme::text::BODY).into(),
                    theme::button_ghost,
                    None,
                )
            } else {
                mid_button(
                    txt("Пауза").size(theme::text::BODY).into(),
                    theme::button_ghost,
                    Some(Msg::Pause),
                )
            };
            controls = controls.push(pause);
        }
        controls = controls.push(
            button(
                container(
                    row![
                        txt(next_label).size(theme::text::BODY).font(SANS_BOLD),
                        kbd("Enter"),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                )
                .center_y(Length::Fill),
            )
            .padding([0.0, 22.0])
            .height(theme::size::BTN_MD)
            .style(move |_, s| theme::button_tq_go(&p, s))
            .on_press(Msg::Next),
        );

        let mut col = Column::new().spacing(12).width(Length::Fill);
        col = col.push(txt(verdict).size(19.6).font(SERIF_BOLD).color(color));
        col = col.push(controls);
        if run.auto_setting {
            let frac = run.auto_frac;
            col = col.push(
                progress_bar(0.0..=1.0, frac.clamp(0.0, 1.0))
                    .girth(3.0)
                    .style(move |_| theme::progress_bar(&p)),
            );
        }
        col.into()
    }

    /// Ladder column (right side, wide screens): numbers n…1 with status bars.
    fn view_ladder<'a>(&'a self, run: &'a TimedRun) -> Element<'a, Msg> {
        let p = tp();
        let mut col = Column::new().spacing(3).width(Length::Fixed(96.0));
        for i in (0..run.items.len()).rev() {
            let r = run.res.get(i).and_then(|r| r.as_ref());
            let current = i == run.idx && r.is_none();
            let (num_color, bar_color) = match r {
                Some(TimedRes {
                    status: TimedStatus::Right,
                    ..
                }) => (p.muted, p.gold),
                Some(TimedRes {
                    status: TimedStatus::Wrong,
                    ..
                }) => (p.muted, p.bad),
                Some(_) => (p.muted, p.surface3),
                None if current => (p.gold2, p.flame),
                None => (p.muted, alpha(p.surface3, 0.5)),
            };
            col = col.push(
                row![
                    container(
                        txt(format!("{}", i + 1))
                            .size(12.0)
                            .font(if current { SANS_BOLD } else { SANS })
                            .color(num_color),
                    )
                    .width(Length::Fixed(24.0)),
                    container(space())
                        .width(Length::Fill)
                        .height(6.0)
                        .style(move |_| container::Style {
                            background: Some(Background::Color(bar_color)),
                            border: Border {
                                radius: theme::radius::PILL.into(),
                                ..Border::default()
                            },
                            ..plain()
                        }),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }
        col.into()
    }

    // -----------------------------------------------------------------------
    // Results view (§10.3)
    // -----------------------------------------------------------------------

    fn view_results<'a>(&'a self, ctx: &'a Ctx, r: &'a TimedResults) -> Element<'a, Msg> {
        let p = tp();
        let res = &r.res;
        let (title, sub) = exam::timed_result_title(res.pct);

        let mut col = Column::new().spacing(18).width(Length::Fill);
        let mut head = Column::new()
            .spacing(10)
            .width(Length::Fill)
            .align_x(Alignment::Center);
        head = head.push(kicker(format!("ВИКТОРИНА · {} С НА ВОПРОС", res.seconds)));
        head = head.push(result_ring(res.pct, 150.0));
        head = head.push(
            txt(title)
                .size(theme::text::H1)
                .font(SERIF_BOLD)
                .color(p.text),
        );
        head = head.push(muted(sub).size(theme::text::BODY));

        // Tiles.
        let tiles = row![
            result_tile(
                format!("{}", res.right),
                crate::util::plural(res.right as i64, &["верный", "верных", "верных"]),
                p.ok,
            ),
            result_tile(
                format!("{}", res.wrong),
                crate::util::plural(res.wrong as i64, &["ошибка", "ошибки", "ошибок"]),
                p.bad,
            ),
            result_tile(format!("{}", res.skip), "пропущено", p.gold2),
        ]
        .spacing(12)
        .width(Length::Fill);
        head = head.push(container(tiles).width(Length::Fill).padding([8.0, 0.0]));
        head = head.push(
            muted(format!(
                "Всего вопросов: {} · верно {}%",
                res.total, res.pct
            ))
            .size(theme::text::SMALL),
        );
        let today_xp = game::today_xp(&ctx.state, ctx.today());
        let goal = game::goal(&ctx.state);
        head = head.push(
            txt(format!("+{} XP · сегодня {today_xp} / {goal} XP", res.xp))
                .size(theme::text::BODY)
                .color(p.gold2),
        );

        // Buttons.
        let mut buttons = Row::new().spacing(10).align_y(Alignment::Center);
        buttons = buttons.push(
            button(
                container(txt("Ещё раз").size(theme::text::BODY).font(SANS_BOLD))
                    .center_y(Length::Fill),
            )
            .padding([0.0, 24.0])
            .height(theme::size::BTN_LG)
            .style(move |_, s| theme::button_tq_go(&p, s))
            .on_press(Msg::Again),
        );
        if !r.miss_ids.is_empty() {
            buttons = buttons.push(
                button(
                    container(
                        txt(format!("Повторить ошибки ({})", r.miss_ids.len()))
                            .size(theme::text::BODY),
                    )
                    .center_y(Length::Fill),
                )
                .padding([0.0, 20.0])
                .height(theme::size::BTN_LG)
                .style(move |_, s| theme::button_secondary(&p, s))
                .on_press(Msg::RetryMisses),
            );
        }
        buttons = buttons.push(
            button(container(txt("На главную").size(theme::text::BODY)).center_y(Length::Fill))
                .padding([0.0, 20.0])
                .height(theme::size::BTN_LG)
                .style(move |_, s| theme::button_ghost(&p, s))
                .on_press(Msg::ToMap),
        );
        head = head.push(
            container(buttons)
                .width(Length::Fill)
                .align_x(Alignment::Center),
        );
        col = col.push(head);

        // Review.
        if r.cards.is_empty() {
            col = col.push(
                container(
                    muted("Ни одной ошибки — превосходно!")
                        .size(theme::text::BODY)
                        .align_x(iced::alignment::Horizontal::Center)
                        .width(Length::Fill),
                )
                .padding(12),
            );
        } else {
            col = col.push(
                txt("Разбор ошибок и пропусков")
                    .size(theme::text::H3)
                    .font(SERIF_BOLD)
                    .color(p.text),
            );
            for card in &r.cards {
                let mut c = Column::new().spacing(8).width(Length::Fill);
                c = c.push(
                    txt(card.prompt.as_str())
                        .size(theme::text::BODY * 1.05)
                        .font(SERIF_BOLD)
                        .color(p.text),
                );
                c = c.push(
                    txt(format!("Ваш ответ: {}", card.yours))
                        .size(theme::text::BODY)
                        .color(p.bad),
                );
                c = c.push(
                    iced::widget::rich_text::<(), Msg, Theme, Renderer>(vec![
                        iced::widget::text::Span::new("Правильно: ")
                            .font(SANS_BOLD)
                            .color(p.text),
                        iced::widget::text::Span::new(card.right.as_str())
                            .font(SANS)
                            .color(p.ok),
                    ])
                    .size(theme::text::BODY),
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
                col = col.push(
                    container(c)
                        .padding(theme::spacing::CARD_PAD)
                        .width(Length::Fill)
                        .style(move |_| container::Style {
                            background: Some(Background::Color(p.surface)),
                            border: Border {
                                color: p.line2,
                                width: 1.0,
                                radius: theme::radius::CARD.into(),
                            },
                            ..plain()
                        }),
                );
            }
        }

        container(col)
            .padding(28)
            .width(Length::Fill)
            .style(move |_| theme::tq_container(&tp()))
            .into()
    }
}

fn option_filled(p: &Palette, bg: Color, border: Color) -> button::Style {
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: p.text,
        border: Border {
            color: border,
            width: 1.0,
            radius: theme::radius::TIMED.into(),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

/// One results tile (serif value + small label, §10.3).
fn result_tile<'a>(value: String, label: &'a str, color: Color) -> Element<'a, Msg> {
    container(
        column![
            txt(value)
                .size(theme::text::BODY * 1.6)
                .font(SERIF_BOLD)
                .color(color),
            txt(label).size(theme::text::SMALL).color(tp().muted),
        ]
        .spacing(2)
        .align_x(Alignment::Center),
    )
    .padding([12.0, 8.0])
    .width(Length::FillPortion(1))
    .style(move |_| container::Style {
        background: Some(Background::Color(tp().surface)),
        border: Border {
            color: tp().line2,
            width: 1.0,
            radius: theme::radius::SMALL.into(),
        },
        ..plain()
    })
    .into()
}
