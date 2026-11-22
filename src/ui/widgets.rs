//! Reusable view helpers, generic over the application `Message`.
//!
//! Every `text` uses `Shaping::Advanced` so Cyrillic + Hebrew runs are shaped
//! and bidi-correct; rich text (markdown-lite) is advanced-shaped by iced
//! itself. Colours come from the process-global active palette
//! (`theme::current`), which the app sets on every theme switch.

use iced::alignment::Horizontal;
use iced::widget::canvas::{self, Canvas, Frame, Path, Stroke};
use iced::widget::text::{IntoFragment, Shaping};
use iced::widget::{
    button, column, container, rich_text, row, space, span, text, Column, Row, Text,
};
use iced::{
    Alignment, Background, Border, Color, Element, Font, Length, Point, Rectangle, Renderer,
    Shadow, Theme,
};

use crate::ui::fonts::{HEBREW, SANS, SANS_BOLD, SERIF_BOLD};
use crate::ui::theme::{self, Movement, Palette};

fn palette() -> Palette {
    theme::current()
}

/// Plain text with advanced shaping applied.
fn txt<'a>(content: impl IntoFragment<'a>) -> Text<'a> {
    text(content).shaping(Shaping::Advanced)
}

/// A bare container style with an optional solid background.
fn plain_container(background: Option<Color>) -> iced::widget::container::Style {
    iced::widget::container::Style {
        text_color: None,
        background: background.map(Background::Color),
        border: Border::default(),
        shadow: Shadow::default(),
        snap: true,
    }
}

/// Kicker line: uppercase, letter-spaced gold label (§0.5).
pub fn kicker<'a, Message: 'a>(content: &'a str) -> Element<'a, Message> {
    let p = palette();
    txt(content.to_uppercase())
        .size(theme::text::KICKER)
        .font(SANS_BOLD)
        .color(p.gold)
        .into()
}

/// A heading (level 1–4) in the serif bold face (§T.4).
pub fn heading<'a, Message: 'a>(content: &'a str, level: u8) -> Element<'a, Message> {
    let p = palette();
    let size = match level {
        1 => theme::text::H1,
        2 => theme::text::H2,
        3 => theme::text::H3,
        _ => theme::text::BODY,
    };
    txt(content)
        .size(size)
        .font(SERIF_BOLD)
        .color(p.text)
        .into()
}

// ---------------------------------------------------------------------------
// Markdown-lite rich text
// ---------------------------------------------------------------------------

/// Hebrew code points that trigger the inline Hebrew treatment (PARITY §0).
fn is_hebrew(c: char) -> bool {
    ('\u{0590}'..='\u{05FF}').contains(&c) || ('\u{FB1D}'..='\u{FB4F}').contains(&c)
}

fn italic(font: Font) -> Font {
    Font {
        style: iced::font::Style::Italic,
        ..font
    }
}

/// Split a plain fragment on Hebrew runs and push styled spans.
fn push_runs<'a>(
    out: &mut Vec<iced::widget::text::Span<'a>>,
    fragment: &'a str,
    font: Font,
    size: f32,
    color: Color,
    hebrew_color: Color,
) {
    let mut run = String::new();
    let mut run_hebrew = false;
    let mut first = true;

    let mut flush = |run: &mut String, run_hebrew: bool| {
        if run.is_empty() {
            return;
        }
        let span = if run_hebrew {
            span(std::mem::take(run))
                .font(HEBREW)
                .color(hebrew_color)
                .size(size * 1.25)
        } else {
            span(std::mem::take(run)).font(font).color(color).size(size)
        };
        out.push(span);
    };

    for c in fragment.chars() {
        let hebrew = is_hebrew(c);
        if !first && hebrew != run_hebrew {
            flush(&mut run, run_hebrew);
        }
        run_hebrew = hebrew;
        first = false;
        run.push(c);
    }
    flush(&mut run, run_hebrew);
}

/// Parse `**bold**` / `*italic*` markers and append styled spans.
fn push_markdown<'a>(
    out: &mut Vec<iced::widget::text::Span<'a>>,
    input: &'a str,
    font: Font,
    bold_font: Font,
    size: f32,
    color: Color,
    hebrew_color: Color,
) {
    let mut bold = false;
    let mut italics = false;
    let mut rest = input;

    while !rest.is_empty() {
        let marker = rest.find('*');
        let (before, at) = match marker {
            Some(i) => (&rest[..i], i),
            None => (rest, rest.len()),
        };
        let face = if italics {
            italic(if bold { bold_font } else { font })
        } else if bold {
            bold_font
        } else {
            font
        };
        push_runs(out, before, face, size, color, hebrew_color);

        if at == rest.len() {
            break;
        }
        if rest[at..].starts_with("**") {
            bold = !bold;
            rest = &rest[at + 2..];
        } else {
            italics = !italics;
            rest = &rest[at + 1..];
        }
    }
}

