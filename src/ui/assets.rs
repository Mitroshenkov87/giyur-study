//! Embedded raster assets: every WebP in `assets/img` and `assets/icon`,
//! decoded to RGBA once and cached, then served as iced image handles.

use std::collections::HashMap;
use std::sync::OnceLock;

use iced::widget::image::Handle;
use iced::window::Icon;

/// Build-time table of all images in `assets/img` (name → file bytes).
const IMAGES: &[(&str, &[u8])] = &[
    ("app-icon", include_bytes!("../../assets/img/app-icon.webp")),
    (
        "badge-all-lessons",
        include_bytes!("../../assets/img/badge-all-lessons.webp"),
    ),
    (
        "badge-beit-din-mock",
        include_bytes!("../../assets/img/badge-beit-din-mock.webp"),
    ),
    (
        "badge-cards-100",
        include_bytes!("../../assets/img/badge-cards-100.webp"),
    ),
    (
        "badge-first-deck",
        include_bytes!("../../assets/img/badge-first-deck.webp"),
    ),
    (
        "badge-first-lesson",
        include_bytes!("../../assets/img/badge-first-lesson.webp"),
    ),
    (
        "badge-first-test",
        include_bytes!("../../assets/img/badge-first-test.webp"),
    ),
    (
        "badge-goal-first",
        include_bytes!("../../assets/img/badge-goal-first.webp"),
    ),
    (
        "badge-millionaire",
        include_bytes!("../../assets/img/badge-millionaire.webp"),
    ),
    (
        "badge-perfect",
        include_bytes!("../../assets/img/badge-perfect.webp"),
    ),
    (
        "badge-section-beit-din",
        include_bytes!("../../assets/img/badge-section-beit-din.webp"),
    ),
    (
        "badge-section-etika",
        include_bytes!("../../assets/img/badge-section-etika.webp"),
    ),
    (
        "badge-section-ivrit",
        include_bytes!("../../assets/img/badge-section-ivrit.webp"),
    ),
    (
        "badge-section-kashrut",
        include_bytes!("../../assets/img/badge-section-kashrut.webp"),
    ),
    (
        "badge-section-osnovy",
        include_bytes!("../../assets/img/badge-section-osnovy.webp"),
    ),
    (
        "badge-section-prazdniki",
        include_bytes!("../../assets/img/badge-section-prazdniki.webp"),
    ),
    (
        "badge-section-semya",
        include_bytes!("../../assets/img/badge-section-semya.webp"),
    ),
    (
        "badge-section-shabbat",
        include_bytes!("../../assets/img/badge-section-shabbat.webp"),
    ),
    (
        "badge-streak-3",
        include_bytes!("../../assets/img/badge-streak-3.webp"),
    ),
    (
        "badge-streak-30",
        include_bytes!("../../assets/img/badge-streak-30.webp"),
    ),
    (
        "badge-streak-7",
        include_bytes!("../../assets/img/badge-streak-7.webp"),
    ),
    (
        "level-conservative",
        include_bytes!("../../assets/img/level-conservative.webp"),
    ),
    (
        "level-orthodox",
        include_bytes!("../../assets/img/level-orthodox.webp"),
    ),
    (
        "level-reform",
        include_bytes!("../../assets/img/level-reform.webp"),
    ),
    (
        "section-beit-din",
        include_bytes!("../../assets/img/section-beit-din.webp"),
    ),
    (
        "section-etika",
        include_bytes!("../../assets/img/section-etika.webp"),
    ),
    (
        "section-ivrit",
        include_bytes!("../../assets/img/section-ivrit.webp"),
    ),
    (
        "section-kashrut",
        include_bytes!("../../assets/img/section-kashrut.webp"),
    ),
    (
        "section-osnovy",
        include_bytes!("../../assets/img/section-osnovy.webp"),
    ),
    (
        "section-prazdniki",
        include_bytes!("../../assets/img/section-prazdniki.webp"),
    ),
    (
        "section-semya",
        include_bytes!("../../assets/img/section-semya.webp"),
    ),
    (
        "section-shabbat",
        include_bytes!("../../assets/img/section-shabbat.webp"),
    ),
    ("tex-klaf", include_bytes!("../../assets/img/tex-klaf.webp")),
    (
        "tex-leather",
        include_bytes!("../../assets/img/tex-leather.webp"),
    ),
    (
        "tex-walnut",
        include_bytes!("../../assets/img/tex-walnut.webp"),
    ),
    ("welcome", include_bytes!("../../assets/img/welcome.webp")),
];

/// Window icon source.
const ICON_256: &[u8] = include_bytes!("../../assets/icon/shorashim-256.webp");

/// Decode a WebP byte slice to RGBA pixels.
fn decode(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let image = image::load_from_memory(bytes).ok()?;
    let rgba = image.to_rgba8();
    Some((rgba.width(), rgba.height(), rgba.into_raw()))
}

fn handles() -> &'static HashMap<&'static str, Handle> {
    static CACHE: OnceLock<HashMap<&'static str, Handle>> = OnceLock::new();
    CACHE.get_or_init(|| {
        IMAGES
            .iter()
            .filter_map(|(name, bytes)| {
                let (w, h, pixels) = decode(bytes)?;
                Some((*name, Handle::from_rgba(w, h, pixels)))
            })
            .collect()
    })
}

/// An image handle by asset name (e.g. `"section-shabbat"`, `"welcome"`),
/// or `None` if the name is unknown or the file failed to decode.
pub fn img(name: &str) -> Option<Handle> {
    handles().get(name).cloned()
}

/// All embedded image names.
pub fn names() -> impl Iterator<Item = &'static str> {
    IMAGES.iter().map(|(name, _)| *name)
}

/// The application window icon (256 px WebP), decoded once.
pub fn window_icon() -> Option<Icon> {
    static CACHE: OnceLock<Option<Icon>> = OnceLock::new();
    CACHE
        .get_or_init(|| {
            let (w, h, pixels) = decode(ICON_256)?;
            iced::window::icon::from_rgba(pixels, w, h).ok()
        })
        .clone()
}
