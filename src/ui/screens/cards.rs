//! Flashcard sessions: lesson deck, section deck, daily review
//! (PARITY §6, js/ui/cards.js).

use std::collections::HashSet;

use iced::widget::canvas::{self, Canvas, Frame, Path, Stroke};
use iced::widget::text::{Shaping, Text};
use iced::widget::{button, column, container, progress_bar, row, text, Column, Row, Space};
use iced::{
    keyboard, Alignment, Background, Border, Element, Length, Point, Rectangle, Renderer,
    Subscription, Theme,
};

use crate::content::{self, Differs};
use crate::game;
use crate::srs;
use crate::ui::app::{Action, Ctx, Deck, Route};
use crate::ui::fonts::{SANS, SANS_BOLD, SERIF_BOLD};
use crate::ui::theme::{self, Palette};
use crate::ui::widgets;
use crate::util;

fn txt<'a>(content: impl iced::widget::text::IntoFragment<'a>) -> Text<'a> {
    text(content).shaping(Shaping::Advanced)
}

fn palette() -> Palette {
    theme::current()
}

/// One queue entry; a missed card is re-queued once at the end with `repeat`.
#[derive(Debug, Clone)]
struct Entry {
    id: String,
    repeat: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Phase {
    /// First-run explainer «Как это работает?».
    Help,
    /// Working through the queue.
    Cards,
    /// Deck finished, summary card.
    Done,
}

/// Guard failure (§6).
#[derive(Debug, Clone, PartialEq)]
enum Guard {
    /// «Урок не найден.»
    LessonNotFound,
    /// «Раздел закрыт или не найден.»
    SectionLocked,
}

pub struct Screen {
    // Deck context (§6 table).
    title: String,
    kicker: String,
    back: Route,
    back_label: String,
    word: &'static str,
    done_label: String,
    done_text: String,
    empty_text: String,

