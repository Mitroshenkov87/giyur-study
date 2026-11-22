//! Calendar tests: hand-checked known dates, plus sweeps against fixtures
//! produced by an independent implementation (pyluach, via
//! tools/gen_calendar_fixtures.py).

use chrono::{Datelike, Duration, NaiveDate, Weekday};

use super::days::{self, day_info};
use super::hebrew::*;
use super::parasha::{self, reading_on, Reading};

fn g(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

fn h(y: i32, m: u8, d: u8) -> HebrewDate {
    HebrewDate::new(y, m, d)
}

#[test]
fn known_hebrew_dates() {
    let cases = [
        (g(2026, 11, 22), h(5787, KISLEV, 12)),
        (g(2024, 10, 3), h(5785, TISHREI, 1)),
        (g(2000, 1, 1), h(5760, TEVET, 23)),
        (g(1948, 5, 14), h(5708, IYAR, 5)),
        (g(2023, 12, 8), h(5784, KISLEV, 25)),
        (g(2024, 3, 24), h(5784, ADAR_II, 14)),
        (g(2024, 2, 23), h(5784, ADAR, 14)),
        (g(2030, 4, 18), h(5790, NISAN, 15)),
        (g(1900, 1, 1), h(5660, SHEVAT, 1)),
        (g(1900, 10, 1), h(5661, TISHREI, 8)),
    ];
    for (greg, heb) in cases {
        assert_eq!(HebrewDate::from_gregorian(greg), heb, "{greg}");
        assert_eq!(heb.to_gregorian(), greg, "{heb:?}");
    }
}

#[test]
fn round_trip_every_day_1800_2300() {
    let mut d = g(1800, 1, 1);
    let end = g(2300, 1, 1);
    let mut prev = HebrewDate::from_gregorian(d - Duration::days(1));
    while d < end {
        let hd = HebrewDate::from_gregorian(d);
        assert_eq!(hd.to_gregorian(), d);
        assert!(hd.day >= 1 && hd.day <= days_in_month(hd.year, hd.month));
        // Consecutive days advance by exactly one Hebrew day.
        if hd.day > 1 {
            assert_eq!(
                (hd.year, hd.month, hd.day - 1),
                (prev.year, prev.month, prev.day)
            );
        }
        prev = hd;
        d += Duration::days(1);
    }
}

#[test]
fn oracle_hebrew_dates_1900_2100() {
    let data = include_str!("testdata/hebrew_dates.txt");
    let mut n = 0;
    for line in data.lines() {
        let p: Vec<&str> = line.split(' ').collect();
        let greg = NaiveDate::parse_from_str(p[0], "%Y-%m-%d").unwrap();
        let want = h(
            p[1].parse().unwrap(),
            p[2].parse().unwrap(),
            p[3].parse().unwrap(),
        );
        assert_eq!(HebrewDate::from_gregorian(greg), want, "{greg}");
        n += 1;
    }
    assert!(n > 5000);
}

#[test]
fn leap_years_and_lengths() {
    // 19-year cycle: years 3, 6, 8, 11, 14, 17, 19 are leap.
    let leaps: Vec<i32> = (5777..=5795).filter(|y| is_leap(*y)).collect();
    assert_eq!(leaps, vec![5779, 5782, 5784, 5787, 5790, 5793, 5795]);
    assert_eq!(days_in_year(5784), 383);
    assert_eq!(days_in_year(5785), 355);
    assert_eq!(days_in_year(5786), 354);
    assert_eq!(days_in_year(5787), 385);
    for y in 5600..6000 {
        let n = days_in_year(y);
        if is_leap(y) {
            assert!([383, 384, 385].contains(&n), "{y}: {n}");
        } else {
            assert!([353, 354, 355].contains(&n), "{y}: {n}");
        }
        // Rosh Hashana never falls on Sunday, Wednesday or Friday.
        let rh = HebrewDate::new(y, TISHREI, 1).to_gregorian().weekday();
        assert!(![Weekday::Sun, Weekday::Wed, Weekday::Fri].contains(&rh));
    }
    assert_eq!(month_name_ru(5787, ADAR), "Адар I");
    assert_eq!(month_name_ru(5786, ADAR), "Адар");
    assert_eq!(months_in_order(5787).len(), 13);
    assert_eq!(months_in_order(5786).len(), 12);
}

#[test]
fn formatting() {
    assert_eq!(h(5787, KISLEV, 2).format_ru(), "2 кислева 5787");
    assert_eq!(h(5787, KISLEV, 2).format_he(), "ב׳ כסלו תשפ״ז");
    assert_eq!(h(5787, KISLEV, 12).format_he(), "י״ב כסלו תשפ״ז");
    assert_eq!(gematria(15), "ט״ו");
    assert_eq!(gematria(16), "ט״ז");
    assert_eq!(gematria(30), "ל׳");
    assert_eq!(gematria(785), "תשפ״ה");
    assert_eq!(format_gregorian_ru(g(2026, 11, 22)), "22 ноября 2026");
    assert_eq!(weekday_ru(g(2026, 11, 22)), "воскресенье");
}

#[test]
fn major_holidays_5785_5790() {
    // (year, Rosh Hashana, Yom Kippur, 1st day of Pesach, 1st day of Shavuot)
    let cases = [
        (
            5785,
            g(2024, 10, 3),
            g(2024, 10, 12),
            g(2025, 4, 13),
            g(2025, 6, 2),
        ),
        (
            5786,
            g(2025, 9, 23),
            g(2025, 10, 2),
            g(2026, 4, 2),
            g(2026, 5, 22),
        ),
        (
            5787,
            g(2026, 9, 12),
            g(2026, 9, 21),
            g(2027, 4, 22),
            g(2027, 6, 11),
        ),
        (
            5788,
            g(2027, 10, 2),
            g(2027, 10, 11),
            g(2028, 4, 11),
            g(2028, 5, 31),
        ),
        (
            5789,
            g(2028, 9, 21),
            g(2028, 9, 30),
            g(2029, 3, 31),
            g(2029, 5, 20),
        ),
        (
            5790,
            g(2029, 9, 10),
            g(2029, 9, 19),
            g(2030, 4, 18),
            g(2030, 6, 7),
        ),
    ];
    for (y, rh, yk, pesach, shavuot) in cases {
        for diaspora in [true, false] {
            assert!(day_info(rh, diaspora).has("rosh-hashana"), "{y}");
            assert!(day_info(rh + Duration::days(1), diaspora).has("rosh-hashana"));
            let k = day_info(yk, diaspora);
            assert!(k.has("yom-kippur") && k.fast().is_some(), "{y}");
            assert!(day_info(pesach, diaspora).has("pesach"), "{y}");
            assert!(day_info(pesach - Duration::days(1), diaspora).has("erev-pesach"));
            assert!(day_info(shavuot, diaspora).has("shavuot"), "{y}");
        }
        // Second day of Pesach / Shavuot: yom tov only in the diaspora.
        assert!(day_info(pesach + Duration::days(1), true).is_yom_tov());
        assert!(!day_info(pesach + Duration::days(1), false).is_yom_tov());
        assert!(day_info(shavuot + Duration::days(1), true).has("shavuot"));
        assert!(!day_info(shavuot + Duration::days(1), false).has("shavuot"));
        // 8th day of Pesach.
        assert!(day_info(pesach + Duration::days(7), true).has("pesach-7"));
        assert!(!day_info(pesach + Duration::days(7), false).is_yom_tov());
    }
}

#[test]
fn fasts_and_postponements() {
    // 9 Av 5785 = Shabbat 2 Aug 2025 → fast moved to Sunday 3 Aug.
    assert!(!day_info(g(2025, 8, 2), true).has("tisha-beav"));
    assert!(day_info(g(2025, 8, 3), true).has("tisha-beav"));
    // 9 Av 5786 = Thursday 23 Jul 2026.
    assert!(day_info(g(2026, 7, 23), true).has("tisha-beav"));
    // Fast of Esther 5785: 13 Adar = Thursday 13 Mar 2025.
    assert!(day_info(g(2025, 3, 13), true).has("taanit-ester"));
    // Fast of Esther 5784: 13 Adar II = Shabbat 23 Mar 2024 → Thursday 21 Mar.
    assert!(day_info(g(2024, 3, 21), true).has("taanit-ester"));
    // Fast of Gedaliah 5787: 3 Tishrei = Monday 14 Sep 2026.
    assert!(day_info(g(2026, 9, 14), true).has("tzom-gedalia"));
    // 10 Tevet can fall on Friday: 5784 → Friday 22 Dec 2023.
    assert!(day_info(g(2023, 12, 22), true).has("asara-betevet"));
    // Yom Kippur is a major fast (starts the evening before).
    assert_eq!(
        day_info(g(2025, 10, 2), true).fast().unwrap().kind,
        days::Kind::MajorFast
    );
    // Ordinary day: no fast.
    assert!(day_info(g(2026, 11, 22), true).fast().is_none());
}

#[test]
fn modern_and_minor_days() {
    // 2025: 5 Iyar = Shabbat → Yom HaAtzmaut Thursday 1 May, Zikaron 30 Apr.
    assert!(day_info(g(2025, 5, 1), false).has("yom-haatzmaut"));
    assert!(day_info(g(2025, 4, 30), false).has("yom-hazikaron"));
    assert!(day_info(g(2025, 4, 24), false).has("yom-hashoah"));
    // 2024: 5 Iyar = Monday → moved to Tuesday 14 May.
    assert!(day_info(g(2024, 5, 14), false).has("yom-haatzmaut"));
    // Lag BaOmer 5785: 16 May 2025; Omer day 33.
    let lag = day_info(g(2025, 5, 16), true);
    assert!(lag.has("lag-baomer"));
    assert_eq!(lag.omer, Some(33));
    assert_eq!(day_info(g(2025, 4, 14), true).omer, Some(1));
    assert_eq!(day_info(g(2025, 6, 1), true).omer, Some(49));
    assert_eq!(day_info(g(2025, 6, 2), true).omer, None);
    // Three Weeks / Nine Days 5785: 17 Tammuz = 13 Jul 2025, 1 Av = 26 Jul.
    assert!(day_info(g(2025, 7, 20), true).has("three-weeks"));
    assert!(day_info(g(2025, 7, 28), true).has("nine-days"));
    // Chanukah 5787: 25 Kislev = Shabbat 5 Dec 2026; 8 days.
    assert!(day_info(g(2026, 12, 4), true).has("erev-chanuka"));
    assert!(day_info(g(2026, 12, 5), true).has("chanuka"));
    assert!(day_info(g(2026, 12, 12), true).has("chanuka"));
    assert!(!day_info(g(2026, 12, 13), true).has("chanuka"));
    // Rosh Chodesh Tevet 5787 (Kislev 5787 has 30 days): 10–11 Dec 2026.
    assert!(day_info(g(2026, 12, 10), true).has("rosh-chodesh"));
    assert!(day_info(g(2026, 12, 11), true).has("rosh-chodesh"));
    // Purim Katan only in a leap year.
    assert!(day_info(g(2024, 2, 23), true).has("purim-katan"));
    assert!(day_info(g(2024, 3, 24), true).has("purim"));
    // Tu BiShvat 5786: 2 Feb 2026; Tu BeAv 5785: 9 Aug 2025.
    assert!(day_info(g(2026, 2, 2), true).has("tu-bishvat"));
    assert!(day_info(g(2025, 8, 9), true).has("tu-beav"));
}

/// Our festival/fast ids per day must equal the oracle's for 5780–5800.
#[test]
fn oracle_festivals_5780_5800() {
    use std::collections::{BTreeSet, HashMap};
    let data = include_str!("testdata/festivals.txt");
    let mut want: HashMap<(NaiveDate, bool), BTreeSet<String>> = HashMap::new();
    for line in data.lines() {
        let p: Vec<&str> = line.split(' ').collect();
        let d = NaiveDate::parse_from_str(p[0], "%Y-%m-%d").unwrap();
        let ids = p[2].split(',').map(|s| s.to_string()).collect();
        want.insert((d, p[1] == "D"), ids);
    }
    let map = |id: &str| -> Option<&'static str> {
        Some(match id {
            "rosh-hashana" => "rosh-hashana",
            "yom-kippur" => "yom-kippur",
            "sukkot" | "chol-hamoed-sukkot" | "hoshana-raba" => "sukkot*",
            "shmini-atzeret" => "shmini-atzeret",
            "simchat-tora" => "simchat-tora",
            "chanuka" => "chanuka",
            "tu-bishvat" => "tu-bishvat",
            "purim-katan" => "purim-katan",
            "purim" => "purim",
            "shushan-purim" => "shushan-purim",
            "pesach" | "chol-hamoed-pesach" | "pesach-7" => "pesach*",
            "pesach-sheni" => "pesach-sheni",
            "lag-baomer" => "lag-baomer",
            "shavuot" => "shavuot",
            "tu-beav" => "tu-beav",
            "tzom-gedalia" => "tzom-gedalia",
            "asara-betevet" => "asara-betevet",
            "taanit-ester" => "taanit-ester",
            "shiva-asar-betamuz" => "shiva-asar-betamuz",
            "tisha-beav" => "tisha-beav",
            _ => return None,
        })
    };
    let mut d = HebrewDate::new(5780, TISHREI, 1).to_gregorian();
    let end = HebrewDate::new(5801, TISHREI, 1).to_gregorian();
    let mut days_checked = 0;
    while d < end {
        for diaspora in [true, false] {
            let got: BTreeSet<String> = day_info(d, diaspora)
                .events
                .iter()
                .filter_map(|e| map(e.id))
                .map(|s| s.to_string())
                .collect();
            let w = want.get(&(d, diaspora)).cloned().unwrap_or_default();
            assert_eq!(got, w, "{d} diaspora={diaspora}");
        }
        days_checked += 1;
        d += Duration::days(1);
    }
    assert!(days_checked > 7000);
}

