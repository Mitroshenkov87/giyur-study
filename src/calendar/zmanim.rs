//! Zmanim: sunrise/sunset-based halachic times, computed offline.
//!
//! Solar times use the public-domain NOAA "General Solar Position
//! Calculations" (the spreadsheet algorithm, the same one KosherJava's
//! NOAACalculator is built on): Julian centuries from J2000, equation of
//! time, solar declination, then the hour angle for a zenith. Longitude in
//! this module is degrees east. The NOAA noon iteration uses west-positive
//! longitude internally.
//!
//! Sunrise and sunset use zenith 90.833° (atmospheric refraction plus the
//! solar radius, sea level). Alot ha-shachar is the sun 16.1° below the
//! horizon (zenith 106.1°) in the morning. Tzeit ha-kochavim is the sun
//! 8.5° below the horizon (zenith 98.5°) in the evening. Polar day and
//! polar night, when the sun never reaches that zenith, produce `None`.
//!
//! Time zones are rule ids on [`Place`], not the IANA database:
//!
//! * `local` — this computer's offset at local noon (`chrono::Local`).
//! * `none` — the fixed `utc_offset_min` (standard time, no extra hour).
//! * `eu` — +60 min from the last Sunday of March up to, but not including,
//!   the last Sunday of October.
//! * `us` — +60 min from the second Sunday of March up to the first Sunday
//!   of November.
//! * `il` — +60 min from the Friday before the last Sunday of March up to
//!   the last Sunday of October.
//! * `au` — southern hemisphere: +60 min from the first Sunday of October
//!   up to the first Sunday of April (the period wraps the new year).
//!
//! Any other id is treated as a fixed offset, the same as `none`.
//!
//! Daylight saving is applied for the whole civil day. The spring-forward
//! date counts as daylight time. The autumn-back date counts as standard
//! time. Real clocks change in the early morning, so this matches the
//! offset used for most of each switch date.
//!
//! Results are local civil times, rounded to the nearest minute.

use chrono::{Datelike, NaiveDate, NaiveTime, TimeDelta};

use crate::calendar::prefs::{CalendarPrefs, Place};

/// Sunrise/sunset: refraction plus solar radius, sea level.
const ZENITH_RISE_SET: f64 = 90.833;
/// Alot ha-shachar: 16.1° below the horizon.
const ZENITH_ALOT: f64 = 106.1;
/// Tzeit ha-kochavim: 8.5° below the horizon.
const ZENITH_TZEIT: f64 = 98.5;

const JULIAN_DAY_J2000: f64 = 2_451_545.0;
const JULIAN_DAYS_PER_CENTURY: f64 = 36_525.0;

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

/// Which limb of the day the hour-angle solution belongs to.
#[derive(Clone, Copy)]
enum SolarEvent {
    Sunrise,
    Sunset,
}

/// Compute the times for `date` at `place` with the user's settings.
pub fn day_times(date: NaiveDate, place: &Place, prefs: &CalendarPrefs) -> DayTimes {
    let utc_offset_min = utc_offset_minutes(place, date);
    let alot = local_sun(
        date,
        place.lat,
        place.lon,
        ZENITH_ALOT,
        SolarEvent::Sunrise,
        utc_offset_min,
    );
    let sunrise = local_sun(
        date,
        place.lat,
        place.lon,
        ZENITH_RISE_SET,
        SolarEvent::Sunrise,
        utc_offset_min,
    );
    let sunset = local_sun(
        date,
        place.lat,
        place.lon,
        ZENITH_RISE_SET,
        SolarEvent::Sunset,
        utc_offset_min,
    );
    let tzeit = local_sun(
        date,
        place.lat,
        place.lon,
        ZENITH_TZEIT,
        SolarEvent::Sunset,
        utc_offset_min,
    );
    let candle_lighting = sunset.map(|t| shift_minutes(t, -(prefs.candle_minutes as i64)));
    let havdalah = if prefs.havdalah_minutes > 0 {
        sunset.map(|t| shift_minutes(t, prefs.havdalah_minutes as i64))
    } else {
        tzeit
    };
    DayTimes {
        alot,
        sunrise,
        sunset,
        tzeit,
        candle_lighting,
        havdalah,
        utc_offset_min,
    }
}

