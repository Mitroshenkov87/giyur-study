//! «Еврейский календарь» screen: three tabs — «Сегодня» (a day in detail),
//! «Месяц» (a Hebrew month grid with a day picker) and «Настройки»
//! (diaspora/Israel, the location used for candle / havdalah / fast times,
//! candle and havdalah offsets).
//!
//! Texts come from `content/calendar/*.json` ([`texts`]); while those files
//! are still being written every block degrades gracefully: whatever has no
//! text is simply omitted. Location is off by default and consent-based
//! ([`prefs`]); coordinates are never logged or shown.

use std::sync::{Arc, Mutex, OnceLock};

use chrono::{Datelike, Duration, NaiveDate, Weekday};
use iced::alignment::Horizontal;
use iced::widget::text::IntoFragment;
use iced::widget::{
    button, column, container, mouse_area, opaque, row, rule, space, stack, text_input, Column, Row,
};
use iced::{Alignment, Background, Border, Color, Element, Length, Shadow, Subscription, Theme};

use crate::calendar::{days, geo, hebrew, parasha, prefs, texts, times};
use crate::content;
use crate::storage::Level;
use crate::ui::app::{Action, Ctx, Route};
use crate::ui::fonts::{HEBREW, SANS, SANS_BOLD, SERIF_BOLD};
use crate::ui::screens::map::{heading_owned, txt};
use crate::ui::theme::{self, Palette};
use crate::ui::widgets;

/// Which tab is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Today,
    Month,
    Settings,
}

/// Process-global «open the calendar on this tab» request (consumed once by
/// the next `Screen::new`); set by the settings screen's «Настройки
/// календаря» button, which cannot pass route arguments.
static OPEN_TAB: OnceLock<Mutex<Option<Tab>>> = OnceLock::new();

/// Ask the calendar screen (opened next) to start on `tab`.
pub fn open_tab(tab: Tab) {
    let slot = OPEN_TAB.get_or_init(|| Mutex::new(None));
    if let Ok(mut g) = slot.lock() {
        *g = Some(tab);
    }
}

fn take_open_tab() -> Option<Tab> {
    OPEN_TAB.get()?.lock().ok().and_then(|mut g| g.take())
}

/// Off-thread result of [`geo::request_approximate`]; polled, never printed.
type GeoResult = Result<(f64, f64), geo::GeoError>;

pub struct Screen {
    tab: Tab,
    /// The selected date (Today tab and the month grid selection).
    date: NaiveDate,
    /// The Hebrew month shown in the «Месяц» tab.
    month_year: i32,
    month: u8,
    prefs: prefs::CalendarPrefs,
    /// The location consent dialog is open.
    consent: bool,
    city_query: String,
    city_results: Vec<geo::City>,
    lat_input: String,
    lon_input: String,
    coord_error: Option<String>,
    /// Result message of the last geolocation attempt (never coordinates).
    geo_note: Option<String>,
    geo_pending: Option<Arc<Mutex<Option<GeoResult>>>>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Tab(Tab),
    PrevDay,
    NextDay,
    Today,
    PickDate(NaiveDate),
    OpenDay(NaiveDate),
    PrevMonth,
    NextMonth,
    Lesson(String),
    SetDiaspora(bool),
    SetCandles(u32),
    SetHavdalah(u32),
    /// «Определить примерно» — open the consent dialog.
    AskGeo,
    /// «Разрешить» in the consent dialog.
    GeoAllow,
    /// «Не сейчас» in the consent dialog.
    GeoNotNow,
    GeoTick,
    CityQuery(String),
    PickCity(usize),
    LatEdit(String),
    LonEdit(String),
    SaveCoords,
    LocationOff,
    /// First-open card: «Выбрать город».
    ConsentCity,
    /// First-open card: «Не нужно».
    ConsentSkip,
    Esc,
}

impl Screen {
    pub fn new(ctx: &mut Ctx, _route: &Route) -> Self {
        let prefs = prefs::CalendarPrefs::load(&ctx.state.settings);
        // Screenshot hooks: GIYUR_CALENDAR_TAB=today|month|settings and
        // GIYUR_CALENDAR_DATE=YYYY-MM-DD (set by main::parse_route).
        let env_tab = match std::env::var("GIYUR_CALENDAR_TAB").as_deref() {
            Ok("month") => Some(Tab::Month),
            Ok("settings") => Some(Tab::Settings),
            Ok("today") => Some(Tab::Today),
            _ => None,
        };
        let tab = take_open_tab().or(env_tab).unwrap_or_default();
        let date = std::env::var("GIYUR_CALENDAR_DATE")
            .ok()
            .and_then(|v| NaiveDate::parse_from_str(&v, "%Y-%m-%d").ok())
            .unwrap_or_else(|| ctx.today());
        let h = hebrew::HebrewDate::from_gregorian(date);
        Screen {
            tab,
            date,
            month_year: h.year,
            month: h.month,
            prefs,
            consent: false,
            city_query: String::new(),
            city_results: Vec::new(),
            lat_input: String::new(),
            lon_input: String::new(),
            coord_error: None,
            geo_note: None,
            geo_pending: None,
        }
    }

