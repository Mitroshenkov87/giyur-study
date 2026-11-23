//! What a given day is in the Jewish calendar: Shabbat, festivals, Chol
//! HaMoed, Rosh Chodesh, fasts (with the standard postponement rules), the
//! Omer, the Three Weeks / Nine Days, minor and modern days and the special
//! Shabbatot. Israel vs diaspora is a parameter.

use chrono::{Datelike, Duration, NaiveDate, Weekday};

use crate::calendar::hebrew::*;
use crate::i18n::tr;
use crate::trf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Shabbat,
    /// Festival day with work restrictions (yom tov).
    YomTov,
    CholHamoed,
    /// Eve of Shabbat or a festival (candle lighting in the evening).
    Erev,
    RoshChodesh,
    /// Starts the evening before (Yom Kippur, Tisha BeAv).
    MajorFast,
    /// Dawn to nightfall.
    MinorFast,
    /// Chanukah, Purim, Tu BiShvat, Lag BaOmer, Tu BeAv …
    Minor,
    /// Israeli national memorial and festive days.
    Modern,
    SpecialShabbat,
    /// A season: Omer, the Three Weeks, the Nine Days, Elul, the Ten Days.
    Period,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    /// Key into content/ru/calendar/days.json.
    pub id: &'static str,
    pub kind: Kind,
    /// Russian title for this day, e.g. «Ханука, 3-я свеча».
    pub title: String,
}

impl Event {
    fn new(id: &'static str, kind: Kind, title: impl Into<String>) -> Event {
        Event {
            id,
            kind,
            title: title.into(),
        }
    }

    pub fn is_fast(&self) -> bool {
        matches!(self.kind, Kind::MajorFast | Kind::MinorFast)
    }

