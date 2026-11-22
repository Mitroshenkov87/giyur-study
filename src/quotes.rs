//! «Цитата дня»: short sayings from Jewish sources (Pirkei Avot, Tanakh,
//! Talmud, sages), embedded from `content/quotes.json`, one per calendar day.
//! Every quote is listed in docs/RABBI-REVIEW.md for review.

use std::sync::OnceLock;

use chrono::NaiveDate;
use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Quote {
    /// Russian text (a meaning-level translation).
    pub text: String,
    /// «Пиркей Авот 1:14», «Дварим 6:4», «Шаббат 31а», …
    pub source: String,
    /// Who said it, when the source names a person.
    pub author: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct File {
    quotes: Vec<Quote>,
}

/// All quotes, in file order.
pub fn all() -> &'static [Quote] {
    static ALL: OnceLock<Vec<Quote>> = OnceLock::new();
    ALL.get_or_init(|| {
        serde_json::from_str::<File>(include_str!("../content/quotes.json"))
            .map(|f| f.quotes)
            .unwrap_or_default()
    })
}

/// The quote for a date: rotates by day, so each day shows the next quote
/// and the whole list cycles (a stride coprime with the length spreads
/// neighbouring days across different sources).
pub fn for_date(date: NaiveDate) -> Option<&'static Quote> {
    let list = all();
    if list.is_empty() {
        return None;
    }
    let n = list.len() as i64;
    let day = date
        .signed_duration_since(NaiveDate::from_ymd_opt(2000, 1, 1)?)
        .num_days();
    let stride = (1..n)
        .rev()
        .find(|s| gcd(*s, n) == 1 && *s <= 37)
        .unwrap_or(1);
    Some(&list[(day * stride).rem_euclid(n) as usize])
}

fn gcd(a: i64, b: i64) -> i64 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

/// «— рабби Акива, Пиркей Авот 3:14» style attribution line.
pub fn attribution(q: &Quote) -> String {
    match q.author.as_deref().filter(|a| !a.is_empty()) {
        Some(a) => format!("{a} · {}", q.source),
        None => q.source.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BANNED: [&str; 6] = [
        "библи",
        "ветхий завет",
        "ветхого завета",
        "старый завет",
        "старого завета",
        "псалт",
    ];

    #[test]
    fn at_least_sixty_quotes_all_filled() {
        let list = all();
        assert!(list.len() >= 60, "only {} quotes", list.len());
        for q in list {
            assert!(!q.text.trim().is_empty());
            assert!(!q.source.trim().is_empty(), "no source: {}", q.text);
            assert!(q.text.chars().count() <= 220, "too long: {}", q.text);
        }
    }

    #[test]
    fn jewish_framing_only() {
        for q in all() {
            let all = format!(
                "{} {} {}",
                q.text,
                q.source,
                q.author.as_deref().unwrap_or_default()
            )
            .to_lowercase();
            for b in BANNED {
                assert!(!all.contains(b), "«{b}» in {all}");
            }
            assert!(
                !all.chars().any(|c| c.is_ascii_alphabetic()),
                "Latin letters: {all}"
            );
        }
    }

    #[test]
    fn rotates_daily_and_covers_the_list() {
        let d0 = NaiveDate::from_ymd_opt(2026, 11, 22).unwrap();
        let a = for_date(d0).unwrap();
        let b = for_date(d0.succ_opt().unwrap()).unwrap();
        assert_ne!(a, b);
        assert_eq!(a, for_date(d0).unwrap(), "stable for a day");
        let n = all().len();
        let mut seen = std::collections::HashSet::new();
        for i in 0..n as i64 {
            let q = for_date(d0 + chrono::Duration::days(i)).unwrap();
            seen.insert(q.text.clone());
        }
        assert_eq!(seen.len(), n, "a full cycle shows every quote once");
    }
}
