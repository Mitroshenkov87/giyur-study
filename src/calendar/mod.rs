//! «Еврейский календарь»: offline Hebrew calendar, holidays and fasts,
//! the weekly Torah portion, daily ideas and zmanim.
//!
//! * [`hebrew`]  — Hebrew ↔ Gregorian conversion (the standard arithmetic
//!   calendar algorithm, Dershowitz & Reingold, *Calendrical Calculations*).
//! * [`days`]    — what a given day is: holidays, fasts, Rosh Chodesh, Omer,
//!   the Three Weeks, special Shabbatot.
//! * [`parasha`] — the weekly Torah portion (Israel / diaspora).
//! * [`zmanim`]  — sunrise/sunset based times (candle lighting, havdalah,
//!   fast start/end), computed offline with the NOAA solar algorithm.
//! * [`geo`]     — optional, consent-based approximate location and the
//!   bundled offline city list.
//! * [`prefs`]   — calendar settings, stored locally inside the progress
//!   file (`settings.calendar`), never sent anywhere.
//! * [`texts`]   — the Russian content (content/calendar/*.json).

pub mod days;
pub mod geo;
pub mod hebrew;
pub mod parasha;
pub mod prefs;
pub mod texts;
pub mod zmanim;
