//! Palette and widget styles per PARITY §T (dark navy-and-gold default,
//! light «Пергамент», and the timed-studio palette scoped to quiz-show screens).
//!
//! Colours are exactly the CSS custom properties from `css/app.css` (§T.1–T.3).
//! Style functions take a [`Palette`]; the active palette is also mirrored in a
//! process-global so view helpers that lack palette parameters (`widgets.rs`)
//! can style themselves consistently with the app's current theme.

use std::sync::RwLock;

use iced::gradient::Linear;
use iced::theme::Palette as IcedPalette;
use iced::widget::{button, container, progress_bar as pb, text_input as ti};
use iced::{Background, Border, Color, Gradient, Radians, Shadow, Theme, Vector};

// ---------------------------------------------------------------------------
// Palettes
// ---------------------------------------------------------------------------

/// Full application palette (superset of the 6-colour `iced::theme::Palette`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    pub bg0: Color,
    pub bg1: Color,
    pub surface: Color,
    pub surface2: Color,
    pub surface3: Color,
    pub line: Color,
    pub line2: Color,
    pub text: Color,
    pub muted: Color,
    pub gold: Color,
    pub gold2: Color,
    pub gold_soft: Color,
    pub on_gold: Color,
    /// Hebrew block text (gold2 in dark, `#5a400a` in parchment).
    pub hebrew: Color,
    pub ok: Color,
    pub ok_soft: Color,
    pub bad: Color,
    pub bad_soft: Color,
    pub info: Color,
    pub info_soft: Color,
    pub violet: Color,
    pub violet_soft: Color,
    pub flame: Color,
    pub flame_soft: Color,
    /// Background of text inputs.
    pub input_bg: Color,
}

const fn rgb(hex: u32) -> Color {
    Color::from_rgb8(
        ((hex >> 16) & 0xff) as u8,
        ((hex >> 8) & 0xff) as u8,
        (hex & 0xff) as u8,
    )
}

const fn rgba(r: u8, g: u8, b: u8, a: f32) -> Color {
    Color::from_rgba8(r, g, b, a)
}

fn alpha(color: Color, a: f32) -> Color {
    Color { a, ..color }
}

/// Linear blend of two colours (`t = 0` → `a`, `t = 1` → `b`).
fn mix(a: Color, b: Color, t: f32) -> Color {
    Color {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: a.a + (b.a - a.a) * t,
    }
}

/// The dark navy-and-gold palette (§T.1).
pub const DARK: Palette = Palette {
    bg0: rgb(0x0b1020),
    bg1: rgb(0x141a2e),
    surface: rgb(0x151c32),
    surface2: rgb(0x1b2340),
    surface3: rgb(0x232c4d),
    line: rgba(0xff, 0xff, 0xff, 0.06),
    line2: rgba(0xff, 0xff, 0xff, 0.12),
    text: rgb(0xece7da),
    muted: rgb(0xa9a496),
    gold: rgb(0xd4af37),
    gold2: rgb(0xe6c36a),
    gold_soft: rgba(0xd4, 0xaf, 0x37, 0.14),
    on_gold: rgb(0x1b1404),
    hebrew: rgb(0xe6c36a),
    ok: rgb(0x6cc49a),
    ok_soft: rgba(0x6c, 0xc4, 0x9a, 0.14),
    bad: rgb(0xe88b7a),
    bad_soft: rgba(0xe8, 0x8b, 0x7a, 0.14),
    info: rgb(0x8fb1ea),
    info_soft: rgba(0x8f, 0xb1, 0xea, 0.14),
    violet: rgb(0xb9a3ec),
    violet_soft: rgba(0xb9, 0xa3, 0xec, 0.12),
    flame: rgb(0xf0a24a),
    flame_soft: rgba(0xf0, 0xa2, 0x4a, 0.12),
    input_bg: rgb(0x1b2340),
};

