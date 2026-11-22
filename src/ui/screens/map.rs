//! Map / home screen (PARITY §3): header with level chip, goal ring, streak
//! and due stats, reminder banners, timed-exam CTA and the vertical path of
//! section nodes. Also hosts the small canvas icon/emblem helpers shared by
//! the sibling screens (welcome, level, section, sources).

use iced::widget::{button, column, container, row, space, stack, text, Column};
use iced::{Alignment, Background, Border, Color, Element, Length};

use crate::content::{self, Library, SectionMeta};
use crate::game;
use crate::ui::app::{Action, Ctx, Deck, Route};
use crate::ui::{fonts, theme, widgets};
use crate::util;

// ---------------------------------------------------------------------------
// Shared canvas icons (stroke icons from js/ui/core.js, drawn on a 24×24 grid)
// ---------------------------------------------------------------------------

pub(crate) mod icons {
    use iced::mouse;
    use iced::widget::canvas::{self, Canvas, Frame, Path, Stroke};
    use iced::{Color, Element, Length, Point, Radians, Rectangle, Renderer, Size, Theme};

    use crate::ui::theme;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Kind {
        Book,
        Cards,
        Exam,
        Sources,
        Lock,
        Check,
        ChevronDown,
        ChevronLeft,
        Flame,
        Star,
        StarDavid,
        Clock,
        Review,
        External,
        Snow,
        Timer,
        /// Crossing arrows: mixed questions (general exam).
        Shuffle,
        /// Balance scales: the beit din.
        Scales,
    }

    struct Icon {
        kind: Kind,
        color: Color,
    }

