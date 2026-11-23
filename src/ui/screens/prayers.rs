//! «Молитвы»: daily prayers and blessings (src/prayers.rs) in Hebrew, Russian
//! transliteration and Russian translation. The list groups them by time of
//! day (утро, день, вечер, еда, шаббат, особые случаи) with a «Избранное»
//! filter; one prayer opens as a reading page with «when / how / why», the
//! «До гиюра» note and the source line. What to show (all / Russian /
//! Hebrew / transliteration) and the text size are kept between runs.

use iced::alignment::Horizontal;
use iced::widget::{button, column, container, row, Column, Row};
use iced::{Alignment, Element, Length};

use crate::prayers::{self, Display, Prayer, Prefs};
use crate::ui::app::{Action, Ctx, Route};
use crate::ui::fonts::{HEBREW, SANS, SANS_BOLD};
use crate::ui::screens::map::{heading_owned, icons, txt};
use crate::ui::{theme, widgets};

/// Base sizes of the prayer text before the reader's size step.
const HE_SIZE: f32 = 27.0;
const TR_SIZE: f32 = 16.5;
const RU_SIZE: f32 = 17.0;

/// Which prayers the list shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Filter {
    All,
    Favorites,
    Group(String),
}

pub struct Screen {
    /// The prayer being read (None = the list).
    open: Option<String>,
    filter: Filter,
    prefs: Prefs,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Open(String),
    /// Back to the list.
    Close,
    Filter(Filter),
    SetDisplay(Display),
    Bigger,
    Smaller,
    ToggleFavorite(String),
}

impl Screen {
    pub fn new(ctx: &mut Ctx, route: &Route) -> Self {
        let open = match route {
            Route::Prayers(Some(id)) if prayers::book().prayer(id).is_some() => Some(id.clone()),
            _ => None,
        };
        Screen {
            open,
            filter: Filter::All,
            prefs: Prefs::load(&ctx.state),
        }
    }

    pub fn subscription(&self) -> iced::Subscription<Msg> {
        iced::Subscription::none()
    }

    /// The prayer on screen, if one is open.
    pub fn open_id(&self) -> Option<&str> {
        self.open.as_deref()
    }

    pub fn update(&mut self, ctx: &mut Ctx, msg: Msg) -> Action {
        match msg {
            // Each prayer is its own route: the shell scrolls to the top and
            // «Назад» returns to where the reader came from.
            Msg::Open(id) => {
                if prayers::book().prayer(&id).is_some() {
                    return Action::Go(Route::Prayers(Some(id)));
                }
            }
            Msg::Close => return Action::Go(Route::Prayers(None)),
            Msg::Filter(f) => self.filter = f,
            Msg::SetDisplay(d) => {
                self.prefs.display = d;
                self.save(ctx);
            }
            Msg::Bigger => {
                self.prefs.bigger();
                self.save(ctx);
            }
            Msg::Smaller => {
                self.prefs.smaller();
                self.save(ctx);
            }
            Msg::ToggleFavorite(id) => {
                self.prefs.toggle_favorite(&id);
                self.save(ctx);
            }
        }
        Action::None
    }

    fn save(&self, ctx: &mut Ctx) {
        self.prefs.store(&mut ctx.state);
        ctx.save();
    }

    pub fn view<'a>(&'a self, _ctx: &'a Ctx) -> Element<'a, Msg> {
        let book = prayers::book();
        match self.open.as_deref().and_then(|id| book.prayer(id)) {
            Some(p) => self.view_prayer(p),
            None => self.view_list(),
        }
    }

    // ------------------------------------------------------------- list

    fn view_list(&self) -> Element<'_, Msg> {
        let p = theme::current();
        let book = prayers::book();
        let mut page = Column::new().spacing(14).width(Length::Fill);
        page = page.push(widgets::heading("Молитвы на каждый день", 1));
        page = page.push(
            txt("Утренние и вечерние молитвы и благословения: иврит с огласовками, чтение русскими буквами и перевод. Если вы пока не читаете на иврите, молитесь по-русски — это разрешено.")
                .size(theme::text::BODY)
                .color(p.muted)
                .width(Length::Fill),
        );
        for n in &book.notes {
            page = page.push(note(n.title.as_str(), n.text.as_str(), n.rabbi));
        }
        page = page.push(widgets::card(self.controls()));