    pub fn update(&mut self, ctx: &mut Ctx, msg: Msg) -> Action {
        match msg {
            Msg::Tab(tab) => {
                self.tab = tab;
                if tab == Tab::Month {
                    let h = hebrew::HebrewDate::from_gregorian(self.date);
                    self.month_year = h.year;
                    self.month = h.month;
                }
                Action::None
            }
            Msg::PrevDay => {
                self.date -= Duration::days(1);
                Action::None
            }
            Msg::NextDay => {
                self.date += Duration::days(1);
                Action::None
            }
            Msg::Today => {
                self.date = ctx.today();
                Action::None
            }
            Msg::PickDate(d) => {
                self.date = d;
                Action::None
            }
            Msg::OpenDay(d) => {
                self.date = d;
                self.tab = Tab::Today;
                Action::None
            }
            Msg::PrevMonth => {
                let (y, m) = prev_month(self.month_year, self.month);
                self.month_year = y;
                self.month = m;
                Action::None
            }
            Msg::NextMonth => {
                let (y, m) = next_month(self.month_year, self.month);
                self.month_year = y;
                self.month = m;
                Action::None
            }
            Msg::Lesson(id) => Action::Go(Route::Lesson(id)),
            Msg::SetDiaspora(v) => {
                self.prefs.diaspora = v;
                self.store(ctx);
                Action::None
            }
            Msg::SetCandles(v) => {
                self.prefs.candle_minutes = v;
                self.store(ctx);
                Action::None
            }
            Msg::SetHavdalah(v) => {
                self.prefs.havdalah_minutes = v;
                self.store(ctx);
                Action::None
            }
            Msg::AskGeo => {
                self.consent = true;
                Action::None
            }
            Msg::GeoAllow => {
                self.consent = false;
                let shared: Arc<Mutex<Option<GeoResult>>> = Arc::new(Mutex::new(None));
                self.geo_pending = Some(Arc::clone(&shared));
                // geo::request_approximate is blocking: run off the UI thread
                // and poll the slot with a subscription (no Action::Task).
                std::thread::spawn(move || {
                    let res = geo::request_approximate();
                    if let Ok(mut slot) = shared.lock() {
                        *slot = Some(res);
                    }
                });
                Action::None
            }
            Msg::GeoNotNow => {
                self.consent = false;
                self.prefs.consent_asked = true;
                self.store(ctx);
                Action::None
            }
            Msg::GeoTick => {
                let res = self
                    .geo_pending
                    .as_ref()
                    .and_then(|s| s.lock().ok())
                    .and_then(|mut g| g.take());
                let Some(res) = res else {
                    return Action::None;
                };
                self.geo_pending = None;
                self.prefs.consent_asked = true;
                match res {
                    Ok((lat, lon)) => {
                        self.prefs.location_mode = prefs::LocationMode::System;
                        self.prefs.place = Some(prefs::Place {
                            name: "Примерное местоположение".into(),
                            lat,
                            lon,
                            tz: "local".into(),
                            utc_offset_min: 0,
                        });
                        self.geo_note = None;
                        self.store(ctx);
                        ctx.toast(
                            "Местоположение определено",
                            "Координаты округлены примерно до города.",
                            "✓",
                        );
                    }
                    Err(geo::GeoError::Denied) => {
                        self.geo_note = Some(
                            "Доступ к местоположению запрещён в Windows (Параметры → Конфиденциальность → Расположение). Выберите город вручную."
                                .into(),
                        );
                        self.store(ctx);
                    }
                    Err(geo::GeoError::Unsupported) => {
                        self.geo_note = Some(
                            "На этой системе автоматическое определение недоступно — выберите город или введите координаты."
                                .into(),
                        );
                        self.store(ctx);
                    }
                    Err(geo::GeoError::Unavailable(why)) => {
                        self.geo_note =
                            Some(format!("Не удалось определить местоположение: {why}"));
                        self.store(ctx);
                    }
                }
                Action::None
            }
            Msg::CityQuery(q) => {
                self.city_query = q.clone();
                self.city_results = geo::search(&q).into_iter().take(8).cloned().collect();
                Action::None
            }
            Msg::PickCity(i) => {
                let Some(c) = self.city_results.get(i).cloned() else {
                    return Action::None;
                };
                self.prefs.location_mode = prefs::LocationMode::Manual;
                self.prefs.place = Some(prefs::Place {
                    name: format!("{}, {}", c.name_ru, c.country_ru),
                    lat: c.lat,
                    lon: c.lon,
                    tz: c.tz.clone(),
                    utc_offset_min: c.utc_offset_min,
                });
                self.prefs.consent_asked = true;
                self.geo_note = None;
                self.city_results.clear();
                self.city_query.clear();
                self.store(ctx);
                Action::None
            }
            Msg::LatEdit(v) => {
                self.lat_input = v;
                Action::None
            }
            Msg::LonEdit(v) => {
                self.lon_input = v;
                Action::None
            }
            Msg::SaveCoords => {
                let parse = |s: &str| s.trim().replace(',', ".").parse::<f64>().ok();
                let (Some(lat), Some(lon)) = (parse(&self.lat_input), parse(&self.lon_input))
                else {
                    self.coord_error = Some(
                        "Введите широту и долготу числами, например 31,78 и 35,23 (Иерусалим)."
                            .into(),
                    );
                    return Action::None;
                };
                if !(-90.0..=90.0).contains(&lat) {
                    self.coord_error = Some("Широта должна быть от −90 до 90.".into());
                    return Action::None;
                }
                if !(-180.0..=180.0).contains(&lon) {
                    self.coord_error = Some("Долгота должна быть от −180 до 180.".into());
                    return Action::None;
                }
                self.prefs.location_mode = prefs::LocationMode::Manual;
                self.prefs.place = Some(prefs::Place {
                    name: "Координаты вручную".into(),
                    lat,
                    lon,
                    tz: "local".into(),
                    utc_offset_min: 0,
                });
                self.prefs.consent_asked = true;
                self.coord_error = None;
                self.geo_note = None;
                self.store(ctx);
                Action::None
            }
            Msg::LocationOff => {
                self.prefs.location_mode = prefs::LocationMode::Off;
                self.prefs.place = None;
                self.geo_note = None;
                self.store(ctx);
                Action::None
            }
            Msg::ConsentCity => {
                self.prefs.consent_asked = true;
                self.store(ctx);
                self.tab = Tab::Settings;
                Action::None
            }
            Msg::ConsentSkip => {
                self.prefs.consent_asked = true;
                self.store(ctx);
                Action::None
            }
            Msg::Esc => {
                self.consent = false;
                Action::None
            }
        }
    }

    pub fn subscription(&self) -> Subscription<Msg> {
        let mut subs: Vec<Subscription<Msg>> = Vec::new();
        if self.consent {
            subs.push(iced::keyboard::listen().filter_map(|event| match event {
                iced::keyboard::Event::KeyPressed {
                    key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
                    ..
                } => Some(Msg::Esc),
                _ => None,
            }));
        }
        if self.geo_pending.is_some() {
            subs.push(
                iced::time::every(std::time::Duration::from_millis(300)).map(|_| Msg::GeoTick),
            );
        }
        Subscription::batch(subs)
    }