    fn stroke(color: Color, width: f32) -> Stroke<'static> {
        Stroke {
            style: canvas::Style::Solid(color),
            width,
            line_cap: canvas::LineCap::Round,
            line_join: canvas::LineJoin::Round,
            ..Stroke::default()
        }
    }

    impl<Message> canvas::Program<Message> for Icon {
        type State = ();

        fn draw(
            &self,
            _state: &Self::State,
            renderer: &Renderer,
            _theme: &Theme,
            bounds: Rectangle,
            _cursor: mouse::Cursor,
        ) -> Vec<canvas::Geometry> {
            let mut frame = Frame::new(renderer, bounds.size());
            let size = bounds.size();
            let s = size.width.min(size.height) / 24.0;
            let ox = (size.width - 24.0 * s) / 2.0;
            let oy = (size.height - 24.0 * s) / 2.0;
            let pt = |x: f32, y: f32| Point::new(ox + x * s, oy + y * s);
            let st = stroke(self.color, 1.8 * s);

            let path = |build: &dyn Fn(&mut canvas::path::Builder)| Path::new(|b| build(b));
            let line = |b: &mut canvas::path::Builder, x1: f32, y1: f32, x2: f32, y2: f32| {
                b.move_to(pt(x1, y1));
                b.line_to(pt(x2, y2));
            };

            match self.kind {
                Kind::Check => {
                    frame.stroke(
                        &path(&|b| {
                            b.move_to(pt(5.0, 12.0));
                            b.line_to(pt(10.0, 17.0));
                            b.line_to(pt(19.0, 7.0));
                        }),
                        st,
                    );
                }
                Kind::ChevronDown => {
                    frame.stroke(
                        &path(&|b| {
                            b.move_to(pt(6.0, 9.0));
                            b.line_to(pt(12.0, 15.0));
                            b.line_to(pt(18.0, 9.0));
                        }),
                        st,
                    );
                }
                Kind::ChevronLeft => {
                    frame.stroke(
                        &path(&|b| {
                            b.move_to(pt(15.0, 6.0));
                            b.line_to(pt(9.0, 12.0));
                            b.line_to(pt(15.0, 18.0));
                        }),
                        st,
                    );
                }
                Kind::Lock => {
                    frame.stroke(
                        &Path::rounded_rectangle(
                            Point::new(ox + 5.0 * s, oy + 11.0 * s),
                            Size::new(14.0 * s, 10.0 * s),
                            (2.0 * s).into(),
                        ),
                        st,
                    );
                    frame.stroke(
                        &path(&|b| {
                            b.move_to(pt(8.0, 11.0));
                            b.line_to(pt(8.0, 7.0));
                            b.arc(canvas::path::Arc {
                                center: pt(12.0, 7.0),
                                radius: 4.0 * s,
                                start_angle: Radians(std::f32::consts::PI),
                                end_angle: Radians(std::f32::consts::TAU),
                            });
                            b.line_to(pt(16.0, 11.0));
                        }),
                        st,
                    );
                }
                Kind::Clock => {
                    frame.stroke(&Path::circle(pt(12.0, 12.0), 9.0 * s), st);
                    frame.stroke(
                        &path(&|b| {
                            b.move_to(pt(12.0, 7.0));
                            b.line_to(pt(12.0, 12.0));
                            b.line_to(pt(15.0, 14.0));
                        }),
                        st,
                    );
                }
                Kind::Review => {
                    frame.stroke(
                        &path(&|b| {
                            b.arc(canvas::path::Arc {
                                center: pt(12.0, 12.0),
                                radius: 8.0 * s,
                                start_angle: Radians(3.6),
                                end_angle: Radians(3.6 + 5.3),
                            });
                        }),
                        st,
                    );
                    frame.stroke(
                        &path(&|b| {
                            b.move_to(pt(4.0, 4.0));
                            b.line_to(pt(4.0, 9.0));
                            b.line_to(pt(9.0, 9.0));
                        }),
                        st,
                    );
                    frame.stroke(
                        &path(&|b| {
                            b.move_to(pt(12.0, 8.0));
                            b.line_to(pt(12.0, 12.0));
                            b.line_to(pt(15.0, 14.0));
                        }),
                        st,
                    );
                }
                Kind::Flame => {
                    frame.stroke(
                        &path(&|b| {
                            b.move_to(pt(12.0, 3.0));
                            b.bezier_curve_to(pt(13.5, 7.0), pt(17.0, 8.5), pt(17.0, 13.0));
                            b.bezier_curve_to(pt(17.0, 16.5), pt(14.8, 18.5), pt(12.0, 18.5));
                            b.bezier_curve_to(pt(9.2, 18.5), pt(7.0, 16.5), pt(7.0, 13.0));
                            b.bezier_curve_to(pt(7.0, 10.5), pt(8.5, 9.0), pt(9.5, 7.5));
                            b.bezier_curve_to(pt(9.5, 9.5), pt(10.5, 10.8), pt(11.5, 10.8));
                            b.bezier_curve_to(pt(11.0, 8.0), pt(10.8, 6.0), pt(12.0, 3.0));
                        }),
                        st,
                    );
                }
                Kind::Star => {
                    frame.stroke(
                        &path(&|b| {
                            for i in 0..10 {
                                let r = if i % 2 == 0 { 9.0_f32 } else { 3.6_f32 };
                                let a = -std::f32::consts::FRAC_PI_2
                                    + i as f32 * std::f32::consts::PI / 5.0;
                                let point = pt(12.0 + r * a.cos(), 12.0 + r * a.sin());
                                if i == 0 {
                                    b.move_to(point);
                                } else {
                                    b.line_to(point);
                                }
                            }
                            b.close();
                        }),
                        st,
                    );
                }
                Kind::StarDavid => {
                    let tri = |b: &mut canvas::path::Builder, up: bool| {
                        let (top, bl, br) = if up {
                            ((12.0, 3.0), (4.2, 16.5), (19.8, 16.5))
                        } else {
                            ((12.0, 21.0), (4.2, 7.5), (19.8, 7.5))
                        };
                        b.move_to(pt(top.0, top.1));
                        b.line_to(pt(bl.0, bl.1));
                        b.line_to(pt(br.0, br.1));
                        b.close();
                    };
                    frame.stroke(&path(&|b| tri(b, true)), st);
                    frame.stroke(&path(&|b| tri(b, false)), st);
                }
                Kind::External => {
                    frame.stroke(
                        &path(&|b| {
                            line(b, 14.0, 4.0, 20.0, 4.0);
                            line(b, 20.0, 4.0, 20.0, 10.0);
                            line(b, 20.0, 4.0, 11.0, 13.0);
                        }),
                        st,
                    );
                    frame.stroke(
                        &path(&|b| {
                            b.move_to(pt(18.0, 14.0));
                            b.line_to(pt(18.0, 19.0));
                            b.quadratic_curve_to(pt(18.0, 20.0), pt(17.0, 20.0));
                            b.line_to(pt(5.0, 20.0));
                            b.quadratic_curve_to(pt(4.0, 20.0), pt(4.0, 19.0));
                            b.line_to(pt(4.0, 7.0));
                            b.quadratic_curve_to(pt(4.0, 6.0), pt(5.0, 6.0));
                            b.line_to(pt(10.0, 6.0));
                        }),
                        st,
                    );
                }
                Kind::Snow => {
                    frame.stroke(
                        &path(&|b| {
                            line(b, 12.0, 2.0, 12.0, 22.0);
                            line(b, 4.9, 6.5, 19.1, 17.5);
                            line(b, 19.1, 6.5, 4.9, 17.5);
                            b.move_to(pt(9.5, 3.5));
                            b.line_to(pt(12.0, 5.5));
                            b.line_to(pt(14.5, 3.5));
                            b.move_to(pt(9.5, 20.5));
                            b.line_to(pt(12.0, 18.5));
                            b.line_to(pt(14.5, 20.5));
                        }),
                        st,
                    );
                }
                Kind::Timer => {
                    frame.stroke(&Path::circle(pt(12.0, 13.5), 7.5 * s), st);
                    frame.stroke(
                        &path(&|b| {
                            b.move_to(pt(12.0, 9.5));
                            b.line_to(pt(12.0, 13.5));
                            b.line_to(pt(14.5, 15.0));
                            line(b, 10.0, 2.5, 14.0, 2.5);
                            line(b, 12.0, 2.5, 12.0, 6.0);
                            line(b, 18.5, 6.5, 20.0, 5.0);
                        }),
                        st,
                    );
                }
                Kind::Book => {
                    frame.stroke(
                        &path(&|b| {
                            b.move_to(pt(12.0, 6.0));
                            b.bezier_curve_to(pt(10.0, 4.5), pt(7.0, 4.0), pt(4.0, 4.0));
                            b.line_to(pt(4.0, 18.0));
                            b.bezier_curve_to(pt(7.0, 18.0), pt(10.0, 18.5), pt(12.0, 20.0));
                            b.bezier_curve_to(pt(14.0, 18.5), pt(17.0, 18.0), pt(20.0, 18.0));
                            b.line_to(pt(20.0, 4.0));
                            b.bezier_curve_to(pt(17.0, 4.0), pt(14.0, 4.5), pt(12.0, 6.0));
                            b.close();
                            line(b, 12.0, 6.0, 12.0, 20.0);
                        }),
                        st,
                    );
                }
                Kind::Cards => {
                    frame.stroke(
                        &Path::rounded_rectangle(
                            Point::new(ox + 3.0 * s, oy + 6.0 * s),
                            Size::new(14.0 * s, 14.0 * s),
                            (2.0 * s).into(),
                        ),
                        st,
                    );
                    frame.stroke(
                        &path(&|b| {
                            b.move_to(pt(7.0, 3.0));
                            b.line_to(pt(19.0, 3.0));
                            b.quadratic_curve_to(pt(21.0, 3.0), pt(21.0, 5.0));
                            b.line_to(pt(21.0, 17.0));
                        }),
                        st,
                    );
                }
                Kind::Shuffle => {
                    frame.stroke(
                        &path(&|b| {
                            b.move_to(pt(4.0, 7.0));
                            b.line_to(pt(8.0, 7.0));
                            b.line_to(pt(16.0, 17.0));
                            b.line_to(pt(20.0, 17.0));
                            b.move_to(pt(4.0, 17.0));
                            b.line_to(pt(8.0, 17.0));
                            b.line_to(pt(16.0, 7.0));
                            b.line_to(pt(20.0, 7.0));
                            line(b, 17.0, 4.0, 20.0, 7.0);
                            line(b, 20.0, 7.0, 17.0, 10.0);
                            line(b, 17.0, 14.0, 20.0, 17.0);
                            line(b, 20.0, 17.0, 17.0, 20.0);
                        }),
                        st,
                    );
                }
                Kind::Scales => {
                    frame.stroke(
                        &path(&|b| {
                            line(b, 12.0, 4.0, 12.0, 20.0);
                            line(b, 8.0, 20.0, 16.0, 20.0);
                            line(b, 5.0, 7.0, 19.0, 7.0);
                            for cx in [5.0_f32, 19.0] {
                                b.move_to(pt(cx, 7.0));
                                b.line_to(pt(cx - 3.0, 13.0));
                                b.move_to(pt(cx, 7.0));
                                b.line_to(pt(cx + 3.0, 13.0));
                                b.move_to(pt(cx - 3.0, 13.0));
                                b.quadratic_curve_to(pt(cx, 17.0), pt(cx + 3.0, 13.0));
                                b.line_to(pt(cx - 3.0, 13.0));
                            }
                        }),
                        st,
                    );
                }
                Kind::Exam => {
                    frame.stroke(
                        &path(&|b| {
                            b.move_to(pt(9.0, 4.0));
                            b.line_to(pt(15.0, 4.0));
                            b.line_to(pt(16.0, 6.0));
                            b.line_to(pt(19.0, 6.0));
                            b.line_to(pt(19.0, 20.0));
                            b.line_to(pt(5.0, 20.0));
                            b.line_to(pt(5.0, 6.0));
                            b.line_to(pt(8.0, 6.0));
                            b.close();
                            b.move_to(pt(9.0, 13.0));
                            b.line_to(pt(11.0, 15.0));
                            b.line_to(pt(15.0, 11.0));
                        }),
                        st,
                    );
                }
                Kind::Sources => {
                    frame.stroke(
                        &path(&|b| {
                            b.move_to(pt(4.0, 5.0));
                            b.quadratic_curve_to(pt(4.0, 3.0), pt(6.0, 3.0));
                            b.line_to(pt(19.0, 3.0));
                            b.line_to(pt(19.0, 19.0));
                            b.line_to(pt(6.0, 19.0));
                            b.quadratic_curve_to(pt(4.0, 19.0), pt(4.0, 21.0));
                            b.close();
                            line(b, 8.0, 7.0, 15.0, 7.0);
                            line(b, 8.0, 11.0, 15.0, 11.0);
                        }),
                        st,
                    );
                }
            }
            vec![frame.into_geometry()]
        }
    }

    /// A stroke icon at `size` px.
    pub fn icon<'a, Message: 'a>(kind: Kind, size: f32, color: Color) -> Element<'a, Message> {
        Canvas::new(Icon { kind, color })
            .width(Length::Fixed(size))
            .height(Length::Fixed(size))
            .into()
    }

    struct Ring {
        pct: f32,
        color: Color,
    }

    impl<Message> canvas::Program<Message> for Ring {
        type State = ();

        fn draw(
            &self,
            _state: &Self::State,
            renderer: &Renderer,
            _theme: &Theme,
            bounds: Rectangle,
            _cursor: mouse::Cursor,
        ) -> Vec<canvas::Geometry> {
            let p = theme::current();
            let mut frame = Frame::new(renderer, bounds.size());
            let size = bounds.size();
            let min = size.width.min(size.height);
            let width = min * 0.09;
            let radius = (min - width) / 2.0;
            let center = Point::new(size.width / 2.0, size.height / 2.0);
            frame.stroke(&Path::circle(center, radius), stroke(p.surface3, width));
            let pct = self.pct.clamp(0.0, 1.0);
            if pct > 0.0 {
                let start = -std::f32::consts::FRAC_PI_2;
                frame.stroke(
                    &Path::new(|b| {
                        b.arc(canvas::path::Arc {
                            center,
                            radius,
                            start_angle: Radians(start),
                            end_angle: Radians(start + pct * std::f32::consts::TAU),
                        });
                    }),
                    stroke(self.color, width),
                );
            }
            vec![frame.into_geometry()]
        }
    }

    /// Progress ring of any size (`pct` in 0.0–1.0).
    pub fn ring<'a, Message: 'a>(pct: f32, size: f32, color: Color) -> Element<'a, Message> {
        Canvas::new(Ring { pct, color })
            .width(Length::Fixed(size))
            .height(Length::Fixed(size))
            .into()
    }

    struct LevelRings {
        filled: usize,
        color: Color,
    }

    impl<Message> canvas::Program<Message> for LevelRings {
        type State = ();

        fn draw(
            &self,
            _state: &Self::State,
            renderer: &Renderer,
            _theme: &Theme,
            bounds: Rectangle,
            _cursor: mouse::Cursor,
        ) -> Vec<canvas::Geometry> {
            let mut frame = Frame::new(renderer, bounds.size());
            let size = bounds.size();
            let min = size.width.min(size.height);
            let s = min / 100.0;
            let center = Point::new(size.width / 2.0, size.height / 2.0);
            let dark = Color::from_rgb8(0x0e, 0x13, 0x27);
            for (i, r) in [40.0_f32, 28.0, 16.0].into_iter().enumerate() {
                let circle = Path::circle(center, r * s);
                if i < self.filled {
                    frame.fill(&circle, self.color);
                } else {
                    frame.fill(&circle, dark);
                }
                frame.stroke(&circle, stroke(self.color, 3.0 * s));
            }
            vec![frame.into_geometry()]
        }
    }

    /// Nested-rings fallback for the level emblems (1/2/3 rings filled).
    pub fn level_rings<'a, Message: 'a>(
        filled: usize,
        size: f32,
        color: Color,
    ) -> Element<'a, Message> {
        Canvas::new(LevelRings { filled, color })
            .width(Length::Fixed(size))
            .height(Length::Fixed(size))
            .into()
    }
}

