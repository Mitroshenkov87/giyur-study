//! The 3D «Маалот» stair: fifteen stone steps rising away from the viewer,
//! like the fifteen steps of the Temple court the Levites sang on (the
//! fifteen «Песни ступеней», Шир а-маалот). Drawn as a 2.5D canvas
//! (perspective trapezoids, lit treads, shaded risers, side parapets), so it
//! costs a few dozen filled paths on any renderer, the software one too. A
//! brass lamp stands on the last climbed step and hops up (or drops to the
//! safe step after a fall) with an eased animation.

use std::time::{Duration, Instant};

use iced::widget::canvas::{self, gradient, Frame, Path, Stroke};
use iced::widget::text::Shaping;
use iced::widget::{container, pin, row, space, stack, text, Stack};
use iced::{Alignment, Color, Element, Length, Point, Rectangle, Renderer, Size, Theme, Vector};

use super::RungState;
use crate::ladder;
use crate::ui::fonts::{HEBREW, SANS, SANS_BOLD};
use crate::ui::theme::{self, Palette};

/// Length of one climb (or fall) animation.
pub const CLIMB: Duration = Duration::from_millis(700);
/// Canvas height of the stair in the ladder column.
pub const HEIGHT: f32 = 540.0;

/// Canvas width of the stair (the full ladder column width): the
/// stair itself ([`STAIR_W`]) plus a margin on each side for the lamp,
/// which stands on the left parapet beside its step, so it never covers
/// the step names on the risers.
pub const WIDTH: f32 = 200.0;
/// Width the stair's own geometry is laid out for (widest step + walls).
pub const STAIR_W: f32 = 176.0;

