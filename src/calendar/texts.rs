//! Russian calendar content, embedded at build time from content/calendar/.
//! Every text here is listed in docs/RABBI-REVIEW.md for review.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct ParashaText {
    pub id: String,
    /// «Берешит», «Шмот», …
    pub book: String,
    /// «Берешит 1:1–6:8».
    #[serde(rename = "ref")]
    pub reference: String,
    pub summary: String,
    pub theme: String,
    /// Seven short ideas: Sunday … Shabbat.
    pub daily: Vec<String>,
    /// Lesson ids in the app to read next.
    pub lessons: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Movements {
    pub reform: Option<String>,
    pub conservative: Option<String>,
    pub orthodox: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct DayText {
    pub id: String,
    pub name_he: String,
    /// What the day means in the tradition (2–4 sentences).
    pub meaning: String,
    pub customs: Vec<String>,
    /// The day's Torah idea.
    pub idea: String,
    /// For fasts: what is forbidden, who is exempt, when it starts and ends.
    pub fast_note: Option<String>,
    /// Where the movements' customs differ.
    pub movements: Option<Movements>,
    pub lessons: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct MonthText {
    /// «tishrei», «cheshvan», …, «adar», «adar-1», «adar-2».
    pub key: String,
    pub idea: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct ParashotFile {
    parashot: Vec<ParashaText>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct DaysFile {
    days: Vec<DayText>,
    months: Vec<MonthText>,
}

pub struct Texts {
    pub parashot: HashMap<String, ParashaText>,
    pub days: HashMap<String, DayText>,
    pub months: HashMap<String, MonthText>,
}

impl Texts {
    fn parse(parashot: &str, days: &str) -> Texts {
        let p: ParashotFile = serde_json::from_str(parashot).unwrap_or_default();
        let d: DaysFile = serde_json::from_str(days).unwrap_or_default();
        Texts {
            parashot: p.parashot.into_iter().map(|x| (x.id.clone(), x)).collect(),
            days: d.days.into_iter().map(|x| (x.id.clone(), x)).collect(),
            months: d.months.into_iter().map(|x| (x.key.clone(), x)).collect(),
        }
    }

    pub fn get() -> &'static Texts {
        static T: OnceLock<Texts> = OnceLock::new();
        T.get_or_init(|| {
            Texts::parse(
                include_str!("../../content/calendar/parashot.json"),
                include_str!("../../content/calendar/days.json"),
            )
        })
    }

    /// Text for portion index `n` (0 = Bereshit).
    pub fn parasha(&self, n: u8) -> Option<&ParashaText> {
        let id = crate::calendar::parasha::PARASHOT.get(n as usize)?.0;
        self.parashot.get(id)
    }

    pub fn day(&self, id: &str) -> Option<&DayText> {
        self.days.get(id)
    }

    pub fn month(&self, year: i32, month: u8) -> Option<&MonthText> {
        self.months.get(month_key(year, month))
    }
}

pub fn month_key(year: i32, month: u8) -> &'static str {
    use crate::calendar::hebrew::*;
    match month {
        NISAN => "nisan",
        IYAR => "iyar",
        SIVAN => "sivan",
        TAMMUZ => "tammuz",
        AV => "av",
        ELUL => "elul",
        TISHREI => "tishrei",
        CHESHVAN => "cheshvan",
        KISLEV => "kislev",
        TEVET => "tevet",
        SHEVAT => "shevat",
        ADAR if is_leap(year) => "adar-1",
        ADAR => "adar",
        _ => "adar-2",
    }
}