    guard: Option<Guard>,
    queue: Vec<Entry>,
    total: usize,
    pos: usize,
    knew: usize,
    missed: usize,
    requeued: HashSet<String>,
    flipped: bool,
    phase: Phase,
    /// Grade feedback note shown above the card (text, ok?).
    note: Option<(String, bool)>,
    session_xp0: i64,
    gained: i64,
    open_others: HashSet<u32>,
    // Summary strings computed at finish (owned, for the done card).
    sum_pct: String,
    sum_counts: String,
    sum_xp: String,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Go(Route),
    Flip,
    Grade(bool),
    Help,
    HelpOk,
    ToggleDiffers(u32),
}

impl Screen {
    pub fn new(ctx: &mut Ctx, route: &Route) -> Self {
        let deck = match route {
            Route::Cards(d) => d.clone(),
            _ => Deck::Review,
        };
        let user = content::Library::user_level(&ctx.state);
        let mut rng = rand::thread_rng();

        let mut s = Screen {
            title: String::new(),
            kicker: String::new(),
            back: Route::Map,
            back_label: "Карта".into(),
            word: "уроке",
            done_label: "Готово".into(),
            done_text: "Колода пройдена!".into(),
            empty_text: "Карточек нет.".into(),
            guard: None,
            queue: Vec::new(),
            total: 0,
            pos: 0,
            knew: 0,
            missed: 0,
            requeued: HashSet::new(),
            flipped: false,
            phase: Phase::Cards,
            note: None,
            session_xp0: ctx.state.xp_total,
            gained: 0,
            open_others: HashSet::new(),
            sum_pct: String::new(),
            sum_counts: String::new(),
            sum_xp: String::new(),
        };

        match deck {
            Deck::Lesson(id) => match ctx.lib.lesson(&id) {
                Some((lesson, sec)) if ctx.lib.is_unlocked(sec, &ctx.state) => {
                    s.title = "Карточки урока".into();
                    s.kicker = lesson.title.clone().unwrap_or_default();
                    s.back = Route::Section(sec.id.clone());
                    s.back_label = sec.title.clone();
                    s.word = "уроке";
                    s.done_label = "К разделу".into();
                    s.queue = ctx
                        .lib
                        .lesson_cards(lesson, user)
                        .iter()
                        .map(|c| Entry {
                            id: c.id.clone(),
                            repeat: false,
                        })
                        .collect();
                }
                _ => s.guard = Some(Guard::LessonNotFound),
            },
            Deck::Section(id) => match ctx.lib.section(&id) {
                Some(sec) if sec.available && ctx.lib.is_unlocked(sec, &ctx.state) => {
                    s.title = "Карточки раздела".into();
                    s.kicker = sec.title.clone();
                    s.back = Route::Section(sec.id.clone());
                    s.back_label = sec.title.clone();
                    s.word = "разделе";
                    s.done_label = "К разделу".into();
                    let all = ctx.lib.section_cards(sec, user);
                    let today = ctx.today();
                    // Due (shuffled) → new → rest (shuffled), js/ui/cards.js.
                    let due: Vec<String> = all
                        .iter()
                        .filter(|c| srs::is_due(&ctx.state, &c.id, today))
                        .map(|c| c.id.clone())
                        .collect();
                    let fresh: Vec<String> = all
                        .iter()
                        .filter(|c| srs::is_new(&ctx.state, &c.id))
                        .map(|c| c.id.clone())
                        .collect();
                    let rest: Vec<String> = all
                        .iter()
                        .filter(|c| !due.contains(&c.id) && !fresh.contains(&c.id))
                        .map(|c| c.id.clone())
                        .collect();
                    let mut queue: Vec<Entry> = Vec::new();
                    for id in util::shuffle(&due, &mut rng) {
                        queue.push(Entry { id, repeat: false });
                    }
                    for id in fresh {
                        queue.push(Entry { id, repeat: false });
                    }
                    for id in util::shuffle(&rest, &mut rng) {
                        queue.push(Entry { id, repeat: false });
                    }
                    s.queue = queue;
                }
                _ => s.guard = Some(Guard::SectionLocked),
            },
            Deck::Review => {
                s.title = "Повторение дня".into();
                s.kicker = util::fmt_date(ctx.today());
                s.back = Route::Map;
                s.back_label = "Карта".into();
                s.word = "повторении";
                s.done_label = "На карту".into();
                s.done_text = "Повторение на сегодня завершено!".into();
                s.empty_text = "На сегодня повторять нечего. Изучайте новые карточки \
                                в разделах — они появятся здесь в нужный день."
                    .into();
                let due: Vec<String> = ctx
                    .lib
                    .due_cards(&ctx.state, ctx.today())
                    .iter()
                    .map(|c| c.id.clone())
                    .collect();
                s.queue = util::shuffle(&due, &mut rng)
                    .into_iter()
                    .map(|id| Entry { id, repeat: false })
                    .collect();
            }
        }

        s.total = s.queue.len();
        if s.guard.is_none() && s.total > 0 && !ctx.state.cards_help_seen {
            s.phase = Phase::Help;
        }
        s
    }