    pub fn view<'a>(&'a self, ctx: &'a Ctx) -> Element<'a, Msg> {
        let p = palette();
        let base: Element<'a, Msg> = column![
            widgets::page_header("Еврейский календарь", None),
            self.tabs_bar(),
            match self.tab {
                Tab::Today => self.today_tab(ctx, p),
                Tab::Month => self.month_tab(ctx, p),
                Tab::Settings => self.settings_tab(p),
            },
        ]
        .spacing(16)
        .width(Length::Fill)
        .max_width(theme::spacing::CONTENT_MAX)
        .into();
        if self.consent {
            self.with_dialog(base)
        } else {
            base
        }
    }

    // ---- Consent dialog -----------------------------------------------------

    /// The location consent modal (§0.3 style, like settings.rs).
    fn with_dialog<'a>(&'a self, base: Element<'a, Msg>) -> Element<'a, Msg> {
        let p = palette();
        let backdrop = mouse_area(
            container(space::horizontal())
                .width(Length::Fill)
                .height(Length::Fill)
                .style(move |_| plain_container(Some(Color { a: 0.66, ..p.bg0 }))),
        )
        .on_press(Msg::GeoNotNow);

        let allow = widgets::btn(
            "Разрешить",
            widgets::Btn::Primary,
            widgets::BtnSize::Md,
            Some(Msg::GeoAllow),
        );
        let not_now = widgets::btn(
            "Не сейчас",
            widgets::Btn::Ghost,
            widgets::BtnSize::Md,
            Some(Msg::GeoNotNow),
        );

        let modal = container(
            column![
                widgets::heading("Примерное местоположение", 3),
                txt("Шорашим может определить ваше примерное местоположение (с точностью до города), чтобы считать время зажигания свечей, авдалы и постов. Координаты округляются примерно до 10 км, хранятся только на этом компьютере и никуда не отправляются. Windows может спросить разрешение на доступ к местоположению.")
                    .size(theme::text::SMALL + 1.0)
                    .color(p.muted)
                    .width(Length::Fill),
                row![space::horizontal().width(Length::Fill), not_now, allow]
                    .spacing(10)
                    .align_y(Alignment::Center),
            ]
            .spacing(14),
        )
        .padding(24.0)
        .max_width(460.0)
        .style(move |_| container::Style {
            background: Some(Background::Color(p.surface)),
            border: Border {
                color: p.line2,
                width: 1.0,
                radius: theme::radius::MODAL.into(),
            },
            shadow: Shadow {
                color: alpha(Color::BLACK, 0.4),
                offset: iced::Vector::new(0.0, 10.0),
                blur_radius: 30.0,
            },
            text_color: None,
            snap: true,
        });

        stack![
            base,
            opaque(backdrop),
            container(modal).center(Length::Fill).padding(20.0),
        ]
        .into()
    }

    fn tabs_bar<'a>(&self) -> Element<'a, Msg> {
        seg_row(&[
            ("Сегодня", self.tab == Tab::Today, Msg::Tab(Tab::Today)),
            ("Месяц", self.tab == Tab::Month, Msg::Tab(Tab::Month)),
            (
                "Настройки",
                self.tab == Tab::Settings,
                Msg::Tab(Tab::Settings),
            ),
        ])
    }

    // ---- «Сегодня» ----------------------------------------------------------

    fn today_tab<'a>(&'a self, ctx: &'a Ctx, p: Palette) -> Element<'a, Msg> {
        let date = self.date;
        let info = days::day_info(date, self.prefs.diaspora);
        let t = texts::Texts::get();

        let mut col = Column::new().spacing(16).width(Length::Fill);
        if !self.prefs.consent_asked {
            col = col.push(self.consent_prompt(p));
        }
        col = col.push(self.day_nav(date, ctx.today(), p));
        col = col.push(self.hero(&info, p));
        col = col.push(self.what_day(&info, t, p));
        for el in self.day_text_blocks(ctx, &info, t, p) {
            col = col.push(el);
        }
        col = col.push(self.parasha_card(date, ctx, t, p));
        col = col.push(self.times_card(date, p));
        if let Some(card) = self.nearest_card(date, p) {
            col = col.push(card);
        }
        col.into()
    }

    /// Gentle first-open prompt about location-based times (not a modal).
    fn consent_prompt<'a>(&'a self, p: Palette) -> Element<'a, Msg> {
        widgets::card(
            column![
                widgets::heading("Показывать время свечей и постов?", 3),
                txt("Для расчёта времени нужно примерное местоположение. Оно хранится только на этом компьютере.")
                    .size(theme::text::SMALL + 0.5)
                    .color(p.muted)
                    .width(Length::Fill),
                row![
                    btn_primary("Определить примерно", Msg::AskGeo),
                    btn_secondary("Выбрать город", Msg::ConsentCity),
                    btn_ghost("Не нужно", Msg::ConsentSkip),
                ]
                .spacing(10)
                .wrap(),
            ]
            .spacing(12),
        )
    }

    fn day_nav<'a>(&'a self, date: NaiveDate, today: NaiveDate, p: Palette) -> Element<'a, Msg> {
        let today_btn = if date == today {
            btn_ghost("Сегодня", Msg::Today)
        } else {
            btn_secondary("Сегодня", Msg::Today)
        };
        row![
            nav_arrow("‹", Msg::PrevDay),
            column![
                txt(hebrew::format_gregorian_ru(date))
                    .size(theme::text::H3)
                    .font(SERIF_BOLD)
                    .color(p.text),
                txt(hebrew::HebrewDate::from_gregorian(date).format_ru())
                    .size(theme::text::SMALL)
                    .color(p.muted),
            ]
            .spacing(2)
            .align_x(Horizontal::Center)
            .width(Length::Fill),
            today_btn,
            nav_arrow("›", Msg::NextDay),
        ]
        .spacing(10)
        .align_y(Alignment::Center)
        .into()
    }

    fn hero<'a>(&'a self, info: &days::DayInfo, p: Palette) -> Element<'a, Msg> {
        let d = info.date;
        container(
            column![
                txt(format!(
                    "{}, {}",
                    hebrew::weekday_ru(d),
                    hebrew::format_gregorian_ru(d)
                ))
                .size(theme::text::H2)
                .font(SERIF_BOLD)
                .color(p.text)
                .width(Length::Fill),
                txt(info.hebrew.format_ru())
                    .size(theme::text::H3)
                    .color(p.gold2)
                    .width(Length::Fill),
                // RTL Hebrew paragraph: cosmic-text lays it out flush right;
                // keep Left + Fill (see widgets::hebrew_block).
                txt(info.hebrew.format_he())
                    .size(28.0)
                    .font(HEBREW)
                    .color(p.hebrew)
                    .align_x(Horizontal::Left)
                    .width(Length::Fill),
            ]
            .spacing(8)
            .width(Length::Fill),
        )
        .padding(24.0)
        .width(Length::Fill)
        .style(move |_| theme::hero(&palette()))
        .into()
    }

    fn what_day<'a>(
        &'a self,
        info: &days::DayInfo,
        t: &'a texts::Texts,
        p: Palette,
    ) -> Element<'a, Msg> {
        let mut col = Column::new().spacing(12).width(Length::Fill);
        col = col.push(widgets::heading("Что за день", 3));

        let mut chips = Row::new().spacing(8);
        for ev in &info.events {
            let (accent, _) = kind_tint(&p, ev.kind);
            chips = chips.push(event_chip(ev.title.clone(), accent));
        }
        col = col.push(chips.wrap());

        if !info.events.iter().any(|e| e.is_notable()) {
            col = col.push(
                txt("Будний день")
                    .size(theme::text::BODY)
                    .font(SANS_BOLD)
                    .color(p.text),
            );
            if let Some(w) = t.day("weekday").filter(|w| !w.meaning.is_empty()) {
                col = col.push(widgets::rich_paragraph(&w.meaning));
            }
        }

        match info.fast() {
            Some(f) => {
                let note = t
                    .day(f.id)
                    .and_then(|d| d.fast_note.as_deref())
                    .filter(|s| !s.is_empty());
                col = col.push(banner(
                    format!("Сегодня пост: {}", f.title),
                    note,
                    p.bad,
                    p.bad_soft,
                ));
            }
            None => {
                col = col.push(
                    txt("Сегодня поста нет")
                        .size(theme::text::SMALL)
                        .color(p.muted),
                );
                let tomorrow = days::day_info(info.date + Duration::days(1), info.diaspora);
                let major = tomorrow.fast().filter(|f| f.kind == days::Kind::MajorFast);
                if let Some(f) = major {
                    col = col.push(
                        txt(format!(
                            "Завтра пост — {} — начинается сегодня вечером на закате",
                            f.title
                        ))
                        .size(theme::text::SMALL + 0.5)
                        .color(p.flame)
                        .width(Length::Fill),
                    );
                }
            }
        }

        if let Some(n) = info.omer {
            col = col.push(
                column![
                    txt(format!("Счёт омера: сегодня вечером — {} омера", nth(n as u32)))
                        .size(theme::text::BODY)
                        .font(SANS_BOLD)
                        .color(p.gold2)
                        .width(Length::Fill),
                    txt(format!(
                        "{} пути от Песаха до Шавуота. Счёт произносят на закате, который начинает этот день.",
                        omer_breakdown(n)
                    ))
                    .size(theme::text::SMALL)
                    .color(p.muted)
                    .width(Length::Fill),
                ]
                .spacing(4),
            );
        }
        widgets::card(col)
    }

    /// One card per event that has a text in [`texts`]; missing texts are
    /// skipped (content is written in parallel with this screen).
    fn day_text_blocks<'a>(
        &'a self,
        ctx: &'a Ctx,
        info: &days::DayInfo,
        t: &'a texts::Texts,
        p: Palette,
    ) -> Vec<Element<'a, Msg>> {
        let user = content::Library::user_level(&ctx.state);
        let mut out = Vec::new();
        let mut seen: Vec<&str> = Vec::new();
        for ev in &info.events {
            if seen.contains(&ev.id) {
                continue;
            }
            seen.push(ev.id);
            let Some(d) = t.day(ev.id) else { continue };
            if d.meaning.is_empty()
                && d.customs.is_empty()
                && d.idea.is_empty()
                && d.fast_note.is_none()
                && d.movements.is_none()
                && d.lessons.is_empty()
            {
                continue;
            }
            let (accent, _) = kind_tint(&p, ev.kind);
            let mut col = Column::new().spacing(10).width(Length::Fill);
            col = col.push(
                row![
                    dot(accent, 8.0),
                    txt(ev.title.clone())
                        .size(theme::text::H3)
                        .font(SERIF_BOLD)
                        .color(p.text),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            );
            if !d.meaning.is_empty() {
                col = col.push(block_label("Смысл", p));
                col = col.push(widgets::rich_paragraph(&d.meaning));
            }
            if !d.customs.is_empty() {
                col = col.push(block_label("Обычаи", p));
                for c in &d.customs {
                    col = col.push(
                        row![
                            txt("•").size(theme::text::BODY).color(accent),
                            widgets::rich_paragraph(c),
                        ]
                        .spacing(8)
                        .align_y(Alignment::Start),
                    );
                }
            }
            if !d.idea.is_empty() {
                col = col.push(block_label("Мысль дня", p));
                col = col.push(widgets::rich_paragraph(&d.idea));
            }
            if let Some(mv) = &d.movements {
                for (level, body) in [
                    (Level::Reform, mv.reform.as_deref()),
                    (Level::Conservative, mv.conservative.as_deref()),
                    (Level::Orthodox, mv.orthodox.as_deref()),
                ] {
                    let Some(body) = body.filter(|s| !s.is_empty()) else {
                        continue;
                    };
                    let label = content::level_info(level).differs_label;
                    let note: Element<Msg> = widgets::differs_note(label, body);
                    // Highlight the user's own movement with a gold frame.
                    let note = if level == user {
                        container(note)
                            .style(move |_| container::Style {
                                border: Border {
                                    color: p.gold,
                                    width: 1.0,
                                    radius: theme::radius::SMALL.into(),
                                },
                                ..plain_container(None)
                            })
                            .into()
                    } else {
                        note
                    };
                    col = col.push(note);
                }
            }
            let mut pushed_label = false;
            for id in &d.lessons {
                if let Some(el) = lesson_link(ctx, id) {
                    if !pushed_label {
                        col = col.push(block_label("Уроки в приложении", p));
                        pushed_label = true;
                    }
                    col = col.push(el);
                }
            }
            out.push(widgets::card(col));
        }
        out
    }

    fn parasha_card<'a>(
        &'a self,
        date: NaiveDate,
        ctx: &'a Ctx,
        t: &'a texts::Texts,
        p: Palette,
    ) -> Element<'a, Msg> {
        let diaspora = self.prefs.diaspora;
        let (sat, reading, next) = parasha::weekly(date, diaspora);
        let mut col = Column::new().spacing(12).width(Length::Fill);
        col = col.push(widgets::heading("Недельная глава", 3));
        col = col.push(
            txt(format!("Шаббат, {}", hebrew::format_gregorian_ru(sat)))
                .size(theme::text::SMALL)
                .color(p.muted),
        );
        if reading == parasha::Reading::Festival {
            col = col.push(
                txt("В этот Шаббат читают праздничный отрывок")
                    .size(theme::text::BODY)
                    .color(p.text)
                    .width(Length::Fill),
            );
            if let Some((next_sat, next_reading)) = next {
                col = col.push(
                    txt(format!(
                        "Следующая регулярная глава: {} — {}",
                        next_reading.name_ru(),
                        hebrew::format_gregorian_ru(next_sat)
                    ))
                    .size(theme::text::SMALL + 0.5)
                    .color(p.muted)
                    .width(Length::Fill),
                );
            }
        } else {
            let mut title = row![txt(reading.name_ru())
                .size(theme::text::H3)
                .font(SERIF_BOLD)
                .color(p.text)]
            .spacing(10)
            .align_y(Alignment::Center);
            let he_name = reading.name_he();
            if !he_name.is_empty() {
                title = title.push(txt(he_name).size(24.0).font(HEBREW).color(p.hebrew));
            }
            col = col.push(title);
            for idx in reading.indices() {
                if let Some(pt) = t.parasha(idx) {
                    if !pt.reference.is_empty() {
                        col = col.push(
                            txt(pt.reference.as_str())
                                .size(theme::text::SMALL)
                                .color(p.gold)
                                .width(Length::Fill),
                        );
                    }
                    if !pt.summary.is_empty() {
                        col = col.push(widgets::rich_paragraph(&pt.summary));
                    }
                    if !pt.theme.is_empty() {
                        col = col.push(
                            txt(format!("Тема: {}", pt.theme))
                                .size(theme::text::SMALL + 0.5)
                                .color(p.muted)
                                .width(Length::Fill),
                        );
                    }
                }
            }
            // Daily idea; on a notable day the day's own idea (shown above)
            // takes precedence.
            let info = days::day_info(date, diaspora);
            let day_idea = info
                .events
                .iter()
                .filter(|e| e.is_notable())
                .find_map(|e| t.day(e.id))
                .map(|d| d.idea.as_str())
                .filter(|s| !s.is_empty());
            if day_idea.is_none() {
                let wd = date.weekday().num_days_from_sunday() as usize;
                for idx in reading.indices() {
                    let idea = t
                        .parasha(idx)
                        .and_then(|pt| pt.daily.get(wd))
                        .filter(|s| !s.is_empty());
                    if let Some(idea) = idea {
                        col = col.push(block_label("Тема дня из Торы", p));
                        col = col.push(widgets::rich_paragraph(idea));
                        break;
                    }
                }
            }
            for idx in reading.indices() {
                if let Some(pt) = t.parasha(idx) {
                    let mut pushed_label = false;
                    for id in &pt.lessons {
                        if let Some(el) = lesson_link(ctx, id) {
                            if !pushed_label {
                                col = col.push(block_label("Уроки в приложении", p));
                                pushed_label = true;
                            }
                            col = col.push(el);
                        }
                    }
                }
            }
        }
        widgets::card(col)
    }

    fn times_card<'a>(&'a self, date: NaiveDate, p: Palette) -> Element<'a, Msg> {
        let mut col = Column::new().spacing(12).width(Length::Fill);
        col = col.push(widgets::heading("Время свечей и постов", 3));
        match self.prefs.active_place() {
            None => {
                col = col.push(
                    txt("Чтобы показывать время зажигания свечей, авдалы и начала и конца постов, нужно примерное местоположение — город или координаты.")
                        .size(theme::text::SMALL + 0.5)
                        .color(p.muted)
                        .width(Length::Fill),
                );
                col = col.push(btn_secondary("Включить", Msg::AskGeo));
            }
            Some(place) => {
                col = col.push(
                    txt(format!("Место: {}", place.name))
                        .size(theme::text::SMALL)
                        .color(p.muted)
                        .width(Length::Fill),
                );
                for it in times::for_day(date, &self.prefs) {
                    let color = if it.minor { p.muted } else { p.text };
                    col = col.push(
                        row![
                            txt(it.label)
                                .size(theme::text::SMALL + 0.5)
                                .color(color)
                                .width(Length::Fill),
                            txt(it.time.format("%H:%M").to_string())
                                .size(theme::text::SMALL + 0.5)
                                .font(SANS_BOLD)
                                .color(color),
                        ]
                        .spacing(10),
                    );
                }
                col = col.push(
                    txt(times::CAVEAT)
                        .size(theme::text::SMALL - 1.0)
                        .color(p.muted)
                        .width(Length::Fill),
                );
            }
        }
        widgets::card(col)
    }

    fn nearest_card<'a>(&'a self, date: NaiveDate, p: Palette) -> Option<Element<'a, Msg>> {
        let (d, ev) = days::next_holiday_or_fast(date, self.prefs.diaspora)?;
        let n = (d - date).num_days();
        let h = hebrew::HebrewDate::from_gregorian(d);
        Some(widgets::card(
            column![
                widgets::heading("Ближайшее", 3),
                txt(format!("До {}: {}", ev.title, plural_days(n)))
                    .size(theme::text::BODY)
                    .font(SANS_BOLD)
                    .color(p.gold2)
                    .width(Length::Fill),
                txt(format!(
                    "{} — {}",
                    hebrew::format_gregorian_ru(d),
                    h.format_ru()
                ))
                .size(theme::text::SMALL)
                .color(p.muted)
                .width(Length::Fill),
            ]
            .spacing(8),
        ))
    }

    // ---- «Месяц» ------------------------------------------------------------

    fn month_tab<'a>(&'a self, ctx: &'a Ctx, p: Palette) -> Element<'a, Msg> {
        let y = self.month_year;
        let m = self.month;
        let diaspora = self.prefs.diaspora;
        let today = ctx.today();
        let t = texts::Texts::get();

        let mut col = Column::new().spacing(16).width(Length::Fill);

        let he_title = format!(
            "{} {}",
            hebrew::month_name_he(y, m),
            hebrew::gematria((y % 1000) as u32)
        );
        col = col.push(
            row![
                nav_arrow("‹", Msg::PrevMonth),
                column![
                    heading_owned(format!("{} {}", hebrew::month_name_ru(y, m), y), 2),
                    txt(he_title)
                        .size(theme::text::H3)
                        .font(HEBREW)
                        .color(p.hebrew),
                ]
                .spacing(4)
                .align_x(Horizontal::Center)
                .width(Length::Fill),
                nav_arrow("›", Msg::NextMonth),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        );

        // Weekday header, Sunday first; Shabbat highlighted.
        let mut head = Row::new().spacing(4);
        for (i, w) in ["Вс", "Пн", "Вт", "Ср", "Чт", "Пт", "Сб"]
            .iter()
            .enumerate()
        {
            head = head.push(
                txt(*w)
                    .size(theme::text::SMALL)
                    .font(if i == 6 { SANS_BOLD } else { SANS })
                    .color(if i == 6 { p.gold2 } else { p.muted })
                    .width(Length::Fill)
                    .align_x(Horizontal::Center),
            );
        }
        col = col.push(head);

        let blanks = leading_blanks(y, m);
        let dim = hebrew::days_in_month(y, m) as usize;
        let total = blanks + dim;
        let rows = total.div_ceil(7);
        let mut grid = Column::new().spacing(4);
        for r in 0..rows {
            let mut line = Row::new().spacing(4);
            for c in 0..7 {
                let idx = r * 7 + c;
                if idx < blanks || idx >= total {
                    line = line.push(
                        container(space::horizontal())
                            .width(Length::Fill)
                            .height(Length::Fixed(64.0)),
                    );
                } else {
                    let day = (idx - blanks + 1) as u8;
                    let date = hebrew::HebrewDate::new(y, m, day).to_gregorian();
                    line = line.push(self.month_cell(date, today, diaspora, c == 6, p));
                }
            }
            grid = grid.push(line);
        }
        col = col.push(grid);

        let mut legend = Row::new().spacing(14);
        for (color, label) in [
            (p.gold2, "Шаббат"),
            (p.gold, "Йом тов"),
            (p.flame, "Холь а-моэд / канун"),
            (p.bad, "Пост"),
            (p.info, "Рош Ходеш"),
            (p.violet, "Малый праздник или особый день"),
        ] {
            legend = legend.push(
                row![
                    dot(color, 8.0),
                    txt(label).size(theme::text::SMALL).color(p.muted)
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            );
        }
        col = col.push(legend.wrap());

        col = col.push(self.month_selected_card(p));
        col = col.push(self.month_events_card(y, m, diaspora, p));
        if let Some(mt) = t.month(y, m).filter(|mt| !mt.idea.is_empty()) {
            col = col.push(widgets::card(
                column![
                    widgets::heading("Об этом месяце", 3),
                    widgets::rich_paragraph(&mt.idea),
                ]
                .spacing(10),
            ));
        }
        col.into()
    }

    fn month_cell<'a>(
        &'a self,
        date: NaiveDate,
        today: NaiveDate,
        diaspora: bool,
        shabbat_col: bool,
        p: Palette,
    ) -> Element<'a, Msg> {
        let info = days::day_info(date, diaspora);
        let is_today = date == today;
        let is_selected = date == self.date;

        let num_color = if is_today { p.gold2 } else { p.text };
        let greg = if date.day() == 1 {
            format!("1 {}", GREG_MONTHS_ABBR[date.month0() as usize])
        } else {
            date.day().to_string()
        };

        let mut dots = Row::new().spacing(3).height(Length::Fixed(6.0));
        let mut seen: Vec<Color> = Vec::new();
        for ev in &info.events {
            let (accent, _) = kind_tint(&p, ev.kind);
            if !seen.contains(&accent) {
                seen.push(accent);
                dots = dots.push(dot(accent, 6.0));
            }
        }

        button(
            column![
                row![
                    txt(hebrew::gematria(info.hebrew.day as u32))
                        .size(16.0)
                        .font(SANS_BOLD)
                        .color(num_color),
                    space::horizontal().width(Length::Fill),
                    txt(greg).size(11.0).color(p.muted),
                ],
                space::vertical().height(Length::Fill),
                dots,
            ]
            .spacing(2)
            .width(Length::Fill)
            .height(Length::Fixed(52.0)),
        )
        .padding(6.0)
        .width(Length::Fill)
        .height(Length::Fixed(64.0))
        .style(move |_, status| cell_style(&p, is_today, is_selected, shabbat_col, status))
        .on_press(Msg::PickDate(date))
        .into()
    }

    /// Compact details of the selected date, under the month grid.
    fn month_selected_card<'a>(&'a self, p: Palette) -> Element<'a, Msg> {
        let date = self.date;
        let diaspora = self.prefs.diaspora;
        let info = days::day_info(date, diaspora);
        let mut col = Column::new().spacing(10).width(Length::Fill);
        col = col.push(heading_owned(
            format!(
                "{} — {}",
                hebrew::weekday_ru(date),
                hebrew::format_gregorian_ru(date)
            ),
            3,
        ));
        for ev in &info.events {
            let (accent, _) = kind_tint(&p, ev.kind);
            col = col.push(
                row![
                    dot(accent, 8.0),
                    txt(ev.title.clone())
                        .size(theme::text::SMALL + 0.5)
                        .color(p.text),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }
        if let Some(f) = info.fast() {
            col = col.push(
                txt(format!("Пост: {}", f.title))
                    .size(theme::text::SMALL + 0.5)
                    .font(SANS_BOLD)
                    .color(p.bad),
            );
        }
        if date.weekday() == Weekday::Sat {
            let (_, reading, _) = parasha::weekly(date, diaspora);
            col = col.push(
                txt(format!("Глава: {}", reading.name_ru()))
                    .size(theme::text::SMALL + 0.5)
                    .color(p.gold2),
            );
        }
        if self.prefs.active_place().is_some() {
            for it in times::for_day(date, &self.prefs) {
                let color = if it.minor { p.muted } else { p.text };
                col = col.push(
                    row![
                        txt(it.label)
                            .size(theme::text::SMALL)
                            .color(color)
                            .width(Length::Fill),
                        txt(it.time.format("%H:%M").to_string())
                            .size(theme::text::SMALL)
                            .font(SANS_BOLD)
                            .color(color),
                    ]
                    .spacing(10),
                );
            }
        }
        col = col.push(widgets::btn(
            "Открыть день",
            widgets::Btn::Primary,
            widgets::BtnSize::Md,
            Some(Msg::OpenDay(date)),
        ));
        widgets::card(col)
    }

    fn month_events_card<'a>(
        &'a self,
        y: i32,
        m: u8,
        diaspora: bool,
        p: Palette,
    ) -> Element<'a, Msg> {
        let mut col = Column::new().spacing(10).width(Length::Fill);
        col = col.push(widgets::heading("Праздники и посты этого месяца", 3));
        let mut seen: Vec<&str> = Vec::new();
        let mut any = false;
        for day in 1..=hebrew::days_in_month(y, m) {
            let date = hebrew::HebrewDate::new(y, m, day).to_gregorian();
            let info = days::day_info(date, diaspora);
            for ev in info.events {
                if !ev.is_notable() || seen.contains(&ev.id) {
                    continue;
                }
                seen.push(ev.id);
                any = true;
                let (accent, _) = kind_tint(&p, ev.kind);
                col = col.push(
                    row![
                        txt(format!(
                            "{} {}",
                            date.day(),
                            GREG_MONTHS_ABBR[date.month0() as usize]
                        ))
                        .size(theme::text::SMALL)
                        .color(p.muted)
                        .width(Length::Fixed(56.0)),
                        dot(accent, 8.0),
                        txt(ev.title).size(theme::text::SMALL + 0.5).color(p.text),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                );
            }
        }
        if !any {
            col = col.push(
                txt("В этом месяце нет праздников и постов.")
                    .size(theme::text::SMALL)
                    .color(p.muted),
            );
        }
        widgets::card(col)
    }

    // ---- «Настройки» --------------------------------------------------------

    fn settings_tab<'a>(&'a self, p: Palette) -> Element<'a, Msg> {
        let mut col = Column::new().spacing(16).width(Length::Fill);
        col = col.push(self.where_card(p));
        col = col.push(self.location_card(p));
        col = col.push(self.candles_card(p));
        col = col.push(
            txt("Всё хранится только на этом компьютере.")
                .size(theme::text::SMALL)
                .color(p.muted),
        );
        col.into()
    }

    fn where_card<'a>(&'a self, p: Palette) -> Element<'a, Msg> {
        widgets::card(
            column![
                widgets::heading("Где вы соблюдаете праздники", 3),
                txt("В диаспоре многие праздники длятся два дня, а порядок недельных глав иной, чем в Израиле.")
                    .size(theme::text::SMALL)
                    .color(p.muted)
                    .width(Length::Fill),
                seg_row(&[
                    ("Диаспора", self.prefs.diaspora, Msg::SetDiaspora(true)),
                    ("Израиль", !self.prefs.diaspora, Msg::SetDiaspora(false)),
                ]),
            ]
            .spacing(12),
        )
    }

    fn location_card<'a>(&'a self, p: Palette) -> Element<'a, Msg> {
        let mode_label = mode_title(self.prefs.location_mode);
        let status = match &self.prefs.place {
            Some(pl) => format!("{mode_label}: {}", pl.name),
            None => mode_label.to_string(),
        };
        let mut col = Column::new().spacing(12).width(Length::Fill);
        col = col.push(widgets::heading("Местоположение для времени", 3));
        col = col.push(
            txt(status)
                .size(theme::text::SMALL + 0.5)
                .color(p.text)
                .width(Length::Fill),
        );
        col = col.push(
            txt("Нужно, чтобы считать время зажигания свечей, авдалы и постов.")
                .size(theme::text::SMALL)
                .color(p.muted)
                .width(Length::Fill),
        );
        col = col.push(
            row![
                btn_secondary("Определить примерно", Msg::AskGeo),
                btn_danger("Выключить местоположение", Msg::LocationOff),
            ]
            .spacing(10)
            .wrap(),
        );
        if let Some(note) = &self.geo_note {
            col = col.push(widgets::note_block("Местоположение", note));
        }

        col = col.push(divider(p));
        col = col.push(
            txt("Город из встроенного списка")
                .size(theme::text::BODY)
                .font(SANS_BOLD),
        );
        col = col.push(
            text_input("Найти город", &self.city_query)
                .on_input(Msg::CityQuery)
                .padding(12)
                .style(move |_t, s| theme::text_input(&palette(), s)),
        );
        for (i, c) in self.city_results.iter().enumerate() {
            col = col.push(
                widgets::btn(
                    format!("{}, {}", c.name_ru, c.country_ru),
                    widgets::Btn::Secondary,
                    widgets::BtnSize::Sm,
                    Some(Msg::PickCity(i)),
                )
                .width(Length::Fill),
            );
        }

        col = col.push(divider(p));
        col = col.push(
            txt("Или введите координаты вручную")
                .size(theme::text::BODY)
                .font(SANS_BOLD),
        );
        col = col.push(
            row![
                text_input("Широта, например 31,78", &self.lat_input)
                    .on_input(Msg::LatEdit)
                    .padding(12)
                    .width(Length::Fill)
                    .style(move |_t, s| theme::text_input(&palette(), s)),
                text_input("Долгота, например 35,23", &self.lon_input)
                    .on_input(Msg::LonEdit)
                    .padding(12)
                    .width(Length::Fill)
                    .style(move |_t, s| theme::text_input(&palette(), s)),
            ]
            .spacing(10),
        );
        if let Some(err) = &self.coord_error {
            col = col.push(
                txt(err.as_str())
                    .size(theme::text::SMALL)
                    .color(p.bad)
                    .width(Length::Fill),
            );
        }
        col = col.push(btn_secondary("Сохранить", Msg::SaveCoords));
        widgets::card(col)
    }

    fn candles_card<'a>(&'a self, p: Palette) -> Element<'a, Msg> {
        let mut candles = Row::new().spacing(6);
        for v in [18, 20, 22, 30, 40] {
            candles = candles.push(seg_button(
                format!("{v}"),
                self.prefs.candle_minutes == v,
                Msg::SetCandles(v),
            ));
        }
        let mut havdalah = Row::new().spacing(6);
        for (label, v) in [
            ("по выходу звёзд", 0),
            ("42 минуты", 42),
            ("50 минут", 50),
            ("72 минуты", 72),
        ] {
            havdalah = havdalah.push(seg_button(
                label,
                self.prefs.havdalah_minutes == v,
                Msg::SetHavdalah(v),
            ));
        }
        widgets::card(
            column![
                widgets::heading("Свечи", 3),
                txt("Зажигание свечей — за сколько минут до заката")
                    .size(theme::text::BODY)
                    .font(SANS_BOLD),
                candles,
                txt("В Иерусалиме принято зажигать свечи за 40 минут до заката.")
                    .size(theme::text::SMALL)
                    .color(p.muted)
                    .width(Length::Fill),
                divider(p),
                txt("Авдала — выход Шаббата и праздников")
                    .size(theme::text::BODY)
                    .font(SANS_BOLD),
                havdalah,
            ]
            .spacing(12),
        )
    }

    fn store(&self, ctx: &mut Ctx) {
        self.prefs.store(&mut ctx.state.settings);
        ctx.save();
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn palette() -> Palette {
    theme::current()
}

fn alpha(color: Color, a: f32) -> Color {
    Color { a, ..color }
}

fn plain_container(background: Option<Color>) -> container::Style {
    container::Style {
        text_color: None,
        background: background.map(Background::Color),
        border: Border::default(),
        shadow: Shadow::default(),
        snap: true,
    }
}

/// Hairline separator.
fn divider<'a>(p: Palette) -> Element<'a, Msg> {
    rule::horizontal(1)
        .style(move |_theme: &Theme| iced::widget::rule::Style {
            color: p.line,
            radius: 0.0.into(),
            fill_mode: iced::widget::rule::FillMode::Full,
            snap: true,
        })
        .into()
}

/// Small muted uppercase-ish block label.
fn block_label<'a>(label: &'a str, p: Palette) -> Element<'a, Msg> {
    txt(label)
        .size(theme::text::SMALL)
        .font(SANS_BOLD)
        .color(p.muted)
        .into()
}

