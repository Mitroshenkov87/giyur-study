//! Progress screen (PARITY §11): KPI tiles, «Цель и серия» card with the
//! 7-day XP bars, the 20-badge grid, the 28-day activity strip, per-section
//! progress rows, exam history with a line chart, and weak topics.

use std::collections::HashMap;

use chrono::Datelike;
use iced::alignment::{Horizontal, Vertical};
use iced::widget::canvas::{self, Canvas, Frame, Path, Stroke};
use iced::widget::text::{IntoFragment, Shaping};
use iced::widget::{
    button, column, container, image, progress_bar, row, space, stack, text, tooltip, Column, Row,
    Text,
};
use iced::{
    Alignment, Background, Border, Color, Element, Length, Point, Rectangle, Renderer, Shadow,
    Theme,
};

use crate::content::Library;
use crate::game;
use crate::ui::app::{Action, Ctx, Route};
use crate::ui::fonts::{SANS, SANS_BOLD, SERIF_BOLD};
use crate::ui::theme::{self, Palette};
use crate::ui::{assets, widgets};
use crate::util;

fn palette() -> Palette {
    theme::current()
}

fn alpha(color: Color, a: f32) -> Color {
    Color { a, ..color }
}

/// Plain text with advanced shaping (Cyrillic + Hebrew).
fn txt<'a>(content: impl IntoFragment<'a>) -> Text<'a> {
    text(content).shaping(Shaping::Advanced)
}

fn plain_container(background: Option<Color>) -> container::Style {
    container::Style {
        text_color: None,
        background: background.map(Background::Color),
        border: Border::default(),
        shadow: Shadow::default(),
        snap: true,
    }
}

fn rounded_box(background: Color, radius: f32) -> container::Style {
    container::Style {
        border: Border {
            radius: radius.into(),
            ..Border::default()
        },
        ..plain_container(Some(background))
    }
}

/// A progress-bar style with an arbitrary bar colour (gold is the default
/// `theme::progress_bar`; «alt» is blue `#6f8fd0`, «bad» is red — §11.5/7).
fn bar_style(p: &Palette, bar: Color) -> iced::widget::progress_bar::Style {
    iced::widget::progress_bar::Style {
        background: Background::Color(p.surface3),
        bar: Background::Color(bar),
        border: Border {
            radius: theme::radius::PILL.into(),
            ..Border::default()
        },
    }
}

// ---------------------------------------------------------------------------
// Canvas icons (emoji-free replacements: flame, lock, goal ring, chart)
// ---------------------------------------------------------------------------

/// Small flame glyph (§11.2 streak block). Lit = flame colour, else muted.
struct Flame {
    lit: bool,
}

impl<Message> canvas::Program<Message> for Flame {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let p = palette();
        let mut frame = Frame::new(renderer, bounds.size());
        let w = bounds.width;
        let h = bounds.height;
        let color = if self.lit {
            p.flame
        } else {
            alpha(p.muted, 0.55)
        };
        let pt = |x: f32, y: f32| Point::new(x * w, y * h);
        let flame = Path::new(|b| {
            b.move_to(pt(0.50, 0.02));
            b.bezier_curve_to(pt(0.58, 0.22), pt(0.86, 0.30), pt(0.90, 0.58));
            b.bezier_curve_to(pt(0.92, 0.80), pt(0.74, 0.98), pt(0.50, 0.98));
            b.bezier_curve_to(pt(0.26, 0.98), pt(0.08, 0.80), pt(0.10, 0.58));
            b.bezier_curve_to(pt(0.12, 0.36), pt(0.34, 0.26), pt(0.38, 0.10));
            b.bezier_curve_to(pt(0.42, 0.18), pt(0.46, 0.22), pt(0.50, 0.02));
        });
        frame.fill(&flame, color);
        // Inner highlight.
        let inner = Path::new(|b| {
            b.move_to(pt(0.50, 0.44));
            b.bezier_curve_to(pt(0.66, 0.58), pt(0.70, 0.68), pt(0.68, 0.78));
            b.bezier_curve_to(pt(0.66, 0.90), pt(0.58, 0.96), pt(0.50, 0.96));
            b.bezier_curve_to(pt(0.42, 0.96), pt(0.34, 0.90), pt(0.32, 0.78));
            b.bezier_curve_to(pt(0.30, 0.66), pt(0.40, 0.56), pt(0.50, 0.44));
        });
        frame.fill(
            &inner,
            if self.lit {
                alpha(Color::WHITE, 0.35)
            } else {
                alpha(p.surface, 0.6)
            },
        );
        vec![frame.into_geometry()]
    }
}

/// Small padlock glyph for locked badges (§11.3).
struct LockIcon;

