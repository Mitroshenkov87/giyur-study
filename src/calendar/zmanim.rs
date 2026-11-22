//! Zmanim: sunrise/sunset-based halachic times, computed offline.
//! STUB — the API below is fixed; the implementation is being written.

use chrono::{NaiveDate, NaiveTime};

use crate::calendar::prefs::{CalendarPrefs, Place};

/// Times for one civil day at one place, in the place's local time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DayTimes {
    /// Dawn, sun 16.1° below the horizon (start of a minor fast).
    pub alot: Option<NaiveTime>,
    pub sunrise: Option<NaiveTime>,
    pub sunset: Option<NaiveTime>,
    /// Nightfall, sun 8.5° below the horizon (end of Shabbat / fasts).
    pub tzeit: Option<NaiveTime>,
    /// sunset − candle_minutes.
    pub candle_lighting: Option<NaiveTime>,
    /// tzeit, or sunset + havdalah_minutes when that is set.
    pub havdalah: Option<NaiveTime>,
    /// UTC offset used, minutes.
    pub utc_offset_min: i32,
}

/// Compute the times for `date` at `place` with the user's settings.
pub fn day_times(date: NaiveDate, place: &Place, prefs: &CalendarPrefs) -> DayTimes {
    let _ = (date, place, prefs);
    DayTimes {
        alot: None,
        sunrise: None,
        sunset: None,
        tzeit: None,
        candle_lighting: None,
        havdalah: None,
        utc_offset_min: 0,
    }
}

/// UTC offset (minutes) for `place` on `date`: its time-zone rule, or this
/// computer's zone when `place.tz == "local"`.
pub fn utc_offset_minutes(place: &Place, date: NaiveDate) -> i32 {
    let _ = (place, date);
    0
}
