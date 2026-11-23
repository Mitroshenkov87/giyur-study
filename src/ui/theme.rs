//! Palette and widget styles: the dark «Библиотека» default (walnut, oxblood
//! leather, brass, lamp-light) and the light calf-hide «Кожа».
//!
//! Style functions take a [`Palette`]; the active palette is also mirrored in a
//! process-global so view helpers that lack palette parameters (`widgets.rs`)
//! can style themselves consistently with the app's current theme.

use std::sync::RwLock;

use iced::gradient::Linear;
use iced::theme::Palette as IcedPalette;
use iced::widget::{button, container, progress_bar as pb, rule, text_input as ti};
use iced::{Background, Border, Color, Gradient, Radians, Shadow, Theme, Vector};

use crate::i18n::tr;

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
    /// Hebrew block text (gold2 in dark, `#4a2e06` in «Кожа»).
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
    /// Window background texture (asset name).
    pub texture: &'static str,
    /// Sidebar background texture (asset name).
    pub side_texture: &'static str,
    /// Solid veil composited over the window texture.
    pub veil: Color,
    /// Solid veil composited over the sidebar texture.
    pub side_veil: Color,
    /// Extra tint over the sidebar veil (transparent when unused).
    pub side_tint: Color,
    /// Warm lamp highlight.
    pub lamp: Color,
    /// Top/bottom edge vignette (transparent when unused).
    pub vignette: Color,
    /// Primary-button brass gradient: highlight stop.
    pub brass_hi: Color,
    /// Primary-button brass gradient: middle stop.
    pub brass_mid: Color,
    /// Primary-button brass gradient: low stop.
    pub brass_lo: Color,
    /// Primary-button 1 px brass edge.
    pub brass_edge: Color,
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

/// Hex + alpha helper for the translucent palette entries.
const fn hexa(hex: u32, a: f32) -> Color {
    Color::from_rgba8(
        ((hex >> 16) & 0xff) as u8,
        ((hex >> 8) & 0xff) as u8,
        (hex & 0xff) as u8,
        a,
    )
}

/// The dark «Библиотека» palette: walnut, oxblood leather, brass, lamp-light.
pub const DARK: Palette = Palette {
    bg0: rgb(0x120b08),
    bg1: rgb(0x1c120d),
    surface: rgb(0x241710),
    surface2: rgb(0x2e1e15),
    surface3: rgb(0x3a271b),
    line: hexa(0xc9a25a, 0.22),
    line2: hexa(0xc9a25a, 0.40),
    text: rgb(0xefe3cc),
    muted: rgb(0xbfae8f),
    gold: rgb(0xc9a25a),
    gold2: rgb(0xe3c27e),
    gold_soft: hexa(0xc9a25a, 0.14),
    on_gold: rgb(0x23160b),
    hebrew: rgb(0xe3c27e),
    ok: rgb(0x8cc49a),
    ok_soft: hexa(0x8cc49a, 0.14),
    bad: rgb(0xe39580),
    bad_soft: hexa(0xe39580, 0.14),
    info: rgb(0x9fb7d9),
    info_soft: hexa(0x9fb7d9, 0.14),
    violet: rgb(0xc0a6e0),
    violet_soft: hexa(0xc0a6e0, 0.12),
    flame: rgb(0xe8a052),
    flame_soft: hexa(0xe8a052, 0.12),
    input_bg: rgb(0x1a110c),
    // Veil, side veil and lamp light are baked into the WebP backgrounds
    // (tools/bake-textures.py), so they are fully transparent here.
    texture: "bg-dark",
    side_texture: "side-dark",
    veil: hexa(0x120b08, 0.0),
    side_veil: hexa(0x120b08, 0.0),
    side_tint: hexa(0x000000, 0.0),
    lamp: hexa(0xffbe6e, 0.0),
    vignette: hexa(0x000000, 0.0),
    brass_hi: rgb(0xecd08f),
    brass_mid: rgb(0xd2ad62),
    brass_lo: rgb(0xb08a45),
    brass_edge: rgb(0x7a5a26),
};

