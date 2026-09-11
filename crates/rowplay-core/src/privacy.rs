// SPDX-License-Identifier: GPL-3.0-or-later
//! Privacy guards: the Concept2 share-link check and log redaction.
//!
//! [`is_publicly_shareable`] ports the web app's `src/lib/privacy.ts`.
//! [`redact`] ports the web app's `src/lib/server/logger.ts` and merges in the
//! extra patterns rowplay-studio added in `PrivacySafeLogger.swift` (cookie
//! headers, JSON credential keys, form credentials, large JSON arrays) plus
//! Studio's 16 KiB input bound. Every code path that logs user data must go
//! through [`redact`] or [`PrivacySafeLogger`].

use std::fmt::Display;
use std::sync::LazyLock;

use regex::Regex;

/// Concept2 logbook privacy level that makes a workout publicly visible.
const PUBLIC_PRIVACY: &str = "everyone";

/// Redaction marker substituted for sensitive content.
pub const REDACTED: &str = "[REDACTED]";

/// Inputs longer than this (in characters) are truncated before the regex pass
/// to keep redaction linear on hostile payloads.
pub const MAX_REDACT_INPUT_CHARS: usize = 16_384;
const TRUNCATED_MARKER: &str = " [TRUNCATED]";

/// Whether a workout may be exposed through a public share link.
///
/// Fail closed: a public link is allowed ONLY when the Concept2 `privacy`
/// value is exactly `everyone` (trimmed, case-insensitive). Narrower levels
/// (`logged_in`, `partners`, `private`) and absent or unrecognised values are
/// non-public, so rowplay never overrides the athlete's stated preference.
#[must_use]
pub fn is_publicly_shareable(privacy: Option<&str>) -> bool {
    privacy.is_some_and(|value| value.trim().eq_ignore_ascii_case(PUBLIC_PRIVACY))
}

struct RedactionRule {
    regex: Regex,
    replacement: &'static str,
}

static RULES: LazyLock<Vec<RedactionRule>> = LazyLock::new(|| {
    let specs: [(&str, &str); 11] = [
        // Concept2 API token: 32+ character hex string.
        (r"(?i)\b[a-f0-9]{32,}\b", REDACTED),
        // Sealed web cookies (web logger.ts).
        (r"(?i)rp_tok=[^;]+", REDACTED),
        (r"(?i)session=[^;]+", REDACTED),
        // Authorization: Bearer … header.
        (r"(?i)Authorization:\s*Bearer\s+\S+", REDACTED),
        // SESSION_SECRET env value (web logger.ts).
        (r"(?i)SESSION_SECRET[=:]\s*\S+", REDACTED),
        // Cookie headers (Studio).
        (r"(?i)(Cookie|Set-Cookie):\s*[^\n]+", "${1}: [REDACTED]"),
        // Credential values in JSON: keep the key, replace only the value.
        (
            r#"(?i)("(?:token|access_token|refresh_token|id_token|client_secret|password)"\s*:\s*")(?:\\.|[^"\\])*(")"#,
            "${1}[REDACTED]${2}",
        ),
        // Query/form credentials: token=…, access_token=… .
        (
            r"(?i)\b(token|access_token|refresh_token|id_token|client_secret|password)\s*=\s*[^\s&]+",
            "${1}=[REDACTED]",
        ),
        // Full workout payloads: JSON object or array over 100 units, one level of nesting.
        (r"\{(?:[^{}]|\{[^{}]*\}){100,}\}", REDACTED),
        (r"\[(?:[^\[\]]|\[[^\[\]]*\]){100,}\]", REDACTED),
        // Multi-line debug dumps of workout detail (web logger.ts).
        (
            r"(?i)workoutDetail:\s*\{[\s\S]*?\b(?:strokes|splits)\b[\s\S]*?\}",
            REDACTED,
        ),
    ];
    specs
        .into_iter()
        .map(|(pattern, replacement)| RedactionRule {
            regex: Regex::new(pattern).expect("redaction pattern compiles"),
            replacement,
        })
        .collect()
});

