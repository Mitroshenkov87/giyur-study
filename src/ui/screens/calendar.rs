//! «Еврейский календарь» screen: three tabs — «Сегодня» (a day in detail),
//! «Месяц» (a Hebrew month grid with a day picker) and «Настройки»
//! (diaspora/Israel, the location used for candle / havdalah / fast times,
//! and compact reference rules).
//!
//! Texts come from `content/ru/calendar/*.json` ([`texts`]); while those files
//! are still being written every block degrades gracefully: whatever has no
//! text is simply omitted. Location is off by default and uses the OS permission flow
//! ([`prefs`]); coordinates are never logged or shown.

use std::sync::{Arc, Mutex, OnceLock};

use chrono::{Datelike, Duration, NaiveDate, Weekday};
use iced::alignment::Horizontal;
use iced::widget::text::IntoFragment;
use iced::widget::{button, column, container, row, rule, space, text_input, Column, Row};
use iced::{Alignment, Background, Border, Color, Element, Length, Shadow, Subscription, Theme};

use crate::calendar::{days, geo, hebrew, parasha, prefs, texts, times};
use crate::content;
use crate::i18n::tr;
use crate::storage::Level;
use crate::ui::app::{Action, Ctx, Route};
use crate::ui::fonts::{HEBREW, SANS, SANS_BOLD, SERIF_BOLD};
use crate::ui::screens::map::{heading_owned, txt};
use crate::ui::theme::{self, Palette};
use crate::ui::widgets;
use crate::util::plural_l;

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
type GeoResult = geo::GeoResult;

pub struct Screen {
    tab: Tab,
    /// The selected date (Today tab and the month grid selection).
    date: NaiveDate,
    /// The Hebrew month shown in the «Месяц» tab.
    month_year: i32,
    month: u8,
    prefs: prefs::CalendarPrefs,
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
    /// Start the native Windows permission request from the UI thread.
    AskGeo,
    /// Open Windows Settings → Privacy & security → Location.
    OpenLocationSettings,
    GeoTick,
    CityQuery(String),
    PickCity(usize),
    LatEdit(String),
    LonEdit(String),
    SaveCoords,
    LocationOff,
}

