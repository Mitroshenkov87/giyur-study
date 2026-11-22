//! Section page (PARITY §4, docs/STRUCTURE.md): emblem header with reading
//! and readiness rings, a «Читать / Готовиться» switch, and two views:
//! - «Читать»: overview, lessons in order (key ideas, beit-din questions),
//!   «Углубиться»;
//! - «Готовиться»: exam readiness with its parts and the next step, cards,
//!   beit-din rehearsal, section test, and per-lesson self-checks.

use iced::widget::{button, column, container, progress_bar, row, stack, Column};
use iced::{Alignment, Background, Border, Element, Length};

use crate::content::{self, Library, SectionMeta};
use crate::srs;
use crate::study::{self, NextStep, Readiness, StudyMode};
use crate::ui::app::{Action, Ctx, Deck, ExamSpec, Route};
use crate::ui::screens::map::{alpha, emblem, fmt_pct, icons, kicker_owned, txt};
use crate::ui::{fonts, theme, widgets};

pub struct Screen {
    id: String,
    mode: StudyMode,
    /// 3D objects whose caption is open.
    captions: Vec<crate::ui::model3d::Model>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Go(Route),
    Mode(StudyMode),
    OpenUrl(String),
    /// Start a lesson self-check (question ids picked on press).
    SelfCheck(String),
    /// Start the section's beit-din rehearsal.
    Rehearse,
    /// A 3D object was drawn by the GPU (rebuild without its placeholder).
    Model3dReady,
    /// Click on a 3D object: show / hide its caption.
    ToggleCaption(crate::ui::model3d::Model),
}

impl Screen {
    pub fn new(_ctx: &mut Ctx, route: &Route) -> Self {
        let id = match route {
            Route::Section(id) => id.clone(),
            _ => String::new(),
        };
        let mode = _ctx
            .lib
            .section(&id)
            .map(|sec| study::mode_for(&_ctx.lib, sec, &_ctx.state))
            .unwrap_or(StudyMode::Read);
        Screen {
            id,
            mode,
            captions: Vec::new(),
        }
    }

    pub fn update(&mut self, ctx: &mut Ctx, msg: Msg) -> Action {
        match msg {
            Msg::Model3dReady => Action::None,
            Msg::ToggleCaption(m) => {
                crate::ui::screens::map::toggle(&mut self.captions, m);
                Action::None
            }
            Msg::Go(route) => Action::Go(route),
            Msg::Mode(mode) => {
                self.mode = mode;
                study::set_mode(&mut ctx.state, &self.id, mode);
                ctx.save();
                Action::None
            }
            Msg::OpenUrl(url) => {
                widgets::open_url(&url);
                Action::None
            }
            Msg::SelfCheck(lesson_id) => {
                let Some(sec) = ctx.lib.section(&self.id) else {
                    return Action::None;
                };
                let ids = study::self_check_ids(&ctx.lib, sec, &lesson_id, &ctx.state);
                if ids.is_empty() {
                    return Action::None;
                }
                Action::Go(Route::ExamRun(ExamSpec::SelfCheck {
                    section_id: self.id.clone(),
                    ids,
                }))
            }
            Msg::Rehearse => {
                let Some(sec) = ctx.lib.section(&self.id) else {
                    return Action::None;
                };
                let ids = study::rehearsal_ids(&ctx.lib, sec, &ctx.state);
                if ids.is_empty() {
                    return Action::None;
                }
                Action::Go(Route::ExamRun(ExamSpec::Rehearsal {
                    section_id: self.id.clone(),
                    ids,
                }))
            }
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

        page = page.push(widgets::back_button("Уроки", Msg::Go(Route::Map)));
        let rd = study::readiness(&ctx.lib, sec, &ctx.state);
        let read_pct = study::read_pct(&rd);

        // ---- Header: emblem · back/kicker/h1/sub · two rings ----
        let head_text = column![
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
                labeled_ring(read_pct, "Прочитано", p.gold2, p),
                labeled_ring(rd.pct, "Готовность", p.gold, p),
            ]
            .spacing(20)
            .align_y(Alignment::Center),
        );

        // ---- «Читать / Готовиться» ----
        page = page.push(mode_switch(self.mode, &rd, p));

        page = match self.mode {
            StudyMode::Read => self.view_read(ctx, sec, &stats, &lessons, user, page, p),
            StudyMode::Prep => self.view_prep(ctx, sec, &rd, &lessons, page, p),
        };
        page.into()
    }

