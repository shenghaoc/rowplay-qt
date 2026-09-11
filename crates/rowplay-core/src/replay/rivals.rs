// SPDX-License-Identifier: GPL-3.0-or-later
//! Ghost "sources" for replay comparison (web `replay/sources.ts`, Studio
//! `ReplayRival*`): a constant pace, or an uploaded CSV / TCX / FIT trace.
//! Each resolves to a `Vec<Stroke>` that the replay renders in the ghost lane
//! via `sample_at`.
//!
//! The web decodes in the browser (`DOMParser`, `DataView`, `File`); here the
//! parsers consume byte slices with Studio's hardening — bounded file size
//! and sample count, streaming quoted CSV, a namespace-insensitive TCX
//! scanner that rejects DTDs, and a structurally validated FIT decoder —
//! plus Studio's trace normalisation (sort, one sample per timestamp, no
//! backward distance, zero-based time). No I/O: the platform layer reads the
//! bytes.

use crate::formatting::pace_to_watts_for_sport;
use crate::models::{Sport, Stroke};

/// Rival files are rejected above this size (Studio: 25 MiB).
pub const MAXIMUM_FILE_SIZE_BYTES: usize = 25 * 1024 * 1024;
/// Rival traces are rejected above this many usable samples.
pub const MAXIMUM_ACCEPTED_SAMPLES: usize = 200_000;

/// Errors from rival file import. Messages never include paths or contents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RivalParseError {
    /// Input above [`MAXIMUM_FILE_SIZE_BYTES`].
    #[error("rival file exceeds the 25 MiB size limit")]
    FileTooLarge,
    /// The bytes could not be decoded as text.
    #[error("could not read the selected rival file")]
    Unreadable,
    /// No usable samples survived normalisation.
    #[error("no usable samples found in the selected file")]
    UnsupportedOrEmpty,
    /// Fewer than two usable samples.
    #[error("rival file needs at least two usable samples")]
    TooFewSamples,
    /// More than [`MAXIMUM_ACCEPTED_SAMPLES`] usable samples.
    #[error("rival file exceeds the maximum sample count")]
    TooManySamples,
    /// Structurally broken payload (unbalanced markup, bad FIT header, …).
    #[error("rival file is malformed or truncated")]
    Malformed,
}

/// A parsed rival trace plus its display file name (last path component
/// only — never a full path).
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedTrace {
    /// Normalised ghost stroke array.
    pub strokes: Vec<Stroke>,
    /// Display file name (last path component only).
    pub file_name: String,
}

/// Intermediate sample shared by the format decoders before common
/// validation, ordering and pace/power derivation.
#[derive(Debug, Clone, Copy, PartialEq)]
struct RawSample {
    t: f64,
    d: f64,
    pace: Option<f64>,
    spm: Option<f64>,
    hr: Option<f64>,
    watts: Option<f64>,
}

// ── constant pace ────────────────────────────────────────────────────────────

/// A flat-pace pacer ghost over `total_distance` (web `constantPaceGhost` /
/// Studio `constantPaceStrokes`). Two points suffice because `sample_at`
/// interpolates linearly between samples; watts use the sport's PM basis
/// (BikeErg divisor 8).
#[must_use]
pub fn constant_pace_strokes(pace_per_500m: f64, total_distance: f64, sport: Sport) -> Vec<Stroke> {
    if !is_valid_positive_finite(pace_per_500m) || !is_valid_positive_finite(total_distance) {
        return Vec::new();
    }
    let total_time = (total_distance / 500.0) * pace_per_500m;
    if !is_valid_positive_finite(total_time) || total_time >= 1e12 {
        return Vec::new();
    }
    let watts = pace_to_watts_for_sport(sport, pace_per_500m);
    if !watts.is_finite() || watts <= 0.0 {
        return Vec::new();
    }
    vec![
        Stroke {
            t: 0.0,
            d: 0.0,
            pace: pace_per_500m,
            spm: 0.0,
            hr: None,
            watts,
            raw_t: None,
            raw_d: None,
        },
        Stroke {
            t: total_time,
            d: total_distance,
            pace: pace_per_500m,
            spm: 0.0,
            hr: None,
            watts,
            raw_t: None,
            raw_d: None,
        },
    ]
}

/// The web's rower-basis name for [`constant_pace_strokes`].
#[must_use]
pub fn constant_pace_ghost(pace_per_500m: f64, total_distance: f64) -> Vec<Stroke> {
    constant_pace_strokes(pace_per_500m, total_distance, Sport::Rower)
}

fn is_valid_positive_finite(value: f64) -> bool {
    value.is_finite() && value > 0.0
}

// ── dispatch + normalisation ────────────────────────────────────────────────

/// Parse rival file bytes using content detection first, then extension
/// hints (Studio `ReplayRivalFileParser.parse`).
pub fn parse_rival_file(data: &[u8], file_name: &str) -> Result<ParsedTrace, RivalParseError> {
    if data.len() > MAXIMUM_FILE_SIZE_BYTES {
        return Err(RivalParseError::FileTooLarge);
    }
    let last_component = last_path_component(file_name);
    let extension = last_component
        .rsplit_once('.')
        .map(|(_, ext)| ext.to_lowercase())
        .unwrap_or_default();

    // A recognized payload is stronger evidence than a filename hint.
    let raw = if fit::has_plausible_signature(data) {
        fit::parse(data, MAXIMUM_ACCEPTED_SAMPLES)?
    } else if tcx::looks_like_tcx(data) {
        tcx::parse(data, MAXIMUM_ACCEPTED_SAMPLES)?
    } else if extension == "fit" {
        fit::parse(data, MAXIMUM_ACCEPTED_SAMPLES)?
    } else if extension == "tcx" {
        tcx::parse(data, MAXIMUM_ACCEPTED_SAMPLES)?
    } else {
        csv::parse(data, MAXIMUM_ACCEPTED_SAMPLES)?
    };

    let strokes = finalize(raw)?;
    Ok(ParsedTrace {
        strokes,
        file_name: last_component,
    })
}

/// Last path component only (`/` and `\` separators); never a full path.
fn last_path_component(file_name: &str) -> String {
    let trimmed = file_name.trim();
    match trimmed.rsplit(['/', '\\']).next() {
        Some(component) if !component.is_empty() => component.to_string(),
        _ => trimmed.to_string(),
    }
}

