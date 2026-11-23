//! «Тренажёр собеседования с бейт-дином» (map banner; route `interview`).
//!
//! Three tabs over the bank in `crate::interview`:
//! - «Темы»: questions by topic; each opens to what the rabbis check, the
//!   points of an honest answer, common mistakes and links to lessons;
//! - «Практика»: one question at a time (weak ones come back more often),
//!   think or write the answer, reveal the guidance, rate yourself;
//! - «Репетиция»: a mock interview of ten questions in a row and a summary.
//!
//! All rules live in `crate::interview`; this screen owns only view state.

use iced::widget::text::Shaping;
use iced::widget::{button, column, container, row, text, text_editor, Column, Row, Text};
use iced::{Alignment, Background, Border, Color, Element, Length};

use crate::interview::{self, Item, Mode, Rating, Run};
use crate::ui::app::{Action, Ctx, Route};
use crate::ui::fonts::{SANS, SANS_BOLD, SERIF_BOLD};
use crate::ui::screens::map::icons;
use crate::ui::theme::{self, Palette};
use crate::ui::widgets;

fn txt<'a>(content: impl text::IntoFragment<'a>) -> Text<'a> {
    text(content).shaping(Shaping::Advanced)
}

fn alpha(c: Color, a: f32) -> Color {
    Color { a, ..c }
}