/// The light «Пергамент» palette (§T.2).
pub const PARCHMENT: Palette = Palette {
    bg0: rgb(0xf3ead6),
    bg1: rgb(0xebdfc4),
    surface: rgb(0xfbf6ea),
    surface2: rgb(0xf4ecda),
    surface3: rgb(0xebdfc6),
    line: rgba(0x46, 0x32, 0x0f, 0.10),
    line2: rgba(0x46, 0x32, 0x0f, 0.20),
    text: rgb(0x2b2316),
    muted: rgb(0x6d604a),
    gold: rgb(0x9c7514),
    gold2: rgb(0x7f5f0d),
    gold_soft: rgba(0x9c, 0x75, 0x14, 0.13),
    on_gold: rgb(0xfffaf0),
    hebrew: rgb(0x5a400a),
    ok: rgb(0x2f7d55),
    ok_soft: rgba(0x2f, 0x7d, 0x55, 0.12),
    bad: rgb(0xa4402e),
    bad_soft: rgba(0xa4, 0x40, 0x2e, 0.10),
    info: rgb(0x335f9e),
    info_soft: rgba(0x33, 0x5f, 0x9e, 0.10),
    violet: rgb(0x6449a8),
    violet_soft: rgba(0x64, 0x49, 0xa8, 0.10),
    flame: rgb(0xd08a2d),
    flame_soft: rgba(0xd0, 0x8a, 0x2d, 0.12),
    input_bg: rgb(0xfffdf7),
};

/// The timed-studio palette scoped to quiz-show screens (§T.3).
pub const TIMED: Palette = Palette {
    bg0: rgb(0x05050f),
    bg1: rgb(0x0a0a24),
    surface: rgb(0x16184a),
    surface2: rgb(0x1c1f55),
    surface3: rgb(0x272b6a),
    line: rgba(0xff, 0xff, 0xff, 0.07),
    line2: rgba(0xf0, 0xcf, 0x6a, 0.22),
    text: rgb(0xf4ecd4),
    muted: rgb(0xb6aecb),
    gold: rgb(0xd4af37),
    gold2: rgb(0xf0cf6a),
    gold_soft: rgba(0xd4, 0xaf, 0x37, 0.16),
    on_gold: rgb(0x1d1402),
    hebrew: rgb(0xf0cf6a),
    ok: rgb(0x3fd17f),
    ok_soft: rgba(0x3f, 0xd1, 0x7f, 0.16),
    bad: rgb(0xff6a5c),
    bad_soft: rgba(0xff, 0x6a, 0x5c, 0.16),
    info: rgb(0x8fb1ea),
    info_soft: rgba(0x8f, 0xb1, 0xea, 0.14),
    violet: rgb(0xb9a3ec),
    violet_soft: rgba(0xb9, 0xa3, 0xec, 0.14),
    flame: rgb(0xf2a51f),
    flame_soft: rgba(0xf2, 0xa5, 0x1f, 0.16),
    input_bg: rgb(0x1c1f55),
};

// ---------------------------------------------------------------------------
// Theme kinds and the active palette
// ---------------------------------------------------------------------------

/// Which theme the app is using.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ThemeKind {
    /// Тёмная — основная (§T.1).
    #[default]
    Dark,
    /// Пергамент (§T.2).
    Parchment,
}

impl ThemeKind {
    /// The concrete palette for this theme kind.
    pub fn palette(self) -> Palette {
        match self {
            ThemeKind::Dark => DARK,
            ThemeKind::Parchment => PARCHMENT,
        }
    }

    /// Label used in settings.
    pub fn title(self) -> &'static str {
        match self {
            ThemeKind::Dark => "Тёмная",
            ThemeKind::Parchment => "Пергамент",
        }
    }

    /// The other theme (for a toggle).
    pub fn other(self) -> Self {
        match self {
            ThemeKind::Dark => ThemeKind::Parchment,
            ThemeKind::Parchment => ThemeKind::Dark,
        }
    }
}

static CURRENT: RwLock<Option<Palette>> = RwLock::new(None);

/// Sets the process-global active palette (call on theme switch).
pub fn set_current(kind: ThemeKind) {
    *CURRENT.write().expect("theme lock") = Some(kind.palette());
}

/// The active palette; defaults to [`DARK`] before [`set_current`] is called.
pub fn current() -> Palette {
    CURRENT.read().expect("theme lock").unwrap_or(DARK)
}

// ---------------------------------------------------------------------------
// iced theme
// ---------------------------------------------------------------------------

/// Builds the iced theme for a kind (used for the `application` theme).
pub fn iced_theme(kind: ThemeKind) -> Theme {
    let p = kind.palette();
    Theme::custom(
        match kind {
            ThemeKind::Dark => "Giyur Dark",
            ThemeKind::Parchment => "Giyur Parchment",
        },
        IcedPalette {
            background: p.bg0,
            text: p.text,
            primary: p.gold,
            success: p.ok,
            warning: p.gold2,
            danger: p.bad,
        },
    )
}

