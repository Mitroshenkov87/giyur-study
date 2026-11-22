//! Section page (PARITY §4): emblem header with progress ring, action cards
//! (cards deck / section test) and the lessons list with read state.

use iced::widget::{button, column, container, progress_bar, row, stack, Column};
use iced::{Alignment, Background, Border, Element, Length};

use crate::content::{self, Library, SectionMeta};
use crate::srs;
use crate::ui::app::{Action, Ctx, Deck, Route};
use crate::ui::screens::map::{alpha, emblem, fmt_pct, icons, kicker_owned, txt};
use crate::ui::{fonts, theme, widgets};

pub struct Screen {
    id: String,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Go(Route),
}

impl Screen {
    pub fn new(_ctx: &mut Ctx, route: &Route) -> Self {
        let id = match route {
            Route::Section(id) => id.clone(),
            _ => String::new(),
        };
        Screen { id }
    }

    pub fn update(&mut self, _ctx: &mut Ctx, msg: Msg) -> Action {
        match msg {
            Msg::Go(route) => Action::Go(route),
        }
    }

    pub fn subscription(&self) -> iced::Subscription<Msg> {
        iced::Subscription::none()
    }

    pub fn view<'a>(&'a self, ctx: &'a Ctx) -> Element<'a, Msg> {
        let p = theme::current();
        let Some(sec) = ctx.lib.section(&self.id) else {
            return not_found("Раздел ещё не готов.", false, p);
        };
        if !sec.available {
            return not_found("Раздел ещё не готов.", false, p);
        }
        if !ctx.lib.is_unlocked(sec, &ctx.state) {
            return not_found("Этот раздел пока закрыт.", true, p);
        }

        let user = Library::user_level(&ctx.state);
        let stats = ctx.lib.section_stats(sec, user, &ctx.state);
        let lessons = ctx.lib.lessons(sec, user);

        let mut page = Column::new().spacing(18).width(Length::Fill);

        // ---- Header: emblem · back/kicker/h1/sub · ring ----
        let ring = stack![
            icons::ring(stats.pct as f32 / 100.0, 76.0, p.gold),
            container(
                txt(format!("{}%", stats.pct))
                    .size(15.0)
                    .font(fonts::SANS_BOLD)
            )
            .width(76.0)
            .height(76.0),
        ];
        let head_text = column![
            back_button("Карта", Msg::Go(Route::Map)),
            kicker_owned(format!("Раздел {}", sec.index + 1)),
            widgets::heading(&sec.title, 1),
            txt(&sec.subtitle).size(theme::text::BODY).color(p.muted),
        ]
        .spacing(8)
        .width(Length::Fill);
        page = page.push(
            row![
                emblem(&sec.id, &sec.icon, 96.0, theme::radius::EMBLEM_LG, false),
                head_text,
                ring,
            ]
            .spacing(20)
            .align_y(Alignment::Center),
        );

        // ---- Intro ----
        if let Some(intro) = sec
            .data
            .as_ref()
            .and_then(|d| d.intro.as_deref())
            .filter(|i| !i.is_empty())
        {
            page = page.push(widgets::rich_paragraph(intro));
        }

        // ---- Action cards ----
        let cards_action = action_card(
            icons::Kind::Cards,
            false,
            "Карточки раздела",
            format!("{} из {} выучено", stats.mastered, stats.cards),
            Msg::Go(Route::Cards(Deck::Section(sec.id.clone()))),
            p,
        );
        let test_sub = match stats.best {
            None => "Ещё не сдавали · порог 80%".to_string(),
            Some(best) => format!("Лучший результат: {} · порог 80%", fmt_pct(best)),
        };
        let test_action = action_card(
            icons::Kind::Exam,
            true,
            "Тест раздела",
            test_sub,
            Msg::Go(Route::Exam(Some(sec.id.clone()))),
            p,
        );
        page = page.push(
            row![cards_action, test_action]
                .spacing(12)
                .align_y(Alignment::Center),
        );

        // ---- Lessons ----
        page = page.push(widgets::heading("Уроки", 3));
        if lessons.is_empty() {
            page = page.push(
                txt("Для вашего уровня в этом разделе пока нет уроков.")
                    .size(theme::text::BODY)
                    .color(p.muted),
            );
        } else {
            let mut list = Column::new().spacing(8).width(Length::Fill);
            for (i, lesson) in lessons.iter().enumerate() {
                list = list.push(lesson_row(ctx, sec, lesson, i + 1, user, p));
            }
            page = page.push(list);
        }

        page.into()
    }
}

fn back_button<'a>(label: &'a str, msg: Msg) -> Element<'a, Msg> {
    button(row![txt("‹").size(22.0), txt(label).size(15.0)].spacing(2))
        .padding([6.0, 10.0])
        .style(move |_, status| theme::button_ghost(&theme::current(), status))
        .on_press(msg)
        .into()
}

