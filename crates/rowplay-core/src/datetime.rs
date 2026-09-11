// SPDX-License-Identifier: GPL-3.0-or-later
//! Calendar and date-time helpers for Concept2 logbook timestamps.
//!
//! Port of the web app's `src/lib/datetime.ts` (canonical) with the input
//! bounds from rowplay-studio's `RowPlayDateTime.swift`. Logbook timestamps are
//! wall-clock strings (`YYYY-MM-DD HH:MM:SS`, no offset) interpreted as UTC for
//! sorting and day keys, exactly like the web app. IANA time-zone projection
//! uses the `chrono-tz` database compiled into the binary, so there is no
//! runtime dependency on the host's zoneinfo files.
//!
//! Locale-dependent display formatting (`fmtDate`, `fmtLogbookDateTime`, …)
//! is deliberately not ported: the QML shell formats dates with `QLocale`.

use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{Datelike, NaiveDate, NaiveDateTime, TimeZone, Timelike};
use chrono_tz::Tz;

use crate::num::js_round;

/// Milliseconds per second.
pub const MS_PER_SECOND: f64 = 1000.0;
/// Milliseconds per day.
pub const MS_PER_DAY: i64 = 86_400_000;

const MAX_RAW_LEN: usize = 64;
const MAX_TRIMMED_LOGBOOK_LEN: usize = 30;
const MAX_TRIMMED_DAY_KEY_LEN: usize = 20;
const MAX_TRIMMED_INSTANT_LEN: usize = 40;

/// Wall-clock parts of a logbook timestamp.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LogbookDateTime {
    /// Four-digit year.
    pub year: i32,
    /// Month 1–12.
    pub month: u32,
    /// Day of month 1–31.
    pub day: u32,
    /// Hour 0–23.
    pub hour: u32,
    /// Minute 0–59.
    pub minute: u32,
    /// Second 0–59.
    pub second: u32,
}

/// Calendar date parts of a `YYYY-MM-DD` key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct DayParts {
    year: i32,
    month: u32,
    day: u32,
}

impl DayParts {
    fn at_midnight(self) -> LogbookDateTime {
        LogbookDateTime {
            year: self.year,
            month: self.month,
            day: self.day,
            hour: 0,
            minute: 0,
            second: 0,
        }
    }
}

// --- civil calendar arithmetic (proleptic Gregorian, Howard Hinnant) --------

fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let m = i64::from(month);
    let d = i64::from(day);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Epoch milliseconds of wall-clock parts read as UTC (web `utcEpochMillis`).
#[must_use]
pub fn utc_epoch_millis(parts: LogbookDateTime) -> i64 {
    let days = days_from_civil(i64::from(parts.year), parts.month, parts.day);
    let seconds = days * 86_400
        + i64::from(parts.hour) * 3600
        + i64::from(parts.minute) * 60
        + i64::from(parts.second);
    seconds * 1000
}

fn utc_parts_from_epoch_millis(ms: i64) -> LogbookDateTime {
    let days = ms.div_euclid(MS_PER_DAY);
    let rem = ms.rem_euclid(MS_PER_DAY) / 1000;
    let (year, month, day) = civil_from_days(days);
    LogbookDateTime {
        year: year as i32,
        month,
        day,
        hour: (rem / 3600) as u32,
        minute: ((rem % 3600) / 60) as u32,
        second: (rem % 60) as u32,
    }
}

fn valid_parts(parts: LogbookDateTime) -> bool {
    if !(1..=12).contains(&parts.month)
        || !(1..=31).contains(&parts.day)
        || parts.hour > 23
        || parts.minute > 59
        || parts.second > 59
    {
        return false;
    }
    utc_parts_from_epoch_millis(utc_epoch_millis(parts)) == parts
}

// --- strict parsers ---------------------------------------------------------

fn take_digits(s: &str, n: usize) -> Option<(u32, &str)> {
    let (head, rest) = s.split_at_checked(n)?;
    if head.len() != n || !head.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some((head.parse().ok()?, rest))
}