// ---------------------------------------------------------------------------
// Shared emblem helper (section-*.webp with a fallback, js/ui/core.js imgSlot)
// ---------------------------------------------------------------------------

/// Section emblem: `section-{id}.webp` or a fallback (Hebrew letter / Star of
/// David on the emblem gradient, §T.1).
pub(crate) fn emblem<'a, Message: 'a>(
    id: &str,
    icon: &'a str,
    size: f32,
    radius: f32,
    dim: bool,
) -> Element<'a, Message> {
    let p = theme::current();
    if let Some(handle) = crate::ui::assets::img(&format!("section-{id}")) {
        return container(iced::widget::image(handle).width(size).height(size))
            .width(Length::Fixed(size))
            .height(Length::Fixed(size))
            .style(move |_| iced::widget::container::Style {
                border: Border {
                    radius: radius.into(),
                    ..Border::default()
                },
                ..iced::widget::container::Style::default()
            })
            .into();
    }
    let mut chars = icon.chars();
    let single_hebrew = matches!(chars.next(), Some(c) if ('\u{0590}'..='\u{05FF}').contains(&c))
        && chars.next().is_none();
    let inner: Element<'a, Message> = if single_hebrew {
        text(icon)
            .size(size * 0.55)
            .font(fonts::HEBREW)
            .shaping(text::Shaping::Advanced)
            .color(if dim { p.muted } else { p.gold2 })
            .into()
    } else {
        icons::icon(
            icons::Kind::StarDavid,
            size * 0.55,
            if dim { p.muted } else { p.gold2 },
        )
    };
    container(inner)
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .center_x(Length::Fixed(size))
        .center_y(Length::Fixed(size))
        .style(move |_| iced::widget::container::Style {
            background: Some(v_gradient(
                Color::from_rgb8(0x26, 0x30, 0x5a),
                Color::from_rgb8(0x0f, 0x14, 0x28),
            )),
            border: Border {
                color: alpha(p.gold, 0.4),
                width: 1.0,
                radius: radius.into(),
            },
            ..iced::widget::container::Style::default()
        })
        .into()
}