/// Redact sensitive content from a string.
///
/// Applies every known sensitive pattern, replacing matches with
/// [`REDACTED`]. Idempotent — safe to call on already-redacted strings. Input
/// beyond [`MAX_REDACT_INPUT_CHARS`] characters is truncated and marked.
#[must_use]
pub fn redact(input: &str) -> String {
    let mut result: String = if input.chars().count() > MAX_REDACT_INPUT_CHARS {
        let mut truncated: String = input.chars().take(MAX_REDACT_INPUT_CHARS).collect();
        truncated.push_str(TRUNCATED_MARKER);
        truncated
    } else {
        input.to_owned()
    };
    for rule in RULES.iter() {
        if let std::borrow::Cow::Owned(replaced) = rule.regex.replace_all(&result, rule.replacement)
        {
            result = replaced;
        }
    }
    result
}

/// Redact any displayable value (errors, numbers, …) via its `Display` form.
#[must_use]
pub fn redact_display(value: &dyn Display) -> String {
    redact(&value.to_string())
}

/// Join a message and arguments after redacting each of them (Studio
/// `formatPrivacySafeLogMessage`).
#[must_use]
pub fn format_privacy_safe_log_message(message: &str, args: &[&dyn Display]) -> String {
    let mut parts = Vec::with_capacity(args.len() + 1);
    parts.push(redact(message));
    parts.extend(args.iter().map(|arg| redact_display(*arg)));
    parts.join(" ")
}

/// Severity of a privacy-safe log line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogLevel {
    /// Something failed.
    Error,
    /// Something is degraded but recoverable.
    Warn,
}

impl LogLevel {
    /// Upper-case label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            LogLevel::Error => "ERROR",
            LogLevel::Warn => "WARN",
        }
    }
}

/// Destination for already-redacted log lines. The core crate performs no I/O;
/// the platform crate supplies the real sink.
pub trait LogSink {
    /// Receive one redacted line.
    fn write(&self, level: LogLevel, category: &str, message: &str);
}

/// Logger that redacts every message and argument before handing the line to
/// its sink (web `createLogger`, Studio `PrivacySafeLogger`).
pub struct PrivacySafeLogger<'a> {
    category: String,
    sink: &'a dyn LogSink,
}

impl<'a> PrivacySafeLogger<'a> {
    /// Create a logger for a category (`sync`, `token`, `cache`, …).
    pub fn new(category: impl Into<String>, sink: &'a dyn LogSink) -> Self {
        PrivacySafeLogger {
            category: category.into(),
            sink,
        }
    }

    /// Log an error with redacted arguments.
    pub fn error(&self, message: &str, args: &[&dyn Display]) {
        self.sink.write(
            LogLevel::Error,
            &self.category,
            &format_privacy_safe_log_message(message, args),
        );
    }

    /// Log a warning with redacted arguments.
    pub fn warn(&self, message: &str, args: &[&dyn Display]) {
        self.sink.write(
            LogLevel::Warn,
            &self.category,
            &format_privacy_safe_log_message(message, args),
        );
    }
}

/// In-memory sink for tests.
#[derive(Debug, Default)]
pub struct MemorySink {
    lines: std::sync::Mutex<Vec<(LogLevel, String, String)>>,
}

impl MemorySink {
    /// Lines received so far as `(level, category, message)`.
    #[must_use]
    pub fn lines(&self) -> Vec<(LogLevel, String, String)> {
        self.lines.lock().expect("sink lock").clone()
    }
}

