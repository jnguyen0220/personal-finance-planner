use chrono::NaiveDate;

/// Today in UTC, the reference point for all date-relative domain rules.
pub fn today() -> NaiveDate {
    chrono::Utc::now().date_naive()
}

/// Parses a "YYYY-MM-DD" date; returns None for empty or malformed values.
pub fn parse(d: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(d, "%Y-%m-%d").ok()
}

/// Whole days from today until `date` (negative once it is in the past).
pub fn days_until(date: &str) -> i64 {
    match parse(date) {
        Some(d) => (d - today()).num_days(),
        None => 0,
    }
}

/// The current calendar year (UTC).
pub fn current_year() -> i32 {
    use chrono::Datelike;
    today().year()
}

/// Absolute month index (year*12 + month-1) for a "YYYY-MM-DD" date; a missing
/// month defaults to January.
pub fn month_index(d: &str) -> Option<i32> {
    let year: i32 = d.get(0..4)?.parse().ok()?;
    let month: i32 = d.get(5..7).and_then(|m| m.parse().ok()).unwrap_or(1);
    Some(year * 12 + (month - 1))
}

/// Absolute month index for the current month (UTC).
pub fn current_month_index() -> i32 {
    use chrono::Datelike;
    let now = today();
    now.year() * 12 + (now.month() as i32 - 1)
}