fn rich_in<'a, Message: 'a>(
    input: &'a str,
    font: Font,
    bold_font: Font,
    size: f32,
    color: Color,
    hebrew_color: Color,
) -> iced::widget::text::Rich<'a, (), Message> {
    let mut spans: Vec<iced::widget::text::Span<'a>> = Vec::new();
    push_markdown(
        &mut spans,
        input,
        font,
        bold_font,
        size,
        color,
        hebrew_color,
    );
    rich_text::<(), Message, Theme, Renderer>(spans)
        .size(size)
        .line_height(1.6)
        .width(Length::Fill)
}

/// A paragraph rendering markdown-lite (`**bold**`, `*italic*`); Hebrew runs
/// are wrapped in the Hebrew font with the theme Hebrew colour, inline.
pub fn rich_paragraph<'a, Message: 'a>(input: &'a str) -> Element<'a, Message> {
    let p = palette();
    rich_in(input, SANS, SANS_BOLD, theme::text::BODY, p.text, p.hebrew).into()
}

/// A serif rich paragraph (quotes, term definitions).
fn rich_serif<'a, Message: 'a>(input: &'a str, size: f32, italics: bool) -> Element<'a, Message> {
    let p = palette();
    let serif = if italics {
        italic(SERIF_BOLD)
    } else {
        SERIF_BOLD
    };
    rich_in(input, serif, SERIF_BOLD, size, p.text, p.hebrew).into()
}

// ---------------------------------------------------------------------------
// Lesson-content blocks
// ---------------------------------------------------------------------------

/// Hebrew figure: right-aligned gold Hebrew line, then transliteration
/// (italic, muted) and the Russian translation below (§5, §T.7).
pub fn hebrew_block<'a, Message: 'a>(
    he: &'a str,
    translit: Option<&'a str>,
    ru: Option<&'a str>,
    reference: Option<&'a str>,
) -> Element<'a, Message> {
    let p = palette();

    let mut lines = Column::new().spacing(8).width(Length::Fill);
    lines = lines.push(
        txt(he)
            .size(theme::text::HEBREW)
            .font(HEBREW)
            .color(p.hebrew)
            // cosmic-text already lays out an RTL paragraph flush right inside
            // the buffer; iced's Right/Center alignment would shift it a second
            // time (off-screen). Keep Left + Fill for Hebrew paragraphs.
            .align_x(Horizontal::Left)
            .width(Length::Fill),
    );
    if let Some(translit) = translit {
        lines = lines.push(
            txt(translit)
                .size(theme::text::SMALL + 1.0)
                .font(italic(SANS))
                .color(p.muted)
                .align_x(Horizontal::Center)
                .width(Length::Fill),
        );
    }
    if let Some(ru) = ru {
        lines = lines.push(
            txt(ru)
                .size(theme::text::BODY)
                .color(p.text)
                .align_x(Horizontal::Center)
                .width(Length::Fill),
        );
    }
    if let Some(reference) = reference {
        lines = lines.push(
            txt(reference)
                .size(theme::text::SMALL - 1.5)
                .color(p.gold)
                .align_x(Horizontal::Center)
                .width(Length::Fill),
        );
    }

    container(lines)
        .padding([20.0, 24.0])
        .width(Length::Fill)
        .style(move |_| theme::hebrew_block(&palette()))
        .into()
}

/// Term box: bold serif term (with optional Hebrew) + definition text.
pub fn term_block<'a, Message: 'a>(
    term: &'a str,
    hebrew: Option<&'a str>,
    definition: &'a str,
) -> Element<'a, Message> {
    let p = palette();

    let mut inner = Column::new().spacing(8).width(Length::Fill);
    {
        let mut title = Row::new().align_y(Alignment::Center).spacing(8);
        let mut spans: Vec<iced::widget::text::Span<'a>> = Vec::new();
        push_runs(
            &mut spans,
            term,
            SERIF_BOLD,
            theme::text::H3,
            p.text,
            p.hebrew,
        );
        title = title.push(
            rich_text::<(), Message, Theme, Renderer>(spans)
                .size(theme::text::H3)
                .line_height(1.25),
        );
        if let Some(hebrew) = hebrew {
            title = title.push(
                txt(hebrew)
                    .size(theme::text::H3 * 1.25)
                    .font(HEBREW)
                    .color(p.hebrew),
            );
        }
        inner = inner.push(title);
    }
    inner = inner.push(rich_paragraph(definition));

    container(inner)
        .padding([14.0, 16.0])
        .width(Length::Fill)
        .style(move |_| theme::term(&palette()))
        .into()
}

