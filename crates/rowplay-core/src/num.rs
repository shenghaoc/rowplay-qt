// SPDX-License-Identifier: GPL-3.0-or-later
//! JavaScript-compatible numeric helpers.
//!
//! The web app is the parity oracle, so the ports need `Math.round` and
//! `Number.prototype.toFixed` semantics rather than Rust's defaults:
//! `Math.round` rounds half toward +∞ (Rust's `f64::round` rounds half away
//! from zero) and `toFixed` rounds ties away from zero on the *exact* binary
//! value (Rust's `{:.N}` rounds ties to even).

/// `Math.round(x)`: nearest integer, ties toward +∞. Non-finite values pass through.
#[must_use]
pub fn js_round(x: f64) -> f64 {
    if !x.is_finite() {
        return x;
    }
    let floor = x.floor();
    // `x - floor` is exact for |x| < 2^52, which covers every value we round.
    if x - floor >= 0.5 { floor + 1.0 } else { floor }
}

/// `Math.round(x * 10) / 10` — the web app's `round1`.
#[must_use]
pub fn round1(x: f64) -> f64 {
    js_round(x * 10.0) / 10.0
}

/// `Number.prototype.toFixed(digits)` for finite values below 1e21.
///
/// Ties are resolved on the exact decimal expansion of the binary value and
/// rounded away from zero, matching ECMAScript. Negative inputs keep their sign
/// even when the rounded magnitude is zero (`(-0.04).toFixed(1) === "-0.0"`).
#[must_use]
pub fn js_to_fixed(x: f64, digits: usize) -> String {
    if x.is_nan() {
        return "NaN".to_owned();
    }
    if x.is_infinite() {
        return if x > 0.0 { "Infinity" } else { "-Infinity" }.to_owned();
    }
    if x.abs() >= 1e21 {
        // ECMAScript falls back to ToString(x) here; exponent notation is not
        // something the app ever formats, so plain Rust formatting is enough.
        return format!("{x}");
    }
    let negative = x < 0.0;
    let magnitude = x.abs();
    // Rust prints the exact decimal expansion when asked for enough digits; a
    // double >= 1e-8 has at most ~80 fractional digits, so this is exact.
    let exact = format!("{magnitude:.*}", digits + 80);
    let (int_part, frac_part) = exact.split_once('.').unwrap_or((&exact, ""));
    let keep = &frac_part[..digits.min(frac_part.len())];
    let rest = &frac_part[digits.min(frac_part.len())..];
    let round_up = rest.as_bytes().first().is_some_and(|first| *first >= b'5');

    let mut digits_out: Vec<u8> = int_part.bytes().chain(keep.bytes()).collect();
    if round_up {
        let mut i = digits_out.len();
        loop {
            if i == 0 {
                digits_out.insert(0, b'1');
                break;
            }
            i -= 1;
            if digits_out[i] == b'9' {
                digits_out[i] = b'0';
            } else {
                digits_out[i] += 1;
                break;
            }
        }
    }
    let int_len = digits_out.len() - digits;
    let mut out = String::with_capacity(digits_out.len() + 2);
    if negative {
        out.push('-');
    }
    out.push_str(std::str::from_utf8(&digits_out[..int_len]).expect("ascii digits"));
    if digits > 0 {
        out.push('.');
        out.push_str(std::str::from_utf8(&digits_out[int_len..]).expect("ascii digits"));
    }
    out
}

/// `String(Math.floor(x)).padStart(width, "0")` for non-negative finite `x`.
#[must_use]
pub fn pad_int(x: f64, width: usize) -> String {
    let value = x.floor();
    if !value.is_finite() || value.abs() >= 1e15 {
        return format!("{value}");
    }
    format!("{:0width$}", value as i64, width = width)
}

/// Arithmetic mean of a slice; `0.0` when empty (web `avg`).
#[must_use]
pub fn avg(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

/// Median of a slice; `0.0` when empty (web `median`).
#[must_use]
pub fn median(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 1 {
        sorted[mid]
    } else {
        (sorted[mid - 1] + sorted[mid]) / 2.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn js_round_matches_math_round() {
        assert_eq!(js_round(2.5), 3.0);
        assert_eq!(js_round(-2.5), -2.0);
        assert_eq!(js_round(0.499_999_999_999_999_94), 0.0);
        assert_eq!(js_round(-0.4), -0.0);
        assert!(js_round(f64::NAN).is_nan());
    }

    #[test]
    fn to_fixed_matches_ecmascript() {
        assert_eq!(js_to_fixed(125.3, 1), "125.3");
        assert_eq!(js_to_fixed(0.25, 1), "0.3"); // exact tie rounds away from zero
        assert_eq!(js_to_fixed(0.35, 1), "0.3"); // 0.35 is really 0.34999…
        assert_eq!(js_to_fixed(1.005, 2), "1.00"); // 1.005 is really 1.00499…
        assert_eq!(js_to_fixed(2.5, 2), "2.50");
        assert_eq!(js_to_fixed(9.96, 1), "10.0");
        assert_eq!(js_to_fixed(59.96, 1), "60.0");
        assert_eq!(js_to_fixed(-0.04, 1), "-0.0");
        assert_eq!(js_to_fixed(1234.5, 0), "1235");
        assert_eq!(js_to_fixed(0.0, 1), "0.0");
        assert_eq!(js_to_fixed(f64::NAN, 1), "NaN");
    }

    #[test]
    fn pad_int_pads_with_zeros() {
        assert_eq!(pad_int(5.0, 2), "05");
        assert_eq!(pad_int(59.9, 2), "59");
        assert_eq!(pad_int(123.0, 2), "123");
    }

    #[test]
    fn avg_and_median() {
        assert_eq!(avg(&[]), 0.0);
        assert_eq!(avg(&[1.0, 2.0, 3.0]), 2.0);
        assert_eq!(median(&[]), 0.0);
        assert_eq!(median(&[3.0, 1.0, 2.0]), 2.0);
        assert_eq!(median(&[4.0, 1.0, 2.0, 3.0]), 2.5);
    }
}