/// (accent, soft tint) per event kind — the calendar's colour language.
fn kind_tint(p: &Palette, k: days::Kind) -> (Color, Color) {
    match k {
        days::Kind::MajorFast | days::Kind::MinorFast => (p.bad, p.bad_soft),
        days::Kind::YomTov => (p.gold, p.gold_soft),
        days::Kind::CholHamoed | days::Kind::Erev => (p.flame, p.flame_soft),
        days::Kind::RoshChodesh => (p.info, p.info_soft),
        days::Kind::Minor | days::Kind::Modern | days::Kind::SpecialShabbat => {
            (p.violet, p.violet_soft)
        }
        days::Kind::Shabbat => (p.gold2, p.gold_soft),
        days::Kind::Period => (p.muted, p.surface3),
    }
}

/// A rounded colour dot.
fn dot<'a>(color: Color, size: f32) -> Element<'a, Msg> {
    container(space::horizontal())
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .style(move |_| container::Style {
            background: Some(Background::Color(color)),
            border: Border {
                radius: theme::radius::PILL.into(),
                ..Border::default()
            },
            ..plain_container(None)
        })
        .into()
}

/// Event chip: the shared tinted pill in the kind's accent colour.
fn event_chip<'a>(title: impl IntoFragment<'a>, accent: Color) -> Element<'a, Msg> {
    widgets::pill(title, widgets::Pill::Tint(accent))
}

