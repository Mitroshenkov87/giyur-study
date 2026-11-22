//! Headless widget probe: lays out a screen's real `view()` with the
//! tiny-skia renderer (no window), lists its texts and input fields, finds
//! which texts are clickable by simulating a mouse click on each one, and
//! renders frames for screenshots.
//!
//! A click on a text produces exactly the `Message`s the real widget would
//! publish for a user click, so "pressing" a control later just feeds those
//! messages into the app's normal `update`.

use std::borrow::Cow;
use std::cell::RefCell;
use std::fmt::Debug;
use std::sync::Once;

use iced::advanced::clipboard;
use iced::advanced::renderer::Headless;
use iced::advanced::widget::operation::{Focusable, Outcome, Scrollable, TextInput};
use iced::advanced::widget::{Id, Operation};
use iced::keyboard::{self, key};
use iced::{mouse, Color, Element, Event, Font, Pixels, Point, Rectangle, Size, Theme, Vector};
use iced_runtime::user_interface::{Cache, UserInterface};

/// A piece of visible text and where it is (logical pixels).
#[derive(Debug, Clone)]
pub struct TextItem {
    pub text: String,
    pub bounds: Rectangle,
}

/// An editable field (text input or text editor).
#[derive(Debug, Clone)]
pub struct InputItem {
    pub bounds: Rectangle,
    /// Visible text of a text input (its value, or the placeholder).
    pub text: Option<String>,
    pub kind: &'static str,
}

/// A drop-down (pick list) reported by `automation::reported_choice`.
#[derive(Debug, Clone)]
pub struct ChoiceItem {
    pub bounds: Rectangle,
    pub shown: String,
    pub options: Vec<String>,
}

impl ChoiceItem {
    /// Where option `i` sits once the menu is open: iced's menu opens right
    /// below the field, one row per option (text line + pick-list padding).
    pub fn option_point(&self, i: usize) -> Point {
        const ROW: f32 = 16.0 * 1.3 + 10.0;
        Point::new(
            self.bounds.center_x(),
            self.bounds.y + self.bounds.height + ROW * i as f32 + ROW / 2.0,
        )
    }
}

/// Result of a scan.
pub struct Scan<M> {
    pub texts: Vec<TextItem>,
    pub inputs: Vec<InputItem>,
    pub choices: Vec<ChoiceItem>,
    /// For every text index: the messages a click on it publishes.
    pub clicks: Vec<Vec<M>>,
    /// Height of the first scrollable's content (the page), if any.
    pub content_height: Option<f32>,
}

/// One step of a scripted interaction on a single live widget tree.
#[derive(Debug, Clone)]
pub enum Step {
    Click(Point),
    SelectAll,
    Type(String),
    Enter,
}

thread_local! {
    static RENDERER: RefCell<Option<iced::Renderer>> = const { RefCell::new(None) };
}

static FONTS: Once = Once::new();

/// Register the app's embedded fonts with the global font system (needed
/// when no window ever loaded them, e.g. headless runs and tests).
pub fn ensure_fonts() {
    FONTS.call_once(|| {
        let mut fs = iced::advanced::graphics::text::font_system()
            .write()
            .expect("font system lock");
        for f in crate::ui::fonts::all() {
            fs.load_font(Cow::Borrowed(f));
        }
    });
}

fn with_renderer<R>(f: impl FnOnce(&mut iced::Renderer) -> R) -> R {
    ensure_fonts();
    RENDERER.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.is_none() {
            let r = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
                Font::with_name("Noto Sans"),
                Pixels(16.0),
                Some("tiny-skia"),
            ))
            .expect("headless tiny-skia renderer");
            *slot = Some(r);
        }
        f(slot.as_mut().expect("renderer"))
    })
}

#[derive(Default)]
struct Collector {
    texts: Vec<TextItem>,
    inputs: Vec<InputItem>,
    choices: Vec<ChoiceItem>,
    content_height: Option<f32>,
}

impl Operation for Collector {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<()>)) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        _id: Option<&Id>,
        _bounds: Rectangle,
        content_bounds: Rectangle,
        _translation: Vector,
        _state: &mut dyn Scrollable,
    ) {
        if self.content_height.is_none() {
            self.content_height = Some(content_bounds.height);
        }
    }

    fn focusable(&mut self, _id: Option<&Id>, bounds: Rectangle, _state: &mut dyn Focusable) {
        // Text inputs report both focusable and text_input; keep one entry.
        if !self.inputs.iter().any(|i| i.bounds == bounds) {
            self.inputs.push(InputItem {
                bounds,
                text: None,
                kind: "editor",
            });
        }
    }

    fn text_input(&mut self, _id: Option<&Id>, bounds: Rectangle, state: &mut dyn TextInput) {
        let item = InputItem {
            bounds,
            text: Some(state.text().to_string()),
            kind: "text_input",
        };
        match self.inputs.iter_mut().find(|i| i.bounds == bounds) {
            Some(existing) => *existing = item,
            None => self.inputs.push(item),
        }
    }

    fn text(&mut self, _id: Option<&Id>, bounds: Rectangle, text: &str) {
        let t = text.trim();
        if !t.is_empty() {
            self.texts.push(TextItem {
                text: t.to_string(),
                bounds,
            });
        }
    }

    fn custom(&mut self, _id: Option<&Id>, bounds: Rectangle, state: &mut dyn std::any::Any) {
        if let Some(c) = state.downcast_ref::<super::reported::ChoiceInfo>() {
            self.choices.push(ChoiceItem {
                bounds,
                shown: c.shown.clone(),
                options: c.options.clone(),
            });
        }
    }

    fn finish(&self) -> Outcome<()> {
        Outcome::None
    }
}