/// Eased (cubic in-out) progress 0–1 of an animation started at `at`.
pub fn ease(at: Instant, now: Instant) -> f32 {
    let t = (now.saturating_duration_since(at).as_secs_f32() / CLIMB.as_secs_f32()).clamp(0.0, 1.0);
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

/// The climber position in steps (0 = the floor, n = on step n) for an
/// animation from `from` to `to` that started at `at`.
pub fn position(from: f32, to: f32, at: Option<Instant>, now: Instant) -> f32 {
    match at {
        Some(at) => from + (to - from) * ease(at, now),
        None => to,
    }
}

fn mix(a: Color, b: Color, t: f32) -> Color {
    Color {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: a.a + (b.a - a.a) * t,
    }
}

fn alpha(c: Color, a: f32) -> Color {
    Color { a, ..c }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Screen geometry of one step: riser from `y_bottom` to `y_riser`, tread
/// from `y_riser` to `y_top`; half-widths at those lines.
#[derive(Debug, Clone, Copy)]
pub struct StepGeom {
    pub y_bottom: f32,
    pub y_riser: f32,
    pub y_top: f32,
    pub hw_bottom: f32,
    pub hw_riser: f32,
    pub hw_top: f32,
}

/// Layout of `n` steps in a `w`×`h` box (bottom step first). Steps shrink
/// with distance: narrower, lower risers, thinner treads.
pub fn layout(n: usize, w: f32, h: f32) -> Vec<StepGeom> {
    let n = n.max(1);
    let top_pad = 46.0; // the landing and its light
    let bottom_pad = 14.0;
    let unit = |i: usize| -> (f32, f32) {
        let t = i as f32 / (n.max(2) - 1) as f32;
        (lerp(1.0, 0.70, t), lerp(0.32, 0.20, t))
    };
    let total: f32 = (0..n).map(|i| unit(i).0 + unit(i).1).sum();
    let u = (h - top_pad - bottom_pad).max(10.0) / total;
    let hw = |i: usize| -> f32 {
        let t = i as f32 / n as f32;
        lerp(w * 0.43, w * 0.25, t)
    };
    let mut y = h - bottom_pad;
    (0..n)
        .map(|i| {
            let (rh, td) = unit(i);
            let g = StepGeom {
                y_bottom: y,
                y_riser: y - rh * u,
                y_top: y - (rh + td) * u,
                hw_bottom: hw(i),
                hw_riser: hw(i) * 0.985,
                hw_top: hw(i + 1),
            };
            y = g.y_top;
            g
        })
        .collect()
}

/// Parapet (side wall) width at the bottom and at the top of the stair.
fn parapet_w() -> (f32, f32) {
    (STAIR_W * 0.06, STAIR_W * 0.035)
}

/// Where the lamp stands for a position `pos` (0 = floor, k = tread k),
/// with a hop between steps; also its scale (farther = smaller). The lamp
/// stands on the left parapet at the level of its tread, beside the step
/// labels rather than on them.
pub fn climber_at(steps: &[StepGeom], cx: f32, pos: f32) -> (Point, f32) {
    let n = steps.len().max(1) as f32;
    let (pw0, pw1) = parapet_w();
    let spot = |k: usize| -> Point {
        let pw = lerp(pw0, pw1, k as f32 / n);
        if k == 0 {
            let g = steps[0];
            Point::new(cx - g.hw_bottom - pw * 0.5, g.y_bottom + 2.0)
        } else {
            let g = steps[(k - 1).min(steps.len() - 1)];
            let hw = (g.hw_riser + g.hw_top) / 2.0;
            Point::new(cx - hw - pw * 0.5, (g.y_riser + g.y_top) / 2.0)
        }
    };
    let max = steps.len() as f32;
    let pos = pos.clamp(0.0, max);
    let a = pos.floor() as usize;
    let f = pos - a as f32;
    let p0 = spot(a);
    let p1 = spot((a + 1).min(steps.len()));
    let rise = steps[a.min(steps.len() - 1)].y_bottom - steps[a.min(steps.len() - 1)].y_riser;
    let hop = (f * std::f32::consts::PI).sin() * rise * 0.55;
    let p = Point::new(lerp(p0.x, p1.x, f), lerp(p0.y, p1.y, f) - hop);
    (p, lerp(1.0, 0.72, pos / max))
}

/// The stair canvas program.
pub struct Stair {
    pub states: Vec<RungState>,
    /// Climber position in steps (fractional while animating).
    pub pos: f32,
}

/// (riser top, riser bottom, tread, ink) colours of a step.
fn step_colors(p: &Palette, st: RungState) -> (Color, Color, Color, Color) {
    let stone_hi = mix(p.surface2, p.text, 0.10);
    let stone_lo = mix(p.surface2, p.bg0, 0.45);
    let tread = mix(p.surface3, p.text, 0.16);
    match st {
        RungState::Current => (
            p.brass_hi,
            p.brass_lo,
            mix(p.brass_hi, p.text, 0.25),
            p.on_gold,
        ),
        RungState::Done => (
            mix(stone_hi, p.gold2, 0.62),
            mix(stone_lo, p.gold, 0.45),
            mix(tread, p.gold2, 0.60),
            p.text,
        ),
        RungState::Lost => (
            mix(stone_hi, p.gold, 0.12),
            stone_lo,
            mix(tread, p.gold, 0.12),
            p.muted,
        ),
        RungState::Fell => (
            mix(stone_hi, p.bad, 0.55),
            mix(stone_lo, p.bad, 0.35),
            mix(tread, p.bad, 0.35),
            p.text,
        ),
        RungState::Ahead => (
            mix(stone_hi, p.bg0, 0.12),
            stone_lo,
            mix(tread, p.bg0, 0.10),
            alpha(p.muted, 0.9),
        ),
    }
}

fn quad(a: Point, b: Point, c: Point, d: Point) -> Path {
    Path::new(|pb| {
        pb.move_to(a);
        pb.line_to(b);
        pb.line_to(c);
        pb.line_to(d);
        pb.close();
    })
}

fn v_fill(top: f32, bottom: f32, c_top: Color, c_bottom: Color) -> canvas::Fill {
    canvas::Fill {
        style: canvas::Style::Gradient(canvas::Gradient::Linear(
            gradient::Linear::new(Point::new(0.0, top), Point::new(0.0, bottom))
                .add_stop(0.0, c_top)
                .add_stop(1.0, c_bottom),
        )),
        ..canvas::Fill::default()
    }
}

fn ellipse(c: Point, rx: f32, ry: f32) -> Path {
    Path::new(|b| {
        b.ellipse(canvas::path::arc::Elliptical {
            center: c,
            radii: Vector::new(rx.max(0.1), ry.max(0.1)),
            rotation: iced::Radians(0.0),
            start_angle: iced::Radians(0.0),
            end_angle: iced::Radians(std::f32::consts::TAU),
        });
    })
}

impl<Message> canvas::Program<Message> for Stair {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let p = theme::current();
        let mut frame = Frame::new(renderer, bounds.size());
        let (w, h) = (bounds.width, bounds.height);
        let cx = w / 2.0;
        let n = self.states.len().max(1);
        let steps = layout(n, STAIR_W, h);
        let first = steps[0];
        let last = steps[n - 1];

        // Light from the top landing (the goal) and a floor shadow.
        let sw = STAIR_W;
        for (r, a) in [(sw * 0.46, 0.05), (sw * 0.32, 0.07), (sw * 0.2, 0.09)] {
            frame.fill(
                &ellipse(Point::new(cx, last.y_top - 8.0), r, r * 0.55),
                alpha(p.gold, a),
            );
        }
        frame.fill(
            &ellipse(
                Point::new(cx, first.y_bottom + 4.0),
                first.hw_bottom * 1.25,
                9.0,
            ),
            alpha(Color::BLACK, 0.28),
        );

        // Side parapets: darker stone walls from the floor to the landing.
        let (pw0, pw1) = parapet_w();
        let cap = 10.0;
        let stone_side_hi = mix(p.surface2, p.bg0, 0.25);
        let stone_side_lo = mix(p.surface2, p.bg0, 0.65);
        for side in [-1.0f32, 1.0] {
            let inner0 = cx + side * first.hw_bottom;
            let outer0 = cx + side * (first.hw_bottom + pw0);
            let inner1 = cx + side * last.hw_top;
            let outer1 = cx + side * (last.hw_top + pw1);
            let wall = quad(
                Point::new(inner0, first.y_bottom),
                Point::new(outer0, first.y_bottom),
                Point::new(outer1, last.y_top - cap),
                Point::new(inner1, last.y_top - cap),
            );
            frame.fill(
                &wall,
                v_fill(last.y_top, first.y_bottom, stone_side_hi, stone_side_lo),
            );
            // Coping: a lit top edge.
            frame.stroke(
                &Path::line(
                    Point::new(outer0, first.y_bottom - 1.0),
                    Point::new(outer1, last.y_top - cap),
                ),
                Stroke {
                    style: canvas::Style::Solid(alpha(p.brass_hi, 0.35)),
                    width: 1.2,
                    ..Stroke::default()
                },
            );
        }

        // Landing at the top with two brass posts of a gate.
        let land_hw = last.hw_top;
        let land = quad(
            Point::new(cx - land_hw, last.y_top),
            Point::new(cx + land_hw, last.y_top),
            Point::new(cx + land_hw * 0.92, last.y_top - 12.0),
            Point::new(cx - land_hw * 0.92, last.y_top - 12.0),
        );
        frame.fill(&land, mix(mix(p.surface3, p.text, 0.16), p.gold, 0.18));
        let gate_h = 30.0;
        for side in [-1.0f32, 1.0] {
            let x = cx + side * land_hw * 0.72;
            frame.fill(
                &Path::rectangle(
                    Point::new(x - 2.5, last.y_top - 12.0 - gate_h),
                    Size::new(5.0, gate_h + 2.0),
                ),
                v_fill(
                    last.y_top - 12.0 - gate_h,
                    last.y_top - 10.0,
                    p.brass_hi,
                    p.brass_lo,
                ),
            );
        }
        frame.fill(
            &Path::rectangle(
                Point::new(cx - land_hw * 0.72 - 6.0, last.y_top - 14.0 - gate_h),
                Size::new(land_hw * 1.44 + 12.0, 4.0),
            ),
            p.brass_mid,
        );

        // Steps, the far (top) ones first so nearer treads overlap them.
        for (i, g) in steps.iter().enumerate().rev() {
            let st = self.states.get(i).copied().unwrap_or(RungState::Ahead);
            let r = ladder::rung(i + 1);
            let (hi, lo, tread, _) = step_colors(&p, st);
            // Tread (lit from above).
            let tq = quad(
                Point::new(cx - g.hw_riser, g.y_riser),
                Point::new(cx + g.hw_riser, g.y_riser),
                Point::new(cx + g.hw_top, g.y_top),
                Point::new(cx - g.hw_top, g.y_top),
            );
            frame.fill(
                &tq,
                v_fill(g.y_top, g.y_riser, mix(tread, p.bg0, 0.12), tread),
            );
            // Glow behind the step being played.
            if st == RungState::Current {
                for (grow, a) in [(9.0, 0.10), (5.0, 0.16), (2.5, 0.22)] {
                    frame.fill(
                        &Path::rounded_rectangle(
                            Point::new(cx - g.hw_bottom - grow, g.y_riser - grow),
                            Size::new(
                                2.0 * (g.hw_bottom + grow),
                                g.y_bottom - g.y_riser + 2.0 * grow,
                            ),
                            (grow + 2.0).into(),
                        ),
                        alpha(p.gold, a),
                    );
                }
            }
            // Riser (front face), lighter at the top edge.
            let rq = quad(
                Point::new(cx - g.hw_bottom, g.y_bottom),
                Point::new(cx + g.hw_bottom, g.y_bottom),
                Point::new(cx + g.hw_riser, g.y_riser),
                Point::new(cx - g.hw_riser, g.y_riser),
            );
            frame.fill(&rq, v_fill(g.y_riser, g.y_bottom, hi, lo));
            // Crisp nose line and the shadow under it.
            let nose = if r.safe { p.gold } else { alpha(p.text, 0.18) };
            frame.stroke(
                &Path::line(
                    Point::new(cx - g.hw_riser, g.y_riser),
                    Point::new(cx + g.hw_riser, g.y_riser),
                ),
                Stroke {
                    style: canvas::Style::Solid(nose),
                    width: if r.safe { 2.0 } else { 1.0 },
                    ..Stroke::default()
                },
            );
            frame.stroke(
                &Path::line(
                    Point::new(cx - g.hw_bottom, g.y_bottom - 0.5),
                    Point::new(cx + g.hw_bottom, g.y_bottom - 0.5),
                ),
                Stroke {
                    style: canvas::Style::Solid(alpha(Color::BLACK, 0.30)),
                    width: 1.0,
                    ..Stroke::default()
                },
            );
        }

        // The lamp: shadow on the tread, glow, brass body, highlight.
        let (c, scale) = climber_at(&steps, cx, self.pos);
        let base = climber_at(&steps, cx, self.pos.round()).0;
        let r = 10.0 * scale;
        let lift = (base.y - c.y).max(0.0);
        frame.fill(
            &ellipse(Point::new(c.x, c.y + lift), r * 1.3, r * 0.45),
            alpha(Color::BLACK, 0.35 * (1.0 - (lift / 30.0).min(0.6))),
        );
        let core = Point::new(c.x, c.y - r * 1.1);
        for (k, a) in [(2.8, 0.10), (2.0, 0.16), (1.45, 0.26)] {
            frame.fill(&Path::circle(core, r * k), alpha(p.gold, a));
        }
        frame.fill(
            &Path::circle(core, r),
            canvas::Fill {
                style: canvas::Style::Gradient(canvas::Gradient::Linear(
                    gradient::Linear::new(
                        Point::new(core.x, core.y - r),
                        Point::new(core.x, core.y + r),
                    )
                    .add_stop(0.0, p.brass_hi)
                    .add_stop(1.0, p.brass_lo),
                )),
                ..canvas::Fill::default()
            },
        );
        frame.stroke(
            &Path::circle(core, r),
            Stroke {
                style: canvas::Style::Solid(p.brass_edge),
                width: 1.0,
                ..Stroke::default()
            },
        );
        frame.fill(
            &Path::circle(Point::new(core.x - r * 0.35, core.y - r * 0.35), r * 0.3),
            alpha(Color::WHITE, 0.55),
        );
        vec![frame.into_geometry()]
    }
}

/// Step labels (number, name of the named steps, Hebrew numeral), laid
/// over the canvas as text widgets pinned to each riser: the app's own
/// text shaping, Hebrew included, and no canvas text.
fn labels<'a, M: 'a>(states: &[RungState]) -> Element<'a, M> {
    let p = theme::current();
    let n = states.len().max(1);
    let steps = layout(n, STAIR_W, HEIGHT);
    let cx = WIDTH / 2.0;
    let mut layer: Stack<'a, M> = Stack::new()
        .width(Length::Fixed(WIDTH))
        .height(Length::Fixed(HEIGHT));
    layer = layer.push(space().width(WIDTH).height(HEIGHT));
    for (i, g) in steps.iter().enumerate() {
        let st = states.get(i).copied().unwrap_or(RungState::Ahead);
        let r = ladder::rung(i + 1);
        let (_, _, _, ink) = step_colors(&p, st);
        let he_ink = match st {
            RungState::Current => p.on_gold,
            RungState::Done => p.gold2,
            RungState::Fell => p.bad,
            _ => alpha(p.hebrew, 0.75),
        };
        let h = g.y_bottom - g.y_riser;
        let size = (h * 0.5).clamp(10.0, 14.0);
        let bold = matches!(st, RungState::Current | RungState::Fell);
        let line = row![
            text((i + 1).to_string())
                .size(size)
                .font(SANS_BOLD)
                .color(ink),
            container(
                text(r.name)
                    .size(size * 0.95)
                    .font(if bold { SANS_BOLD } else { SANS })
                    .color(ink)
                    .shaping(Shaping::Advanced)
            )
            .center_x(Length::Fill),
            text(r.he)
                .size(size * 1.08)
                .font(HEBREW)
                .color(he_ink)
                .shaping(Shaping::Advanced),
        ]
        .align_y(Alignment::Center)
        .padding([0.0, 7.0]);
        let w = 2.0 * g.hw_riser;
        layer = layer.push(
            pin(container(line)
                .width(Length::Fixed(w))
                .center_y(Length::Fixed(h)))
            .x(cx - g.hw_riser)
            .y(g.y_riser),
        );
    }
    layer.into()
}

