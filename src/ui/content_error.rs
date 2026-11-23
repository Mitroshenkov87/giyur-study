//! The window shown instead of the app when the course files beside the
//! exe are missing or broken (src/content_store.rs): it names the file and
//! the place in it, calmly, and never crashes.

use iced::widget::text::Shaping;
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Alignment, Element, Font, Length, Task, Theme};

use crate::content_store::LoadError;
use crate::ui::fonts;
use crate::ui::theme::{self, ThemeKind};

struct Window {
    err: LoadError,
}

#[derive(Debug, Clone)]
enum Msg {
    OpenFolder,
    Close,
}

/// The folder worth opening: the broken file's folder, or the nearest one
/// that exists.
fn folder(err: &LoadError) -> Option<std::path::PathBuf> {
    let mut p = err.file.parent().map(|p| p.to_path_buf());
    while let Some(d) = p {
        if d.is_dir() {
            return Some(d);
        }
        p = d.parent().map(|p| p.to_path_buf());
    }
    None
}

/// The text of the window (also printed to stderr), in plain Russian.
pub fn message(err: &LoadError) -> (String, String) {
    let missing_folder = !err.searched.is_empty();
    let title = if missing_folder {
        "Не найдены материалы курса".to_string()
    } else {
        "Не удалось открыть материалы курса".to_string()
    };
    let mut body = String::new();
    if missing_folder {
        body.push_str(
            "Уроки, карточки и вопросы лежат в папке content рядом с программой, \
             но её не удалось найти.\n\n",
        );
        for p in &err.searched {
            body.push_str(&format!("Искали здесь: {}\n", p.display()));
        }
        body.push_str(
            "\nПереустановите программу из Shorashim-Setup.exe — он положит папку на место.",
        );
    } else {
        body.push_str(&format!(
            "Файл: {}\n\nЧто не так: {}\n\n",
            err.file.display(),
            err.reason
        ));
        body.push_str(
            "Исправьте этот файл и запустите программу снова, \
             или переустановите её из Shorashim-Setup.exe. \
             Ваш прогресс хранится отдельно и не пострадает.",
        );
    }
    (title, body)
}

impl Window {
    fn update(&mut self, msg: Msg) -> Task<Msg> {
        match msg {
            Msg::OpenFolder => {
                if let Some(d) = folder(&self.err) {
                    crate::ui::widgets::open_url(&d.to_string_lossy());
                }
                Task::none()
            }
            Msg::Close => iced::exit(),
        }
    }

    fn view(&self) -> Element<'_, Msg> {
        let p = theme::current();
        let (title, body) = message(&self.err);
        let mut buttons = row![].spacing(12);
        if folder(&self.err).is_some() {
            buttons = buttons.push(
                button(text("Открыть папку").shaping(Shaping::Advanced))
                    .on_press(Msg::OpenFolder)
                    .style(button::secondary),
            );
        }
        buttons = buttons.push(
            button(text("Закрыть").shaping(Shaping::Advanced))
                .on_press(Msg::Close)
                .style(button::primary),
        );
        container(
            column![
                text(title)
                    .size(22)
                    .color(p.gold)
                    .shaping(Shaping::Advanced),
                scrollable(text(body).size(15).color(p.text).shaping(Shaping::Advanced))
                    .height(Length::Fill),
                buttons,
            ]
            .spacing(18)
            .align_x(Alignment::Start),
        )
        .padding(28)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

/// Run the error window until it is closed.
pub fn run(err: LoadError) -> iced::Result {
    let (title, body) = message(&err);
    eprintln!("{title}\n{body}");
    theme::set_current(ThemeKind::Dark);
    let settings = iced::window::Settings {
        size: iced::Size::new(640.0, 400.0),
        min_size: Some(iced::Size::new(480.0, 320.0)),
        icon: crate::ui::assets::window_icon(),
        ..Default::default()
    };
    let app = iced::application(
        move || Window { err: err.clone() },
        Window::update,
        Window::view,
    )
    .title(|_: &Window| "Шорашим — материалы курса".to_string())
    .theme(|_: &Window| -> Theme { theme::iced_theme(ThemeKind::Dark) })
    .window(settings)
    .default_font(Font::with_name("Noto Sans"));
    fonts::all()
        .into_iter()
        .fold(app, |app, font| app.font(font))
        .run()
}