impl<Message> canvas::Program<Message> for LockIcon {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let p = palette();
        let mut frame = Frame::new(renderer, bounds.size());
        let w = bounds.width;
        let h = bounds.height;
        let c = p.muted;
        // Shackle: open arc over the body.
        let shackle = Path::new(|b| {
            b.arc(canvas::path::Arc {
                center: Point::new(w / 2.0, h * 0.42),
                radius: w * 0.26,
                start_angle: iced::Radians(std::f32::consts::PI),
                end_angle: iced::Radians(std::f32::consts::TAU),
            });
        });
        frame.stroke(
            &shackle,
            Stroke {
                style: canvas::Style::Solid(c),
                width: (w * 0.12).max(1.2),
                line_cap: canvas::LineCap::Round,
                ..Stroke::default()
            },
        );
        // Body.
        let body = Path::rounded_rectangle(
            Point::new(w * 0.14, h * 0.42),
            iced::Size::new(w * 0.72, h * 0.5),
            (w * 0.12).into(),
        );
        frame.fill(&body, c);
        vec![frame.into_geometry()]
    }
}

/// Goal ring 76 px (§11.2): track + arc; ok colour and a check mark when the
/// daily goal is reached. The center number is overlaid by the caller.
struct GoalRing {
    frac: f32,
    done: bool,
}

impl<Message> canvas::Program<Message> for GoalRing {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let p = palette();
        let mut frame = Frame::new(renderer, bounds.size());
        let size = bounds.size();
        let min = size.width.min(size.height);
        let stroke_width = min * 0.09;
        let radius = (min - stroke_width) / 2.0;
        let center = Point::new(size.width / 2.0, size.height / 2.0);

        let track = Path::circle(center, radius);
        frame.stroke(
            &track,
            Stroke {
                style: canvas::Style::Solid(p.surface3),
                width: stroke_width,
                line_cap: canvas::LineCap::Round,
                ..Stroke::default()
            },
        );

        let accent = if self.done { p.ok } else { p.gold };
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
                    style: canvas::Style::Solid(accent),
                    width: stroke_width,
                    line_cap: canvas::LineCap::Round,
                    ..Stroke::default()
                },
            );
        }

        if self.done {
            // Check mark in the centre.
            let s = min;
            let check = Path::new(|b| {
                b.move_to(Point::new(center.x - s * 0.16, center.y + s * 0.01));
                b.line_to(Point::new(center.x - s * 0.03, center.y + s * 0.13));
                b.line_to(Point::new(center.x + s * 0.18, center.y - s * 0.13));
            });
            frame.stroke(
                &check,
                Stroke {
                    style: canvas::Style::Solid(accent),
                    width: stroke_width * 0.8,
                    line_cap: canvas::LineCap::Round,
                    ..Stroke::default()
                },
            );
        }
        vec![frame.into_geometry()]
    }
}

/// Line chart of the last exam results (§11.6): gold area + line, dashed
/// green 80 % line, gold points.
struct Chart {
    pts: Vec<f64>,
}

impl<Message> canvas::Program<Message> for Chart {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let p = palette();
        let mut frame = Frame::new(renderer, bounds.size());
        let n = self.pts.len();
        if n < 2 {
            return vec![frame.into_geometry()];
        }
        let w = bounds.width;
        let h = bounds.height;
        let pad = 24.0;
        let x = |i: usize| pad + i as f32 * (w - 2.0 * pad) / (n - 1) as f32;
        let y = |v: f64| h - pad - (v as f32 / 100.0).clamp(0.0, 1.0) * (h - 2.0 * pad);

        // Dashed 80 % grid line + label.
        let grid = Path::line(Point::new(pad, y(80.0)), Point::new(w - pad, y(80.0)));
        frame.stroke(
            &grid,
            Stroke {
                style: canvas::Style::Solid(alpha(p.ok, 0.8)),
                width: 1.0,
                line_dash: canvas::LineDash {
                    segments: &[6.0, 6.0],
                    offset: 0,
                },
                ..Stroke::default()
            },
        );
        frame.fill_text(canvas::Text {
            content: "80%".to_string(),
            position: Point::new(w - pad, y(80.0) - 3.0),
            color: p.muted,
            size: iced::Pixels(11.0),
            font: SANS,
            align_x: Horizontal::Right.into(),
            align_y: Vertical::Bottom,
            ..canvas::Text::default()
        });

        // Area under the curve.
        let area = Path::new(|b| {
            b.move_to(Point::new(x(0), h - pad));
            for (i, v) in self.pts.iter().enumerate() {
                b.line_to(Point::new(x(i), y(*v)));
            }
            b.line_to(Point::new(x(n - 1), h - pad));
            b.close();
        });
        frame.fill(
            &area,
            canvas::Gradient::Linear(
                canvas::gradient::Linear::new(Point::new(0.0, 0.0), Point::new(0.0, h))
                    .add_stop(0.0, alpha(p.gold, 0.35))
                    .add_stop(1.0, alpha(p.gold, 0.0)),
            ),
        );

        // The line itself.
        let line = Path::new(|b| {
            for (i, v) in self.pts.iter().enumerate() {
                let pt = Point::new(x(i), y(*v));
                if i == 0 {
                    b.move_to(pt);
                } else {
                    b.line_to(pt);
                }
            }
        });
        frame.stroke(
            &line,
            Stroke {
                style: canvas::Style::Solid(p.gold),
                width: 2.0,
                line_cap: canvas::LineCap::Round,
                ..Stroke::default()
            },
        );

        for (i, v) in self.pts.iter().enumerate() {
            let dot = Path::circle(Point::new(x(i), y(*v)), 4.0);
            frame.fill(&dot, p.gold2);
        }
        vec![frame.into_geometry()]
    }
}