// ---------------------------------------------------------------------------
// Radii, spacing, type scale (§T.4, §T.5)
// ---------------------------------------------------------------------------

/// Border radii in px.
pub mod radius {
    /// Cards.
    pub const CARD: f32 = 18.0;
    /// Small boxes (hebrew figure, differs notes).
    pub const SMALL: f32 = 12.0;
    /// Buttons.
    pub const BUTTON: f32 = 14.0;
    /// Chips, tags, pills.
    pub const PILL: f32 = 999.0;
    /// Segment buttons / step segments.
    pub const SEGMENT: f32 = 10.0;
    /// Modals / dialogs.
    pub const MODAL: f32 = 20.0;
    /// Flashcard flip card.
    pub const FLIP: f32 = 22.0;
    /// Section / badge emblems.
    pub const EMBLEM: f32 = 16.0;
    /// Large emblems (section header).
    pub const EMBLEM_LG: f32 = 22.0;
    /// Welcome hero.
    pub const HERO: f32 = 24.0;
    /// Timed quiz-show container.
    pub const TIMED: f32 = 26.0;
}

/// Spacing / layout constants.
pub mod spacing {
    /// Card padding (1.35 rem).
    pub const CARD_PAD: f32 = 21.6;
    /// Gap between rows (.6 rem).
    pub const ROW: f32 = 9.6;
    /// Content max width.
    pub const CONTENT_MAX: f32 = 820.0;
    /// Minimum button height.
    pub const BTN_MIN: f32 = 46.0;
    /// Minimum large button height.
    pub const BTN_LG_MIN: f32 = 52.0;
    /// Minimum chip height.
    pub const CHIP_MIN: f32 = 44.0;
    /// Minimum input height.
    pub const INPUT_MIN: f32 = 46.0;
}

/// Type scale (root 17 px, §T.4).
pub mod text {
    /// Body text.
    pub const BODY: f32 = 17.0;
    /// h1 (2 rem).
    pub const H1: f32 = 34.0;
    /// h2 (1.4 rem).
    pub const H2: f32 = 24.0;
    /// h3 / .h2 (1.12 rem).
    pub const H3: f32 = 19.0;
    /// Small text (.85 rem).
    pub const SMALL: f32 = 14.5;
    /// Kicker (.72 rem, uppercase, letter-spaced).
    pub const KICKER: f32 = 12.25;
    /// Hebrew blocks (1.85 em).
    pub const HEBREW: f32 = 31.5;
}

// ---------------------------------------------------------------------------
// Gradient helpers
// ---------------------------------------------------------------------------

fn v_gradient(top: Color, bottom: Color) -> Background {
    // CSS `linear-gradient(180deg, …)` (top → bottom) is a quarter turn
    // clockwise from the +x axis in iced's angle convention.
    Background::Gradient(Gradient::Linear(
        Linear::new(Radians(std::f32::consts::FRAC_PI_2))
            .add_stop(0.0, top)
            .add_stop(1.0, bottom),
    ))
}

fn gold_background(p: &Palette) -> Background {
    v_gradient(p.gold2, p.gold)
}

fn is_light(color: &Color) -> bool {
    0.299 * color.r + 0.587 * color.g + 0.114 * color.b > 0.5
}

// ---------------------------------------------------------------------------
// Buttons (§T.6)
// ---------------------------------------------------------------------------

fn button_style(background: Option<Background>, text_color: Color) -> button::Style {
    button::Style {
        background,
        text_color,
        border: Border::default(),
        shadow: Shadow::default(),
        snap: true,
    }
}