fn action_card<'a>(
    kind: icons::Kind,
    gold: bool,
    title: &'a str,
    sub: String,
    msg: Msg,
    p: theme::Palette,
) -> Element<'a, Msg> {
    button(
        row![
            icons::icon(kind, 24.0, if gold { p.gold } else { p.gold2 }),
            column![
                txt(title).size(theme::text::BODY).font(fonts::SANS_BOLD),
                txt(sub).size(theme::text::SMALL).color(p.muted),
            ]
            .spacing(2),
        ]
        .spacing(14)
        .align_y(Alignment::Center),
    )
    .width(Length::FillPortion(1))
    .padding(18.0)
    .style(move |_, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        let (bg, border) = if gold {
            (
                if hovered {
                    alpha(p.gold, 0.22)
                } else {
                    p.gold_soft
                },
                p.gold,
            )
        } else {
            (if hovered { p.surface2 } else { p.surface }, p.line)
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: p.text,
            border: Border {
                color: border,
                width: 1.0,
                radius: theme::radius::CARD.into(),
            },
            shadow: iced::Shadow::default(),
            snap: true,
        }
    })
    .on_press(msg)
    .into()
}

fn lesson_row<'a>(
    ctx: &'a Ctx,
    sec: &'a SectionMeta,
    lesson: &'a content::Lesson,
    number: usize,
    user: crate::storage::Level,
    p: theme::Palette,
) -> Element<'a, Msg> {
    let cards = ctx.lib.lesson_cards(lesson, user);
    let total = cards.len();
    let mastered = cards
        .iter()
        .filter(|c| srs::is_mastered(&ctx.state, &c.id))
        .count();
    let read = Library::is_read(&ctx.state, &lesson.id);
    let minutes = lesson.minutes.unwrap_or(5);
    let pct = if total > 0 {
        mastered as f32 / total as f32 * 100.0
    } else {
        0.0
    };

    let circle_inner: Element<'a, Msg> = if read {
        icons::icon(icons::Kind::Check, 16.0, p.on_gold)
    } else {
        txt(number.to_string())
            .size(14.0)
            .font(fonts::SANS_BOLD)
            .color(p.text)
            .into()
    };
    let circle = container(circle_inner)
        .width(36.0)
        .height(36.0)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .style(move |_| iced::widget::container::Style {
            background: Some(Background::Color(if read { p.gold } else { p.surface3 })),
            border: Border {
                color: if read { p.gold } else { p.line2 },
                width: 1.0,
                radius: theme::radius::PILL.into(),
            },
            ..iced::widget::container::Style::default()
        });

    let mut text_col = Column::new().spacing(4).width(Length::Fill);
    text_col = text_col.push(
        txt(lesson.title.as_deref().unwrap_or(&lesson.id))
            .size(theme::text::BODY)
            .font(fonts::SANS_BOLD),
    );
    text_col = text_col.push(
        row![
            icons::icon(icons::Kind::Clock, 12.0, p.muted),
            txt(format!("{minutes} мин · карточки {mastered}/{total}"))
                .size(theme::text::SMALL)
                .color(p.muted),
        ]
        .spacing(5)
        .align_y(Alignment::Center),
    );
    text_col = text_col.push(
        progress_bar(0.0..=100.0, pct)
            .girth(4.0)
            .style(move |_| theme::progress_bar(&theme::current())),
    );

    let mut content = row![circle, text_col]
        .spacing(12)
        .align_y(Alignment::Center);
    if let Some(min_level) = lesson.min_level {
        if min_level != crate::storage::Level::Reform {
            content = content.push(widgets::tag(content::level_info(min_level).short));
        }
    }
    let _ = sec;

    button(content)
        .width(Length::Fill)
        .padding([10.0, 12.0])
        .style(move |_, status| {
            let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: Some(Background::Color(if hovered {
                    p.surface2
                } else {
                    p.surface
                })),
                text_color: p.text,
                border: Border {
                    color: p.line,
                    width: 1.0,
                    radius: theme::radius::SMALL.into(),
                },
                shadow: iced::Shadow::default(),
                snap: true,
            }
        })
        .on_press(Msg::Go(Route::Lesson(lesson.id.clone())))
        .into()
}

/// «Не найдено» / «Раздел закрыт» view (§0.3, §4).
fn not_found<'a>(message: &'a str, locked: bool, p: theme::Palette) -> Element<'a, Msg> {
    column![
        back_button("Карта", Msg::Go(Route::Map)),
        widgets::heading(
            if locked {
                "Раздел закрыт"
            } else {
                "Не найдено"
            },
            1
        ),
        widgets::card(
            column![
                icons::icon(icons::Kind::StarDavid, 56.0, p.gold),
                txt(message).size(theme::text::BODY).color(p.text),
                button(
                    txt("На карту")
                        .size(theme::text::BODY)
                        .font(fonts::SANS_BOLD)
                )
                .padding([12.0, 26.0])
                .style(move |_, status| theme::button_primary(&theme::current(), status))
                .on_press(Msg::Go(Route::Map)),
            ]
            .spacing(16)
            .align_x(Alignment::Center)
            .width(Length::Fill),
        ),
    ]
    .spacing(12)
    .width(Length::Fill)
    .into()
}