// ---------------------------------------------------------------------------
// Small shared style helpers
// ---------------------------------------------------------------------------

pub(crate) fn alpha(color: Color, a: f32) -> Color {
    Color { a, ..color }
}

pub(crate) fn v_gradient(top: Color, bottom: Color) -> Background {
    Background::Gradient(iced::Gradient::Linear(
        iced::gradient::Linear::new(iced::Radians(std::f32::consts::FRAC_PI_2))
            .add_stop(0.0, top)
            .add_stop(1.0, bottom),
    ))
}

/// Plain text with advanced shaping (Cyrillic + Hebrew).
pub(crate) fn txt<'a>(content: impl text::IntoFragment<'a>) -> iced::widget::Text<'a> {
    text(content).shaping(text::Shaping::Advanced)
}

/// Kicker from an owned string (widgets::kicker borrows).
pub(crate) fn kicker_owned<'a, Message: 'a>(content: String) -> Element<'a, Message> {
    txt(content.to_uppercase())
        .size(theme::text::KICKER)
        .font(fonts::SANS_BOLD)
        .color(theme::current().gold2)
        .into()
}

/// Serif heading from an owned string (widgets::heading borrows).
pub(crate) fn heading_owned<'a, Message: 'a>(content: String, level: u8) -> Element<'a, Message> {
    let size = match level {
        1 => theme::text::H1,
        2 => theme::text::H2,
        3 => theme::text::H3,
        _ => theme::text::BODY,
    };
    txt(content)
        .size(size)
        .font(fonts::SERIF_BOLD)
        .color(theme::current().text)
        .into()
}