fn tinted<'a, M: 'a>(inner: impl Into<Element<'a, M>>, bg: Color, edge: Color) -> Element<'a, M> {
    container(inner)
        .padding([14.0, 16.0])
        .width(Length::Fill)
        .style(move |_| container::Style {
            background: Some(Background::Color(bg)),
            border: Border {
                color: edge,
                width: 1.0,
                radius: theme::radius::SMALL.into(),
            },
            ..container::Style::default()
        })
        .into()
}

/// One question being answered (practice and rehearsal).
struct Answering {
    id: String,
    notes: text_editor::Content,
    revealed: bool,
    rated: Option<Rating>,
}

impl Answering {
    fn new(id: String) -> Self {
        Answering {
            id,
            notes: text_editor::Content::new(),
            revealed: false,
            rated: None,
        }
    }
}

struct Rehearsal {
    ids: Vec<String>,
    ratings: Vec<Rating>,
    cur: Answering,
    /// Saved to the history (the summary is showing).
    done: bool,
}

pub struct Screen {
    mode: Mode,
    /// «Темы» and «Практика» topic filter (None = all topics).
    topic: Option<String>,
    /// Questions opened in «Темы».
    open: Vec<String>,
    practice: Option<Answering>,
    rehearsal: Option<Rehearsal>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Tab(Mode),
    Topic(Option<String>),
    Toggle(String),
    /// «Отработать»: practise this question now.
    PracticeOne(String),
    Edit(text_editor::Action),
    Reveal,
    Rate(Rating),
    /// «Следующий вопрос» (practice) / «Дальше» (rehearsal).
    Next,
    /// «Новая репетиция».
    Restart,
    Lesson(String),
    Go(Route),
}

fn today(ctx: &Ctx) -> chrono::NaiveDate {
    ctx.today()
}

impl Screen {
    pub fn new(ctx: &mut Ctx, route: &Route) -> Self {
        let mode = match route {
            Route::Interview(m) => *m,
            _ => Mode::Browse,
        };
        let mut s = Screen {
            mode,
            topic: None,
            open: Vec::new(),
            practice: None,
            rehearsal: None,
        };
        s.enter(ctx, mode);
        s
    }

    fn enter(&mut self, ctx: &Ctx, mode: Mode) {
        self.mode = mode;
        match mode {
            Mode::Browse => {}
            Mode::Practice => {
                if self.practice.as_ref().is_none_or(|a| a.rated.is_some()) {
                    self.next_practice(ctx);
                }
            }
            Mode::Rehearsal => {
                if self.rehearsal.as_ref().is_none_or(|r| r.done) {
                    self.new_rehearsal(ctx);
                }
            }
        }
    }

    fn next_practice(&mut self, ctx: &Ctx) {
        let avoid = self.practice.as_ref().map(|a| a.id.clone());
        let id = interview::pick(
            &ctx.state,
            self.topic.as_deref(),
            avoid.as_deref(),
            today(ctx),
            &mut rand::thread_rng(),
        );
        self.practice = id.map(Answering::new);
    }

    fn new_rehearsal(&mut self, ctx: &Ctx) {
        let ids = interview::rehearsal(&ctx.state, today(ctx), &mut rand::thread_rng());
        self.rehearsal = ids.first().cloned().map(|first| Rehearsal {
            cur: Answering::new(first),
            ids,
            ratings: Vec::new(),
            done: false,
        });
    }

    fn current_mut(&mut self) -> Option<&mut Answering> {
        match self.mode {
            Mode::Practice => self.practice.as_mut(),
            Mode::Rehearsal => self
                .rehearsal
                .as_mut()
                .filter(|r| !r.done)
                .map(|r| &mut r.cur),
            Mode::Browse => None,
        }
    }

    pub fn update(&mut self, ctx: &mut Ctx, msg: Msg) -> Action {
        match msg {
            Msg::Tab(m) => self.enter(ctx, m),
            Msg::Topic(t) => {
                self.topic = t;
                if self.mode == Mode::Practice {
                    self.practice = None;
                    self.next_practice(ctx);
                }
            }
            Msg::Toggle(id) => {
                if let Some(i) = self.open.iter().position(|x| *x == id) {
                    self.open.remove(i);
                } else {
                    self.open.push(id);
                }
            }
            Msg::PracticeOne(id) => {
                self.mode = Mode::Practice;
                self.practice = Some(Answering::new(id));
            }
            Msg::Edit(a) => {
                if let Some(cur) = self.current_mut() {
                    if !cur.revealed {
                        cur.notes.perform(a);
                    }
                }
            }
            Msg::Reveal => {
                if let Some(cur) = self.current_mut() {
                    cur.revealed = true;
                }
            }
            Msg::Rate(r) => {
                let day = today(ctx);
                let Some(cur) = self.current_mut() else {
                    return Action::None;
                };
                if !cur.revealed || cur.rated.is_some() {
                    return Action::None;
                }
                cur.rated = Some(r);
                let id = cur.id.clone();
                interview::rate(&mut ctx.state, &id, r, day);
                if r == Rating::Ready {
                    ctx.award(interview::XP_READY);
                }
                if self.mode == Mode::Rehearsal {
                    if let Some(reh) = self.rehearsal.as_mut() {
                        reh.ratings.push(r);
                    }
                }
                ctx.save();
            }
            Msg::Next => match self.mode {
                Mode::Practice => {
                    if self.practice.as_ref().is_some_and(|a| a.rated.is_some()) {
                        self.next_practice(ctx);
                    }
                }
                Mode::Rehearsal => {
                    let day = ctx.today_key();
                    let mut finished = None;
                    if let Some(reh) = self.rehearsal.as_mut() {
                        if reh.cur.rated.is_some() && !reh.done {
                            let i = reh.ratings.len();
                            if let Some(id) = reh.ids.get(i) {
                                reh.cur = Answering::new(id.clone());
                            } else {
                                reh.done = true;
                                finished = Some(Run {
                                    day,
                                    ids: reh.ids.clone(),
                                    ratings: reh.ratings.clone(),
                                });
                            }
                        }
                    }
                    if let Some(run) = finished {
                        interview::finish_run(&mut ctx.state, run);
                        ctx.save();
                    }
                }
                Mode::Browse => {}
            },
            Msg::Restart => {
                self.mode = Mode::Rehearsal;
                self.new_rehearsal(ctx);
            }
            Msg::Lesson(id) => return Action::Go(Route::Lesson(id)),
            Msg::Go(r) => return Action::Go(r),
        }
        Action::None
    }

    pub fn subscription(&self) -> iced::Subscription<Msg> {
        iced::Subscription::none()
    }

    pub fn view<'a>(&'a self, ctx: &'a Ctx) -> Element<'a, Msg> {
        let p = theme::current();
        let bank = interview::bank();
        let st = interview::stats(&ctx.state, today(ctx));

        let mut page = Column::new().spacing(16).width(Length::Fill);
        page = page.push(widgets::back_button("Уроки", Msg::Go(Route::Map)));
        page = page.push(
            column![
                widgets::kicker("Подготовка к бейт-дину"),
                widgets::heading(&bank.title, 1),
            ]
            .spacing(6),
        );
        if self.mode == Mode::Browse {
            page = page.push(
                txt(bank.intro.as_str())
                    .size(theme::text::BODY)
                    .color(p.muted)
                    .width(Length::Fill),
            );
        }
        page = page.push(
            row![
                widgets::pill(
                    format!("Отработано {} из {}", st.practiced, st.total),
                    widgets::Pill::Tag
                ),
                widgets::pill(format!("Готов: {}", st.ready), widgets::Pill::Tag),
                widgets::pill(format!("Репетиций: {}", st.runs), widgets::Pill::Tag),
            ]
            .spacing(8)
            .wrap(),
        );
        let tabs = [Mode::Browse, Mode::Practice, Mode::Rehearsal];
        page = page.push(widgets::segmented(
            vec![
                ("Темы".to_string(), Msg::Tab(Mode::Browse)),
                ("Практика".to_string(), Msg::Tab(Mode::Practice)),
                (
                    format!("Репетиция · {} вопросов", interview::REHEARSAL_LEN),
                    Msg::Tab(Mode::Rehearsal),
                ),
            ],
            tabs.iter().position(|m| *m == self.mode),
        ));

        page = match self.mode {
            Mode::Browse => page.push(self.browse(ctx, p)),
            Mode::Practice => page.push(self.practice_view(ctx, p)),
            Mode::Rehearsal => page.push(self.rehearsal_view(ctx, p)),
        };
        page.into()
    }

    fn topic_chips<'a>(&'a self) -> Element<'a, Msg> {
        let bank = interview::bank();
        let mut chips = Row::new().spacing(8);
        chips = chips.push(chip("Все темы", self.topic.is_none(), Msg::Topic(None)));
        for t in &bank.topics {
            chips = chips.push(chip(
                t.title.as_str(),
                self.topic.as_deref() == Some(t.id.as_str()),
                Msg::Topic(Some(t.id.clone())),
            ));
        }
        chips.wrap().vertical_spacing(8).into()
    }

    // ---- «Темы» --------------------------------------------------------

    fn browse<'a>(&'a self, ctx: &'a Ctx, p: Palette) -> Element<'a, Msg> {
        let bank = interview::bank();
        let mut col = Column::new().spacing(16).width(Length::Fill);
        col = col.push(self.topic_chips());
        for t in &bank.topics {
            if self.topic.as_deref().is_some_and(|x| x != t.id) {
                continue;
            }
            let (ready, n) = interview::topic_ready(&ctx.state, &t.id);
            let mut list = Column::new().spacing(8).width(Length::Fill);
            list = list.push(
                row![
                    column![
                        widgets::heading(&t.title, 3),
                        txt(t.blurb.as_str())
                            .size(theme::text::SMALL)
                            .color(p.muted),
                    ]
                    .spacing(2)
                    .width(Length::Fill),
                    widgets::pill(format!("готов {ready} из {n}"), widgets::Pill::Tag),
                ]
                .align_y(Alignment::Center)
                .spacing(10),
            );
            for q in bank.in_topic(&t.id) {
                let opened = self.open.contains(&q.id);
                list = list.push(question_row(q, opened, p));
                if opened {
                    list = list.push(
                        container(
                            column![
                                guidance(ctx, q, p),
                                row![widgets::btn(
                                    "Отработать этот вопрос",
                                    widgets::Btn::Secondary,
                                    widgets::BtnSize::Sm,
                                    Some(Msg::PracticeOne(q.id.clone())),
                                )],
                            ]
                            .spacing(12),
                        )
                        .padding(iced::Padding {
                            top: 4.0,
                            right: 0.0,
                            bottom: 10.0,
                            left: 34.0,
                        }),
                    );
                }
            }
            col = col.push(widgets::card(list));
        }
        col.into()
    }

    // ---- «Практика» ----------------------------------------------------

    fn practice_view<'a>(&'a self, ctx: &'a Ctx, p: Palette) -> Element<'a, Msg> {
        let mut col = Column::new().spacing(16).width(Length::Fill);
        col = col.push(self.topic_chips());
        let Some(a) = &self.practice else {
            return col
                .push(widgets::card(
                    txt("В этой теме пока нет вопросов.").color(p.muted),
                ))
                .into();
        };
        let Some(q) = interview::bank().item(&a.id) else {
            return col.into();
        };
        let mut body = answering(ctx, q, a, p, None);
        if a.rated.is_some() {
            body = body.push(
                row![widgets::btn(
                    "Следующий вопрос",
                    widgets::Btn::Primary,
                    widgets::BtnSize::Md,
                    Some(Msg::Next),
                )]
                .spacing(10),
            );
        }
        col.push(widgets::card(body)).into()
    }

    // ---- «Репетиция» ---------------------------------------------------

    fn rehearsal_view<'a>(&'a self, ctx: &'a Ctx, p: Palette) -> Element<'a, Msg> {
        let Some(r) = &self.rehearsal else {
            return widgets::card(txt("Нет вопросов для репетиции.").color(p.muted));
        };
        if r.done {
            return summary(ctx, r, p);
        }
        let Some(q) = interview::bank().item(&r.cur.id) else {
            return column![].into();
        };
        let no = r.ratings.len() + usize::from(r.cur.rated.is_none());
        let no = no.max(1);
        let mut body = answering(ctx, q, &r.cur, p, Some((no, r.ids.len())));
        if r.cur.rated.is_some() {
            let last = r.ratings.len() >= r.ids.len();
            body = body.push(row![widgets::btn(
                if last {
                    "Итоги репетиции"
                } else {
                    "Дальше"
                },
                widgets::Btn::Primary,
                widgets::BtnSize::Md,
                Some(Msg::Next),
            )]);
        }
        column![
            widgets::step_bar(r.ratings.len(), r.ids.len()),
            widgets::card(body)
        ]
        .spacing(12)
        .into()
    }
}

