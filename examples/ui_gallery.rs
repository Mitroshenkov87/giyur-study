//! Live gallery of the `ui` module: every widget with real Russian and Hebrew
//! samples, plus a theme toggle. Run with `cargo run --example ui_gallery`.

use shorashim::ui;

use iced::widget::{
    button, column, container, image, progress_bar, row, scrollable, space, stack, text,
    text_input, Column, Row,
};
use iced::{Center, Element, Length, Task};

use ui::theme::{self, Palette};
use ui::widgets;
use ui::{assets, fonts};

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    InputChanged(String),
    BackPressed,
}

struct Gallery {
    kind: theme::ThemeKind,
    input: String,
    tokens: String,
}

fn init() -> (Gallery, Task<Message>) {
    theme::set_current(theme::ThemeKind::Dark);
    let load_fonts = Task::batch(fonts::all().into_iter().map(iced::font::load)).discard();
    let tokens = format!(
        "Радиусы (px): карточка {}, малый {}, кнопка {}, пилюля ∞, сегмент {}, модал {}, \
         переворот {}, эмблема {} (большая {}), hero {}, таймер {}.\n\
         Отступы: карточка {:.1}, ряд {:.1}, контент {:.0} px. \
         Минимальные высоты: кнопка {}, кнопка lg {}, чип {}, ввод {}, ссылка назад 40.\n\
         Изображений встроено: {}.",
        theme::radius::CARD,
        theme::radius::SMALL,
        theme::radius::BUTTON,
        theme::radius::SEGMENT,
        theme::radius::MODAL,
        theme::radius::FLIP,
        theme::radius::EMBLEM,
        theme::radius::EMBLEM_LG,
        theme::radius::HERO,
        theme::radius::TIMED,
        theme::spacing::CARD_PAD,
        theme::spacing::ROW,
        theme::spacing::CONTENT_MAX,
        theme::spacing::BTN_MIN,
        theme::spacing::BTN_LG_MIN,
        theme::spacing::CHIP_MIN,
        theme::spacing::INPUT_MIN,
        assets::names().count(),
    );
    (
        Gallery {
            kind: theme::ThemeKind::Dark,
            input: String::new(),
            tokens,
        },
        load_fonts,
    )
}

fn update(state: &mut Gallery, message: Message) -> Task<Message> {
    match message {
        Message::ToggleTheme => {
            state.kind = state.kind.other();
            theme::set_current(state.kind);
        }
        Message::InputChanged(value) => state.input = value,
        Message::BackPressed => {}
    }
    Task::none()
}

fn section<'a>(title: &'a str, content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    let inner = Column::new()
        .spacing(12)
        .push(widgets::heading(title, 2))
        .push(content.into());
    widgets::card(inner)
}

fn styled_button(
    label: &'static str,
    style: fn(&Palette, iced::widget::button::Status) -> iced::widget::button::Style,
) -> Element<'static, Message> {
    button(text(label).shaping(iced::widget::text::Shaping::Advanced))
        .padding([10.0, 22.0])
        .style(move |_theme, status| style(&theme::current(), status))
        .into()
}

fn option_button(
    label: &'static str,
    style: fn(&Palette, iced::widget::button::Status) -> iced::widget::button::Style,
) -> Element<'static, Message> {
    button(text(label).shaping(iced::widget::text::Shaping::Advanced))
        .padding([12.0, 16.0])
        .width(Length::Fill)
        .style(move |_theme, status| style(&theme::current(), status))
        .into()
}

/// A small box demonstrating a border radius.
fn radius_sample(radius: f32, label: &'static str) -> Element<'static, Message> {
    column![
        container(space::vertical())
            .width(56.0)
            .height(36.0)
            .style(move |_theme| iced::widget::container::Style {
                background: Some(iced::Background::Color(theme::current().surface3)),
                border: iced::Border {
                    color: theme::current().line2,
                    width: 1.0,
                    radius: radius.into(),
                },
                ..plain_style()
            }),
        text(label).size(11.0).color(theme::current().muted),
    ]
    .spacing(4)
    .align_x(Center)
    .into()
}

fn plain_style() -> iced::widget::container::Style {
    iced::widget::container::Style {
        text_color: None,
        background: None,
        border: iced::Border::default(),
        shadow: iced::Shadow::default(),
        snap: true,
    }
}

fn sample_image<'a>(name: &'a str, width: f32, caption: &'a str) -> Element<'a, Message> {
    let content: Element<'_, Message> = match assets::img(name) {
        Some(handle) => image(handle).width(Length::Fixed(width)).into(),
        None => text(format!("({name} не найдено)")).into(),
    };
    column![content, widgets::tag(caption)]
        .spacing(6)
        .align_x(Center)
        .into()
}

