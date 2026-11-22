//! Optional approximate location (with consent) and the offline city list.
//!
//! The UI asks before calling [`request_approximate`]. This module only
//! performs the request. Coordinates are rounded to 0.1° before they are
//! returned and are never logged.
//!
//! Windows uses the WinRT geolocator. Other platforms return
//! [`GeoError::Unsupported`]; the user picks a city or types coordinates.

use std::sync::OnceLock;

/// One bundled city (content/calendar/cities.json).
#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub struct City {
    pub name_ru: String,
    pub country_ru: String,
    pub lat: f64,
    pub lon: f64,
    /// Time-zone rule: `il`, `eu`, `us`, `au`, `none` (fixed offset).
    pub tz: String,
    /// Standard (winter) UTC offset, minutes.
    pub utc_offset_min: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GeoError {
    /// No system location API on this platform (manual entry instead).
    Unsupported,
    /// The OS or the user denied location access.
    Denied,
    /// Location service off, timed out or failed.
    Unavailable(String),
}

/// Ask the OS for an approximate (city-level) position. Blocking: call it
/// off the UI thread. Returns coordinates already rounded to 0.1°.
pub fn request_approximate() -> Result<(f64, f64), GeoError> {
    #[cfg(windows)]
    {
        windows_approximate()
    }
    #[cfg(not(windows))]
    {
        Err(GeoError::Unsupported)
    }
}

/// WinRT geolocator. `join` is the blocking wait (windows 0.62; older
/// windows-rs called this `get`).
#[cfg(windows)]
fn windows_approximate() -> Result<(f64, f64), GeoError> {
    use windows::core::Interface;
    use windows::Devices::Geolocation::{GeolocationAccessStatus, Geolocator, PositionAccuracy};
    use windows::Foundation::{IReference, PropertyValue, TimeSpan};

    use crate::calendar::prefs::round_coord;

    fn fail(err: &windows::core::Error) -> GeoError {
        let code = err.code().0 as u32;
        GeoError::Unavailable(format!("location unavailable (0x{code:08X})"))
    }

    let status = Geolocator::RequestAccessAsync()
        .map_err(|err| fail(&err))?
        .join()
        .map_err(|err| fail(&err))?;
    if status != GeolocationAccessStatus::Allowed {
        return Err(GeoError::Denied);
    }

    let locator = Geolocator::new().map_err(|err| fail(&err))?;
    locator
        .SetDesiredAccuracy(PositionAccuracy::Default)
        .map_err(|err| fail(&err))?;
    // About 3 km: city level. A refusal leaves the Default accuracy set above.
    if let Ok(boxed) = PropertyValue::CreateUInt32(3_000) {
        if let Ok(meters) = boxed.cast::<IReference<u32>>() {
            let _meters_set = locator.SetDesiredAccuracyInMeters(&meters);
        }
    }

    // TimeSpan::Duration is in 100-nanosecond units.
    const TICKS_PER_SEC: i64 = 10_000_000;
    let position = locator
        .GetGeopositionAsyncWithAgeAndTimeout(
            TimeSpan {
                Duration: 60 * 60 * TICKS_PER_SEC,
            },
            TimeSpan {
                Duration: 15 * TICKS_PER_SEC,
            },
        )
        .map_err(|err| fail(&err))?
        .join()
        .map_err(|err| fail(&err))?;

    let basic = position
        .Coordinate()
        .map_err(|err| fail(&err))?
        .Point()
        .map_err(|err| fail(&err))?
        .Position()
        .map_err(|err| fail(&err))?;
    Ok((round_coord(basic.Latitude), round_coord(basic.Longitude)))
}

/// All bundled cities.
pub fn cities() -> &'static [City] {
    static CITIES: OnceLock<Vec<City>> = OnceLock::new();
    CITIES.get_or_init(load_cities).as_slice()
}

fn load_cities() -> Vec<City> {
    serde_json::from_str(include_str!("../../content/calendar/cities.json"))
        .expect("content/calendar/cities.json")
}