/// Soft banner (fast warning, note).
fn banner<'a>(
    title: String,
    body: Option<&'a str>,
    accent: Color,
    soft: Color,
) -> Element<'a, Msg> {
    let mut c = Column::new().spacing(6).width(Length::Fill);
    c = c.push(
        txt(title)
            .size(theme::text::BODY)
            .font(SANS_BOLD)
            .color(accent)
            .width(Length::Fill),
    );
    if let Some(body) = body {
        c = c.push(
            txt(body)
                .size(theme::text::SMALL + 0.5)
                .color(accent)
                .width(Length::Fill),
        );
    }
    container(c)
        .padding([12.0, 16.0])
        .width(Length::Fill)
        .style(move |_| container::Style {
            background: Some(Background::Color(soft)),
            border: Border {
                radius: theme::radius::SMALL.into(),
                ..Border::default()
            },
            ..plain_container(None)
        })
        .into()
}

/// Button to a lesson that exists in the library; None otherwise.
fn lesson_link<'a>(ctx: &'a Ctx, id: &'a str) -> Option<Element<'a, Msg>> {
    let (lesson, _) = ctx.lib.lesson(id)?;
    let title = lesson.title.as_deref().unwrap_or(id);
    Some(
        widgets::btn(
            format!("{title} →"),
            widgets::Btn::Secondary,
            widgets::BtnSize::Sm,
            Some(Msg::Lesson(id.to_string())),
        )
        .into(),
    )
}