/// Common normalisation (Studio `ReplayRivalFileParser.finalize`): keep
/// finite, non-negative samples; sort by time (ties: farthest distance, then
/// input order); keep one deterministic sample per timestamp; drop backward
/// distance; re-base time to the first sample; resolve pace from deltas when
/// missing and watts from pace.
fn finalize(mut raw: Vec<RawSample>) -> Result<Vec<Stroke>, RivalParseError> {
    let mut points: Vec<(usize, RawSample)> = Vec::with_capacity(raw.len());
    for (offset, sample) in raw.drain(..).enumerate() {
        if sample.t.is_finite() && sample.d.is_finite() && sample.d >= 0.0 && sample.t >= 0.0 {
            points.push((offset, sample));
        }
    }
    points.sort_by(|(offset_a, a), (offset_b, b)| {
        a.t.partial_cmp(&b.t)
            .unwrap_or(std::cmp::Ordering::Equal)
            // Farthest distance first so the dedupe below keeps it.
            .then_with(|| b.d.partial_cmp(&a.d).unwrap_or(std::cmp::Ordering::Equal))
            .then_with(|| offset_a.cmp(offset_b))
    });

    let mut cleaned: Vec<RawSample> = Vec::with_capacity(points.len());
    for (_, sample) in points {
        if let Some(last) = cleaned.last() {
            if sample.t == last.t {
                continue;
            }
            if sample.d < last.d {
                continue;
            }
        }
        cleaned.push(sample);
        if cleaned.len() > MAXIMUM_ACCEPTED_SAMPLES {
            return Err(RivalParseError::TooManySamples);
        }
    }

    if cleaned.is_empty() {
        return Err(RivalParseError::UnsupportedOrEmpty);
    }
    if cleaned.len() < 2 {
        return Err(RivalParseError::TooFewSamples);
    }

    let first_time = cleaned[0].t;
    let mut output: Vec<Stroke> = Vec::with_capacity(cleaned.len());
    for (index, sample) in cleaned.iter().enumerate() {
        let time = sample.t - first_time;
        if !time.is_finite() || time < 0.0 {
            continue;
        }
        let resolved_pace = if let Some(pace) = sample.pace {
            pace
        } else if index > 0 {
            let previous = &cleaned[index - 1];
            let distance_delta = sample.d - previous.d;
            let time_delta = sample.t - previous.t;
            if distance_delta > 0.0 && time_delta > 0.0 {
                time_delta / (distance_delta / 500.0)
            } else if let Some(last) = output.last() {
                last.pace
            } else {
                0.0
            }
        } else {
            0.0
        };
        let safe_pace = if resolved_pace.is_finite() && resolved_pace >= 0.0 {
            resolved_pace
        } else {
            0.0
        };
        let watts = match sample.watts {
            Some(value) if value.is_finite() && value > 0.0 => value,
            _ => {
                let derived = pace_to_watts_for_sport(Sport::Rower, safe_pace);
                if derived.is_finite() && derived > 0.0 {
                    derived
                } else {
                    0.0
                }
            }
        };
        let cadence = match sample.spm {
            Some(value) if value.is_finite() && value >= 0.0 => value,
            _ => 0.0,
        };
        let heart_rate = match sample.hr {
            Some(value) if value > 0.0 => Some(value),
            _ => None,
        };
        output.push(Stroke {
            t: time,
            d: sample.d,
            pace: safe_pace,
            spm: cadence,
            hr: heart_rate,
            watts,
            raw_t: None,
            raw_d: None,
        });
    }

    if output.len() > MAXIMUM_ACCEPTED_SAMPLES {
        return Err(RivalParseError::TooManySamples);
    }
    if output.is_empty() {
        return Err(RivalParseError::UnsupportedOrEmpty);
    }
    if output.len() < 2 {
        return Err(RivalParseError::TooFewSamples);
    }
    Ok(output)
}

// ── CSV ─────────────────────────────────────────────────────────────────────

/// RFC 4180-style CSV decoder with flexible column detection (web
/// `parseCsv` + Studio's streaming quoted rows and strict clock parsing).
mod csv {
    use super::{RawSample, RivalParseError};

    pub fn parse(data: &[u8], sample_limit: usize) -> Result<Vec<RawSample>, RivalParseError> {
        let text = decode_text(data);
        parse_text(&text, sample_limit)
    }

    /// UTF-8 with a Latin-1 fallback, so any byte stream decodes (web
    /// `file.text()`; Studio's UTF-8/ISO-Latin-1 ladder).
    fn decode_text(data: &[u8]) -> String {
        String::from_utf8_lossy(data).into_owned()
    }

    fn parse_text(text: &str, sample_limit: usize) -> Result<Vec<RawSample>, RivalParseError> {
        let mut did_read_header = false;
        let mut ti: isize = -1;
        let mut di: isize = -1;
        let mut pi: isize = -1;
        let mut hi: isize = -1;
        let mut si: isize = -1;
        let mut wi: isize = -1;
        let mut out: Vec<RawSample> = Vec::new();

        for row in Rows::new(text) {
            let columns = row.map_err(|()| RivalParseError::Malformed)?;
            if !did_read_header {
                let header: Vec<String> = columns.iter().map(|c| c.trim().to_lowercase()).collect();
                // Prefer exact/token matches so "timestamp" does not steal
                // elapsed time and "parameter" does not steal distance.
                ti = find_header(
                    &header,
                    &["elapsed", "elapsedtime", "seconds", "timer", "time"],
                    &["stamp", "date", "day", "zone", "clock", "local", "utc"],
                );
                di = find_header(
                    &header,
                    &[
                        "distance",
                        "distancemeters",
                        "distancemetres",
                        "dist",
                        "meters",
                        "metres",
                        "meter",
                        "metre",
                    ],
                    &["parameter", "diameter"],
                );
                pi = find_header(&header, &["avgpace", "averagepace", "pace"], &[]);
                hi = find_header(
                    &header,
                    &["heart_rate", "heartrate", "hr", "bpm", "heart"],
                    &[],
                );
                si = find_header(
                    &header,
                    &["stroke_rate", "strokerate", "stroke rate", "spm", "cadence"],
                    &[],
                );
                if si < 0 {
                    if let Some(rate_index) = header
                        .iter()
                        .enumerate()
                        .find(|(index, column)| {
                            *index as isize != hi
                                && header_tokens(column).contains(&"rate".to_string())
                                && !header_tokens(column).contains(&"heart".to_string())
                        })
                        .map(|(index, _)| index)
                    {
                        si = rate_index as isize;
                    }
                }
                wi = find_header(
                    &header,
                    &["powerwatts", "averagepower", "watts", "watt", "power"],
                    &[],
                );
                did_read_header = true;
                continue;
            }

            if ti < 0 || di < 0 {
                continue;
            }
            let t = parse_clock(columns.get(ti as usize).map(String::as_str));
            let d = number_or_nan(columns.get(di as usize).map(String::as_str));
            if !t.is_finite() || !d.is_finite() {
                continue;
            }
            let pace = if pi >= 0 {
                parse_clock(columns.get(pi as usize).map(String::as_str))
            } else {
                f64::NAN
            };
            out.push(RawSample {
                t,
                d,
                pace: if pace.is_finite() && pace > 0.0 {
                    Some(pace)
                } else {
                    None
                },
                spm: if si >= 0 {
                    optional_number(columns.get(si as usize).map(String::as_str))
                } else {
                    None
                },
                hr: if hi >= 0 {
                    optional_positive_int(columns.get(hi as usize).map(String::as_str))
                } else {
                    None
                },
                watts: if wi >= 0 {
                    optional_number(columns.get(wi as usize).map(String::as_str))
                } else {
                    None
                },
            });
            if out.len() > sample_limit {
                return Err(RivalParseError::TooManySamples);
            }
        }
        Ok(out)
    }