fn expect_byte<'a>(s: &'a str, allowed: &[u8]) -> Option<&'a str> {
    let first = *s.as_bytes().first()?;
    if allowed.contains(&first) {
        Some(&s[1..])
    } else {
        None
    }
}

/// `YYYY-MM-DD` with the same bounds as Studio: raw ≤ 64, trimmed ≤ 20.
fn parse_day_key(key: &str) -> Option<DayParts> {
    if key.len() > MAX_RAW_LEN {
        return None;
    }
    let value = key.trim();
    if value.len() > MAX_TRIMMED_DAY_KEY_LEN {
        return None;
    }
    let (year, rest) = take_digits(value, 4)?;
    let rest = expect_byte(rest, b"-")?;
    let (month, rest) = take_digits(rest, 2)?;
    let rest = expect_byte(rest, b"-")?;
    let (day, rest) = take_digits(rest, 2)?;
    if !rest.is_empty() {
        return None;
    }
    let parts = DayParts {
        year: year as i32,
        month,
        day,
    };
    if valid_parts(parts.at_midnight()) {
        Some(parts)
    } else {
        None
    }
}

/// Concept2 logbook timestamps: `YYYY-MM-DD HH:MM:SS` (a `T` separator is also
/// accepted, no offset). Returns `None` for malformed, impossible or over-long input.
#[must_use]
pub fn parse_logbook_date_time(text: &str) -> Option<LogbookDateTime> {
    if text.len() > MAX_RAW_LEN {
        return None;
    }
    let value = text.trim();
    if value.len() > MAX_TRIMMED_LOGBOOK_LEN {
        return None;
    }
    let (year, rest) = take_digits(value, 4)?;
    let rest = expect_byte(rest, b"-")?;
    let (month, rest) = take_digits(rest, 2)?;
    let rest = expect_byte(rest, b"-")?;
    let (day, rest) = take_digits(rest, 2)?;
    let rest = expect_byte(rest, b" T")?;
    let (hour, rest) = take_digits(rest, 2)?;
    let rest = expect_byte(rest, b":")?;
    let (minute, rest) = take_digits(rest, 2)?;
    let rest = expect_byte(rest, b":")?;
    let (second, rest) = take_digits(rest, 2)?;
    if !rest.is_empty() {
        return None;
    }
    let parts = LogbookDateTime {
        year: year as i32,
        month,
        day,
        hour,
        minute,
        second,
    };
    if valid_parts(parts) {
        Some(parts)
    } else {
        None
    }
}

/// Epoch milliseconds for sorting; logbook wall times interpreted as UTC.
/// `NaN` when the string cannot be parsed (web `logbookEpochMillis`).
#[must_use]
pub fn logbook_epoch_millis(text: &str) -> f64 {
    parse_logbook_date_time(text).map_or(f64::NAN, |parts| utc_epoch_millis(parts) as f64)
}

/// ISO-8601 instant or RFC 3339 string → epoch milliseconds, `NaN` if invalid
/// (web `parseInstantMillis`).
///
/// Accepts `YYYY-MM-DDTHH:MM:SS[.fff](Z|±HH:MM|±HHMM)`; offset-less strings are
/// rejected so logbook parsing stays timezone-stable. Fractional seconds beyond
/// milliseconds are truncated like `Date.parse`.
#[must_use]
pub fn parse_instant_millis(text: &str) -> f64 {
    parse_instant(text).map_or(f64::NAN, |ms| ms as f64)
}

