//! «Эрец-Исраэль» screen: a geographic map of the Land as one Jewish
//! country, with a toggleable Torah-borders layer and schematic tribal
//! allotments. Click a city for a short card and, where there is one, a
//! link to the related lesson.

use std::sync::{Mutex, OnceLock};

use iced::alignment::{Horizontal, Vertical};
use iced::mouse;
use iced::widget::canvas::{self, Canvas, Fill, Frame, LineDash, Path, Stroke, Text as CText};
use iced::widget::{container, row, space, Column, Row};
use iced::{Background, Border, Color, Element, Length, Point, Rectangle, Renderer, Theme};

use crate::eretz::{
    self, Coord, Extent, Place, CANAAN, COVENANT, JORDAN, KINNERET, LAND, PLACES, REGIONS,
    SALT_SEA, TORAH_CAPTION, TORAH_LABELS, TRIBES,
};
use crate::ui::app::{Action, Ctx, Route};
use crate::ui::fonts::{HEBREW, SANS, SANS_BOLD, SERIF_BOLD};
use crate::ui::screens::map::txt;
use crate::ui::theme::{self, Palette};
use crate::ui::widgets;

/// Optional start layer, consumed once by the next `Screen::new`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Start {
    Land,
    Torah,
    Tribes,
}

static OPEN: OnceLock<Mutex<Option<Start>>> = OnceLock::new();

/// Ask the next Eretz screen to open with this layer.
pub fn open_with(start: Start) {
    let slot = OPEN.get_or_init(|| Mutex::new(None));
    if let Ok(mut g) = slot.lock() {
        *g = Some(start);
    }
}

fn take_open() -> Option<Start> {
    OPEN.get()?.lock().ok().and_then(|mut g| g.take())
}

pub struct Screen {
    torah: bool,
    tribes: bool,
    selected: Option<&'static str>,
    /// Bumped when the default view should be reapplied (layer change, reset).
    view_gen: u32,
}

#[derive(Debug, Clone)]
pub enum Msg {
    ToggleTorah,
    ToggleTribes,
    /// Both layers off, view reset to the land itself.
    ShowLand,
    Select(&'static str),
    CloseCard,
    GoLesson(String),
    ResetView,
}

impl Screen {
    pub fn new(_ctx: &mut Ctx, _route: &Route) -> Self {
        let start = take_open().or_else(|| {
            match std::env::var("SHORASHIM_ERETZ")
                .ok()
                .as_deref()
                .map(str::trim)
            {
                Some("torah") => Some(Start::Torah),
                Some("tribes") => Some(Start::Tribes),
                _ => None,
            }
        });
        let (torah, tribes) = match start {
            Some(Start::Torah) => (true, false),
            Some(Start::Tribes) => (true, true),
            _ => (false, false),
        };
        Screen {
            torah,
            tribes,
            selected: None,
            view_gen: 1,
        }
    }

    pub fn update(&mut self, _ctx: &mut Ctx, msg: Msg) -> Action {
        match msg {
            Msg::ShowLand => {
                self.torah = false;
                self.tribes = false;
                self.view_gen = self.view_gen.wrapping_add(1);
                Action::None
            }
            Msg::ToggleTorah => {
                self.torah = !self.torah;
                if !self.torah {
                    self.tribes = false;
                }
                self.view_gen = self.view_gen.wrapping_add(1);
                Action::None
            }
            Msg::ToggleTribes => {
                self.tribes = !self.tribes;
                if self.tribes {
                    self.torah = true;
                }
                self.view_gen = self.view_gen.wrapping_add(1);
                Action::None
            }
            Msg::Select(id) => {
                self.selected = Some(id);
                Action::None
            }
            Msg::CloseCard => {
                self.selected = None;
                Action::None
            }
            Msg::GoLesson(id) => Action::Go(Route::Lesson(id)),
            Msg::ResetView => {
                self.view_gen = self.view_gen.wrapping_add(1);
                Action::None
            }
        }
    }