/// Case-insensitive substring search over Russian names and countries.
/// `ё` and `е` match. Name-prefix hits come before other hits.
pub fn search(query: &str) -> Vec<&'static City> {
    let q = normalize(query.trim());
    if q.is_empty() {
        return cities().iter().collect();
    }
    let mut hits: Vec<&City> = cities()
        .iter()
        .filter(|city| {
            normalize(&city.name_ru).contains(&q) || normalize(&city.country_ru).contains(&q)
        })
        .collect();
    // Stable: original file order inside each group.
    hits.sort_by_key(|city| usize::from(!normalize(&city.name_ru).starts_with(&q)));
    hits
}

fn normalize(s: &str) -> String {
    s.to_lowercase().replace('ё', "е")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn city(name: &str) -> &City {
        cities()
            .iter()
            .find(|c| c.name_ru == name)
            .unwrap_or_else(|| panic!("missing {name}"))
    }

    #[test]
    fn cities_parse_and_search_jerusalem() {
        let all = cities();
        assert!(all.len() >= 50, "got {}", all.len());
        let hits = search("иер");
        assert!(!hits.is_empty());
        assert_eq!(hits[0].name_ru, "Иерусалим");
        assert!(hits.iter().any(|c| c.name_ru == "Иерусалим"));
        assert_eq!(search("ИЕР")[0].name_ru, "Иерусалим");
    }

    #[test]
    fn search_folds_yo_and_ranks_prefixes() {
        let kishinev = search("кишинев");
        assert!(kishinev.iter().any(|c| c.name_ru == "Кишинёв"));
        // "ан" prefixes Антверпен and is only a substring of Сан-Паулу
        // and of the country name Франция (Париж, Марсель).
        let hits = search("ан");
        assert_eq!(hits[0].name_ru, "Антверпен");
        let at = |name: &str| hits.iter().position(|c| c.name_ru == name).unwrap();
        assert!(at("Антверпен") < at("Сан-Паулу"));
        assert!(at("Антверпен") < at("Париж"));
    }

    #[test]
    fn standard_offsets_match_the_rules() {
        let expect = [
            ("Москва", "none", 180),
            ("Санкт-Петербург", "none", 180),
            ("Казань", "none", 180),
            ("Нижний Новгород", "none", 180),
            ("Ростов-на-Дону", "none", 180),
            ("Краснодар", "none", 180),
            ("Самара", "none", 240),
            ("Екатеринбург", "none", 300),
            ("Новосибирск", "none", 420),
            ("Киев", "eu", 120),
            ("Одесса", "eu", 120),
            ("Днепр", "eu", 120),
            ("Харьков", "eu", 120),
            ("Кишинёв", "eu", 120),
            ("Рига", "eu", 120),
            ("Вильнюс", "eu", 120),
            ("Таллин", "eu", 120),
            ("Минск", "none", 180),
            ("Тбилиси", "none", 240),
            ("Баку", "none", 240),
            ("Ереван", "none", 240),
            ("Алматы", "none", 300),
            ("Ташкент", "none", 300),
            ("Буэнос-Айрес", "none", -180),
            ("Сан-Паулу", "none", -180),
            ("Мехико", "none", -360),
            ("Йоханнесбург", "none", 120),
            ("Мельбурн", "au", 600),
            ("Сидней", "au", 600),
            ("Иерусалим", "il", 120),
            ("Нью-Йорк", "us", -300),
            ("Лос-Анджелес", "us", -480),
            ("Чикаго", "us", -360),
            ("Лондон", "eu", 0),
            ("Париж", "eu", 60),
        ];
        for (name, tz, offset) in expect {
            let c = city(name);
            assert_eq!(c.tz, tz, "{name}");
            assert_eq!(c.utc_offset_min, offset, "{name}");
        }
        for c in cities() {
            assert!(
                matches!(c.tz.as_str(), "il" | "eu" | "us" | "au" | "none"),
                "{}",
                c.name_ru
            );
            assert!((-90.0..=90.0).contains(&c.lat), "{}", c.name_ru);
            assert!((-180.0..=180.0).contains(&c.lon), "{}", c.name_ru);
        }
    }

    #[test]
    fn system_location_unsupported_off_windows() {
        if cfg!(not(windows)) {
            assert_eq!(request_approximate(), Err(GeoError::Unsupported));
        }
    }
}
