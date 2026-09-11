// SPDX-License-Identifier: GPL-3.0-or-later
//! Parse and format user-entered `/500m` pace strings (`M:SS` or bare seconds).
//!
//! Port of the web app's `src/lib/paceInput.ts` with the input-length bounds
//! from rowplay-studio's `PaceInput.swift` (raw ≤ 64 characters, trimmed ≤ 20)
//! so untrusted text is bounded before it is scanned.

use crate::num::{js_round, pad_int};

const MAX_RAW_LEN: usize = 64;
const MAX_TRIMMED_LEN: usize = 20;

/// Parse `M:SS`, `MM:SS`, `M:SS.t` or bare seconds into positive seconds.
///
/// Returns `None` for empty, malformed, non-positive, over-long or non-finite input.
#[must_use]
pub fn parse_pace_input(raw: &str) -> Option<f64> {
    if raw.len() > MAX_RAW_LEN {
        return None;
    }
    let s = raw.trim();
    if s.len() > MAX_TRIMMED_LEN {
        return None;
    }
    let total = if let Some((minutes, seconds)) = s.split_once(':') {
        // ^(\d+):([0-5]?\d(?:\.\d+)?)$
        if minutes.is_empty() || !minutes.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let (int_secs, frac) = match seconds.split_once('.') {
            Some((int_secs, frac)) => (int_secs, Some(frac)),
            None => (seconds, None),
        };
        let int_ok = match int_secs.as_bytes() {
            [d] => d.is_ascii_digit(),
            [tens, d] => (b'0'..=b'5').contains(tens) && d.is_ascii_digit(),
            _ => false,
        };
        if !int_ok {
            return None;
        }
        if frac.is_some_and(|frac| frac.is_empty() || !frac.bytes().all(|b| b.is_ascii_digit())) {
            return None;
        }
        let minutes: f64 = minutes.parse().ok()?;
        let seconds: f64 = seconds.parse().ok()?;
        minutes * 60.0 + seconds
    } else {
        // ^(\d+(?:\.\d+)?)$
        let (int_part, frac) = match s.split_once('.') {
            Some((int_part, frac)) => (int_part, Some(frac)),
            None => (s, None),
        };
        if int_part.is_empty() || !int_part.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        if frac.is_some_and(|frac| frac.is_empty() || !frac.bytes().all(|b| b.is_ascii_digit())) {
            return None;
        }
        s.parse::<f64>().ok()?
    };
    if total > 0.0 && total.is_finite() {
        Some(total)
    } else {
        None
    }
}

/// Format positive seconds as canonical `M:SS` for display or prefill.
///
/// Returns an empty string for non-positive, non-finite or unrepresentable input.
#[must_use]
pub fn format_pace_input(seconds: f64) -> String {
    if !seconds.is_finite() || seconds <= 0.0 {
        return String::new();
    }
    let whole = js_round(seconds);
    if whole >= 1e15 {
        return String::new();
    }
    let minutes = (whole / 60.0).floor();
    let secs = whole % 60.0;
    format!("{}:{}", minutes as i64, pad_int(secs, 2))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_common_paces() {
        assert_eq!(format_pace_input(112.0), "1:52");
        assert_eq!(format_pace_input(120.0), "2:00");
        assert_eq!(format_pace_input(90.0), "1:30");
        assert_eq!(format_pace_input(425.0), "7:05");
        assert_eq!(format_pace_input(120.4), "2:00");
        assert_eq!(format_pace_input(120.6), "2:01");
    }

    #[test]
    fn format_returns_empty_for_invalid_values() {
        assert_eq!(format_pace_input(0.0), "");
        assert_eq!(format_pace_input(-1.0), "");
        assert_eq!(format_pace_input(-5.0), "");
        assert_eq!(format_pace_input(f64::INFINITY), "");
        assert_eq!(format_pace_input(f64::NAN), "");
        assert_eq!(format_pace_input(f64::MAX), "");
    }

    #[test]
    fn round_trips_integer_seconds() {
        for n in [60.0, 90.0, 100.0, 112.0, 120.0, 180.0, 240.0] {
            assert_eq!(parse_pace_input(&format_pace_input(n)), Some(n));
        }
        for input in ["1:30", "2:00", "7:05", "10:00", "1:59"] {
            let parsed = parse_pace_input(input).unwrap();
            let reparsed = parse_pace_input(&format_pace_input(parsed)).unwrap();
            assert!((reparsed - parsed).abs() <= 0.5, "{input}");
        }
    }

    #[test]
    fn parses_clock_and_bare_forms() {
        assert_eq!(parse_pace_input("1:52"), Some(112.0));
        assert_eq!(parse_pace_input("01:52"), Some(112.0));
        assert_eq!(parse_pace_input("2:05"), Some(125.0));
        assert_eq!(parse_pace_input("112"), Some(112.0));
        assert_eq!(parse_pace_input("2:00"), Some(120.0));
        assert_eq!(parse_pace_input("1:30"), Some(90.0));
        assert_eq!(parse_pace_input("7:05"), Some(425.0));
        assert!((parse_pace_input("2:05.5").unwrap() - 125.5).abs() < 0.01);
        assert_eq!(parse_pace_input("90"), Some(90.0));
        assert!((parse_pace_input("120.5").unwrap() - 120.5).abs() < 0.01);
        assert_eq!(parse_pace_input("  2:00  "), Some(120.0));
        assert_eq!(parse_pace_input("\t2:00\n"), Some(120.0));
    }

    #[test]
    fn rejects_invalid_input() {
        for bad in [
            "", "0:00", "abc", "-30", "99:99", "-1:30", "1:60", "1:90", "1e2", "0", "1:", ":30",
            "1.", "1:5.", "1:.5",
        ] {
            assert_eq!(parse_pace_input(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn rejects_long_raw_input_before_trimming() {
        let padded = format!("{}2:00", " ".repeat(1000));
        assert_eq!(parse_pace_input(&padded), None);
        let long_digits = "1".repeat(21);
        assert_eq!(parse_pace_input(&long_digits), None);
    }
}
