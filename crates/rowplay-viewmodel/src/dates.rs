// SPDX-License-Identifier: GPL-3.0-or-later
//! Locale date display for the six supported languages.
//!
//! Ports the display half of the web's `src/lib/datetime.ts` (`fmtDate`,
//! `fmtLogbookDateTime`, `fmtTimeFromEpochMillis`, `monthShortName`), which
//! delegates to `Intl.DateTimeFormat`. `Intl` is unavailable here, so the
//! per-language patterns and month-name tables below were generated from the
//! web app itself (pinned `011e830`) with Node's `Intl` and are asserted
//! against those golden outputs in the tests.
//!
//! Divergence (recorded in `docs/source-map.md`): the web formats ISO
//! instants in the *browser's* time zone when no zone is given; this port uses
//! the caller-supplied home time zone (the desktop equivalent of the
//! `settings.timezone*` preference) and UTC when unset. Logbook strings and
//! day keys follow the web's fixed UTC interpretation (`LOGBOOK_ZONE`).

use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, Timelike, Utc};
use rowplay_core::datetime::{
    day_key_epoch_millis, parse_instant_millis, parse_logbook_date_time, utc_epoch_millis,
};

use crate::settings::Language;

/// Short month name for a 1-based month, standalone (web `monthShortName`).
///
/// Note the German standalone forms differ from the in-date forms
/// (`Mär` vs `März`), exactly like `Intl`.
#[must_use]
pub fn month_short_name(month: u32, language: Language) -> &'static str {
    let idx = month_index(month);
    match language {
        Language::En => EN_MONTHS[idx],
        Language::Zh | Language::Ja => CJK_MONTHS[idx],
        Language::De => DE_MONTHS_STANDALONE[idx],
        Language::Es => ES_MONTHS[idx],
        Language::Fr => FR_MONTHS[idx],
    }
}

/// Short locale date from a logbook string, ISO instant or day key (web
/// `fmtDate` without an explicit zone: `{year numeric, month short, day
/// numeric}`). Falls back to the input verbatim, like the web.
#[must_use]
pub fn fmt_date(value: &str, language: Language, home_timezone: Option<&str>) -> String {
    let Some(date) = display_date(value, home_timezone) else {
        return value.to_owned();
    };
    fmt_short_date(date.year(), date.month(), date.day(), language)
}

/// Locale date-time from a logbook `YYYY-MM-DD HH:MM:SS` string (web
/// `fmtLogbookDateTime`: numeric year/month/day, numeric hour, 2-digit
/// minute/second). Falls back to the input verbatim, like the web.
#[must_use]
pub fn fmt_logbook_date_time(value: &str, language: Language) -> String {
    let Some(parts) = parse_logbook_date_time(value) else {
        return value.to_owned();
    };
    let (year, month, day) = (parts.year, parts.month, parts.day);
    let time = clock_text(language, parts.hour, parts.minute, parts.second, false);
    let date = match language {
        Language::En => format!("{month}/{day}/{year}"),
        Language::De => format!("{day}.{month}.{year}"),
        Language::Es => format!("{day}/{month}/{year}"),
        Language::Fr => format!("{day:02}/{month:02}/{year}"),
        Language::Zh | Language::Ja => format!("{year}/{month}/{day}"),
    };
    match language {
        Language::En | Language::De | Language::Es => format!("{date}, {time}"),
        Language::Fr | Language::Zh | Language::Ja => format!("{date} {time}"),
    }
}

/// Locale time-of-day from epoch milliseconds (web `fmtTimeFromEpochMillis`
/// with `TIME_FMT`: 2-digit hour/minute/second). `timezone` is an IANA name;
/// an unknown zone falls back to UTC like the web's candidate list.
#[must_use]
pub fn fmt_time_from_epoch_millis(
    epoch_ms: f64,
    language: Language,
    timezone: Option<&str>,
) -> String {
    let Some(dt) = in_timezone(epoch_ms, timezone) else {
        return "--".to_owned();
    };
    clock_text(language, dt.hour(), dt.minute(), dt.second(), true)
}

