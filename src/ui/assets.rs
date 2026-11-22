//! Embedded raster assets: every WebP in `assets/img` and `assets/icon`,
//! decoded to RGBA once and cached, then served as iced image handles.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

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
    ("bg-dark", include_bytes!("../../assets/img/bg-dark.webp")),
    (
        "bg-leather",
        include_bytes!("../../assets/img/bg-leather.webp"),
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
    (
        "side-dark",
        include_bytes!("../../assets/img/side-dark.webp"),
    ),
    (
        "side-leather",
        include_bytes!("../../assets/img/side-leather.webp"),
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

/// Procedural 3D art (assets/3d, rendered to WebP): sidebar icons
/// `icon-<item>-<wood|leather>[-hover]` (128 px) and section medallions
/// `emblem-<section>-<wood|leather>` (256 px). Decoded lazily on first use and
/// cached (a theme switch decodes the other set too, about 2 MB in all), and
/// downscaled once with Lanczos to about twice the display size, because the
/// GPU image pipeline has no mipmaps and big downscales would alias.
const ART: &[(&str, &[u8])] = &[
    (
        "icon-lessons-wood",
        include_bytes!("../../assets/img/icons/lessons-wood-128.webp"),
    ),
    (
        "icon-lessons-wood-hover",
        include_bytes!("../../assets/img/icons/lessons-wood-hover-128.webp"),
    ),
    (
        "icon-lessons-leather",
        include_bytes!("../../assets/img/icons/lessons-leather-128.webp"),
    ),
    (
        "icon-lessons-leather-hover",
        include_bytes!("../../assets/img/icons/lessons-leather-hover-128.webp"),
    ),
    (
        "icon-calendar-wood",
        include_bytes!("../../assets/img/icons/calendar-wood-128.webp"),
    ),
    (
        "icon-calendar-wood-hover",
        include_bytes!("../../assets/img/icons/calendar-wood-hover-128.webp"),
    ),
    (
        "icon-calendar-leather",
        include_bytes!("../../assets/img/icons/calendar-leather-128.webp"),
    ),
    (
        "icon-calendar-leather-hover",
        include_bytes!("../../assets/img/icons/calendar-leather-hover-128.webp"),
    ),
    (
        "icon-cards-wood",
        include_bytes!("../../assets/img/icons/cards-wood-128.webp"),
    ),
    (
        "icon-cards-wood-hover",
        include_bytes!("../../assets/img/icons/cards-wood-hover-128.webp"),
    ),
    (
        "icon-cards-leather",
        include_bytes!("../../assets/img/icons/cards-leather-128.webp"),
    ),
    (
        "icon-cards-leather-hover",
        include_bytes!("../../assets/img/icons/cards-leather-hover-128.webp"),
    ),
    (
        "icon-quiz-wood",
        include_bytes!("../../assets/img/icons/quiz-wood-128.webp"),
    ),
    (
        "icon-quiz-wood-hover",
        include_bytes!("../../assets/img/icons/quiz-wood-hover-128.webp"),
    ),
    (
        "icon-quiz-leather",
        include_bytes!("../../assets/img/icons/quiz-leather-128.webp"),
    ),
    (
        "icon-quiz-leather-hover",
        include_bytes!("../../assets/img/icons/quiz-leather-hover-128.webp"),
    ),
    (
        "icon-tests-wood",
        include_bytes!("../../assets/img/icons/tests-wood-128.webp"),
    ),
    (
        "icon-tests-wood-hover",
        include_bytes!("../../assets/img/icons/tests-wood-hover-128.webp"),
    ),
    (
        "icon-tests-leather",
        include_bytes!("../../assets/img/icons/tests-leather-128.webp"),
    ),
    (
        "icon-tests-leather-hover",
        include_bytes!("../../assets/img/icons/tests-leather-hover-128.webp"),
    ),
    (
        "icon-progress-wood",
        include_bytes!("../../assets/img/icons/progress-wood-128.webp"),
    ),
    (
        "icon-progress-wood-hover",
        include_bytes!("../../assets/img/icons/progress-wood-hover-128.webp"),
    ),
    (
        "icon-progress-leather",
        include_bytes!("../../assets/img/icons/progress-leather-128.webp"),
    ),
    (
        "icon-progress-leather-hover",
        include_bytes!("../../assets/img/icons/progress-leather-hover-128.webp"),
    ),
    (
        "icon-settings-wood",
        include_bytes!("../../assets/img/icons/settings-wood-128.webp"),
    ),
    (
        "icon-settings-wood-hover",
        include_bytes!("../../assets/img/icons/settings-wood-hover-128.webp"),
    ),
    (
        "icon-settings-leather",
        include_bytes!("../../assets/img/icons/settings-leather-128.webp"),
    ),
    (
        "icon-settings-leather-hover",
        include_bytes!("../../assets/img/icons/settings-leather-hover-128.webp"),
    ),
    (
        "icon-sources-wood",
        include_bytes!("../../assets/img/icons/sources-wood-128.webp"),
    ),
    (
        "icon-sources-wood-hover",
        include_bytes!("../../assets/img/icons/sources-wood-hover-128.webp"),
    ),
    (
        "icon-sources-leather",
        include_bytes!("../../assets/img/icons/sources-leather-128.webp"),
    ),
    (
        "icon-sources-leather-hover",
        include_bytes!("../../assets/img/icons/sources-leather-hover-128.webp"),
    ),
    (
        "emblem-osnovy-wood",
        include_bytes!("../../assets/img/emblems/osnovy-wood-256.webp"),
    ),
    (
        "emblem-osnovy-leather",
        include_bytes!("../../assets/img/emblems/osnovy-leather-256.webp"),
    ),
    (
        "emblem-ivrit-wood",
        include_bytes!("../../assets/img/emblems/ivrit-wood-256.webp"),
    ),
    (
        "emblem-ivrit-leather",
        include_bytes!("../../assets/img/emblems/ivrit-leather-256.webp"),
    ),
    (
        "emblem-shabbat-wood",
        include_bytes!("../../assets/img/emblems/shabbat-wood-256.webp"),
    ),
    (
        "emblem-shabbat-leather",
        include_bytes!("../../assets/img/emblems/shabbat-leather-256.webp"),
    ),
    (
        "emblem-kashrut-wood",
        include_bytes!("../../assets/img/emblems/kashrut-wood-256.webp"),
    ),
    (
        "emblem-kashrut-leather",
        include_bytes!("../../assets/img/emblems/kashrut-leather-256.webp"),
    ),
    (
        "emblem-prazdniki-wood",
        include_bytes!("../../assets/img/emblems/prazdniki-wood-256.webp"),
    ),
    (
        "emblem-prazdniki-leather",
        include_bytes!("../../assets/img/emblems/prazdniki-leather-256.webp"),
    ),
    (
        "emblem-semya-wood",
        include_bytes!("../../assets/img/emblems/semya-wood-256.webp"),
    ),
    (
        "emblem-semya-leather",
        include_bytes!("../../assets/img/emblems/semya-leather-256.webp"),
    ),
    (
        "emblem-etika-wood",
        include_bytes!("../../assets/img/emblems/etika-wood-256.webp"),
    ),
    (
        "emblem-etika-leather",
        include_bytes!("../../assets/img/emblems/etika-leather-256.webp"),
    ),
    (
        "emblem-beit-din-wood",
        include_bytes!("../../assets/img/emblems/beit_din-wood-256.webp"),
    ),
    (
        "emblem-beit-din-leather",
        include_bytes!("../../assets/img/emblems/beit_din-leather-256.webp"),
    ),
];

fn art_cache() -> &'static Mutex<HashMap<&'static str, Option<Handle>>> {
    static CACHE: OnceLock<Mutex<HashMap<&'static str, Option<Handle>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// A rendered art handle by name (see [`ART`]), decoded once on first use.
pub fn art(name: &str) -> Option<Handle> {
    let (key, bytes) = ART.iter().find(|(k, _)| *k == name)?;
    let mut cache = art_cache().lock().ok()?;
    cache
        .entry(key)
        .or_insert_with(|| {
            let image = image::load_from_memory(bytes).ok()?;
            let target = if key.starts_with("icon-") { 80 } else { 192 };
            let rgba = if image.width() > target {
                image::imageops::resize(
                    &image.to_rgba8(),
                    target,
                    target,
                    image::imageops::FilterType::Lanczos3,
                )
            } else {
                image.to_rgba8()
            };
            Some(Handle::from_rgba(
                rgba.width(),
                rgba.height(),
                rgba.into_raw(),
            ))
        })
        .clone()
}

/// All embedded art names.
pub fn art_names() -> impl Iterator<Item = &'static str> {
    ART.iter().map(|(name, _)| *name)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_art_decodes_with_alpha() {
        for (name, bytes) in ART {
            let (w, h, px) = decode(bytes).unwrap_or_else(|| panic!("{name} does not decode"));
            assert_eq!(w, h, "{name} is square");
            assert!(px.chunks(4).any(|c| c[3] == 0), "{name} has transparency");
        }
        assert!(art("icon-lessons-wood").is_some());
        assert!(art("emblem-beit-din-leather").is_some());
        assert!(art("nope").is_none());
    }
}