/// The light «Кожа» palette, v0.5: smoothed calf hide. A warm
/// tan/caramel ground (relative luminance ≈ 0.25, darker than paper so it
/// does not glare), lighter tooled-hide cards (≈ 0.40) with a darker
/// pressed edge, and a dark leather sidebar with cream ink
/// ([`side_palette`]). Body text is ≥ 7.5:1 on cards; `muted` ≈ 6.5:1.
pub const LEATHER: Palette = Palette {
    bg0: rgb(0x9c7650),
    bg1: rgb(0xa8825a),
    surface: rgb(0xcaa67d),
    surface2: rgb(0xc09c73),
    surface3: rgb(0xb5916a),
    line: hexa(0x4a2e14, 0.24),
    line2: hexa(0x4a2e14, 0.42),
    text: rgb(0x22140a),
    muted: rgb(0x35240f),
    gold: rgb(0x6a4810),
    gold2: rgb(0x4a2e06),
    gold_soft: hexa(0x6a4810, 0.14),
    on_gold: rgb(0x22140a),
    hebrew: rgb(0x4a2e06),
    ok: rgb(0x1d4529),
    ok_soft: hexa(0x1d4529, 0.12),
    bad: rgb(0x7a2414),
    bad_soft: hexa(0x7a2414, 0.10),
    info: rgb(0x1f3c66),
    info_soft: hexa(0x1f3c66, 0.10),
    violet: rgb(0x452c72),
    violet_soft: hexa(0x452c72, 0.10),
    flame: rgb(0x6a3c08),
    flame_soft: hexa(0x6a3c08, 0.12),
    input_bg: rgb(0xd3b189),
    // Veil, tint, lamp and vignette are baked into the WebP backgrounds.
    texture: "bg-leather",
    side_texture: "side-leather",
    veil: hexa(0xe8d6b0, 0.0),
    side_veil: hexa(0xe8d6b0, 0.0),
    side_tint: hexa(0x6b4712, 0.0),
    lamp: hexa(0xfff1cc, 0.0),
    vignette: hexa(0x3a2510, 0.0),
    brass_hi: rgb(0xd9b56a),
    brass_mid: rgb(0xc8a258),
    brass_lo: rgb(0xb8914a),
    brass_edge: rgb(0x5a3a10),
};

/// Palette for the sidebar. In the light theme the sidebar is a strip of
/// dark leather, so it gets cream ink and brighter brass (≥ 8:1 on the
/// strip); the dark theme uses its palette unchanged.
pub fn side_palette(p: Palette) -> Palette {
    if !is_light(&p.text) {
        return p;
    }
    Palette {
        text: rgb(0xf3e6cc),
        muted: rgb(0xd8c3a0),
        gold: rgb(0xd9b56a),
        gold2: rgb(0xf0d79a),
        gold_soft: hexa(0xd9b56a, 0.16),
        line: hexa(0xf0d79a, 0.18),
        line2: hexa(0xf0d79a, 0.32),
        ..p
    }
}

// ---------------------------------------------------------------------------
// Theme kinds and the active palette
// ---------------------------------------------------------------------------

/// Which theme the app is using.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ThemeKind {
    /// Тёмная — основная (§T.1).
    #[default]
    Dark,
    /// Кожа — the light hide theme (§T.2; «Пергамент» before v0.5).
    Leather,
}

impl ThemeKind {
    /// The concrete palette for this theme kind.
    pub fn palette(self) -> Palette {
        match self {
            ThemeKind::Dark => DARK,
            ThemeKind::Leather => LEATHER,
        }
    }

    /// Label used in settings.
    pub fn title(self) -> &'static str {
        match self {
            ThemeKind::Dark => tr("Тёмная"),
            ThemeKind::Leather => tr("Кожа"),
        }
    }

    /// The other theme (for a toggle).
    pub fn other(self) -> Self {
        match self {
            ThemeKind::Dark => ThemeKind::Leather,
            ThemeKind::Leather => ThemeKind::Dark,
        }
    }
}

static CURRENT: RwLock<Option<(Palette, ThemeKind)>> = RwLock::new(None);

/// Sets the process-global active palette (call on theme switch).
pub fn set_current(kind: ThemeKind) {
    *CURRENT.write().expect("theme lock") = Some((kind.palette(), kind));
}

/// The active theme kind; [`ThemeKind::Dark`] before [`set_current`].
pub fn current_kind() -> ThemeKind {
    CURRENT
        .read()
        .expect("theme lock")
        .map_or(ThemeKind::Dark, |(_, k)| k)
}

