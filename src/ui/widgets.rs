//! Reusable view helpers, generic over the application `Message`.
//!
//! Every `text` uses `Shaping::Advanced` so Cyrillic + Hebrew runs are shaped
//! and bidi-correct; rich text (markdown-lite) is advanced-shaped by iced
//! itself. Colours come from the process-global active palette
//! (`theme::current`), which the app sets on every theme switch.

use iced::advanced::layout::Node;
use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::alignment::Horizontal;
use iced::widget::canvas::{self, Canvas, Frame, Path, Stroke};
use iced::widget::text::{IntoFragment, Shaping};
use iced::widget::{
    button, column, container, rich_text, row, space, span, text, Button, Column, Row, Text,
};
use iced::{
    Alignment, Background, Border, Color, Element, Event, Font, Length, Point, Rectangle, Renderer,
    Shadow, Theme, Vector,
};

use crate::ui::fonts::{HEBREW, SANS, SANS_BOLD, SERIF_BOLD};
use crate::ui::screens::map::icons;
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

/// Kicker line: uppercase, letter-spaced gold2 label (§0.5).
pub fn kicker<'a, Message: 'a>(content: &'a str) -> Element<'a, Message> {
    let p = palette();
    txt(content.to_uppercase())
        .size(theme::text::KICKER)
        .font(SANS_BOLD)
        .color(p.gold2)
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
    crate::automation::reported(
        rich_in(input, SANS, SANS_BOLD, theme::text::BODY, p.text, p.hebrew),
        input,
    )
}

/// A serif rich paragraph (quotes, term definitions).
fn rich_serif<'a, Message: 'a>(input: &'a str, size: f32, italics: bool) -> Element<'a, Message> {
    let p = palette();
    let serif = if italics {
        italic(SERIF_BOLD)
    } else {
        SERIF_BOLD
    };
    crate::automation::reported(
        rich_in(input, serif, SERIF_BOLD, size, p.text, p.hebrew),
        input,
    )
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
        title = title.push(crate::automation::reported(
            rich_text::<(), Message, Theme, Renderer>(spans)
                .size(theme::text::H3)
                .line_height(1.25),
            term,
        ));
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
// Shared controls (§3): the only way to build buttons, pills and chips
// ---------------------------------------------------------------------------

/// Button kind (which theme style to use).
#[derive(Debug, Clone, Copy)]
pub enum Btn {
    Primary,
    Secondary,
    Ghost,
    Danger,
}

/// Button size (fixed height, horizontal padding, font size).
#[derive(Debug, Clone, Copy)]
pub enum BtnSize {
    Sm,
    Md,
    Lg,
}

fn btn_metrics(size: BtnSize) -> (f32, f32, f32) {
    match size {
        BtnSize::Sm => (theme::size::BTN_SM, 12.0, 15.0),
        BtnSize::Md => (theme::size::BTN_MD, 20.0, 16.0),
        BtnSize::Lg => (theme::size::BTN_LG, 26.0, 17.0),
    }
}

fn btn_style(kind: Btn) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let p = palette();
        match kind {
            Btn::Primary => theme::button_primary(&p, status),
            Btn::Secondary => theme::button_secondary(&p, status),
            Btn::Ghost => theme::button_ghost(&p, status),
            Btn::Danger => theme::button_danger(&p, status),
        }
    }
}

/// The only way to build a text button. Fixed height, centred label.
pub fn btn<'a, M: Clone + 'a>(
    label: impl IntoFragment<'a>,
    kind: Btn,
    size: BtnSize,
    on_press: Option<M>,
) -> Button<'a, M> {
    let (h, px, fs) = btn_metrics(size);
    let font = if matches!(kind, Btn::Primary) {
        SANS_BOLD
    } else {
        SANS
    };
    button(container(txt(label).size(fs).font(font)).center_y(Length::Fill))
        .height(h)
        .padding([0.0, px])
        .on_press_maybe(on_press)
        .style(btn_style(kind))
}

/// Same, with a leading icon (icon + label vertically centred).
pub fn btn_icon<'a, M: Clone + 'a>(
    icon: Element<'a, M>,
    label: impl IntoFragment<'a>,
    kind: Btn,
    size: BtnSize,
    on_press: Option<M>,
) -> Button<'a, M> {
    let (h, px, fs) = btn_metrics(size);
    let font = if matches!(kind, Btn::Primary) {
        SANS_BOLD
    } else {
        SANS
    };
    button(
        container(
            row![icon, txt(label).size(fs).font(font)]
                .spacing(8)
                .align_y(Alignment::Center),
        )
        .center_y(Length::Fill),
    )
    .height(h)
    .padding([0.0, px])
    .on_press_maybe(on_press)
    .style(btn_style(kind))
}

