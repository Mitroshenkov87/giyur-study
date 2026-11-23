//! Lesson reader (PARITY §5, js/ui/lesson.js).

use std::collections::HashSet;

use iced::alignment::Horizontal;
use iced::widget::text::{Shaping, Text};
use iced::widget::{button, column, container, row, text, Column, Row, Space};
use iced::{Alignment, Background, Element, Length, Subscription};

use crate::content::{self, Block, Differs, Lesson, SectionMeta};
use crate::game;
use crate::storage::Level;
use crate::study;
use crate::ui::app::{Action, Ctx, Deck, ExamSpec, Route};
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
    /// Beit-din questions whose model answer is shown.
    revealed: HashSet<String>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Go(Route),
    MarkRead,
    ToggleDiffers(u32),
    OpenUrl(String),
    /// Show / hide the model answer of a beit-din question.
    Reveal(String),
    /// «Самопроверка» of this lesson.
    SelfCheck,
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
                // «Открытый режим рецензента»: any lesson, at any level.
                if !content::Library::visible(lesson.min_level, None, user)
                    && !crate::reviewer::active(&ctx.state)
                {
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
            revealed: HashSet::new(),
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
                ctx.sync_collection();
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
            Msg::Reveal(id) => {
                if !self.revealed.remove(&id) {
                    self.revealed.insert(id);
                }
                Action::None
            }
            Msg::SelfCheck => {
                let Some((_, sec)) = ctx.lib.lesson(&self.lesson_id) else {
                    return Action::None;
                };
                let ids = study::self_check_ids(&ctx.lib, sec, &self.lesson_id, &ctx.state);
                if ids.is_empty() {
                    return Action::None;
                }
                Action::Go(Route::ExamRun(ExamSpec::SelfCheck {
                    section_id: sec.id.clone(),
                    ids,
                }))
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

        // ---- «Главное» (docs/STRUCTURE.md): the lesson in 3–5 lines ----
        if !lesson.key_ideas.is_empty() {
            root = root.push(key_ideas_card(&lesson.key_ideas));
        }

        // ---- «Что спрашивают на бейт-дине» ----
        let interview = ctx.lib.lesson_interview(sec, &lesson.id, user);
        if !interview.is_empty() {
            root = root.push(interview_card(&interview, user, &self.revealed));
        }

        // Actions (centred): mark read, or read mark + cards button.
        let has_check = !study::self_check_ids(&ctx.lib, sec, &lesson.id, &ctx.state).is_empty();
        let actions: Element<Msg> = if read {
            let p = palette();
            let mut r = row![txt("✓ Прочитано")
                .size(theme::text::BODY)
                .font(SANS_BOLD)
                .color(p.ok),]
            .spacing(16)
            .align_y(Alignment::Center);
            if has_check {
                r = r.push(widgets::btn(
                    "Самопроверка",
                    widgets::Btn::Primary,
                    widgets::BtnSize::Lg,
                    Some(Msg::SelfCheck),
                ));
            }
            r = r.push(widgets::btn(
                "К карточкам урока",
                if has_check {
                    widgets::Btn::Secondary
                } else {
                    widgets::Btn::Primary
                },
                widgets::BtnSize::Lg,
                Some(Msg::Go(Route::Cards(Deck::Lesson(lesson.id.clone())))),
            ));
            r.into()
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

/// «Главное»: key ideas of the lesson as a gold-marked list.
fn key_ideas_card<'a>(ideas: &'a [String]) -> Element<'a, Msg> {
    let p = palette();
    let mut col = Column::new().spacing(8).width(Length::Fill);
    col = col.push(widgets::kicker("Главное"));
    for (i, idea) in ideas.iter().enumerate() {
        col = col.push(
            row![
                txt(format!("{}", i + 1))
                    .size(theme::text::BODY)
                    .font(SANS_BOLD)
                    .color(p.gold),
                widgets::rich_paragraph(idea),
            ]
            .spacing(10)
            .align_y(Alignment::Start),
        );
    }
    widgets::card(col)
}

/// «Что спрашивают на бейт-дине»: the lesson's interview questions; the
/// model answer and key points open on demand, so the reader can answer
/// in their own words first.
fn interview_card<'a>(
    qs: &[&'a content::Question],
    user: Level,
    revealed: &HashSet<String>,
) -> Element<'a, Msg> {
    let p = palette();
    let mut col = Column::new().spacing(12).width(Length::Fill);
    col = col.push(widgets::kicker("Что спрашивают на бейт-дине"));
    col = col.push(
        txt("Сначала ответьте вслух своими словами, потом сверьтесь с образцом.")
            .size(theme::text::SMALL)
            .color(p.muted),
    );
    for q in qs {
        let content::Question::Open {
            common,
            model_answer,
            key_points,
        } = q
        else {
            continue;
        };
        let open = revealed.contains(&common.id);
        let mut item = Column::new().spacing(8).width(Length::Fill);
        item = item.push(
            row![
                txt("?")
                    .size(theme::text::BODY)
                    .font(SANS_BOLD)
                    .color(p.gold),
                widgets::rich_paragraph(&common.prompt),
            ]
            .spacing(10)
            .align_y(Alignment::Start),
        );
        item = item.push(widgets::btn(
            if open {
                "Скрыть образец ответа"
            } else {
                "Показать образец ответа"
            },
            widgets::Btn::Ghost,
            widgets::BtnSize::Sm,
            Some(Msg::Reveal(common.id.clone())),
        ));
        if open {
            item = item.push(widgets::note_block("Образец ответа", model_answer));
            if !key_points.is_empty() {
                let mut kp = Column::new().spacing(4);
                kp = kp.push(
                    txt("Ключевые пункты")
                        .size(theme::text::SMALL)
                        .font(SANS_BOLD),
                );
                for k in key_points {
                    kp = kp.push(
                        row![
                            txt("✓").size(theme::text::SMALL).color(p.ok),
                            widgets::rich_paragraph(k),
                        ]
                        .spacing(8),
                    );
                }
                item = item.push(kp);
            }
            for d in common.differs.iter().filter(|d| d.level == user) {
                item = item.push(widgets::differs_note(
                    content::level_info(d.level).differs_label,
                    &d.text,
                ));
            }
        }
        col = col.push(
            container(item)
                .padding([10.0, 12.0])
                .width(Length::Fill)
                .style(move |_| iced::widget::container::Style {
                    background: Some(Background::Color(p.surface2)),
                    border: iced::Border {
                        color: p.line,
                        width: 1.0,
                        radius: theme::radius::SMALL.into(),
                    },
                    ..Default::default()
                }),
        );
    }
    widgets::card(col)
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
        widgets::back_button("Уроки", Msg::Go(Route::Map)),
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
                    "К урокам",
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

#[cfg(test)]
mod tests {
    use super::*;
    use iced::advanced::renderer::Headless;
    use iced::advanced::widget::operation::Scrollable;
    use iced::advanced::widget::{Id, Operation};
    use iced::{Font, Pixels, Rectangle, Size, Vector};
    use iced_runtime::user_interface::{Cache, UserInterface};

    /// Collects the page scrollable's content size and every text's bounds.
    #[derive(Default)]
    struct Probe {
        content: Option<Rectangle>,
        texts: Vec<(String, Rectangle)>,
        boxes: Vec<Rectangle>,
    }

    impl Operation for Probe {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
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
            self.content.get_or_insert(content_bounds);
        }
        fn text(&mut self, _id: Option<&Id>, bounds: Rectangle, text: &str) {
            self.texts.push((text.to_owned(), bounds));
        }
        fn container(&mut self, _id: Option<&Id>, bounds: Rectangle) {
            self.boxes.push(bounds);
        }
    }

    fn test_ctx(level: Level) -> Ctx {
        let mut state = crate::storage::State::default();
        state.settings.level = Some(level);
        state.settings.unlock_all = true;
        Ctx {
            lib: content::Library::current(),
            state,
            path: std::env::temp_dir().join("shorashim-lesson-test.json"),
            toasts: Vec::new(),
            today_override: None,
            test_timer: None,
            writable: false,
            music_title: None,
            music_available: false,
            music_error: None,
            music_track_count: 0,
            sandbox: None,
        }
    }

    /// Every lesson in every section, at every level and with all «у других
    /// течений» notes expanded, must lay out as a real page inside the
    /// shell's vertical scrollable: no Fill-height block may collapse it
    /// (shabbat-1 used to open blank: its quote's accent bar is Fill-height,
    /// and the shell clipped such a page to 0 px — see `layout::page`).
    #[test]
    fn every_lesson_lays_out_non_empty() {
        {
            let mut fs = iced::advanced::graphics::text::font_system()
                .write()
                .expect("font system lock");
            for f in crate::ui::fonts::all() {
                fs.load_font(std::borrow::Cow::Borrowed(f));
            }
        }
        let mut renderer = iced::futures::executor::block_on(<iced::Renderer as Headless>::new(
            Font::with_name("Noto Sans"),
            Pixels(16.0),
            Some("tiny-skia"),
        ))
        .expect("headless tiny-skia renderer");
        let viewport = Size::new(1100.0, 500.0);
        let mut checked = 0;
        for level in [Level::Reform, Level::Conservative, Level::Orthodox] {
            let mut ctx = test_ctx(level);
            let ids: Vec<String> = ctx
                .lib
                .sections
                .iter()
                .filter_map(|s| s.data.as_ref())
                .flat_map(|d| d.lessons.iter())
                .filter(|l| content::Library::visible(l.min_level, None, level))
                .map(|l| l.id.clone())
                .collect();
            assert!(!ids.is_empty());
            for id in ids {
                let mut screen = Screen::new(&mut ctx, &Route::Lesson(id.clone()));
                assert_eq!(screen.guard, None, "{id}: guarded at {level:?}");
                screen.open_others = (0..256).collect();
                // The shell's reading column (src/main.rs `grid`).
                let g = crate::ui::layout::compute(1600.0, true);
                let page = crate::ui::layout::page(screen.view(&ctx), g.content, g.right);
                let page: Element<Msg> = iced::widget::scrollable(page)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into();
                let mut ui = UserInterface::build(page, viewport, Cache::default(), &mut renderer);
                let mut probe = Probe::default();
                ui.operate(&renderer, &mut probe);

                let content = probe.content.expect("page scrollable");
                let finite =
                    |r: &Rectangle| [r.x, r.y, r.width, r.height].iter().all(|v| v.is_finite());
                assert!(
                    finite(&content)
                        && probe.texts.iter().all(|(_, b)| finite(b))
                        && probe.boxes.iter().all(finite),
                    "{id} at {level:?}: non-finite layout"
                );
                assert!(
                    content.height > viewport.height,
                    "{id} at {level:?}: page is {}px tall — layout collapsed",
                    content.height
                );
                if let Some((t, b)) = probe
                    .texts
                    .iter()
                    .find(|(t, b)| !t.trim().is_empty() && b.height <= 0.0)
                {
                    panic!("{id} at {level:?}: text {t:?} squashed to {b:?}");
                }
                let title = ctx.lib.lesson(&id).and_then(|(l, _)| l.title.clone());
                let title = title.unwrap_or_default();
                assert!(
                    probe
                        .texts
                        .iter()
                        .any(|(t, b)| *t == title && b.width > 0.0 && b.height > 0.0),
                    "{id} at {level:?}: title not laid out"
                );
                let bottom = probe
                    .texts
                    .iter()
                    .map(|(_, b)| b.y + b.height)
                    .fold(0.0, f32::max);
                assert!(
                    bottom > viewport.height,
                    "{id} at {level:?}: text ends at {bottom}px"
                );
                checked += 1;
            }
        }
        assert!(checked > 30, "only {checked} lessons checked");
    }
}