    /// Streaming row iterator: preserves quoted commas / newlines, unescapes
    /// doubled quotes, rejects unbalanced quotes, and skips blank rows.
    struct Rows<'a> {
        chars: Vec<char>,
        position: usize,
        _text: &'a str,
    }

    impl<'a> Rows<'a> {
        fn new(text: &'a str) -> Self {
            Rows {
                chars: text.chars().collect(),
                position: 0,
                _text: text,
            }
        }
    }

    impl Iterator for Rows<'_> {
        type Item = Result<Vec<String>, ()>;

        fn next(&mut self) -> Option<Self::Item> {
            loop {
                if self.position >= self.chars.len() {
                    return None;
                }
                let mut row: Vec<String> = Vec::new();
                let mut field = String::new();
                let mut is_quoted = false;
                let mut did_close_quote = false;

                while self.position < self.chars.len() {
                    let c = self.chars[self.position];
                    self.position += 1;
                    if is_quoted {
                        if c == '"' {
                            let next = self.chars.get(self.position);
                            if next == Some(&'"') {
                                field.push('"');
                                self.position += 1;
                            } else {
                                is_quoted = false;
                                did_close_quote = true;
                            }
                        } else {
                            field.push(c);
                        }
                    } else if c == '"' {
                        if did_close_quote || !field.trim().is_empty() {
                            return Some(Err(()));
                        }
                        field.clear();
                        is_quoted = true;
                    } else if c == ',' {
                        row.push(field.trim().to_string());
                        field.clear();
                    } else if c == '\r' {
                        if self.chars.get(self.position) == Some(&'\n') {
                            self.position += 1;
                        }
                        break;
                    } else if is_newline(c) {
                        break;
                    } else if did_close_quote {
                        if !c.is_whitespace() {
                            return Some(Err(()));
                        }
                    } else {
                        field.push(c);
                    }
                }
                if is_quoted {
                    return Some(Err(()));
                }
                row.push(field.trim().to_string());
                // Blank lines (and a lone trailing comma) are skipped.
                if row.len() > 1 || row.iter().any(|c| !c.is_empty()) {
                    return Some(Ok(row));
                }
            }
        }
    }

    fn is_newline(c: char) -> bool {
        matches!(
            c,
            '\n' | '\u{0b}' | '\u{0c}' | '\u{85}' | '\u{2028}' | '\u{2029}'
        )
    }

    /// Numeric seconds, or an "M:SS(.t)" / "H:MM:SS" clock string (web
    /// `parseClock` with Studio's strict field validation).
    fn parse_clock(value: Option<&str>) -> f64 {
        let Some(value) = value else { return f64::NAN };
        let text = value.trim();
        if text.is_empty() {
            return f64::NAN;
        }
        if text.contains(':') {
            let raw_parts: Vec<&str> = text.split(':').collect();
            if !(2..=3).contains(&raw_parts.len()) {
                return f64::NAN;
            }
            let mut parts: Vec<f64> = Vec::with_capacity(raw_parts.len());
            for raw in &raw_parts {
                let component = raw.trim();
                if component.is_empty() {
                    return f64::NAN;
                }
                match component.parse::<f64>() {
                    Ok(value) if value.is_finite() && value >= 0.0 => parts.push(value),
                    _ => return f64::NAN,
                }
            }
            // All components but the last must be integral; seconds < 60;
            // minutes < 60 in the H:MM:SS form.
            if parts[..parts.len() - 1].iter().any(|p| p.trunc() != *p) {
                return f64::NAN;
            }
            let seconds = parts[parts.len() - 1];
            if seconds >= 60.0 {
                return f64::NAN;
            }
            if parts.len() == 3 && parts[1] >= 60.0 {
                return f64::NAN;
            }
            let total = parts.iter().fold(0.0, |acc, p| acc * 60.0 + p);
            if total.is_finite() { total } else { f64::NAN }
        } else {
            match normalized_number_string(text).parse::<f64>() {
                Ok(value) if value.is_finite() => value,
                _ => f64::NAN,
            }
        }
    }

    fn number_or_nan(value: Option<&str>) -> f64 {
        match value.map(|v| normalized_number_string(v).parse::<f64>()) {
            Some(Ok(number)) if number.is_finite() => number,
            _ => f64::NAN,
        }
    }

    fn optional_number(value: Option<&str>) -> Option<f64> {
        match value.map(|v| normalized_number_string(v).parse::<f64>()) {
            Some(Ok(number)) if number.is_finite() => Some(number),
            _ => None,
        }
    }

    fn optional_positive_int(value: Option<&str>) -> Option<f64> {
        let number = optional_number(value)?;
        if number > 0.0 {
            Some(number.round())
        } else {
            None
        }
    }

    /// European decimal commas vs dots, thousand separators
    /// (Studio `normalizedNumberString`).
    fn normalized_number_string(value: &str) -> String {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return trimmed.to_string();
        }
        let has_dot = trimmed.contains('.');
        let has_comma = trimmed.contains(',');
        if has_dot && has_comma {
            // The later separator is the decimal one.
            let last_dot = trimmed.rfind('.').expect("has dot");
            let last_comma = trimmed.rfind(',').expect("has comma");
            if last_comma > last_dot {
                return trimmed.replace('.', "").replace(',', ".");
            }
            return trimmed.replace(',', "");
        }
        if has_comma {
            let parts: Vec<&str> = trimmed.split(',').collect();
            if parts.len() == 2
                && (1..=2).contains(&parts[1].len())
                && parts[0]
                    .chars()
                    .all(|c| c.is_ascii_digit() || c == '+' || c == '-')
                && parts[1].chars().all(|c| c.is_ascii_digit())
            {
                return format!("{}.{}", parts[0], parts[1]);
            }
            return trimmed.replace(',', "");
        }
        trimmed.to_string()
    }

    /// Exact match first, then token match; rejects columns that also carry
    /// disambiguating tokens (e.g. `timestamp` when looking for `time`).
    fn find_header(header: &[String], names: &[&str], rejected_tokens: &[&str]) -> isize {
        for name in names {
            if let Some(index) = header.iter().position(|column| column == name) {
                return index as isize;
            }
        }
        for name in names {
            if let Some(index) = header.iter().position(|column| {
                let tokens = header_tokens(column);
                (tokens.iter().any(|t| t == name) || column == name)
                    && !rejected_tokens
                        .iter()
                        .any(|token| tokens.iter().any(|t| t == token) || column.contains(token))
            }) {
                return index as isize;
            }
        }
        for name in names {
            if name.contains(' ') || name.contains('_') {
                let needle = name.replace('_', " ");
                if let Some(index) = header
                    .iter()
                    .position(|column| column.replace('_', " ").contains(&needle))
                {
                    return index as isize;
                }
            }
        }
        -1
    }

    fn header_tokens(column: &str) -> Vec<String> {
        column
            .split(|c: char| !c.is_alphanumeric())
            .filter(|t| !t.is_empty())
            .map(String::from)
            .collect()
    }
}