    pub fn subscription(&self) -> iced::Subscription<Msg> {
        iced::Subscription::none()
    }

    pub fn view<'a>(&'a self, _ctx: &'a Ctx) -> Element<'a, Msg> {
        let p = theme::current();
        let mut page = Column::new().spacing(14).width(Length::Fill);
        page = page.push(widgets::kicker("ЭРЕЦ-ИСРАЭЛЬ"));
        page = page.push(widgets::heading("Эрец-Исраэль", 1));
        page = page.push(
            txt("Вся Земля Израиля — одна еврейская земля: Иудея и Шомрон, Голан, Аза и единый Иерусалим, вечная неделимая столица. Без линий раздела и без вырезанных областей.")
                .size(theme::text::BODY)
                .color(p.muted)
                .width(Length::Fill),
        );

        let bar = Row::new()
            .spacing(8)
            .push(layer_btn(
                "Земля",
                !self.torah && !self.tribes,
                Msg::ShowLand,
            ))
            .push(layer_btn("Границы по Торе", self.torah, Msg::ToggleTorah))
            .push(layer_btn("Наделы колен", self.tribes, Msg::ToggleTribes))
            .push(layer_btn("Сбросить вид", false, Msg::ResetView));
        page = page.push(bar);

        let map = Canvas::new(MapView {
            pal: p,
            torah: self.torah,
            tribes: self.tribes,
            selected: self.selected,
            view_gen: self.view_gen,
            mini: false,
        })
        .width(Length::Fill)
        .height(Length::Fixed(if self.torah { 620.0 } else { 580.0 }));
        page = page.push(
            container(map)
                .width(Length::Fill)
                .padding(2)
                .style(move |_| container::Style {
                    background: Some(Background::Color(sea_color(p))),
                    border: Border {
                        color: p.line,
                        width: 1.0,
                        radius: theme::radius::CARD.into(),
                    },
                    ..container::Style::default()
                }),
        );

        if self.torah {
            page = page.push(
                container(
                    txt(TORAH_CAPTION)
                        .size(theme::text::SMALL)
                        .color(p.muted)
                        .width(Length::Fill),
                )
                .padding([8, 12])
                .width(Length::Fill)
                .style(move |_| container::Style {
                    background: Some(Background::Color(p.gold_soft)),
                    border: Border {
                        color: p.gold,
                        width: 1.0,
                        radius: theme::radius::SMALL.into(),
                    },
                    ..container::Style::default()
                }),
            );
        } else {
            page = page.push(
                txt("Включите слой «Границы по Торе», чтобы увидеть пределы Земли, обещанной в Торе: от реки Египта до Евфрата (Берешит 15:18; Бемидбар 34; Дварим 11:24).")
                    .size(theme::text::SMALL)
                    .color(p.muted)
                    .width(Length::Fill),
            );
        }

        if let Some(place) = self.selected.and_then(eretz::place) {
            page = page.push(place_card(place, p));
        } else {
            page = page.push(
                txt("Нажмите город, чтобы открыть карточку места.")
                    .size(theme::text::SMALL)
                    .color(p.muted),
            );
        }

        page.into()
    }
}

fn layer_btn(label: &'static str, on: bool, msg: Msg) -> Element<'static, Msg> {
    widgets::choice(label, on, true, msg)
}

fn place_card(place: &'static Place, p: Palette) -> Element<'static, Msg> {
    let mut body = Column::new().spacing(8).width(Length::Fill);
    body = body.push(
        row![
            txt(place.name)
                .size(theme::text::H3)
                .font(SERIF_BOLD)
                .color(p.text),
            space::horizontal(),
            txt(place.hebrew)
                .size(theme::text::H3)
                .font(HEBREW)
                .color(p.hebrew),
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center),
    );
    if place.capital {
        body = body.push(
            txt("Вечная и неделимая столица")
                .size(theme::text::SMALL)
                .font(SANS_BOLD)
                .color(p.gold2),
        );
    }
    body = body.push(
        txt(place.text)
            .size(theme::text::BODY)
            .color(p.text)
            .width(Length::Fill),
    );
    let mut actions = Row::new().spacing(10);
    if let Some((id, title)) = place.lesson {
        actions = actions.push(widgets::btn(
            format!("К уроку: {title}"),
            widgets::Btn::Primary,
            widgets::BtnSize::Md,
            Some(Msg::GoLesson(id.to_string())),
        ));
    }
    actions = actions.push(widgets::btn(
        "Закрыть",
        widgets::Btn::Ghost,
        widgets::BtnSize::Md,
        Some(Msg::CloseCard),
    ));
    body = body.push(actions);
    widgets::card(body)
}

fn view_extent(torah: bool, tribes: bool) -> Extent {
    if tribes {
        Extent::TRIBES
    } else if torah {
        Extent::TORAH
    } else {
        Extent::LAND
    }
}

fn is_dark() -> bool {
    theme::current_kind() == theme::ThemeKind::Dark
}

fn sea_color(_p: Palette) -> Color {
    if is_dark() {
        Color::from_rgb(0.10, 0.16, 0.22)
    } else {
        Color::from_rgb(0.72, 0.80, 0.84)
    }
}

fn land_fill(_p: Palette) -> Color {
    if is_dark() {
        Color::from_rgb(0.42, 0.34, 0.22)
    } else {
        Color::from_rgb(0.86, 0.78, 0.58)
    }
}

fn land_stroke(p: Palette) -> Color {
    p.gold
}

// ---------------------------------------------------------------------------
// Canvas
// ---------------------------------------------------------------------------

struct MapView {
    pal: Palette,
    torah: bool,
    tribes: bool,
    selected: Option<&'static str>,
    view_gen: u32,
    mini: bool,
}

#[derive(Debug, Clone)]
struct MapState {
    gen: u32,
    /// Pan in projected units (added after projection, in pixels).
    pan: Point,
    zoom: f32,
    drag: Option<(Point, Point)>,
}

impl Default for MapState {
    fn default() -> Self {
        Self {
            gen: 0,
            pan: Point::ORIGIN,
            zoom: 1.0,
            drag: None,
        }
    }
}

impl MapState {
    fn sync(&mut self, gen: u32) {
        if self.gen != gen {
            self.gen = gen;
            self.pan = Point::ORIGIN;
            self.zoom = 1.0;
            self.drag = None;
        }
    }
}

struct Projector {
    origin: Point,
    scale: f32,
    mid_cos: f32,
    west: f32,
    north: f32,
}

impl Projector {
    fn new(extent: Extent, bounds: Rectangle, zoom: f32, pan: Point) -> Self {
        let pad = 16.0;
        let avail_w = (bounds.width - 2.0 * pad).max(8.0);
        let avail_h = (bounds.height - 2.0 * pad).max(8.0);
        let pw = extent.proj_width().max(0.01);
        let ph = extent.proj_height().max(0.01);
        let scale = (avail_w / pw).min(avail_h / ph) * zoom;
        let used_w = pw * scale;
        let used_h = ph * scale;
        let origin = Point::new(
            bounds.x + (bounds.width - used_w) * 0.5 + pan.x,
            bounds.y + (bounds.height - used_h) * 0.5 + pan.y,
        );
        Self {
            origin,
            scale,
            mid_cos: extent.mid_lat().to_radians().cos().max(0.2),
            west: extent.west,
            north: extent.north,
        }
    }

    fn xy(&self, c: Coord) -> Point {
        Point::new(
            self.origin.x + (c.lon - self.west) * self.mid_cos * self.scale,
            self.origin.y + (self.north - c.lat) * self.scale,
        )
    }

    fn unproj(&self, p: Point) -> Coord {
        let lon = self.west + (p.x - self.origin.x) / (self.mid_cos * self.scale);
        let lat = self.north - (p.y - self.origin.y) / self.scale;
        Coord::new(lon, lat)
    }
}

fn ring_path(proj: &Projector, ring: &[Coord]) -> Path {
    Path::new(|b| {
        if let Some((first, rest)) = ring.split_first() {
            b.move_to(proj.xy(*first));
            for p in rest {
                b.line_to(proj.xy(*p));
            }
            b.close();
        }
    })
}

fn line_path(proj: &Projector, line: &[Coord]) -> Path {
    Path::new(|b| {
        if let Some((first, rest)) = line.split_first() {
            b.move_to(proj.xy(*first));
            for p in rest {
                b.line_to(proj.xy(*p));
            }
        }
    })
}

impl canvas::Program<Msg> for MapView {
    type State = MapState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Msg>> {
        state.sync(self.view_gen);
        if self.mini {
            return None;
        }
        match event {
            canvas::Event::Mouse(mouse::Event::WheelScrolled { delta })
                if cursor.is_over(bounds) =>
            {
                let y = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y,
                    mouse::ScrollDelta::Pixels { y, .. } => y / 80.0,
                };
                let new = (state.zoom * (1.0 + 0.12 * y)).clamp(0.8, 4.0);
                state.zoom = new;
                Some(canvas::Action::request_redraw().and_capture())
            }
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let p = cursor.position_over(bounds)?;
                state.drag = Some((p, state.pan));
                Some(canvas::Action::request_redraw().and_capture())
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { position }) => {
                if let Some((start, pan0)) = state.drag {
                    state.pan =
                        Point::new(pan0.x + position.x - start.x, pan0.y + position.y - start.y);
                    return Some(canvas::Action::request_redraw().and_capture());
                }
                cursor.is_over(bounds).then(canvas::Action::request_redraw)
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                let start = state.drag.take();
                let p = cursor.position_over(bounds)?;
                let clicked_in_place = start.is_some_and(|(s, _)| {
                    let dx = p.x - s.x;
                    let dy = p.y - s.y;
                    dx * dx + dy * dy < 25.0
                });
                if clicked_in_place {
                    let extent = view_extent(self.torah, self.tribes);
                    let proj = Projector::new(extent, bounds, state.zoom, state.pan);
                    let geo = proj.unproj(p);
                    // ~12 px in degrees at this scale
                    let max_deg = 12.0 / proj.scale.max(1.0);
                    if let Some(place) = eretz::nearest_place(geo, max_deg) {
                        return Some(canvas::Action::publish(Msg::Select(place.id)).and_capture());
                    }
                }
                Some(canvas::Action::request_redraw().and_capture())
            }
            _ => None,
        }
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if self.mini {
            return mouse::Interaction::default();
        }
        if state.drag.is_some() {
            return mouse::Interaction::Grabbing;
        }
        if cursor.position_over(bounds).is_some() {
            return mouse::Interaction::Pointer;
        }
        mouse::Interaction::default()
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut state = state.clone();
        state.sync(self.view_gen);
        let p = self.pal;
        let mut frame = Frame::new(renderer, bounds.size());
        // Frame coords are local to the canvas, so shift the projector.
        let local = Rectangle::new(Point::ORIGIN, bounds.size());
        let extent = view_extent(self.torah, self.tribes);
        let proj = Projector::new(extent, local, state.zoom, state.pan);

        // Background sea is already the container; a soft wash helps the leather theme.
        frame.fill(
            &Path::rectangle(Point::ORIGIN, bounds.size()),
            Fill::from(sea_color(p)),
        );

        const DASH: [f32; 2] = [8.0, 5.0];
        const DOT: [f32; 2] = [3.0, 5.0];

        if self.torah {
            let cov = ring_path(&proj, COVENANT);
            let fill = if is_dark() {
                Color::from_rgba(0.78, 0.62, 0.30, 0.16)
            } else {
                Color::from_rgba(0.55, 0.40, 0.10, 0.12)
            };
            frame.fill(&cov, Fill::from(fill));
            frame.stroke(
                &cov,
                Stroke {
                    style: canvas::Style::Solid(p.gold2),
                    width: 2.2,
                    line_dash: LineDash {
                        segments: &DASH,
                        offset: 0,
                    },
                    ..Stroke::default()
                },
            );
            let can = ring_path(&proj, CANAAN);
            frame.stroke(
                &can,
                Stroke {
                    style: canvas::Style::Solid(p.gold),
                    width: 1.6,
                    line_dash: LineDash {
                        segments: &DOT,
                        offset: 0,
                    },
                    ..Stroke::default()
                },
            );
        }

        if self.tribes {
            let colors = tribe_colors(p);
            for (i, tribe) in TRIBES.iter().enumerate() {
                let path = ring_path(&proj, tribe.pts);
                let (fill, stroke) = colors[i % colors.len()];
                frame.fill(&path, Fill::from(fill));
                frame.stroke(
                    &path,
                    Stroke {
                        style: canvas::Style::Solid(stroke),
                        width: 1.0,
                        ..Stroke::default()
                    },
                );
            }
        }

        let land = ring_path(&proj, LAND);
        let mut lf = land_fill(p);
        if self.tribes {
            // Keep the country outline visible over the allotments.
            lf.a = 0.28;
        }
        frame.fill(&land, Fill::from(lf));
        frame.stroke(
            &land,
            Stroke {
                style: canvas::Style::Solid(land_stroke(p)),
                width: if self.mini { 1.4 } else { 2.0 },
                ..Stroke::default()
            },
        );

        let water = if is_dark() {
            Color::from_rgb(0.18, 0.32, 0.42)
        } else {
            Color::from_rgb(0.55, 0.70, 0.78)
        };
        frame.fill(&ring_path(&proj, KINNERET), Fill::from(water));
        frame.fill(&ring_path(&proj, SALT_SEA), Fill::from(water));
        frame.stroke(
            &line_path(&proj, JORDAN),
            Stroke {
                style: canvas::Style::Solid(water),
                width: 1.6,
                ..Stroke::default()
            },
        );

        if !self.mini {
            let label_col = p.text;
            for region in REGIONS {
                if self.tribes {
                    continue;
                }
                let pt = proj.xy(Coord::new(
                    region.at.lon + region.dx,
                    region.at.lat + region.dy,
                ));
                if !local.contains(pt) {
                    continue;
                }
                frame.fill_text(CText {
                    content: region.name.to_string(),
                    position: pt,
                    color: label_col,
                    size: if self.torah { 11.0 } else { 13.0 }.into(),
                    font: SANS_BOLD,
                    align_x: Horizontal::Center.into(),
                    align_y: Vertical::Center,
                    ..CText::default()
                });
            }
            if self.torah && !self.tribes {
                for lab in TORAH_LABELS {
                    let pt = proj.xy(Coord::new(lab.at.lon + lab.dx, lab.at.lat + lab.dy));
                    if !local.contains(pt) {
                        continue;
                    }
                    frame.fill_text(CText {
                        content: lab.name.to_string(),
                        position: pt,
                        color: p.gold2,
                        size: 11.0.into(),
                        font: SANS_BOLD,
                        align_x: Horizontal::Center.into(),
                        align_y: Vertical::Center,
                        ..CText::default()
                    });
                }
            }
            if self.tribes {
                for tribe in TRIBES {
                    let pt = centroid(tribe.pts);
                    let xy = proj.xy(pt);
                    if !local.contains(xy) {
                        continue;
                    }
                    frame.fill_text(CText {
                        content: tribe.name.to_string(),
                        position: xy,
                        color: p.gold2,
                        size: 11.5.into(),
                        font: SANS_BOLD,
                        align_x: Horizontal::Center.into(),
                        align_y: Vertical::Center,
                        ..CText::default()
                    });
                }
            }

            let hover = cursor.position_in(local).and_then(|pt| {
                let geo = proj.unproj(pt);
                let max_deg = 12.0 / proj.scale.max(1.0);
                eretz::nearest_place(geo, max_deg)
            });

            for place in PLACES {
                let pt = proj.xy(place.at);
                if !local.contains(pt) {
                    continue;
                }
                let on = self.selected == Some(place.id) || hover.is_some_and(|h| h.id == place.id);
                let r = if place.capital { 5.5 } else { 3.6 };
                let r = if on { r + 1.6 } else { r };
                frame.fill(
                    &Path::circle(pt, r + 1.2),
                    Fill::from(if is_dark() {
                        Color::from_rgba(0.1, 0.07, 0.04, 0.7)
                    } else {
                        Color::from_rgba(1.0, 1.0, 1.0, 0.7)
                    }),
                );
                frame.fill(&Path::circle(pt, r), Fill::from(p.gold2));
                if place.capital {
                    // A small six-pointed hint around the capital.
                    frame.stroke(
                        &Path::circle(pt, r + 3.0),
                        Stroke {
                            style: canvas::Style::Solid(p.gold),
                            width: 1.2,
                            ..Stroke::default()
                        },
                    );
                }
                if (!self.torah && !self.tribes) || on || place.capital {
                    let lp = proj.xy(Coord::new(place.at.lon + place.dx, place.at.lat + place.dy));
                    let name_pos = if place.dy < 0.0 {
                        Point::new(lp.x, lp.y + r + 3.0)
                    } else {
                        Point::new(lp.x, lp.y - r - 3.0)
                    };
                    frame.fill_text(CText {
                        content: place.name.to_string(),
                        position: name_pos,
                        color: if on { p.gold2 } else { p.text },
                        size: if place.capital { 12.0 } else { 10.5 }.into(),
                        font: if place.capital { SANS_BOLD } else { SANS },
                        align_x: Horizontal::Center.into(),
                        align_y: if place.dy < 0.0 {
                            Vertical::Top
                        } else {
                            Vertical::Bottom
                        },
                        ..CText::default()
                    });
                }
            }
        }

        vec![frame.into_geometry()]
    }
}

fn centroid(pts: &[Coord]) -> Coord {
    let n = pts.len().max(1) as f32;
    let lon = pts.iter().map(|p| p.lon).sum::<f32>() / n;
    let lat = pts.iter().map(|p| p.lat).sum::<f32>() / n;
    Coord::new(lon, lat)
}

fn tribe_colors(p: Palette) -> [(Color, Color); 8] {
    let a = if is_dark() { 0.28 } else { 0.32 };
    [
        (Color::from_rgba(0.75, 0.45, 0.20, a), p.gold),
        (Color::from_rgba(0.40, 0.55, 0.30, a), p.gold),
        (Color::from_rgba(0.45, 0.40, 0.65, a), p.gold),
        (Color::from_rgba(0.70, 0.35, 0.40, a), p.gold),
        (Color::from_rgba(0.30, 0.50, 0.55, a), p.gold),
        (Color::from_rgba(0.60, 0.50, 0.20, a), p.gold),
        (Color::from_rgba(0.35, 0.40, 0.50, a), p.gold),
        (Color::from_rgba(0.55, 0.35, 0.50, a), p.gold),
    ]
}

/// Compact preview used on the lessons home (no hit-testing).
pub fn mini_map(height: f32) -> Element<'static, Msg> {
    let p = theme::current();
    let canvas = Canvas::new(MapView {
        pal: p,
        torah: false,
        tribes: false,
        selected: None,
        view_gen: 1,
        mini: true,
    })
    .width(Length::Fill)
    .height(Length::Fixed(height));
    container(canvas)
        .width(Length::Fill)
        .style(move |_| container::Style {
            background: Some(Background::Color(sea_color(p))),
            border: Border {
                color: p.line,
                width: 1.0,
                radius: theme::radius::SMALL.into(),
            },
            ..container::Style::default()
        })
        .into()
}