/// A topic chip as wide as its label (the shared `widgets::choice` fills
/// the row, which would stack nine topics one per line).
fn chip<'a>(label: &'a str, selected: bool, msg: Msg) -> Element<'a, Msg> {
    button(
        container(txt(label).size(theme::text::SMALL).font(if selected {
            SANS_BOLD
        } else {
            SANS
        }))
        .center_y(Length::Fill),
    )
    .height(theme::size::BTN_SM)
    .padding([0.0, 16.0])
    .style(move |_, status| theme::choice(&theme::current(), selected, true, status))
    .on_press(msg)
    .into()
}

fn question_row<'a>(q: &'a Item, opened: bool, p: Palette) -> Element<'a, Msg> {
    button(
        row![
            icons::icon(
                if opened {
                    icons::Kind::ChevronDown
                } else {
                    icons::Kind::Speech
                },
                18.0,
                p.gold2,
            ),
            txt(q.question.as_str())
                .size(theme::text::BODY)
                .color(p.text)
                .width(Length::Fill),
        ]
        .spacing(12)
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding([8.0, 8.0])
    .style(move |_, s| theme::button_ghost(&theme::current(), s))
    .on_press(Msg::Toggle(q.id.clone()))
    .into()
}

fn bullets<'a>(items: &'a [String], mark: &'static str, color: Color) -> Element<'a, Msg> {
    let p = theme::current();
    let mut col = Column::new().spacing(6).width(Length::Fill);
    for it in items {
        col = col.push(
            row![
                txt(mark).size(theme::text::BODY).color(color),
                txt(it.as_str())
                    .size(theme::text::BODY)
                    .color(p.text)
                    .width(Length::Fill),
            ]
            .spacing(8),
        );
    }
    col.into()
}