    pub fn update(&mut self, ctx: &mut Ctx, msg: Msg) -> Action {
        match msg {
            Msg::Go(route) => Action::Go(route),
            Msg::Flip => {
                if self.phase == Phase::Cards && self.pos < self.queue.len() {
                    self.flipped = !self.flipped;
                }
                Action::None
            }
            Msg::Grade(knew) => {
                if self.phase != Phase::Cards || self.pos >= self.queue.len() {
                    return Action::None;
                }
                // Grading a card that is not flipped yet flips it instead.
                if !self.flipped {
                    self.flipped = true;
                    return Action::None;
                }
                let entry = self.queue[self.pos].clone();
                if !entry.repeat {
                    let today = ctx.today();
                    let c = srs::grade(&mut ctx.state, &entry.id, knew, today);
                    self.note = Some((srs::grade_message(&c, knew, self.word, today), knew));
                    if knew {
                        self.knew += 1;
                    } else {
                        self.missed += 1;
                    }
                } else {
                    self.note = Some((
                        if knew {
                            "Принято".to_string()
                        } else {
                            "Завтра покажем снова".to_string()
                        },
                        knew,
                    ));
                }
                if !knew && !self.requeued.contains(&entry.id) {
                    self.requeued.insert(entry.id.clone());
                    self.queue.push(Entry {
                        id: entry.id,
                        repeat: true,
                    });
                }
                self.pos += 1;
                self.flipped = false;
                // Both answers count («Не вспомнил» тоже: важна работа).
                ctx.award(game::XP.card);
                if self.pos >= self.queue.len() {
                    self.finish(ctx);
                }
                Action::None
            }
            Msg::Help => {
                self.phase = Phase::Help;
                Action::None
            }
            Msg::HelpOk => {
                ctx.state.cards_help_seen = true;
                ctx.save();
                self.phase = Phase::Cards;
                Action::None
            }
            Msg::ToggleDiffers(id) => {
                if !self.open_others.remove(&id) {
                    self.open_others.insert(id);
                }
                Action::None
            }
        }
    }

    /// Deck finished: decks counter, +5 XP, summary (js/ui/cards.js finish).
    fn finish(&mut self, ctx: &mut Ctx) {
        game::count(&mut ctx.state, "decks", 1);
        ctx.award(game::XP.deck);
        self.gained = ctx.state.xp_total - self.session_xp0;
        let pct = if self.total > 0 {
            self.knew as f32 / self.total as f32
        } else {
            0.0
        };
        self.sum_pct = format!("{}%", (pct * 100.0).round() as i64);
        self.sum_counts = format!(
            "Вспомнил: **{}** · Не вспомнил: **{}**",
            self.knew, self.missed
        );
        self.sum_xp = format!(
            "+{} XP за колоду · сегодня {} / {} XP",
            self.gained,
            game::today_xp(&ctx.state, ctx.today()),
            game::goal(&ctx.state)
        );
        self.phase = Phase::Done;
    }

    pub fn view<'a>(&'a self, ctx: &'a Ctx) -> Element<'a, Msg> {
        if let Some(guard) = &self.guard {
            return not_found_view(guard);
        }
        let header = header(
            &self.back_label,
            Msg::Go(self.back.clone()),
            &self.kicker,
            &self.title,
        );

        let body: Element<Msg> = if self.total == 0 {
            empty_view(&self.empty_text)
        } else {
            match self.phase {
                Phase::Help => help_view(),
                Phase::Cards => self.card_view(ctx),
                Phase::Done => self.done_view(ctx),
            }
        };

        column![header, body].spacing(16).width(Length::Fill).into()
    }