/// UTC offset (minutes) for `place` on `date`: its time-zone rule, or this
/// computer's zone when `place.tz == "local"`.
pub fn utc_offset_minutes(place: &Place, date: NaiveDate) -> i32 {
    if place.tz == "local" {
        return computer_offset_minutes(date);
    }
    let dst = if daylight_saving(&place.tz, date) {
        60
    } else {
        0
    };
    place.utc_offset_min + dst
}

fn computer_offset_minutes(date: NaiveDate) -> i32 {
    use chrono::{Local, LocalResult, Offset, TimeZone};

    // Local noon is outside the 02:00 transition, so the offset is unambiguous.
    let Some(noon) = date.and_hms_opt(12, 0, 0) else {
        return 0;
    };
    let dt = match Local.from_local_datetime(&noon) {
        LocalResult::Single(dt) => dt,
        LocalResult::Ambiguous(earliest, _) => earliest,
        LocalResult::None => return 0,
    };
    dt.offset().fix().local_minus_utc() / 60
}

/// `true` when `date` falls in the daylight-saving period of `tz`.
fn daylight_saving(tz: &str, date: NaiveDate) -> bool {
    let year = date.year();
    match tz {
        "eu" => {
            let start = last_sunday(year, 3);
            let end = last_sunday(year, 10);
            date >= start && date < end
        }
        "us" => {
            let start = nth_sunday(year, 3, 2);
            let end = nth_sunday(year, 11, 1);
            date >= start && date < end
        }
        "il" => {
            // Friday before the last Sunday of March.
            let start = last_sunday(year, 3) - TimeDelta::days(2);
            let end = last_sunday(year, 10);
            date >= start && date < end
        }
        "au" => {
            let april = nth_sunday(year, 4, 1);
            let october = nth_sunday(year, 10, 1);
            date < april || date >= october
        }
        _ => false,
    }
}

fn nth_sunday(year: i32, month: u32, n: u32) -> NaiveDate {
    let first = NaiveDate::from_ymd_opt(year, month, 1).expect("month");
    let to_sunday = (7 - first.weekday().num_days_from_sunday()) % 7;
    first + TimeDelta::days(i64::from(to_sunday + 7 * (n - 1)))
}

fn last_sunday(year: i32, month: u32) -> NaiveDate {
    let first_next = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .expect("month");
    let last = first_next - TimeDelta::days(1);
    last - TimeDelta::days(i64::from(last.weekday().num_days_from_sunday()))
}

fn shift_minutes(time: NaiveTime, minutes: i64) -> NaiveTime {
    time.overflowing_add_signed(TimeDelta::minutes(minutes)).0
}

fn local_sun(
    date: NaiveDate,
    lat: f64,
    lon_east: f64,
    zenith: f64,
    event: SolarEvent,
    offset_min: i32,
) -> Option<NaiveTime> {
    let utc = sun_utc_minutes(date, lat, lon_east, zenith, event)?;
    minutes_to_time(utc + f64::from(offset_min))
}

/// Minutes from 00:00 UTC, wrapped into a civil clock after rounding.
fn minutes_to_time(minutes: f64) -> Option<NaiveTime> {
    if !minutes.is_finite() {
        return None;
    }
    let rounded = minutes.round() as i64;
    let mins = rounded.rem_euclid(24 * 60) as u32;
    NaiveTime::from_num_seconds_from_midnight_opt(mins * 60, 0)
}