    fn object_card<'a>(
        &'a self,
        ctx: &'a Ctx,
        m: crate::ui::model3d::Model,
        section: &str,
    ) -> Element<'a, Msg> {
        use crate::ui::model3d::card;
        card::card(
            ctx,
            m,
            m.kicker_in(section),
            self.captions.contains(&m),
            card::Msgs {
                ready: Msg::Model3dReady,
                toggle_caption: Msg::ToggleCaption(m),
                link: None,
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn view_read<'a>(
        &'a self,
        ctx: &'a Ctx,
        sec: &'a SectionMeta,
        stats: &content::SectionStats,
        lessons: &[&'a content::Lesson],
        user: crate::storage::Level,
        mut page: Column<'a, Msg>,
        p: theme::Palette,
    ) -> Column<'a, Msg> {
        // ---- Overview (or the plain intro for content without one) ----
        match ctx.lib.overview(sec) {
            Some(ov) => page = page.push(overview_card(ov, p)),
            None => {
                if let Some(intro) = sec
                    .data
                    .as_ref()
                    .and_then(|d| d.intro.as_deref())
                    .filter(|i| !i.is_empty())
                {
                    page = page.push(widgets::rich_paragraph(intro));
                }
            }
        }

        // ---- Ritual objects of the section (live 3D) ----
        for &m in crate::ui::model3d::Model::for_section(&sec.id) {
            page = page.push(self.object_card(ctx, m, &sec.id));
        }

        // ---- Lessons ----
        page = page.push(
            row![
                widgets::heading("Уроки", 3),
                iced::widget::Space::new().width(Length::Fill),
                txt(format!("Прочитано {} из {}", stats.read, stats.lessons))
                    .size(theme::text::SMALL)
                    .color(p.muted),
            ]
            .align_y(Alignment::Center),
        );
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

        // ---- «Углубиться» ----
        let deeper = ctx.lib.deeper(sec, user);
        if !deeper.is_empty() {
            let mut col = Column::new().spacing(12).width(Length::Fill);
            col = col.push(widgets::heading("Углубиться", 3));
            col = col.push(
                txt("Что почитать после уроков, чтобы понять тему глубже.")
                    .size(theme::text::SMALL)
                    .color(p.muted),
            );
            for d in deeper {
                let src = d.source_ref.as_deref().and_then(|id| {
                    sec.data
                        .as_ref()
                        .and_then(|data| data.sources.iter().find(|s| s.id == id))
                });
                let mut item = Column::new().spacing(4).width(Length::Fill);
                item = item.push(
                    txt(d.title.as_str())
                        .size(theme::text::BODY)
                        .font(fonts::SANS_BOLD),
                );
                item = item.push(widgets::rich_paragraph(&d.text));
                if let Some(src) = src {
                    let label = src.title.as_deref().unwrap_or(&src.id);
                    let line = txt(label).size(theme::text::SMALL).color(p.gold2);
                    item = item.push(match src.url.as_deref().filter(|u| !u.is_empty()) {
                        Some(url) => Element::from(
                            button(
                                row![line, icons::icon(icons::Kind::External, 12.0, p.gold2)]
                                    .spacing(6)
                                    .align_y(Alignment::Center),
                            )
                            .padding([2.0, 0.0])
                            .style(move |_, s| theme::button_ghost(&theme::current(), s))
                            .on_press(Msg::OpenUrl(url.to_string())),
                        ),
                        None => line.into(),
                    });
                }
                col = col.push(item);
            }
            page = page.push(widgets::card(col));
        }

        // ---- Hand-off to the prep mode ----
        page = page.push(cta_row(
            "Прочитали? Проверьте себя и готовьтесь к бейт-дину.",
            "Готовиться →",
            Msg::Mode(StudyMode::Prep),
            p,
        ));
        page
    }

    fn view_prep<'a>(
        &'a self,
        ctx: &'a Ctx,
        sec: &'a SectionMeta,
        rd: &Readiness,
        lessons: &[&'a content::Lesson],
        mut page: Column<'a, Msg>,
        p: theme::Palette,
    ) -> Column<'a, Msg> {
        page = page.push(readiness_card(ctx, sec, rd, p));

        // ---- Three ways to practise ----
        let cards_action = action_card(
            icons::Kind::Cards,
            false,
            "Карточки раздела",
            format!("{} из {} выучено", rd.cards_mastered, rd.cards),
            Some(Msg::Go(Route::Cards(Deck::Section(sec.id.clone())))),
            p,
        );
        let rehearse_action = action_card(
            icons::Kind::StarDavid,
            false,
            "Репетиция бейт-дина",
            if rd.interview > 0 {
                format!("Верно {} из {}", rd.interview_known, rd.interview)
            } else {
                "Для вашего уровня вопросов пока нет".to_string()
            },
            (rd.interview > 0).then_some(Msg::Rehearse),
            p,
        );
        let test_sub = match rd.best {
            None => "Ещё не сдавали · порог 80%".to_string(),
            Some(best) => format!("Лучший: {} · порог 80%", fmt_pct(best)),
        };
        let test_action = action_card(
            icons::Kind::Exam,
            true,
            "Тест раздела",
            test_sub,
            Some(Msg::Go(Route::Exam(Some(sec.id.clone())))),
            p,
        );
        page = page.push(
            row![cards_action, rehearse_action, test_action]
                .spacing(12)
                .align_y(Alignment::Center),
        );

        // ---- Per-lesson prep ----
        page = page.push(widgets::heading("По урокам", 3));
        let mut list = Column::new().spacing(8).width(Length::Fill);
        for (i, lesson) in lessons.iter().enumerate() {
            list = list.push(prep_row(ctx, sec, lesson, i + 1, p));
        }
        page = page.push(list);

        page = page.push(cta_row(
            "Что-то подзабылось? Вернитесь к текстам уроков.",
            "← Читать",
            Msg::Mode(StudyMode::Read),
            p,
        ));
        page
    }
}