fn nav_arrow<'a>(label: &'static str, msg: Msg) -> Element<'a, Msg> {
    widgets::btn(
        label,
        widgets::Btn::Secondary,
        widgets::BtnSize::Sm,
        Some(msg),
    )
    .into()
}

fn btn_primary<'a>(label: &'static str, msg: Msg) -> Element<'a, Msg> {
    widgets::btn(
        label,
        widgets::Btn::Primary,
        widgets::BtnSize::Md,
        Some(msg),
    )
    .into()
}

fn btn_secondary<'a>(label: &'static str, msg: Msg) -> Element<'a, Msg> {
    widgets::btn(
        label,
        widgets::Btn::Secondary,
        widgets::BtnSize::Md,
        Some(msg),
    )
    .into()
}

fn btn_ghost<'a>(label: &'static str, msg: Msg) -> Element<'a, Msg> {
    widgets::btn(label, widgets::Btn::Ghost, widgets::BtnSize::Md, Some(msg)).into()
}

fn btn_danger<'a>(label: &'static str, msg: Msg) -> Element<'a, Msg> {
    widgets::btn(label, widgets::Btn::Danger, widgets::BtnSize::Md, Some(msg)).into()
}

/// A row of segmented buttons (selected = gold-soft, gold2 text, gold border).
fn seg_row<'a>(items: &[(&'static str, bool, Msg)]) -> Element<'a, Msg> {
    let mut r = Row::new().spacing(4);
    for (label, selected, msg) in items {
        r = r.push(seg_button(*label, *selected, msg.clone()));
    }
    r.into()
}