// ---------------------------------------------------------------------------
// Screen
// ---------------------------------------------------------------------------

pub struct Screen;

#[derive(Debug, Clone)]
pub enum Msg {
    Go(Route),
    /// The live 3D model has been drawn by the GPU: rebuild the view
    /// without its static placeholder.
    Model3dReady,
}

impl Screen {
    pub fn new(_ctx: &mut Ctx, _route: &Route) -> Self {
        Screen
    }

    pub fn update(&mut self, _ctx: &mut Ctx, msg: Msg) -> Action {
        match msg {
            Msg::Go(route) => Action::Go(route),
            Msg::Model3dReady => Action::None,
        }
    }

    pub fn subscription(&self) -> iced::Subscription<Msg> {
        iced::Subscription::none()
    }

    pub fn view<'a>(&'a self, ctx: &'a Ctx) -> Element<'a, Msg> {
        let p = theme::current();
        let state = &ctx.state;
        let user = Library::user_level(state);

        let mut page = Column::new().spacing(18).width(Length::Fill);
        page = page.push(header(ctx, p));
        page = page.push(reminders(ctx, p));
        page = page.push(timed_cta(p));
        page = page.push(kiddush_hero(ctx, p));

        if !ctx.lib.sections.iter().any(|s| s.available) {
            page = page.push(notice(
                icons::Kind::Book,
                "Материалы ещё готовятся. Разделы появятся после сборки содержания.",
                p,
            ));
        }

        // ---- Path of sections ----
        let mut path = Column::new().spacing(12).width(Length::Fill);
        let mut current_seen = false;
        for (i, sec) in ctx.lib.sections.iter().enumerate() {
            let unlocked = ctx.lib.is_unlocked(sec, state);
            let stats = sec
                .available
                .then(|| ctx.lib.section_stats(sec, user, state));
            let item_state = if !sec.available {
                ItemState::Soon
            } else if !unlocked {
                ItemState::Locked
            } else if stats.as_ref().map(|s| s.complete).unwrap_or(false) {
                ItemState::Done
            } else {
                ItemState::Open
            };
            let is_cur = item_state == ItemState::Open && !current_seen;
            if is_cur {
                current_seen = true;
            }
            let prev_available = (0..i)
                .rev()
                .find(|&j| ctx.lib.sections[j].available)
                .map(|j| j + 1);
            path = path.push(path_item(
                sec,
                item_state,
                is_cur,
                prev_available,
                stats.as_ref(),
                p,
            ));
        }
        page = page.push(path);

        if !state.settings.unlock_all {
            page = page.push(
                txt("Следующий раздел открывается, когда прочитаны все уроки и тест сдан на 80% и выше. Свободный режим — в настройках.")
                    .size(theme::text::SMALL)
                    .color(p.muted)
                    .align_x(Alignment::Center)
                    .width(Length::Fill),
            );
        }
        page.into()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ItemState {
    Soon,
    Locked,
    Open,
    Done,
}

// ---------------------------------------------------------------------------
// Header: level chip + h1 + goal ring / streak / due stats
// ---------------------------------------------------------------------------

fn header<'a>(ctx: &'a Ctx, p: theme::Palette) -> Element<'a, Msg> {
    let state = &ctx.state;
    let lvl = content::level_info(Library::user_level(state));