// ---------------------------------------------------------------------------
// Study pieces (docs/STRUCTURE.md)
// ---------------------------------------------------------------------------

/// Ring with a percentage inside and a caption below.
fn labeled_ring<'a>(
    pct: i64,
    label: &'a str,
    color: iced::Color,
    p: theme::Palette,
) -> Element<'a, Msg> {
    column![
        stack![
            icons::ring(pct as f32 / 100.0, 68.0, color),
            container(txt(format!("{pct}%")).size(14.0).font(fonts::SANS_BOLD))
                .width(68.0)
                .height(68.0)
                .align_x(iced::alignment::Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center),
        ],
        txt(label).size(theme::text::SMALL).color(p.muted),
    ]
    .spacing(4)
    .align_x(Alignment::Center)
    .into()
}

/// Big two-way switch «Читать / Готовиться» with a status line under each.
fn mode_switch<'a>(mode: StudyMode, rd: &Readiness, p: theme::Palette) -> Element<'a, Msg> {
    let seg = |m: StudyMode, icon: icons::Kind, sub: String| -> Element<'a, Msg> {
        let selected = mode == m;
        button(
            row![
                icons::icon(icon, 22.0, if selected { p.gold } else { p.muted }),
                column![
                    txt(m.title())
                        .size(theme::text::BODY + 1.0)
                        .font(fonts::SANS_BOLD)
                        .color(if selected { p.gold2 } else { p.text }),
                    txt(sub).size(theme::text::SMALL).color(p.muted),
                ]
                .spacing(2),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        )
        .width(Length::FillPortion(1))
        .padding([12.0, 16.0])
        .style(move |_, status| {
            let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: Some(Background::Color(if selected {
                    p.gold_soft
                } else if hovered {
                    p.surface2
                } else {
                    p.surface
                })),
                text_color: p.text,
                border: Border {
                    color: if selected { p.gold } else { p.line },
                    width: if selected { 1.5 } else { 1.0 },
                    radius: theme::radius::SEGMENT.into(),
                },
                shadow: iced::Shadow::default(),
                snap: true,
            }
        })
        .on_press(Msg::Mode(m))
        .into()
    };
    row![
        seg(
            StudyMode::Read,
            icons::Kind::Book,
            format!("Понять тему · прочитано {} из {}", rd.read, rd.lessons),
        ),
        seg(
            StudyMode::Prep,
            icons::Kind::Exam,
            format!("К бейт-дину · {}% · {}", rd.pct, rd.band.title()),
        ),
    ]
    .spacing(10)
    .width(Length::Fill)
    .into()
}