/// Primary gold button: `linear-gradient(180deg, gold2, gold)`, on-gold text.
pub fn button_primary(p: &Palette, status: button::Status) -> button::Style {
    let style = button::Style {
        border: Border {
            radius: radius::BUTTON.into(),
            ..Border::default()
        },
        ..button_style(Some(gold_background(p)), p.on_gold)
    };
    match status {
        button::Status::Active => button::Style {
            shadow: Shadow {
                color: alpha(p.gold, 0.25),
                offset: Vector::new(0.0, 6.0),
                blur_radius: 18.0,
            },
            ..style
        },
        button::Status::Hovered => button::Style {
            background: Some(v_gradient(
                mix(p.gold2, Color::WHITE, 0.15),
                mix(p.gold, Color::WHITE, 0.10),
            )),
            ..style
        },
        button::Status::Pressed => button::Style {
            background: Some(v_gradient(p.gold, p.gold2)),
            shadow: Shadow::default(),
            ..style
        },
        button::Status::Disabled => button::Style {
            text_color: alpha(p.on_gold, 0.5),
            ..style
        },
    }
}

/// Secondary button: surface2 bg, 1 px line2 border.
pub fn button_secondary(p: &Palette, status: button::Status) -> button::Style {
    let border = Border {
        color: p.line2,
        width: 1.0,
        radius: radius::BUTTON.into(),
    };
    let style = button_style(Some(Background::Color(p.surface2)), p.text);
    match status {
        button::Status::Hovered | button::Status::Pressed => button::Style {
            background: Some(Background::Color(p.surface3)),
            border,
            ..style
        },
        button::Status::Disabled => button::Style {
            text_color: alpha(p.text, 0.45),
            border,
            ..style
        },
        button::Status::Active => button::Style { border, ..style },
    }
}

/// Ghost / link button: transparent with a 1 px border.
pub fn button_ghost(p: &Palette, status: button::Status) -> button::Style {
    let border = Border {
        color: p.line2,
        width: 1.0,
        radius: radius::BUTTON.into(),
    };
    let style = button_style(None, p.text);
    match status {
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(p.surface)),
            border,
            ..style
        },
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(p.surface2)),
            border,
            ..style
        },
        button::Status::Disabled => button::Style {
            text_color: alpha(p.text, 0.45),
            border,
            ..style
        },
        button::Status::Active => button::Style { border, ..style },
    }
}

/// Danger button: bad-soft bg, bad text, bad 40 % border.
pub fn button_danger(p: &Palette, status: button::Status) -> button::Style {
    let border = Border {
        color: alpha(p.bad, 0.4),
        width: 1.0,
        radius: radius::BUTTON.into(),
    };
    let style = button_style(Some(Background::Color(p.bad_soft)), p.bad);
    match status {
        button::Status::Hovered | button::Status::Pressed => button::Style {
            background: Some(Background::Color(alpha(p.bad, 0.22))),
            border,
            ..style
        },
        _ => button::Style { border, ..style },
    }
}

/// Timed «Начать игру» gold button (§T.3 `tq-go`); min-width applied by caller.
pub fn button_tq_go(p: &Palette, status: button::Status) -> button::Style {
    let style = button::Style {
        border: Border {
            radius: radius::BUTTON.into(),
            ..Border::default()
        },
        ..button_style(Some(v_gradient(rgb(0xf6dc86), p.gold)), p.on_gold)
    };
    match status {
        button::Status::Hovered => button::Style {
            background: Some(v_gradient(rgb(0xf9e6a8), rgb(0xe6c36a))),
            ..style
        },
        button::Status::Pressed => button::Style {
            background: Some(v_gradient(rgb(0xd4af37), rgb(0xa8821c))),
            ..style
        },
        button::Status::Disabled => button::Style {
            text_color: alpha(p.on_gold, 0.5),
            ..style
        },
        button::Status::Active => style,
    }
}

// ---------------------------------------------------------------------------
// Quiz option buttons
// ---------------------------------------------------------------------------