    /// The current card with counter, step bar, flip card and grade buttons.
    fn card_view<'a>(&'a self, ctx: &'a Ctx) -> Element<'a, Msg> {
        let p = palette();
        let entry = &self.queue[self.pos.min(self.queue.len() - 1)];
        let (card, lesson, _) = ctx
            .lib
            .card(&entry.id)
            .expect("queue entries resolve in the library");
        let len = self.queue.len();
        let done = self.pos.min(len);

        // Top row: counter · step bar · «повтор» tag or level dots · help pill.
        let steps: Element<Msg> = if len <= 20 {
            widgets::step_bar(done, len)
        } else {
            progress_bar(0.0..=len as f32, done as f32)
                .length(Length::Fill)
                .girth(14.0)
                .style(move |_| theme::progress_bar(&p))
                .into()
        };
        let marker: Element<Msg> = if entry.repeat {
            widgets::tag("повтор")
        } else {
            let level = ctx.state.cards.get(&entry.id).map(|c| c.l).unwrap_or(0);
            level_dots(level)
        };
        let help = button(
            txt("Как это работает?")
                .size(theme::text::SMALL - 1.5)
                .color(p.muted),
        )
        .padding([8.0, 12.0])
        .style(move |_, s| theme::button_ghost(&p, s))
        .on_press(Msg::Help);
        let top = row![
            txt(format!("{} / {}", self.pos + 1, len))
                .size(theme::text::SMALL)
                .color(p.muted),
            container(steps).width(Length::Fill).padding([0.0, 8.0]),
            marker,
            help,
        ]
        .spacing(10)
        .align_y(Alignment::Center)
        .width(Length::Fill);

        // Flip card: front until clicked, back with differs notes after.
        let face: Element<Msg> = if self.flipped {
            let mut back = Column::new().spacing(10).width(Length::Fill);
            back = back.push(widgets::kicker("Ответ"));
            back = back.push(
                txt(&card.back)
                    .size(theme::text::BODY * 1.12)
                    .font(SANS)
                    .color(p.text),
            );
            let mut group = 1000u32; // card-back differs group ids
            if let Some(el) = differs_view(&card.differs, ctx, &mut group, &self.open_others) {
                back = back.push(el);
            }
            flip_container(back.into(), true)
        } else {
            let hint = txt("Вспомните ответ и нажмите, чтобы перевернуть")
                .size(theme::text::SMALL)
                .color(p.muted)
                .align_x(iced::alignment::Horizontal::Center)
                .width(Length::Fill);
            let front = column![
                widgets::kicker(lesson.title.as_deref().unwrap_or("")),
                Space::new().height(Length::Fill),
                txt(&card.front)
                    .size(theme::text::BODY * 1.35)
                    .font(SERIF_BOLD)
                    .color(p.text)
                    .align_x(iced::alignment::Horizontal::Center)
                    .width(Length::Fill),
                Space::new().height(Length::Fill),
                hint,
            ]
            .spacing(10)
            .align_x(Alignment::Center)
            .width(Length::Fill)
            // Fixed height: Fill spacers inside the shell's scrollable would
            // otherwise collapse the card to zero height.
            .height(Length::Fixed(252.0));
            flip_container(front.into(), false)
        };
        let flip = button(face)
            .width(Length::Fill)
            .padding(0.0)
            .style(|_, _| button::Style::default())
            .on_press(Msg::Flip);

        // Grade buttons, disabled until the card is flipped.
        let knew = widgets::btn_icon(
            grade_key("1"),
            "Вспомнил",
            widgets::Btn::Primary,
            widgets::BtnSize::Lg,
            self.flipped.then_some(Msg::Grade(true)),
        );
        let miss = widgets::btn_icon(
            grade_key("2"),
            "Не вспомнил",
            widgets::Btn::Secondary,
            widgets::BtnSize::Lg,
            self.flipped.then_some(Msg::Grade(false)),
        );
        let grades = row![
            Space::new().width(Length::Fill),
            knew,
            miss,
            Space::new().width(Length::Fill),
        ]
        .spacing(12);

        let mut col = Column::new().spacing(14).width(Length::Fill);
        if let Some((note, ok)) = &self.note {
            col = col.push(grade_note(note, *ok));
        }
        col = col.push(top).push(flip).push(grades);
        col.into()
    }