/// UTC minutes when the sun's center is at `zenith`. `None` on polar day/night.
fn sun_utc_minutes(
    date: NaiveDate,
    lat: f64,
    lon_east: f64,
    zenith: f64,
    event: SolarEvent,
) -> Option<f64> {
    // NOAA's iteration takes longitude positive west.
    let lon_west = -lon_east;
    let jd = julian_day(date);
    let noon = solar_noon_utc_minutes(jd, lon_west);

    let t_noon = julian_centuries(jd + noon / 1440.0);
    let mut eqt = equation_of_time(t_noon);
    let mut decl = sun_declination(t_noon);
    let mut hour = sun_hour_angle(lat, decl, zenith, event);
    if !hour.is_finite() {
        // Declination at noon missed the zenith. Try solar midnight, where
        // the sun is lowest (before sunrise, or after sunset).
        let midnight = noon
            + match event {
                SolarEvent::Sunrise => -720.0,
                SolarEvent::Sunset => 720.0,
            };
        let t_mid = julian_centuries(jd + midnight / 1440.0);
        eqt = equation_of_time(t_mid);
        decl = sun_declination(t_mid);
        hour = sun_hour_angle(lat, decl, zenith, event);
    }
    if !hour.is_finite() {
        return None;
    }

    let mut utc = 720.0 + 4.0 * (lon_west - hour.to_degrees()) - eqt;
    // Second pass at the estimated event time.
    let t_event = julian_centuries(jd + utc / 1440.0);
    eqt = equation_of_time(t_event);
    decl = sun_declination(t_event);
    hour = sun_hour_angle(lat, decl, zenith, event);
    if !hour.is_finite() {
        return None;
    }
    utc = 720.0 + 4.0 * (lon_west - hour.to_degrees()) - eqt;
    if utc.is_finite() {
        Some(utc)
    } else {
        None
    }
}

fn solar_noon_utc_minutes(jd: f64, lon_west: f64) -> f64 {
    let t0 = julian_centuries(jd + lon_west / 360.0);
    let mut eqt = equation_of_time(t0);
    let mut noon = lon_west * 4.0 - eqt;
    for _ in 0..2 {
        let t = julian_centuries(jd + noon / 1440.0);
        eqt = equation_of_time(t);
        noon = 720.0 + lon_west * 4.0 - eqt;
    }
    noon
}

/// Julian day at 00:00 UT (the Meeus integer formula).
fn julian_day(date: NaiveDate) -> f64 {
    let mut year = date.year();
    let mut month = date.month() as i32;
    let day = date.day() as f64;
    if month <= 2 {
        year -= 1;
        month += 12;
    }
    let a = year / 100;
    let b = 2 - a + a / 4;
    (365.25 * f64::from(year + 4716)).floor()
        + (30.6001 * f64::from(month + 1)).floor()
        + day
        + f64::from(b)
        - 1524.5
}

fn julian_centuries(jd: f64) -> f64 {
    (jd - JULIAN_DAY_J2000) / JULIAN_DAYS_PER_CENTURY
}

fn wrap360(mut deg: f64) -> f64 {
    deg %= 360.0;
    if deg < 0.0 {
        deg += 360.0;
    }
    deg
}

fn sin_deg(deg: f64) -> f64 {
    deg.to_radians().sin()
}

fn cos_deg(deg: f64) -> f64 {
    deg.to_radians().cos()
}

fn tan_deg(deg: f64) -> f64 {
    deg.to_radians().tan()
}

fn geom_mean_longitude(t: f64) -> f64 {
    wrap360(280.46646 + t * (36000.76983 + 0.0003032 * t))
}

fn geom_mean_anomaly(t: f64) -> f64 {
    357.52911 + t * (35999.05029 - 0.0001537 * t)
}

fn earth_orbit_eccentricity(t: f64) -> f64 {
    0.016708634 - t * (0.000042037 + 0.0000001267 * t)
}

fn sun_equation_of_center(t: f64) -> f64 {
    let m = geom_mean_anomaly(t);
    sin_deg(m) * (1.914602 - t * (0.004817 + 0.000014 * t))
        + sin_deg(2.0 * m) * (0.019993 - 0.000101 * t)
        + sin_deg(3.0 * m) * 0.000289
}

fn sun_apparent_longitude(t: f64) -> f64 {
    let omega = 125.04 - 1934.136 * t;
    geom_mean_longitude(t) + sun_equation_of_center(t) - 0.00569 - 0.00478 * sin_deg(omega)
}