// ---------------------------------------------------------------------------
// Precomputed view data
// ---------------------------------------------------------------------------

struct DayBar {
    tip: String,
    label: String,
    frac: f32,
    goal_frac: f32,
    kept: bool,
    frozen: bool,
    is_today: bool,
}

struct SectionRow {
    title: String,
    soon: bool,
    read: usize,
    lessons: usize,
    mastered: usize,
    cards: usize,
    best: Option<f64>,
}

struct BadgeItem {
    id: String,
    title: String,
    text: String,
    earned: bool,
}

struct HistoryItem {
    date: String,
    mode: String,
    section: Option<String>,
    pct: i64,
}

struct WeakItem {
    lesson_id: String,
    lesson: String,
    section: String,
    /// Raw error rate (miss / n) — the sort key; `pct` is only for display.
    rate: f64,
    pct: i64,
}

pub struct Screen {
    streak: i64,
    goal: i64,
    today_xp: i64,
    kept_today: bool,
    freezes: i64,
    xp_total: i64,
    kpi_read: usize,
    kpi_lessons: usize,
    kpi_mastered: usize,
    kpi_exams: usize,
    week: Vec<DayBar>,
    badges: Vec<BadgeItem>,
    badges_got: usize,
    activity: Vec<(String, i64)>,
    sections: Vec<SectionRow>,
    history: Vec<HistoryItem>,
    chart: Vec<f64>,
    weak: Vec<WeakItem>,
    explain_open: bool,
    explain_items: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    ToggleExplain,
    OpenLesson(String),
}

/// js/exam.js E.MODE_TITLES (history entries store the raw mode string).
fn mode_title(mode: &str) -> &str {
    match mode {
        "section" => "Тест раздела",
        "general" => "Общий экзамен",
        "beitdin" => "Пробный бейт-дин",
        "retry" => "Работа над ошибками",
        "timed" => "Экзамен (на время)",
        "selfcheck" => "Самопроверка",
        "rehearsal" => "Репетиция бейт-дина",
        other => other,
    }
}

/// Lesson self-checks and section rehearsals are practice, not exams: they
/// stay out of the exam chart and the exam count (docs/STRUCTURE.md).
fn is_practice(e: &serde_json::Value) -> bool {
    matches!(
        e.get("mode").and_then(|m| m.as_str()),
        Some("selfcheck") | Some("rehearsal")
    )
}

fn fmt_ts(ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|dt| util::fmt_date(dt.with_timezone(&chrono::Local).date_naive()))
        .unwrap_or_default()
}