#[test]
fn parasha_known_shabbatot() {
    // Vayishlach, 28 Nov 2026 (5787).
    assert_eq!(reading_on(g(2026, 11, 28), true), Reading::Parasha(7));
    // Vayera, 16 Nov 2024 (5785).
    assert_eq!(reading_on(g(2024, 11, 16), true), Reading::Parasha(3));
    // 5785 (non-leap, Rosh Hashana on Thursday, 355 days): Vayakhel and
    // Pekudei read separately; 5786: together.
    assert_eq!(reading_on(g(2025, 3, 22), true), Reading::Parasha(21));
    assert_eq!(reading_on(g(2025, 3, 29), true), Reading::Parasha(22));
    assert_eq!(reading_on(g(2026, 3, 14), true), Reading::Double(21, 22));
    assert_eq!(reading_on(g(2026, 4, 25), true), Reading::Double(28, 29));
    // Pekudei alone, 16 Mar 2024 (leap 5784).
    assert_eq!(reading_on(g(2024, 3, 16), true), Reading::Parasha(22));
    // Bereshit, 26 Oct 2024; Haazinu on Shabbat Shuva... 5785: 5 Oct 2024? No:
    // RH Thu 3 Oct → Shabbat 5 Oct = Haazinu.
    assert_eq!(reading_on(g(2024, 10, 26), true), Reading::Parasha(0));
    assert_eq!(reading_on(g(2024, 10, 5), true), Reading::Parasha(52));
    // Shabbat Chol HaMoed Sukkot: festival reading.
    assert_eq!(reading_on(g(2024, 10, 19), true), Reading::Festival);
    // Israel/diaspora split: 27 Apr 2019 = 8th day of Pesach in the diaspora,
    // Acharei Mot in Israel; Israel stays a week ahead until Matot/Masei.
    assert_eq!(reading_on(g(2019, 4, 27), true), Reading::Festival);
    assert_eq!(reading_on(g(2019, 4, 27), false), Reading::Parasha(28));
    assert_eq!(reading_on(g(2019, 6, 8), true), Reading::Parasha(33)); // Bamidbar
    assert_eq!(reading_on(g(2019, 6, 8), false), Reading::Parasha(34)); // Nasso
    assert_eq!(reading_on(g(2019, 8, 3), true), Reading::Double(41, 42));
    assert_eq!(reading_on(g(2019, 8, 3), false), Reading::Parasha(42));
    // A weekday maps to the coming Shabbat.
    assert_eq!(reading_on(g(2026, 11, 22), true), Reading::Parasha(7));
    let (sat, r, next) = parasha::weekly(g(2024, 10, 17), true);
    assert_eq!(sat, g(2024, 10, 19));
    assert_eq!(r, Reading::Festival);
    assert_eq!(next, Some((g(2024, 10, 26), Reading::Parasha(0))));
}

