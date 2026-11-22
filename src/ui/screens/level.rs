//! Movement / level choice (PARITY §2): three level cards, confirmation
//! dialog when changing an already chosen movement. Progress is never reset.

use iced::widget::{button, column, container, opaque, row, space, stack, Column, Row};
use iced::{Alignment, Background, Border, Color, Element, Length};

use crate::content;
use crate::storage::Level;
use crate::ui::app::{Action, Ctx, Route};
use crate::ui::screens::map::{heading_owned, icons, kicker_owned, txt};
use crate::ui::{assets, fonts, theme, widgets};

pub struct Screen {
    /// Came from the map chip / settings («Назад» returns without changes).
    reopened: bool,
    /// Pending movement change awaiting confirmation.
    confirm: Option<Level>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Back,
    Pick(Level),
    ConfirmYes,
    ConfirmNo,
}

impl Screen {
    pub fn new(ctx: &mut Ctx, route: &Route) -> Self {
        let reopened = matches!(route, Route::Level { reopened: true });
        let _ = ctx;
        Screen {
            reopened,
            confirm: None,
        }
    }

    fn apply(&mut self, ctx: &mut Ctx, level: Level) -> Action {
        let first = !ctx.state.onboarded;
        ctx.state.settings.level = Some(level);
        ctx.state.onboarded = true;
        ctx.save();
        let info = content::level_info(level);
        ctx.toast(&format!("Выбрано: {}", info.title), "", "★");
        self.confirm = None;
        if first {
            Action::Go(Route::Map)
        } else {
            Action::Back
        }
    }

    pub fn update(&mut self, ctx: &mut Ctx, msg: Msg) -> Action {
        match msg {
            Msg::Back => {
                if ctx.state.onboarded {
                    Action::Back
                } else {
                    Action::Go(Route::Welcome)
                }
            }
            Msg::Pick(level) => {
                let cur = ctx.state.settings.level;
                if ctx.state.onboarded && cur.is_some() && cur != Some(level) {
                    self.confirm = Some(level);
                    Action::None
                } else {
                    self.apply(ctx, level)
                }
            }
            Msg::ConfirmYes => match self.confirm {
                Some(level) => self.apply(ctx, level),
                None => Action::None,
            },
            Msg::ConfirmNo => {
                self.confirm = None;
                Action::None
            }
        }
    }

    pub fn subscription(&self) -> iced::Subscription<Msg> {
        iced::Subscription::none()
    }

    pub fn view<'a>(&'a self, ctx: &'a Ctx) -> Element<'a, Msg> {
        let p = theme::current();
        let first = !ctx.state.onboarded;

        let mut page = Column::new().spacing(10).width(Length::Fill);
        page = page.push(
            button(row![txt("‹").size(22.0), txt("Назад").size(15.0)].spacing(2))
                .padding([6.0, 10.0])
                .style(move |_, status| theme::button_ghost(&theme::current(), status))
                .on_press(Msg::Back),
        );
        page = page.push(widgets::kicker(if first {
            "Шаг 2 из 2"
        } else {
            "Течение"
        }));
        page = page.push(widgets::heading("Выберите течение", 1));
        page = page.push(
            txt("Уровни вложены друг в друга: консервативный включает всё реформистское, ортодоксальный — всё. Прогресс сохранится; уроки и вопросы другого уровня появятся или скроются.")
                .size(theme::text::BODY)
                .color(p.muted)
                .width(Length::Fill),
        );
        page = page.push(space::vertical().height(8.0));

        let cur = ctx.state.settings.level;
        let mut cards = Row::new().spacing(16).width(Length::Fill);
        for (i, level) in [Level::Reform, Level::Conservative, Level::Orthodox]
            .into_iter()
            .enumerate()
        {
            cards = cards.push(level_card(level, i + 1, cur == Some(level), p));
        }
        page = page.push(cards);

        let base: Element<'a, Msg> = page.into();
        match self.confirm {
            Some(level) => stack![base, opaque(confirm_dialog(level, p))].into(),
            None => base,
        }
    }
}

fn level_card<'a>(
    level: Level,
    number: usize,
    selected: bool,
    p: theme::Palette,
) -> Element<'a, Msg> {
    let info = content::level_info(level);
    let name = match level {
        Level::Reform => "level-reform",
        Level::Conservative => "level-conservative",
        Level::Orthodox => "level-orthodox",
    };
    let emblem: Element<'a, Msg> = match assets::img(name) {
        Some(handle) => iced::widget::image(handle)
            .width(104.0)
            .height(104.0)
            .into(),
        None => icons::level_rings(number, 104.0, p.gold2),
    };
    button(
        column![
            emblem,
            kicker_owned(format!("Уровень {number}")),
            txt(info.title)
                .size(theme::text::H3)
                .font(fonts::SANS_BOLD)
                .align_x(Alignment::Center)
                .width(Length::Fill),
            txt(info.text)
                .size(theme::text::SMALL)
                .color(p.muted)
                .align_x(Alignment::Center)
                .width(Length::Fill),
        ]
        .spacing(10)
        .align_x(Alignment::Center)
        .width(Length::Fill),
    )
    .width(Length::FillPortion(1))
    .padding(20.0)
    .style(move |_, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        let (bg, border_color, width) = if selected {
            (p.gold_soft, p.gold, 2.0)
        } else if hovered {
            (p.surface2, p.gold2, 1.0)
        } else {
            (p.surface, p.line, 1.0)
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: p.text,
            border: Border {
                color: border_color,
                width,
                radius: theme::radius::CARD.into(),
            },
            shadow: iced::Shadow::default(),
            snap: true,
        }
    })
    .on_press(Msg::Pick(level))
    .into()
}

fn confirm_dialog<'a>(level: Level, p: theme::Palette) -> Element<'a, Msg> {
    let info = content::level_info(level);
    let dialog = container(
        column![
            heading_owned(format!("Сменить течение на «{}»?", info.title), 2),
            txt("Прогресс сохранится; уроки и вопросы другого уровня появятся или скроются.")
                .size(theme::text::BODY)
                .color(p.muted)
                .width(Length::Fill),
            row![
                space::horizontal(),
                button(txt("Отмена").size(theme::text::BODY - 1.0))
                    .padding([10.0, 20.0])
                    .style(move |_, status| theme::button_ghost(&theme::current(), status))
                    .on_press(Msg::ConfirmNo),
                button(
                    txt("Сменить")
                        .size(theme::text::BODY - 1.0)
                        .font(fonts::SANS_BOLD)
                )
                .padding([10.0, 20.0])
                .style(move |_, status| theme::button_primary(&theme::current(), status))
                .on_press(Msg::ConfirmYes),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        ]
        .spacing(16),
    )
    .padding(24.0)
    .max_width(460.0)
    .width(Length::Fill)
    .style(move |_| iced::widget::container::Style {
        background: Some(Background::Color(p.surface)),
        border: Border {
            color: p.line2,
            width: 1.0,
            radius: theme::radius::MODAL.into(),
        },
        shadow: iced::Shadow {
            color: Color::from_rgba8(0, 0, 0, 0.4),
            offset: iced::Vector::new(0.0, 12.0),
            blur_radius: 34.0,
        },
        ..iced::widget::container::Style::default()
    });

    container(dialog)
        .width(Length::Fill)
        .height(Length::Fill)
        .center(Length::Fill)
        .padding(32.0)
        .style(|_| iced::widget::container::Style {
            background: Some(Background::Color(Color::from_rgba8(3, 5, 12, 0.66))),
            ..iced::widget::container::Style::default()
        })
        .into()
}