fn option_style(bg: Color, text: Color, border_color: Color) -> button::Style {
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: text,
        border: Border {
            color: border_color,
            width: 1.0,
            radius: radius::BUTTON.into(),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

/// Quiz option, normal state.
pub fn option_normal(p: &Palette, status: button::Status) -> button::Style {
    let style = option_style(p.surface2, p.text, p.line2);
    match status {
        button::Status::Hovered | button::Status::Pressed => button::Style {
            background: Some(Background::Color(p.surface3)),
            ..style
        },
        _ => style,
    }
}

/// Quiz option, selected (highlighted before reveal).
pub fn option_selected(p: &Palette, status: button::Status) -> button::Style {
    let style = option_style(p.gold_soft, p.gold2, p.gold);
    match status {
        button::Status::Hovered | button::Status::Pressed => button::Style {
            border: Border {
                color: p.gold2,
                ..style.border
            },
            ..style
        },
        _ => style,
    }
}

/// Quiz option, correct answer.
pub fn option_correct(p: &Palette, status: button::Status) -> button::Style {
    let _ = status;
    option_style(p.ok_soft, p.ok, p.ok)
}

/// Quiz option, wrong (chosen) answer.
pub fn option_wrong(p: &Palette, status: button::Status) -> button::Style {
    let _ = status;
    option_style(p.bad_soft, p.bad, p.bad)
}

// ---------------------------------------------------------------------------
// Containers
// ---------------------------------------------------------------------------

fn container_style(p: &Palette, background: Option<Background>) -> container::Style {
    container::Style {
        text_color: Some(p.text),
        background,
        border: Border::default(),
        shadow: Shadow::default(),
        snap: true,
    }
}

fn card_shadow(p: &Palette) -> Shadow {
    if is_light(&p.bg0) {
        Shadow {
            color: rgba(0x50, 0x37, 0x0f, 0.10),
            offset: Vector::new(0.0, 3.0),
            blur_radius: 10.0,
        }
    } else {
        Shadow {
            color: rgba(0x00, 0x00, 0x00, 0.28),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 14.0,
        }
    }
}

/// Card: surface bg, 1 px line border, radius 18, soft shadow.
pub fn card(p: &Palette) -> container::Style {
    container::Style {
        border: Border {
            color: p.line,
            width: 1.0,
            radius: radius::CARD.into(),
        },
        shadow: card_shadow(p),
        ..container_style(p, Some(Background::Color(p.surface)))
    }
}

/// Hero / banner container: bg1 → bg0 vertical gradient, radius 24.
pub fn hero(p: &Palette) -> container::Style {
    container::Style {
        border: Border {
            color: p.line,
            width: 1.0,
            radius: radius::HERO.into(),
        },
        ..container_style(p, Some(v_gradient(p.bg1, p.bg0)))
    }
}

/// Timed quiz-show studio container (§T.3; radial gradient approximated).
pub fn tq_container(p: &Palette) -> container::Style {
    container::Style {
        border: Border {
            color: alpha(p.gold, 0.28),
            width: 1.0,
            radius: radius::TIMED.into(),
        },
        ..container_style(p, Some(v_gradient(rgb(0x1a1650), rgb(0x0a0a24))))
    }
}

/// Tag pill: surface3 bg, muted text (§T.6 «Tags»).
pub fn tag(p: &Palette) -> container::Style {
    container::Style {
        text_color: Some(p.muted),
        border: Border {
            radius: radius::PILL.into(),
            ..Border::default()
        },
        ..container_style(p, Some(Background::Color(p.surface3)))
    }
}

/// Selected chip / segment pill: gold-soft bg, gold border, gold2 text.
pub fn chip(p: &Palette) -> container::Style {
    container::Style {
        text_color: Some(p.gold2),
        border: Border {
            color: p.gold,
            width: 1.0,
            radius: radius::PILL.into(),
        },
        ..container_style(p, Some(Background::Color(p.gold_soft)))
    }
}

/// Movement for «differs» notes (§T.8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Movement {
    Reform,
    Conservative,
    Orthodox,
    Other,
}

impl Movement {
    /// Russian label shown before the note text (§T.8).
    pub fn label(self) -> &'static str {
        match self {
            Movement::Reform => "В реформистском иудаизме иначе:",
            Movement::Conservative => "В консервативном иудаизме иначе:",
            Movement::Orthodox => "В ортодоксальном иудаизме иначе:",
            Movement::Other => "В других течениях иначе:",
        }
    }

    /// (accent, soft tint) colour pair for the note box.
    pub fn accent_colors(self, p: &Palette) -> (Color, Color) {
        match self {
            Movement::Reform => (p.violet, p.violet_soft),
            Movement::Conservative => (p.ok, p.ok_soft),
            Movement::Orthodox => (p.info, p.info_soft),
            Movement::Other => (p.muted, p.surface3),
        }
    }
}

/// «differs» note box: soft tint of the movement accent; the 3 px accent
/// left bar is added by the layout in `widgets::differs_note`.
pub fn differs(p: &Palette, movement: Movement) -> container::Style {
    let (_, soft) = movement.accent_colors(p);
    container::Style {
        border: Border {
            radius: radius::SMALL.into(),
            ..Border::default()
        },
        ..container_style(p, Some(Background::Color(soft)))
    }
}