/// The calendar date used for display, resolving the same three input shapes
/// as the web's `fmtDate`: ISO instant → logbook string → day key.
fn display_date(value: &str, home_timezone: Option<&str>) -> Option<NaiveDate> {
    let instant_ms = parse_instant_millis(value);
    if instant_ms.is_finite() {
        return in_timezone(instant_ms, home_timezone).map(|dt| dt.date());
    }
    if let Some(parts) = parse_logbook_date_time(value) {
        return naive_date_from_millis(utc_epoch_millis(parts) as f64);
    }
    let key_ms = day_key_epoch_millis(value);
    if key_ms.is_finite() {
        return naive_date_from_millis(key_ms);
    }
    None
}

fn naive_date_from_millis(ms: f64) -> Option<NaiveDate> {
    DateTime::<Utc>::from_timestamp_millis(ms as i64).map(|dt| dt.date_naive())
}

/// Wall-clock date-time in `timezone` (IANA), UTC when unset or unknown.
fn in_timezone(epoch_ms: f64, timezone: Option<&str>) -> Option<NaiveDateTime> {
    let naive_utc = DateTime::<Utc>::from_timestamp_millis(epoch_ms as i64)?.naive_utc();
    match timezone.and_then(|name| name.parse::<chrono_tz::Tz>().ok()) {
        Some(tz) => {
            use chrono::TimeZone as _;
            Some(tz.from_utc_datetime(&naive_utc).naive_local())
        }
        None => Some(naive_utc),
    }
}

/// `Intl` `{year numeric, month short, day numeric}` patterns, golden-tested.
fn fmt_short_date(year: i32, month: u32, day: u32, language: Language) -> String {
    let idx = month_index(month);
    match language {
        Language::En => format!("{} {}, {}", EN_MONTHS[idx], day, year),
        Language::De => format!("{}. {} {}", day, DE_MONTHS_IN_DATE[idx], year),
        Language::Es => format!("{} {} {}", day, ES_MONTHS[idx], year),
        Language::Fr => format!("{} {} {}", day, FR_MONTHS[idx], year),
        Language::Zh | Language::Ja => format!("{year}年{month}月{day}日"),
    }
}

/// `Intl` clock patterns: en is 12-hour with AM/PM, everything else 24-hour.
/// `pad_hour` selects the 2-digit hour (`TIME_FMT`) over the numeric hour
/// (`fmtLogbookDateTime`).
fn clock_text(language: Language, hour24: u32, minute: u32, second: u32, pad_hour: bool) -> String {
    if language == Language::En {
        let suffix = if hour24 < 12 { "AM" } else { "PM" };
        let hour12 = match hour24 % 12 {
            0 => 12,
            h => h,
        };
        if pad_hour {
            format!("{hour12:02}:{minute:02}:{second:02} {suffix}")
        } else {
            format!("{hour12}:{minute:02}:{second:02} {suffix}")
        }
    } else if pad_hour {
        format!("{hour24:02}:{minute:02}:{second:02}")
    } else {
        format!("{hour24}:{minute:02}:{second:02}")
    }
}

fn month_index(month: u32) -> usize {
    month.saturating_sub(1).min(11) as usize
}

const EN_MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
/// German in-date short months carry dots except `März`; standalone `Mär`
/// does not — both straight from `Intl`.
const DE_MONTHS_STANDALONE: [&str; 12] = [
    "Jan", "Feb", "Mär", "Apr", "Mai", "Jun", "Jul", "Aug", "Sep", "Okt", "Nov", "Dez",
];
const DE_MONTHS_IN_DATE: [&str; 12] = [
    "Jan.", "Feb.", "März", "Apr.", "Mai", "Juni", "Juli", "Aug.", "Sept.", "Okt.", "Nov.", "Dez.",
];
const ES_MONTHS: [&str; 12] = [
    "ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sept", "oct", "nov", "dic",
];
const FR_MONTHS: [&str; 12] = [
    "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.",
    "déc.",
];
const CJK_MONTHS: [&str; 12] = [
    "1月", "2月", "3月", "4月", "5月", "6月", "7月", "8月", "9月", "10月", "11月", "12月",
];