fn overview_card<'a>(ov: &'a content::Overview, p: theme::Palette) -> Element<'a, Msg> {
    let mut col = Column::new().spacing(10).width(Length::Fill);
    col = col.push(widgets::kicker("О чём раздел"));
    if !ov.why.is_empty() {
        col = col.push(widgets::rich_paragraph(&ov.why));
    }
    if !ov.goals.is_empty() {
        col = col.push(
            txt("Что вы сможете")
                .size(theme::text::BODY)
                .font(fonts::SANS_BOLD),
        );
        for g in &ov.goals {
            col = col.push(
                row![
                    txt("•")
                        .size(theme::text::BODY)
                        .font(fonts::SANS_BOLD)
                        .color(p.gold),
                    widgets::rich_paragraph(g),
                ]
                .spacing(10),
            );
        }
    }
    if !ov.exam_focus.is_empty() {
        col = col.push(widgets::note_block("На бейт-дине", &ov.exam_focus));
    }
    widgets::card(col)
}

fn bar_line<'a>(label: &'a str, value: String, frac: f64, p: theme::Palette) -> Element<'a, Msg> {
    column![
        row![
            txt(label).size(theme::text::SMALL).font(fonts::SANS_BOLD),
            iced::widget::Space::new().width(Length::Fill),
            txt(value).size(theme::text::SMALL).color(p.muted),
        ],
        progress_bar(0.0..=100.0, (frac * 100.0).clamp(0.0, 100.0) as f32)
            .girth(6.0)
            .style(move |_| theme::progress_bar(&theme::current())),
    ]
    .spacing(4)
    .width(Length::Fill)
    .into()
}

fn frac(a: usize, b: usize) -> f64 {
    if b == 0 {
        0.0
    } else {
        a as f64 / b as f64
    }
}