/// Parse an ISO instant / RFC 3339 timestamp to epoch milliseconds (web `parseInstant`).
#[must_use]
pub fn parse_instant(text: &str) -> Option<i64> {
    if text.len() > MAX_RAW_LEN {
        return None;
    }
    let value = text.trim();
    if value.len() > MAX_TRIMMED_INSTANT_LEN {
        return None;
    }
    let (year, rest) = take_digits(value, 4)?;
    let rest = expect_byte(rest, b"-")?;
    let (month, rest) = take_digits(rest, 2)?;
    let rest = expect_byte(rest, b"-")?;
    let (day, rest) = take_digits(rest, 2)?;
    let rest = expect_byte(rest, b"Tt")?;
    let (hour, rest) = take_digits(rest, 2)?;
    let rest = expect_byte(rest, b":")?;
    let (minute, rest) = take_digits(rest, 2)?;
    let rest = expect_byte(rest, b":")?;
    let (second, mut rest) = take_digits(rest, 2)?;

    let mut fraction_ms: i64 = 0;
    if let Some(after_dot) = rest.strip_prefix('.') {
        let digits_len = after_dot.bytes().take_while(u8::is_ascii_digit).count();
        if digits_len == 0 {
            return None;
        }
        let digits = &after_dot[..digits_len];
        let mut ms_digits = digits.chars().take(3).collect::<String>();
        while ms_digits.len() < 3 {
            ms_digits.push('0');
        }
        fraction_ms = ms_digits.parse().ok()?;
        rest = &after_dot[digits_len..];
    }

    let offset_ms: i64 = match rest.as_bytes() {
        [b'Z' | b'z'] => 0,
        [sign @ (b'+' | b'-'), ..] => {
            let body = &rest[1..];
            let (hh, body) = take_digits(body, 2)?;
            let body = body.strip_prefix(':').unwrap_or(body);
            let (mm, body) = take_digits(body, 2)?;
            if !body.is_empty() || hh > 23 || mm > 59 {
                return None;
            }
            let magnitude = (i64::from(hh) * 60 + i64::from(mm)) * 60_000;
            if *sign == b'+' { magnitude } else { -magnitude }
        }
        _ => return None,
    };

    let parts = LogbookDateTime {
        year: year as i32,
        month,
        day,
        hour,
        minute,
        second,
    };
    if !valid_parts(parts) {
        return None;
    }
    Some(utc_epoch_millis(parts) + fraction_ms - offset_ms)
}

// --- clock --------------------------------------------------------------------

/// Current epoch milliseconds (web `nowEpochMillis`).
#[must_use]
pub fn now_epoch_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

/// Current instant as an ISO-8601 string (web `nowIsoString`).
#[must_use]
pub fn now_iso_string() -> String {
    instant_iso_from_epoch_millis(now_epoch_millis())
}

/// Today as a `YYYY-MM-DD` key in UTC (web `todayKeyUtc`).
#[must_use]
pub fn today_key_utc() -> String {
    day_key_from_epoch_millis(now_epoch_millis())
}

/// Today as `YYYY-MM-DD` in the given IANA zone, or UTC when absent/invalid (web `todayKeyForTz`).
#[must_use]
pub fn today_key_for_tz(tz: Option<&str>) -> String {
    let Some(zone) = tz.map(str::trim).filter(|zone| !zone.is_empty()) else {
        return today_key_utc();
    };
    day_key_for_epoch_in_zone(now_epoch_millis(), zone).unwrap_or_else(today_key_utc)
}

/// Current UTC calendar year (web `currentUtcYear`).
#[must_use]
pub fn current_utc_year() -> i32 {
    utc_parts_from_epoch_millis(now_epoch_millis()).year
}

// --- day keys ------------------------------------------------------------------

/// `YYYY-MM-DD` of an epoch-millisecond instant in UTC (web `isoDayFromEpoch`,
/// Studio `dayKeyFromDate`).
#[must_use]
pub fn day_key_from_epoch_millis(ms: i64) -> String {
    let parts = utc_parts_from_epoch_millis(ms);
    format!("{:04}-{:02}-{:02}", parts.year, parts.month, parts.day)
}

/// `YYYY-MM-DD` day key → UTC-midnight epoch milliseconds; `NaN` if unparseable (web `dayKeyEpochMillis`).
#[must_use]
pub fn day_key_epoch_millis(key: &str) -> f64 {
    parse_day_key(key).map_or(f64::NAN, |parts| {
        utc_epoch_millis(parts.at_midnight()) as f64
    })
}

