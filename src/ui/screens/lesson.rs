//! Lesson reader (PARITY §5, js/ui/lesson.js).

use std::collections::HashSet;

use iced::alignment::Horizontal;
use iced::widget::text::{Shaping, Text};
use iced::widget::{button, column, container, row, text, Column, Row, Space};
use iced::{Alignment, Background, Element, Length, Subscription};

use crate::content::{self, Block, Differs, Lesson, SectionMeta};
use crate::game;
use crate::storage::Level;
use crate::ui::app::{Action, Ctx, Deck, Route};
use crate::ui::fonts::SANS_BOLD;
use crate::ui::theme::{self, Palette};
use crate::ui::widgets;

fn txt<'a>(content: impl iced::widget::text::IntoFragment<'a>) -> Text<'a> {
    text(content).shaping(Shaping::Advanced)
}

fn palette() -> Palette {
    theme::current()
}

/// Guard failure: lesson missing/invisible or its section locked.
#[derive(Debug, Clone, PartialEq)]
enum Guard {
    /// «Урок не найден.»
    NotFound,
    /// «Этот раздел пока закрыт.»
    Locked,
}

pub struct Screen {
    lesson_id: String,
    guard: Option<Guard>,
    /// Expanded «Как в других течениях» groups (by deterministic index).
    open_others: HashSet<u32>,
    /// «**Ссылки:** …» line, precomputed (owned) for the sources footer.
    citations_line: Option<String>,
    /// «Урок {pos} из {count} · {minutes} мин» header line.
    kicker: String,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Go(Route),
    MarkRead,
    ToggleDiffers(u32),
    OpenUrl(String),
}

impl Screen {
    pub fn new(ctx: &mut Ctx, route: &Route) -> Self {
        let lesson_id = match route {
            Route::Lesson(id) => id.clone(),
            _ => String::new(),
        };
        let guard = match ctx.lib.lesson(&lesson_id) {
            None => Some(Guard::NotFound),
            Some((lesson, sec)) => {
                let user = content::Library::user_level(&ctx.state);
                if !content::Library::visible(lesson.min_level, None, user) {
                    Some(Guard::NotFound)
                } else if !ctx.lib.is_unlocked(sec, &ctx.state) {
                    Some(Guard::Locked)
                } else {
                    None
                }
            }
        };
        let citations_line = ctx
            .lib
            .lesson(&lesson_id)
            .filter(|(_, _)| guard.is_none())
            .and_then(|(lesson, _)| {
                (!lesson.citations.is_empty())
                    .then(|| format!("**Ссылки:** {}", lesson.citations.join("; ")))
            });
        let kicker = match guard {
            Some(_) => String::new(),
            None => {
                let (lesson, sec) = ctx.lib.lesson(&lesson_id).expect("guard checked");
                let user = content::Library::user_level(&ctx.state);
                let lessons = ctx.lib.lessons(sec, user);
                let pos = lessons.iter().position(|l| l.id == lesson.id).unwrap_or(0);
                format!(
                    "Урок {} из {} · {} мин",
                    pos + 1,
                    lessons.len(),
                    lesson.minutes.unwrap_or(5)
                )
            }
        };
        Screen {
            lesson_id,
            guard,
            open_others: HashSet::new(),
            citations_line,
            kicker,
        }
    }

    pub fn update(&mut self, ctx: &mut Ctx, msg: Msg) -> Action {
        match msg {
            Msg::Go(route) => Action::Go(route),
            Msg::MarkRead => {
                let today = ctx.today_key();
                ctx.state.lessons_read.insert(self.lesson_id.clone(), today);
                ctx.activity();
                ctx.toast("Урок отмечен прочитанным", "", "✓");
                ctx.award(game::XP.lesson);
                Action::None
            }
            Msg::ToggleDiffers(id) => {
                if !self.open_others.remove(&id) {
                    self.open_others.insert(id);
                }
                Action::None
            }
            Msg::OpenUrl(url) => {
                widgets::open_url(&url);
                Action::None
            }
        }
    }