    let chip = button(
        row![
            txt(format!("{} уровень", lvl.title).to_uppercase())
                .size(theme::text::KICKER)
                .font(fonts::SANS_BOLD)
                .color(p.gold2),
            icons::icon(icons::Kind::ChevronDown, 14.0, p.gold2),
            txt("сменить").size(12.0).color(p.muted),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .padding([8.0, 14.0])
    .style(move |_, status| {
        let base = theme::button_ghost(&p, status);
        button::Style {
            border: Border {
                color: p.gold,
                width: 1.0,
                radius: theme::radius::PILL.into(),
            },
            ..base
        }
    })
    .on_press(Msg::Go(Route::Level { reopened: true }));

    let left = column![chip, widgets::heading("Уроки", 1)]
        .spacing(10)
        .align_x(Alignment::Start);

    row![left, space::horizontal(), stats(ctx, p)]
        .width(Length::Fill)
        .spacing(20)
        .align_y(Alignment::Center)
        .wrap()
        .into()
}

fn stats<'a>(ctx: &'a Ctx, p: theme::Palette) -> Element<'a, Msg> {
    let state = &ctx.state;
    let today = ctx.today();
    let goal = game::goal(state);
    let xp = game::today_xp(state, today);
    let done = xp >= goal;
    let streak = game::streak(state, today);
    let kept_today = game::kept(state, &ctx.today_key());
    let due = ctx.lib.due_cards(state, today).len() as i64;

    // Goal ring (→ Прогресс).
    let ring_center: Element<'a, Msg> = if done {
        icons::icon(icons::Kind::Check, 16.0, p.ok)
    } else {
        txt(xp.to_string()).size(13.0).font(fonts::SANS_BOLD).into()
    };
    let goal_ring = stack![
        icons::ring(
            xp as f32 / goal.max(1) as f32,
            48.0,
            if done { p.ok } else { p.gold }
        ),
        container(ring_center)
            .width(48.0)
            .height(48.0)
            .center_x(48.0)
            .center_y(48.0),
    ];
    let goal_stat = stat_button(
        row![
            goal_ring,
            column![
                txt(format!("{} / {} XP", xp.min(goal), goal))
                    .size(theme::text::SMALL + 1.0)
                    .font(fonts::SANS_BOLD),
                txt(if done {
                    "цель дня выполнена"
                } else {
                    "цель дня"
                })
                .size(theme::text::SMALL)
                .color(p.muted),
            ]
            .spacing(2),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        Msg::Go(Route::Progress),
        p,
    );

    // Streak (→ Прогресс).
    let mut streak_small = row![txt(format!(
        "{} подряд",
        util::plural(streak, &["день", "дня", "дней"])
    ))
    .size(theme::text::SMALL)
    .color(p.muted),]
    .spacing(4)
    .align_y(Alignment::Center);
    if state.freezes > 0 {
        streak_small = streak_small
            .push(txt("·").size(theme::text::SMALL).color(p.muted))
            .push(icons::icon(icons::Kind::Snow, 13.0, p.info))
            .push(
                txt(state.freezes.to_string())
                    .size(theme::text::SMALL)
                    .color(p.muted),
            );
    }
    let streak_stat = stat_button(
        row![
            icons::icon(
                icons::Kind::Flame,
                22.0,
                if kept_today { p.flame } else { p.muted }
            ),
            column![
                txt(streak.to_string())
                    .size(theme::text::BODY)
                    .font(fonts::SANS_BOLD),
                streak_small,
            ]
            .spacing(2),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        Msg::Go(Route::Progress),
        p,
    );

    // Due cards (→ Повторение).
    let due_stat = stat_button(
        row![
            icons::icon(icons::Kind::Review, 22.0, p.gold2),
            column![
                txt(due.to_string())
                    .size(theme::text::BODY)
                    .font(fonts::SANS_BOLD),
                txt(format!(
                    "{} на сегодня",
                    util::plural(due, &["карточка", "карточки", "карточек"])
                ))
                .size(theme::text::SMALL)
                .color(p.muted),
            ]
            .spacing(2),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        Msg::Go(Route::Cards(Deck::Review)),
        p,
    );

    row![goal_stat, streak_stat, due_stat]
        .spacing(20)
        .align_y(Alignment::Center)
        .into()
}

fn stat_button<'a>(
    content: impl Into<Element<'a, Msg>>,
    msg: Msg,
    p: theme::Palette,
) -> Element<'a, Msg> {
    button(content)
        .padding([6.0, 8.0])
        .style(move |_, status| {
            let base = theme::button_ghost(&p, status);
            button::Style {
                border: Border {
                    radius: theme::radius::BUTTON.into(),
                    ..Border::default()
                },
                ..base
            }
        })
        .on_press(msg)
        .into()
}

// ---------------------------------------------------------------------------
// Reminder banners (js/game.js G.reminders)
// ---------------------------------------------------------------------------

