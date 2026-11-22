//! Shared helpers ported from js/util.js.
//! Day keys are local dates formatted "YYYY-MM-DD" exactly like the PWA.

use chrono::NaiveDate;
use rand::Rng;

/// Debug / screenshot hook `SHORASHIM_<name>`; the pre-rename
/// `GIYUR_<name>` spelling is still read so old scripts keep working.
pub fn env_hook(name: &str) -> Option<String> {
    std::env::var(format!("SHORASHIM_{name}"))
        .or_else(|_| std::env::var(format!("GIYUR_{name}")))
        .ok()
}

/// Format a date as "YYYY-MM-DD" (JS `U.dayKey`).
pub fn day_key(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

/// Today's day key in local time (JS `U.today`).
pub fn today() -> String {
    day_key(chrono::Local::now().date_naive())
}

/// Parse a "YYYY-MM-DD" day key (JS `U.parseDay`).
pub fn parse_day(key: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(key, "%Y-%m-%d").ok()
}

/// Add `n` days to a day key, returning a day key (JS `U.addDays`).
pub fn add_days(key: &str, n: i64) -> String {
    match parse_day(key) {
        Some(d) => day_key(d + chrono::Duration::days(n)),
        None => key.to_string(),
    }
}

/// Whole days from `a` to `b` (b - a) (JS `U.diffDays`).
pub fn diff_days(a: &str, b: &str) -> i64 {
    match (parse_day(a), parse_day(b)) {
        (Some(a), Some(b)) => (b - a).num_days(),
        _ => 0,
    }
}

/// Format a date as "DD.MM.YYYY" (JS `U.fmtDate`).
pub fn fmt_date(d: NaiveDate) -> String {
    d.format("%d.%m.%Y").to_string()
}

/// Fisher–Yates shuffle returning a new vector (JS `U.shuffle`).
pub fn shuffle<T: Clone>(items: &[T], rng: &mut impl Rng) -> Vec<T> {
    let mut a = items.to_vec();
    for i in (1..a.len()).rev() {
        let j = rng.gen_range(0..=i);
        a.swap(i, j);
    }
    a
}

/// Weighted sampling without replacement, Efraimidis–Spirakis (JS `U.weightedSample`).
pub fn weighted_sample<T: Clone>(
    items: &[T],
    weight_fn: impl Fn(&T) -> f64,
    n: usize,
    rng: &mut impl Rng,
) -> Vec<T> {
    let mut keyed: Vec<(T, f64)> = items
        .iter()
        .map(|it| {
            let w = weight_fn(it).max(0.01);
            let k = rng.gen::<f64>().powf(1.0 / w);
            (it.clone(), k)
        })
        .collect();
    keyed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    keyed.truncate(n);
    keyed.into_iter().map(|x| x.0).collect()
}

/// Normalise typed answers: lower-case, ё→е, no nikkud, no punctuation,
/// single spaces (JS `U.normalize`).
pub fn normalize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut pending_space = false;
    for ch in s.chars().flat_map(|c| c.to_lowercase()) {
        let ch = if ch == 'ё' { 'е' } else { ch };
        // Hebrew nikkud/cantillation U+0591..U+05C7
        if ('\u{0591}'..='\u{05C7}').contains(&ch) {
            continue;
        }
        if ch.is_alphanumeric() {
            if pending_space && !out.is_empty() {
                out.push(' ');
            }
            pending_space = false;
            out.push(ch);
        } else {
            pending_space = true;
        }
    }
    out
}

/// Russian plural: `plural(5, &["карточка", "карточки", "карточек"])` (JS `U.plural`).
pub fn plural<'a>(n: i64, forms: &[&'a str; 3]) -> &'a str {
    let a = n.unsigned_abs() % 100;
    let b = a % 10;
    if a > 10 && a < 20 {
        return forms[2];
    }
    if b > 1 && b < 5 {
        return forms[1];
    }
    if b == 1 {
        return forms[0];
    }
    forms[2]
}

/// Round a fraction to a whole percentage (JS `U.pct`).
pub fn pct(x: f64) -> i64 {
    (x * 100.0).round() as i64
}

pub fn clamp<T: PartialOrd>(x: T, a: T, b: T) -> T {
    if x < a {
        a
    } else if x > b {
        b
    } else {
        x
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    fn rng() -> impl Rng {
        rand::rngs::StdRng::seed_from_u64(42)
    }

    #[test]
    fn day_key_roundtrip() {
        let d = NaiveDate::from_ymd_opt(2026, 11, 22).unwrap();
        assert_eq!(day_key(d), "2026-11-22");
        assert_eq!(parse_day("2026-11-22"), Some(d));
    }

    #[test]
    fn add_days_rollover() {
        assert_eq!(add_days("2026-01-31", 1), "2026-02-01");
        assert_eq!(add_days("2026-11-22", -21), "2026-11-01");
        assert_eq!(add_days("2026-12-31", 1), "2027-01-01");
    }

    #[test]
    fn diff_days_works() {
        assert_eq!(diff_days("2026-11-20", "2026-11-22"), 2);
        assert_eq!(diff_days("2026-11-22", "2026-11-20"), -2);
    }

    #[test]
    fn fmt_date_ddmmyyyy() {
        let d = NaiveDate::from_ymd_opt(2026, 1, 5).unwrap();
        assert_eq!(fmt_date(d), "05.01.2026");
    }

    #[test]
    fn shuffle_keeps_elements() {
        let v: Vec<i32> = (0..20).collect();
        let s = shuffle(&v, &mut rng());
        assert_eq!(s.len(), v.len());
        let mut sorted = s.clone();
        sorted.sort();
        assert_eq!(sorted, v);
    }

    #[test]
    fn weighted_sample_size_and_uniqueness() {
        let v: Vec<i32> = (0..10).collect();
        let s = weighted_sample(&v, |_| 1.0, 5, &mut rng());
        assert_eq!(s.len(), 5);
        let mut sorted = s.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 5);
    }

    #[test]
    fn normalize_typed_answers() {
        assert_eq!(normalize("  Шалом,  Мир! "), "шалом мир");
        assert_eq!(normalize("Ёлка-ёЛКА"), "елка елка");
        assert_eq!(normalize("שָׁלוֹם"), "שלום"); // nikkud stripped
        assert_eq!(normalize("Берейшит (Быт. 1:1)"), "берейшит быт 1 1");
    }

    #[test]
    fn plural_forms() {
        let f = ["карточка", "карточки", "карточек"];
        assert_eq!(plural(1, &f), "карточка");
        assert_eq!(plural(2, &f), "карточки");
        assert_eq!(plural(5, &f), "карточек");
        assert_eq!(plural(11, &f), "карточек");
        assert_eq!(plural(21, &f), "карточка");
        assert_eq!(plural(111, &f), "карточек");
    }

    #[test]
    fn pct_and_clamp() {
        assert_eq!(pct(0.756), 76);
        assert_eq!(clamp(15, 0, 10), 10);
        assert_eq!(clamp(-3, 0, 10), 0);
        assert_eq!(clamp(5, 0, 10), 5);
    }
}