// ── TCX (Garmin Training Center XML) ────────────────────────────────────────

/// Namespace-insensitive TCX decoder implemented as a hand-rolled element
/// stack scanner (the core must stay dependency-free). Rejects DOCTYPE /
/// entity declarations, multiple roots and unbalanced tags; skips comments,
/// processing instructions and CDATA; treats timezone-less ISO timestamps as
/// UTC (web behaviour + Studio's structural hardening).
mod tcx {
    use crate::datetime::parse_instant_millis;

    use super::{RawSample, RivalParseError};

    pub fn looks_like_tcx(data: &[u8]) -> bool {
        let probe: String = data.iter().take(4096).map(|&b| b as char).collect();
        let probe = probe.trim_start_matches('\u{feff}').to_lowercase();
        // Local-name style matching also catches prefixed roots
        // (`<t:TrainingCenterDatabase>`).
        probe.contains("trainingcenterdatabase") || probe.contains("trackpoint")
    }

    pub fn parse(data: &[u8], sample_limit: usize) -> Result<Vec<RawSample>, RivalParseError> {
        let text = decode_text(data);
        if contains_ascii_case_insensitive(&text, "<!doctype") {
            return Err(RivalParseError::Malformed);
        }

        let mut scanner = Scanner::new(&text);
        let mut samples: Vec<RawSample> = Vec::new();
        let mut root_count = 0_usize;

        let mut pending: Option<PendingTrackpoint> = None;
        let mut current_element: Option<String> = None;
        let mut text_buffer = String::new();
        let mut inside_heart_rate = false;
        let mut element_stack: Vec<String> = Vec::new();

        loop {
            match scanner.next_token() {
                Some(Token::Open { name, self_closing }) => {
                    if element_stack.is_empty() {
                        root_count += 1;
                        if root_count > 1 {
                            return Err(RivalParseError::Malformed);
                        }
                    }
                    if name == "trackpoint" && !self_closing {
                        pending = Some(PendingTrackpoint::default());
                    }
                    if !self_closing {
                        element_stack.push(name.clone());
                        if name == "heartratebpm" {
                            inside_heart_rate = true;
                        }
                        if pending.is_some() {
                            current_element = Some(name);
                            text_buffer.clear();
                        }
                    } else if name == "trackpoint" {
                        pending = None;
                    }
                }
                Some(Token::Text(chunk)) => {
                    if pending.is_some() && current_element.is_some() {
                        text_buffer.push_str(&chunk);
                    }
                }
                Some(Token::Close { name }) => {
                    match element_stack.last() {
                        Some(top) if *top == name => {
                            element_stack.pop();
                        }
                        _ => return Err(RivalParseError::Malformed),
                    }
                    let value = text_buffer.trim().to_string();
                    if let Some(point) = pending.as_mut() {
                        match name.as_str() {
                            "time" => point.time = (!value.is_empty()).then_some(value),
                            "distancemeters" => point.distance = finite_double(&value),
                            "cadence" => point.cadence = finite_double(&value),
                            "watts" => point.watts = finite_double(&value),
                            "value" if inside_heart_rate => {
                                if let Some(hr) = finite_double(&value) {
                                    if hr > 0.0 {
                                        point.heart_rate = Some(hr.round());
                                    }
                                }
                            }
                            "heartratebpm" => {
                                inside_heart_rate = false;
                            }
                            "trackpoint" => {
                                let point = pending.take().unwrap_or_default();
                                if let (Some(time), Some(distance)) =
                                    (point.time.as_deref(), point.distance)
                                {
                                    if let Some(seconds) = seconds_since_epoch(time) {
                                        samples.push(RawSample {
                                            t: seconds,
                                            d: distance,
                                            pace: None,
                                            spm: point.cadence,
                                            hr: point.heart_rate,
                                            watts: point.watts,
                                        });
                                        if samples.len() > sample_limit {
                                            return Err(RivalParseError::TooManySamples);
                                        }
                                    }
                                }
                                current_element = None;
                                text_buffer.clear();
                                continue;
                            }
                            _ => {}
                        }
                        current_element = None;
                        text_buffer.clear();
                    } else if name == "heartratebpm" {
                        inside_heart_rate = false;
                    }
                }
                Some(Token::Ignored) => {}
                None => break,
            }
        }

        if !element_stack.is_empty() || root_count != 1 {
            return Err(RivalParseError::Malformed);
        }
        Ok(samples)
    }

    fn decode_text(data: &[u8]) -> String {
        String::from_utf8_lossy(data).into_owned()
    }

    /// ASCII-case-insensitive substring search without allocating a lowered
    /// copy of the whole (possibly 25 MiB) document.
    fn contains_ascii_case_insensitive(haystack: &str, needle: &str) -> bool {
        let haystack = haystack.as_bytes();
        let needle = needle.as_bytes();
        if needle.is_empty() || haystack.len() < needle.len() {
            return false;
        }
        haystack
            .windows(needle.len())
            .any(|window| window.eq_ignore_ascii_case(needle))
    }

    #[derive(Default)]
    struct PendingTrackpoint {
        time: Option<String>,
        distance: Option<f64>,
        cadence: Option<f64>,
        heart_rate: Option<f64>,
        watts: Option<f64>,
    }

    fn finite_double(text: &str) -> Option<f64> {
        match text.trim().parse::<f64>() {
            Ok(value) if value.is_finite() => Some(value),
            _ => None,
        }
    }