    /// Worth a countdown: festivals, fasts, minor and modern days.
    pub fn is_notable(&self) -> bool {
        matches!(
            self.kind,
            Kind::YomTov | Kind::MajorFast | Kind::MinorFast | Kind::Minor | Kind::Modern
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DayInfo {
    pub date: NaiveDate,
    pub hebrew: HebrewDate,
    pub diaspora: bool,
    /// Most important first.
    pub events: Vec<Event>,
    /// Day of the Omer (1..=49) counted on the evening before this date's
    /// daytime — i.e. the count said at nightfall that begins this date.
    pub omer: Option<u8>,
}

impl DayInfo {
    pub fn fast(&self) -> Option<&Event> {
        self.events.iter().find(|e| e.is_fast())
    }

    pub fn has(&self, id: &str) -> bool {
        self.events.iter().any(|e| e.id == id)
    }

    pub fn is_yom_tov(&self) -> bool {
        self.events.iter().any(|e| e.kind == Kind::YomTov)
    }

    /// Shabbat or yom tov (work restrictions).
    pub fn is_rest_day(&self) -> bool {
        self.date.weekday() == Weekday::Sat || self.is_yom_tov()
    }
}

fn hd(year: i32, month: u8, day: u8) -> NaiveDate {
    HebrewDate::new(year, month, day).to_gregorian()
}

fn wd(d: NaiveDate) -> Weekday {
    d.weekday()
}

/// Purim's Adar: Adar II in a leap year.
pub fn purim_month(year: i32) -> u8 {
    if is_leap(year) {
        ADAR_II
    } else {
        ADAR
    }
}

/// Fast of Gedaliah: 3 Tishrei, Sunday 4 Tishrei when the 3rd is Shabbat.
pub fn tzom_gedalia(year: i32) -> NaiveDate {
    let d = hd(year, TISHREI, 3);
    if wd(d) == Weekday::Sat {
        d + Duration::days(1)
    } else {
        d
    }
}

/// Fast of Esther: 13 Adar, moved back to Thursday 11 Adar when on Shabbat.
pub fn taanit_esther(year: i32) -> NaiveDate {
    let d = hd(year, purim_month(year), 13);
    if wd(d) == Weekday::Sat {
        d - Duration::days(2)
    } else {
        d
    }
}

/// Fast of the Firstborn: 14 Nisan, Thursday 12 Nisan when the 14th is Shabbat.
pub fn taanit_bechorot(year: i32) -> NaiveDate {
    let d = hd(year, NISAN, 14);
    if wd(d) == Weekday::Sat {
        d - Duration::days(2)
    } else {
        d
    }
}

/// 17 Tammuz, Sunday 18 Tammuz when on Shabbat.
pub fn shiva_asar_betammuz(year: i32) -> NaiveDate {
    let d = hd(year, TAMMUZ, 17);
    if wd(d) == Weekday::Sat {
        d + Duration::days(1)
    } else {
        d
    }
}

/// 9 Av, Sunday 10 Av when on Shabbat.
pub fn tisha_beav(year: i32) -> NaiveDate {
    let d = hd(year, AV, 9);
    if wd(d) == Weekday::Sat {
        d + Duration::days(1)
    } else {
        d
    }
}

/// Yom HaShoah: 27 Nisan; Thursday 26 if Friday, Monday 28 if Sunday
/// (observed from 5711 / 1951).
pub fn yom_hashoah(year: i32) -> Option<NaiveDate> {
    if year < 5711 {
        return None;
    }
    let d = hd(year, NISAN, 27);
    Some(match wd(d) {
        Weekday::Fri => d - Duration::days(1),
        Weekday::Sun => d + Duration::days(1),
        _ => d,
    })
}

/// Yom HaAtzmaut: 5 Iyar; moved to Thursday when on Friday/Shabbat,
/// and to Tuesday when on Monday (since 5764 / 2004). From 5708.
pub fn yom_haatzmaut(year: i32) -> Option<NaiveDate> {
    if year < 5708 {
        return None;
    }
    let d = hd(year, IYAR, 5);
    Some(match wd(d) {
        Weekday::Fri => d - Duration::days(1),
        Weekday::Sat => d - Duration::days(2),
        Weekday::Mon if year >= 5764 => d + Duration::days(1),
        _ => d,
    })
}

/// Yom HaZikaron: the day before Yom HaAtzmaut.
pub fn yom_hazikaron(year: i32) -> Option<NaiveDate> {
    yom_haatzmaut(year).map(|d| d - Duration::days(1))
}

/// Yom Yerushalayim: 28 Iyar (from 5728 / 1968).
pub fn yom_yerushalayim(year: i32) -> Option<NaiveDate> {
    (year >= 5728).then(|| hd(year, IYAR, 28))
}

/// The Shabbat on or before `d`.
fn shabbat_on_or_before(d: NaiveDate) -> NaiveDate {
    let back = (wd(d).num_days_from_sunday() + 1) % 7; // Sat → 0
    d - Duration::days(back as i64)
}

/// The last Shabbat strictly before `d`.
fn shabbat_before(d: NaiveDate) -> NaiveDate {
    shabbat_on_or_before(d - Duration::days(1))
}

/// Everything about one date.
pub fn day_info(date: NaiveDate, diaspora: bool) -> DayInfo {
    let h = HebrewDate::from_gregorian(date);
    let (y, m, d) = (h.year, h.month, h.day);
    let leap = is_leap(y);
    let w = wd(date);
    let mut ev: Vec<Event> = Vec::new();
    let mut push =
        |id: &'static str, kind: Kind, title: String| ev.push(Event::new(id, kind, title));

    // --- Tishrei ---
    if m == TISHREI {
        match d {
            1 | 2 => {
                let n = d as u32;
                push(
                    "rosh-hashana",
                    Kind::YomTov,
                    trf!("Рош а-Шана, {n}-й день", "Rosh Hashanah, day {n}"),
                );
            }
            10 => push("yom-kippur", Kind::MajorFast, tr("Йом Кипур").to_string()),
            9 => push(
                "erev-yom-kippur",
                Kind::Erev,
                tr("Канун Йом Кипура").to_string(),
            ),
            14 => push("erev-sukkot", Kind::Erev, tr("Канун Суккота").to_string()),
            15 => push("sukkot", Kind::YomTov, tr("Суккот, 1-й день").to_string()),
            16 if diaspora => push("sukkot", Kind::YomTov, tr("Суккот, 2-й день").to_string()),
            16..=20 => {
                let n = d as u32 - 14;
                push(
                    "chol-hamoed-sukkot",
                    Kind::CholHamoed,
                    trf!(
                        "Холь а-моэд Суккот, {n}-й день",
                        "Chol HaMoed Sukkot, day {n}"
                    ),
                );
            }
            21 => push(
                "hoshana-raba",
                Kind::CholHamoed,
                tr("Ошана Раба (7-й день Суккота)").to_string(),
            ),
            22 if diaspora => push(
                "shmini-atzeret",
                Kind::YomTov,
                tr("Шмини Ацерет").to_string(),
            ),
            22 => push(
                "shmini-atzeret",
                Kind::YomTov,
                tr("Шмини Ацерет и Симхат Тора").to_string(),
            ),
            23 if diaspora => push("simchat-tora", Kind::YomTov, tr("Симхат Тора").to_string()),
            _ => {}
        }
        if date == tzom_gedalia(y) {
            push(
                "tzom-gedalia",
                Kind::MinorFast,
                tr("Пост Гедальи").to_string(),
            );
        }
        if (1..=10).contains(&d) {
            let n = d as u32;
            push(
                "aseret-yemei-teshuva",
                Kind::Period,
                trf!(
                    "Десять дней трепета, {n}-й день",
                    "Ten Days of Repentance, day {n}"
                ),
            );
        }
    }
    // --- Chanukah: 25 Kislev + 8 days ---
    let chanuka_start = hd(if m >= TISHREI { y } else { y - 1 }, KISLEV, 25);
    let since = (date - chanuka_start).num_days();
    if (0..8).contains(&since) {
        let title = if since < 7 {
            let day = since as u32 + 1;
            let candles = since + 2;
            // Shamash plus the night's candles: 2, 3, 4 → «свечи», 5+ → «свечей».
            let word = crate::util::plural_l(
                candles,
                &["свеча", "свечи", "свечей"],
                &["candle", "candles"],
            );
            trf!(
                "Ханука, {day}-й день: вечером зажигают {candles} {word}",
                "Chanukah, day {day}: light {candles} {word} in the evening"
            )
        } else {
            tr("Ханука, 8-й день").to_string()
        };
        push("chanuka", Kind::Minor, title);
    }
    if date == chanuka_start - Duration::days(1) {
        push(
            "erev-chanuka",
            Kind::Erev,
            tr("Канун Хануки: вечером первая свеча").to_string(),
        );
    }
    if m == TEVET && d == 10 {
        push(
            "asara-betevet",
            Kind::MinorFast,
            tr("Пост 10 тевета").to_string(),
        );
    }
    if m == SHEVAT && d == 15 {
        push(
            "tu-bishvat",
            Kind::Minor,
            tr("Ту би-Шват — Новый год деревьев").to_string(),
        );
    }
    // --- Adar ---
    let pm = purim_month(y);
    if leap && m == ADAR && d == 14 {
        push("purim-katan", Kind::Minor, tr("Пурим Катан").to_string());
    }
    if date == taanit_esther(y) {
        push(
            "taanit-ester",
            Kind::MinorFast,
            tr("Пост Эстер").to_string(),
        );
    }
    if m == pm && d == 14 {
        push("purim", Kind::Minor, tr("Пурим").to_string());
    }
    if m == pm && d == 15 {
        push(
            "shushan-purim",
            Kind::Minor,
            tr("Шушан Пурим (Пурим в Иерусалиме)").to_string(),
        );
    }
    // --- Nisan ---
    if m == NISAN {
        match d {
            14 => push(
                "erev-pesach",
                Kind::Erev,
                tr("Канун Песаха: вечером седер").to_string(),
            ),
            15 => push("pesach", Kind::YomTov, tr("Песах, 1-й день").to_string()),
            16 if diaspora => push(
                "pesach",
                Kind::YomTov,
                tr("Песах, 2-й день (второй седер)").to_string(),
            ),
            21 => push(
                "pesach-7",
                Kind::YomTov,
                tr("Седьмой день Песаха").to_string(),
            ),
            22 if diaspora => push(
                "pesach-7",
                Kind::YomTov,
                tr("Последний день Песаха").to_string(),
            ),
            _ => {}
        }
        let chm_start = if diaspora { 17 } else { 16 };
        if (chm_start..=20).contains(&d) {
            let n = d as u32 - 14;
            push(
                "chol-hamoed-pesach",
                Kind::CholHamoed,
                trf!(
                    "Холь а-моэд Песах, {n}-й день",
                    "Chol HaMoed Pesach, day {n}"
                ),
            );
        }
    }
    if date == taanit_bechorot(y) {
        push(
            "taanit-bechorot",
            Kind::MinorFast,
            tr("Пост первенцев").to_string(),
        );
    }
    // --- Omer: from the 2nd night of Pesach (16 Nisan) to 5 Sivan ---
    let omer_start = hd(y, NISAN, 16);
    let od = (date - omer_start).num_days() + 1;
    let omer = (1..=49).contains(&od).then_some(od as u8);
    if let Some(n) = omer {
        // The count for day n is said at nightfall *before* this date.
        push(
            "omer",
            Kind::Period,
            trf!("Счёт омера: {n}-й день", "Omer count: day {n}"),
        );
    }
    if m == IYAR && d == 14 {
        push("pesach-sheni", Kind::Minor, tr("Песах Шени").to_string());
    }
    if m == IYAR && d == 18 {
        push("lag-baomer", Kind::Minor, tr("Лаг ба-Омер").to_string());
    }
    if Some(date) == yom_hashoah(y) {
        push(
            "yom-hashoah",
            Kind::Modern,
            tr("Йом а-Шоа — День памяти Катастрофы").to_string(),
        );
    }
    if Some(date) == yom_hazikaron(y) {
        push(
            "yom-hazikaron",
            Kind::Modern,
            tr("Йом а-Зикарон — День памяти павших").to_string(),
        );
    }
    if Some(date) == yom_haatzmaut(y) {
        push(
            "yom-haatzmaut",
            Kind::Modern,
            tr("Йом а-Ацмаут — День независимости Израиля").to_string(),
        );
    }
    if Some(date) == yom_yerushalayim(y) {
        push(
            "yom-yerushalayim",
            Kind::Modern,
            tr("Йом Иерушалаим — День Иерусалима").to_string(),
        );
    }
    // --- Sivan ---
    if m == SIVAN {
        match d {
            5 => push("erev-shavuot", Kind::Erev, tr("Канун Шавуота").to_string()),
            6 => push("shavuot", Kind::YomTov, tr("Шавуот, 1-й день").to_string()),
            7 if diaspora => push("shavuot", Kind::YomTov, tr("Шавуот, 2-й день").to_string()),
            _ => {}
        }
    }
    // --- Three Weeks / Nine Days / 9 Av ---
    let t17 = hd(y, TAMMUZ, 17);
    let av9 = hd(y, AV, 9);
    let fast17 = shiva_asar_betammuz(y);
    let fast9 = tisha_beav(y);
    if date == fast17 {
        push(
            "shiva-asar-betamuz",
            Kind::MinorFast,
            tr("Пост 17 тамуза").to_string(),
        );
    }
    if date == fast9 {
        push(
            "tisha-beav",
            Kind::MajorFast,
            tr("Тиша бе-Ав — 9 ава").to_string(),
        );
    }
    if date >= t17 && date <= av9 && date != fast9 {
        if m == AV {
            let n = d as u32;
            push(
                "nine-days",
                Kind::Period,
                trf!(
                    "Девять дней (траур по Храму), {n}-й день",
                    "The Nine Days (mourning for the Temple), day {n}"
                ),
            );
        } else {
            push(
                "three-weeks",
                Kind::Period,
                tr("Три недели (бейн а-мецарим)").to_string(),
            );
        }
    }
    if m == AV && d == 15 {
        push("tu-beav", Kind::Minor, tr("Ту бе-Ав").to_string());
    }
    if m == ELUL {
        push(
            "elul",
            Kind::Period,
            tr("Месяц элул: время тшувы").to_string(),
        );
        if d == 29 {
            push(
                "erev-rosh-hashana",
                Kind::Erev,
                tr("Канун Рош а-Шана").to_string(),
            );
        }
    }
    // --- Rosh Chodesh: day 30 of the previous month and/or day 1 ---
    if (d == 1 && m != TISHREI) || d == 30 {
        let (ry, rm) = if d == 30 { next_month(y, m) } else { (y, m) };
        // `month_name_ru_gen` already returns the name in the interface language.
        let month = month_name_ru_gen(ry, rm);
        push(
            "rosh-chodesh",
            Kind::RoshChodesh,
            trf!("Рош Ходеш {month}", "Rosh Chodesh {month}"),
        );
    }
    // --- Shabbat and special Shabbatot ---
    if w == Weekday::Sat {
        push("shabbat", Kind::Shabbat, tr("Шаббат").to_string());
        for (id, title) in special_shabbat(date, y, diaspora) {
            push(id, Kind::SpecialShabbat, title.to_string());
        }
    }
    if w == Weekday::Fri {
        push(
            "erev-shabbat",
            Kind::Erev,
            tr("Канун Шаббата: вечером зажигают свечи").to_string(),
        );
    }

    ev.sort_by_key(|e| rank(e.kind));
    DayInfo {
        date,
        hebrew: h,
        diaspora,
        events: ev,
        omer,
    }
}

fn rank(k: Kind) -> u8 {
    match k {
        Kind::MajorFast => 0,
        Kind::YomTov => 1,
        Kind::CholHamoed => 2,
        Kind::MinorFast => 3,
        Kind::Minor => 4,
        Kind::Modern => 5,
        Kind::RoshChodesh => 6,
        Kind::SpecialShabbat => 7,
        Kind::Shabbat => 8,
        Kind::Erev => 9,
        Kind::Period => 10,
    }
}

fn next_month(y: i32, m: u8) -> (i32, u8) {
    if m == ELUL {
        (y + 1, TISHREI)
    } else if m == last_month(y) {
        (y, NISAN)
    } else {
        (y, m + 1)
    }
}

fn special_shabbat(date: NaiveDate, y: i32, diaspora: bool) -> Vec<(&'static str, &'static str)> {
    let mut v = Vec::new();
    let h = HebrewDate::from_gregorian(date);
    if h.month == TISHREI && (3..=9).contains(&h.day) {
        v.push(("shabbat-shuva", tr("Шаббат Шува")));
    }
    let pm = purim_month(y);
    if date == shabbat_on_or_before(hd(y, pm, 1)) {
        v.push(("shabbat-shekalim", tr("Шаббат Шкалим")));
    }
    if date == shabbat_before(hd(y, pm, 14)) {
        v.push(("shabbat-zachor", tr("Шаббат Захор")));
    }
    let hachodesh = shabbat_on_or_before(hd(y, NISAN, 1));
    if date == hachodesh {
        v.push(("shabbat-hachodesh", tr("Шаббат а-Ходеш")));
    }
    if date == hachodesh - Duration::days(7) {
        v.push(("shabbat-para", tr("Шаббат Пара")));
    }
    if date == shabbat_before(hd(y, NISAN, 15)) {
        v.push(("shabbat-hagadol", tr("Шаббат а-Гадоль")));
    }
    let av9 = hd(y, AV, 9);
    if date == shabbat_on_or_before(av9) {
        v.push(("shabbat-chazon", tr("Шаббат Хазон")));
    }
    if date == shabbat_before(av9) + Duration::days(7) {
        v.push(("shabbat-nachamu", tr("Шаббат Нахаму")));
    }
    if let crate::calendar::parasha::Reading::Parasha(15) =
        crate::calendar::parasha::reading_on(date, diaspora)
    {
        v.push(("shabbat-shira", tr("Шаббат Шира")));
    }
    v
}

/// Upcoming notable days (festivals, fasts, minor and modern days), one entry
/// per occasion (the first day of a multi-day festival), starting at `from`.
pub fn upcoming(from: NaiveDate, diaspora: bool, count: usize) -> Vec<(NaiveDate, Event)> {
    let mut out: Vec<(NaiveDate, Event)> = Vec::new();
    let mut seen_prev: Vec<&'static str> = Vec::new();
    let mut date = from;
    for _ in 0..400 {
        let info = day_info(date, diaspora);
        let mut today_ids = Vec::new();
        for e in info.events {
            if !e.is_notable() {
                continue;
            }
            today_ids.push(e.id);
            if !seen_prev.contains(&e.id) && out.len() < count {
                out.push((date, e));
            }
        }
        seen_prev = today_ids;
        if out.len() >= count {
            break;
        }
        date += Duration::days(1);
    }
    out
}

/// The next festival or fast that starts after `from` (an occasion already
/// under way on `from` is skipped).
pub fn next_holiday_or_fast(from: NaiveDate, diaspora: bool) -> Option<(NaiveDate, Event)> {
    upcoming(from, diaspora, 12)
        .into_iter()
        .find(|(d, _)| *d > from)
}

/// Does candle lighting happen this evening (Friday or the eve of yom tov)?
pub fn candles_tonight(date: NaiveDate, diaspora: bool) -> bool {
    let tomorrow = day_info(date + Duration::days(1), diaspora);
    tomorrow.date.weekday() == Weekday::Sat || tomorrow.is_yom_tov() || tomorrow.has("yom-kippur")
}

/// Does Shabbat or a yom tov end this evening (havdalah)?
pub fn havdalah_tonight(date: NaiveDate, diaspora: bool) -> bool {
    let today = day_info(date, diaspora);
    let tomorrow = day_info(date + Duration::days(1), diaspora);
    (today.is_rest_day() || today.has("yom-kippur")) && !tomorrow.is_rest_day()
}