/// «Готовность к экзамену»: ring, band, the four parts and the next step.
fn readiness_card<'a>(
    ctx: &'a Ctx,
    sec: &'a SectionMeta,
    rd: &Readiness,
    p: theme::Palette,
) -> Element<'a, Msg> {
    let ring = stack![
        icons::ring(rd.pct as f32 / 100.0, 112.0, p.gold),
        container(
            column![
                txt(format!("{}%", rd.pct))
                    .size(24.0)
                    .font(fonts::SANS_BOLD),
                txt("готовность").size(11.0).color(p.muted),
            ]
            .align_x(Alignment::Center)
        )
        .width(112.0)
        .height(112.0)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center),
    ];
    let test_value = match rd.best {
        None => "не сдавали".to_string(),
        Some(b) => format!("{} · порог 80%", fmt_pct(b)),
    };
    // A part with nothing at this level counts as done in the score
    // (study::part), so it is not shown as an empty bar either.
    let mut parts = Column::new();
    if rd.cards > 0 {
        parts = parts.push(bar_line(
            "Карточки",
            format!(
                "{} из {} выучено · прогресс {}%",
                rd.cards_mastered,
                rd.cards,
                (rd.cards_score * 100.0).round() as i64
            ),
            rd.cards_score,
            p,
        ));
    }
    if rd.checks > 0 {
        parts = parts.push(bar_line(
            "Самопроверка по урокам",
            format!("{} из {} верно", rd.checks_known, rd.checks),
            frac(rd.checks_known, rd.checks),
            p,
        ));
    }
    if rd.interview > 0 {
        parts = parts.push(bar_line(
            "Ответы для бейт-дина",
            format!("{} из {} верно", rd.interview_known, rd.interview),
            frac(rd.interview_known, rd.interview),
            p,
        ));
    }
    let parts = parts
        .push(bar_line(
            "Тест раздела",
            test_value,
            rd.best.unwrap_or(0.0) / 100.0,
            p,
        ))
        .spacing(10)
        .width(Length::Fill);

    let (next_text, next_label, next_msg) = next_step(ctx, sec, &rd.next);
    let mut right = Column::new().spacing(12).width(Length::Fill);
    right = right.push(
        row![
            widgets::kicker("Готовность к экзамену"),
            iced::widget::Space::new().width(Length::Fill),
            widgets::pill(rd.band.title(), widgets::Pill::Chip),
        ]
        .align_y(Alignment::Center),
    );
    right = right.push(parts);
    let mut next_row = row![column![
        txt("Следующий шаг").size(theme::text::SMALL).color(p.muted),
        txt(next_text)
            .size(theme::text::BODY)
            .font(fonts::SANS_BOLD),
    ]
    .spacing(2)
    .width(Length::Fill)]
    .spacing(12)
    .align_y(Alignment::Center);
    if let Some(msg) = next_msg {
        next_row = next_row.push(widgets::btn(
            next_label,
            widgets::Btn::Primary,
            widgets::BtnSize::Md,
            Some(msg),
        ));
    }
    right = right.push(next_row);

    widgets::card(row![ring, right].spacing(24).align_y(Alignment::Center))
}

/// Text, button label and action for the next step.
fn next_step<'a>(
    ctx: &'a Ctx,
    sec: &'a SectionMeta,
    next: &NextStep,
) -> (String, &'static str, Option<Msg>) {
    let title = |id: &str| {
        ctx.lib
            .lesson(id)
            .and_then(|(l, _)| l.title.clone())
            .unwrap_or_else(|| id.to_string())
    };
    match next {
        NextStep::Read(id) => (
            format!("Прочитать урок «{}»", title(id)),
            "Читать",
            Some(Msg::Go(Route::Lesson(id.clone()))),
        ),
        NextStep::SelfCheck(id) => (
            format!("Самопроверка: «{}»", title(id)),
            "Начать",
            Some(Msg::SelfCheck(id.clone())),
        ),
        NextStep::Cards => (
            "Повторить карточки раздела".to_string(),
            "Карточки",
            Some(Msg::Go(Route::Cards(Deck::Section(sec.id.clone())))),
        ),
        NextStep::Rehearse => (
            "Отрепетировать ответы для бейт-дина".to_string(),
            "Репетиция",
            Some(Msg::Rehearse),
        ),
        NextStep::Test => (
            "Сдать тест раздела (порог 80%)".to_string(),
            "К тесту",
            Some(Msg::Go(Route::Exam(Some(sec.id.clone())))),
        ),
        NextStep::Done => (
            "Раздел пройден. Повторяйте карточки по расписанию.".to_string(),
            "",
            None,
        ),
    }
}