/// «‹ label» back button: Sm, ghost, ChevronLeft icon 14 px, spacing 6.
pub fn back_button<'a, M: Clone + 'a>(label: &'a str, msg: M) -> Element<'a, M> {
    let p = palette();
    button(
        container(
            row![
                icons::icon(icons::Kind::ChevronLeft, 14.0, p.muted),
                txt(label).size(15.0),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .center_y(Length::Fill),
    )
    .height(theme::size::BTN_SM)
    .padding([0.0, 12.0])
    .style(move |_, s| theme::button_ghost(&palette(), s))
    .on_press(msg)
    .into()
}

/// «× Выйти»: same metrics as [`back_button`].
pub fn exit_button<'a, M: Clone + 'a>(label: &'a str, msg: M) -> Element<'a, M> {
    let p = palette();
    button(
        container(
            row![txt("×").size(16.0).color(p.muted), txt(label).size(15.0),]
                .spacing(6)
                .align_y(Alignment::Center),
        )
        .center_y(Length::Fill),
    )
    .height(theme::size::BTN_SM)
    .padding([0.0, 12.0])
    .style(move |_, s| theme::button_ghost(&palette(), s))
    .on_press(msg)
    .into()
}

/// Pill kind (non-interactive).
#[derive(Debug, Clone, Copy)]
pub enum Pill {
    /// Selected chip («Требует проверки раввином»): gold-soft, gold border.
    Chip,
    /// Tag (movement, «Скоро», «повтор»): surface3, muted text.
    Tag,
    /// In-card CTA («Играть» / «Начать» / «Повторить»), 32 px.
    Cta,
    /// Soft tint of the given colour (timed difficulty pill).
    Tint(Color),
}

/// Non-interactive pill with a centred label.
pub fn pill<'a, M: 'a>(label: impl IntoFragment<'a>, kind: Pill) -> Element<'a, M> {
    let (h, px) = match kind {
        Pill::Cta => (theme::size::PILL, 14.0),
        _ => (theme::size::TAG, 12.0),
    };
    container(txt(label).size(theme::text::SMALL).font(SANS_BOLD))
        .padding([0.0, px])
        .center_y(Length::Fixed(h))
        .style(move |_| match kind {
            Pill::Chip | Pill::Cta => theme::chip(&palette()),
            Pill::Tag => theme::tag(&palette()),
            Pill::Tint(color) => container::Style {
                text_color: Some(color),
                background: Some(Background::Color(Color { a: 0.14, ..color })),
                border: Border {
                    radius: theme::radius::PILL.into(),
                    ..Border::default()
                },
                shadow: Shadow::default(),
                snap: true,
            },
        })
        .into()
}

/// Selectable chip / segment (count chips, «20 с / 30 с», settings segments).
/// `round` gives a pill radius, otherwise the segment radius.
pub fn choice<'a, M: Clone + 'a>(
    label: impl IntoFragment<'a>,
    selected: bool,
    round: bool,
    msg: M,
) -> Element<'a, M> {
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
    .style(move |_, status| theme::choice(&palette(), selected, round, status))
    .on_press(msg)
    .into()
}

/// Segmented control: equal segments in one bordered box (question count,
/// timer seconds). `options` are (label, message); `selected` is an index.
pub fn segmented<'a, M: Clone + 'a>(
    options: Vec<(String, M)>,
    selected: Option<usize>,
) -> Element<'a, M> {
    let mut r = Row::new().spacing(2).width(Length::Fill);
    for (i, (label, msg)) in options.into_iter().enumerate() {
        let is_sel = selected == Some(i);
        r = r.push(
            button(
                container(txt(label).size(theme::text::SMALL).font(if is_sel {
                    SANS_BOLD
                } else {
                    SANS
                }))
                .center_x(Length::Fill),
            )
            .width(Length::FillPortion(1))
            .padding([6, 8])
            .style(move |_, s| theme::segment(&palette(), is_sel, s))
            .on_press(msg),
        );
    }
    container(r)
        .padding(3)
        .width(Length::Fill)
        .style(|_| theme::segmented_frame(&palette()))
        .into()
}

/// Circle / badge with centred content (`d` px diameter).
pub fn disc<'a, M: 'a>(inner: Element<'a, M>, d: f32, bg: Color, border: Color) -> Element<'a, M> {
    container(inner)
        .center(Length::Fixed(d))
        .style(move |_| container::Style {
            background: Some(Background::Color(bg)),
            border: Border {
                color: border,
                width: 1.0,
                radius: theme::radius::PILL.into(),
            },
            shadow: Shadow::default(),
            text_color: None,
            snap: true,
        })
        .into()
}

/// Horizontally centred block (replaces Space-Fill / elem / Space-Fill rows).
pub fn centered<'a, M: 'a>(el: impl Into<Element<'a, M>>) -> Element<'a, M> {
    container(el).center_x(Length::Fill).into()
}

/// Selected chip pill («Требует проверки раввином», gold-soft, gold border).
pub fn chip<'a, Message: 'a>(content: &'a str) -> Element<'a, Message> {
    pill(content, Pill::Chip)
}

/// Tag pill (surface3 bg, muted text).
pub fn tag<'a, Message: 'a>(content: &'a str) -> Element<'a, Message> {
    pill(content, Pill::Tag)
}

/// Card container: surface bg, border, radius 18, padding.
/// A modal dialog card that always fits the window: as tall as its content
/// when there is room, and scrolling inside the window when there is not
/// (a very small window or the largest font size), so the buttons stay
/// reachable. Place it in a centring container with some padding.
pub fn modal_fit<'a, Message: 'a>(card: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    iced::widget::scrollable(card)
        .width(Length::Shrink)
        .height(Length::Shrink)
        .into()
}

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
        header = header.push(back_button("Назад", message));
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

/// A measure-only copy of `content`: it takes part in layout (so it gives
/// its parent a size) but draws nothing, takes no input and is invisible to
/// widget operations. Used as the base layer of a [`iced::widget::Stack`] to
/// give a row of `Fill`-height tiles the height of their tallest content
/// inside a scrollable, where a `Fill` cross axis would otherwise collapse
/// to zero.
pub fn ghost<'a, Message: 'a>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> Element<'a, Message, Theme, Renderer> {
    Element::new(Ghost {
        content: content.into(),
    })
}

struct Ghost<'a, Message> {
    content: Element<'a, Message, Theme, Renderer>,
}

impl<Message> iced::advanced::Widget<Message, Theme, Renderer> for Ghost<'_, Message> {
    fn size(&self) -> iced::Size<Length> {
        self.content.as_widget().size()
    }
    fn size_hint(&self) -> iced::Size<Length> {
        self.content.as_widget().size_hint()
    }
    fn tag(&self) -> iced::advanced::widget::tree::Tag {
        self.content.as_widget().tag()
    }
    fn state(&self) -> iced::advanced::widget::tree::State {
        self.content.as_widget().state()
    }
    fn children(&self) -> Vec<iced::advanced::widget::Tree> {
        self.content.as_widget().children()
    }
    fn diff(&self, tree: &mut iced::advanced::widget::Tree) {
        self.content.as_widget().diff(tree);
    }
    fn layout(
        &mut self,
        tree: &mut iced::advanced::widget::Tree,
        renderer: &Renderer,
        limits: &iced::advanced::layout::Limits,
    ) -> iced::advanced::layout::Node {
        self.content.as_widget_mut().layout(tree, renderer, limits)
    }
    fn draw(
        &self,
        _tree: &iced::advanced::widget::Tree,
        _renderer: &mut Renderer,
        _theme: &Theme,
        _style: &iced::advanced::renderer::Style,
        _layout: iced::advanced::Layout<'_>,
        _cursor: iced::mouse::Cursor,
        _viewport: &Rectangle,
    ) {
    }
}

/// Make `content` at least `min` tall. If the content is shorter, it is
/// centred in the extra space; if it is taller, the widget grows with it.
/// Used as the body of a Fill-height scrollable so a short page stays
/// centred with no scrollbar and a tall one can scroll.
pub fn min_height<'a, Message: 'a>(
    min: f32,
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
) -> Element<'a, Message, Theme, Renderer> {
    Element::new(MinHeight {
        min,
        content: content.into(),
    })
}

struct MinHeight<'a, Message> {
    min: f32,
    content: Element<'a, Message, Theme, Renderer>,
}

impl<Message> Widget<Message, Theme, Renderer> for MinHeight<'_, Message> {
    fn size(&self) -> iced::Size<Length> {
        let mut size = self.content.as_widget().size();
        size.height = Length::Shrink;
        size
    }

    fn size_hint(&self) -> iced::Size<Length> {
        self.size()
    }

    fn tag(&self) -> iced::advanced::widget::tree::Tag {
        self.content.as_widget().tag()
    }

    fn state(&self) -> iced::advanced::widget::tree::State {
        self.content.as_widget().state()
    }

    fn children(&self) -> Vec<Tree> {
        self.content.as_widget().children()
    }

    fn diff(&self, tree: &mut Tree) {
        self.content.as_widget().diff(tree);
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &iced::advanced::layout::Limits,
    ) -> Node {
        let width = self.content.as_widget().size().width;
        let limits = limits.width(width);
        let content = self
            .content
            .as_widget_mut()
            .layout(tree, renderer, &limits.loose());
        let width = limits.resolve(width, Length::Shrink, content.size()).width;
        let height = content.size().height.max(self.min);
        let y = ((height - content.size().height) / 2.0).max(0.0);
        Node::with_children(
            iced::Size::new(width, height),
            vec![content.move_to((0.0, y))],
        )
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.container(None, layout.bounds());
        if let Some(child) = layout.children().next() {
            operation.traverse(&mut |operation| {
                self.content
                    .as_widget_mut()
                    .operate(tree, child, renderer, operation);
            });
        }
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: iced::mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let Some(child) = layout.children().next() else {
            return;
        };
        self.content.as_widget_mut().update(
            tree, event, child, cursor, renderer, clipboard, shell, viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: iced::mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> iced::mouse::Interaction {
        let Some(child) = layout.children().next() else {
            return iced::mouse::Interaction::None;
        };
        self.content
            .as_widget()
            .mouse_interaction(tree, child, cursor, viewport, renderer)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &iced::advanced::renderer::Style,
        layout: Layout<'_>,
        cursor: iced::mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let Some(child) = layout.children().next() else {
            return;
        };
        self.content
            .as_widget()
            .draw(tree, renderer, theme, style, child, cursor, viewport);
    }

    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut Tree,
        layout: Layout<'a>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<iced::advanced::overlay::Element<'a, Message, Theme, Renderer>> {
        let child = layout.children().next()?;
        self.content
            .as_widget_mut()
            .overlay(tree, child, renderer, viewport, translation)
    }
}

/// A confirm dialog for a shell layer above the whole window: dimmed
/// backdrop (a click on it = `on_stay`), a modal card with a serif title,
/// body text and two buttons, «stay» (ghost) and «go» (primary).
pub fn confirm_modal<'a, M: Clone + 'a>(
    title: impl IntoFragment<'a>,
    body: impl IntoFragment<'a>,
    stay_label: &'a str,
    go_label: &'a str,
    on_stay: M,
    on_go: M,
) -> Element<'a, M> {
    let p = palette();
    let dialog = container(
        column![
            txt(title)
                .font(SERIF_BOLD)
                .size(theme::text::H2)
                .color(p.text),
            txt(body).size(theme::text::BODY).color(p.text),
            row![
                iced::widget::space::horizontal(),
                btn(stay_label, Btn::Ghost, BtnSize::Md, Some(on_stay.clone())),
                btn(go_label, Btn::Primary, BtnSize::Md, Some(on_go)),
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
        ..container::Style::default()
    });
    let back = Color::from_rgba8(3, 5, 12, 0.66);
    let backdrop = button(
        container(iced::widget::space()).style(move |_| container::Style {
            background: Some(Background::Color(back)),
            ..container::Style::default()
        }),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .on_press(on_stay);
    iced::widget::stack![
        backdrop,
        iced::widget::center(modal_fit(dialog)).padding(16)
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// «Открытый режим рецензента» panel over a run: a gold kicker, a hint and
/// numbered chips `1..=count` to jump straight to any step or question
/// (`current` is 1-based), plus optional extra buttons on the right.
pub fn reviewer_jumps<'a, M: Clone + 'a>(
    hint: &'a str,
    count: usize,
    current: usize,
    on_jump: impl Fn(usize) -> M,
    extra: Vec<Element<'a, M>>,
) -> Element<'a, M> {
    let p = palette();
    let mut chips = Row::new().spacing(6);
    for n in 1..=count {
        chips = chips.push(
            button(
                container(txt(n.to_string()).size(13.0).font(if n == current {
                    SANS_BOLD
                } else {
                    SANS
                }))
                .center(Length::Fill),
            )
            .width(34.0)
            .height(30.0)
            .padding(0)
            .style(move |_, status| theme::choice(&palette(), n == current, true, status))
            .on_press(on_jump(n)),
        );
    }
    let mut head = Row::new()
        .spacing(10)
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .push(
            txt(format!("◆ {}", crate::reviewer::TITLE).to_uppercase())
                .font(SANS_BOLD)
                .size(theme::text::KICKER)
                .color(p.gold),
        )
        .push(space::horizontal());
    for e in extra {
        head = head.push(e);
    }
    container(
        column![
            head,
            txt(hint).size(theme::text::SMALL).color(p.muted),
            chips.wrap().vertical_spacing(6),
        ]
        .spacing(8)
        .width(Length::Fill),
    )
    .padding([12.0, 14.0])
    .width(Length::Fill)
    .style(move |_| container::Style {
        background: Some(Background::Color(Color { a: 0.10, ..p.gold })),
        border: Border {
            color: Color { a: 0.5, ..p.gold },
            width: 1.0,
            radius: theme::radius::SMALL.into(),
        },
        ..container::Style::default()
    })
    .into()
}