    /// Summary card after the last card (js/ui/cards.js finish).
    fn done_view<'a>(&'a self, _ctx: &'a Ctx) -> Element<'a, Msg> {
        let p = palette();
        let pct = if self.total > 0 {
            self.knew as f32 / self.total as f32
        } else {
            0.0
        };

        let explain_text = format!(
            "Карточки, которые вы не вспомнили, покажем ещё раз в этом {}, а завтра \
             они снова в работе. Остальные — через 1, 3, 7, 14 и 30 дней.",
            self.word
        );

        let ring = done_ring(pct, &self.sum_pct, "Вспомнил");
        let counts = widgets::rich_paragraph(&self.sum_counts);
        let xp_line = txt(&self.sum_xp)
            .size(theme::text::BODY)
            .color(p.gold2)
            .align_x(iced::alignment::Horizontal::Center)
            .width(Length::Fill);
        let explain = txt(explain_text)
            .size(theme::text::SMALL)
            .color(p.muted)
            .align_x(iced::alignment::Horizontal::Center)
            .width(Length::Fill);

        let buttons = row![
            widgets::btn(
                self.done_label.clone(),
                widgets::Btn::Primary,
                widgets::BtnSize::Lg,
                Some(Msg::Go(self.back.clone())),
            ),
            widgets::btn(
                "На карту",
                widgets::Btn::Ghost,
                widgets::BtnSize::Lg,
                Some(Msg::Go(Route::Map)),
            ),
        ]
        .spacing(12);

        let card = widgets::card(
            column![
                ring,
                widgets::heading(&self.done_text, 2),
                counts,
                xp_line,
                explain,
                buttons,
            ]
            .spacing(14)
            .align_x(Alignment::Center)
            .width(Length::Fill),
        );

        let mut col = Column::new().spacing(14).width(Length::Fill);
        if let Some((note, ok)) = &self.note {
            col = col.push(grade_note(note, *ok));
        }
        col = col.push(card);
        col.into()
    }

    pub fn subscription(&self) -> Subscription<Msg> {
        if self.guard.is_some() || self.phase != Phase::Cards || self.total == 0 {
            return Subscription::none();
        }
        keyboard::listen().filter_map(|event| {
            let keyboard::Event::KeyPressed { key, modifiers, .. } = event else {
                return None;
            };
            // Global rule (§0.4): ignore when Alt/Ctrl/Meta is held.
            if modifiers.alt() || modifiers.control() || modifiers.logo() {
                return None;
            }
            match key {
                keyboard::Key::Named(keyboard::key::Named::Space)
                | keyboard::Key::Named(keyboard::key::Named::Enter) => Some(Msg::Flip),
                keyboard::Key::Named(keyboard::key::Named::ArrowLeft) => Some(Msg::Grade(true)),
                keyboard::Key::Named(keyboard::key::Named::ArrowRight) => Some(Msg::Grade(false)),
                keyboard::Key::Character(c) if c.as_str() == "1" => Some(Msg::Grade(true)),
                keyboard::Key::Character(c) if c.as_str() == "2" => Some(Msg::Grade(false)),
                _ => None,
            }
        })
    }
}

// ---------------------------------------------------------------------------
// Pieces
// ---------------------------------------------------------------------------

fn header<'a>(back_label: &'a str, back: Msg, kicker: &'a str, title: &'a str) -> Element<'a, Msg> {
    column![
        widgets::back_button(back_label, back),
        widgets::kicker(kicker),
        widgets::heading(title, 1),
    ]
    .spacing(10)
    .width(Length::Fill)
    .into()
}

/// The small key-hint box shown before a grade label («[1]», §6).
fn grade_key<'a>(key: &'a str) -> Element<'a, Msg> {
    let p = palette();
    container(txt(key).size(theme::text::SMALL - 2.0).font(SANS_BOLD))
        .padding([1.0, 6.0])
        .style(move |_| container::Style {
            border: Border {
                color: p.line2,
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        })
        .into()
}

/// The flip card container (radius 22; gold hairline front, gold-tinted back).
/// The 3D flip animation of the PWA is reduced to an instant swap.
fn flip_container<'a>(content: Element<'a, Msg>, back: bool) -> Element<'a, Msg> {
    let p = palette();
    let (bg, border_color) = if back {
        (p.gold_soft, p.gold)
    } else {
        (p.surface2, iced::Color { a: 0.25, ..p.gold })
    };
    container(
        // Zero-width strut enforces the 300 px min height while letting the
        // back side grow for long answers.
        row![Space::new().width(0.0).height(300.0), content]
            .spacing(0)
            .width(Length::Fill),
    )
    .padding(24.0)
    .width(Length::Fill)
    .style(move |_| container::Style {
        text_color: Some(p.text),
        background: Some(Background::Color(bg)),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: theme::radius::FLIP.into(),
        },
        ..Default::default()
    })
    .into()
}