#[cfg(test)]
mod tests {
    use super::*;

    /// Golden outputs generated from the web app (`src/lib/datetime.ts`,
    /// pinned 011e830) running on Node's `Intl`:
    /// `fmtDate("2026-MM-01", lang)` for every month.
    #[test]
    fn fmt_date_matches_intl_goldens() {
        let en = [
            "Jan 1, 2026",
            "Feb 1, 2026",
            "Mar 1, 2026",
            "Apr 1, 2026",
            "May 1, 2026",
            "Jun 1, 2026",
            "Jul 1, 2026",
            "Aug 1, 2026",
            "Sep 1, 2026",
            "Oct 1, 2026",
            "Nov 1, 2026",
            "Dec 1, 2026",
        ];
        let de = [
            "1. Jan. 2026",
            "1. Feb. 2026",
            "1. März 2026",
            "1. Apr. 2026",
            "1. Mai 2026",
            "1. Juni 2026",
            "1. Juli 2026",
            "1. Aug. 2026",
            "1. Sept. 2026",
            "1. Okt. 2026",
            "1. Nov. 2026",
            "1. Dez. 2026",
        ];
        let es = [
            "1 ene 2026",
            "1 feb 2026",
            "1 mar 2026",
            "1 abr 2026",
            "1 may 2026",
            "1 jun 2026",
            "1 jul 2026",
            "1 ago 2026",
            "1 sept 2026",
            "1 oct 2026",
            "1 nov 2026",
            "1 dic 2026",
        ];
        let fr = [
            "1 janv. 2026",
            "1 févr. 2026",
            "1 mars 2026",
            "1 avr. 2026",
            "1 mai 2026",
            "1 juin 2026",
            "1 juil. 2026",
            "1 août 2026",
            "1 sept. 2026",
            "1 oct. 2026",
            "1 nov. 2026",
            "1 déc. 2026",
        ];
        let cjk = [
            "2026年1月1日",
            "2026年2月1日",
            "2026年3月1日",
            "2026年4月1日",
            "2026年5月1日",
            "2026年6月1日",
            "2026年7月1日",
            "2026年8月1日",
            "2026年9月1日",
            "2026年10月1日",
            "2026年11月1日",
            "2026年12月1日",
        ];
        for month in 1..=12 {
            let key = format!("2026-{month:02}-01");
            let i = (month - 1) as usize;
            assert_eq!(fmt_date(&key, Language::En, None), en[i], "en {key}");
            assert_eq!(fmt_date(&key, Language::De, None), de[i], "de {key}");
            assert_eq!(fmt_date(&key, Language::Es, None), es[i], "es {key}");
            assert_eq!(fmt_date(&key, Language::Fr, None), fr[i], "fr {key}");
            assert_eq!(fmt_date(&key, Language::Ja, None), cjk[i], "ja {key}");
            assert_eq!(fmt_date(&key, Language::Zh, None), cjk[i], "zh {key}");
        }
    }

    /// Web `monthShortName(m, lang)` goldens (standalone forms).
    #[test]
    fn month_short_name_matches_intl_goldens() {
        for month in 1..=12u32 {
            let i = (month - 1) as usize;
            assert_eq!(month_short_name(month, Language::En), EN_MONTHS[i]);
            assert_eq!(
                month_short_name(month, Language::De),
                DE_MONTHS_STANDALONE[i]
            );
            assert_eq!(month_short_name(month, Language::Es), ES_MONTHS[i]);
            assert_eq!(month_short_name(month, Language::Fr), FR_MONTHS[i]);
            assert_eq!(month_short_name(month, Language::Ja), CJK_MONTHS[i]);
            assert_eq!(month_short_name(month, Language::Zh), CJK_MONTHS[i]);
        }
    }