/// A thin accent bar used as a left border (3 px, §T.8/§5).
fn accent_bar<'a, Message: 'a>(color: Color) -> Element<'a, Message> {
    iced::widget::rule::vertical(3)
        .style(move |_theme| iced::widget::rule::Style {
            color,
            radius: 2.0.into(),
            fill_mode: iced::widget::rule::FillMode::Full,
            snap: true,
        })
        .into()
}

/// Blockquote: 3 px gold left bar, serif italic text, optional cite line.
pub fn quote_block<'a, Message: 'a>(quote: &'a str, cite: Option<&'a str>) -> Element<'a, Message> {
    let p = palette();

    let mut inner = Column::new().spacing(6).width(Length::Fill);
    inner = inner.push(rich_serif(quote, theme::text::BODY * 1.08, true));
    if let Some(cite) = cite {
        inner = inner.push(txt(cite).size(theme::text::SMALL).color(p.muted));
    }

    row![
        accent_bar::<Message>(p.gold),
        container(inner)
            .padding([4.0, 0.0])
            .width(Length::Fill)
            .style(move |_| theme::quote(&palette())),
    ]
    .spacing(12)
    .width(Length::Fill)
    .into()
}

/// Info / tip box («Совет»): uppercase label + text (§5 `note`).
pub fn note_block<'a, Message: 'a>(label: &str, body: &'a str) -> Element<'a, Message> {
    let p = palette();

    let inner = column![
        txt(label.to_uppercase())
            .size(theme::text::KICKER)
            .font(SANS_BOLD)
            .color(p.info),
        rich_paragraph(body),
    ]
    .spacing(8)
    .width(Length::Fill);

    container(inner)
        .padding([14.0, 16.0])
        .width(Length::Fill)
        .style(move |_| theme::note(&palette()))
        .into()
}

/// «differs» note: label in the movement accent colour + text (§T.8).
/// The accent is chosen by matching the (movement-generated) label.
pub fn differs_note<'a, Message: 'a>(label: &'a str, body: &'a str) -> Element<'a, Message> {
    let p = palette();
    let movement = movement_for_label(label);
    let (accent, _) = movement.accent_colors(&p);

    let inner = row![
        accent_bar::<Message>(accent),
        container(
            column![
                txt(label)
                    .size(theme::text::BODY)
                    .font(SANS_BOLD)
                    .color(accent),
                rich_paragraph(body),
            ]
            .spacing(6)
            .width(Length::Fill)
        )
        .padding([10.0, 14.0])
        .width(Length::Fill),
    ]
    .width(Length::Fill);

    container(inner)
        .width(Length::Fill)
        .style(move |_| theme::differs(&palette(), movement))
        .into()
}

fn movement_for_label(label: &str) -> Movement {
    if label.contains("реформист") {
        Movement::Reform
    } else if label.contains("консервативн") {
        Movement::Conservative
    } else if label.contains("ортодокс") {
        Movement::Orthodox
    } else {
        Movement::Other
    }
}

// ---------------------------------------------------------------------------
// Progress indicators
// ---------------------------------------------------------------------------

/// Segmented step bar (§T.6): `done` of `total` gold segments.
pub fn step_bar<'a, Message: 'a>(done: usize, total: usize) -> Element<'a, Message> {
    let p = palette();
    let total = total.max(1);
    let done = done.min(total);

    let mut bar = Row::new().spacing(2).width(Length::Fill).height(14.0);
    for i in 0..total {
        let fill = if i < done { Some(theme_gold(&p)) } else { None };
        bar = bar.push(
            container(space::horizontal())
                .width(Length::FillPortion(1))
                .height(14.0)
                .style(move |_| iced::widget::container::Style {
                    background: fill,
                    border: Border {
                        radius: theme::radius::SEGMENT.into(),
                        ..Border::default()
                    },
                    ..plain_container(None)
                }),
        );
    }
    bar.into()
}

fn theme_gold(p: &Palette) -> Background {
    Background::Color(p.gold)
}