/// Suffix of the theme's rendered art variants (`assets/img/icons/*-wood.webp`
/// for the dark wood library, `*-leather.webp` for «Кожа»).
pub fn art_variant() -> &'static str {
    match current_kind() {
        ThemeKind::Dark => "wood",
        ThemeKind::Leather => "leather",
    }
}

/// The active palette; defaults to [`DARK`] before [`set_current`] is called.
pub fn current() -> Palette {
    CURRENT.read().expect("theme lock").map_or(DARK, |(p, _)| p)
}

// ---------------------------------------------------------------------------
// iced theme
// ---------------------------------------------------------------------------

/// Builds the iced theme for a kind (used for the `application` theme).
pub fn iced_theme(kind: ThemeKind) -> Theme {
    let p = kind.palette();
    Theme::custom(
        match kind {
            ThemeKind::Dark => "Shorashim Dark",
            ThemeKind::Leather => "Shorashim Leather",
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
    pub const BUTTON: f32 = 12.0;
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
}

/// Control heights: every fixed-height control uses one of these.
pub mod size {
    /// Back, exit, ghost links, segments, count chips.
    pub const BTN_SM: f32 = 36.0;
    /// Dialog and option buttons (= `spacing::BTN_MIN`).
    pub const BTN_MD: f32 = 46.0;
    /// Page CTAs (= `spacing::BTN_LG_MIN`).
    pub const BTN_LG: f32 = 52.0;
    /// In-card CTA pills (Играть / Начать / Повторить).
    pub const PILL: f32 = 32.0;
    /// Tags, difficulty, review chip.
    pub const TAG: f32 = 24.0;
    /// Sidebar items (single line).
    pub const NAV: f32 = 40.0;
    /// Sidebar items with a subtitle line (label + grey description).
    pub const NAV_ITEM: f32 = 50.0;
    /// Rendered 3D icon in a sidebar item.
    pub const NAV_ICON: f32 = 34.0;
    /// The same icon under the mouse (also the icon's fixed box).
    pub const NAV_ICON_HOVER: f32 = 40.0;
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

/// Three-stop top → bottom gradient.
fn v_gradient3(top: Color, mid: Color, bottom: Color) -> Background {
    Background::Gradient(Gradient::Linear(
        Linear::new(Radians(std::f32::consts::FRAC_PI_2))
            .add_stop(0.0, top)
            .add_stop(0.5, mid)
            .add_stop(1.0, bottom),
    ))
}

fn gold_background(p: &Palette) -> Background {
    v_gradient(p.gold2, p.gold)
}

/// Brass (primary) gradient; `lift` mixes every stop towards `toward`.
fn brass_background(p: &Palette, toward: Color, lift: f32) -> Background {
    v_gradient3(
        mix(p.brass_hi, toward, lift),
        mix(p.brass_mid, toward, lift),
        mix(p.brass_lo, toward, lift),
    )
}

/// A light theme is one with dark ink (the hide theme's background is a
/// mid-tone tan, so the background alone no longer tells).
fn is_light(text: &Color) -> bool {
    0.299 * text.r + 0.587 * text.g + 0.114 * text.b < 0.5
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

/// Primary brass button: three-stop brass gradient, 1 px brass edge,
/// on-gold label, warm drop shadow.
pub fn button_primary(p: &Palette, status: button::Status) -> button::Style {
    let light = is_light(&p.text);
    let border = Border {
        color: p.brass_edge,
        width: 1.0,
        radius: radius::BUTTON.into(),
    };
    let style = button::Style {
        border,
        ..button_style(Some(brass_background(p, Color::WHITE, 0.0)), p.on_gold)
    };
    let shadow = if light {
        Shadow {
            color: hexa(0x3a2510, 0.20),
            offset: Vector::new(0.0, 2.0),
            blur_radius: 6.0,
        }
    } else {
        Shadow {
            color: alpha(p.brass_mid, 0.22),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 14.0,
        }
    };
    match status {
        button::Status::Active => button::Style { shadow, ..style },
        button::Status::Hovered => button::Style {
            background: Some(if light {
                brass_background(p, rgb(0xfff1cc), 0.10)
            } else {
                brass_background(p, Color::WHITE, 0.08)
            }),
            shadow,
            ..style
        },
        button::Status::Pressed => button::Style {
            background: Some(v_gradient3(p.brass_lo, p.brass_mid, p.brass_hi)),
            shadow: Shadow::default(),
            ..style
        },
        button::Status::Disabled => button::Style {
            background: Some(Background::Color(p.surface3)),
            text_color: alpha(p.muted, 0.7),
            border: Border {
                color: p.line,
                ..border
            },
            ..style
        },
    }
}

/// Secondary button: leather (dark) / darker hide («Кожа») gradient with
/// a brass border.
pub fn button_secondary(p: &Palette, status: button::Status) -> button::Style {
    let border = Border {
        color: alpha(p.gold, 0.55),
        width: 1.0,
        radius: radius::BUTTON.into(),
    };
    let base = v_gradient(p.surface2, mix(p.surface2, p.bg0, 0.25));
    let style = button::Style {
        border,
        ..button_style(Some(base), p.text)
    };
    match status {
        button::Status::Hovered | button::Status::Pressed => button::Style {
            background: Some(v_gradient(
                mix(p.surface2, p.gold, p.gold_soft.a),
                mix(mix(p.surface2, p.bg0, 0.25), p.gold, p.gold_soft.a),
            )),
            border: Border {
                color: p.gold2,
                ..border
            },
            ..style
        },
        button::Status::Disabled => button::Style {
            text_color: alpha(p.text, 0.45),
            border: Border {
                color: p.line,
                ..border
            },
            ..style
        },
        button::Status::Active => style,
    }
}

/// Ghost / link button: transparent with a 1 px line2 border.
pub fn button_ghost(p: &Palette, status: button::Status) -> button::Style {
    let border = Border {
        color: p.line2,
        width: 1.0,
        radius: radius::BUTTON.into(),
    };
    let style = button_style(None, p.text);
    match status {
        button::Status::Hovered | button::Status::Pressed => button::Style {
            background: Some(Background::Color(p.gold_soft)),
            border: Border {
                color: alpha(p.gold, 0.6),
                ..border
            },
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

/// Sidebar item: no box at rest, a soft wash on hover.
pub fn button_nav(p: &Palette, status: button::Status) -> button::Style {
    let style = button_style(None, p.text);
    match status {
        button::Status::Hovered | button::Status::Pressed => button::Style {
            background: Some(Background::Color(alpha(p.gold, 0.06))),
            text_color: p.gold2,
            border: Border {
                radius: radius::BUTTON.into(),
                ..Border::default()
            },
            ..style
        },
        _ => style,
    }
}

/// Pick list in the theme's materials (iced's default uses the iced
/// palette's primary colour, a solid dark block on «Кожа»).
pub fn pick_list(
    p: &Palette,
    status: iced::widget::pick_list::Status,
) -> iced::widget::pick_list::Style {
    use iced::widget::pick_list::Status;
    let (edge, bg) = match status {
        Status::Active => (p.line2, p.input_bg),
        Status::Hovered => (alpha(p.gold, 0.6), p.input_bg),
        Status::Opened { .. } => (p.gold, p.surface3),
    };
    iced::widget::pick_list::Style {
        text_color: p.text,
        placeholder_color: p.muted,
        handle_color: p.gold2,
        background: Background::Color(bg),
        border: Border {
            color: edge,
            width: 1.0,
            radius: radius::SEGMENT.into(),
        },
    }
}

/// The drop-down menu of [`pick_list`].
pub fn pick_menu(p: &Palette) -> iced::overlay::menu::Style {
    iced::overlay::menu::Style {
        background: Background::Color(p.surface),
        border: Border {
            color: p.line2,
            width: 1.0,
            radius: radius::SEGMENT.into(),
        },
        text_color: p.text,
        selected_text_color: p.gold2,
        selected_background: Background::Color(p.gold_soft),
        shadow: Shadow::default(),
    }
}

/// Outer frame of a segmented control: one bordered rounded box.
pub fn segmented_frame(p: &Palette) -> container::Style {
    container::Style {
        background: Some(Background::Color(p.surface2)),
        border: Border {
            color: p.line2,
            width: 1.0,
            radius: radius::SEGMENT.into(),
        },
        ..container::Style::default()
    }
}

/// One segment inside [`segmented_frame`]: flat; the selected one is a
/// brass-tinted inset with a brass edge.
pub fn segment(p: &Palette, selected: bool, status: button::Status) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    let (bg, text_color, border_color) = if selected {
        (Some(Background::Color(p.gold_soft)), p.gold2, p.gold)
    } else if hovered {
        (
            Some(Background::Color(alpha(p.gold, 0.07))),
            p.text,
            Color::TRANSPARENT,
        )
    } else {
        (None, p.text, Color::TRANSPARENT)
    };
    button::Style {
        background: bg,
        text_color,
        border: Border {
            color: border_color,
            width: if selected { 1.0 } else { 0.0 },
            radius: (radius::SEGMENT - 3.0).into(),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

/// Selectable chip / segment (count chips, «20 с / 30 с», settings segments).
/// `round` gives a pill radius, otherwise the segment radius.
pub fn choice(p: &Palette, selected: bool, round: bool, status: button::Status) -> button::Style {
    let radius = if round { radius::PILL } else { radius::SEGMENT };
    let (bg, border_color, text_color) = if selected {
        (p.gold_soft, p.gold, p.gold2)
    } else {
        (p.surface2, p.line2, p.text)
    };
    let style = button::Style {
        background: Some(Background::Color(bg)),
        text_color,
        border: Border {
            color: border_color,
            width: 1.0,
            radius: radius.into(),
        },
        shadow: Shadow::default(),
        snap: true,
    };
    match status {
        button::Status::Hovered | button::Status::Pressed if !selected => button::Style {
            background: Some(Background::Color(p.surface3)),
            border: Border {
                color: alpha(p.gold, 0.6),
                ..style.border
            },
            ..style
        },
        button::Status::Disabled => button::Style {
            text_color: alpha(text_color, 0.45),
            ..style
        },
        _ => style,
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
    if is_light(&p.text) {
        Shadow {
            color: rgba(0x3a, 0x25, 0x10, 0.18),
            offset: Vector::new(0.0, 3.0),
            blur_radius: 10.0,
        }
    } else {
        Shadow {
            color: rgba(0x00, 0x00, 0x00, 0.45),
            offset: Vector::new(0.0, 6.0),
            blur_radius: 18.0,
        }
    }
}

/// Card: translucent surface over the window texture (the grain stays
/// visible); the top edge is slightly lamp-lit. 1 px brass border.
pub fn card(p: &Palette) -> container::Style {
    let light = is_light(&p.text);
    let lamp_rgb = Color { a: 1.0, ..p.lamp };
    let border_color = if light {
        // Pressed (tooled) edge of a hide panel.
        hexa(0x4a2e14, 0.50)
    } else {
        alpha(p.gold, 0.35)
    };
    container::Style {
        border: Border {
            color: border_color,
            width: 1.0,
            radius: radius::CARD.into(),
        },
        shadow: card_shadow(p),
        ..container_style(
            p,
            Some(v_gradient(
                alpha(mix(p.surface, lamp_rgb, 0.06), 0.90),
                alpha(p.surface, 0.90),
            )),
        )
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
    /// Label shown before the note text (§T.8).
    pub fn label(self) -> &'static str {
        match self {
            Movement::Reform => tr("В реформистском иудаизме иначе:"),
            Movement::Conservative => tr("В консервативном иудаизме иначе:"),
            Movement::Orthodox => tr("В ортодоксальном иудаизме иначе:"),
            Movement::Other => tr("В других течениях иначе:"),
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
// Shell (sidebar, window background, lamp, dividers)
// ---------------------------------------------------------------------------

/// Window background fallback when the texture is missing: plain bg0.
pub fn app_background(p: &Palette) -> container::Style {
    container_style(p, Some(Background::Color(p.bg0)))
}

/// Solid veil composited over the window texture.
pub fn veil(p: &Palette) -> container::Style {
    container_style(p, Some(Background::Color(p.veil)))
}

/// Reading-lamp highlight, vertical: a warm pool from the top.
pub fn lamp_v(p: &Palette) -> container::Style {
    container_style(
        p,
        Some(Background::Gradient(Gradient::Linear(
            Linear::new(Radians(std::f32::consts::FRAC_PI_2))
                .add_stop(0.0, p.lamp)
                .add_stop(0.45, alpha(p.lamp, 0.0))
                .add_stop(1.0, Color::TRANSPARENT),
        ))),
    )
}

/// Reading-lamp highlight, horizontal: brightest behind the content column.
pub fn lamp_h(p: &Palette) -> container::Style {
    container_style(
        p,
        Some(Background::Gradient(Gradient::Linear(
            Linear::new(Radians(0.0))
                .add_stop(0.0, Color::TRANSPARENT)
                .add_stop(0.5, alpha(p.lamp, 0.05))
                .add_stop(1.0, Color::TRANSPARENT),
        ))),
    )
}

/// Light-theme («Кожа») edge vignette: the uneven, aged tone at the top and bottom.
pub fn vignette(p: &Palette) -> container::Style {
    container_style(
        p,
        Some(Background::Gradient(Gradient::Linear(
            Linear::new(Radians(std::f32::consts::FRAC_PI_2))
                .add_stop(0.0, p.vignette)
                .add_stop(0.15, Color::TRANSPARENT)
                .add_stop(0.85, Color::TRANSPARENT)
                .add_stop(1.0, alpha(p.vignette, 0.12)),
        ))),
    )
}

/// Sidebar: the side veil (plus the optional tint) over the side texture.
/// The 1 px brass right border is a separate `divider_v` element in the row.
pub fn sidebar(p: &Palette) -> container::Style {
    // Composite the tint over the veil ("over" operator).
    let (v, t) = (p.side_veil, p.side_tint);
    let a = t.a + v.a * (1.0 - t.a);
    let bg = if t.a > 0.0 && a > 0.0 {
        Color {
            r: (t.r * t.a + v.r * v.a * (1.0 - t.a)) / a,
            g: (t.g * t.a + v.g * v.a * (1.0 - t.a)) / a,
            b: (t.b * t.a + v.b * v.a * (1.0 - t.a)) / a,
            a,
        }
    } else {
        v
    };
    container_style(p, Some(Background::Color(bg)))
}

/// Vertical 1 px hairline (the sidebar's right brass border).
pub fn divider_v(p: &Palette) -> container::Style {
    container_style(p, Some(Background::Color(p.line)))
}

/// Thin divider between sidebar items (side palette, faint).
pub fn nav_divider(p: &Palette) -> container::Style {
    container_style(p, Some(Background::Color(alpha(p.line2, 0.55))))
}

/// Hover tooltip bubble (main palette: a raised surface card).
pub fn tooltip(p: &Palette) -> container::Style {
    container::Style {
        background: Some(Background::Color(p.surface2)),
        border: Border {
            color: p.line2,
            width: 1.0,
            radius: 10.0.into(),
        },
        shadow: iced::Shadow {
            color: alpha(Color::BLACK, 0.3),
            offset: iced::Vector::new(0.0, 4.0),
            blur_radius: 12.0,
        },
        text_color: Some(p.text),
        snap: true,
    }
}

/// Horizontal 1 px hairline for the settings rows (use with `rule::horizontal(1)`).
pub fn divider_h(p: &Palette) -> rule::Style {
    rule::Style {
        color: p.line2,
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: true,
    }
}

/// Brass "fillet" section break: a horizontal line fading out at the ends.
pub fn fillet(p: &Palette) -> container::Style {
    container_style(
        p,
        Some(Background::Gradient(Gradient::Linear(
            Linear::new(Radians(0.0))
                .add_stop(0.0, Color::TRANSPARENT)
                .add_stop(0.5, alpha(p.gold, 0.55))
                .add_stop(1.0, Color::TRANSPARENT),
        ))),
    )
}

/// Active sidebar item: gold2 text on gold-soft (the 3 px brass left bar is
/// added by the layout in `main.rs`).
pub fn button_nav_active(p: &Palette, status: button::Status) -> button::Style {
    let wash = match status {
        button::Status::Hovered | button::Status::Pressed => 0.22,
        _ => 0.16,
    };
    button::Style {
        background: Some(Background::Color(alpha(p.gold, wash))),
        text_color: p.gold2,
        border: Border {
            color: p.gold,
            width: 0.0,
            radius: radius::BUTTON.into(),
        },
        ..button_style(None, p.gold2)
    }
}

/// Clickable card in the wide-window side panel («Сегодня» → calendar):
/// looks like [`card`], brass border brightens on hover.
pub fn aside_card(p: &Palette, status: button::Status) -> button::Style {
    let c = card(p);
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: c.background,
        text_color: p.text,
        border: Border {
            color: if hovered {
                alpha(p.gold, 0.70)
            } else {
                c.border.color
            },
            ..c.border
        },
        shadow: c.shadow,
        snap: true,
    }
}