impl LogSink for MemorySink {
    fn write(&self, level: LogLevel, category: &str, message: &str) {
        self.lines.lock().expect("sink lock").push((
            level,
            category.to_owned(),
            message.to_owned(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- web privacy.test.ts / Studio PrivacyRedactionTests -----------------

    #[test]
    fn only_everyone_is_shareable() {
        assert!(is_publicly_shareable(Some("everyone")));
        for level in ["logged_in", "partners", "private"] {
            assert!(!is_publicly_shareable(Some(level)));
        }
        assert!(!is_publicly_shareable(None));
        assert!(!is_publicly_shareable(Some("")));
        assert!(!is_publicly_shareable(Some("public")));
        assert!(!is_publicly_shareable(Some("everybody")));
        assert!(!is_publicly_shareable(Some("some_random_value")));
        assert!(is_publicly_shareable(Some(" everyone ")));
        assert!(is_publicly_shareable(Some("EVERYONE")));
        assert!(is_publicly_shareable(Some("Everyone")));
    }

    // --- web logger.test.ts -------------------------------------------------

    #[test]
    fn returns_value_unchanged_when_nothing_matches() {
        assert_eq!(redact("Workout sync completed"), "Workout sync completed");
        assert_eq!(
            redact("D1 query failed: no such table: workouts"),
            "D1 query failed: no such table: workouts"
        );
        assert_eq!(
            redact("Normal log message with no secrets"),
            "Normal log message with no secrets"
        );
    }

    #[test]
    fn redacts_web_patterns() {
        assert_eq!(
            redact("Token: abcdef0123456789abcdef0123456789"),
            format!("Token: {REDACTED}")
        );
        let cookie = redact("rp_tok=sealed-value-here; path=/");
        assert!(cookie.contains(REDACTED) && !cookie.contains("sealed-value-here"));
        assert!(redact("Authorization: Bearer token123abc").contains(REDACTED));
        assert!(redact("SESSION_SECRET=my-secret-key").contains(REDACTED));
        let payload = r#"{"workouts":[{"id":1,"date":"2026-01-01","sport":"rower","distance":2000,"time":480,"pace":120,"hasStrokeData":false,"strokes":[]}]}"#;
        let result = redact(payload);
        assert!(result.contains(REDACTED));
        assert!(!result.contains("strokes"));
        assert_eq!(redact_display(&42), "42");
    }

    // --- Studio PrivacySafeLoggerTests --------------------------------------

    #[test]
    fn redacts_hex_tokens() {
        let result = redact("Token is abcdef1234567890abcdef1234567890 here");
        assert!(result.contains(REDACTED) && !result.contains("abcdef1234567890abcdef1234567890"));
        assert_eq!(redact("abcdef1234567890abcdef1234567890abcdef"), REDACTED);
        assert_eq!(redact("abc123"), "abc123");
    }

    #[test]
    fn redacts_bearer_headers() {
        let result = redact("Authorization: Bearer abcdef1234567890abcdef1234567890");
        assert!(result.contains(REDACTED) && !result.contains("Bearer"));
        assert!(redact("authorization: bearer mytokenvalue").contains(REDACTED));
    }

    #[test]
    fn redacts_json_credential_values() {
        assert_eq!(
            redact(r#"{"token": "super-secret-value-12345"}"#),
            r#"{"token": "[REDACTED]"}"#
        );
        assert_eq!(
            redact(r#"{"access_token": "super-secret-value-12345"}"#),
            r#"{"access_token": "[REDACTED]"}"#
        );
        assert_eq!(
            redact(r#"{"refresh_token": "refresh-12345"}"#),
            r#"{"refresh_token": "[REDACTED]"}"#
        );
        assert_eq!(
            redact(r#"{"client_secret": "secret-abc"}"#),
            r#"{"client_secret": "[REDACTED]"}"#
        );
        assert_eq!(
            redact(r#"{"id_token": "eyJhbGciOiJSUzI1NiJ9"}"#),
            r#"{"id_token": "[REDACTED]"}"#
        );
        assert_eq!(
            redact(r#"{"password": "hunter2"}"#),
            r#"{"password": "[REDACTED]"}"#
        );
        let escaped = redact(r#"{"password": "abc\"def", "safe": "visible"}"#);
        assert_eq!(escaped, r#"{"password": "[REDACTED]", "safe": "visible"}"#);
    }

    #[test]
    fn redacts_large_json_blobs_but_not_small_ones() {
        let payload = "x".repeat(200);
        assert_eq!(redact(&format!("{{\"{payload}\": \"value\"}}")), REDACTED);
        let items: Vec<String> = (0..20)
            .map(|i| format!(r#"{{"id": {i}, "name": "workout-{i}"}}"#))
            .collect();
        let array = format!("[{}]", items.join(","));
        assert!(array.len() > 100);
        assert_eq!(redact(&array), REDACTED);
        assert_eq!(redact(r#"{"key": "value"}"#), r#"{"key": "value"}"#);
    }

    #[test]
    fn redacts_cookie_headers() {
        let cookie = redact("Cookie: session_id=abcdef1234567890abcdef1234567890");
        assert!(
            cookie.contains(REDACTED)
                && !cookie.contains("session_id=abcdef1234567890abcdef1234567890")
        );
        let set_cookie = redact("Set-Cookie: auth_token=supersecretvalue123; Path=/");
        assert!(set_cookie.contains(REDACTED) && !set_cookie.contains("supersecretvalue123"));
    }

    #[test]
    fn redacts_query_and_form_credentials() {
        let cases = [
            (
                "https://api.example.com/callback?token=abcdef1234567890&state=xyz",
                "abcdef1234567890",
            ),
            (
                "access_token=supersecrettokenvalue12345",
                "supersecrettokenvalue12345",
            ),
            ("refresh_token=refresh-abc12345", "refresh-abc12345"),
            ("client_secret=secret-value-xyz", "secret-value-xyz"),
            (
                "id_token=eyJhbGciOiJSUzI1NiJ9.payload",
                "eyJhbGciOiJSUzI1NiJ9.payload",
            ),
            ("password=hunter2", "hunter2"),
        ];
        for (input, secret) in cases {
            let result = redact(input);
            assert!(result.contains(REDACTED), "{input}");
            assert!(!result.contains(secret), "{input}");
        }
    }

    #[test]
    fn redaction_is_idempotent_and_bounded() {
        let once = redact("Token: abcdef1234567890abcdef1234567890");
        assert_eq!(redact(&once), once);
        assert_eq!(redact(REDACTED), REDACTED);
        let long = "z".repeat(16_385);
        assert_eq!(redact(&long), format!("{} [TRUNCATED]", "z".repeat(16_384)));
        let multi = redact(
            "Token abcdef1234567890abcdef1234567890 and Authorization: Bearer 1234567890abcdef1234567890abcdef",
        );
        assert!(!multi.contains("abcdef1234567890abcdef1234567890"));
        assert!(!multi.contains("1234567890abcdef1234567890abcdef"));
        assert!(multi.matches(REDACTED).count() >= 2);
    }

    #[test]
    fn logger_redacts_message_and_arguments() {
        let formatted = format_privacy_safe_log_message(
            "Sync failed: Authorization: Bearer abcdef1234567890abcdef1234567890",
            &[],
        );
        assert!(
            !formatted.contains("abcdef1234567890abcdef1234567890") && formatted.contains(REDACTED)
        );
        let formatted =
            format_privacy_safe_log_message("Token value:", &[&"abcdef1234567890abcdef1234567890"]);
        assert!(
            !formatted.contains("abcdef1234567890abcdef1234567890") && formatted.contains(REDACTED)
        );

        let sink = MemorySink::default();
        let logger = PrivacySafeLogger::new("test", &sink);
        logger.error("Token value:", &[&"abcdef1234567890abcdef1234567890"]);
        logger.warn(
            "Auth header:",
            &[&"Authorization: Bearer abcdef1234567890abcdef1234567890"],
        );
        logger.error("Sync failed", &[&"abcdef0123456789abcdef0123456789"]);
        let lines = sink.lines();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].0, LogLevel::Error);
        assert_eq!(lines[1].0, LogLevel::Warn);
        assert!(
            lines
                .iter()
                .all(|(_, category, msg)| category == "test" && !msg.contains("abcdef"))
        );
        assert_eq!(lines[2].2, format!("Sync failed {REDACTED}"));
    }
}
