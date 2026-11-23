//! The times worth showing for a day (candle lighting, havdalah, fast start
//! and end, sunrise/sunset), combining [`days`] with [`zmanim`].

use chrono::{Datelike, Duration, NaiveDate, NaiveTime, Weekday};

use crate::calendar::days::{self, Kind};
use crate::calendar::prefs::CalendarPrefs;
use crate::calendar::zmanim;
use crate::i18n::tr;

#[derive(Debug, Clone, PartialEq)]
pub struct TimeItem {
    /// Russian label, e.g. «Зажигание свечей».
    pub label: String,
    pub time: NaiveTime,
    /// Shown dimmer (sunrise/sunset reference lines).
    pub minor: bool,
}

/// Times for `date`; empty when location is off.
pub fn for_day(date: NaiveDate, prefs: &CalendarPrefs) -> Vec<TimeItem> {
    let Some(place) = prefs.active_place() else {
        return Vec::new();
    };
    let diaspora = prefs.diaspora;
    let t = zmanim::day_times(date, place, prefs);
    let info = days::day_info(date, diaspora);
    let mut out = Vec::new();
    let mut add = |label: &str, time: Option<NaiveTime>, minor: bool| {
        if let Some(time) = time {
            out.push(TimeItem {
                label: label.to_string(),
                time,
                minor,
            });
        }
    };

    if let Some(f) = info.fast() {
        match f.kind {
            Kind::MajorFast => {
                // Began at sunset yesterday.
                let y = zmanim::day_times(date - Duration::days(1), place, prefs);
                add(tr("Начало поста (вчера на закате)"), y.sunset, false);
            }
            _ => add(tr("Начало поста (рассвет)"), t.alot, false),
        }
        add(tr("Конец поста (выход звёзд)"), t.tzeit, false);
    }
    // Tonight a major fast begins (eve of Yom Kippur / Tisha BeAv).
    let tomorrow = days::day_info(date + Duration::days(1), diaspora);
    if let Some(f) = tomorrow.fast() {
        if f.kind == Kind::MajorFast {
            add(tr("Вечером начинается пост (закат)"), t.sunset, false);
        }
    }
    if days::candles_tonight(date, diaspora) {
        if info.is_rest_day() && date.weekday() != Weekday::Fri {
            // Second festival day or yom tov after Shabbat: light only after
            // nightfall, from an existing flame.
            add(
                tr("Зажигание свечей (не раньше выхода звёзд)"),
                t.tzeit,
                false,
            );
        } else {
            add(tr("Зажигание свечей"), t.candle_lighting, false);
        }
    }
    if days::havdalah_tonight(date, diaspora) {
        add(tr("Авдала (исход Шаббата / праздника)"), t.havdalah, false);
    }
    add(tr("Восход"), t.sunrise, true);
    add(tr("Закат"), t.sunset, true);
    out
}

/// One-line caveat shown under the times.
pub const CAVEAT: &str = "Время рассчитано приблизительно по координатам. В общинах приняты разные обычаи (например, в Иерусалиме свечи зажигают за 40 минут до заката) — уточните у своего раввина или в календаре общины.";
