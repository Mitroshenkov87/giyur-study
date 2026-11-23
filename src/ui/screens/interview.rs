//! «Тренажёр собеседования с бейт-дином» (map banner; route `interview`).
//!
//! Three tabs over the bank in `crate::interview`:
//! - «Темы»: reading mode — topic list with progress, then question + model
//!   answer; no text input;
//! - «Практика»: multiple choice (3–4 options), checked at once, feeds SRS;
//! - «Репетиция»: ten multiple-choice questions and a score, plus an optional
//!   «Ответить вслух» (speak, reveal the model, self-mark — not scored).
//!
//! All rules live in `crate::interview`; this screen owns only view state.

use iced::widget::text::Shaping;
use iced::widget::{button, column, container, row, space, text, Column, Row, Text};
use iced::{Alignment, Background, Border, Color, Element, Length};

use crate::i18n::tr;
use crate::interview::{self, Item, Mode, Rating, Run};
use crate::storage::Level;
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

fn level(ctx: &Ctx) -> Level {
    interview::user_level(&ctx.state)
}

/// One question being answered (practice and rehearsal).
struct Answering {
    id: String,
    /// Display order of option indices (shuffled).
    order: Vec<usize>,
    /// Original-index of the option the learner picked.
    picked: Option<usize>,
    /// «Ответить вслух» is open (rehearsal only).
    oral: bool,
    oral_revealed: bool,
    oral_knew: Option<bool>,
}