impl Screen {
    pub fn new(ctx: &mut Ctx, _route: &Route) -> Self {
        let state = &ctx.state;
        let lib = &ctx.lib;
        let today = ctx.today();
        let today_key = ctx.today_key();
        let user = Library::user_level(state);

        let streak = game::streak(state, today);
        let goal = game::goal(state);
        let today_xp = game::today_xp(state, today);
        let kept_today = game::kept(state, &today_key);

        // Per-section rows + KPI totals (exclude «Скоро» sections).
        let mut kpi_read = 0;
        let mut kpi_lessons = 0;
        let mut kpi_mastered = 0;
        let mut sections = Vec::new();
        for sec in &lib.sections {
            if !sec.available {
                sections.push(SectionRow {
                    title: format!("{}. {}", sec.index + 1, sec.title),
                    soon: true,
                    read: 0,
                    lessons: 0,
                    mastered: 0,
                    cards: 0,
                    best: None,
                });
                continue;
            }
            let s = lib.section_stats(sec, user, state);
            kpi_read += s.read;
            kpi_lessons += s.lessons;
            kpi_mastered += s.mastered;
            sections.push(SectionRow {
                title: format!("{}. {}", sec.index + 1, sec.title),
                soon: false,
                read: s.read,
                lessons: s.lessons,
                mastered: s.mastered,
                cards: s.cards,
                best: s.best,
            });
        }

        // XP week bars (last 7 days).
        let mut week = Vec::new();
        let mut max = goal;
        let names = ["вс", "пн", "вт", "ср", "чт", "пт", "сб"];
        let mut raw: Vec<(String, i64)> = Vec::new();
        for i in (0..7).rev() {
            let d = util::add_days(&today_key, -i);
            let v = state.xp.get(&d).copied().unwrap_or(0);
            max = max.max(v);
            raw.push((d, v));
        }
        let max = max.max(1) as f32;
        for (i, (d, v)) in raw.into_iter().enumerate() {
            let frozen = state.freeze_days.get(&d).copied().unwrap_or(false);
            let weekday = util::parse_day(&d)
                .map(|dt| names[dt.weekday().num_days_from_sunday() as usize])
                .unwrap_or("");
            week.push(DayBar {
                tip: format!("{d}: {v} XP{}", if frozen { " (заморозка)" } else { "" }),
                label: weekday.to_string(),
                frac: v as f32 / max,
                goal_frac: goal as f32 / max,
                kept: game::kept(state, &d),
                frozen,
                is_today: i == 6,
            });
        }

        // Badges.
        let mut badges = Vec::new();
        let mut badges_got = 0;
        for b in game::badge_list(lib) {
            let earned = state.badges.contains_key(&b.id);
            if earned {
                badges_got += 1;
            }
            badges.push(BadgeItem {
                id: b.id,
                title: b.title,
                text: b.text,
                earned,
            });
        }

        // Activity strip (28 days).
        let mut activity = Vec::new();
        for i in (0..28).rev() {
            let d = util::add_days(&today_key, -i);
            let v = state.activity.get(&d).copied().unwrap_or(0);
            activity.push((d, v));
        }

        // Exam history (last 20) + chart points (chronological).
        let last: Vec<&serde_json::Value> = state
            .exams
            .iter()
            .rev()
            .filter(|e| !is_practice(e))
            .take(20)
            .collect();
        let mut history = Vec::new();
        let mut chart = Vec::new();
        for e in last.iter().rev() {
            chart.push(e.get("pct").and_then(|v| v.as_f64()).unwrap_or(0.0));
        }
        for e in last {
            let mode = e.get("mode").and_then(|v| v.as_str()).unwrap_or("");
            let section = e
                .get("sectionId")
                .and_then(|v| v.as_str())
                .and_then(|id| lib.section(id))
                .map(|s| s.title.clone());
            history.push(HistoryItem {
                date: fmt_ts(e.get("at").and_then(|v| v.as_i64()).unwrap_or(0)),
                mode: mode_title(mode).to_string(),
                section,
                pct: e.get("pct").and_then(|v| v.as_i64()).unwrap_or(0),
            });
        }

        // Weak topics: error rate per lesson from cards + questions
        // (js/ui/progress.js weakTopics).
        let mut agg: HashMap<String, (i64, f64)> = HashMap::new();
        let mut add = |lesson_id: &str, n: i64, miss: f64| {
            let a = agg.entry(lesson_id.to_string()).or_insert((0, 0.0));
            a.0 += n;
            a.1 += miss;
        };
        for (id, c) in &state.cards {
            if let Some((_, lesson, _)) = lib.card(id) {
                add(&lesson.id.clone(), c.n, c.miss as f64);
            }
        }
        for (id, q) in &state.questions {
            if let Some((question, _)) = lib.question(id) {
                if let Some(lid) = question.common().lesson_id.clone() {
                    if lib.lesson(&lid).is_some() {
                        add(&lid, q.n, q.miss);
                    }
                }
            }
        }
        let mut weak: Vec<WeakItem> = agg
            .into_iter()
            .filter(|(_, (n, miss))| *n >= 3 && *miss > 0.0)
            .filter_map(|(id, (n, miss))| {
                let (lesson, sec) = lib.lesson(&id)?;
                Some(WeakItem {
                    lesson_id: id,
                    lesson: lesson.title.clone().unwrap_or_else(|| sec.title.clone()),
                    section: sec.title.clone(),
                    rate: miss / n as f64,
                    pct: util::pct(miss / n as f64),
                })
            })
            .collect();
        // Raw rate first (rounded pct would tie randomly: `agg` is a
        // HashMap), lesson id as a stable tiebreaker.
        weak.sort_by(|a, b| {
            b.rate
                .partial_cmp(&a.rate)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.lesson_id.cmp(&b.lesson_id))
        });
        weak.truncate(5);

        // Collapsible «Как работают цель, серия и заморозки» (§11.2).
        let goal_title = game::GOALS
            .iter()
            .find(|g| g.xp == goal)
            .map(|g| g.title)
            .unwrap_or("");
        let explain_items = vec![
            "**XP** начисляются за учёбу: урок прочитан — 10, каждая оценённая карточка — 1 («Не вспомнил» тоже: важна работа), колода пройдена — 5, тест раздела сдан — 20 (+10 за 100%), экзамен — 2 за каждый верный ответ, пробный бейт-дин — 15, самопроверка, репетиция бейт-дина и работа над ошибками — 1 за каждый верный ответ.".to_string(),
            format!(
                "**Цель дня** — {} XP ({}). Её можно изменить в настройках: 10, 20, 30 или 50 XP. Сегодня набрано {} XP.",
                goal, goal_title, today_xp
            ),
            "**Серия** — сколько дней подряд выполнена цель дня. Если сегодня цель ещё не выполнена, серия не сгорает до конца дня.".to_string(),
            "**Заморозка серии** — за каждые 7 дней серии вы получаете одну (хранится не больше двух). Если вы пропустите день, заморозка спишется автоматически и серия сохранится.".to_string(),
        ];