fn add_days_to_parts(parts: DayParts, days: i64) -> String {
    day_key_from_epoch_millis(utc_epoch_millis(parts.at_midnight()) + days * MS_PER_DAY)
}

/// `YYYY-MM-DD` one calendar day before a logbook date-time (or day key) string;
/// `None` when the input cannot be parsed (web `overlapDate`).
#[must_use]
pub fn overlap_date(date: &str) -> Option<String> {
    if let Some(parts) = parse_logbook_date_time(date) {
        return Some(add_days_to_parts(
            DayParts {
                year: parts.year,
                month: parts.month,
                day: parts.day,
            },
            -1,
        ));
    }
    parse_day_key(date).map(|day| add_days_to_parts(day, -1))
}

/// Add calendar days to a `YYYY-MM-DD` key; the key is returned unchanged when it
/// cannot be parsed (web `addDaysToKey`, Studio `dayKeyAddingDays`).
#[must_use]
pub fn add_days_to_key(key: &str, days: i64) -> String {
    parse_day_key(key).map_or_else(|| key.to_owned(), |parts| add_days_to_parts(parts, days))
}

/// Day of week of a `YYYY-MM-DD` key: 0 = Sunday … 6 = Saturday; 0 when invalid (web `dayOfWeekUtc`).
#[must_use]
pub fn day_of_week_utc(key: &str) -> u32 {
    parse_day_key(key).map_or(0, |parts| {
        // 1970-01-01 was a Thursday.
        ((days_from_civil(i64::from(parts.year), parts.month, parts.day) + 4).rem_euclid(7)) as u32
    })
}

/// One-based day of year of a `YYYY-MM-DD` key; 0 when invalid (web `dayOfYearUtc`).
#[must_use]
pub fn day_of_year_utc(key: &str) -> u32 {
    parse_day_key(key).map_or(0, |parts| {
        let current = days_from_civil(i64::from(parts.year), parts.month, parts.day);
        let start = days_from_civil(i64::from(parts.year), 1, 1);
        (current - start + 1) as u32
    })
}

/// Non-negative calendar days between two `YYYY-MM-DD` keys; 0 when either is
/// invalid or `from` is after `to` (web `daysBetweenUtc`).
#[must_use]
pub fn days_between_utc(from: &str, to: &str) -> i64 {
    match (parse_day_key(from), parse_day_key(to)) {
        (Some(from), Some(to)) => {
            let from_ms = utc_epoch_millis(from.at_midnight());
            let to_ms = utc_epoch_millis(to.at_midnight());
            ((to_ms - from_ms) / MS_PER_DAY).max(0)
        }
        _ => 0,
    }
}

/// Add calendar months to a `YYYY-MM-DD` key, clamping to the last day of the
/// target month; unchanged when invalid (web `addMonthsToKey`).
#[must_use]
pub fn add_months_to_key(key: &str, months: i64) -> String {
    let Some(parts) = parse_day_key(key) else {
        return key.to_owned();
    };
    let month_index = i64::from(parts.year) * 12 + i64::from(parts.month - 1) + months;
    let year = month_index.div_euclid(12);
    let month = (month_index.rem_euclid(12) + 1) as u32;
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    let last_day_ms = utc_epoch_millis(LogbookDateTime {
        year: next_year as i32,
        month: next_month,
        day: 1,
        hour: 0,
        minute: 0,
        second: 0,
    }) - MS_PER_DAY;
    let last_day = utc_parts_from_epoch_millis(last_day_ms).day;
    let day = parts.day.min(last_day);
    day_key_from_epoch_millis(utc_epoch_millis(LogbookDateTime {
        year: year as i32,
        month,
        day,
        hour: 0,
        minute: 0,
        second: 0,
    }))
}

// --- time zones ------------------------------------------------------------------

fn parse_zone(name: &str) -> Option<Tz> {
    name.parse::<Tz>().ok()
}

