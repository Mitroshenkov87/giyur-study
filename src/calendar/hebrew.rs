//! Hebrew calendar arithmetic (Dershowitz & Reingold, *Calendrical
//! Calculations*, ch. 8). Months are numbered from Nisan = 1 … Adar = 12,
//! Adar II = 13 (in a leap year month 12 is Adar I). The year starts on
//! 1 Tishrei (month 7).
//!
//! Fixed day numbers ("R.D.") are chrono's `num_days_from_ce()`
//! (0001-01-01 Gregorian = 1).

use chrono::{Datelike, NaiveDate};

pub const NISAN: u8 = 1;
pub const IYAR: u8 = 2;
pub const SIVAN: u8 = 3;
pub const TAMMUZ: u8 = 4;
pub const AV: u8 = 5;
pub const ELUL: u8 = 6;
pub const TISHREI: u8 = 7;
pub const CHESHVAN: u8 = 8;
pub const KISLEV: u8 = 9;
pub const TEVET: u8 = 10;
pub const SHEVAT: u8 = 11;
pub const ADAR: u8 = 12;
pub const ADAR_II: u8 = 13;

/// R.D. of 1 Tishrei AM 1 (Julian 7 Oct 3761 BCE).
const EPOCH: i64 = -1_373_427;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HebrewDate {
    pub year: i32,
    /// 1 = Nisan … 7 = Tishrei … 12 = Adar (Adar I in leap years), 13 = Adar II.
    pub month: u8,
    pub day: u8,
}

pub fn is_leap(year: i32) -> bool {
    (7 * year as i64 + 1).rem_euclid(19) < 7
}

pub fn last_month(year: i32) -> u8 {
    if is_leap(year) {
        13
    } else {
        12
    }
}

fn elapsed_days(year: i32) -> i64 {
    let y = year as i64;
    let months = (235 * y - 234).div_euclid(19);
    let parts = 12084 + 13753 * months;
    let days = 29 * months + parts.div_euclid(25920);
    if (3 * (days + 1)).rem_euclid(7) < 3 {
        days + 1
    } else {
        days
    }
}

fn year_length_correction(year: i32) -> i64 {
    let ny0 = elapsed_days(year - 1);
    let ny1 = elapsed_days(year);
    let ny2 = elapsed_days(year + 1);
    if ny2 - ny1 == 356 {
        2
    } else if ny1 - ny0 == 382 {
        1
    } else {
        0
    }
}

/// R.D. of 1 Tishrei of `year`.
pub fn new_year(year: i32) -> i64 {
    EPOCH + elapsed_days(year) + year_length_correction(year)
}

/// 353, 354, 355, 383, 384 or 385.
pub fn days_in_year(year: i32) -> i64 {
    new_year(year + 1) - new_year(year)
}

pub fn long_cheshvan(year: i32) -> bool {
    matches!(days_in_year(year), 355 | 385)
}

pub fn short_kislev(year: i32) -> bool {
    matches!(days_in_year(year), 353 | 383)
}

pub fn days_in_month(year: i32, month: u8) -> u8 {
    let short = matches!(month, IYAR | TAMMUZ | ELUL | TEVET | ADAR_II)
        || (month == ADAR && !is_leap(year))
        || (month == CHESHVAN && !long_cheshvan(year))
        || (month == KISLEV && short_kislev(year));
    if short {
        29
    } else {
        30
    }
}

/// Months of `year` in calendar order (Tishrei first).
pub fn months_in_order(year: i32) -> Vec<u8> {
    let mut v: Vec<u8> = (TISHREI..=last_month(year)).collect();
    v.extend(NISAN..=ELUL);
    v
}

pub fn fixed_from_hebrew(d: HebrewDate) -> i64 {
    let HebrewDate { year, month, day } = d;
    let mut days = new_year(year) + day as i64 - 1;
    if month < TISHREI {
        for m in TISHREI..=last_month(year) {
            days += days_in_month(year, m) as i64;
        }
        for m in NISAN..month {
            days += days_in_month(year, m) as i64;
        }
    } else {
        for m in TISHREI..month {
            days += days_in_month(year, m) as i64;
        }
    }
    days
}

pub fn hebrew_from_fixed(date: i64) -> HebrewDate {
    // 35975351/98496 = average Hebrew year length in days.
    let approx = ((date - EPOCH) * 98496).div_euclid(35_975_351) + 1;
    let approx = approx as i32;
    let year = if new_year(approx) > date {
        approx - 1
    } else {
        approx
    };
    let start = if date
        < fixed_from_hebrew(HebrewDate {
            year,
            month: NISAN,
            day: 1,
        }) {
        TISHREI
    } else {
        NISAN
    };
    let mut month = start;
    while date
        > fixed_from_hebrew(HebrewDate {
            year,
            month,
            day: days_in_month(year, month),
        })
    {
        month += 1;
    }
    let first = fixed_from_hebrew(HebrewDate {
        year,
        month,
        day: 1,
    });
    HebrewDate {
        year,
        month,
        day: (date - first + 1) as u8,
    }
}

pub fn fixed_from_gregorian(d: NaiveDate) -> i64 {
    d.num_days_from_ce() as i64
}