fn label<'a>(s: &str, color: Color) -> Text<'a> {
    txt(s.to_uppercase())
        .size(theme::text::KICKER)
        .font(SANS_BOLD)
        .color(color)
}

/// What the rabbis check, the points of an honest answer, common mistakes,
/// how movements differ, and links to the lessons.
fn guidance<'a>(ctx: &'a Ctx, q: &'a Item, p: Palette) -> Element<'a, Msg> {
    let mut col = Column::new().spacing(12).width(Length::Fill);
    col = col.push(tinted(
        column![
            label("Что хотят услышать раввины", p.gold2),
            txt(q.purpose.as_str())
                .size(theme::text::BODY)
                .color(p.text),
        ]
        .spacing(6),
        p.gold_soft,
        alpha(p.gold, 0.3),
    ));
    col = col.push(
        column![
            label("О чём сказать честно, своими словами", p.ok),
            bullets(&q.key_points, "•", p.ok),
        ]
        .spacing(6),
    );
    col = col.push(
        column![
            label("Частые ошибки", p.bad),
            bullets(&q.mistakes, "×", p.bad),
        ]
        .spacing(6),
    );
    if let Some(m) = &q.movements {
        col = col.push(widgets::note_block("Различия течений", m.as_str()));
    }
    let mut links = Row::new().spacing(8);
    for id in &q.lessons {
        let Some((lesson, sec)) = ctx.lib.lesson(id) else {
            continue;
        };
        links = links.push(widgets::btn(
            format!("{} · {}", sec.title, lesson.title.as_deref().unwrap_or(id)),
            widgets::Btn::Ghost,
            widgets::BtnSize::Sm,
            Some(Msg::Lesson(id.clone())),
        ));
    }
    col = col.push(
        column![
            label("Уроки по теме", p.muted),
            links.wrap().vertical_spacing(6)
        ]
        .spacing(6),
    );
    col.into()
}

