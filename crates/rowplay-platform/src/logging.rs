// SPDX-License-Identifier: GPL-3.0-or-later
//! Log sinks. Every line arriving here has already been redacted by
//! `rowplay_core::privacy`.

use rowplay_core::privacy::{LogLevel, LogSink, PrivacySafeLogger};

/// Writes redacted lines to standard error.
#[derive(Debug, Default, Clone, Copy)]
pub struct StderrSink;

impl LogSink for StderrSink {
    fn write(&self, level: LogLevel, category: &str, message: &str) {
        eprintln!("[{}] [{category}] {message}", level.as_str());
    }
}

static STDERR: StderrSink = StderrSink;

/// A privacy-safe logger for `category` that writes to standard error.
#[must_use]
pub fn logger(category: &str) -> PrivacySafeLogger<'static> {
    PrivacySafeLogger::new(category, &STDERR)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logger_can_be_constructed_and_used() {
        let log = logger("test");
        log.warn("smoke", &[&"abcdef1234567890abcdef1234567890"]);
        log.error("smoke", &[]);
    }
}