fn collect<M>(
    ui: &mut UserInterface<'_, M, Theme, iced::Renderer>,
    r: &iced::Renderer,
) -> Collector {
    let mut c = Collector::default();
    ui.operate(r, &mut c);
    c
}

fn click_events() -> Vec<Event> {
    vec![
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
    ]
}

fn key_press(k: keyboard::Key, text: Option<&str>, modifiers: keyboard::Modifiers) -> Event {
    Event::Keyboard(keyboard::Event::KeyPressed {
        key: k.clone(),
        modified_key: k,
        physical_key: key::Physical::Unidentified(key::NativeCode::Unidentified),
        location: keyboard::Location::Standard,
        modifiers,
        repeat: false,
        text: text.map(Into::into),
    })
}

/// Lay out `element` at `size`, list texts and inputs, and find the messages
/// a click on each text publishes.
pub fn scan<'a, M: Clone>(view: &dyn Fn() -> Element<'a, M>, size: Size) -> Scan<M> {
    with_renderer(|renderer| {
        let mut ui = UserInterface::build(view(), size, Cache::default(), renderer);
        let c = collect(&mut ui, renderer);
        let mut clicks = Vec::with_capacity(c.texts.len());
        for t in &c.texts {
            let mut out = Vec::new();
            let center = t.bounds.center();
            if center.x >= 0.0 && center.y >= 0.0 && center.x < size.width && center.y < size.height
            {
                let (_, statuses) = ui.update(
                    &click_events(),
                    mouse::Cursor::Available(center),
                    renderer,
                    &mut clipboard::Null,
                    &mut out,
                );
                // A captured click without a message (pick list opened, field
                // focused) may leave state behind: start from a fresh tree.
                if out.is_empty() && statuses.contains(&iced::event::Status::Captured) {
                    drop(ui);
                    ui = UserInterface::build(view(), size, Cache::default(), renderer);
                }
            }
            clicks.push(out);
        }
        Scan {
            texts: c.texts,
            inputs: c.inputs,
            choices: c.choices,
            clicks,
            content_height: c.content_height,
        }
    })
}

/// Run `steps` on one live widget tree and return every published message,
/// in order. Clicks keep the cursor where they happened; typing goes to the
/// focused field (focus is kept inside this one tree).
pub fn interact<'a, M: Clone>(element: Element<'a, M>, size: Size, steps: &[Step]) -> Vec<M> {
    with_renderer(|renderer| {
        let mut ui = UserInterface::build(element, size, Cache::default(), renderer);
        let mut out = Vec::new();
        let mut cursor = mouse::Cursor::Unavailable;
        for step in steps {
            let events: Vec<Event> = match step {
                Step::Click(p) => {
                    cursor = mouse::Cursor::Available(*p);
                    let mut e = vec![Event::Mouse(mouse::Event::CursorMoved { position: *p })];
                    e.extend(click_events());
                    e
                }
                Step::SelectAll => vec![
                    Event::Keyboard(keyboard::Event::ModifiersChanged(
                        keyboard::Modifiers::COMMAND,
                    )),
                    key_press(
                        keyboard::Key::Character("a".into()),
                        Some("a"),
                        keyboard::Modifiers::COMMAND,
                    ),
                    Event::Keyboard(keyboard::Event::ModifiersChanged(
                        keyboard::Modifiers::empty(),
                    )),
                ],
                Step::Type(s) => s
                    .chars()
                    .map(|ch| {
                        let t = ch.to_string();
                        key_press(
                            keyboard::Key::Character(t.as_str().into()),
                            Some(&t),
                            keyboard::Modifiers::empty(),
                        )
                    })
                    .collect(),
                Step::Enter => vec![key_press(
                    keyboard::Key::Named(key::Named::Enter),
                    None,
                    keyboard::Modifiers::empty(),
                )],
            };
            // One event per update, like a real event loop delivering input.
            for e in events {
                let _ = ui.update(&[e], cursor, renderer, &mut clipboard::Null, &mut out);
            }
        }
        out
    })
}

/// Render `element` at `size` (logical px) × `scale` into RGBA8.
pub fn render<'a, M>(
    element: Element<'a, M>,
    size: Size,
    scale: f32,
    theme: &Theme,
    background: Color,
) -> (Vec<u8>, u32, u32) {
    with_renderer(|renderer| {
        let mut ui = UserInterface::build(element, size, Cache::default(), renderer);
        let mut sink = Vec::new();
        let _ = ui.update(
            &[Event::Window(iced::window::Event::RedrawRequested(
                iced::time::Instant::now(),
            ))],
            mouse::Cursor::Unavailable,
            renderer,
            &mut clipboard::Null,
            &mut sink,
        );
        let base = iced::theme::Base::base(theme);
        ui.draw(
            renderer,
            theme,
            &iced::advanced::renderer::Style {
                text_color: base.text_color,
            },
            mouse::Cursor::Unavailable,
        );
        let w = (size.width * scale).round() as u32;
        let h = (size.height * scale).round() as u32;
        let rgba = renderer.screenshot(Size::new(w, h), scale, background);
        (rgba, w, h)
    })
}

/// Encode RGBA8 as lossless WebP.
pub fn write_webp(path: &std::path::Path, rgba: &[u8], w: u32, h: u32) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        if !dir.as_os_str().is_empty() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
    }
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let enc = image::codecs::webp::WebPEncoder::new_lossless(std::io::BufWriter::new(file));
    enc.encode(rgba, w, h, image::ExtendedColorType::Rgba8)
        .map_err(|e| e.to_string())
}

/// Debug text of a value, for stable control ids.
pub fn debug_of<M: Debug>(m: &M) -> String {
    format!("{m:?}")
}