fn view(state: &Gallery) -> Element<'_, Message> {
    let header = container(
        column![
            widgets::page_header("Витрина виджетов", Some(Message::BackPressed)),
            row![
                text(format!("Тема: {}", state.kind.title())).size(15),
                button(
                    text(if state.kind == theme::ThemeKind::Dark {
                        "Кожа"
                    } else {
                        "Тёмная"
                    })
                    .shaping(iced::widget::text::Shaping::Advanced)
                )
                .padding([8.0, 18.0])
                .style(move |_theme, status| theme::button_primary(&theme::current(), status))
                .on_press(Message::ToggleTheme),
            ]
            .spacing(16)
            .align_y(Center),
        ]
        .spacing(14),
    )
    .padding(20.0)
    .style(|_| theme::hero(&theme::current()));

    let typography = section(
        "Типографика и markdown-lite",
        column![
            widgets::kicker("Подготовка к гиюру"),
            widgets::heading("Шма Исраэль — основа молитвы", 1),
            widgets::heading("Раздел 3 · Шаббат", 2),
            widgets::heading("Уроки и карточки", 3),
            widgets::rich_paragraph(
                "Обычный текст с **жирным выделением**, *курсивом* и ивритом внутри строки: \
                 שְׁמַע יִשְׂרָאֵל — שמע ישראל. Шаббат (שַׁבָּת) начинается с заходом солнца.",
            ),
        ]
        .spacing(10),
    );

    let hebrew = section(
        "Иврит-блок",
        widgets::hebrew_block(
            "שְׁמַע יִשְׂרָאֵל יְהוָה אֱלֹהֵינוּ יְהוָה אֶחָד",
            Some("Шма Исраэль, Адонай Элохейну, Адонай эхад"),
            Some("Слушай, Израиль: Господь, Бог наш, Господь — един!"),
            Some("Дварим 6:4"),
        ),
    );

    let quote = section(
        "Цитата, термин, совет",
        column![
            widgets::quote_block(
                "Кто спасает одну жизнь — тот спасает целый мир.",
                Some("Мишна, Сангедрин 4:5"),
            ),
            widgets::term_block(
                "Тфилин",
                Some("תְּפִלִּין"),
                "Кожаные коробочки с отрывками Торы, которые носят на руке и на голове во время утренней молитвы.",
            ),
            widgets::note_block(
                "Совет",
                "Повторяйте карточки каждый день: интервалы 1 → 3 → 7 → 14 → 30 дней помогают запомнить надолго.",
            ),
        ]
        .spacing(12),
    );

    let differs = section(
        "Заметки «в других течениях»",
        column![
            widgets::differs_note(
                theme::Movement::Reform.label(),
                "В реформистском иудаизме соблюдение кашрута носит добровольный характер.",
            ),
            widgets::differs_note(
                theme::Movement::Conservative.label(),
                "Масорти требуют кашрута в общественных мероприятиях общины.",
            ),
            widgets::differs_note(
                theme::Movement::Orthodox.label(),
                "Ортодоксальная практика требует отдельной посуды для молочного и мясного.",
            ),
        ]
        .spacing(10),
    );

    let progress = section(
        "Прогресс",
        column![
            widgets::step_bar(3, 8),
            widgets::step_bar(20, 20),
            row![
                stack![
                    widgets::progress_ring::<Message>(0.75),
                    container(
                        text("75%")
                            .size(15)
                            .shaping(iced::widget::text::Shaping::Advanced)
                    )
                    .center(Length::Fill),
                ],
                progress_bar(0.0..=100.0, 62.0).style(|_| theme::progress_bar(&theme::current())),
            ]
            .spacing(20)
            .align_y(Center),
        ]
        .spacing(12),
    );

    let buttons = section(
        "Кнопки",
        column![
            row![
                styled_button("Начать", theme::button_primary),
                styled_button("Вторичная", theme::button_secondary),
                styled_button("Назад", theme::button_ghost),
                styled_button("Сбросить прогресс", theme::button_danger),
                styled_button("Начать игру", theme::button_tq_go),
            ]
            .spacing(10)
            .wrap(),
            row![
                widgets::chip("Масорти уровень"),
                widgets::chip("10 / 20 XP"),
                widgets::tag("повтор"),
                widgets::tag("лёгкий"),
                widgets::toast("Урок отмечен прочитанным"),
            ]
            .spacing(10)
            .align_y(Center)
            .wrap(),
        ]
        .spacing(14),
    );

    let quiz_options = section(
        "Варианты ответов",
        column![
            Row::new()
                .push(option_button("Иерусалим", theme::option_normal))
                .push(option_button("Тель-Авив", theme::option_selected),),
            Row::new()
                .push(option_button("Шаббат", theme::option_correct))
                .push(option_button("Йом-Кипур (выбрано)", theme::option_wrong),),
        ]
        .spacing(10),
    );

    let input = section(
        "Поле ввода",
        text_input("Ваш ответ", &state.input)
            .on_input(Message::InputChanged)
            .padding(12)
            .style(|_theme, status| theme::text_input(&theme::current(), status)),
    );

    let stats = section(
        "Показатели",
        row![
            widgets::stat_tile("12", "дней подряд"),
            widgets::stat_tile("34/56", "уроков прочитано"),
            widgets::stat_tile("128", "карточек выучено"),
            widgets::stat_tile("5", "экзаменов пройдено"),
        ]
        .spacing(12),
    );

    let images = section(
        "Изображения",
        row![
            sample_image("welcome", 260.0, "welcome.webp"),
            sample_image("section-shabbat", 96.0, "section-shabbat"),
            sample_image("badge-streak-7", 96.0, "badge-streak-7"),
            sample_image("level-orthodox", 96.0, "level-orthodox"),
            sample_image("app-icon", 96.0, "app-icon"),
        ]
        .spacing(16)
        .wrap(),
    );

    let timed = container(
        column![
            widgets::heading("Экзамен", 2),
            widgets::rich_paragraph(
                "Вопрос, четыре варианта и таймер — как в телевикторине. Звуковые сигналы отключены.",
            ),
            styled_button("Начать игру", theme::button_tq_go),
        ]
        .spacing(12),
    )
    .padding(24)
    .style(|_| theme::tq_container(&theme::TIMED));

    let tokens = section(
        "Дизайн-токены (§T.5)",
        column![
            widgets::rich_paragraph(&state.tokens),
            row![
                radius_sample(theme::radius::CARD, "card 18"),
                radius_sample(theme::radius::SMALL, "small 12"),
                radius_sample(theme::radius::BUTTON, "btn 14"),
                radius_sample(theme::radius::PILL, "pill"),
                radius_sample(theme::radius::SEGMENT, "seg 10"),
                radius_sample(theme::radius::MODAL, "modal 20"),
                radius_sample(theme::radius::FLIP, "flip 22"),
                radius_sample(theme::radius::EMBLEM, "emb 16"),
                radius_sample(theme::radius::EMBLEM_LG, "emb-lg 22"),
                radius_sample(theme::radius::HERO, "hero 24"),
                radius_sample(theme::radius::TIMED, "timed 26"),
            ]
            .spacing(10)
            .wrap(),
        ]
        .spacing(12),
    );

    let body = column![
        header,
        typography,
        hebrew,
        quote,
        differs,
        progress,
        buttons,
        quiz_options,
        input,
        stats,
        images,
        timed,
        tokens,
    ]
    .spacing(20)
    .width(Length::Fill)
    .max_width(theme::spacing::CONTENT_MAX + 100.0);

    scrollable(container(body).padding(28.0).center_x(Length::Fill)).into()
}

fn main() -> iced::Result {
    iced::application(init, update, view)
        .title("Шорашим — витрина UI")
        .theme(|state: &Gallery| theme::iced_theme(state.kind))
        .window(iced::window::Settings {
            size: iced::Size::new(1100.0, 820.0),
            min_size: Some(iced::Size::new(760.0, 560.0)),
            icon: assets::window_icon(),
            ..Default::default()
        })
        .run()
}
