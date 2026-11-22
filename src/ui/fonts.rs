//! Embedded fonts and the [`iced::Font`] constants used across the app.
//!
//! Family names were verified with `fc-scan`:
//! - `NotoSans-*.ttf`        → "Noto Sans"
//! - `NotoSerif-Bold.ttf`    → "Noto Serif"
//! - `NotoSerifHebrew-*.ttf` → "Noto Serif Hebrew"
//! - `NotoSansHebrew-*.ttf`  → "Noto Sans Hebrew"

use iced::font::{Style, Weight};
use iced::Font;

/// Body / UI text.
pub const SANS: Font = Font {
    family: iced::font::Family::Name("Noto Sans"),
    weight: Weight::Normal,
    stretch: iced::font::Stretch::Normal,
    style: Style::Normal,
};

/// Bold UI text (labels, emphasis).
pub const SANS_BOLD: Font = Font {
    family: iced::font::Family::Name("Noto Sans"),
    weight: Weight::Bold,
    stretch: iced::font::Stretch::Normal,
    style: Style::Normal,
};

/// Headings (h1–h4), card titles, quotes, KPI numbers.
pub const SERIF_BOLD: Font = Font {
    family: iced::font::Family::Name("Noto Serif"),
    weight: Weight::Bold,
    stretch: iced::font::Stretch::Normal,
    style: Style::Normal,
};

/// Hebrew text (RTL, pointed Hebrew). Serif Hebrew per the PWA Hebrew stack.
pub const HEBREW: Font = Font {
    family: iced::font::Family::Name("Noto Serif Hebrew"),
    weight: Weight::Normal,
    stretch: iced::font::Stretch::Normal,
    style: Style::Normal,
};

const NOTO_SANS_REGULAR: &[u8] = include_bytes!("../../assets/fonts/NotoSans-Regular.ttf");
const NOTO_SANS_BOLD: &[u8] = include_bytes!("../../assets/fonts/NotoSans-Bold.ttf");
const NOTO_SERIF_BOLD: &[u8] = include_bytes!("../../assets/fonts/NotoSerif-Bold.ttf");
const NOTO_SERIF_HEBREW_REGULAR: &[u8] =
    include_bytes!("../../assets/fonts/NotoSerifHebrew-Regular.ttf");
const NOTO_SANS_HEBREW_REGULAR: &[u8] =
    include_bytes!("../../assets/fonts/NotoSansHebrew-Regular.ttf");

/// Symbol fallback (arrows →, ≥ ≤ ≈ ⊂, ✓ ❄ ⚠ ★ ✡) missing from the Noto
/// subsets; cosmic-text falls back to it automatically. DejaVu fonts licence.
const DEJAVU_SANS: &[u8] = include_bytes!("../../assets/fonts/DejaVuSans.ttf");

/// All embedded font files, for loading at startup via `iced::font::load`.
pub fn all() -> Vec<&'static [u8]> {
    vec![
        NOTO_SANS_REGULAR,
        NOTO_SANS_BOLD,
        NOTO_SERIF_BOLD,
        NOTO_SERIF_HEBREW_REGULAR,
        NOTO_SANS_HEBREW_REGULAR,
        DEJAVU_SANS,
    ]
}