#[test]
fn oracle_parashot_1990_2060() {
    let data = include_str!("testdata/parashot.txt");
    let parse = |s: &str| -> Reading {
        if s == "-" {
            return Reading::Festival;
        }
        let v: Vec<u8> = s.split('+').map(|x| x.parse().unwrap()).collect();
        if v.len() == 1 {
            Reading::Parasha(v[0])
        } else {
            Reading::Double(v[0], v[1])
        }
    };
    let mut n = 0;
    for line in data.lines() {
        let p: Vec<&str> = line.split(' ').collect();
        let d = NaiveDate::parse_from_str(p[0], "%Y-%m-%d").unwrap();
        assert_eq!(d.weekday(), Weekday::Sat);
        assert_eq!(reading_on(d, true), parse(p[1]), "{d} diaspora");
        assert_eq!(reading_on(d, false), parse(p[2]), "{d} israel");
        n += 1;
    }
    assert!(n > 3600);
}

#[test]
fn every_year_reads_the_whole_torah() {
    for y in 5700..5900 {
        for diaspora in [true, false] {
            let mut seen = [false; 54];
            let mut d = parasha::next_shabbat(HebrewDate::new(y, TISHREI, 1).to_gregorian());
            let end = HebrewDate::new(y + 1, TISHREI, 1).to_gregorian();
            while d < end {
                for i in reading_on(d, diaspora).indices() {
                    // Vayelech can be read in both autumns of one year.
                    assert!(i == 51 || !seen[i as usize], "{y} {i} twice");
                    seen[i as usize] = true;
                }
                d += Duration::days(7);
            }
            // Vezot HaBeracha is read on Simchat Torah, not on a Shabbat;
            // Vayelech may be read in the previous/next year's autumn.
            let missing: Vec<usize> = (0..54).filter(|i| !seen[*i]).collect();
            assert!(
                missing.iter().all(|i| [51, 52, 53].contains(i)),
                "{y}: {missing:?}"
            );
        }
    }
}