/// One lesson in the prep list: read state, self-check and beit-din counts.
fn prep_row<'a>(
    ctx: &'a Ctx,
    sec: &'a SectionMeta,
    lesson: &'a content::Lesson,
    number: usize,
    p: theme::Palette,
) -> Element<'a, Msg> {
    let lp = study::lesson_prep(&ctx.lib, sec, &lesson.id, &ctx.state);
    let mut meta = vec![if lp.read {
        "✓ прочитан".to_string()
    } else {
        "не прочитан".to_string()
    }];
    if lp.checks > 0 {
        meta.push(format!("самопроверка {}/{}", lp.checks_known, lp.checks));
    }
    if lp.interview > 0 {
        meta.push(format!("бейт-дин {}/{}", lp.interview_known, lp.interview));
    }
    let text_col = column![
        txt(format!(
            "{number}. {}",
            lesson.title.as_deref().unwrap_or(&lesson.id)
        ))
        .size(theme::text::BODY)
        .font(fonts::SANS_BOLD),
        txt(meta.join(" · "))
            .size(theme::text::SMALL)
            .color(p.muted),
        progress_bar(0.0..=100.0, (lp.ratio() * 100.0) as f32)
            .girth(4.0)
            .style(move |_| theme::progress_bar(&theme::current())),
    ]
    .spacing(4)
    .width(Length::Fill);
    let read_btn = widgets::btn(
        "Читать",
        widgets::Btn::Ghost,
        widgets::BtnSize::Sm,
        Some(Msg::Go(Route::Lesson(lesson.id.clone()))),
    );
    let check_btn = widgets::btn(
        "Самопроверка",
        widgets::Btn::Secondary,
        widgets::BtnSize::Sm,
        (lp.checks > 0).then(|| Msg::SelfCheck(lesson.id.clone())),
    );
    container(
        row![text_col, read_btn, check_btn]
            .spacing(10)
            .align_y(Alignment::Center),
    )
    .padding([10.0, 12.0])
    .width(Length::Fill)
    .style(move |_| iced::widget::container::Style {
        background: Some(Background::Color(p.surface)),
        border: Border {
            color: p.line,
            width: 1.0,
            radius: theme::radius::SMALL.into(),
        },
        ..iced::widget::container::Style::default()
    })
    .into()
}

/// Centred hand-off line between the two modes.
fn cta_row<'a>(text_: &'a str, label: &'a str, msg: Msg, p: theme::Palette) -> Element<'a, Msg> {
    container(
        row![
            txt(text_).size(theme::text::BODY).color(p.muted),
            widgets::btn(
                label,
                widgets::Btn::Secondary,
                widgets::BtnSize::Md,
                Some(msg)
            ),
        ]
        .spacing(16)
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .align_x(iced::alignment::Horizontal::Center)
    .padding([8.0, 0.0])
    .into()
}

fn action_card<'a>(
    kind: icons::Kind,
    gold: bool,
    title: &'a str,
    sub: String,
    msg: Option<Msg>,
    p: theme::Palette,
) -> Element<'a, Msg> {
    let b = button(
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
            text_color: if matches!(status, button::Status::Disabled) {
                p.muted
            } else {
                p.text
            },
            border: Border {
                color: border,
                width: 1.0,
                radius: theme::radius::CARD.into(),
            },
            shadow: iced::Shadow::default(),
            snap: true,
        }
    })
    .on_press_maybe(msg);
    b.into()
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
            background: Some(Background::Color(if read {
                p.brass_mid
            } else {
                p.surface3
            })),
            border: Border {
                color: if read { p.brass_edge } else { p.line2 },
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
            txt({
                let bd = ctx.lib.lesson_interview(sec, &lesson.id, user).len();
                let mut m = format!("{minutes} мин · карточки {mastered}/{total}");
                if !lesson.key_ideas.is_empty() {
                    m.push_str(&format!(" · главное: {}", lesson.key_ideas.len()));
                }
                if bd > 0 {
                    m.push_str(&format!(" · вопросы бейт-дина: {bd}"));
                }
                m
            })
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
        widgets::back_button("Уроки", Msg::Go(Route::Map)),
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
                widgets::btn(
                    "К урокам",
                    widgets::Btn::Primary,
                    widgets::BtnSize::Lg,
                    Some(Msg::Go(Route::Map)),
                ),
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
