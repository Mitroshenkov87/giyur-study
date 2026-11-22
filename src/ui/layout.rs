//! The window grid shared by every screen (the calendar's proportions):
//!
//! ```text
//! | margin | nav 230 | content 820 (+2×32 padding) | aside 300 | margin |
//! ```
//!
//! The whole group is centred in the window, so on a maximized window the
//! navigation moves in from the edge together with the content instead of
//! staying glued to the left. When the window is wide enough the extra room
//! on the right holds the «Цитата дня» panel, which keeps the reading column
//! slightly left of centre and the right side filled. On narrow windows the
//! margins collapse first, then the aside goes, then the content shrinks.

/// Navigation column width.
pub const NAV: f32 = 230.0;
/// Reading width of the content column (cards, text).
pub const CONTENT: f32 = 820.0;
/// Horizontal padding on each side of the content column.
pub const CONTENT_PAD: f32 = 32.0;
/// Quote / «Сегодня» side panel width.
pub const ASIDE: f32 = 300.0;
/// Empty space kept on each side before the aside is shown.
pub const MIN_MARGIN: f32 = 40.0;

/// Column positions for one window width (logical pixels).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grid {
    /// Empty space left of the navigation column.
    pub left: f32,
    /// Content column width including its padding.
    pub content: f32,
    /// Whether the aside panel is shown.
    pub aside: bool,
    /// Empty space right of the content (or of the aside).
    pub right: f32,
}

impl Grid {
    /// The x of the aside's left edge (only meaningful when `aside`).
    pub fn aside_x(&self) -> f32 {
        self.left + NAV + 1.0 + self.content
    }
}

/// Full content column width (reading width plus padding).
pub const fn content_col() -> f32 {
    CONTENT + 2.0 * CONTENT_PAD
}

/// The group width with or without the aside (nav + 1 px divider + content).
pub const fn group(aside: bool) -> f32 {
    NAV + 1.0 + content_col() + if aside { ASIDE } else { 0.0 }
}

/// Lay the grid out for a window `width`; `want_aside` is the user setting.
pub fn compute(width: f32, want_aside: bool) -> Grid {
    let aside = want_aside && width >= group(true) + 2.0 * MIN_MARGIN;
    let g = group(aside);
    if width >= g {
        let m = ((width - g) / 2.0).floor();
        Grid {
            left: m,
            content: content_col(),
            aside,
            right: width - g - m,
        }
    } else {
        Grid {
            left: 0.0,
            content: (width - NAV - 1.0).max(0.0),
            aside: false,
            right: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn total(g: &Grid) -> f32 {
        g.left + NAV + 1.0 + g.content + if g.aside { ASIDE } else { 0.0 } + g.right
    }

    #[test]
    fn maximized_full_hd_is_centred_with_aside() {
        let g = compute(1920.0, true);
        assert!(g.aside);
        assert_eq!(g.content, content_col());
        assert!((g.left - g.right).abs() <= 1.0, "{g:?}");
        assert_eq!(total(&g), 1920.0);
        // Reading column sits left of the window centre.
        let col_mid = g.left + NAV + 1.0 + g.content / 2.0;
        assert!(col_mid < 960.0);
    }

    #[test]
    fn aside_can_be_turned_off() {
        let g = compute(1920.0, false);
        assert!(!g.aside);
        assert!((g.left - g.right).abs() <= 1.0);
        assert_eq!(total(&g), 1920.0);
    }

    #[test]
    fn medium_window_drops_aside_and_centres() {
        let g = compute(1280.0, true);
        assert!(!g.aside);
        assert!(g.left > 0.0);
        assert_eq!(total(&g), 1280.0);
    }

    #[test]
    fn narrow_window_shrinks_content_and_pins_nav() {
        let g = compute(900.0, true);
        assert_eq!(g.left, 0.0);
        assert!(!g.aside);
        assert_eq!(total(&g), 900.0);
        assert!(g.content < content_col());
    }

    #[test]
    fn grid_is_continuous_around_the_aside_threshold() {
        let t = group(true) + 2.0 * MIN_MARGIN;
        assert!(!compute(t - 1.0, true).aside);
        assert!(compute(t, true).aside);
        for w in (720..2600).step_by(7) {
            let g = compute(w as f32, true);
            assert_eq!(total(&g), w as f32, "{w}: {g:?}");
            assert!(g.left >= 0.0 && g.right >= 0.0);
        }
    }
}