#[test]
fn special_shabbatot_5785() {
    let has = |d, id| day_info(d, true).has(id);
    assert!(has(g(2024, 10, 5), "shabbat-shuva"));
    assert!(has(g(2025, 3, 1), "shabbat-shekalim"));
    assert!(has(g(2025, 3, 8), "shabbat-zachor"));
    assert!(has(g(2025, 3, 22), "shabbat-para"));
    assert!(has(g(2025, 3, 29), "shabbat-hachodesh"));
    assert!(has(g(2025, 4, 12), "shabbat-hagadol"));
    assert!(has(g(2025, 8, 2), "shabbat-chazon"));
    assert!(has(g(2025, 8, 9), "shabbat-nachamu"));
    assert!(has(g(2025, 2, 8), "shabbat-shira"));
}

#[test]
fn upcoming_and_candles() {
    // From 22 Nov 2026 the next occasion is Chanukah (5 Dec).
    let (d, e) = days::next_holiday_or_fast(g(2026, 11, 22), true).unwrap();
    assert_eq!((d, e.id), (g(2026, 12, 5), "chanuka"));
    // During Chanukah, the next occasion is the 10 Tevet fast.
    let (d, e) = days::next_holiday_or_fast(g(2026, 12, 8), true).unwrap();
    assert_eq!(e.id, "asara-betevet");
    assert_eq!(d, HebrewDate::new(5787, TEVET, 10).to_gregorian());
    // Friday → candles; Shabbat → havdalah; erev Pesach → candles.
    assert!(days::candles_tonight(g(2026, 11, 27), true));
    assert!(days::havdalah_tonight(g(2026, 11, 28), true));
    assert!(days::candles_tonight(g(2025, 4, 12), true));
    // 1st day of Pesach in the diaspora: candles again (2nd day), no havdalah.
    assert!(days::candles_tonight(g(2025, 4, 13), true));
    assert!(!days::havdalah_tonight(g(2025, 4, 13), true));
    assert!(days::havdalah_tonight(g(2025, 4, 13), false));
}