/// Ring program: track + gold arc for `pct` (0.0–1.0), round caps.
struct Ring {
    pct: f32,
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
        let p = palette();
        let mut frame = Frame::new(renderer, bounds.size());

        let size = bounds.size();
        let min = size.width.min(size.height);
        let stroke_width = min * 0.08;
        let radius = (min - stroke_width) / 2.0;
        let center = Point::new(size.width / 2.0, size.height / 2.0);

        // Track.
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

        // Fill arc, clockwise from 12 o'clock.
        let pct = self.pct.clamp(0.0, 1.0);
        if pct > 0.0 {
            let start = -std::f32::consts::FRAC_PI_2;
            let end = start + pct * std::f32::consts::TAU;
            let arc = Path::new(|builder| {
                builder.move_to(Point::new(
                    center.x + radius * start.cos(),
                    center.y + radius * start.sin(),
                ));
                builder.arc(canvas::path::Arc {
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

/// Circular progress ring (`pct` in 0.0–1.0), 54 px.
pub fn progress_ring<'a, Message: 'a>(pct: f32) -> Element<'a, Message> {
    Canvas::new(Ring { pct })
        .width(Length::Fixed(54.0))
        .height(Length::Fixed(54.0))
        .into()
}

// ---------------------------------------------------------------------------
// Small widgets
// ---------------------------------------------------------------------------

/// Selected chip pill (gold-soft bg, gold border, gold2 text).
pub fn chip<'a, Message: 'a>(content: &'a str) -> Element<'a, Message> {
    container(txt(content).size(theme::text::SMALL).font(SANS_BOLD))
        .padding([0.0, 16.0])
        .height(theme::spacing::CHIP_MIN)
        .style(move |_| theme::chip(&palette()))
        .into()
}

/// Tag pill (surface3 bg, muted text).
pub fn tag<'a, Message: 'a>(content: &'a str) -> Element<'a, Message> {
    container(txt(content).size(theme::text::SMALL).font(SANS_BOLD))
        .padding([0.0, 12.0])
        .height(30.0)
        .style(move |_| theme::tag(&palette()))
        .into()
}

/// Card container: surface bg, border, radius 18, padding.
pub fn card<'a, Message: 'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content)
        .padding(theme::spacing::CARD_PAD)
        .width(Length::Fill)
        .style(move |_| theme::card(&palette()))
        .into()
}

/// Toast pill (visual only; positioning is done by overlays).
pub fn toast<'a, Message: 'a>(content: &'a str) -> Element<'a, Message> {
    container(txt(content).size(theme::text::SMALL + 0.5))
        .padding([12.0, 18.0])
        .style(move |_| theme::toast(&palette()))
        .into()
}

/// Page header: optional back button («‹ Назад») + h1 (§0.5).
pub fn page_header<'a, Message: Clone + 'a>(
    title: &'a str,
    back: Option<Message>,
) -> Element<'a, Message> {
    let mut header = Column::new().spacing(10).width(Length::Fill);
    if let Some(message) = back {
        header = header.push(
            button(
                row![txt("‹").size(22.0), txt("Назад").size(15.0)]
                    .align_y(Alignment::Center)
                    .spacing(2),
            )
            .padding([6.0, 10.0])
            .style(move |_theme, status| theme::button_ghost(&palette(), status))
            .on_press(message),
        );
    }
    header = header.push(heading(title, 1));
    header.into()
}

/// KPI tile: serif value + small muted label (§11).
pub fn stat_tile<'a, Message: 'a>(value: &'a str, label: &'a str) -> Element<'a, Message> {
    let p = palette();
    column![
        txt(value)
            .size(theme::text::BODY * 1.6)
            .font(SERIF_BOLD)
            .color(p.gold2),
        txt(label)
            .size(theme::text::SMALL)
            .color(p.muted)
            .align_x(Horizontal::Center),
    ]
    .spacing(4)
    .align_x(Horizontal::Center)
    .width(Length::Fill)
    .into()
}

/// Open an external link in the system browser.
pub fn open_url(url: &str) {
    #[cfg(target_os = "windows")]
    let (prog, args): (&str, [&str; 2]) = ("rundll32", ["url.dll,FileProtocolHandler", url]);
    #[cfg(target_os = "macos")]
    let (prog, args): (&str, [&str; 1]) = ("open", [url]);
    #[cfg(all(unix, not(target_os = "macos")))]
    let (prog, args): (&str, [&str; 1]) = ("xdg-open", [url]);
    let _ = std::process::Command::new(prog).args(args).spawn();
}