    pub fn view<'a>(&'a self, ctx: &'a Ctx) -> Element<'a, Msg> {
        if let Some(guard) = &self.guard {
            return not_found_view(guard);
        }
        let Some((lesson, sec)) = ctx.lib.lesson(&self.lesson_id) else {
            return not_found_view(&Guard::NotFound);
        };

        let user = content::Library::user_level(&ctx.state);
        let lessons = ctx.lib.lessons(sec, user);
        let pos = lessons.iter().position(|l| l.id == lesson.id).unwrap_or(0);
        let next = lessons.get(pos + 1);
        let read = content::Library::is_read(&ctx.state, &lesson.id);
        let translit = ctx.state.settings.translit;

        // Fixed top reading-progress indicator (§5; scroll tracking lives in the
        // shell's scrollable, so the bar is shown as a full step bar).
        let bar = widgets::step_bar::<Msg>(1, 1);

        let header = header(
            &sec.title,
            Msg::Go(Route::Section(sec.id.clone())),
            &self.kicker,
            lesson.title.as_deref().unwrap_or(""),
        );

        let mut article = Column::new().spacing(14).width(Length::Fill);

        let mut group = 0u32;
        for block in ctx.lib.lesson_blocks(lesson, user) {
            if let Some(el) = block_view(block, translit) {
                article = article.push(el);
            }
            if let Some(el) =
                differs_view(block_differs(block), user, &mut group, &self.open_others)
            {
                article = article.push(el);
            }
        }

        if let Some(footer) = sources_footer(ctx, sec, lesson, self.citations_line.as_deref()) {
            article = article.push(footer);
        }

        let mut root = Column::new().spacing(16).width(Length::Fill);
        root = root.push(bar);
        root = root.push(header);
        root = root.push(widgets::card(article));

        // Actions (centred): mark read, or read mark + cards button.
        let actions: Element<Msg> = if read {
            let p = palette();
            row![
                txt("✓ Прочитано")
                    .size(theme::text::BODY)
                    .font(SANS_BOLD)
                    .color(p.ok),
                widgets::btn(
                    "К карточкам урока",
                    widgets::Btn::Primary,
                    widgets::BtnSize::Lg,
                    Some(Msg::Go(Route::Cards(Deck::Lesson(lesson.id.clone())))),
                ),
            ]
            .spacing(16)
            .align_y(Alignment::Center)
            .into()
        } else {
            row![widgets::btn(
                "Отметить прочитанным",
                widgets::Btn::Primary,
                widgets::BtnSize::Lg,
                Some(Msg::MarkRead),
            )]
            .into()
        };
        root = root.push(
            Row::new()
                .width(Length::Fill)
                .push(Space::new().width(Length::Fill))
                .push(actions)
                .push(Space::new().width(Length::Fill)),
        );

        if let Some(next) = next {
            let p = palette();
            let title = next.title.clone().unwrap_or_default();
            let link = button(
                txt(format!("Следующий урок: {title} →"))
                    .size(theme::text::BODY)
                    .color(p.gold2),
            )
            .padding([6.0, 10.0])
            .style(move |_, s| theme::button_ghost(&p, s))
            .on_press(Msg::Go(Route::Lesson(next.id.clone())));
            root = root.push(
                Row::new()
                    .width(Length::Fill)
                    .push(Space::new().width(Length::Fill))
                    .push(link)
                    .push(Space::new().width(Length::Fill)),
            );
        }

        root.into()
    }

    pub fn subscription(&self) -> Subscription<Msg> {
        Subscription::none()
    }
}

// ---------------------------------------------------------------------------
// Pieces
// ---------------------------------------------------------------------------

fn header<'a>(back_label: &'a str, back: Msg, kicker: &'a str, title: &'a str) -> Element<'a, Msg> {
    column![
        widgets::back_button(back_label, back),
        row![
            widgets::kicker(kicker),
            Space::new().width(Length::Fill),
            widgets::chip::<Msg>("Требует проверки раввином"),
        ]
        .align_y(Alignment::Center),
        widgets::heading(title, 1),
    ]
    .spacing(10)
    .width(Length::Fill)
    .into()
}

fn block_differs(b: &Block) -> &[Differs] {
    match b {
        Block::H { common, .. }
        | Block::P { common, .. }
        | Block::List { common, .. }
        | Block::Hebrew { common, .. }
        | Block::Quote { common, .. }
        | Block::Note { common, .. }
        | Block::Term { common, .. } => &common.differs,
        Block::Unknown => &[],
    }
}

fn block_view<'a>(block: &'a Block, translit: bool) -> Option<Element<'a, Msg>> {
    let p = palette();
    match block {
        Block::H { text, .. } => Some(
            column![
                // 36×2 px gold bar above the heading (§5).
                container(Space::new())
                    .width(36.0)
                    .height(2.0)
                    .style(move |_| iced::widget::container::Style {
                        background: Some(Background::Color(p.gold)),
                        ..Default::default()
                    }),
                widgets::heading(text, 2),
            ]
            .spacing(10)
            .into(),
        ),
        Block::P { text, .. } => Some(widgets::rich_paragraph(text)),
        Block::List { items, ordered, .. } => {
            let mut list = Column::new().spacing(6).width(Length::Fill);
            for (i, item) in items.iter().enumerate() {
                let marker = if *ordered {
                    format!("{}.", i + 1)
                } else {
                    "•".to_string()
                };
                list = list.push(
                    row![
                        txt(marker)
                            .size(theme::text::BODY)
                            .font(SANS_BOLD)
                            .color(p.gold),
                        widgets::rich_paragraph(item),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Start),
                );
            }
            Some(list.into())
        }
        Block::Hebrew {
            he,
            translit: tr,
            ru,
            ref_,
            ..
        } => Some(widgets::hebrew_block(
            he,
            if translit { tr.as_deref() } else { None },
            ru.as_deref(),
            ref_.as_deref(),
        )),
        Block::Quote { text, ref_, .. } => Some(widgets::quote_block(text, ref_.as_deref())),
        Block::Note { text, .. } => Some(widgets::note_block("Совет", text)),
        Block::Term { term, he, text, .. } => Some(widgets::term_block(
            term,
            he.as_deref(),
            text.as_deref().unwrap_or(""),
        )),
        Block::Unknown => None,
    }
}