pub fn gregorian_from_fixed(rd: i64) -> NaiveDate {
    NaiveDate::from_num_days_from_ce_opt(rd as i32).expect("date in range")
}

impl HebrewDate {
    pub fn new(year: i32, month: u8, day: u8) -> HebrewDate {
        HebrewDate { year, month, day }
    }

    pub fn from_gregorian(d: NaiveDate) -> HebrewDate {
        hebrew_from_fixed(fixed_from_gregorian(d))
    }

    pub fn to_gregorian(self) -> NaiveDate {
        gregorian_from_fixed(fixed_from_hebrew(self))
    }

    pub fn is_leap(self) -> bool {
        is_leap(self.year)
    }

    /// «12 кислева 5787».
    pub fn format_ru(self) -> String {
        format!(
            "{} {} {}",
            self.day,
            month_name_ru_gen(self.year, self.month),
            self.year
        )
    }

    /// «י״ב כסלו תשפ״ז».
    pub fn format_he(self) -> String {
        format!(
            "{} {} {}",
            gematria(self.day as u32),
            month_name_he(self.year, self.month),
            gematria((self.year % 1000) as u32)
        )
    }
}

/// Month name, nominative («Кислев», «Адар I»).
pub fn month_name_ru(year: i32, month: u8) -> &'static str {
    let leap = is_leap(year);
    match month {
        NISAN => "Нисан",
        IYAR => "Ияр",
        SIVAN => "Сиван",
        TAMMUZ => "Тамуз",
        AV => "Ав",
        ELUL => "Элул",
        TISHREI => "Тишрей",
        CHESHVAN => "Хешван",
        KISLEV => "Кислев",
        TEVET => "Тевет",
        SHEVAT => "Шват",
        ADAR if leap => "Адар I",
        ADAR => "Адар",
        _ => "Адар II",
    }
}

/// Month name, genitive, as used in dates («12 кислева»).
pub fn month_name_ru_gen(year: i32, month: u8) -> &'static str {
    let leap = is_leap(year);
    match month {
        NISAN => "нисана",
        IYAR => "ияра",
        SIVAN => "сивана",
        TAMMUZ => "тамуза",
        AV => "ава",
        ELUL => "элула",
        TISHREI => "тишрея",
        CHESHVAN => "хешвана",
        KISLEV => "кислева",
        TEVET => "тевета",
        SHEVAT => "швата",
        ADAR if leap => "адара I",
        ADAR => "адара",
        _ => "адара II",
    }
}

pub fn month_name_he(year: i32, month: u8) -> &'static str {
    let leap = is_leap(year);
    match month {
        NISAN => "ניסן",
        IYAR => "אייר",
        SIVAN => "סיון",
        TAMMUZ => "תמוז",
        AV => "אב",
        ELUL => "אלול",
        TISHREI => "תשרי",
        CHESHVAN => "חשון",
        KISLEV => "כסלו",
        TEVET => "טבת",
        SHEVAT => "שבט",
        ADAR if leap => "אדר א׳",
        ADAR => "אדר",
        _ => "אדר ב׳",
    }
}

/// Hebrew numeral with geresh/gershayim (1..=999): 15 → ט״ו, 787 → תשפ״ז.
pub fn gematria(mut n: u32) -> String {
    const HUNDREDS: [(u32, char); 4] = [(400, 'ת'), (300, 'ש'), (200, 'ר'), (100, 'ק')];
    const TENS: [char; 9] = ['י', 'כ', 'ל', 'מ', 'נ', 'ס', 'ע', 'פ', 'צ'];
    const ONES: [char; 9] = ['א', 'ב', 'ג', 'ד', 'ה', 'ו', 'ז', 'ח', 'ט'];
    let mut letters: Vec<char> = Vec::new();
    for (v, c) in HUNDREDS {
        while n >= v {
            letters.push(c);
            n -= v;
        }
    }
    if n == 15 || n == 16 {
        letters.push('ט');
        letters.push(if n == 15 { 'ו' } else { 'ז' });
        n = 0;
    }
    if n >= 10 {
        letters.push(TENS[(n / 10 - 1) as usize]);
        n %= 10;
    }
    if n > 0 {
        letters.push(ONES[(n - 1) as usize]);
    }
    let mut s = String::new();
    match letters.len() {
        0 => {}
        1 => {
            s.push(letters[0]);
            s.push('׳');
        }
        len => {
            for (i, c) in letters.iter().enumerate() {
                if i == len - 1 {
                    s.push('״');
                }
                s.push(*c);
            }
        }
    }
    s
}

/// Weekday names in Russian, Monday first (chrono `num_days_from_monday`).
pub fn weekday_ru(d: NaiveDate) -> &'static str {
    [
        "понедельник",
        "вторник",
        "среда",
        "четверг",
        "пятница",
        "суббота",
        "воскресенье",
    ][d.weekday().num_days_from_monday() as usize]
}

/// «22 ноября 2026».
pub fn format_gregorian_ru(d: NaiveDate) -> String {
    const M: [&str; 12] = [
        "января", "февраля", "марта", "апреля", "мая", "июня", "июля", "августа", "сентября",
        "октября", "ноября", "декабря",
    ];
    format!("{} {} {}", d.day(), M[d.month0() as usize], d.year())
}
