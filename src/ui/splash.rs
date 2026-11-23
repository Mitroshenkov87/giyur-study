//! The start-up splash: for five seconds a picture of the menorah from the
//! «Коллекция предметов» with «Шорашим» under it, then a soft fade into the
//! main screen. A click or any key skips it.
//!
//! Light on purpose: one embedded WebP (`assets/img/splash/menorah.webp`,
//! rendered once from `assets/3d/menorah.glb` with `tools/3d/still.py`:
//! `blender -b --python tools/3d/still.py -- assets/3d/menorah.glb
//! out.webp 1100 0.7 0.22`, then cropped square around the menorah with a
//! 6 % margin). No 3D viewer and no GPU work of its own: the page is not
//! built while the splash covers it, and its live 3D viewers show their
//! still pictures until the fade is over.
//!
//! Skipped (never shown) when `SHORASHIM_NO_SPLASH=1` or `--no-splash` is
//! given, and in automation runs (`--control`, the `SHORASHIM_START_ROUTE`
//! screenshot hook), so tests and the night test are not delayed.
//! `SHORASHIM_SPLASH=1` shows it even then (for screenshots), and
//! `SHORASHIM_SPLASH_HOLD=<0..1>` freezes it at that opacity (1 = fully
//! shown, 0.5 = mid-fade) so a slow screenshot can catch it.

use std::time::{Duration, Instant};

use iced::widget::image::Handle;
use iced::widget::text::Shaping;
use iced::widget::{column, container, image, text};
use iced::{Alignment, Background, Color, Element, Length};

use crate::ui::fonts::{HEBREW, SANS, SERIF_BOLD};
use crate::ui::theme;

static PICTURE: &[u8] = include_bytes!("../../assets/img/splash/menorah.webp");

/// How long the splash stays before it starts to fade.
pub const SHOW: Duration = Duration::from_secs(5);
/// The fade into the main screen.
pub const FADE: Duration = Duration::from_millis(600);
/// The quicker fade after a click or a key.
pub const SKIP_FADE: Duration = Duration::from_millis(250);
/// Size of the picture on screen.
const SIZE: f32 = 300.0;

/// The splash picture, decoded once.
fn picture() -> Handle {
    static H: std::sync::OnceLock<Handle> = std::sync::OnceLock::new();
    H.get_or_init(|| match ::image::load_from_memory(PICTURE) {
        Ok(i) => {
            let i = i.to_rgba8();
            Handle::from_rgba(i.width(), i.height(), i.into_raw())
        }
        Err(_) => Handle::from_rgba(1, 1, vec![0; 4]),
    })
    .clone()
}

/// Should the splash be shown on this launch?
pub fn wanted() -> bool {
    let flag = |name: &str| {
        crate::util::env_hook(name).is_some_and(|v| !matches!(v.trim(), "" | "0" | "false"))
    };
    if flag("SPLASH") {
        return true;
    }
    let no_arg = std::env::args().any(|a| a == "--no-splash");
    !(no_arg
        || flag("NO_SPLASH")
        || crate::automation::server::enabled()
        || crate::util::env_hook("START_ROUTE").is_some())
}

/// Screenshot hook `SHORASHIM_SPLASH_HOLD=<opacity>`.
fn hold() -> Option<f32> {
    static H: std::sync::OnceLock<Option<f32>> = std::sync::OnceLock::new();
    *H.get_or_init(|| {
        crate::util::env_hook("SPLASH_HOLD")
            .and_then(|v| v.trim().parse::<f32>().ok())
            .map(|a| a.clamp(0.01, 1.0))
    })
}

/// The splash's clock: when it opened and, after a click or a key, when the
/// quick fade began.
#[derive(Debug, Clone, Copy)]
pub struct Splash {
    start: Instant,
    skipped: Option<Instant>,
    /// The clock was restarted on the first frame, so the five seconds
    /// count from when the window is on screen, not from process start.
    shown: bool,
}

impl Splash {
    pub fn new(now: Instant) -> Self {
        let _ = picture();
        Splash {
            start: now,
            skipped: None,
            shown: false,
        }
    }

    /// A frame tick: the first one starts the five seconds.
    pub fn tick(&mut self, now: Instant) {
        if !self.shown {
            self.shown = true;
            self.start = now;
        }
    }

    /// A click or a key: fade out now (quickly).
    pub fn skip(&mut self, now: Instant) {
        if self.skipped.is_none() && self.opacity(now) >= 1.0 {
            self.skipped = Some(now);
        }
    }

    /// 1 while shown, falling to 0 during the fade.
    pub fn opacity(&self, now: Instant) -> f32 {
        if let Some(a) = hold() {
            return a;
        }
        let (from, len) = match self.skipped {
            Some(t) => (t, SKIP_FADE),
            None => (self.start + SHOW, FADE),
        };
        if now < from {
            return 1.0;
        }
        let t = (now - from).as_secs_f32() / len.as_secs_f32();
        // Ease out: quick at first, soft at the end.
        let t = t.clamp(0.0, 1.0);
        1.0 - t * t * (3.0 - 2.0 * t)
    }

    /// The fade is over: drop the splash.
    pub fn done(&self, now: Instant) -> bool {
        self.opacity(now) <= 0.0
    }

    /// Fading out.
    pub fn fading(&self, now: Instant) -> bool {
        self.opacity(now) < 1.0
    }
}

/// The splash layer, full window, over the main screen.
pub fn view<'a, M: 'a>(splash: &Splash, now: Instant) -> Element<'a, M> {
    let p = theme::current();
    let a = splash.opacity(now);
    let fade = |c: Color| Color { a: c.a * a, ..c };
    let body = column![
        image(picture())
            .width(Length::Fixed(SIZE))
            .height(Length::Fixed(SIZE))
            .opacity(a),
        text("Шорашим")
            .font(SERIF_BOLD)
            .size(44.0)
            .color(fade(p.text)),
        text("שורשים")
            .font(HEBREW)
            .size(22.0)
            .color(fade(p.hebrew))
            .shaping(Shaping::Advanced),
        text("основы иудаизма и подготовка к гиюру")
            .font(SANS)
            .size(15.0)
            .color(fade(p.muted)),
    ]
    .spacing(6)
    .align_x(Alignment::Center);
    let bg = fade(p.bg0);
    container(body)
        .center(Length::Fill)
        .style(move |_| container::Style {
            background: Some(Background::Color(bg)),
            ..container::Style::default()
        })
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_picture_decodes() {
        let i = ::image::load_from_memory(PICTURE).expect("splash picture");
        assert!(i.width() >= 600 && i.height() >= 600);
    }

    #[test]
    fn shows_five_seconds_then_fades_out() {
        let t0 = Instant::now();
        let s = Splash::new(t0);
        assert_eq!(s.opacity(t0), 1.0);
        assert_eq!(s.opacity(t0 + SHOW - Duration::from_millis(1)), 1.0);
        let mid = s.opacity(t0 + SHOW + FADE / 2);
        assert!(mid > 0.0 && mid < 1.0, "{mid}");
        assert!(s.done(t0 + SHOW + FADE));
    }

    #[test]
    fn a_click_or_key_skips_it_quickly() {
        let t0 = Instant::now();
        let mut s = Splash::new(t0);
        let t1 = t0 + Duration::from_secs(1);
        s.skip(t1);
        assert!(s.fading(t1 + Duration::from_millis(10)));
        assert!(s.done(t1 + SKIP_FADE));
        // A second press does not restart the fade.
        s.skip(t1 + Duration::from_millis(100));
        assert!(s.done(t1 + SKIP_FADE));
    }
}