        Screen {
            streak,
            goal,
            today_xp,
            kept_today,
            freezes: state.freezes.clamp(0, game::MAX_FREEZES),
            xp_total: state.xp_total,
            kpi_read,
            kpi_lessons,
            kpi_mastered,
            kpi_exams: state.exams.iter().filter(|e| !is_practice(e)).count(),
            week,
            badges,
            badges_got,
            activity,
            sections,
            history,
            chart,
            weak,
            explain_open: false,
            explain_items,
        }
    }

    pub fn update(&mut self, _ctx: &mut Ctx, msg: Msg) -> Action {
        match msg {
            Msg::ToggleExplain => {
                self.explain_open = !self.explain_open;
                Action::None
            }
            Msg::OpenLesson(id) => Action::Go(Route::Lesson(id)),
        }
    }

    pub fn subscription(&self) -> iced::Subscription<Msg> {
        iced::Subscription::none()
    }

    pub fn view<'a>(&'a self, _ctx: &'a Ctx) -> Element<'a, Msg> {
        let p = palette();
        column![
            column![widgets::kicker("Ваш путь"), widgets::heading("Прогресс", 1),].spacing(6),
            self.kpis(p),
            self.game_card(p),
            self.badges_card(p),
            self.activity_card(p),
            self.sections_card(p),
            self.history_card(p),
            self.weak_card(p),
        ]
        .spacing(16)
        .width(Length::Fill)
        .into()
    }

    // ---- Cards -------------------------------------------------------------

    fn kpis<'a>(&'a self, p: Palette) -> Element<'a, Msg> {
        let tile = |value: String, label: String| -> Element<'a, Msg> {
            container(
                column![
                    txt(value)
                        .size(theme::text::BODY * 1.6)
                        .font(SERIF_BOLD)
                        .color(p.gold2),
                    txt(label)
                        .size(theme::text::SMALL)
                        .color(p.muted)
                        .align_x(Horizontal::Center)
                        .width(Length::Fill),
                ]
                .spacing(4)
                .align_x(Horizontal::Center)
                .width(Length::Fill),
            )
            .padding([14.0, 10.0])
            .width(Length::FillPortion(1))
            .height(Length::Fill)
            .align_y(Vertical::Center)
            .style(move |_| theme::card(&p))
            .into()
        };
        row![
            tile(
                self.streak.to_string(),
                format!(
                    "{} подряд",
                    util::plural(self.streak, &["день", "дня", "дней"])
                ),
            ),
            tile(
                format!("{}/{}", self.kpi_read, self.kpi_lessons),
                "уроков прочитано".to_string(),
            ),
            tile(
                self.kpi_mastered.to_string(),
                "карточек выучено (интервал 30 дней)".to_string(),
            ),
            tile(
                self.kpi_exams.to_string(),
                format!(
                    "{} пройдено",
                    util::plural(self.kpi_exams as i64, &["экзамен", "экзамена", "экзаменов"])
                ),
            ),
        ]
        .spacing(12)
        .into()
    }

    fn game_card<'a>(&'a self, p: Palette) -> Element<'a, Msg> {
        let goal_frac = if self.goal > 0 {
            (self.today_xp as f32 / self.goal as f32).min(1.0)
        } else {
            0.0
        };
        let ring: Element<'a, Msg> = stack![
            Canvas::new(GoalRing {
                frac: goal_frac,
                done: self.kept_today,
            })
            .width(Length::Fixed(76.0))
            .height(Length::Fixed(76.0)),
            container(if self.kept_today {
                txt("").size(10.0)
            } else {
                txt(self.today_xp.to_string())
                    .size(16.0)
                    .font(SANS_BOLD)
                    .color(p.text)
            })
            .center(Length::Fixed(76.0)),
        ]
        .into();

        let streak_block = row![
            Canvas::new(Flame {
                lit: self.kept_today,
            })
            .width(Length::Fixed(26.0))
            .height(Length::Fixed(32.0)),
            column![
                txt(self.streak.to_string())
                    .size(26.0)
                    .font(SERIF_BOLD)
                    .color(p.text),
                txt(format!(
                    "{} подряд",
                    util::plural(self.streak, &["день", "дня", "дней"])
                ))
                .size(theme::text::SMALL)
                .color(p.muted),
            ]
            .spacing(0),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let mut freeze_dots = Row::new().spacing(4).align_y(Alignment::Center);
        for i in 0..game::MAX_FREEZES {
            let on = i < self.freezes;
            freeze_dots = freeze_dots.push(
                container(space::horizontal())
                    .width(Length::Fixed(14.0))
                    .height(Length::Fixed(14.0))
                    .style(move |_| container::Style {
                        background: on.then_some(Background::Color(p.gold2)),
                        border: Border {
                            color: if on { p.gold2 } else { p.muted },
                            width: 1.5,
                            radius: theme::radius::PILL.into(),
                        },
                        ..plain_container(None)
                    }),
            );
        }
        let freeze_box = column![
            freeze_dots,
            txt(format!(
                "{} из {} заморозок",
                self.freezes,
                game::MAX_FREEZES
            ))
            .size(theme::text::SMALL)
            .color(p.muted),
        ]
        .spacing(6);

        let xp_total = column![
            txt(self.xp_total.to_string())
                .size(26.0)
                .font(SERIF_BOLD)
                .color(p.text),
            txt("XP всего").size(theme::text::SMALL).color(p.muted),
        ]
        .spacing(0);

        let top_row = row![ring, streak_block, freeze_box, xp_total]
            .spacing(24)
            .align_y(Alignment::Center)
            .wrap();

        widgets::card(
            column![
                widgets::heading("Цель и серия", 3),
                top_row,
                self.xp_week(p),
                self.explainer(p),
            ]
            .spacing(14),
        )
    }

    /// 7 bars of daily XP with the green goal line (§11.2).
    fn xp_week<'a>(&'a self, p: Palette) -> Element<'a, Msg> {
        const H: f32 = 90.0;
        let mut bars = Row::new().spacing(6).width(Length::Fill);
        for d in &self.week {
            let fill_h = if d.frac > 0.0 {
                (H * d.frac).max(3.0)
            } else {
                0.0
            };
            let fill_color = if d.kept { p.gold } else { alpha(p.gold, 0.45) };
            let track = container(space::horizontal())
                .width(Length::Fill)
                .height(H)
                .style(move |_| rounded_box(p.surface3, 4.0));
            let fill = container(
                container(space::horizontal())
                    .width(Length::Fill)
                    .height(Length::Fixed(fill_h))
                    .style(move |_| rounded_box(fill_color, 4.0)),
            )
            .width(Length::Fill)
            .height(H)
            .align_y(Alignment::End);
            let goal_top = (H * (1.0 - d.goal_frac.clamp(0.0, 1.0)) - 1.0).max(0.0);
            let goal_line = container(
                container(space::horizontal())
                    .width(Length::Fill)
                    .height(Length::Fixed(2.0))
                    .style(move |_| rounded_box(p.ok, 1.0)),
            )
            .padding(iced::Padding::default().top(goal_top))
            .width(Length::Fill);

            let bar: Element<'a, Msg> = stack![track, fill, goal_line].into();

            let label: Element<'a, Msg> = if d.frozen {
                // Freeze marker: a small hollow gold circle instead of «❄».
                container(
                    container(space::horizontal())
                        .width(Length::Fixed(9.0))
                        .height(Length::Fixed(9.0))
                        .style(move |_| container::Style {
                            border: Border {
                                color: p.gold2,
                                width: 1.5,
                                radius: theme::radius::PILL.into(),
                            },
                            ..plain_container(None)
                        }),
                )
                .center_x(Length::Fill)
                .into()
            } else {
                txt(d.label.clone())
                    .size(theme::text::SMALL - 1.0)
                    .font(if d.is_today { SANS_BOLD } else { SANS })
                    .color(if d.is_today { p.gold2 } else { p.muted })
                    .align_x(Horizontal::Center)
                    .width(Length::Fill)
                    .into()
            };

            bars = bars.push(
                tooltip(
                    column![bar, label].spacing(6).width(Length::FillPortion(1)),
                    tip(d.tip.clone(), p),
                    tooltip::Position::Top,
                )
                .style(move |_| tip_style(&p)),
            );
        }
        bars.into()
    }

    /// Collapsible «Как работают цель, серия и заморозки» (§11.2).
    fn explainer<'a>(&'a self, p: Palette) -> Element<'a, Msg> {
        let mut col = Column::new().spacing(10).width(Length::Fill);
        let marker = if self.explain_open { "−" } else { "+" };
        col = col.push(
            button(
                row![
                    txt(marker).size(theme::text::BODY).color(p.gold),
                    txt("Как работают цель, серия и заморозки")
                        .size(theme::text::SMALL + 0.5)
                        .font(SANS_BOLD)
                        .color(p.gold2),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .padding(0)
            .style(|_, _| iced::widget::button::Style::default())
            .on_press(Msg::ToggleExplain),
        );
        if self.explain_open {
            let mut list = Column::new().spacing(8).width(Length::Fill);
            for item in &self.explain_items {
                list = list.push(
                    row![
                        txt("•").size(theme::text::BODY).color(p.gold),
                        widgets::rich_paragraph(item),
                    ]
                    .spacing(8),
                );
            }
            col = col.push(list);
        }
        col.into()
    }

    fn badges_card<'a>(&'a self, p: Palette) -> Element<'a, Msg> {
        let mut grid = Column::new().spacing(10).width(Length::Fill);
        for chunk in self.badges.chunks(4) {
            let mut r = Row::new().spacing(10).width(Length::Fill);
            for b in chunk {
                r = r.push(badge_cell(b, p));
            }
            for _ in chunk.len()..4 {
                r = r.push(space::horizontal().width(Length::FillPortion(1)));
            }
            grid = grid.push(r);
        }
        widgets::card(
            column![
                row![
                    widgets::heading("Значки", 3),
                    txt(format!("· {} из {}", self.badges_got, self.badges.len()))
                        .size(theme::text::SMALL)
                        .color(p.muted),
                ]
                .spacing(8)
                .align_y(Alignment::End),
                grid,
            ]
            .spacing(14),
        )
    }

    fn activity_card<'a>(&'a self, p: Palette) -> Element<'a, Msg> {
        let mut rows = Column::new().spacing(4);
        for chunk in self.activity.chunks(14) {
            let mut r = Row::new().spacing(4);
            for (day, v) in chunk {
                let lvl = if *v == 0 {
                    0
                } else if *v < 10 {
                    1
                } else if *v < 30 {
                    2
                } else {
                    3
                };
                let color = match lvl {
                    0 => p.surface3,
                    1 => alpha(p.gold, 0.35),
                    2 => alpha(p.gold, 0.65),
                    _ => p.gold2,
                };
                let square = container(space::horizontal())
                    .width(Length::Fixed(14.0))
                    .height(Length::Fixed(14.0))
                    .style(move |_| rounded_box(color, 3.5));
                r = r.push(
                    tooltip(
                        square,
                        tip(format!("{day}: {v}"), p),
                        tooltip::Position::Top,
                    )
                    .style(move |_| tip_style(&p)),
                );
            }
            rows = rows.push(r);
        }
        widgets::card(column![widgets::heading("Активность за 4 недели", 3), rows,].spacing(14))
    }

    fn sections_card<'a>(&'a self, p: Palette) -> Element<'a, Msg> {
        let mut list = Column::new().spacing(12).width(Length::Fill);
        for s in &self.sections {
            if s.soon {
                list = list.push(
                    row![
                        txt(s.title.clone())
                            .size(theme::text::BODY)
                            .color(alpha(p.text, 0.55))
                            .width(Length::Fixed(220.0)),
                        widgets::tag("Скоро"),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                );
                continue;
            }
            let lessons_pct = if s.lessons > 0 {
                s.read as f32 / s.lessons as f32
            } else {
                0.0
            };
            let cards_pct = if s.cards > 0 {
                s.mastered as f32 / s.cards as f32
            } else {
                0.0
            };
            let best = match s.best {
                Some(b) => txt(format!("{}%", b.round() as i64))
                    .size(theme::text::BODY)
                    .font(SANS_BOLD)
                    .color(p.gold2),
                None => txt("—").size(theme::text::BODY).color(p.muted),
            };
            list = list.push(
                row![
                    txt(s.title.clone())
                        .size(theme::text::BODY)
                        .width(Length::Fixed(220.0)),
                    column![
                        row![
                            txt(format!("Уроки {}/{}", s.read, s.lessons))
                                .size(theme::text::SMALL - 1.0)
                                .color(p.muted)
                                .width(Length::Fixed(110.0)),
                            progress_bar(0.0..=1.0, lessons_pct)
                                .girth(6.0)
                                .style(move |_| theme::progress_bar(&palette())),
                        ]
                        .spacing(8)
                        .align_y(Alignment::Center),
                        row![
                            txt(format!("Карточки {}/{}", s.mastered, s.cards))
                                .size(theme::text::SMALL - 1.0)
                                .color(p.muted)
                                .width(Length::Fixed(110.0)),
                            progress_bar(0.0..=1.0, cards_pct)
                                .girth(6.0)
                                .style(move |_| {
                                    bar_style(&palette(), Color::from_rgb8(0x6f, 0x8f, 0xd0))
                                }),
                        ]
                        .spacing(8)
                        .align_y(Alignment::Center),
                    ]
                    .spacing(6)
                    .width(Length::Fill),
                    container(best)
                        .width(Length::Fixed(52.0))
                        .align_x(Alignment::End),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            );
        }
        widgets::card(
            column![
                row![
                    widgets::heading("По разделам", 3),
                    txt("· лучший тест").size(theme::text::SMALL).color(p.muted),
                ]
                .spacing(8)
                .align_y(Alignment::End),
                list,
            ]
            .spacing(14),
        )
    }

    fn history_card<'a>(&'a self, p: Palette) -> Element<'a, Msg> {
        let mut body = Column::new().spacing(12).width(Length::Fill);
        if self.history.is_empty() {
            body = body.push(
                txt("Вы ещё не проходили экзамены.")
                    .size(theme::text::BODY)
                    .color(p.muted),
            );
        } else {
            if self.chart.len() >= 2 {
                body = body.push(
                    Canvas::new(Chart {
                        pts: self.chart.clone(),
                    })
                    .width(Length::Fill)
                    .height(Length::Fixed(160.0)),
                );
            }
            let mut list = Column::new().spacing(8).width(Length::Fill);
            for h in &self.history {
                let ok = h.pct >= 80;
                let pill_color = if ok { p.ok } else { p.bad };
                let pill_bg = if ok { p.ok_soft } else { p.bad_soft };
                let what = match &h.section {
                    Some(s) => format!("{} · {}", h.mode, s),
                    None => h.mode.clone(),
                };
                list = list.push(
                    row![
                        txt(h.date.clone())
                            .size(theme::text::SMALL)
                            .color(p.muted)
                            .width(Length::Fixed(90.0)),
                        txt(what).size(theme::text::SMALL).width(Length::Fill),
                        container(
                            txt(format!("{}%", h.pct))
                                .size(theme::text::SMALL - 1.0)
                                .font(SANS_BOLD)
                                .color(pill_color),
                        )
                        .padding([0.0, 10.0])
                        .height(Length::Fixed(24.0))
                        .center_y(Length::Fixed(24.0))
                        .style(move |_| container::Style {
                            border: Border {
                                radius: theme::radius::PILL.into(),
                                ..Border::default()
                            },
                            ..plain_container(Some(pill_bg))
                        }),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                );
            }
            body = body.push(list);
        }
        widgets::card(column![widgets::heading("История экзаменов", 3), body].spacing(14))
    }

    fn weak_card<'a>(&'a self, p: Palette) -> Element<'a, Msg> {
        let mut body = Column::new().spacing(10).width(Length::Fill);
        if self.weak.is_empty() {
            body = body.push(
                txt("Пока недостаточно данных: решайте тесты и повторяйте карточки.")
                    .size(theme::text::BODY)
                    .color(p.muted),
            );
        } else {
            for w in &self.weak {
                let link = button(
                    column![
                        txt(w.lesson.clone())
                            .size(theme::text::BODY)
                            .font(SANS_BOLD)
                            .color(p.text),
                        txt(w.section.clone())
                            .size(theme::text::SMALL)
                            .color(p.muted),
                    ]
                    .spacing(2),
                )
                .padding([6.0, 10.0])
                .width(Length::Fixed(260.0))
                .style(move |_, s| theme::button_ghost(&palette(), s))
                .on_press(Msg::OpenLesson(w.lesson_id.clone()));
                body = body.push(
                    row![
                        link,
                        progress_bar(0.0..=1.0, w.pct as f32 / 100.0)
                            .girth(6.0)
                            .style(move |_| bar_style(&palette(), palette().bad)),
                        txt(format!("{}% ошибок", w.pct))
                            .size(theme::text::SMALL)
                            .color(p.bad)
                            .width(Length::Fixed(90.0)),
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center),
                );
            }
        }
        widgets::card(column![widgets::heading("Слабые темы", 3), body].spacing(14))
    }
}

