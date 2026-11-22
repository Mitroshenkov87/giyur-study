//! Sources page (PARITY §13): one card per section listing the books and
//! sites its materials rely on.

use iced::widget::{button, container, row, Column};
use iced::{Alignment, Element, Length};

use crate::content::{self, Source};
use crate::storage::Level;
use crate::ui::app::{Action, Ctx, Route};
use crate::ui::screens::map::{heading_owned, icons, txt};
use crate::ui::{theme, widgets};

pub struct Screen;

#[derive(Debug, Clone)]
pub enum Msg {
    Back,
    OpenUrl(String),
}

impl Screen {
    pub fn new(_ctx: &mut Ctx, _route: &Route) -> Self {
        Screen
    }

    pub fn update(&mut self, _ctx: &mut Ctx, msg: Msg) -> Action {
        match msg {
            Msg::Back => Action::Go(Route::Settings),
            Msg::OpenUrl(url) => {
                widgets::open_url(&url);
                Action::None
            }
        }
    }

    pub fn subscription(&self) -> iced::Subscription<Msg> {
        iced::Subscription::none()
    }

    pub fn view<'a>(&'a self, ctx: &'a Ctx) -> Element<'a, Msg> {
        let p = theme::current();

        let mut page = Column::new().spacing(14).width(Length::Fill);
        page = page.push(widgets::back_button("Настройки", Msg::Back));
        page = page.push(widgets::heading("Источники", 1));
        page = page.push(
            txt("Книги и сайты, на которые опираются материалы. Ссылки открываются во внешнем браузере (нужен интернет).")
                .size(theme::text::BODY)
                .color(p.muted)
                .width(Length::Fill),
        );

        let mut any = false;
        for sec in &ctx.lib.sections {
            let sources = ctx.lib.sources(sec, None);
            if sources.is_empty() {
                continue;
            }
            any = true;
            let mut list = Column::new().spacing(10).width(Length::Fill);
            list = list.push(heading_owned(
                format!("{}. {}", sec.index + 1, sec.title),
                3,
            ));
            for src in sources {
                list = list.push(source_row(src, p));
            }
            page = page.push(widgets::card(list));
        }

        if !any {
            page = page.push(widgets::card(
                txt("Список источников появится вместе с материалами разделов.")
                    .size(theme::text::BODY)
                    .color(p.muted),
            ));
        }

        page.into()
    }
}

fn movement_tag(movement: &str) -> Option<&'static str> {
    let level = match movement {
        "reform" => Level::Reform,
        "conservative" => Level::Conservative,
        "orthodox" => Level::Orthodox,
        _ => return None,
    };
    Some(content::level_info(level).short)
}

fn source_row<'a>(src: &'a Source, p: theme::Palette) -> Element<'a, Msg> {
    let is_link = src
        .url
        .as_deref()
        .map(|u| u.starts_with("http://") || u.starts_with("https://"))
        .unwrap_or(false);
    let mut title = row![txt(src.title.as_deref().unwrap_or(&src.id))
        .size(theme::text::SMALL + 1.0)
        .color(p.text)]
    .spacing(6)
    .align_y(Alignment::Center);
    if is_link {
        title = title.push(icons::icon(icons::Kind::External, 14.0, p.gold2));
    }
    let mut line = row![container(title).width(Length::Fill)]
        .spacing(10)
        .align_y(Alignment::Center)
        .height(Length::Fixed(32.0));
    if let Some(short) = src.movement.as_deref().and_then(movement_tag) {
        line = line.push(widgets::tag(short));
    }
    // http(s) sources open in the system browser (§13).
    match src.url.as_deref().filter(|_| is_link) {
        Some(url) => button(line.width(Length::Fill))
            .width(Length::Fill)
            .padding([4.0, 6.0])
            .style(move |_, s| theme::button_ghost(&theme::current(), s))
            .on_press(Msg::OpenUrl(url.to_string()))
            .into(),
        None => line.into(),
    }
}