/// The stair: the canvas with the step labels over it.
pub fn view<'a, M: 'a>(states: Vec<RungState>, pos: f32) -> Element<'a, M> {
    let over = labels(&states);
    stack![
        canvas::Canvas::new(Stair { states, pos })
            .width(Length::Fixed(WIDTH))
            .height(Length::Fixed(HEIGHT)),
        over,
    ]
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_rise_and_recede() {
        let s = layout(15, 176.0, HEIGHT);
        assert_eq!(s.len(), 15);
        assert!(s[0].y_bottom <= HEIGHT && s[14].y_top > 0.0);
        for w in s.windows(2) {
            assert!((w[0].y_top - w[1].y_bottom).abs() < 1e-3, "steps touch");
            assert!(w[1].hw_bottom < w[0].hw_bottom, "farther = narrower");
            assert!(
                w[1].y_bottom - w[1].y_riser < w[0].y_bottom - w[0].y_riser,
                "farther = lower riser"
            );
        }
        // A readable riser even at the top of the stair.
        assert!(s[14].y_bottom - s[14].y_riser >= 18.0);
    }

    #[test]
    fn the_climb_eases_and_ends_on_the_step() {
        let t0 = Instant::now();
        assert_eq!(position(3.0, 4.0, None, t0), 4.0);
        let mid = position(3.0, 4.0, Some(t0), t0 + CLIMB / 2);
        assert!((mid - 3.5).abs() < 0.01, "{mid}");
        let early = position(3.0, 4.0, Some(t0), t0 + CLIMB / 5);
        assert!(early > 3.0 && early < 3.2, "slow start: {early}");
        assert_eq!(position(3.0, 4.0, Some(t0), t0 + CLIMB * 2), 4.0);
        // A fall back to the safe step animates downwards.
        let down = position(7.0, 5.0, Some(t0), t0 + CLIMB / 2);
        assert!(down < 7.0 && down > 5.0);
    }

    #[test]
    fn the_lamp_hops_between_treads() {
        let s = layout(15, 176.0, HEIGHT);
        let (on3, _) = climber_at(&s, 88.0, 3.0);
        let (on4, _) = climber_at(&s, 88.0, 4.0);
        let (mid, _) = climber_at(&s, 88.0, 3.5);
        assert!(on4.y < on3.y);
        // Mid-hop the lamp is above the straight line between the treads.
        assert!(mid.y < (on3.y + on4.y) / 2.0 - 2.0);
        let (floor, big) = climber_at(&s, 88.0, 0.0);
        let (top, small) = climber_at(&s, 88.0, 15.0);
        assert!(floor.y > top.y && big > small);
    }

    #[test]
    fn the_lamp_stays_clear_of_the_step_labels() {
        let s = layout(15, STAIR_W, HEIGHT);
        let cx = WIDTH / 2.0;
        for k in 0..=15 {
            for f in [0.0, 0.5] {
                let pos = (k as f32 + f).min(15.0);
                let (c, scale) = climber_at(&s, cx, pos);
                let r = 10.0 * scale;
                // Whole lamp (body and shadow) inside the canvas.
                assert!(c.x - r * 1.3 >= 0.0, "pos {pos}: x {}", c.x);
                // The body ends left of the text of every label row it is
                // level with (labels start 7 px inside the riser).
                let core_y = c.y - r * 1.1;
                for g in s
                    .iter()
                    .filter(|g| core_y + r > g.y_riser && core_y - r < g.y_bottom)
                {
                    assert!(c.x + r <= cx - g.hw_riser + 7.0, "pos {pos}");
                }
            }
        }
    }
}