fn naive_utc(ms: i64) -> Option<NaiveDateTime> {
    let secs = ms.div_euclid(1000);
    let nanos = (ms.rem_euclid(1000) * 1_000_000) as u32;
    chrono::DateTime::from_timestamp(secs, nanos).map(|dt| dt.naive_utc())
}

fn local_parts(ms: i64, zone: Tz) -> Option<LogbookDateTime> {
    let local = zone.from_utc_datetime(&naive_utc(ms)?);
    Some(LogbookDateTime {
        year: local.year(),
        month: local.month(),
        day: local.day(),
        hour: local.hour(),
        minute: local.minute(),
        second: local.second(),
    })
}

fn offset_millis_for_zone(ms: i64, zone: Tz) -> Option<i64> {
    Some(utc_epoch_millis(local_parts(ms, zone)?) - ms)
}

/// The web app's iterative wall-time → instant resolution (`zonedWallTimeToEpochMillis`).
fn zoned_wall_time_to_epoch_millis(parts: LogbookDateTime, zone: Tz) -> Option<i64> {
    let wall = utc_epoch_millis(parts);
    let mut epoch = wall;
    for _ in 0..3 {
        let next = wall - offset_millis_for_zone(epoch, zone)?;
        if next == epoch {
            return Some(epoch);
        }
        epoch = next;
    }
    Some(epoch)
}

fn day_key_for_epoch_in_zone(ms: i64, zone_name: &str) -> Option<String> {
    let parts = local_parts(ms, parse_zone(zone_name)?)?;
    Some(format!(
        "{:04}-{:02}-{:02}",
        parts.year, parts.month, parts.day
    ))
}

/// Calendar day key for a workout using the resolution chain workout tz → home
/// tz → plain-date fallback (web `workoutLocalDayKey`).
///
/// The Concept2 `date` is monitor-local, so when `workout_tz` is known its
/// plain-date portion is the workout's local day. Cross-zone conversion is only
/// applied when `home_tz` differs from `workout_tz`. Invalid IANA names fall
/// through silently.
#[must_use]
pub fn workout_local_day_key(
    date: &str,
    workout_tz: Option<&str>,
    home_tz: Option<&str>,
) -> String {
    let plain = || date.chars().take(10).collect::<String>();
    let clean_wtz = workout_tz.map(str::trim).filter(|tz| !tz.is_empty());
    let clean_htz = home_tz.map(str::trim).filter(|tz| !tz.is_empty());
    if clean_wtz.is_none() && clean_htz.is_none() {
        return plain();
    }
    let Some(parts) = parse_logbook_date_time(date) else {
        return plain();
    };
    if let Some(wtz) = clean_wtz {
        let Some(htz) = clean_htz else {
            return plain();
        };
        if htz == wtz {
            return plain();
        }
        if let Some(day) = parse_zone(wtz)
            .and_then(|zone| zoned_wall_time_to_epoch_millis(parts, zone))
            .and_then(|epoch| day_key_for_epoch_in_zone(epoch, htz))
        {
            return day;
        }
    }
    plain()
}

// --- ISO output -----------------------------------------------------------------

/// `new Date(ms).toISOString()`: `YYYY-MM-DDTHH:MM:SS.sssZ` (web `instantIsoFromEpochMillis`).
#[must_use]
pub fn instant_iso_from_epoch_millis(ms: i64) -> String {
    let parts = utc_parts_from_epoch_millis(ms);
    let millis = ms.rem_euclid(1000);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        parts.year, parts.month, parts.day, parts.hour, parts.minute, parts.second, millis
    )
}

/// ISO-8601 timestamp from a logbook date plus elapsed seconds; invalid dates
/// fall back to now (web `logbookDatePlusSecondsIso`).
#[must_use]
pub fn logbook_date_plus_seconds_iso(date: &str, elapsed_sec: f64) -> String {
    let base = date.trim().replacen(' ', "T", 1);
    let has_offset = base.contains('Z') || has_trailing_numeric_offset(&base);
    let with_tz = if has_offset { base } else { format!("{base}Z") };
    let base_ms = parse_instant(&with_tz).unwrap_or_else(now_epoch_millis);
    instant_iso_from_epoch_millis(base_ms + js_round(elapsed_sec * MS_PER_SECOND) as i64)
}