    /// Web `fmtDate` on a logbook timestamp: every language, golden-checked.
    #[test]
    fn fmt_date_handles_logbook_strings() {
        let value = "2026-03-05 06:07:08";
        assert_eq!(fmt_date(value, Language::En, None), "Mar 5, 2026");
        assert_eq!(fmt_date(value, Language::De, None), "5. März 2026");
        assert_eq!(fmt_date(value, Language::Zh, None), "2026年3月5日");
        assert_eq!(fmt_date(value, Language::Ja, None), "2026年3月5日");
        assert_eq!(fmt_date(value, Language::Es, None), "5 mar 2026");
        assert_eq!(fmt_date(value, Language::Fr, None), "5 mars 2026");
    }

    /// Unparseable input comes back verbatim (web fallback).
    #[test]
    fn fmt_date_falls_back_to_the_input() {
        assert_eq!(fmt_date("not a date", Language::En, None), "not a date");
        assert_eq!(fmt_logbook_date_time("garbage", Language::En), "garbage");
    }

    /// Web `fmtLogbookDateTime` goldens, including the midnight and PM edges.
    #[test]
    fn fmt_logbook_date_time_matches_intl_goldens() {
        assert_eq!(
            fmt_logbook_date_time("2026-03-05 06:07:08", Language::En),
            "3/5/2026, 6:07:08 AM"
        );
        assert_eq!(
            fmt_logbook_date_time("2026-12-25 23:45:09", Language::En),
            "12/25/2026, 11:45:09 PM"
        );
        assert_eq!(
            fmt_logbook_date_time("2026-01-09 00:01:02", Language::En),
            "1/9/2026, 12:01:02 AM"
        );
        assert_eq!(
            fmt_logbook_date_time("2026-03-05 06:07:08", Language::Zh),
            "2026/3/5 6:07:08"
        );
        assert_eq!(
            fmt_logbook_date_time("2026-03-05 06:07:08", Language::De),
            "5.3.2026, 6:07:08"
        );
        assert_eq!(
            fmt_logbook_date_time("2026-03-05 06:07:08", Language::Es),
            "5/3/2026, 6:07:08"
        );
        assert_eq!(
            fmt_logbook_date_time("2026-03-05 06:07:08", Language::Fr),
            "05/03/2026 6:07:08"
        );
        assert_eq!(
            fmt_logbook_date_time("2026-01-09 00:01:02", Language::Fr),
            "09/01/2026 0:01:02"
        );
        assert_eq!(
            fmt_logbook_date_time("2026-03-05 06:07:08", Language::Ja),
            "2026/3/5 6:07:08"
        );
        assert_eq!(
            fmt_logbook_date_time("2026-01-09 00:01:02", Language::Ja),
            "2026/1/9 0:01:02"
        );
    }

    /// Web `fmtTimeFromEpochMillis(1741154820000, lang, "UTC")` goldens.
    #[test]
    fn fmt_time_from_epoch_millis_matches_intl_goldens() {
        let ms = 1_741_154_820_000.0; // 2025-03-05T06:07:00Z
        assert_eq!(
            fmt_time_from_epoch_millis(ms, Language::En, Some("UTC")),
            "06:07:00 AM"
        );
        for language in [
            Language::Zh,
            Language::De,
            Language::Es,
            Language::Fr,
            Language::Ja,
        ] {
            assert_eq!(
                fmt_time_from_epoch_millis(ms, language, Some("UTC")),
                "06:07:00"
            );
        }
        // Home time zone shifts the wall clock (Asia/Tokyo = UTC+9).
        assert_eq!(
            fmt_time_from_epoch_millis(ms, Language::Ja, Some("Asia/Tokyo")),
            "15:07:00"
        );
    }

    /// ISO instants resolve through the home time zone when one is set
    /// (documented divergence: the web uses the browser's zone).
    #[test]
    fn fmt_date_uses_the_home_timezone_for_instants() {
        let instant = "2026-03-05T23:30:00.000Z";
        assert_eq!(fmt_date(instant, Language::En, None), "Mar 5, 2026");
        assert_eq!(
            fmt_date(instant, Language::En, Some("Asia/Tokyo")),
            "Mar 6, 2026"
        );
        assert_eq!(
            fmt_date(instant, Language::En, Some("Not/AZone")),
            "Mar 5, 2026"
        );
    }
}