/// Five SRS level dots (filled = current level, «Уровень {l} из 5»).
fn level_dots<'a>(level: i64) -> Element<'a, Msg> {
    let p = palette();
    let mut dots = Row::new().spacing(4).align_y(Alignment::Center);
    for i in 1..=5 {
        let fill = if i <= level { p.gold } else { p.surface3 };
        dots = dots.push(
            container(Space::new())
                .width(8.0)
                .height(8.0)
                .style(move |_| container::Style {
                    background: Some(Background::Color(fill)),
                    border: Border {
                        radius: 4.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
        );
    }
    dots.into()
}

/// Feedback note above the card («Следующий показ…», «Принято», …).
fn grade_note<'a>(note: &'a str, ok: bool) -> Element<'a, Msg> {
    let p = palette();
    let (fg, bg) = if ok {
        (p.ok, p.ok_soft)
    } else {
        (p.bad, p.bad_soft)
    };
    container(
        txt(note)
            .size(theme::text::BODY)
            .font(SANS_BOLD)
            .color(fg)
            .align_x(iced::alignment::Horizontal::Center)
            .width(Length::Fill),
    )
    .padding([10.0, 16.0])
    .width(Length::Fill)
    .style(move |_| container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            radius: theme::radius::SMALL.into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}

/// First-run explainer «Как это работает?» (§6, exact strings).
fn help_view<'a>() -> Element<'a, Msg> {
    const ITEMS: &[&str] = &[
        "Вспомните ответ про себя и нажмите на карточку, чтобы перевернуть её.",
        "Оцените себя честно: «Вспомнил» или «Не вспомнил».",
        "«Вспомнил» — карточка вернётся через 1, затем 3, 7, 14 и 30 дней.",
        "«Не вспомнил» — покажем её ещё раз в этом уроке, а расписание снова начнётся с 1 дня.",
        "Карточка считается «выученной», когда вы вспомните её на 30-дневном интервале.",
    ];
    let p = palette();
    let mut list = Column::new().spacing(8).width(Length::Fill);
    for (i, item) in ITEMS.iter().enumerate() {
        list = list.push(
            row![
                txt(format!("{}.", i + 1))
                    .size(theme::text::BODY)
                    .font(SANS_BOLD)
                    .color(p.gold),
                widgets::rich_paragraph(item),
            ]
            .spacing(10)
            .align_y(Alignment::Start),
        );
    }
    widgets::card(
        column![
            widgets::heading("Как это работает?", 3),
            list,
            row![
                Space::new().width(Length::Fill),
                widgets::btn(
                    "Понятно, начнём",
                    widgets::Btn::Primary,
                    widgets::BtnSize::Lg,
                    Some(Msg::HelpOk),
                ),
                Space::new().width(Length::Fill),
            ],
        ]
        .spacing(16)
        .width(Length::Fill),
    )
}

/// Empty deck card (§6): star + text + «На карту».
fn empty_view<'a>(empty_text: &'a str) -> Element<'a, Msg> {
    let p = palette();
    widgets::card(
        column![
            txt("✡")
                .size(40.0)
                .color(p.gold)
                .align_x(iced::alignment::Horizontal::Center),
            txt(empty_text)
                .size(theme::text::BODY)
                .color(p.muted)
                .align_x(iced::alignment::Horizontal::Center)
                .width(Length::Fill),
            widgets::btn(
                "На карту",
                widgets::Btn::Primary,
                widgets::BtnSize::Lg,
                Some(Msg::Go(Route::Map)),
            ),
        ]
        .spacing(14)
        .align_x(Alignment::Center)
        .width(Length::Fill),
    )
}