impl Screen {
    pub fn new(ctx: &mut Ctx, _route: &Route) -> Self {
        let prefs = prefs::CalendarPrefs::load(&ctx.state.settings);
        // Screenshot hooks: SHORASHIM_CALENDAR_TAB=today|month|settings and
        // SHORASHIM_CALENDAR_DATE=YYYY-MM-DD (set by main::parse_route).
        let env_tab = match crate::util::env_hook("CALENDAR_TAB").as_deref() {
            Some("month") => Some(Tab::Month),
            Some("settings") => Some(Tab::Settings),
            Some("today") => Some(Tab::Today),
            _ => None,
        };
        let tab = take_open_tab().or(env_tab).unwrap_or_default();
        let date = crate::util::env_hook("CALENDAR_DATE")
            .and_then(|v| NaiveDate::parse_from_str(&v, "%Y-%m-%d").ok())
            .unwrap_or_else(|| ctx.today());
        let h = hebrew::HebrewDate::from_gregorian(date);
        Screen {
            tab,
            date,
            month_year: h.year,
            month: h.month,
            prefs,
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
            Msg::AskGeo => {
                if self.geo_pending.is_some() {
                    return Action::None;
                }
                self.geo_note = None;
                self.prefs.consent_asked = true;
                self.store(ctx);
                let shared: Arc<Mutex<Option<GeoResult>>> = Arc::new(Mutex::new(None));
                self.geo_pending = Some(Arc::clone(&shared));
                // Initiate RequestAccessAsync here, on the foreground UI thread.
                // Only waiting for access and acquiring a fix happen on a worker.
                geo::request_approximate(move |res| {
                    if let Ok(mut slot) = shared.lock() {
                        *slot = Some(res);
                    }
                });
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
                if res.is_err() {
                    self.tab = Tab::Settings;
                }
                match res {
                    Ok((lat, lon)) => {
                        self.prefs.location_mode = prefs::LocationMode::System;
                        self.prefs.place = Some(prefs::Place {
                            // Russian on purpose: translated in display_place_name.
                            name: "Примерное местоположение".into(),
                            lat,
                            lon,
                            tz: "local".into(),
                            utc_offset_min: 0,
                        });
                        self.prefs.apply_place_customs();
                        self.geo_note = None;
                        self.store(ctx);
                        ctx.toast(
                            tr("Местоположение определено"),
                            tr("Координаты округлены примерно до города."),
                            "✓",
                        );
                    }
                    Err(geo::GeoError::Denied) => {
                        self.geo_note = Some(
                            tr("Windows не разрешила доступ. Его можно изменить в Параметры → Конфиденциальность и безопасность → Расположение, затем запросить снова. Или выберите город ниже. Сохранённое место не меняется.")
                                .to_string(),
                        );
                        self.store(ctx);
                    }
                    Err(geo::GeoError::Unsupported) => {
                        self.geo_note = Some(
                            tr("На этой системе автоматическое определение недоступно — выберите город или введите координаты.")
                                .to_string(),
                        );
                        self.store(ctx);
                    }
                    Err(geo::GeoError::Unavailable(why)) => {
                        self.geo_note = Some(trf!(
                            "Не удалось определить местоположение: {why}. Выберите город ниже. Сохранённое место не меняется.",
                            "Could not determine the location: {why}. Choose a city below. The saved place is unchanged."
                        ));
                        self.store(ctx);
                    }
                }
                Action::None
            }
            Msg::OpenLocationSettings => {
                #[cfg(windows)]
                {
                    let _spawned = std::process::Command::new("explorer.exe")
                        .arg("ms-settings:privacy-location")
                        .spawn();
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
                self.geo_pending = None;
                self.prefs.location_mode = prefs::LocationMode::Manual;
                self.prefs.place = Some(prefs::Place {
                    name: format!("{}, {}", c.name_ru, c.country_ru),
                    lat: c.lat,
                    lon: c.lon,
                    tz: c.tz.clone(),
                    utc_offset_min: c.utc_offset_min,
                });
                self.prefs.consent_asked = true;
                self.prefs.apply_place_customs();
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
                        tr("Введите широту и долготу числами, например 31,78 и 35,23 (Иерусалим).")
                            .to_string(),
                    );
                    return Action::None;
                };
                if !(-90.0..=90.0).contains(&lat) {
                    self.coord_error = Some(tr("Широта должна быть от −90 до 90.").to_string());
                    return Action::None;
                }
                if !(-180.0..=180.0).contains(&lon) {
                    self.coord_error = Some(tr("Долгота должна быть от −180 до 180.").to_string());
                    return Action::None;
                }
                self.geo_pending = None;
                self.prefs.location_mode = prefs::LocationMode::Manual;
                self.prefs.place = Some(prefs::Place {
                    // Russian on purpose: translated in display_place_name.
                    name: "Координаты вручную".into(),
                    lat,
                    lon,
                    tz: "local".into(),
                    utc_offset_min: 0,
                });
                self.prefs.consent_asked = true;
                self.prefs.apply_place_customs();
                self.coord_error = None;
                self.geo_note = None;
                self.store(ctx);
                Action::None
            }
            Msg::LocationOff => {
                self.geo_pending = None;
                self.prefs.location_mode = prefs::LocationMode::Off;
                self.prefs.place = None;
                self.geo_note = None;
                self.store(ctx);
                Action::None
            }
        }
    }

    pub fn subscription(&self) -> Subscription<Msg> {
        let mut subs: Vec<Subscription<Msg>> = Vec::new();
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
            widgets::page_header(tr("Еврейский календарь"), None),
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
        base
    }

    // Kept for the common screen interface: Windows owns the permission UI.
    pub fn dialog<'a>(&'a self, _ctx: &'a Ctx) -> Option<Element<'a, Msg>> {
        None
    }

    fn tabs_bar<'a>(&self) -> Element<'a, Msg> {
        // «Месяц» is also the 30-day streak badge (tr → "One month"). The tab
        // says "Month", so its English stays in trf! and out of the catalog.
        let month = trf!("Месяц", "Month");
        row![
            seg_button(tr("Сегодня"), self.tab == Tab::Today, Msg::Tab(Tab::Today)),
            seg_button(month, self.tab == Tab::Month, Msg::Tab(Tab::Month)),
            seg_button(
                tr("Настройки"),
                self.tab == Tab::Settings,
                Msg::Tab(Tab::Settings),
            ),
        ]
        .spacing(4)
        .into()
    }

    // ---- «Сегодня» ----------------------------------------------------------

    fn today_tab<'a>(&'a self, ctx: &'a Ctx, p: Palette) -> Element<'a, Msg> {
        let date = self.date;
        let info = days::day_info(date, self.prefs.diaspora);
        let t = texts::Texts::get();

        let mut col = Column::new().spacing(16).width(Length::Fill);
        col = col.push(self.day_nav(date, ctx.today(), p));
        col = col.push(self.hero(&info, p));
        col = col.push(self.what_day(&info, t, p));
        col = col.push(self.times_card(date, p));
        for el in self.day_text_blocks(ctx, &info, t, p) {
            col = col.push(el);
        }
        col = col.push(self.parasha_card(date, ctx, t, p));
        if let Some(card) = self.nearest_card(date, p) {
            col = col.push(card);
        }
        col.into()
    }

    fn day_nav<'a>(&'a self, date: NaiveDate, today: NaiveDate, p: Palette) -> Element<'a, Msg> {
        let today_btn = if date == today {
            btn_ghost(tr("Сегодня"), Msg::Today)
        } else {
            btn_secondary(tr("Сегодня"), Msg::Today)
        };
        row![
            nav_arrow("‹", Msg::PrevDay),
            column![
                txt(format_gregorian(date))
                    .size(theme::text::H3)
                    .font(SERIF_BOLD)
                    .color(p.text),
                txt(format_hebrew_date(hebrew::HebrewDate::from_gregorian(date)))
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
                txt(weekday_and_gregorian(d))
                    .size(theme::text::H2)
                    .font(SERIF_BOLD)
                    .color(p.text)
                    .width(Length::Fill),
                txt(format_hebrew_date(info.hebrew))
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
        col = col.push(widgets::heading(tr("Что за день"), 3));

        for ev in &info.events {
            let (accent, _) = kind_tint(&p, ev.kind);
            col = col.push(
                txt(ev.title.clone())
                    .size(theme::text::BODY)
                    .font(SANS_BOLD)
                    .color(accent)
                    .width(Length::Fill),
            );
        }

        if !info.events.iter().any(|e| e.is_notable()) {
            col = col.push(
                txt(tr("Будний день"))
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
                let title = f.title.as_str();
                col = col.push(banner(
                    trf!("Сегодня пост: {title}", "Fast today: {title}"),
                    note,
                    p.bad,
                    p.bad_soft,
                ));
            }
            None => {
                col = col.push(
                    txt(tr("Сегодня поста нет"))
                        .size(theme::text::SMALL)
                        .color(p.muted),
                );
                let tomorrow = days::day_info(info.date + Duration::days(1), info.diaspora);
                let major = tomorrow.fast().filter(|f| f.kind == days::Kind::MajorFast);
                if let Some(f) = major {
                    let title = f.title.as_str();
                    col = col.push(
                        txt(trf!(
                            "Завтра пост — {title} — начинается сегодня вечером на закате",
                            "Tomorrow is a fast — {title} — it begins this evening at sunset"
                        ))
                        .size(theme::text::SMALL + 0.5)
                        .color(p.flame)
                        .width(Length::Fill),
                    );
                }
            }
        }

        if let Some(n) = info.omer {
            let path = omer_breakdown(n);
            col = col.push(
                column![
                    txt(trf!(
                        "Счёт омера: сегодня вечером — {n}-й день омера",
                        "Omer count: tonight is day {n} of the Omer"
                    ))
                        .size(theme::text::BODY)
                        .font(SANS_BOLD)
                        .color(p.gold2)
                        .width(Length::Fill),
                    txt(trf!(
                        "{path} пути от Песаха до Шавуота. Счёт произносят на закате, который начинает этот день.",
                        "{path} of the journey from Pesach to Shavuot. The count is said at sunset, which begins the day."
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
                col = col.push(block_label(tr("Смысл"), p));
                col = col.push(widgets::rich_paragraph(&d.meaning));
            }
            if !d.customs.is_empty() {
                col = col.push(block_label(tr("Обычаи"), p));
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
                col = col.push(block_label(tr("Мысль дня"), p));
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
                        col = col.push(block_label(tr("Уроки в приложении"), p));
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
        col = col.push(widgets::heading(tr("Недельная глава"), 3));
        let sat_s = format_gregorian(sat);
        col = col.push(
            txt(trf!("Шаббат, {date}", "Shabbat, {date}", date = sat_s))
                .size(theme::text::SMALL)
                .color(p.muted),
        );
        if reading == parasha::Reading::Festival {
            col = col.push(
                txt(tr("В этот Шаббат читают праздничный отрывок"))
                    .size(theme::text::BODY)
                    .color(p.text)
                    .width(Length::Fill),
            );
            if let Some((next_sat, next_reading)) = next {
                let name = reading_name(next_reading);
                let date = format_gregorian(next_sat);
                col = col.push(
                    txt(trf!(
                        "Следующая регулярная глава: {name} — {date}",
                        "Next regular portion: {name} — {date}"
                    ))
                    .size(theme::text::SMALL + 0.5)
                    .color(p.muted)
                    .width(Length::Fill),
                );
            }
        } else {
            let mut title = row![txt(reading_name(reading))
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
                            txt({
                                let theme = pt.theme.as_str();
                                trf!("Тема: {theme}", "Theme: {theme}")
                            })
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
                        col = col.push(block_label(tr("Тема дня из Торы"), p));
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
                                col = col.push(block_label(tr("Уроки в приложении"), p));
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
        col = col.push(widgets::heading(tr("Время свечей и постов"), 3));
        match self.prefs.active_place() {
            None => {
                col = col.push(
                    txt(tr("Чтобы показывать время зажигания свечей, авдалы и начала и конца постов, нужно примерное местоположение — город или координаты."))
                        .size(theme::text::SMALL + 0.5)
                        .color(p.muted)
                        .width(Length::Fill),
                );
                col = col.push(
                    txt(tr("Укажите место один раз на вкладке «Настройки» — время будет показано автоматически."))
                        .size(theme::text::SMALL)
                        .color(p.gold2)
                        .width(Length::Fill),
                );
            }
            Some(place) => {
                let name = display_place_name(&place.name);
                col = col.push(
                    txt(trf!("Место: {name}", "Place: {name}"))
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
                                .size(if it.minor {
                                    theme::text::SMALL + 0.5
                                } else {
                                    theme::text::H3
                                })
                                .font(SANS_BOLD)
                                .color(if it.minor { p.muted } else { p.gold2 }),
                        ]
                        .spacing(10),
                    );
                }
                col = col.push(
                    txt(tr(times::CAVEAT))
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
                widgets::heading(tr("Ближайшее"), 3),
                txt({
                    let title = ev.title.as_str();
                    let when = plural_days(n);
                    trf!("До {title}: {when}", "{when} until {title}")
                })
                .size(theme::text::BODY)
                .font(SANS_BOLD)
                .color(p.gold2)
                .width(Length::Fill),
                txt(format!(
                    "{} — {}",
                    format_gregorian(d),
                    format_hebrew_date(h)
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
                    heading_owned(hebrew_month_heading(y, m), 2),
                    txt(he_title)
                        .size(theme::text::H3)
                        .font(HEBREW)
                        .color(p.hebrew),
                    txt(gregorian_span(y, m))
                        .size(theme::text::BODY)
                        .font(SANS_BOLD)
                        .color(p.text),
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
                txt(tr(*w))
                    .size(theme::text::SMALL)
                    .font(if i == 6 { SANS_BOLD } else { SANS })
                    .color(if i == 6 { p.gold2 } else { p.muted })
                    .width(Length::Fill)
                    .align_x(Horizontal::Center),
            );
        }
        col = col.push(
            txt(tr(
                "В клетке слева — еврейская дата (буквами и цифрой), справа — обычная дата.",
            ))
            .size(theme::text::SMALL)
            .color(p.muted)
            .width(Length::Fill),
        );
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
            (p.gold2, tr("Шаббат")),
            (p.gold, tr("Йом тов")),
            (p.flame, tr("Холь а-моэд / канун")),
            (p.bad, tr("Пост")),
            (p.info, tr("Рош Ходеш")),
            (p.violet, tr("Малый праздник или особый день")),
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
                    widgets::heading(tr("Об этом месяце"), 3),
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

        let tint = info
            .events
            .iter()
            .find(|ev| ev.is_notable())
            .map(|ev| alpha(kind_tint(&p, ev.kind).0, 0.16));
        let num_color = if is_today {
            p.gold2
        } else if let Some(ev) = info.events.iter().find(|ev| ev.is_notable()) {
            kind_tint(&p, ev.kind).0
        } else if shabbat_col {
            p.gold2
        } else {
            p.text
        };
        let greg_month = tr(GREG_MONTHS_ABBR[date.month0() as usize]);

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
                // Left: the Hebrew day (letters, then digits); right: the
                // Gregorian day in Arabic numerals with its month.
                row![
                    txt(hebrew::gematria(info.hebrew.day as u32))
                        .size(16.0)
                        .font(HEBREW)
                        .color(num_color),
                    space::horizontal().width(Length::Fill),
                    txt(date.day().to_string())
                        .size(16.0)
                        .font(SANS_BOLD)
                        .color(num_color),
                ]
                .align_y(Alignment::Center),
                row![
                    txt(info.hebrew.day.to_string()).size(11.0).color(num_color),
                    space::horizontal().width(Length::Fill),
                    txt(greg_month).size(11.0).color(p.muted),
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
        .style(move |_, status| cell_style(&p, is_today, is_selected, shabbat_col, tint, status))
        .on_press(Msg::PickDate(date))
        .into()
    }

    /// Compact details of the selected date, under the month grid.
    fn month_selected_card<'a>(&'a self, p: Palette) -> Element<'a, Msg> {
        let date = self.date;
        let diaspora = self.prefs.diaspora;
        let info = days::day_info(date, diaspora);
        let mut col = Column::new().spacing(10).width(Length::Fill);
        col = col.push(heading_owned(weekday_dash_gregorian(date), 3));
        col = col.push(
            txt(format_hebrew_date(info.hebrew))
                .size(theme::text::SMALL + 0.5)
                .color(p.gold2),
        );
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
                txt({
                    let title = f.title.as_str();
                    trf!("Пост: {title}", "Fast: {title}")
                })
                .size(theme::text::SMALL + 0.5)
                .font(SANS_BOLD)
                .color(p.bad),
            );
        }
        if date.weekday() == Weekday::Sat {
            let (_, reading, _) = parasha::weekly(date, diaspora);
            col = col.push(
                txt({
                    let name = reading_name(reading);
                    trf!("Глава: {name}", "Portion: {name}")
                })
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
                            .color(if it.minor { p.muted } else { p.gold2 }),
                    ]
                    .spacing(10),
                );
            }
        }
        col = col.push(widgets::btn(
            tr("Открыть день"),
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
        col = col.push(widgets::heading(tr("Праздники и посты этого месяца"), 3));
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
                            tr(GREG_MONTHS_ABBR[date.month0() as usize])
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
                txt(tr("В этом месяце нет праздников и постов."))
                    .size(theme::text::SMALL)
                    .color(p.muted),
            );
        }
        widgets::card(col)
    }

    // ---- «Настройки» --------------------------------------------------------

    fn settings_tab<'a>(&'a self, p: Palette) -> Element<'a, Msg> {
        let mut col = Column::new().spacing(16).width(Length::Fill);
        col = col.push(self.location_card(p));
        col = col.push(self.where_card(p));

        col = col.push(
            txt(tr("Всё хранится только на этом компьютере."))
                .size(theme::text::SMALL)
                .color(p.muted),
        );
        col.into()
    }

    fn where_card<'a>(&'a self, p: Palette) -> Element<'a, Msg> {
        widgets::card(
            column![
                widgets::heading(tr("Порядок праздников"), 3),
                txt(tr("Выбирается по месту автоматически. В диаспоре многие праздники длятся два дня, и порядок недельных глав иногда иной."))
                    .size(theme::text::SMALL)
                    .color(p.muted)
                    .width(Length::Fill),
                seg_row(&[
                    (tr("Диаспора"), self.prefs.diaspora, Msg::SetDiaspora(true)),
                    (tr("Израиль"), !self.prefs.diaspora, Msg::SetDiaspora(false)),
                ]),
            ]
            .spacing(12),
        )
    }

    fn location_card<'a>(&'a self, p: Palette) -> Element<'a, Msg> {
        let status = match self.prefs.active_place() {
            Some(pl) => {
                let mode = mode_title(self.prefs.location_mode);
                let place = display_place_name(&pl.name);
                format!("{mode}: {place}")
            }
            None => tr("Место не выбрано — время свечей и постов не показывается").to_string(),
        };
        let mut col = Column::new().spacing(8).width(Length::Fill);
        col = col.push(widgets::heading(tr("Местоположение для времени"), 3));
        col = col.push(
            txt(status)
                .size(theme::text::BODY)
                .font(SANS_BOLD)
                .color(p.gold2)
                .width(Length::Fill),
        );
        col = col.push(
            txt(tr("Хранится только на этом компьютере, с точностью до города. Разрешение на доступ спрашивает сама Windows."))
                .size(theme::text::SMALL)
                .color(p.muted)
                .width(Length::Fill),
        );
        if cfg!(windows) {
            let label = if self.geo_pending.is_some() {
                tr("Ожидаем ответ Windows…")
            } else if self.prefs.consent_asked {
                tr("Запросить у Windows снова")
            } else {
                tr("Определить через Windows")
            };
            col = col.push(widgets::btn(
                label,
                widgets::Btn::Secondary,
                widgets::BtnSize::Sm,
                self.geo_pending.is_none().then_some(Msg::AskGeo),
            ));
        } else {
            col = col.push(
                txt(tr(
                    "На этой системе выберите город или введите координаты ниже.",
                ))
                .size(theme::text::SMALL)
                .color(p.muted)
                .width(Length::Fill),
            );
        }
        if self.prefs.active_place().is_some() {
            col = col.push(widgets::btn(
                tr("Удалить сохранённое место"),
                widgets::Btn::Ghost,
                widgets::BtnSize::Sm,
                Some(Msg::LocationOff),
            ));
        }
        if let Some(note) = &self.geo_note {
            col = col.push(widgets::note_block(tr("Местоположение"), note));
            if cfg!(windows) {
                col = col.push(widgets::btn(
                    tr("Открыть параметры расположения Windows"),
                    widgets::Btn::Ghost,
                    widgets::BtnSize::Sm,
                    Some(Msg::OpenLocationSettings),
                ));
            }
        }

        col = col.push(divider(p));
        col = col.push(
            txt(tr("Город из встроенного списка"))
                .size(theme::text::BODY)
                .font(SANS_BOLD),
        );
        col = col.push(
            text_input(tr("Найти город"), &self.city_query)
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
            txt(tr("Или введите координаты вручную"))
                .size(theme::text::BODY)
                .font(SANS_BOLD),
        );
        col = col.push(
            row![
                text_input(tr("Широта, например 31,78"), &self.lat_input)
                    .on_input(Msg::LatEdit)
                    .padding(12)
                    .width(Length::Fill)
                    .style(move |_t, s| theme::text_input(&palette(), s)),
                text_input(tr("Долгота, например 35,23"), &self.lon_input)
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
        col = col.push(btn_secondary(tr("Сохранить координаты"), Msg::SaveCoords));
        col = col.push(divider(p));
        col = col.push(self.calculation_rules(p));
        widgets::card(col)
    }

    /// Reference, not a quiz: display the actual saved calculation rules.
    fn calculation_rules<'a>(&'a self, p: Palette) -> Element<'a, Msg> {
        let havdalah = match self.prefs.havdalah_minutes {
            0 => tr("по выходу звёзд (Солнце на 8,5° ниже горизонта)").to_string(),
            n => trf!("через {n} мин после заката", "{n} min after sunset"),
        };
        let mins = self.prefs.candle_minutes;
        column![
            txt(tr("Расчёт времени")).size(theme::text::BODY).font(SANS_BOLD),
            txt(trf!(
                "Свечи — за {mins} мин до заката. Авдала — {havdalah}.",
                "Candles — {mins} min before sunset. Havdalah — {havdalah}."
            ))
                .size(theme::text::SMALL).color(p.gold2).width(Length::Fill),
            txt(tr("Обычай выбирается по месту: в Иерусалиме за 40 мин, в остальных местах за 18 мин. Начало малого поста — рассвет, большого — закат накануне; конец — выход звёзд. Местный обычай уточните у раввина."))
                .size(theme::text::SMALL).color(p.muted).width(Length::Fill),
        ].spacing(6).into()
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
    tint: Option<Color>,
    status: button::Status,
) -> button::Style {
    let background = if selected {
        p.gold_soft
    } else if today {
        p.surface3
    } else if let Some(tint) = tint {
        tint
    } else if shabbat_col {
        alpha(p.gold, 0.05)
    } else {
        p.surface2
    };
    let base = button::Style {
        background: Some(Background::Color(background)),
        text_color: p.text,
        border: Border {
            color: if today || selected {
                p.gold
            } else if let Some(tint) = tint {
                Color { a: 0.55, ..tint }
            } else {
                p.line
            },
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
        prefs::LocationMode::Off => tr("Не выбрано"),
        prefs::LocationMode::System => tr("Определено Windows"),
        prefs::LocationMode::Manual => tr("Выбрано вручную"),
    }
}

/// Civil date. `format_gregorian_ru` already switches language.
fn format_gregorian(d: NaiveDate) -> String {
    hebrew::format_gregorian_ru(d)
}

/// Hebrew date. `format_ru` already switches language.
fn format_hebrew_date(h: hebrew::HebrewDate) -> String {
    h.format_ru()
}

fn hebrew_month_heading(y: i32, m: u8) -> String {
    format!("{} {}", hebrew::month_name_ru(y, m), y)
}

fn weekday_and_gregorian(d: NaiveDate) -> String {
    format!(
        "{}, {}",
        hebrew::weekday_ru(d),
        hebrew::format_gregorian_ru(d)
    )
}

fn weekday_dash_gregorian(d: NaiveDate) -> String {
    format!(
        "{} — {}",
        hebrew::weekday_ru(d),
        hebrew::format_gregorian_ru(d)
    )
}

fn reading_name(reading: parasha::Reading) -> String {
    reading.name_ru()
}

/// Synthetic place labels are stored in Russian (so the saved value stays a
/// stable key) and translated here. City names from the list stay as stored:
/// they are content text and the text the city search matches.
fn display_place_name(name: &str) -> String {
    if name == "Примерное местоположение" {
        return tr("Примерное местоположение").to_string();
    }
    if name == "Координаты вручную" {
        return tr("Координаты вручную").to_string();
    }
    name.to_string()
}

fn plural_days(n: i64) -> String {
    let word = plural_l(n, &["день", "дня", "дней"], &["day", "days"]);
    format!("{n} {word}")
}

fn plural_weeks(n: i64) -> String {
    let word = plural_l(n, &["неделя", "недели", "недель"], &["week", "weeks"]);
    format!("{n} {word}")
}

/// «1 неделя и 5 дней» (zero parts omitted).
fn omer_breakdown(n: u8) -> String {
    let w = n as i64 / 7;
    let d = n as i64 % 7;
    match (w, d) {
        (0, _) => plural_days(d),
        (_, 0) => plural_weeks(w),
        _ => {
            let weeks = plural_weeks(w);
            let days = plural_days(d);
            trf!("{weeks} и {days}", "{weeks} and {days}")
        }
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

/// «ноябрь — декабрь 2026» / «декабрь 2026 — январь 2027»: the Gregorian
/// months a Hebrew month spans (Arabic numerals next to the Hebrew title).
fn gregorian_span(y: i32, m: u8) -> String {
    let first = hebrew::HebrewDate::new(y, m, 1).to_gregorian();
    let last = hebrew::HebrewDate::new(y, m, hebrew::days_in_month(y, m)).to_gregorian();
    let name = |d: NaiveDate| tr(GREG_MONTHS_NOM[d.month0() as usize]);
    if first.year() != last.year() {
        format!(
            "{} {} — {} {}",
            name(first),
            first.year(),
            name(last),
            last.year()
        )
    } else if first.month() != last.month() {
        format!("{} — {} {}", name(first), name(last), last.year())
    } else {
        format!("{} {}", name(first), first.year())
    }
}

const GREG_MONTHS_NOM: [&str; 12] = [
    "январь",
    "февраль",
    "март",
    "апрель",
    "май",
    "июнь",
    "июль",
    "август",
    "сентябрь",
    "октябрь",
    "ноябрь",
    "декабрь",
];

const GREG_MONTHS_ABBR: [&str; 12] = [
    "янв", "фев", "мар", "апр", "май", "июн", "июл", "авг", "сен", "окт", "ноя", "дек",
];

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gregorian_span_of_hebrew_months() {
        // Кислев 5787 ≈ 11 ноября — 10 декабря 2026.
        assert_eq!(gregorian_span(5787, 9), "ноябрь — декабрь 2026");
        // Тевет 5787 crosses the civil new year.
        assert_eq!(gregorian_span(5787, 10), "декабрь 2026 — январь 2027");
    }
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