    /// ISO-8601 with offset / `Z`, plus the naive-as-UTC fallback (web
    /// `parseTcx`'s regex + `parseInstantMillis`).
    fn seconds_since_epoch(text: &str) -> Option<f64> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return None;
        }
        let mut ms = parse_instant_millis(trimmed);
        if !ms.is_finite() && looks_like_naive_iso8601(trimmed) {
            let utc = if trimmed.ends_with('Z') {
                trimmed.to_string()
            } else {
                format!("{trimmed}Z")
            };
            ms = parse_instant_millis(&utc);
        }
        if ms.is_finite() {
            Some(ms / 1000.0)
        } else {
            None
        }
    }

    fn looks_like_naive_iso8601(text: &str) -> bool {
        let bytes = text.as_bytes();
        if bytes.len() < 19 {
            return false;
        }
        bytes.iter().enumerate().all(|(index, byte)| match index {
            4 | 7 => *byte == b'-',
            10 => *byte == b'T',
            13 | 16 => *byte == b':',
            _ => byte.is_ascii_digit(),
        })
    }

    enum Token {
        Open { name: String, self_closing: bool },
        Close { name: String },
        Text(String),
        Ignored,
    }

    /// Minimal XML tokenizer over the decoded text: element names are
    /// lowercased local names (prefix stripped), attributes skipped with
    /// quote awareness, comments / PIs / CDATA skipped, and the five
    /// predefined entities decoded in text.
    struct Scanner<'a> {
        chars: Vec<char>,
        position: usize,
        _text: &'a str,
    }

    impl<'a> Scanner<'a> {
        fn new(text: &'a str) -> Self {
            let trimmed = text.trim_start_matches('\u{feff}');
            Scanner {
                chars: trimmed.chars().collect(),
                position: 0,
                _text: text,
            }
        }

        fn next_token(&mut self) -> Option<Token> {
            if self.position >= self.chars.len() {
                return None;
            }
            if self.chars[self.position] != '<' {
                // Text run until the next '<'.
                let mut raw = String::new();
                while self.position < self.chars.len() && self.chars[self.position] != '<' {
                    raw.push(self.chars[self.position]);
                    self.position += 1;
                }
                return Some(Token::Text(decode_entities(&raw)));
            }
            // Markup: skip '<'.
            self.position += 1;
            if self.position >= self.chars.len() {
                return Some(Token::Ignored);
            }
            match self.chars[self.position] {
                '/' => {
                    self.position += 1;
                    let name = self.read_name();
                    self.skip_past('>');
                    Some(Token::Close { name })
                }
                '!' => {
                    if self.starts_with("!--") {
                        self.skip_sequence("-->");
                    } else if self.starts_with("![CDATA[") {
                        self.skip_sequence("]]>");
                    } else {
                        // DOCTYPE and friends were already rejected; any
                        // other declaration ends at '>'.
                        self.skip_past('>');
                    }
                    Some(Token::Ignored)
                }
                '?' => {
                    self.skip_sequence("?>");
                    Some(Token::Ignored)
                }
                _ => {
                    let name = self.read_name();
                    // Scan attributes with quote awareness, detecting '/>'.
                    let mut self_closing = false;
                    loop {
                        // Skip whitespace.
                        while self.position < self.chars.len()
                            && self.chars[self.position].is_whitespace()
                        {
                            self.position += 1;
                        }
                        if self.position >= self.chars.len() {
                            break;
                        }
                        match self.chars[self.position] {
                            '>' => {
                                self.position += 1;
                                break;
                            }
                            '/' => {
                                self.position += 1;
                                if self.position < self.chars.len()
                                    && self.chars[self.position] == '>'
                                {
                                    self.position += 1;
                                    self_closing = true;
                                    break;
                                }
                            }
                            '"' | '\'' => {
                                let quote = self.chars[self.position];
                                self.position += 1;
                                while self.position < self.chars.len()
                                    && self.chars[self.position] != quote
                                {
                                    self.position += 1;
                                }
                                self.position += 1;
                            }
                            _ => {
                                self.position += 1;
                            }
                        }
                    }
                    Some(Token::Open { name, self_closing })
                }
            }
        }

        fn read_name(&mut self) -> String {
            let mut name = String::new();
            while self.position < self.chars.len() {
                let c = self.chars[self.position];
                if c.is_whitespace() || c == '>' || c == '/' {
                    break;
                }
                name.push(c);
                self.position += 1;
            }
            local_name(&name).to_lowercase()
        }

        fn skip_past(&mut self, terminator: char) {
            while self.position < self.chars.len() && self.chars[self.position] != terminator {
                self.position += 1;
            }
            self.position += 1;
        }

        fn skip_sequence(&mut self, sequence: &str) {
            let needle: Vec<char> = sequence.chars().collect();
            while self.position < self.chars.len() {
                if self.chars[self.position..].starts_with(&needle) {
                    self.position += needle.len();
                    return;
                }
                self.position += 1;
            }
        }

        fn starts_with(&self, prefix: &str) -> bool {
            let needle: Vec<char> = prefix.chars().collect();
            self.chars[self.position..].starts_with(&needle)
        }
    }

    fn local_name(qualified: &str) -> &str {
        qualified.rsplit(':').next().unwrap_or(qualified)
    }

    fn decode_entities(raw: &str) -> String {
        if !raw.contains('&') {
            return raw.to_string();
        }
        raw.replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&apos;", "'")
            .replace("&amp;", "&")
    }
}

// ── FIT (binary; minimal record-message decoder) ───────────────────────────

/// Hardened decoder for the FIT record-message subset used by rivals
/// (web `parseFit` + Studio's structural validation and compressed
/// timestamps).
mod fit {
    use super::{RawSample, RivalParseError};

    const FIT_RECORD: u16 = 20;

    struct FieldDefinition {
        number: i32,
        size: usize,
        base_type: u8,
    }

    struct MessageDefinition {
        global: u16,
        little_endian: bool,
        fields: Vec<FieldDefinition>,
    }

    #[derive(Default)]
    struct Record {
        timestamp: Option<u32>,
        distance: Option<u32>,
        speed: Option<u32>,
        power: Option<u16>,
        cadence: Option<u8>,
        heart_rate: Option<u8>,
    }

    /// Content sniffing requires a structurally plausible FIT header, not
    /// merely the four `.FIT` bytes, which can occur in ordinary CSV.
    pub fn has_plausible_signature(data: &[u8]) -> bool {
        if data.len() < 12 {
            return false;
        }
        let header_size = data[0] as usize;
        if !(12..=64).contains(&header_size) || header_size > data.len() {
            return false;
        }
        if &data[8..12] != b".FIT" {
            return false;
        }
        let data_size = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
        data_size <= data.len() - header_size
    }