/// Quote block body; the gold left bar is added by `widgets::quote_block`.
pub fn quote(p: &Palette) -> container::Style {
    container_style(p, None)
}

/// Note / tip box («Совет»): info tint, subtle info border.
pub fn note(p: &Palette) -> container::Style {
    container::Style {
        border: Border {
            color: alpha(p.info, 0.25),
            width: 1.0,
            radius: radius::SMALL.into(),
        },
        ..container_style(p, Some(Background::Color(p.info_soft)))
    }
}

/// Term box (dashed in CSS; approximated with a 1 px border).
pub fn term(p: &Palette) -> container::Style {
    container::Style {
        border: Border {
            color: p.line2,
            width: 1.0,
            radius: radius::SMALL.into(),
        },
        ..container_style(p, Some(Background::Color(p.surface2)))
    }
}

/// Toast pill: surface2 bg, border, floating shadow.
pub fn toast(p: &Palette) -> container::Style {
    container::Style {
        border: Border {
            color: p.line2,
            width: 1.0,
            radius: radius::BUTTON.into(),
        },
        shadow: Shadow {
            color: rgba(0x00, 0x00, 0x00, 0.38),
            offset: Vector::new(0.0, 8.0),
            blur_radius: 24.0,
        },
        ..container_style(p, Some(Background::Color(p.surface2)))
    }
}

/// Hebrew figure box: gold-tinted gradient on surface2, 25 % gold border.
pub fn hebrew_block(p: &Palette) -> container::Style {
    container::Style {
        border: Border {
            color: alpha(p.gold, 0.25),
            width: 1.0,
            radius: radius::SMALL.into(),
        },
        ..container_style(p, Some(v_gradient(alpha(p.gold, 0.10), p.surface2)))
    }
}

// ---------------------------------------------------------------------------
// Progress bar & text input
// ---------------------------------------------------------------------------

/// Progress bar: surface3 track, gold gradient fill (§T.6).
pub fn progress_bar(p: &Palette) -> pb::Style {
    pb::Style {
        background: Background::Color(p.surface3),
        bar: gold_background(p),
        border: Border {
            radius: radius::PILL.into(),
            ..Border::default()
        },
    }
}

/// Text input (§T.6 focus: gold border).
pub fn text_input(p: &Palette, status: ti::Status) -> ti::Style {
    let style = |border: Border| ti::Style {
        background: Background::Color(p.input_bg),
        border,
        icon: p.muted,
        placeholder: p.muted,
        value: p.text,
        selection: p.gold_soft,
    };
    let border = |width: f32, color: Color| Border {
        color,
        width,
        radius: radius::BUTTON.into(),
    };
    match status {
        ti::Status::Active => style(border(1.0, p.line2)),
        ti::Status::Hovered => style(border(1.0, p.muted)),
        ti::Status::Focused { .. } => style(border(2.0, p.gold2)),
        ti::Status::Disabled => ti::Style {
            background: Background::Color(p.surface2),
            border: border(1.0, p.line),
            icon: p.muted,
            placeholder: alpha(p.muted, 0.45),
            value: alpha(p.text, 0.45),
            selection: p.gold_soft,
        },
    }
}

// ---------------------------------------------------------------------------
// Shell (sidebar, window background)
// ---------------------------------------------------------------------------

/// Window background: bg1 → bg0 vertical gradient (body background).
pub fn app_background(p: &Palette) -> container::Style {
    container_style(p, Some(v_gradient(p.bg1, p.bg0)))
}

/// Sidebar: slightly translucent bg0 with a right hairline.
pub fn sidebar(p: &Palette) -> container::Style {
    container::Style {
        border: Border {
            color: p.line,
            width: 1.0,
            radius: 0.0.into(),
        },
        ..container_style(p, Some(Background::Color(alpha(p.bg0, 0.88))))
    }
}

/// Active sidebar item: gold2 text on gold-soft.
pub fn button_nav_active(p: &Palette, _status: button::Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(p.gold_soft)),
        text_color: p.gold2,
        border: Border {
            color: p.gold,
            width: 0.0,
            radius: radius::BUTTON.into(),
        },
        ..button_style(None, p.gold2)
    }
}
