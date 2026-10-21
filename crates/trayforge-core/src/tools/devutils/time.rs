//! UTC calendar arithmetic, so the core needs no time zone database.

use std::time::{SystemTime, UNIX_EPOCH};

pub const WEEKDAYS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

pub fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

/// Days since 1970-01-01 to (year, month, day) (Howard Hinnant's `civil_from_days`).
pub fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
pub fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let y = year - i64::from(month <= 2);
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = i64::from((month + 9) % 12);
    let doy = (153 * mp + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// 0 = Sunday.
pub fn weekday(days: i64) -> usize {
    (days + 4).rem_euclid(7) as usize
}

pub fn format_date(timestamp: i64) -> String {
    let (y, m, d) = civil_from_days(timestamp.div_euclid(86_400));
    format!("{y:04}-{m:02}-{d:02}")
}

pub fn format_datetime(timestamp: i64) -> String {
    let secs = timestamp.rem_euclid(86_400);
    format!(
        "{} {:02}:{:02}:{:02}",
        format_date(timestamp),
        secs / 3600,
        secs % 3600 / 60,
        secs % 60
    )
}