/// «differs» notes: own movement expanded, the rest under the collapsible
/// «Как в других течениях» (§T.8, js/ui/core.js UI.differs).
fn differs_view<'a>(
    list: &'a [Differs],
    user: Level,
    group: &mut u32,
    open: &HashSet<u32>,
) -> Option<Element<'a, Msg>> {
    if list.is_empty() {
        return None;
    }
    let p = palette();
    let mine: Vec<&Differs> = list.iter().filter(|d| d.level == user).collect();
    let other: Vec<&Differs> = list.iter().filter(|d| d.level != user).collect();

    let mut col = Column::new().spacing(8).width(Length::Fill);
    for d in &mine {
        col = col.push(widgets::differs_note(
            content::level_info(d.level).differs_label,
            &d.text,
        ));
    }
    if !other.is_empty() {
        let id = *group;
        *group += 1;
        let expanded = open.contains(&id);
        let summary = button(
            txt("Как в других течениях")
                .size(theme::text::SMALL)
                .font(SANS_BOLD)
                .color(p.muted),
        )
        .padding([10.0, 12.0])
        .style(move |_, s| theme::button_ghost(&p, s))
        .on_press(Msg::ToggleDiffers(id));
        col = col.push(summary);
        if expanded {
            for d in &other {
                col = col.push(widgets::differs_note(
                    content::level_info(d.level).differs_label,
                    &d.text,
                ));
            }
        }
    }
    Some(col.into())
}

fn movement_short(movement: Option<&str>) -> Option<&'static str> {
    let level = match movement? {
        "reform" => Level::Reform,
        "conservative" => Level::Conservative,
        "orthodox" => Level::Orthodox,
        _ => return None,
    };
    Some(content::level_info(level).short)
}

fn sources_footer<'a>(
    ctx: &'a Ctx,
    sec: &'a SectionMeta,
    lesson: &'a Lesson,
    citations: Option<&'a str>,
) -> Option<Element<'a, Msg>> {
    let srcs = ctx.lib.sources(sec, Some(&lesson.source_refs));
    if srcs.is_empty() && citations.is_none() {
        return None;
    }
    let p = palette();
    let mut col = Column::new().spacing(10).width(Length::Fill);
    col = col.push(
        container(Space::new())
            .height(1.0)
            .width(Length::Fill)
            .style(move |_| theme::fillet(&palette())),
    );
    col = col.push(
        txt("Источники")
            .size(theme::text::H3)
            .font(crate::ui::fonts::SERIF_BOLD)
            .color(p.gold2),
    );
    for s in &srcs {
        let title = s.title.clone().unwrap_or_else(|| s.id.clone());
        let mut item = Row::new().spacing(8).align_y(Alignment::Center);
        match &s.url {
            Some(url) if url.starts_with("http://") || url.starts_with("https://") => {
                let url = url.clone();
                item = item.push(
                    button(
                        row![
                            txt(title).size(theme::text::BODY).color(p.gold2),
                            txt("↗").size(theme::text::BODY).color(p.gold2),
                        ]
                        .spacing(4)
                        .align_y(Alignment::Center),
                    )
                    .padding([4.0, 6.0])
                    .style(move |_, s| theme::button_ghost(&p, s))
                    .on_press(Msg::OpenUrl(url)),
                );
            }
            _ => {
                item = item.push(txt(title).size(theme::text::BODY).color(p.text));
            }
        }
        if let Some(short) = movement_short(s.movement.as_deref()) {
            item = item.push(widgets::tag::<Msg>(short));
        }
        col = col.push(item);
    }
    if let Some(citations) = citations {
        col = col.push(widgets::rich_paragraph(citations));
    }
    Some(col.into())
}

/// Not-found view (§0.52 / §5 guards).
fn not_found_view(guard: &Guard) -> Element<'static, Msg> {
    let p = palette();
    let (title, message) = match guard {
        Guard::NotFound => ("Не найдено", "Урок не найден."),
        Guard::Locked => ("Раздел закрыт", "Этот раздел пока закрыт."),
    };
    column![
        widgets::back_button("Карта", Msg::Go(Route::Map)),
        widgets::heading(title, 1),
        widgets::card(
            column![
                txt("✡")
                    .size(40.0)
                    .color(p.gold)
                    .align_x(Horizontal::Center),
                txt(message)
                    .size(theme::text::BODY)
                    .color(p.muted)
                    .align_x(Horizontal::Center),
                widgets::btn(
                    "На карту",
                    widgets::Btn::Primary,
                    widgets::BtnSize::Lg,
                    Some(Msg::Go(Route::Map)),
                ),
            ]
            .spacing(14)
            .align_x(Alignment::Center)
            .width(Length::Fill)
        ),
    ]
    .spacing(14)
    .width(Length::Fill)
    .into()
}