fn reminders<'a>(ctx: &'a Ctx, p: theme::Palette) -> Element<'a, Msg> {
    let list = game::reminders(&ctx.lib, &ctx.state, ctx.today());
    let mut col = Column::new().spacing(8).width(Length::Fill);
    for r in list {
        let (kind, color, soft) = match r.kind {
            "streak" => (icons::Kind::Flame, p.flame, p.flame_soft),
            "due" => (icons::Kind::Review, p.info, p.info_soft),
            _ => (icons::Kind::Star, p.gold2, p.gold_soft),
        };
        let mut content = row![
            icons::icon(kind, 18.0, color),
            txt(r.text).size(theme::text::SMALL + 1.0).color(p.text),
        ]
        .spacing(10)
        .align_y(Alignment::Center);
        if r.href.is_some() {
            content = content
                .push(space::horizontal())
                .push(widgets::pill("Повторить", widgets::Pill::Cta));
        }
        let boxed = button(content.width(Length::Fill))
            .width(Length::Fill)
            .padding([10.0, 14.0])
            .style(move |_, status| {
                let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
                button::Style {
                    background: Some(Background::Color(if hovered {
                        alpha(color, 0.20)
                    } else {
                        soft
                    })),
                    text_color: p.text,
                    border: Border {
                        color: alpha(color, 0.4),
                        width: 1.0,
                        radius: theme::radius::SMALL.into(),
                    },
                    shadow: iced::Shadow::default(),
                    snap: true,
                }
            });
        let boxed = if r.href.is_some() {
            boxed.on_press(Msg::Go(Route::Cards(Deck::Review)))
        } else {
            boxed
        };
        col = col.push(boxed);
    }
    col.into()
}

// ---------------------------------------------------------------------------
// «Викторина» (timed quiz) CTA (§3)
// ---------------------------------------------------------------------------

fn timed_cta<'a>(_p: theme::Palette) -> Element<'a, Msg> {
    let tq = theme::TIMED;
    button(
        row![
            icons::icon(icons::Kind::Timer, 26.0, tq.gold2),
            column![
                txt("Викторина")
                    .size(theme::text::BODY)
                    .font(fonts::SANS_BOLD)
                    .color(tq.text),
                txt("Вопрос, четыре варианта и таймер — как в телевикторине")
                    .size(theme::text::SMALL)
                    .color(tq.muted),
            ]
            .spacing(2)
            .width(Length::Fill),
            container(
                txt("Играть")
                    .size(theme::text::SMALL)
                    .font(fonts::SANS_BOLD)
                    .color(tq.on_gold)
            )
            .padding([0.0, 18.0])
            .center_y(Length::Fixed(theme::size::PILL))
            .style(move |_| iced::widget::container::Style {
                background: Some(v_gradient(tq.gold2, tq.gold)),
                border: Border {
                    radius: theme::radius::PILL.into(),
                    ..Border::default()
                },
                ..iced::widget::container::Style::default()
            }),
        ]
        .spacing(14)
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding(18.0)
    .style(move |_, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        // Oxblood leather with a brass edge: the same in both themes, so the
        // banner reads as a bound volume on the library shelf.
        let top = if hovered {
            Color::from_rgb8(0x5a, 0x22, 0x1c)
        } else {
            Color::from_rgb8(0x4a, 0x1c, 0x17)
        };
        button::Style {
            background: Some(v_gradient(top, Color::from_rgb8(0x1e, 0x0c, 0x0a))),
            text_color: tq.text,
            border: Border {
                color: alpha(tq.gold, 0.55),
                width: 1.0,
                radius: 20.0.into(),
            },
            shadow: iced::Shadow::default(),
            snap: true,
        }
    })
    .on_press(Msg::Go(Route::Timed))
    .into()
}

/// Notice box (info tint, §1/§3).
pub(crate) fn notice<'a, Message: Clone + 'a>(
    kind: icons::Kind,
    body: &str,
    p: theme::Palette,
) -> Element<'a, Message> {
    container(
        row![
            icons::icon(kind, 20.0, p.info),
            txt(body.to_string())
                .size(theme::text::SMALL + 1.0)
                .color(p.text)
                .width(Length::Fill),
        ]
        .spacing(12)
        .align_y(Alignment::Center),
    )
    .padding([12.0, 16.0])
    .width(Length::Fill)
    .style(move |_| iced::widget::container::Style {
        background: Some(Background::Color(p.info_soft)),
        border: Border {
            color: alpha(p.info, 0.25),
            width: 1.0,
            radius: theme::radius::SMALL.into(),
        },
        ..iced::widget::container::Style::default()
    })
    .into()
}

// ---------------------------------------------------------------------------
// Section path
// ---------------------------------------------------------------------------