fn seg_button<'a>(label: impl IntoFragment<'a>, selected: bool, msg: Msg) -> Element<'a, Msg> {
    widgets::choice(label, selected, false, msg)
}

/// Month grid cell style: today outlined in gold, the selected day gold-soft,
/// the Shabbat column slightly tinted.
fn cell_style(
    p: &Palette,
    today: bool,
    selected: bool,
    shabbat_col: bool,
    status: button::Status,
) -> button::Style {
    let background = if selected {
        p.gold_soft
    } else if today {
        p.surface3
    } else if shabbat_col {
        alpha(p.gold, 0.05)
    } else {
        p.surface2
    };
    let base = button::Style {
        background: Some(Background::Color(background)),
        text_color: p.text,
        border: Border {
            color: if today || selected { p.gold } else { p.line },
            width: 1.0,
            radius: theme::radius::SMALL.into(),
        },
        shadow: Shadow::default(),
        snap: true,
    };
    match status {
        button::Status::Hovered | button::Status::Pressed if !selected && !today => button::Style {
            background: Some(Background::Color(p.surface3)),
            ..base
        },
        _ => base,
    }
}

fn mode_title(mode: prefs::LocationMode) -> &'static str {
    match mode {
        prefs::LocationMode::Off => "Выключено",
        prefs::LocationMode::System => "Примерное местоположение системы",
        prefs::LocationMode::Manual => "Город или координаты",
    }
}