/// «Раздел закрыт или не найден.» / «Урок не найден.» (§6 guards).
fn not_found_view(guard: &Guard) -> Element<'static, Msg> {
    let p = palette();
    let (title, message) = match guard {
        Guard::LessonNotFound => ("Не найдено", "Урок не найден."),
        Guard::SectionLocked => ("Раздел закрыт", "Раздел закрыт или не найден."),
    };
    column![
        widgets::back_button("Карта", Msg::Go(Route::Map)),
        widgets::heading(title, 1),
        widgets::card(
            column![
                txt("✡")
                    .size(40.0)
                    .color(p.gold)
                    .align_x(iced::alignment::Horizontal::Center),
                txt(message)
                    .size(theme::text::BODY)
                    .color(p.muted)
                    .align_x(iced::alignment::Horizontal::Center)
                    .width(Length::Fill),
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

/// «differs» notes on the card back (§T.8).
fn differs_view<'a>(
    list: &'a [Differs],
    ctx: &'a Ctx,
    group: &mut u32,
    open: &HashSet<u32>,
) -> Option<Element<'a, Msg>> {
    if list.is_empty() {
        return None;
    }
    let p = palette();
    let user = content::Library::user_level(&ctx.state);
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

// ---------------------------------------------------------------------------
// Done ring (110 px, pct + label in the centre)
// ---------------------------------------------------------------------------

struct DoneRing {
    pct: f32,
}

impl<Message> canvas::Program<Message> for DoneRing {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let p = palette();
        let mut frame = Frame::new(renderer, bounds.size());
        let size = bounds.size();
        let min = size.width.min(size.height);
        let stroke_width = min * 0.08;
        let radius = (min - stroke_width) / 2.0;
        let center = Point::new(size.width / 2.0, size.height / 2.0);

        let track = Path::circle(center, radius);
        frame.stroke(
            &track,
            Stroke {
                style: canvas::Style::Solid(p.surface3),
                width: stroke_width,
                line_cap: canvas::LineCap::Round,
                ..Stroke::default()
            },
        );

        let pct = self.pct.clamp(0.0, 1.0);
        if pct > 0.0 {
            let start = -std::f32::consts::FRAC_PI_2;
            let end = start + pct * std::f32::consts::TAU;
            let arc = Path::new(|builder| {
                builder.move_to(Point::new(
                    center.x + radius * start.cos(),
                    center.y + radius * start.sin(),
                ));
                builder.arc(canvas::path::Arc {
                    center,
                    radius,
                    start_angle: iced::Radians(start),
                    end_angle: iced::Radians(end),
                });
            });
            frame.stroke(
                &arc,
                Stroke {
                    style: canvas::Style::Solid(p.gold),
                    width: stroke_width,
                    line_cap: canvas::LineCap::Round,
                    ..Stroke::default()
                },
            );
        }

        vec![frame.into_geometry()]
    }
}

/// Ring 110 px with pct and label («Вспомнил») in the centre (§6 finish).
fn done_ring<'a>(pct: f32, value: &'a str, label: &'a str) -> Element<'a, Msg> {
    let p = palette();
    let canvas_el = Canvas::new(DoneRing { pct })
        .width(Length::Fixed(110.0))
        .height(Length::Fixed(110.0));
    let centre = container(
        column![
            txt(value)
                .size(theme::text::H3)
                .font(SERIF_BOLD)
                .color(p.text)
                .align_x(iced::alignment::Horizontal::Center),
            txt(label)
                .size(theme::text::SMALL - 2.0)
                .color(p.muted)
                .align_x(iced::alignment::Horizontal::Center),
        ]
        .align_x(Alignment::Center),
    )
    .width(110.0)
    .height(110.0)
    .align_x(Alignment::Center)
    .align_y(Alignment::Center);
    iced::widget::stack![canvas_el, centre].into()
}