impl Answering {
    fn new(id: String, ctx: &Ctx) -> Self {
        let (opts, _) = interview::bank()
            .item(&id)
            .map(|q| q.choices(level(ctx)))
            .unwrap_or((&[], 0));
        let mut order: Vec<usize> = (0..opts.len()).collect();
        use rand::seq::SliceRandom;
        order.shuffle(&mut rand::thread_rng());
        Answering {
            id,
            order,
            picked: None,
            oral: false,
            oral_revealed: false,
            oral_knew: None,
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
    /// «Темы» and «Практика» topic filter (None = all topics / the topic list).
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
    PrevTopic,
    NextTopic,
    Toggle(String),
    /// «Отработать»: practise this question now.
    PracticeOne(String),
    /// Pick a multiple-choice option (index in the item's option list).
    Pick(usize),
    /// Open the optional oral path in a rehearsal.
    Speak,
    /// Leave the oral path and go back to the options (before the model is shown).
    CloseOral,
    /// Show the model answer on the oral path.
    RevealOral,
    /// Self-mark after speaking; not scored.
    Knew(bool),
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

    /// The question now on screen (practice or an in-progress rehearsal).
    pub fn current_id(&self) -> Option<&str> {
        match self.mode {
            Mode::Practice => self.practice.as_ref().map(|a| a.id.as_str()),
            Mode::Rehearsal => self
                .rehearsal
                .as_ref()
                .filter(|r| !r.done)
                .map(|r| r.cur.id.as_str()),
            Mode::Browse => None,
        }
    }

    fn enter(&mut self, ctx: &Ctx, mode: Mode) {
        self.mode = mode;
        match mode {
            Mode::Browse => {}
            Mode::Practice => {
                if self.practice.as_ref().is_none_or(|a| a.picked.is_some()) {
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
        self.practice = id.map(|id| Answering::new(id, ctx));
    }

    fn new_rehearsal(&mut self, ctx: &Ctx) {
        let ids = interview::rehearsal(&ctx.state, today(ctx), &mut rand::thread_rng());
        self.rehearsal = ids.first().cloned().map(|first| Rehearsal {
            cur: Answering::new(first, ctx),
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

    fn shift_topic(&mut self, ctx: &Ctx, dir: i32) {
        let next = interview::neighbor_topic(self.topic.as_deref(), dir);
        if next != self.topic.as_deref() {
            self.topic = next.map(str::to_string);
            if self.mode == Mode::Practice {
                self.practice = None;
                self.next_practice(ctx);
            }
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
            Msg::PrevTopic => self.shift_topic(ctx, -1),
            Msg::NextTopic => self.shift_topic(ctx, 1),
            Msg::Toggle(id) => {
                if let Some(i) = self.open.iter().position(|x| *x == id) {
                    self.open.remove(i);
                } else {
                    self.open.push(id);
                }
            }
            Msg::PracticeOne(id) => {
                self.mode = Mode::Practice;
                self.practice = Some(Answering::new(id, ctx));
            }
            Msg::Pick(idx) => {
                let day = today(ctx);
                let Some(cur) = self.current_mut() else {
                    return Action::None;
                };
                if cur.picked.is_some() {
                    return Action::None;
                }
                let id = cur.id.clone();
                cur.picked = Some(idx);
                let Some(q) = interview::bank().item(&id) else {
                    return Action::None;
                };
                let (_, right) = q.choices(level(ctx));
                let ok = idx == right;
                let rating = if ok { Rating::Ready } else { Rating::NotYet };
                interview::rate(&mut ctx.state, &id, rating, day);
                if ok {
                    ctx.award(interview::XP_READY);
                }
                if self.mode == Mode::Rehearsal {
                    if let Some(reh) = self.rehearsal.as_mut() {
                        reh.ratings.push(rating);
                    }
                }
                ctx.save();
            }
            Msg::Speak => {
                if self.mode == Mode::Rehearsal {
                    if let Some(cur) = self.current_mut() {
                        cur.oral = true;
                    }
                }
            }
            Msg::CloseOral => {
                if let Some(cur) = self.current_mut() {
                    if cur.oral && !cur.oral_revealed {
                        cur.oral = false;
                    }
                }
            }
            Msg::RevealOral => {
                if let Some(cur) = self.current_mut() {
                    if cur.oral {
                        cur.oral_revealed = true;
                    }
                }
            }
            Msg::Knew(knew) => {
                if let Some(cur) = self.current_mut() {
                    if cur.oral && cur.oral_revealed && cur.oral_knew.is_none() {
                        cur.oral_knew = Some(knew);
                    }
                }
            }
            Msg::Next => match self.mode {
                Mode::Practice => {
                    if self.practice.as_ref().is_some_and(|a| a.picked.is_some()) {
                        self.next_practice(ctx);
                    }
                }
                Mode::Rehearsal => {
                    let day = ctx.today_key();
                    let mut finished = None;
                    if let Some(reh) = self.rehearsal.as_mut() {
                        if reh.cur.picked.is_some() && !reh.done {
                            let i = reh.ratings.len();
                            if let Some(id) = reh.ids.get(i) {
                                reh.cur = Answering::new(id.clone(), ctx);
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
        page = page.push(widgets::back_button(tr("Уроки"), Msg::Go(Route::Map)));
        page = page.push(
            column![
                widgets::kicker(tr("Подготовка к бейт-дину")),
                widgets::heading(&bank.title, 1),
            ]
            .spacing(6),
        );
        if self.mode == Mode::Browse && self.topic.is_none() {
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
                    crate::trf!(
                        "Отработано {} из {}",
                        "Practiced {} of {}",
                        st.practiced,
                        st.total
                    ),
                    widgets::Pill::Tag
                ),
                widgets::pill(
                    crate::trf!("Готов: {}", "Ready: {}", st.ready),
                    widgets::Pill::Tag
                ),
                widgets::pill(
                    crate::trf!("Репетиций: {}", "Rehearsals: {}", st.runs),
                    widgets::Pill::Tag
                ),
            ]
            .spacing(8)
            .wrap(),
        );
        let tabs = [Mode::Browse, Mode::Practice, Mode::Rehearsal];
        page = page.push(widgets::segmented(
            vec![
                (tr("Темы").to_string(), Msg::Tab(Mode::Browse)),
                (tr("Практика").to_string(), Msg::Tab(Mode::Practice)),
                (
                    crate::trf!(
                        "Репетиция · {} вопросов",
                        "Rehearsal · {} questions",
                        interview::REHEARSAL_LEN
                    ),
                    Msg::Tab(Mode::Rehearsal),
                ),
            ],
            tabs.iter().position(|m| *m == self.mode),
        ));
        page = page.push(
            txt(interview::mode_hint(self.mode))
                .size(theme::text::SMALL)
                .color(p.muted)
                .width(Length::Fill),
        );

        page = match self.mode {
            Mode::Browse => page.push(self.browse(ctx, p)),
            Mode::Practice => page.push(self.practice_view(ctx, p)),
            Mode::Rehearsal => page.push(self.rehearsal_view(ctx, p)),
        };
        page.into()
    }

    fn topic_nav<'a>(&'a self) -> Element<'a, Msg> {
        let prev = interview::neighbor_topic(self.topic.as_deref(), -1);
        let next = interview::neighbor_topic(self.topic.as_deref(), 1);
        row![
            widgets::btn(
                tr("Предыдущая тема"),
                widgets::Btn::Ghost,
                widgets::BtnSize::Sm,
                prev.map(|_| Msg::PrevTopic),
            ),
            widgets::btn(
                tr("Следующая тема"),
                widgets::Btn::Ghost,
                widgets::BtnSize::Sm,
                next.map(|_| Msg::NextTopic),
            ),
        ]
        .spacing(8)
        .into()
    }

    // ---- «Темы» --------------------------------------------------------

    fn browse<'a>(&'a self, ctx: &'a Ctx, p: Palette) -> Element<'a, Msg> {
        let bank = interview::bank();
        if self.topic.is_none() {
            return self.topic_list(ctx, p, false);
        }
        let mut col = Column::new().spacing(16).width(Length::Fill);
        col = col.push(self.topic_nav());
        col = col.push(widgets::btn(
            tr("Ко всем темам"),
            widgets::Btn::Ghost,
            widgets::BtnSize::Sm,
            Some(Msg::Topic(None)),
        ));
        for t in &bank.topics {
            if self.topic.as_deref() != Some(t.id.as_str()) {
                continue;
            }
            col = col.push(self.topic_card(ctx, t, p, true));
        }
        col.into()
    }

    fn topic_list<'a>(&'a self, ctx: &'a Ctx, p: Palette, pickable: bool) -> Element<'a, Msg> {
        let bank = interview::bank();
        let mut col = Column::new().spacing(12).width(Length::Fill);
        for t in &bank.topics {
            let (ready, n) = interview::topic_ready(&ctx.state, &t.id);
            let selected = self.topic.as_deref() == Some(t.id.as_str());
            let body = column![
                row![
                    column![
                        widgets::heading(&t.title, 3),
                        txt(t.blurb.as_str())
                            .size(theme::text::SMALL)
                            .color(p.muted),
                    ]
                    .spacing(2)
                    .width(Length::Fill),
                    widgets::pill(
                        crate::trf!("готов {ready} из {n}", "{ready} of {n} ready"),
                        widgets::Pill::Tag
                    ),
                ]
                .align_y(Alignment::Center)
                .spacing(10),
                topic_bar(ready, n, p),
            ]
            .spacing(8)
            .width(Length::Fill);
            if pickable || self.mode == Mode::Browse {
                let msg = if selected && self.mode == Mode::Practice {
                    Msg::Topic(None)
                } else {
                    Msg::Topic(Some(t.id.clone()))
                };
                col = col.push(widgets::card(
                    button(body)
                        .on_press(msg)
                        .style(|_, s| {
                            let pal = theme::current();
                            theme::button_ghost(&pal, s)
                        })
                        .padding(0)
                        .width(Length::Fill),
                ));
            } else {
                col = col.push(widgets::card(body));
            }
        }
        col.into()
    }

    fn topic_card<'a>(
        &'a self,
        ctx: &'a Ctx,
        t: &'a interview::Topic,
        p: Palette,
        reading: bool,
    ) -> Element<'a, Msg> {
        let bank = interview::bank();
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
                widgets::pill(
                    crate::trf!("готов {ready} из {n}", "{ready} of {n} ready"),
                    widgets::Pill::Tag
                ),
            ]
            .align_y(Alignment::Center)
            .spacing(10),
        );
        list = list.push(topic_bar(ready, n, p));
        for q in bank.in_topic(&t.id) {
            let opened = self.open.contains(&q.id);
            let _ = reading;
            list = list.push(question_row(q, opened, p));
            if opened {
                list = list.push(
                    container(
                        column![
                            guidance(ctx, q, p),
                            row![widgets::btn(
                                tr("Отработать этот вопрос"),
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
        widgets::card(list)
    }

    // ---- «Практика» ----------------------------------------------------

    fn practice_view<'a>(&'a self, ctx: &'a Ctx, p: Palette) -> Element<'a, Msg> {
        let mut col = Column::new().spacing(16).width(Length::Fill);
        col = col.push(self.practice_topics(ctx, p));
        if self.topic.is_some() {
            col = col.push(self.topic_nav());
        }
        let Some(a) = &self.practice else {
            return col
                .push(widgets::card(
                    txt(tr("В этой теме пока нет вопросов.")).color(p.muted),
                ))
                .into();
        };
        let Some(q) = interview::bank().item(&a.id) else {
            return col.into();
        };
        let mut body = answering(ctx, q, a, p, None, false);
        if a.picked.is_some() {
            body = body.push(
                row![widgets::btn(
                    tr("Следующий вопрос"),
                    widgets::Btn::Primary,
                    widgets::BtnSize::Md,
                    Some(Msg::Next),
                )]
                .spacing(10),
            );
        }
        col.push(widgets::card(body)).into()
    }

    fn practice_topics<'a>(&'a self, ctx: &'a Ctx, p: Palette) -> Element<'a, Msg> {
        let bank = interview::bank();
        let mut chips = Row::new().spacing(8);
        chips = chips.push(
            column![chip(tr("Все темы"), self.topic.is_none(), Msg::Topic(None),)].spacing(4),
        );
        for t in &bank.topics {
            let (ready, n) = interview::topic_ready(&ctx.state, &t.id);
            let selected = self.topic.as_deref() == Some(t.id.as_str());
            chips = chips.push(
                column![
                    chip(t.title.as_str(), selected, Msg::Topic(Some(t.id.clone())),),
                    container(topic_bar(ready, n, p)).width(Length::Fixed(92.0)),
                ]
                .spacing(4)
                .align_x(Alignment::Center),
            );
        }
        chips.wrap().vertical_spacing(10).into()
    }

    // ---- «Репетиция» ---------------------------------------------------

    fn rehearsal_view<'a>(&'a self, ctx: &'a Ctx, p: Palette) -> Element<'a, Msg> {
        let Some(r) = &self.rehearsal else {
            return widgets::card(txt(tr("Нет вопросов для репетиции.")).color(p.muted));
        };
        if r.done {
            return summary(ctx, r, p);
        }
        let Some(q) = interview::bank().item(&r.cur.id) else {
            return column![].into();
        };
        let no = r.ratings.len() + usize::from(r.cur.picked.is_none());
        let no = no.max(1);
        let mut body = answering(ctx, q, &r.cur, p, Some((no, r.ids.len())), true);
        if r.cur.picked.is_some() {
            let last = r.ratings.len() >= r.ids.len();
            body = body.push(row![widgets::btn(
                if last {
                    tr("Итоги репетиции")
                } else {
                    tr("Дальше")
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

fn topic_bar<'a, M: 'a>(ready: usize, total: usize, p: Palette) -> Element<'a, M> {
    let total = total.max(1);
    let filled = ((ready.min(total) as f32 / total as f32) * 200.0).round() as u16;
    let rest = 200u16.saturating_sub(filled);
    let mut r = Row::new().height(8.0).width(Length::Fill);
    if filled > 0 {
        r = r.push(
            container(space::horizontal())
                .width(Length::FillPortion(filled))
                .height(8.0)
                .style(move |_| container::Style {
                    background: Some(Background::Color(p.gold)),
                    border: Border {
                        radius: theme::radius::SEGMENT.into(),
                        ..Border::default()
                    },
                    ..container::Style::default()
                }),
        );
    }
    if rest > 0 {
        r = r.push(
            container(space::horizontal())
                .width(Length::FillPortion(rest.max(1)))
                .height(8.0)
                .style(move |_| container::Style {
                    background: Some(Background::Color(p.surface3)),
                    border: Border {
                        radius: theme::radius::SEGMENT.into(),
                        ..Border::default()
                    },
                    ..container::Style::default()
                }),
        );
    }
    container(r)
        .width(Length::Fill)
        .height(8.0)
        .style(move |_| container::Style {
            background: Some(Background::Color(p.surface3)),
            border: Border {
                radius: theme::radius::SEGMENT.into(),
                ..Border::default()
            },
            ..container::Style::default()
        })
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
            label(tr("Что хотят услышать раввины"), p.gold2),
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
            label(tr("О чём сказать честно, своими словами"), p.ok),
            bullets(&q.key_points, "•", p.ok),
        ]
        .spacing(6),
    );
    col = col.push(
        column![
            label(tr("Частые ошибки"), p.bad),
            bullets(&q.mistakes, "×", p.bad),
        ]
        .spacing(6),
    );
    if let Some(m) = &q.movements {
        col = col.push(widgets::note_block(tr("Различия течений"), m.as_str()));
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
            label(tr("Уроки по теме"), p.muted),
            links.wrap().vertical_spacing(6)
        ]
        .spacing(6),
    );
    col.into()
}

/// A shorter model answer for the oral path (what to say, without the
/// "common mistakes" block shouting at the learner mid-rehearsal).
fn model_answer<'a>(q: &'a Item, p: Palette) -> Element<'a, Msg> {
    column![
        tinted(
            column![
                label(tr("Образец ответа"), p.gold2),
                txt(q.purpose.as_str())
                    .size(theme::text::BODY)
                    .color(p.text),
            ]
            .spacing(6),
            p.gold_soft,
            alpha(p.gold, 0.3),
        ),
        column![
            label(tr("О чём сказать своими словами"), p.ok),
            bullets(&q.key_points, "•", p.ok),
        ]
        .spacing(6),
    ]
    .spacing(12)
    .into()
}

fn option_buttons<'a>(q: &'a Item, a: &'a Answering, p: Palette, lvl: Level) -> Element<'a, Msg> {
    let (opts, right) = q.choices(lvl);
    let mut list = Column::new().spacing(10).width(Length::Fill);
    for (k, oi) in a.order.iter().enumerate() {
        let oi = *oi;
        if oi >= opts.len() {
            continue;
        }
        let is_right = a.picked.is_some() && oi == right;
        let is_chosen_wrong = a.picked == Some(oi) && oi != right;
        let label = row![
            txt(format!("{}.", k + 1))
                .size(theme::text::SMALL)
                .font(SANS_BOLD)
                .color(p.muted),
            txt(opts[oi].as_str())
                .size(theme::text::BODY)
                .color(p.text)
                .width(Length::Fill),
        ]
        .spacing(12)
        .align_y(Alignment::Center)
        .width(Length::Fill);
        let b = button(label)
            .width(Length::Fill)
            .padding([10.0, 14.0])
            .style(move |_, s| {
                if is_right {
                    theme::option_correct(&p, s)
                } else if is_chosen_wrong {
                    theme::option_wrong(&p, s)
                } else {
                    theme::option_normal(&p, s)
                }
            });
        let b = if a.picked.is_some() {
            b
        } else {
            b.on_press(Msg::Pick(oi))
        };
        list = list.push(b);
    }
    list.into()
}

/// A question to answer: prompt, options, feedback; in rehearsal, the
/// optional oral path. `step` = (number, of) in a rehearsal.
fn answering<'a>(
    ctx: &'a Ctx,
    q: &'a Item,
    a: &'a Answering,
    p: Palette,
    step: Option<(usize, usize)>,
    rehearsal: bool,
) -> Column<'a, Msg> {
    let bank = interview::bank();
    let topic = bank.topic(&q.topic).map(|t| t.title.as_str()).unwrap_or("");
    let meta = match step {
        Some((n, of)) => crate::trf!(
            "Вопрос {n} из {of} · {topic}",
            "Question {n} of {of} · {topic}"
        ),
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

    let hide_options = rehearsal && a.oral && !a.oral_revealed && a.picked.is_none();

    if rehearsal && a.oral {
        col = col.push(oral_panel(q, a, p));
    } else if rehearsal && a.picked.is_none() {
        col = col.push(row![widgets::btn(
            tr("Ответить вслух"),
            widgets::Btn::Ghost,
            widgets::BtnSize::Sm,
            Some(Msg::Speak),
        )]);
    }

    if !hide_options {
        col = col.push(option_buttons(q, a, p, level(ctx)));
    }

    if let Some(idx) = a.picked {
        let (_, right) = q.choices(level(ctx));
        let ok = idx == right;
        let (opts, _) = q.choices(level(ctx));
        let right_text = opts.get(right).map(|s| s.as_str()).unwrap_or("");
        let note = if ok {
            tr("Верно. Этот вопрос вернётся через несколько дней.").to_string()
        } else {
            crate::trf!(
                "Неверно. Верный ответ: {right_text} Вопрос вернётся завтра.",
                "Incorrect. The right answer: {right_text} The question will come back tomorrow."
            )
        };
        col = col.push(
            txt(note)
                .size(theme::text::SMALL)
                .color(if ok { p.ok } else { p.bad }),
        );
        col = col.push(guidance(ctx, q, p));
    }

    if rehearsal && a.picked.is_some() && !a.oral {
        col = col.push(row![widgets::btn(
            tr("Ответить вслух"),
            widgets::Btn::Ghost,
            widgets::BtnSize::Sm,
            Some(Msg::Speak),
        )]);
    }
    col
}

fn oral_panel<'a>(q: &'a Item, a: &'a Answering, p: Palette) -> Element<'a, Msg> {
    let mut col = Column::new().spacing(12).width(Length::Fill);
    if !a.oral_revealed {
        col = col.push(
            txt(tr("Проговорите ответ вслух своими словами. Когда будете готовы, откройте образец. Это не входит в счёт репетиции."))
                .size(theme::text::BODY)
                .color(p.text),
        );
        col = col.push(
            row![
                widgets::btn(
                    tr("Показать образец"),
                    widgets::Btn::Primary,
                    widgets::BtnSize::Md,
                    Some(Msg::RevealOral),
                ),
                widgets::btn(
                    tr("К вариантам ответа"),
                    widgets::Btn::Ghost,
                    widgets::BtnSize::Sm,
                    Some(Msg::CloseOral),
                ),
            ]
            .spacing(10),
        );
    } else {
        col = col.push(model_answer(q, p));
        match a.oral_knew {
            None => {
                col = col.push(
                    txt(tr("Отметьте честно: это не входит в счёт репетиции."))
                        .size(theme::text::SMALL)
                        .color(p.muted),
                );
                col = col.push(
                    row![
                        widgets::btn(
                            tr("Знал"),
                            widgets::Btn::Primary,
                            widgets::BtnSize::Md,
                            Some(Msg::Knew(true)),
                        ),
                        widgets::btn(
                            tr("Не знал"),
                            widgets::Btn::Secondary,
                            widgets::BtnSize::Md,
                            Some(Msg::Knew(false)),
                        ),
                    ]
                    .spacing(10),
                );
            }
            Some(knew) => {
                let note = if knew {
                    crate::trf!(
                        "Вы отметили: знал. В счёт репетиции это не входит.",
                        "You marked: knew it. This doesn't count toward the rehearsal score."
                    )
                } else {
                    crate::trf!(
                        "Вы отметили: не знал. В счёт репетиции это не входит.",
                        "You marked: didn't know. This doesn't count toward the rehearsal score."
                    )
                };
                col = col.push(txt(note).size(theme::text::SMALL).color(p.muted));
            }
        }
    }
    tinted(col, p.gold_soft, alpha(p.gold, 0.3))
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
        tr("Вы уверенно говорите о своём пути. Продолжайте жить так, как рассказываете.")
    } else if pct >= 50 {
        tr("Хорошая основа. Отработайте вопросы, где ответ был неверным.")
    } else {
        tr("Есть над чем поработать. Начните с вопросов ниже и уроков по их темам.")
    };
    let mut col = Column::new().spacing(14).width(Length::Fill);
    col = col.push(widgets::kicker(tr("Итоги репетиции")));
    col = col.push(
        row![
            widgets::progress_ring(pct as f32 / 100.0),
            column![
                txt(format!("{pct}%"))
                    .font(SERIF_BOLD)
                    .size(theme::text::H2)
                    .color(p.gold2),
                txt(crate::trf!(
                    "Верно: {} · Неверно: {}",
                    "Correct: {} · Incorrect: {}",
                    run.count(Rating::Ready),
                    run.count(Rating::NotYet),
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
        let (label, color) = match rating {
            Rating::Ready => (tr("Верно"), p.ok),
            Rating::NotYet => (tr("Неверно"), p.bad),
            Rating::Partly => (tr("Частично"), p.gold2),
        };
        let mut line = row![
            container(widgets::pill(label, widgets::Pill::Tint(color))).width(Length::Fixed(140.0)),
            txt(q.question.as_str())
                .size(theme::text::BODY)
                .color(p.text)
                .width(Length::Fill),
        ]
        .spacing(10)
        .align_y(Alignment::Center);
        if *rating != Rating::Ready {
            line = line.push(widgets::btn(
                tr("Отработать"),
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
                tr("Новая репетиция"),
                widgets::Btn::Primary,
                widgets::BtnSize::Md,
                Some(Msg::Restart),
            ),
            widgets::btn(
                tr("К темам"),
                widgets::Btn::Secondary,
                widgets::BtnSize::Md,
                Some(Msg::Tab(Mode::Browse)),
            ),
        ]
        .spacing(10),
    );
    widgets::card(col)
}