        // Filter chips.
        let fav_count = self.prefs.favorites.len();
        let mut chips = Row::new().spacing(6);
        chips = chips.push(filter_chip(
            "Все",
            self.filter == Filter::All,
            Msg::Filter(Filter::All),
        ));
        chips = chips.push(filter_chip(
            format!("★ Избранное ({fav_count})"),
            self.filter == Filter::Favorites,
            Msg::Filter(Filter::Favorites),
        ));
        for g in &book.groups {
            let f = Filter::Group(g.id.clone());
            chips = chips.push(filter_chip(
                g.title.as_str(),
                self.filter == f,
                Msg::Filter(f),
            ));
        }
        page = page.push(chips.wrap().vertical_spacing(6));

        match &self.filter {
            Filter::Favorites => {
                let mut list = Column::new().spacing(6).width(Length::Fill);
                list = list.push(heading_owned("Избранное".into(), 3));
                if self.prefs.favorites.is_empty() {
                    list = list.push(
                        txt("Пока пусто. Нажмите ☆ у молитвы, чтобы она появилась здесь.")
                            .size(theme::text::SMALL + 1.0)
                            .color(p.muted),
                    );
                }
                for id in &self.prefs.favorites {
                    if let Some(pr) = book.prayer(id) {
                        list = list.push(self.prayer_row(pr));
                    }
                }
                page = page.push(widgets::card(list));
            }
            f => {
                for g in &book.groups {
                    if let Filter::Group(only) = f {
                        if only != &g.id {
                            continue;
                        }
                    }
                    let mut list = Column::new().spacing(6).width(Length::Fill);
                    list = list.push(
                        row![
                            heading_owned(g.title.clone(), 3),
                            txt(g.hint.as_str()).size(theme::text::SMALL).color(p.muted),
                        ]
                        .spacing(10)
                        .align_y(Alignment::Center),
                    );
                    for pr in book.in_group(&g.id) {
                        list = list.push(self.prayer_row(pr));
                    }
                    page = page.push(widgets::card(list));
                }
            }
        }
        page = page.push(sources_line());
        page.into()
    }

    fn prayer_row<'a>(&'a self, pr: &'a Prayer) -> Element<'a, Msg> {
        let p = theme::current();
        let text_col = column![
            txt(pr.title.as_str())
                .size(theme::text::BODY)
                .font(SANS_BOLD)
                .color(p.text),
            txt(pr.subtitle.as_str())
                .size(theme::text::SMALL)
                .color(p.muted),
        ]
        .spacing(2)
        .width(Length::Fill);
        let he = txt(pr.he_title.as_str())
            .size(20.0)
            .font(HEBREW)
            .color(p.hebrew);
        let open = button(
            row![text_col, he]
                .spacing(12)
                .align_y(Alignment::Center)
                .width(Length::Fill),
        )
        .width(Length::Fill)
        .padding([8.0, 10.0])
        .style(move |_, s| theme::button_ghost(&theme::current(), s))
        .on_press(Msg::Open(pr.id.clone()));
        row![open, self.star(&pr.id, false)]
            .spacing(6)
            .align_y(Alignment::Center)
            .width(Length::Fill)
            .into()
    }

    /// ☆ / ★ toggle; `long` adds the words («В избранное»).
    fn star<'a>(&self, id: &str, long: bool) -> Element<'a, Msg> {
        let p = theme::current();
        let on = self.prefs.is_favorite(id);
        let label = match (on, long) {
            (true, true) => "★ В избранном",
            (false, true) => "☆ В избранное",
            (true, false) => "★",
            (false, false) => "☆",
        };
        button(
            txt(label)
                .size(if long { theme::text::SMALL + 1.0 } else { 22.0 })
                .color(if on { p.gold2 } else { p.muted }),
        )
        .padding([4.0, 10.0])
        .style(move |_, s| theme::button_ghost(&theme::current(), s))
        .on_press(Msg::ToggleFavorite(id.to_string()))
        .into()
    }

    /// «Показывать» segments and the «Размер текста» stepper.
    fn controls(&self) -> Element<'_, Msg> {
        let p = theme::current();
        let sel = Display::ALL.iter().position(|d| *d == self.prefs.display);
        let seg = widgets::segmented(
            Display::ALL
                .iter()
                .map(|d| (d.label().to_string(), Msg::SetDisplay(*d)))
                .collect(),
            sel,
        );
        let pct = (self.prefs.scale() * 100.0).round() as i32;
        let size = row![
            txt("Размер текста").size(theme::text::SMALL).color(p.muted),
            widgets::btn(
                "A−",
                widgets::Btn::Secondary,
                widgets::BtnSize::Sm,
                (self.prefs.size > 0).then_some(Msg::Smaller),
            ),
            txt(format!("{pct}%"))
                .size(theme::text::SMALL)
                .font(SANS_BOLD)
                .color(p.text),
            widgets::btn(
                "A+",
                widgets::Btn::Secondary,
                widgets::BtnSize::Sm,
                (self.prefs.size + 1 < prayers::SIZES.len()).then_some(Msg::Bigger),
            ),
        ]
        .spacing(10)
        .align_y(Alignment::Center);
        column![
            txt("Показывать").size(theme::text::SMALL).color(p.muted),
            seg,
            size,
        ]
        .spacing(8)
        .width(Length::Fill)
        .into()
    }

    // ------------------------------------------------------------- one prayer

    fn view_prayer<'a>(&'a self, pr: &'a Prayer) -> Element<'a, Msg> {
        let p = theme::current();
        let book = prayers::book();
        let mut page = Column::new().spacing(14).width(Length::Fill);
        page = page.push(widgets::back_button("Все молитвы", Msg::Close));

        let group = pr
            .groups
            .first()
            .and_then(|g| book.group(g))
            .map(|g| g.title.as_str())
            .unwrap_or("");
        let mut head = Column::new().spacing(4).width(Length::Fill);
        head = head.push(widgets::kicker(group));
        head = head.push(
            row![
                container(widgets::heading(pr.title.as_str(), 1)).width(Length::Fill),
                txt(pr.he_title.as_str())
                    .size(30.0)
                    .font(HEBREW)
                    .color(p.hebrew),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        );
        head = head.push(
            txt(pr.subtitle.as_str())
                .size(theme::text::BODY)
                .color(p.muted),
        );
        page = page.push(
            row![head, self.star(&pr.id, true)]
                .spacing(10)
                .align_y(Alignment::Start),
        );

        page = page.push(widgets::card(
            column![
                info("Когда", pr.when.as_str()),
                info("Как", pr.how.as_str()),
            ]
            .spacing(12),
        ));
        page = page.push(widgets::card(self.controls()));

        let scale = self.prefs.scale();
        let d = self.prefs.display;
        for part in &pr.parts {
            let mut col = Column::new().spacing(10).width(Length::Fill);
            if let Some(l) = &part.label {
                col = col.push(
                    txt(l.as_str())
                        .size(theme::text::SMALL + 1.0)
                        .font(SANS_BOLD)
                        .color(p.gold2),
                );
            }
            if d.shows_he() {
                col = col.push(
                    container(
                        txt(part.he.as_str())
                            .size(HE_SIZE * scale)
                            .font(HEBREW)
                            .color(p.hebrew)
                            .line_height(1.55)
                            // cosmic-text lays an RTL paragraph out flush
                            // right; Left + Fill keeps it there (see
                            // widgets::hebrew_block).
                            .align_x(Horizontal::Left)
                            .width(Length::Fill),
                    )
                    .padding([12.0, 16.0])
                    .width(Length::Fill)
                    .style(move |_| theme::hebrew_block(&theme::current())),
                );
            }
            if d.shows_translit() {
                col = col.push(
                    txt(part.translit.as_str())
                        .size(TR_SIZE * scale)
                        .font(italic(SANS))
                        .color(if d == Display::Translit {
                            p.text
                        } else {
                            p.muted
                        })
                        .line_height(1.45)
                        .width(Length::Fill),
                );
            }
            if d.shows_ru() {
                col = col.push(
                    txt(part.ru.as_str())
                        .size(RU_SIZE * scale)
                        .color(p.text)
                        .line_height(1.45)
                        .width(Length::Fill),
                );
            }
            page = page.push(widgets::card(col));
        }

        page = page.push(widgets::card(
            column![
                info("О молитве", pr.about.as_str()),
                txt(pr.r#ref.as_str())
                    .size(theme::text::SMALL)
                    .color(p.muted),
            ]
            .spacing(8),
        ));
        page = page.push(note("До гиюра", pr.candidate.as_str(), true));
        page = page.push(
            txt(format!(
                "Иврит: {}. Перевод и транслитерация — «Шорашим». Нусах {}.",
                pr.source, book.nusach
            ))
            .size(theme::text::SMALL)
            .color(p.muted)
            .width(Length::Fill),
        );

        let (prev, next) = book.neighbours(&pr.id);
        let mut nav = Row::new().spacing(10).width(Length::Fill);
        if let Some(prev) = prev {
            nav = nav.push(widgets::btn(
                format!("← {}", prev.title),
                widgets::Btn::Secondary,
                widgets::BtnSize::Md,
                Some(Msg::Open(prev.id.clone())),
            ));
        }
        nav = nav.push(iced::widget::space().width(Length::Fill));
        if let Some(next) = next {
            nav = nav.push(widgets::btn(
                format!("{} →", next.title),
                widgets::Btn::Secondary,
                widgets::BtnSize::Md,
                Some(Msg::Open(next.id.clone())),
            ));
        }
        page = page.push(nav);
        page.into()
    }
}

/// A filter chip as wide as its label (`widgets::choice` fills its row).
fn filter_chip<'a>(
    label: impl iced::widget::text::IntoFragment<'a>,
    selected: bool,
    msg: Msg,
) -> Element<'a, Msg> {
    let font = if selected { SANS_BOLD } else { SANS };
    button(container(txt(label).size(theme::text::SMALL).font(font)).center_y(Length::Fill))
        .height(theme::size::BTN_SM)
        .padding([0.0, 16.0])
        .style(move |_, status| theme::choice(&theme::current(), selected, true, status))
        .on_press(msg)
        .into()
}