    pub fn parse(data: &[u8], sample_limit: usize) -> Result<Vec<RawSample>, RivalParseError> {
        if data.len() < 14 {
            return Err(RivalParseError::Malformed);
        }
        let header_size = data[0] as usize;
        if header_size < 12 || header_size > data.len() {
            return Err(RivalParseError::Malformed);
        }
        if !has_plausible_signature(data) {
            return Err(RivalParseError::Malformed);
        }
        let data_size = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
        if data_size > data.len() - header_size {
            return Err(RivalParseError::Malformed);
        }
        let end = header_size + data_size;

        let mut definitions: [Option<MessageDefinition>; 16] = Default::default();
        let mut records: Vec<Record> = Vec::new();
        let mut last_timestamp: Option<u32> = None;
        let mut position = header_size;

        while position < end {
            let header = data[position];
            position += 1;

            if header & 0x80 != 0 {
                // Compressed-timestamp data message.
                let local = ((header >> 5) & 0x3) as usize;
                let time_offset = u32::from(header & 0x1F);
                let (Some(definition), Some(previous)) = (
                    definitions.get(local).and_then(Option::as_ref),
                    last_timestamp,
                ) else {
                    return Err(RivalParseError::Malformed);
                };
                let mut record = Record::default();
                let mut timestamp = (previous & !0x1F) | time_offset;
                if timestamp < previous {
                    timestamp = timestamp.wrapping_add(0x20);
                }
                record.timestamp = Some(timestamp);
                for field in &definition.fields {
                    if position + field.size > end {
                        return Err(RivalParseError::Malformed);
                    }
                    if field.number != 253 && definition.global == FIT_RECORD && field.number >= 0 {
                        if let Some(value) =
                            read_base(data, position, field.base_type, definition.little_endian)
                        {
                            assign_record_field(&mut record, field.number, value);
                        }
                    }
                    position += field.size;
                }
                last_timestamp = Some(timestamp);
                if definition.global == FIT_RECORD {
                    records.push(record);
                    if records.len() > sample_limit {
                        return Err(RivalParseError::TooManySamples);
                    }
                }
                continue;
            }

            let local = (header & 0x0f) as usize;
            if header & 0x40 != 0 {
                // Definition message.
                if position + 5 > end {
                    return Err(RivalParseError::Malformed);
                }
                let architecture = data[position + 1];
                if architecture > 1 {
                    return Err(RivalParseError::Malformed);
                }
                let little_endian = architecture == 0;
                let global = read_u16(data, position + 2, little_endian);
                let field_count = data[position + 4] as usize;
                position += 5;
                let mut fields: Vec<FieldDefinition> = Vec::with_capacity(field_count);
                for _ in 0..field_count {
                    if position + 3 > end {
                        return Err(RivalParseError::Malformed);
                    }
                    fields.push(FieldDefinition {
                        number: i32::from(data[position]),
                        size: data[position + 1] as usize,
                        base_type: data[position + 2],
                    });
                    position += 3;
                }
                if header & 0x20 != 0 {
                    // Developer field definitions: sizes count, values don't.
                    if position >= end {
                        return Err(RivalParseError::Malformed);
                    }
                    let developer_field_count = data[position] as usize;
                    position += 1;
                    for _ in 0..developer_field_count {
                        if position + 3 > end {
                            return Err(RivalParseError::Malformed);
                        }
                        fields.push(FieldDefinition {
                            number: -1,
                            size: data[position + 1] as usize,
                            base_type: 0x0d,
                        });
                        position += 3;
                    }
                }
                definitions[local] = Some(MessageDefinition {
                    global,
                    little_endian,
                    fields,
                });
            } else {
                // Data message.
                let Some(definition) = definitions.get(local).and_then(Option::as_ref) else {
                    return Err(RivalParseError::Malformed);
                };
                let mut record = Record::default();
                for field in &definition.fields {
                    if position + field.size > end {
                        return Err(RivalParseError::Malformed);
                    }
                    if field.number >= 0 {
                        if let Some(value) =
                            read_base(data, position, field.base_type, definition.little_endian)
                        {
                            // A timestamp on any global message establishes
                            // the base for a later compressed-timestamp message.
                            if field.number == 253 {
                                assign_record_field(&mut record, 253, value);
                            } else if definition.global == FIT_RECORD {
                                assign_record_field(&mut record, field.number, value);
                            }
                        }
                    }
                    position += field.size;
                }
                if let Some(timestamp) = record.timestamp {
                    last_timestamp = Some(timestamp);
                }
                if definition.global == FIT_RECORD && record.timestamp.is_some() {
                    records.push(record);
                    if records.len() > sample_limit {
                        return Err(RivalParseError::TooManySamples);
                    }
                }
            }
        }

        let Some(first_timestamp) = records.iter().filter_map(|r| r.timestamp).min() else {
            return Ok(Vec::new());
        };

        let mut output: Vec<RawSample> = Vec::with_capacity(records.len());
        for record in &records {
            let Some(timestamp) = record.timestamp else {
                continue;
            };
            let speed_mps = record.speed.map(|s| f64::from(s) / 1000.0);
            let pace = speed_mps.filter(|s| *s > 0.0).map(|s| 500.0 / s);
            let distance = record.distance.map(|d| f64::from(d) / 100.0);
            let time = f64::from(timestamp.wrapping_sub(first_timestamp));
            if let Some(d) = distance {
                if time.is_finite() && d.is_finite() {
                    output.push(RawSample {
                        t: time,
                        d,
                        pace,
                        spm: record.cadence.map(f64::from),
                        hr: record.heart_rate.map(f64::from),
                        watts: record.power.map(f64::from),
                    });
                }
            }
        }
        Ok(output)
    }

    fn assign_record_field(record: &mut Record, number: i32, value: f64) {
        let fits_u32 = (0.0..=f64::from(u32::MAX)).contains(&value);
        match number {
            253 if fits_u32 => record.timestamp = Some(value as u32),
            5 if fits_u32 => record.distance = Some(value as u32),
            // `enhanced_speed` (73) only fills in when `speed` (6) has not.
            6 | 73 if record.speed.is_none() && fits_u32 => {
                record.speed = Some(value as u32);
            }
            7 if (0.0..=f64::from(u16::MAX)).contains(&value) => record.power = Some(value as u16),
            4 if (0.0..=f64::from(u8::MAX)).contains(&value) => record.cadence = Some(value as u8),
            3 if (0.0..=f64::from(u8::MAX)).contains(&value) => {
                record.heart_rate = Some(value as u8);
            }
            _ => {}
        }
    }

    fn read_u16(data: &[u8], offset: usize, little_endian: bool) -> u16 {
        let b0 = data[offset];
        let b1 = data[offset + 1];
        if little_endian {
            u16::from_le_bytes([b0, b1])
        } else {
            u16::from_be_bytes([b0, b1])
        }
    }

