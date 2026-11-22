//! Optional approximate location (with consent) and the offline city list.
//! STUB — the API below is fixed; the implementation is being written.

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
    Err(GeoError::Unsupported)
}

/// All bundled cities.
pub fn cities() -> &'static [City] {
    &[]
}

/// Case-insensitive substring search over Russian names (and countries).
pub fn search(query: &str) -> Vec<&'static City> {
    let _ = query;
    Vec::new()
}