fn italic(font: iced::Font) -> iced::Font {
    iced::Font {
        style: iced::font::Style::Italic,
        ..font
    }
}

/// Small uppercase label + body text.
fn info<'a>(label: &'a str, body: &'a str) -> Element<'a, Msg> {
    let p = theme::current();
    column![
        txt(label.to_uppercase())
            .size(theme::text::KICKER)
            .font(SANS_BOLD)
            .color(p.gold2),
        txt(body)
            .size(theme::text::BODY)
            .color(p.text)
            .width(Length::Fill),
    ]
    .spacing(4)
    .width(Length::Fill)
    .into()
}

/// A note box; `rabbi` adds the «Уточните у раввина» tag.
fn note<'a>(title: &'a str, body: &'a str, rabbi: bool) -> Element<'a, Msg> {
    let p = theme::current();
    let mut head = Row::new().spacing(10).align_y(Alignment::Center);
    if rabbi {
        head = head.push(icons::icon(icons::Kind::Shield, 16.0, p.info));
    }
    head = head.push(
        txt(title.to_uppercase())
            .size(theme::text::KICKER)
            .font(SANS_BOLD)
            .color(p.info),
    );
    if rabbi {
        head = head.push(widgets::chip("Уточните у раввина"));
    }
    container(
        column![
            head,
            txt(body)
                .size(theme::text::BODY)
                .color(p.text)
                .width(Length::Fill)
        ]
        .spacing(8)
        .width(Length::Fill),
    )
    .padding([14.0, 16.0])
    .width(Length::Fill)
    .style(move |_| theme::note(&theme::current()))
    .into()
}

fn sources_line<'a>() -> Element<'a, Msg> {
    let p = theme::current();
    let book = prayers::book();
    let list: Vec<String> = book
        .sources
        .iter()
        .map(|s| format!("{} ({})", s.title, s.license))
        .collect();
    txt(format!(
        "Источники: {}. Нусах {}. {}.",
        list.join("; "),
        book.nusach,
        if book.review_status.is_empty() {
            "Требует проверки раввином"
        } else {
            book.review_status.as_str()
        }
    ))
    .size(theme::text::SMALL)
    .color(p.muted)
    .width(Length::Fill)
    .into()
}
