//! Welcome screen (PARITY §1): hero with overlaid text, feature list,
//! rabbi-review notice and the «Начать» button.

use iced::widget::{button, column, container, row, space, stack, Column};
use iced::{Alignment, Background, Border, Color, Element, Length};

use crate::ui::app::{Action, Ctx, Route};
use crate::ui::screens::map::{alpha, icons, txt, v_gradient};
use crate::ui::{assets, fonts, theme, widgets};

pub struct Screen;

#[derive(Debug, Clone)]
pub enum Msg {
    Start,
}

impl Screen {
    pub fn new(_ctx: &mut Ctx, _route: &Route) -> Self {
        Screen
    }

    pub fn update(&mut self, _ctx: &mut Ctx, msg: Msg) -> Action {
        match msg {
            Msg::Start => Action::Go(Route::Level { reopened: false }),
        }
    }

    pub fn subscription(&self) -> iced::Subscription<Msg> {
        iced::Subscription::none()
    }

    pub fn view<'a>(&'a self, _ctx: &'a Ctx) -> Element<'a, Msg> {
        let p = theme::current();

        // ---- Hero (16:9-ish, welcome.webp with fallback) ----
        let hero_inner: Element<'a, Msg> = match assets::img("welcome") {
            Some(handle) => iced::widget::image(handle)
                .width(Length::Fill)
                .height(Length::Fill)
                .content_fit(iced::ContentFit::Cover)
                .into(),
            None => container(icons::icon(
                icons::Kind::StarDavid,
                110.0,
                Color::from_rgb8(0xe6, 0xc3, 0x6a),
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_| iced::widget::container::Style {
                background: Some(v_gradient(
                    Color::from_rgb8(0x1b, 0x25, 0x47),
                    Color::from_rgb8(0x0b, 0x10, 0x20),
                )),
                ..iced::widget::container::Style::default()
            })
            .into(),
        };

        // Bottom dark gradient + overlaid text (fixed light colours: the hero
        // art is dark in both themes).
        let over_text = Color::from_rgb8(0xec, 0xe7, 0xda);
        let over_gold = Color::from_rgb8(0xe6, 0xc3, 0x6a);
        let overlay = container(
            column![
                txt("Добро пожаловать".to_uppercase())
                    .size(theme::text::KICKER)
                    .font(fonts::SANS_BOLD)
                    .color(over_gold),
                txt("Giyur Study")
                    .size(42.0)
                    .font(fonts::SERIF_BOLD)
                    .color(over_text),
                container(
                    txt("Спокойная, последовательная подготовка к гиюру и собеседованию в бейт-дине.")
                        .size(theme::text::H3)
                        .color(over_text)
                        .width(Length::Fill),
                )
                .max_width(640.0),
            ]
            .spacing(8),
        )
        .padding([20.0, 24.0])
        .width(Length::Fill)
        .align_y(iced::alignment::Vertical::Bottom);

        let hero_height = 360.0;
        let hero = container(stack![
            hero_inner,
            container(space::horizontal())
                .width(Length::Fill)
                .height(Length::Fill)
                .style(|_| iced::widget::container::Style {
                    background: Some(v_gradient(
                        Color::from_rgba8(8, 11, 22, 0.0),
                        Color::from_rgba8(8, 11, 22, 0.92),
                    )),
                    ..iced::widget::container::Style::default()
                }),
            container(overlay).width(Length::Fill).height(Length::Fill),
        ])
        .width(Length::Fill)
        .height(hero_height)
        .style(move |_| iced::widget::container::Style {
            border: Border {
                color: p.line,
                width: 1.0,
                radius: theme::radius::HERO.into(),
            },
            ..iced::widget::container::Style::default()
        });

        // ---- Body card (overlaps the hero by ~3rem, §1) ----
        let feature = |kind: icons::Kind, body: &'a str| {
            row![
                icons::icon(kind, 20.0, p.gold2),
                widgets::rich_paragraph(body),
            ]
            .spacing(12)
            .align_y(Alignment::Center)
        };
        let body_card = widgets::card(
            column![
                feature(
                    icons::Kind::Book,
                    "**8 разделов** — от основ веры до бейт-дина, уроки по 5–10 минут.",
                ),
                feature(
                    icons::Kind::Cards,
                    "**Карточки** с интервальным повторением: 1 → 3 → 7 → 14 → 30 дней.",
                ),
                feature(
                    icons::Kind::Exam,
                    "**Тесты и пробный бейт-дин** с разбором ошибок.",
                ),
                container(
                    row![
                        icons::icon(icons::Kind::Sources, 20.0, p.gold2),
                        txt("Материалы требуют проверки раввином вашей общины. Приложение — помощник в учёбе, а не замена учителю.")
                            .size(theme::text::SMALL + 1.0)
                            .color(p.text)
                            .width(Length::Fill),
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center),
                )
                .padding([12.0, 16.0])
                .width(Length::Fill)
                .style(move |_| iced::widget::container::Style {
                    background: Some(Background::Color(p.gold_soft)),
                    border: Border {
                        color: alpha(p.gold, 0.25),
                        width: 1.0,
                        radius: theme::radius::SMALL.into(),
                    },
                    ..iced::widget::container::Style::default()
                }),
                txt("Всё работает без интернета; прогресс хранится только на этом устройстве.")
                    .size(theme::text::SMALL)
                    .color(p.muted),
                row![
                    space::horizontal(),
                    button(
                        txt("Начать")
                            .size(theme::text::BODY)
                            .font(fonts::SANS_BOLD)
                    )
                    .padding([14.0, 34.0])
                    .style(move |_, status| theme::button_primary(&theme::current(), status))
                    .on_press(Msg::Start),
                ]
                .align_y(Alignment::Center),
            ]
            .spacing(16),
        );

        let overlap = 48.0;
        Column::new()
            .push(stack![
                hero,
                column![space::vertical().height(hero_height - overlap), body_card,]
                    .width(Length::Fill),
            ])
            .width(Length::Fill)
            .into()
    }
}