#[test]
fn texts_cover_every_portion_and_day() {
    use super::texts::Texts;
    let t = Texts::get();
    for (i, (id, _, _)) in parasha::PARASHOT.iter().enumerate() {
        let p = t.parasha(i as u8).unwrap_or_else(|| panic!("no text for {id}"));
        assert_eq!(p.daily.len(), 7, "{id}");
        assert!(!p.summary.is_empty() && !p.theme.is_empty(), "{id}");
    }
    // Every event id the calendar can produce over a full 19-year cycle has text.
    let mut d = g(2024, 9, 1);
    while d < g(2044, 9, 1) {
        for diaspora in [true, false] {
            for e in day_info(d, diaspora).events {
                assert!(t.day(e.id).is_some(), "no text for {}", e.id);
            }
        }
        d += Duration::days(1);
    }
    assert!(t.day("weekday").is_some());
    for y in [5786, 5787] {
        for m in months_in_order(y) {
            assert!(t.month(y, m).is_some(), "{y}/{m}");
        }
    }
    // Jewish framing only.
    let all = format!(
        "{}{}",
        include_str!("../../content/calendar/parashot.json"),
        include_str!("../../content/calendar/days.json")
    )
    .to_lowercase();
    for bad in ["библи", "ветхий завет", "новый завет", "христ", "моисей"] {
        assert!(!all.contains(bad), "{bad}");
    }
}