/// A question to answer: prompt, notes, «Показать подсказки», guidance,
/// self-rating. `step` = (number, of) in a rehearsal.
fn answering<'a>(
    ctx: &'a Ctx,
    q: &'a Item,
    a: &'a Answering,
    p: Palette,
    step: Option<(usize, usize)>,
) -> Column<'a, Msg> {
    let bank = interview::bank();
    let topic = bank.topic(&q.topic).map(|t| t.title.as_str()).unwrap_or("");
    let meta = match step {
        Some((n, of)) => format!("Вопрос {n} из {of} · {topic}"),
        None => topic.to_string(),
    };
    let mut col = Column::new().spacing(14).width(Length::Fill);
    col = col.push(label(&meta, p.gold));
    col = col.push(
        txt(q.question.as_str())
            .font(SERIF_BOLD)
            .size(theme::text::H3 + 2.0)
            .color(p.text),
    );
    if !a.revealed {
        col = col.push(
            txt(bank.sincerity.as_str())
                .size(theme::text::SMALL)
                .color(p.muted),
        );
        col = col.push(
            text_editor(&a.notes)
                .placeholder("Ответьте вслух или запишите ответ здесь (необязательно)")
                .on_action(Msg::Edit)
                .height(Length::Fixed(4.0 * 26.0))
                .padding(12.0)
                .size(theme::text::BODY),
        );
        col = col.push(row![widgets::btn(
            "Показать подсказки",
            widgets::Btn::Primary,
            widgets::BtnSize::Md,
            Some(Msg::Reveal),
        )]);
        return col;
    }
    let written = a.notes.text();
    if !written.trim().is_empty() {
        col = col.push(
            column![
                txt("Ваш ответ").size(theme::text::SMALL).color(p.muted),
                txt(written).size(theme::text::BODY).color(p.text),
            ]
            .spacing(4),
        );
    }
    col = col.push(guidance(ctx, q, p));
    match a.rated {
        None => {
            col = col.push(
                txt("Как вы ответили? Оцените честно: слабые вопросы будут возвращаться чаще.")
                    .size(theme::text::SMALL)
                    .color(p.text),
            );
            col = col.push(
                row![
                    widgets::btn(
                        Rating::Ready.label(),
                        widgets::Btn::Primary,
                        widgets::BtnSize::Md,
                        Some(Msg::Rate(Rating::Ready)),
                    ),
                    widgets::btn(
                        Rating::Partly.label(),
                        widgets::Btn::Secondary,
                        widgets::BtnSize::Md,
                        Some(Msg::Rate(Rating::Partly)),
                    ),
                    widgets::btn(
                        Rating::NotYet.label(),
                        widgets::Btn::Ghost,
                        widgets::BtnSize::Md,
                        Some(Msg::Rate(Rating::NotYet)),
                    ),
                ]
                .spacing(10)
                .wrap()
                .vertical_spacing(8),
            );
        }
        Some(r) => {
            let note = match r {
                Rating::Ready => "Отлично. Этот вопрос вернётся через несколько дней.",
                Rating::Partly => "Вопрос вернётся завтра. Перечитайте уроки по теме.",
                Rating::NotYet => {
                    "Ничего страшного: вопрос вернётся завтра. Начните с урока по теме."
                }
            };
            col = col.push(
                txt(format!("Ваша оценка: {}. {note}", r.label()))
                    .size(theme::text::SMALL)
                    .color(if r == Rating::Ready { p.ok } else { p.muted }),
            );
        }
    }
    col
}