fn path_item<'a>(
    sec: &'a SectionMeta,
    state: ItemState,
    is_cur: bool,
    prev_available: Option<usize>,
    stats: Option<&content::SectionStats>,
    p: theme::Palette,
) -> Element<'a, Msg> {
    // ---- Card content ----
    let dim = state == ItemState::Soon;
    let text_color = if dim { alpha(p.text, 0.58) } else { p.text };
    let muted_color = if dim { alpha(p.muted, 0.58) } else { p.muted };

    let mut text_col = Column::new().spacing(4).width(Length::Fill);
    text_col = text_col.push(
        txt(&sec.title)
            .size(theme::text::H3)
            .font(fonts::SERIF_BOLD)
            .color(text_color),
    );
    text_col = text_col.push(
        txt(&sec.subtitle)
            .size(theme::text::SMALL)
            .color(muted_color),
    );
    match state {
        ItemState::Soon => {
            text_col = text_col.push(row![widgets::tag("Скоро")].align_y(Alignment::Center));
        }
        ItemState::Locked => {
            text_col = text_col.push(
                row![
                    icons::icon(icons::Kind::Lock, 12.0, p.muted),
                    txt(format!(
                        "Закрыт: пройдите раздел {}",
                        prev_available.unwrap_or(1)
                    ))
                    .size(theme::text::SMALL)
                    .font(fonts::SANS_BOLD)
                    .color(p.muted),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            );
        }
        ItemState::Open | ItemState::Done => {
            let s = stats.expect("open/done sections have stats");
            let best = s.best.map(fmt_pct).unwrap_or_else(|| "—".to_string());
            let mut status = row![
                txt(format!("{}/{} уроков · тест {}", s.read, s.lessons, best))
                    .size(theme::text::SMALL)
                    .color(muted_color)
            ]
            .spacing(10)
            .align_y(Alignment::Center);
            if is_cur {
                let started = s.read > 0 || s.best.is_some();
                status = status.push(widgets::pill(
                    if started {
                        "Продолжить"
                    } else {
                        "Начать"
                    },
                    widgets::Pill::Cta,
                ));
            }
            text_col = text_col.push(status);
        }
    }

    let right: Element<'a, Msg> = match state {
        ItemState::Open | ItemState::Done => {
            let pct = stats.map(|s| s.pct).unwrap_or(0);
            stack![
                icons::ring(pct as f32 / 100.0, 54.0, p.gold),
                container(
                    txt(format!("{pct}%"))
                        .size(12.0)
                        .font(fonts::SANS_BOLD)
                        .color(text_color)
                )
                .width(54.0)
                .height(54.0)
                .center_x(54.0)
                .center_y(54.0),
            ]
            .into()
        }
        _ => icons::icon(icons::Kind::Lock, 22.0, p.muted),
    };

    let card_inner = row![
        emblem(&sec.id, &sec.icon, 64.0, theme::radius::EMBLEM, dim),
        text_col,
        container(right).padding([0.0, 4.0]),
    ]
    .spacing(14)
    .align_y(Alignment::Center);

    let clickable = matches!(state, ItemState::Open | ItemState::Done);
    let card: Element<'a, Msg> = {
        let b = button(card_inner)
            .width(Length::Fill)
            .padding(16.0)
            .style(move |_, status| {
                let hovered = clickable
                    && matches!(status, button::Status::Hovered | button::Status::Pressed);
                button::Style {
                    background: Some(Background::Color(if hovered {
                        p.surface2
                    } else {
                        p.surface
                    })),
                    text_color: p.text,
                    border: Border {
                        color: if is_cur { p.gold } else { p.line },
                        width: 1.0,
                        radius: theme::radius::CARD.into(),
                    },
                    shadow: iced::Shadow::default(),
                    snap: true,
                }
            });
        let b = if clickable {
            b.on_press(Msg::Go(Route::Section(sec.id.clone())))
        } else {
            b
        };
        b.into()
    };

    // The rail column was dropped (audit §1.2): cards align with the banner.
    card
}

/// Format a best-test score like the PWA (`85`, `85.5`).
pub(crate) fn fmt_pct(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{}%", v as i64)
    } else {
        format!("{v:.1}%")
    }
}

/// «Бокал для кидуша»: a live, rotatable 3D model (wgpu) with a static render
/// as the fallback (tiny-skia, or «Объёмные модели» switched off).
fn kiddush_hero<'a>(ctx: &'a Ctx, p: theme::Palette) -> Element<'a, Msg> {
    use crate::ui::model3d::{self, Model};
    // Screenshot hook `SHORASHIM_3D_SIZE=<px>` (default 190).
    let size = util::env_hook("3D_SIZE")
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(190.0);
    let model = if model3d::enabled(ctx) {
        model3d::view(
            Model::KiddushCup,
            size,
            p.texture == "bg-dark",
            Msg::Model3dReady,
        )
    } else {
        model3d::still(Model::KiddushCup, size)
    };
    let hint = if model3d::enabled(ctx) {
        "Потяните бокал мышью, чтобы рассмотреть его со всех сторон."
    } else {
        "Объёмные модели выключены в настройках."
    };
    let body = column![
        widgets::kicker("Шаббат"),
        widgets::heading("Бокал для кидуша", 3),
        txt("Кидуш — освящение субботы и праздников: благословение произносят над бокалом вина в начале трапезы.")
            .size(theme::text::BODY)
            .color(p.text)
            .width(Length::Fill),
        txt(hint).size(theme::text::SMALL).color(p.muted),
        button(txt("Раздел «Шаббат»").size(theme::text::SMALL).font(fonts::SANS_BOLD))
            .padding([8.0, 16.0])
            .style(move |_, s| theme::button_secondary(&p, s))
            .on_press(Msg::Go(Route::Section("shabbat".to_string()))),
    ]
    .spacing(8)
    .width(Length::Fill);
    widgets::card(row![model, body].spacing(20).align_y(Alignment::Center))
}