    /// Read the first element of a FIT field; `None` for invalid values.
    fn read_base(data: &[u8], offset: usize, base_type: u8, little_endian: bool) -> Option<f64> {
        match base_type & 0x0f {
            1 => {
                let value = data[offset] as i8;
                (value != 0x7f).then(|| f64::from(value))
            }
            0 | 2 | 10 | 13 => {
                let value = data[offset];
                (value != 0xff).then(|| f64::from(value))
            }
            3 => {
                let raw = [data[offset], data[offset + 1]];
                let value = if little_endian {
                    i16::from_le_bytes(raw)
                } else {
                    i16::from_be_bytes(raw)
                };
                (value != 0x7fff).then(|| f64::from(value))
            }
            4 | 11 => {
                let value = read_u16(data, offset, little_endian);
                (value != 0xffff).then(|| f64::from(value))
            }
            5 => {
                let raw = [
                    data[offset],
                    data[offset + 1],
                    data[offset + 2],
                    data[offset + 3],
                ];
                let value = if little_endian {
                    i32::from_le_bytes(raw)
                } else {
                    i32::from_be_bytes(raw)
                };
                (value != 0x7fff_ffff).then(|| f64::from(value))
            }
            6 | 12 => {
                let raw = [
                    data[offset],
                    data[offset + 1],
                    data[offset + 2],
                    data[offset + 3],
                ];
                let value = if little_endian {
                    u32::from_le_bytes(raw)
                } else {
                    u32::from_be_bytes(raw)
                };
                (value != 0xffff_ffff).then(|| f64::from(value))
            }
            8 => {
                let raw = [
                    data[offset],
                    data[offset + 1],
                    data[offset + 2],
                    data[offset + 3],
                ];
                let value = if little_endian {
                    f32::from_le_bytes(raw)
                } else {
                    f32::from_be_bytes(raw)
                };
                (!value.is_nan()).then(|| f64::from(value))
            }
            9 => {
                let mut raw = [0u8; 8];
                raw.copy_from_slice(&data[offset..offset + 8]);
                let value = if little_endian {
                    f64::from_le_bytes(raw)
                } else {
                    f64::from_be_bytes(raw)
                };
                (!value.is_nan()).then_some(value)
            }
            _ => None, // strings / 64-bit ints: not needed here
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_point(pace: f64) -> Vec<Stroke> {
        constant_pace_strokes(pace, 2000.0, Sport::Rower)
    }

    #[test]
    fn constant_pace_ghost_matches_the_web_shape() {
        let strokes = two_point(120.0);
        assert_eq!(strokes.len(), 2);
        assert_eq!(strokes[0].t, 0.0);
        assert_eq!(strokes[0].d, 0.0);
        assert!((strokes[1].t - 480.0).abs() < 1e-9);
        assert_eq!(strokes[1].d, 2000.0);
        assert_eq!(strokes[0].pace, 120.0);
        assert_eq!(strokes[1].pace, 120.0);
        assert!(strokes[0].watts > 0.0);
        assert_eq!(strokes[0].spm, 0.0);
    }

    #[test]
    fn constant_pace_rejects_degenerate_inputs() {
        assert!(two_point(0.0).is_empty());
        assert!(two_point(-10.0).is_empty());
        assert!(constant_pace_strokes(120.0, -100.0, Sport::Rower).is_empty());
        assert!(constant_pace_strokes(120.0, 0.0, Sport::Rower).is_empty());
    }

    #[test]
    fn constant_pace_scales_bike_watts() {
        let bike = constant_pace_strokes(120.0, 2000.0, Sport::Bike);
        let rower = constant_pace_strokes(120.0, 2000.0, Sport::Rower);
        assert!(bike[0].watts > 0.0);
        assert!(bike[0].watts < rower[0].watts);
        assert!((bike[0].watts - rower[0].watts / 8.0).abs() < 1e-9);
    }

    #[test]
    fn csv_parses_basic_columns() {
        let text = "time,distance,pace,stroke_rate,heart_rate\n0,0,2:00,28,140\n10,50,1:58,30,145\n20,100,1:56,32,150\n";
        let trace = parse_rival_file(text.as_bytes(), "workout.csv").unwrap();
        assert_eq!(trace.file_name, "workout.csv");
        assert_eq!(trace.strokes.len(), 3);
        assert_eq!(trace.strokes[0].pace, 120.0);
        assert_eq!(trace.strokes[1].hr, Some(145.0));
        assert_eq!(trace.strokes[1].spm, 30.0);
    }

    #[test]
    fn csv_derives_pace_from_distance_and_time() {
        let text = "time,distance,heart_rate\n0,0,140\n10,50,145\n20,100,150\n";
        let trace = parse_rival_file(text.as_bytes(), "no-pace.csv").unwrap();
        assert_eq!(trace.strokes.len(), 3);
        assert!((trace.strokes[1].pace - 100.0).abs() < 1e-9);
        assert!(trace.strokes[1].watts > 0.0);
    }

    #[test]
    fn csv_clock_formats() {
        let text = "time,distance\n0:00,0\n0:10,50\n0:20,100\n";
        let trace = parse_rival_file(text.as_bytes(), "clock.csv").unwrap();
        assert_eq!(trace.strokes.len(), 3);
        assert!((trace.strokes[1].t - 10.0).abs() < 1e-9);

        let text = "time,distance\n0:00:00,0\n0:00:10,50\n0:01:00,500\n";
        let trace = parse_rival_file(text.as_bytes(), "long.csv").unwrap();
        assert!((trace.strokes[1].t - 10.0).abs() < 1e-9);
        assert!((trace.strokes[2].t - 60.0).abs() < 1e-9);
    }

    #[test]
    fn csv_rejects_missing_columns_and_header_only() {
        let text = "pace,heart_rate\n2:00,140\n1:58,145\n";
        assert!(parse_rival_file(text.as_bytes(), "incomplete.csv").is_err());
        let text = "time,distance\n";
        assert!(parse_rival_file(text.as_bytes(), "empty.csv").is_err());
    }

    #[test]
    fn csv_handles_quoted_fields() {
        let text = "time,distance,label\n0,0,\"hello, world\"\n10,50,\"row\"\n";
        let trace = parse_rival_file(text.as_bytes(), "quoted.csv").unwrap();
        assert_eq!(trace.strokes.len(), 2);
        let text = "time,distance\n0,0,\"unbalanced\n";
        assert!(parse_rival_file(text.as_bytes(), "bad.csv").is_err());
    }

    #[test]
    fn tcx_parses_trackpoints_with_namespaces() {
        let text = r#"<?xml version="1.0" encoding="UTF-8"?>
<TrainingCenterDatabase xmlns="http://www.garmin.com/xmlschemas/TrainingCenterDatabase/v2">
  <Activities><Activity Sport="Biking"><Lap><Track>
    <Trackpoint>
      <Time>2024-01-01T12:00:00Z</Time>
      <DistanceMeters>0</DistanceMeters>
      <Cadence>28</Cadence>
      <HeartRateBpm><Value>140</Value></HeartRateBpm>
      <Watts>200</Watts>
    </Trackpoint>
    <Trackpoint>
      <Time>2024-01-01T12:00:10Z</Time>
      <DistanceMeters>50</DistanceMeters>
      <Cadence>30</Cadence>
      <HeartRateBpm><Value>145</Value></HeartRateBpm>
      <Watts>210</Watts>
    </Trackpoint>
  </Track></Lap></Activity></Activities>
</TrainingCenterDatabase>"#;
        let trace = parse_rival_file(text.as_bytes(), "workout.tcx").unwrap();
        assert_eq!(trace.strokes.len(), 2);
        assert_eq!(trace.strokes[0].t, 0.0);
        assert!((trace.strokes[1].t - 10.0).abs() < 1e-6);
        assert_eq!(trace.strokes[1].hr, Some(145.0));
        assert_eq!(trace.strokes[1].spm, 30.0);
        assert_eq!(trace.strokes[1].watts, 210.0);
    }

    #[test]
    fn tcx_treats_timezone_less_timestamps_as_utc() {
        let text = "<TrainingCenterDatabase><Trackpoint><Time>2024-06-01T08:00:00</Time><DistanceMeters>0</DistanceMeters></Trackpoint><Trackpoint><Time>2024-06-01T08:00:15</Time><DistanceMeters>75</DistanceMeters></Trackpoint></TrainingCenterDatabase>";
        let trace = parse_rival_file(text.as_bytes(), "naive.tcx").unwrap();
        assert_eq!(trace.strokes.len(), 2);
        assert_eq!(trace.strokes[0].t, 0.0);
        assert!((trace.strokes[1].t - 15.0).abs() < 1e-6);
    }

    #[test]
    fn tcx_rejects_doctype_and_broken_structure() {
        let text = "<!DOCTYPE TrainingCenterDatabase [<!ENTITY xxe SYSTEM \"file:///etc/passwd\">]><TrainingCenterDatabase><Trackpoint><Time>2024-01-01T00:00:00Z</Time><DistanceMeters>0</DistanceMeters></Trackpoint><Trackpoint><Time>2024-01-01T00:00:10Z</Time><DistanceMeters>5</DistanceMeters></Trackpoint></TrainingCenterDatabase>";
        assert_eq!(
            parse_rival_file(text.as_bytes(), "xxe.tcx"),
            Err(RivalParseError::Malformed)
        );
        let text = "<TrainingCenterDatabase><Trackpoint></Wrong></TrainingCenterDatabase>";
        assert!(parse_rival_file(text.as_bytes(), "bad.tcx").is_err());
        let text = "<a></a><b></b>";
        assert!(parse_rival_file(text.as_bytes(), "tworoots.tcx").is_err());
    }

    #[test]
    fn normalization_rebases_non_zero_origins() {
        let text = "time,distance\n100,0\n110,50\n120,100\n";
        let trace = parse_rival_file(text.as_bytes(), "origin.csv").unwrap();
        assert!((trace.strokes[0].t - 0.0).abs() < 1e-9);
        assert!((trace.strokes[1].t - 10.0).abs() < 1e-9);
        assert!((trace.strokes[2].t - 20.0).abs() < 1e-9);
    }

    #[test]
    fn normalization_drops_backward_distance_and_duplicate_timestamps() {
        let text = "time,distance\n0,0\n10,50\n10,40\n20,45\n30,90\n";
        let trace = parse_rival_file(text.as_bytes(), "back.csv").unwrap();
        let times: Vec<f64> = trace.strokes.iter().map(|s| s.t).collect();
        let dists: Vec<f64> = trace.strokes.iter().map(|s| s.d).collect();
        assert_eq!(times, vec![0.0, 10.0, 30.0]);
        assert_eq!(dists, vec![0.0, 50.0, 90.0]);
    }

    #[test]
    fn fit_parses_the_minimal_record_stream() {
        // From the golden fixture: header + definition + three records with
        // timestamps 0/10/20 s and distances 0/50/100 m.
        let base64 = "DhAACDYAAAAuRklUAABAAAAUAAT9BIYFBIYGAoQDAQIAQEIPAAAAAABHEIwASkIPAIgTAABHEJEAVEIPABAnAABHEJY=";
        let data = decode_base64(base64);
        let trace = parse_rival_file(&data, "workout.fit").unwrap();
        assert_eq!(trace.strokes.len(), 3);
        assert_eq!(trace.strokes[0].t, 0.0);
        assert!((trace.strokes[1].t - 10.0).abs() < 1e-9);
        assert!((trace.strokes[1].d - 50.0).abs() < 1e-6);
        assert!(trace.strokes[1].pace > 0.0);
        assert!(trace.strokes.iter().skip(1).all(|s| s.watts > 0.0));
        assert!(trace.strokes.iter().any(|s| s.hr.is_some()));
    }

    #[test]
    fn fit_rejects_truncated_and_malformed_payloads() {
        let truncated = decode_base64("DhAACDYAAAAuRklU");
        assert!(parse_rival_file(&truncated, "truncated.fit").is_err());
        assert!(parse_rival_file(&[0u8; 16], "bad.fit").is_err());
    }

    #[test]
    fn oversized_files_are_rejected_before_parsing() {
        let oversized = vec![b' '; MAXIMUM_FILE_SIZE_BYTES + 1];
        assert_eq!(
            parse_rival_file(&oversized, "huge.csv").err(),
            Some(RivalParseError::FileTooLarge)
        );
    }

    #[test]
    fn file_names_keep_only_the_last_component() {
        let text = "time,distance\n0,0\n10,50\n";
        let trace =
            parse_rival_file(text.as_bytes(), "/home/user/secret/path/row.fit.csv").unwrap();
        assert_eq!(trace.file_name, "row.fit.csv");
    }

    /// Minimal standard base64 decoder for fixture payloads (test-only).
    fn decode_base64(input: &str) -> Vec<u8> {
        const TABLE: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = Vec::new();
        let mut buffer = 0u32;
        let mut bits = 0u32;
        for c in input
            .bytes()
            .filter(|c| *c != b'=' && !c.is_ascii_whitespace())
        {
            let value = TABLE.iter().position(|t| *t == c).expect("valid base64") as u32;
            buffer = (buffer << 6) | value;
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                out.push(((buffer >> bits) & 0xff) as u8);
            }
        }
        out
    }
}
