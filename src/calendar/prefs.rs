//! Calendar settings. Stored locally inside the progress file under
//! `settings.calendar` (the settings `extra` map), so no storage schema
//! change is needed and nothing leaves the computer.
//!
//! Location is OFF by default. Coordinates are rounded to 0.1° (≈ 10 km,
//! city level) before they are stored, and are never logged.

use serde::{Deserialize, Serialize};

use crate::storage::Settings;

const KEY: &str = "calendar";

/// Where zmanim coordinates come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LocationMode {
    /// No location: zmanim are not shown (default).
    #[default]
    Off,
    /// Approximate system geolocation (asked with consent).
    System,
    /// A city picked from the bundled list, or coordinates typed by hand.
    Manual,
}

/// A stored, already-rounded location.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Place {
    /// Display name («Иерусалим», «Координаты вручную», «Примерное местоположение»).
    pub name: String,
    pub lat: f64,
    pub lon: f64,
    /// Time-zone rule id from content/calendar/cities.json (`il`, `eu`, `us`, …)
    /// or `local` = use this computer's time zone.
    #[serde(default = "local_tz")]
    pub tz: String,
    /// Standard UTC offset in minutes for `tz` rules other than `local`.
    #[serde(default)]
    pub utc_offset_min: i32,
}

fn local_tz() -> String {
    "local".into()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CalendarPrefs {
    /// true = diaspora (two-day festivals, diaspora parasha), false = Israel.
    pub diaspora: bool,
    pub location_mode: LocationMode,
    pub place: Option<Place>,
    /// The consent prompt was shown (answered either way).
    pub consent_asked: bool,
    /// Candle lighting: minutes before sunset (18 by default; Jerusalem 40).
    pub candle_minutes: u32,
    /// Havdalah / end of Shabbat: minutes after sunset; 0 = when three stars
    /// appear (sun 8.5° below the horizon).
    pub havdalah_minutes: u32,
}

impl Default for CalendarPrefs {
    fn default() -> Self {
        CalendarPrefs {
            diaspora: true,
            location_mode: LocationMode::Off,
            place: None,
            consent_asked: false,
            candle_minutes: 18,
            havdalah_minutes: 0,
        }
    }
}

/// Round to 0.1° (city level).
pub fn round_coord(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

impl CalendarPrefs {
    pub fn load(settings: &Settings) -> CalendarPrefs {
        settings
            .extra
            .get(KEY)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default()
    }

    /// Write back into the settings (caller saves the progress file).
    pub fn store(&self, settings: &mut Settings) {
        let mut me = self.clone();
        if let Some(p) = me.place.as_mut() {
            p.lat = round_coord(p.lat);
            p.lon = round_coord(p.lon);
        }
        if let Ok(v) = serde_json::to_value(&me) {
            settings.extra.insert(KEY.to_string(), v);
        }
    }

    /// The place to compute zmanim for, if location is on.
    pub fn active_place(&self) -> Option<&Place> {
        match self.location_mode {
            LocationMode::Off => None,
            _ => self.place.as_ref(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_off_and_round_trip() {
        let mut s = Settings::default();
        let p = CalendarPrefs::load(&s);
        assert_eq!(p.location_mode, LocationMode::Off);
        assert!(p.active_place().is_none());
        assert_eq!(p.candle_minutes, 18);
        let mut p2 = p.clone();
        p2.diaspora = false;
        p2.location_mode = LocationMode::Manual;
        p2.place = Some(Place {
            name: "Иерусалим".into(),
            lat: 31.7683,
            lon: 35.2137,
            tz: "il".into(),
            utc_offset_min: 120,
        });
        p2.store(&mut s);
        let back = CalendarPrefs::load(&s);
        assert!(!back.diaspora);
        let pl = back.active_place().unwrap();
        assert_eq!(pl.lat, 31.8);
        assert_eq!(pl.lon, 35.2);
    }
}