/// Russian plural: `plural(3, "день", "дня", "дней")` → «3 дня».
fn plural(n: i64, one: &str, few: &str, many: &str) -> String {
    let rem100 = n.unsigned_abs() % 100;
    let word = if rem100 % 10 == 1 && rem100 != 11 {
        one
    } else if (2..=4).contains(&(rem100 % 10)) && !(12..=14).contains(&rem100) {
        few
    } else {
        many
    };
    format!("{n} {word}")
}

fn plural_days(n: i64) -> String {
    plural(n, "день", "дня", "дней")
}

fn plural_weeks(n: i64) -> String {
    plural(n, "неделя", "недели", "недель")
}

/// «3-й день».
fn nth(n: u32) -> String {
    format!("{n}-й день")
}

/// «1 неделя и 5 дней» (zero parts omitted).
fn omer_breakdown(n: u8) -> String {
    let w = n as i64 / 7;
    let d = n as i64 % 7;
    match (w, d) {
        (0, _) => plural_days(d),
        (_, 0) => plural_weeks(w),
        _ => format!("{} и {}", plural_weeks(w), plural_days(d)),
    }
}

/// The Hebrew month after `m` (Elul → Tishrei of the next year; Adar /
/// Adar II → Nisan of the same year).
fn next_month(y: i32, m: u8) -> (i32, u8) {
    if m == hebrew::ELUL {
        (y + 1, hebrew::TISHREI)
    } else if m == hebrew::last_month(y) {
        (y, hebrew::NISAN)
    } else {
        (y, m + 1)
    }
}

/// The Hebrew month before `m` (Tishrei → Elul of the previous year; Nisan →
/// Adar / Adar II of the same year).
fn prev_month(y: i32, m: u8) -> (i32, u8) {
    if m == hebrew::TISHREI {
        (y - 1, hebrew::ELUL)
    } else if m == hebrew::NISAN {
        (y, hebrew::last_month(y))
    } else {
        (y, m - 1)
    }
}

/// Blank cells before the 1st of the Hebrew month in a Sunday-first grid.
fn leading_blanks(year: i32, month: u8) -> usize {
    hebrew::HebrewDate::new(year, month, 1)
        .to_gregorian()
        .weekday()
        .num_days_from_sunday() as usize
}

const GREG_MONTHS_ABBR: [&str; 12] = [
    "янв", "фев", "мар", "апр", "май", "июн", "июл", "авг", "сен", "окт", "ноя", "дек",
];

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::hebrew::{self, HebrewDate};

    #[test]
    fn plural_forms() {
        assert_eq!(plural_days(1), "1 день");
        assert_eq!(plural_days(2), "2 дня");
        assert_eq!(plural_days(4), "4 дня");
        assert_eq!(plural_days(5), "5 дней");
        assert_eq!(plural_days(11), "11 дней");
        assert_eq!(plural_days(12), "12 дней");
        assert_eq!(plural_days(14), "14 дней");
        assert_eq!(plural_days(21), "21 день");
        assert_eq!(plural_days(22), "22 дня");
        assert_eq!(plural_days(101), "101 день");
        assert_eq!(plural_days(111), "111 дней");
        assert_eq!(plural_weeks(1), "1 неделя");
        assert_eq!(plural_weeks(2), "2 недели");
        assert_eq!(plural_weeks(5), "5 недель");
    }

    #[test]
    fn omer_breakdown_omits_zero_parts() {
        assert_eq!(omer_breakdown(5), "5 дней");
        assert_eq!(omer_breakdown(7), "1 неделя");
        assert_eq!(omer_breakdown(12), "1 неделя и 5 дней");
        assert_eq!(omer_breakdown(49), "7 недель");
    }

    #[test]
    fn month_navigation_wraps_the_year() {
        // 5786 — обычный год, 5787 — високосный (Адар I + Адар II).
        assert!(!hebrew::is_leap(5786));
        assert!(hebrew::is_leap(5787));
        assert_eq!(next_month(5787, hebrew::ELUL), (5788, hebrew::TISHREI));
        assert_eq!(prev_month(5788, hebrew::TISHREI), (5787, hebrew::ELUL));
        assert_eq!(next_month(5787, hebrew::TISHREI), (5787, hebrew::CHESHVAN));
        // Обычный год: Адар → Нисан того же года.
        assert_eq!(next_month(5786, hebrew::ADAR), (5786, hebrew::NISAN));
        // Високосный: Адар I → Адар II → Нисан, и обратно.
        assert_eq!(next_month(5787, hebrew::ADAR), (5787, hebrew::ADAR_II));
        assert_eq!(next_month(5787, hebrew::ADAR_II), (5787, hebrew::NISAN));
        assert_eq!(prev_month(5787, hebrew::NISAN), (5787, hebrew::ADAR_II));
        assert_eq!(prev_month(5786, hebrew::NISAN), (5786, hebrew::ADAR));
        assert_eq!(prev_month(5787, hebrew::TISHREI), (5786, hebrew::ELUL));
    }

    #[test]
    fn grid_leading_blanks_are_sunday_first() {
        // 1 тишрея 5787 — 12 сентября 2026, суббота → 6 пустых клеток.
        assert_eq!(
            HebrewDate::new(5787, hebrew::TISHREI, 1)
                .to_gregorian()
                .weekday(),
            Weekday::Sat
        );
        assert_eq!(leading_blanks(5787, hebrew::TISHREI), 6);
        // 1 тишрея 5786 — 23 сентября 2025, вторник → 2 пустые клетки.
        assert_eq!(
            HebrewDate::new(5786, hebrew::TISHREI, 1)
                .to_gregorian()
                .weekday(),
            Weekday::Tue
        );
        assert_eq!(leading_blanks(5786, hebrew::TISHREI), 2);
    }
}