fn summary<'a>(_ctx: &'a Ctx, r: &'a Rehearsal, p: Palette) -> Element<'a, Msg> {
    let bank = interview::bank();
    let run = Run {
        day: String::new(),
        ids: r.ids.clone(),
        ratings: r.ratings.clone(),
    };
    let pct = run.pct();
    let verdict = if pct >= 80 {
        "Вы уверенно говорите о своём пути. Продолжайте жить так, как рассказываете."
    } else if pct >= 50 {
        "Хорошая основа. Отработайте вопросы, где ответ был неполным."
    } else {
        "Есть над чем поработать. Начните с вопросов ниже и уроков по их темам."
    };
    let mut col = Column::new().spacing(14).width(Length::Fill);
    col = col.push(widgets::kicker("Итоги репетиции"));
    col = col.push(
        row![
            widgets::progress_ring(pct as f32 / 100.0),
            column![
                txt(format!("{pct}%"))
                    .font(SERIF_BOLD)
                    .size(theme::text::H2)
                    .color(p.gold2),
                txt(format!(
                    "Готов: {} · Частично: {} · Ещё не готов: {}",
                    run.count(Rating::Ready),
                    run.count(Rating::Partly),
                    run.count(Rating::NotYet)
                ))
                .size(theme::text::SMALL)
                .color(p.muted),
                txt(verdict).size(theme::text::BODY).color(p.text),
            ]
            .spacing(4)
            .width(Length::Fill),
        ]
        .spacing(18)
        .align_y(Alignment::Center),
    );
    for (id, rating) in r.ids.iter().zip(&r.ratings) {
        let Some(q) = bank.item(id) else { continue };
        let color = match rating {
            Rating::Ready => p.ok,
            Rating::Partly => p.gold2,
            Rating::NotYet => p.bad,
        };
        let mut line = row![
            container(widgets::pill(rating.label(), widgets::Pill::Tint(color)))
                .width(Length::Fixed(140.0)),
            txt(q.question.as_str())
                .size(theme::text::BODY)
                .color(p.text)
                .width(Length::Fill),
        ]
        .spacing(10)
        .align_y(Alignment::Center);
        if *rating != Rating::Ready {
            line = line.push(widgets::btn(
                "Отработать",
                widgets::Btn::Ghost,
                widgets::BtnSize::Sm,
                Some(Msg::PracticeOne(id.clone())),
            ));
        }
        col = col.push(line);
    }
    col = col.push(
        row![
            widgets::btn(
                "Новая репетиция",
                widgets::Btn::Primary,
                widgets::BtnSize::Md,
                Some(Msg::Restart),
            ),
            widgets::btn(
                "К темам",
                widgets::Btn::Secondary,
                widgets::BtnSize::Md,
                Some(Msg::Tab(Mode::Browse)),
            ),
        ]
        .spacing(10),
    );
    widgets::card(col)
}