// ---------------------------------------------------------------------------
// Small builders
// ---------------------------------------------------------------------------

/// One badge in the grid (§11.3): 64 px emblem, title, text; locked badges
/// are dimmed and show a padlock.
fn badge_cell<'a>(b: &'a BadgeItem, p: Palette) -> Element<'a, Msg> {
    let emblem: Element<'a, Msg> = match assets::img(&format!("badge-{}", b.id)) {
        Some(handle) => container(
            image(handle)
                .width(Length::Fixed(64.0))
                .height(Length::Fixed(64.0))
                .opacity(if b.earned { 1.0_f32 } else { 0.35_f32 }),
        )
        .clip(true)
        .style(move |_| container::Style {
            border: Border {
                color: alpha(p.gold, 0.4),
                width: 1.0,
                radius: theme::radius::PILL.into(),
            },
            ..plain_container(None)
        })
        .into(),
        None => container(
            txt(b.title.chars().next().unwrap_or('?').to_string())
                .size(24.0)
                .font(SERIF_BOLD)
                .color(p.gold2),
        )
        .width(Length::Fixed(64.0))
        .height(Length::Fixed(64.0))
        .center(Length::Fixed(64.0))
        .style(move |_| container::Style {
            border: Border {
                color: alpha(p.gold, 0.4),
                width: 1.0,
                radius: theme::radius::PILL.into(),
            },
            ..plain_container(Some(p.surface3))
        })
        .into(),
    };

    let mut content = Column::new()
        .spacing(6)
        .align_x(Horizontal::Center)
        .width(Length::Fill);
    content = content.push(container(emblem).center_x(Length::Fill));
    content = content.push(
        txt(b.title.clone())
            .size(theme::text::SMALL + 0.5)
            .font(SANS_BOLD)
            .color(if b.earned { p.text } else { p.muted })
            .align_x(Horizontal::Center)
            .width(Length::Fill),
    );
    content = content.push(
        txt(b.text.clone())
            .size(theme::text::SMALL - 1.5)
            .color(p.muted)
            .align_x(Horizontal::Center)
            .width(Length::Fill)
            .wrapping(iced::widget::text::Wrapping::WordOrGlyph),
    );

    let earned = b.earned;
    let body: Element<'a, Msg> = container(content)
        .padding(12.0)
        .width(Length::Fill)
        .height(Length::Fixed(150.0))
        .style(move |_| container::Style {
            background: Some(Background::Color(if earned {
                p.surface2
            } else {
                p.surface
            })),
            border: Border {
                color: if earned { alpha(p.gold, 0.6) } else { p.line },
                width: 1.0,
                radius: theme::radius::EMBLEM.into(),
            },
            shadow: if earned {
                Shadow {
                    color: alpha(p.gold, 0.18),
                    offset: iced::Vector::new(0.0, 2.0),
                    blur_radius: 14.0,
                }
            } else {
                Shadow::default()
            },
            text_color: None,
            snap: true,
        })
        .into();

    if earned {
        body
    } else {
        stack![
            body,
            container(
                Canvas::new(LockIcon)
                    .width(Length::Fixed(14.0))
                    .height(Length::Fixed(14.0))
            )
            .width(Length::Fill)
            .align_x(Alignment::End)
            .padding(iced::Padding::default().top(8.0).right(8.0)),
        ]
        .width(Length::FillPortion(1))
        .into()
    }
}

/// Tooltip bubble content.
fn tip<'a>(content: String, p: Palette) -> Element<'a, Msg> {
    container(txt(content).size(12.0).color(p.text))
        .padding([6.0, 10.0])
        .into()
}

fn tip_style(p: &Palette) -> container::Style {
    container::Style {
        background: Some(Background::Color(p.surface2)),
        border: Border {
            color: p.line2,
            width: 1.0,
            radius: 10.0.into(),
        },
        shadow: Shadow {
            color: alpha(Color::BLACK, 0.3),
            offset: iced::Vector::new(0.0, 4.0),
            blur_radius: 12.0,
        },
        text_color: None,
        snap: true,
    }
}