fn obliquity_correction(t: f64) -> f64 {
    let seconds = 21.448 - t * (46.8150 + t * (0.00059 - t * 0.001813));
    let mean = 23.0 + (26.0 + seconds / 60.0) / 60.0;
    let omega = 125.04 - 1934.136 * t;
    mean + 0.00256 * cos_deg(omega)
}

fn sun_declination(t: f64) -> f64 {
    let sint = sin_deg(obliquity_correction(t)) * sin_deg(sun_apparent_longitude(t));
    sint.asin().to_degrees()
}

/// Equation of time in minutes.
fn equation_of_time(t: f64) -> f64 {
    let epsilon = obliquity_correction(t);
    let l0 = geom_mean_longitude(t);
    let e = earth_orbit_eccentricity(t);
    let m = geom_mean_anomaly(t);
    let y = tan_deg(epsilon / 2.0);
    let y = y * y;
    let eot = y * sin_deg(2.0 * l0) - 2.0 * e * sin_deg(m)
        + 4.0 * e * y * sin_deg(m) * cos_deg(2.0 * l0)
        - 0.5 * y * y * sin_deg(4.0 * l0)
        - 1.25 * e * e * sin_deg(2.0 * m);
    eot.to_degrees() * 4.0
}

/// Hour angle in radians. `NaN` when the sun never reaches `zenith` that day.
fn sun_hour_angle(latitude: f64, declination: f64, zenith: f64, event: SolarEvent) -> f64 {
    let denom = cos_deg(latitude) * cos_deg(declination);
    if denom.abs() < 1.0e-12 {
        return f64::NAN;
    }
    let ratio = cos_deg(zenith) / denom - tan_deg(latitude) * tan_deg(declination);
    // A hair outside [-1, 1] is rounding error. Clearly outside is polar day/night.
    const EPS: f64 = 1.0e-10;
    if !ratio.is_finite() || ratio > 1.0 + EPS || ratio < -1.0 - EPS {
        return f64::NAN;
    }
    let mut angle = ratio.clamp(-1.0, 1.0).acos();
    if matches!(event, SolarEvent::Sunset) {
        angle = -angle;
    }
    angle
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::prefs::CalendarPrefs;
    use chrono::{NaiveDate, NaiveTime, Timelike};

    fn ymd(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn hm(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }

    fn place(name: &str, lat: f64, lon: f64, tz: &str, offset: i32) -> Place {
        Place {
            name: name.into(),
            lat,
            lon,
            tz: tz.into(),
            utc_offset_min: offset,
        }
    }

    fn jerusalem() -> Place {
        place("Иерусалим", 31.778, 35.235, "il", 120)
    }

    fn new_york() -> Place {
        place("Нью-Йорк", 40.7128, -74.006, "us", -300)
    }

    fn prefs(candle: u32, havdalah: u32) -> CalendarPrefs {
        CalendarPrefs {
            candle_minutes: candle,
            havdalah_minutes: havdalah,
            ..CalendarPrefs::default()
        }
    }

    /// `got` is within ±2 minutes of `h:m`.
    fn near(got: Option<NaiveTime>, h: u32, m: u32) {
        let expect = hm(h, m);
        let Some(got) = got else {
            panic!("missing time, expected {expect}");
        };
        let delta = (got.num_seconds_from_midnight() as i32
            - expect.num_seconds_from_midnight() as i32)
            .abs();
        assert!(delta <= 120, "{got} vs {expect} ({delta}s)");
    }

    #[test]
    fn julian_day_j2000() {
        // 2000-01-01 00:00 UT.
        let jd = julian_day(ymd(2000, 1, 1));
        assert!((jd - 2_451_544.5).abs() < 1e-6);
    }

    #[test]
    fn published_noaa_times() {
        // Hebcal zmanim API v1.27.5 (NOAA solar calculator, sea level, no
        // elevation), retrieved 2026-11-22. Fields: sunrise, sunset,
        // alotHaShachar (16.1°), tzeit85deg (8.5°). Times below are local.
        //
        // Jerusalem 31.778°N, 35.235°E, Asia/Jerusalem:
        //   2026-06-21 (IDT, UTC+3): sunrise 05:34, sunset 19:48,
        //       alot 04:06, tzeit 20:30
        //   2026-01-15 (IST, UTC+2): sunrise 06:39, sunset 16:58,
        //       alot 05:23, tzeit 17:37
        //   2026-12-21 (IST, UTC+2): sunrise 06:35, sunset 16:39,
        //       alot 05:17, tzeit 17:19
        // New York 40.7128°N, 74.006°W, America/New_York:
        //   2026-06-21 (EDT, UTC−4): sunrise 05:25, sunset 20:31,
        //       alot 03:36, tzeit 21:21
        //   2026-01-15 (EST, UTC−5): sunrise 07:18, sunset 16:53,
        //       alot 05:52, tzeit 17:38
        //   2026-12-21 (EST, UTC−5): sunrise 07:17, sunset 16:32,
        //       alot 05:48, tzeit 17:17
        // Candle lighting is sunset minus the configured minutes
        // (Jerusalem 40, New York 18). Havdalah with minutes 0 is tzeit.

        let p = prefs(40, 0);
        let jun = day_times(ymd(2026, 6, 21), &jerusalem(), &p);
        assert_eq!(jun.utc_offset_min, 180);
        near(jun.sunrise, 5, 34);
        near(jun.sunset, 19, 48);
        near(jun.alot, 4, 6);
        near(jun.tzeit, 20, 30);
        near(jun.candle_lighting, 19, 8);
        near(jun.havdalah, 20, 30);

        let jan = day_times(ymd(2026, 1, 15), &jerusalem(), &p);
        assert_eq!(jan.utc_offset_min, 120);
        near(jan.sunrise, 6, 39);
        near(jan.sunset, 16, 58);
        near(jan.alot, 5, 23);
        near(jan.tzeit, 17, 37);
        near(jan.candle_lighting, 16, 18);
        assert_eq!(jan.havdalah, jan.tzeit);

        let dec = day_times(ymd(2026, 12, 21), &jerusalem(), &p);
        assert_eq!(dec.utc_offset_min, 120);
        near(dec.sunrise, 6, 35);
        near(dec.sunset, 16, 39);
        near(dec.alot, 5, 17);
        near(dec.tzeit, 17, 19);

        let p_ny = prefs(18, 0);
        let ny_jun = day_times(ymd(2026, 6, 21), &new_york(), &p_ny);
        assert_eq!(ny_jun.utc_offset_min, -240);
        near(ny_jun.sunrise, 5, 25);
        near(ny_jun.sunset, 20, 31);
        near(ny_jun.alot, 3, 36);
        near(ny_jun.tzeit, 21, 21);
        near(ny_jun.candle_lighting, 20, 13);

        let ny_jan = day_times(ymd(2026, 1, 15), &new_york(), &p_ny);
        assert_eq!(ny_jan.utc_offset_min, -300);
        near(ny_jan.sunrise, 7, 18);
        near(ny_jan.sunset, 16, 53);
        near(ny_jan.alot, 5, 52);
        near(ny_jan.tzeit, 17, 38);

        let ny_dec = day_times(ymd(2026, 12, 21), &new_york(), &prefs(18, 42));
        near(ny_dec.sunrise, 7, 17);
        near(ny_dec.sunset, 16, 32);
        near(ny_dec.alot, 5, 48);
        near(ny_dec.tzeit, 17, 17);
        // 42 minutes after sunset, not tzeit.
        near(ny_dec.havdalah, 17, 14);
        assert_ne!(ny_dec.havdalah, ny_dec.tzeit);
    }

    #[test]
    fn polar_day_has_no_sunset() {
        // 69.6°N is north of the Arctic Circle. On 21 June the sun does not set.
        let arctic = place("high-latitude", 69.6, 25.0, "eu", 60);
        let times = day_times(ymd(2026, 6, 21), &arctic, &prefs(18, 0));
        assert!(times.sunset.is_none());
        assert!(times.sunrise.is_none());
        assert!(times.candle_lighting.is_none());
    }

    #[test]
    fn dst_rules_on_switch_dates() {
        let eu = place("eu", 48.86, 2.35, "eu", 60);
        // Last Sunday of March 2026 is the 29th.
        assert_eq!(utc_offset_minutes(&eu, ymd(2026, 3, 29)), 120);
        assert_eq!(utc_offset_minutes(&eu, ymd(2026, 3, 28)), 60);
        // Last Sunday of October 2026 is the 25th: standard for the whole day.
        assert_eq!(utc_offset_minutes(&eu, ymd(2026, 10, 24)), 120);
        assert_eq!(utc_offset_minutes(&eu, ymd(2026, 10, 25)), 60);

        let us = new_york();
        // Second Sunday of March 2026 is the 8th.
        assert_eq!(utc_offset_minutes(&us, ymd(2026, 3, 8)), -240);
        assert_eq!(utc_offset_minutes(&us, ymd(2026, 3, 7)), -300);
        // First Sunday of November 2026 is the 1st: standard.
        assert_eq!(utc_offset_minutes(&us, ymd(2026, 10, 31)), -240);
        assert_eq!(utc_offset_minutes(&us, ymd(2026, 11, 1)), -300);

        let il = jerusalem();
        // Friday before the last Sunday of March 2026 is the 27th.
        assert_eq!(utc_offset_minutes(&il, ymd(2026, 3, 27)), 180);
        assert_eq!(utc_offset_minutes(&il, ymd(2026, 3, 26)), 120);
        assert_eq!(utc_offset_minutes(&il, ymd(2026, 10, 24)), 180);
        assert_eq!(utc_offset_minutes(&il, ymd(2026, 10, 25)), 120);

        // Australia: first Sunday of October 2026 is the 4th; of April, the 5th.
        let au = place("au", -33.87, 151.21, "au", 600);
        assert_eq!(utc_offset_minutes(&au, ymd(2026, 10, 4)), 660);
        assert_eq!(utc_offset_minutes(&au, ymd(2026, 10, 3)), 600);
        assert_eq!(utc_offset_minutes(&au, ymd(2026, 4, 4)), 660);
        assert_eq!(utc_offset_minutes(&au, ymd(2026, 4, 5)), 600);
        // Mid-summer (January) is still daylight time; mid-winter (July) is not.
        assert_eq!(utc_offset_minutes(&au, ymd(2026, 1, 15)), 660);
        assert_eq!(utc_offset_minutes(&au, ymd(2026, 7, 15)), 600);

        let fixed = place("fixed", -34.6, -58.4, "none", -180);
        assert_eq!(utc_offset_minutes(&fixed, ymd(2026, 6, 21)), -180);
        assert_eq!(utc_offset_minutes(&fixed, ymd(2026, 1, 15)), -180);
    }

    #[test]
    fn local_zone_uses_computer_noon() {
        use chrono::{Local, LocalResult, Offset, TimeZone};

        let date = ymd(2026, 6, 21);
        let here = place("here", 0.0, 0.0, "local", 0);
        let got = utc_offset_minutes(&here, date);
        let noon = date.and_hms_opt(12, 0, 0).unwrap();
        let expect = match Local.from_local_datetime(&noon) {
            LocalResult::Single(dt) | LocalResult::Ambiguous(dt, _) => {
                dt.offset().fix().local_minus_utc() / 60
            }
            LocalResult::None => 0,
        };
        assert_eq!(got, expect);
    }

    #[test]
    fn weekday_helper_finds_sundays() {
        assert_eq!(last_sunday(2026, 3), ymd(2026, 3, 29));
        assert_eq!(nth_sunday(2026, 3, 2), ymd(2026, 3, 8));
        assert_eq!(last_sunday(2026, 3) - TimeDelta::days(2), ymd(2026, 3, 27));
    }
}