/// `/[+-]\d{2}:\d{2}$/`
fn has_trailing_numeric_offset(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.len() < 6 {
        return false;
    }
    let tail = &bytes[bytes.len() - 6..];
    matches!(tail[0], b'+' | b'-')
        && tail[1].is_ascii_digit()
        && tail[2].is_ascii_digit()
        && tail[3] == b':'
        && tail[4].is_ascii_digit()
        && tail[5].is_ascii_digit()
}

/// Convenience: a `chrono::NaiveDate` for a valid day key.
#[must_use]
pub fn naive_date_from_key(key: &str) -> Option<NaiveDate> {
    let parts = parse_day_key(key)?;
    NaiveDate::from_ymd_opt(parts.year, parts.month, parts.day)
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- web datetime.test.ts -----------------------------------------------

    #[test]
    fn workout_local_day_key_cases() {
        assert_eq!(
            workout_local_day_key("2024-01-15 01:00:00", None, None),
            "2024-01-15"
        );
        assert_eq!(
            workout_local_day_key("2024-01-14 23:30:00", Some("America/New_York"), None),
            "2024-01-14"
        );
        assert_eq!(
            workout_local_day_key("2024-01-14 23:30:00", Some("Pacific/Auckland"), None),
            "2024-01-14"
        );
        assert_eq!(
            workout_local_day_key(
                "2024-01-14 23:30:00",
                Some("Pacific/Auckland"),
                Some("America/New_York")
            ),
            "2024-01-14"
        );
        assert_eq!(
            workout_local_day_key(
                "2024-01-14 23:30:00",
                Some("America/New_York"),
                Some("Pacific/Auckland")
            ),
            "2024-01-15"
        );
        assert_eq!(
            workout_local_day_key("2024-01-14 23:30:00", Some("Not/Real"), None),
            "2024-01-14"
        );
        assert_eq!(
            workout_local_day_key("2024-01-15 01:00:00", Some("Bad"), Some("America/New_York")),
            "2024-01-15"
        );
        assert_eq!(
            workout_local_day_key("2024-01-14 23:30:00", None, Some("America/New_York")),
            "2024-01-14"
        );
        assert_eq!(
            workout_local_day_key("2024-01-14 23:30:00", Some("Bad"), Some("Also/Bad")),
            "2024-01-14"
        );
        // Studio cases.
        assert_eq!(
            workout_local_day_key("2026-05-27 06:12:00", None, None),
            "2026-05-27"
        );
        assert_eq!(
            workout_local_day_key("2026-05-27 00:30:00", Some("America/Los_Angeles"), None),
            "2026-05-27"
        );
        assert_eq!(
            workout_local_day_key("2026-05-27 00:30:00", Some("UTC"), Some("Asia/Tokyo")),
            "2026-05-27"
        );
        assert_eq!(
            workout_local_day_key(
                "2026-05-27 01:00:00",
                Some("UTC"),
                Some("America/Los_Angeles")
            ),
            "2026-05-26"
        );
        assert_eq!(
            workout_local_day_key("garbage", Some("UTC"), Some("Asia/Tokyo")),
            "garbage"
        );
    }

    #[test]
    fn today_key_for_tz_cases() {
        assert_eq!(today_key_for_tz(None), today_key_utc());
        let key = today_key_for_tz(Some("America/New_York"));
        assert_eq!(key.len(), 10);
        assert!(parse_day_key(&key).is_some());
        assert_eq!(today_key_for_tz(Some("Not/AZone")), today_key_utc());
        assert_eq!(today_key_for_tz(Some("  ")), today_key_utc());
    }

    #[test]
    fn parse_logbook_date_time_cases() {
        let pdt = parse_logbook_date_time("2026-01-15 08:30:00").unwrap();
        assert_eq!(
            pdt,
            LogbookDateTime {
                year: 2026,
                month: 1,
                day: 15,
                hour: 8,
                minute: 30,
                second: 0
            }
        );
        assert_eq!(
            parse_logbook_date_time("  2026-06-01 12:00:00  ")
                .unwrap()
                .year,
            2026
        );
        assert!(parse_logbook_date_time("2026-01-15T12:00:00").is_some());
        assert_eq!(parse_logbook_date_time("not a date"), None);
        assert_eq!(parse_logbook_date_time(""), None);
        assert_eq!(parse_logbook_date_time("2026-99-01 00:00:00"), None);
        assert_eq!(parse_logbook_date_time("2026-13-01 00:00:00"), None);
        assert_eq!(parse_logbook_date_time("2026-01-01 25:00:00"), None);
        assert_eq!(parse_logbook_date_time("2026-02-30 00:00:00"), None);
        assert_eq!(
            parse_logbook_date_time("2024-02-29 00:00:00").map(|p| p.day),
            Some(29)
        );
        let padded = format!("{}2026-05-27 06:12:00", " ".repeat(1000));
        assert_eq!(parse_logbook_date_time(&padded), None);
    }

    #[test]
    fn logbook_epoch_millis_cases() {
        assert_eq!(
            logbook_epoch_millis("2000-01-01 00:00:00"),
            946_684_800_000.0
        );
        assert_eq!(
            logbook_epoch_millis("2026-05-27 06:12:00"),
            1_779_862_320_000.0
        );
        assert!(logbook_epoch_millis("bad").is_nan());
        assert!(logbook_epoch_millis("garbage").is_nan());
    }

    #[test]
    fn parse_instant_millis_cases() {
        assert_eq!(
            parse_instant_millis("2000-01-01T00:00:00Z"),
            946_684_800_000.0
        );
        assert_eq!(
            parse_instant_millis("2000-01-01T01:00:00+01:00"),
            946_684_800_000.0
        );
        assert_eq!(
            parse_instant_millis("2000-01-01T00:00:00.5Z"),
            946_684_800_500.0
        );
        assert_eq!(
            parse_instant_millis("2000-01-01T00:00:00.123456Z"),
            946_684_800_123.0
        );
        assert!(parse_instant_millis("not a timestamp").is_nan());
        assert!(parse_instant_millis("").is_nan());
        assert!(parse_instant_millis("2000-01-01T00:00:00").is_nan());
        assert!(parse_instant_millis("2026-02-29T12:00:00Z").is_nan());
        assert!(parse_instant_millis("2026-04-31T00:00:00Z").is_nan());
        assert_eq!(
            parse_instant_millis("2024-02-29T12:00:00Z"),
            1_709_208_000_000.0
        );
        assert_eq!(parse_instant("2000-01-01T00:00:00Z"), Some(946_684_800_000));
        assert_eq!(parse_instant("2000-01-01T00:00:00+99:00"), None);
    }

    #[test]
    fn overlap_date_cases() {
        assert_eq!(
            overlap_date("2026-06-03 10:00:00").as_deref(),
            Some("2026-06-02")
        );
        assert_eq!(
            overlap_date("2026-03-01 00:00:00").as_deref(),
            Some("2026-02-28")
        );
        assert_eq!(
            overlap_date("2027-01-01 00:00:00").as_deref(),
            Some("2026-12-31")
        );
        assert_eq!(overlap_date("2026-06-03").as_deref(), Some("2026-06-02"));
        assert_eq!(overlap_date("invalid"), None);
        assert_eq!(overlap_date(""), None);
    }

    #[test]
    fn today_key_utc_is_a_day_key() {
        let key = today_key_utc();
        assert_eq!(key.len(), 10);
        assert!(parse_day_key(&key).is_some());
    }

    #[test]
    fn day_key_epoch_millis_cases() {
        assert_eq!(day_key_epoch_millis("2000-01-01"), 946_684_800_000.0);
        let ms = day_key_epoch_millis("2026-06-03");
        assert!(ms > 0.0 && ms.is_finite());
        assert_eq!(day_key_from_epoch_millis(ms as i64), "2026-06-03");
        assert!(day_key_epoch_millis("not-a-date").is_nan());
        assert!(day_key_epoch_millis("").is_nan());
        assert!(day_key_epoch_millis("bad").is_nan());
        let padded = format!("{}2026-05-27", " ".repeat(1000));
        assert!(day_key_epoch_millis(&padded).is_nan());
    }

    // --- Studio RowPlayDateTimeTests ----------------------------------------

    #[test]
    fn day_key_arithmetic() {
        assert_eq!(day_key_from_epoch_millis(1_779_840_000_000), "2026-05-27");
        assert_eq!(add_days_to_key("2026-05-27", 1), "2026-05-28");
        assert_eq!(add_days_to_key("2026-05-01", -1), "2026-04-30");
        assert_eq!(add_days_to_key("2026-01-31", 1), "2026-02-01");
        assert_eq!(add_days_to_key("not-a-date", 1), "not-a-date");
        assert_eq!(days_between_utc("2026-05-27", "2026-05-27"), 0);
        assert_eq!(days_between_utc("2026-05-27", "2026-05-28"), 1);
        assert_eq!(days_between_utc("2026-05-28", "2026-05-27"), 0);
        assert_eq!(days_between_utc("2026-05-30", "2026-06-02"), 3);
        assert_eq!(days_between_utc("bad", "2026-05-27"), 0);
        assert_eq!(day_of_week_utc("2026-05-27"), 3);
        assert_eq!(day_of_week_utc("2026-05-24"), 0);
        assert_eq!(day_of_week_utc("2026-05-23"), 6);
        assert_eq!(day_of_week_utc("not-a-date"), 0);
        assert_eq!(day_of_week_utc("1970-01-01"), 4);
        assert_eq!(day_of_year_utc("2026-01-01"), 1);
        assert_eq!(day_of_year_utc("2026-05-27"), 147);
        assert_eq!(day_of_year_utc("2024-12-31"), 366);
        assert_eq!(day_of_year_utc("bad"), 0);
    }

    #[test]
    fn add_months_clamps_to_month_end() {
        assert_eq!(add_months_to_key("2026-01-31", 1), "2026-02-28");
        assert_eq!(add_months_to_key("2024-01-31", 1), "2024-02-29");
        assert_eq!(add_months_to_key("2026-03-15", -3), "2025-12-15");
        assert_eq!(add_months_to_key("2026-12-01", 1), "2027-01-01");
        assert_eq!(add_months_to_key("bad", 1), "bad");
    }

    #[test]
    fn iso_output() {
        assert_eq!(
            instant_iso_from_epoch_millis(946_684_800_000),
            "2000-01-01T00:00:00.000Z"
        );
        assert_eq!(
            instant_iso_from_epoch_millis(946_684_800_123),
            "2000-01-01T00:00:00.123Z"
        );
        let iso = now_iso_string();
        assert_eq!(iso.len(), 24);
        assert!(iso.ends_with('Z'));
        assert!(parse_instant(&iso).is_some());
        assert_eq!(
            logbook_date_plus_seconds_iso("2000-01-01 00:00:00", 90.4),
            "2000-01-01T00:01:30.400Z"
        );
        assert_eq!(
            logbook_date_plus_seconds_iso("2000-01-01T01:00:00+01:00", 0.0),
            "2000-01-01T00:00:00.000Z"
        );
        // Invalid dates fall back to "now": only check the shape.
        assert!(parse_instant(&logbook_date_plus_seconds_iso("garbage", 1.0)).is_some());
        assert!(current_utc_year() >= 2026);
        assert_eq!(naive_date_from_key("2026-05-27").map(|d| d.day()), Some(27));
    }

    #[test]
    fn civil_round_trip() {
        for days in [-1_000_000, -1, 0, 1, 19_000, 30_000, 2_000_000] {
            let (y, m, d) = civil_from_days(days);
            assert_eq!(days_from_civil(y, m, d), days);
        }
    }
}
