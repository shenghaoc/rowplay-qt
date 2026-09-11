// SPDX-License-Identifier: GPL-3.0-or-later
//! Settings-screen view model: the language picker, the home-timezone picker
//! and unit mapping/validation.
//!
//! The language list mirrors the web's `SUPPORTED_LANGUAGES` / `LANGUAGES`
//! (`src/lib/i18n.ts`); the timezone groups are a verbatim port of the web's
//! curated `src/lib/timezoneOptions.ts` (group headers are locale message
//! ids resolved through `Tr.t` in QML, option labels are untranslated
//! `(UTC±HH:MM) City` strings exactly like the web).

use rowplay_core::models::DistanceUnit;

/// One of the six supported UI languages (web `SUPPORTED_LANGUAGES`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Language {
    /// English.
    #[default]
    En,
    /// Chinese (simplified).
    Zh,
    /// German.
    De,
    /// Spanish.
    Es,
    /// French.
    Fr,
    /// Japanese.
    Ja,
}

impl Language {
    /// The supported languages in the web's picker order
    /// (`en, zh, de, es, fr, ja`).
    #[must_use]
    pub const fn all() -> [Language; 6] {
        [
            Language::En,
            Language::Zh,
            Language::De,
            Language::Es,
            Language::Fr,
            Language::Ja,
        ]
    }

    /// Parses a locale code; unknown codes are rejected (the web validates
    /// with `isLanguage` and falls back to `en` at the call site).
    #[must_use]
    pub fn parse(code: &str) -> Option<Language> {
        match code {
            "en" => Some(Language::En),
            "zh" => Some(Language::Zh),
            "de" => Some(Language::De),
            "es" => Some(Language::Es),
            "fr" => Some(Language::Fr),
            "ja" => Some(Language::Ja),
            _ => None,
        }
    }

    /// The two-letter locale code (no region suffix, like the web).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Language::En => "en",
            Language::Zh => "zh",
            Language::De => "de",
            Language::Es => "es",
            Language::Fr => "fr",
            Language::Ja => "ja",
        }
    }

    /// The picker endonym (web `LANGUAGES` labels).
    #[must_use]
    pub const fn endonym(self) -> &'static str {
        match self {
            Language::En => "English",
            Language::Zh => "中文",
            Language::De => "Deutsch",
            Language::Es => "Español",
            Language::Fr => "Français",
            Language::Ja => "日本語",
        }
    }

    /// The stored language preference, validated with an English fallback
    /// (web `getStoredLanguage`).
    #[must_use]
    pub fn from_preference(stored: Option<&str>) -> Language {
        stored.and_then(Language::parse).unwrap_or(Language::En)
    }
}

/// A timezone picker group: a locale-message id for the header plus its
/// options (web `TIMEZONE_OPTIONS`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimezoneGroup {
    /// Locale message id for the group header
    /// (`settings.timezoneGroupAmericas` …).
    pub group_id: &'static str,
    /// The group's options, in web order.
    pub options: Vec<TimezoneOption>,
}

/// One curated timezone option (web `{ value, label }`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimezoneOption {
    /// IANA zone name (the persisted preference value).
    pub value: &'static str,
    /// Untranslated display label, `(UTC±HH:MM) City`.
    pub label: &'static str,
}

/// The curated home-timezone picker (verbatim port of the web's
/// `timezoneOptions.ts`; labels keep the web's Unicode minus sign).
#[must_use]
pub fn timezone_options() -> [TimezoneGroup; 3] {
    [
        TimezoneGroup {
            group_id: "settings.timezoneGroupAmericas",
            options: vec![
                tz("Pacific/Honolulu", "(UTC−10:00) Honolulu"),
                tz("America/Anchorage", "(UTC−09:00) Anchorage"),
                tz("America/Los_Angeles", "(UTC−08:00) Los Angeles"),
                tz("America/Vancouver", "(UTC−08:00) Vancouver"),
                tz("America/Denver", "(UTC−07:00) Denver"),
                tz("America/Phoenix", "(UTC−07:00) Phoenix"),
                tz("America/Chicago", "(UTC−06:00) Chicago"),
                tz("America/Mexico_City", "(UTC−06:00) Mexico City"),
                tz("America/New_York", "(UTC−05:00) New York"),
                tz("America/Toronto", "(UTC−05:00) Toronto"),
                tz("America/Halifax", "(UTC−04:00) Halifax"),
                tz("America/Sao_Paulo", "(UTC−03:00) São Paulo"),
                tz("America/Argentina/Buenos_Aires", "(UTC−03:00) Buenos Aires"),
                tz("America/Santiago", "(UTC−04:00) Santiago"),
            ],
        },
        TimezoneGroup {
            group_id: "settings.timezoneGroupEuropeAfrica",
            options: vec![
                tz("Atlantic/Reykjavik", "(UTC+00:00) Reykjavik"),
                tz("Europe/London", "(UTC+00:00) London"),
                tz("Europe/Dublin", "(UTC+00:00) Dublin"),
                tz("Europe/Paris", "(UTC+01:00) Paris"),
                tz("Europe/Berlin", "(UTC+01:00) Berlin"),
                tz("Europe/Amsterdam", "(UTC+01:00) Amsterdam"),
                tz("Europe/Madrid", "(UTC+01:00) Madrid"),
                tz("Europe/Rome", "(UTC+01:00) Rome"),
                tz("Europe/Stockholm", "(UTC+01:00) Stockholm"),
                tz("Europe/Warsaw", "(UTC+01:00) Warsaw"),
                tz("Europe/Athens", "(UTC+02:00) Athens"),
                tz("Europe/Helsinki", "(UTC+02:00) Helsinki"),
                tz("Europe/Istanbul", "(UTC+03:00) Istanbul"),
                tz("Africa/Cairo", "(UTC+02:00) Cairo"),
                tz("Africa/Johannesburg", "(UTC+02:00) Johannesburg"),
                tz("Africa/Lagos", "(UTC+01:00) Lagos"),
                tz("Africa/Nairobi", "(UTC+03:00) Nairobi"),
            ],
        },
        TimezoneGroup {
            group_id: "settings.timezoneGroupAsiaPacific",
            options: vec![
                tz("Asia/Dubai", "(UTC+04:00) Dubai"),
                tz("Asia/Karachi", "(UTC+05:00) Karachi"),
                tz("Asia/Kolkata", "(UTC+05:30) Kolkata"),
                tz("Asia/Dhaka", "(UTC+06:00) Dhaka"),
                tz("Asia/Bangkok", "(UTC+07:00) Bangkok"),
                tz("Asia/Singapore", "(UTC+08:00) Singapore"),
                tz("Asia/Hong_Kong", "(UTC+08:00) Hong Kong"),
                tz("Asia/Shanghai", "(UTC+08:00) Shanghai"),
                tz("Asia/Tokyo", "(UTC+09:00) Tokyo"),
                tz("Asia/Seoul", "(UTC+09:00) Seoul"),
                tz("Australia/Perth", "(UTC+08:00) Perth"),
                tz("Australia/Adelaide", "(UTC+09:30) Adelaide"),
                tz("Australia/Sydney", "(UTC+10:00) Sydney"),
                tz("Australia/Melbourne", "(UTC+10:00) Melbourne"),
                tz("Pacific/Auckland", "(UTC+12:00) Auckland"),
                tz("Pacific/Fiji", "(UTC+12:00) Fiji"),
            ],
        },
    ]
}

const fn tz(value: &'static str, label: &'static str) -> TimezoneOption {
    TimezoneOption { value, label }
}

/// True when `zone` is one of the curated picker values (web
/// `TIMEZONE_VALUES`).
#[must_use]
pub fn is_curated_timezone(zone: &str) -> bool {
    timezone_options()
        .iter()
        .flat_map(|group| group.options.iter())
        .any(|option| option.value == zone)
}

/// The unit picker entries: index in the segmented control → preference.
/// The QML labels are the unit symbols `km` / `mi` (untranslated by design,
/// like the web's chart axis labels — see `docs/source-map.md`).
#[must_use]
pub const fn unit_options() -> [(DistanceUnit, &'static str); 2] {
    [(DistanceUnit::Metric, "km"), (DistanceUnit::Imperial, "mi")]
}

/// Validates a stored unit string, falling back to metric (the preference
/// default) when it is unknown.
#[must_use]
pub fn unit_from_preference(stored: Option<&str>) -> DistanceUnit {
    stored
        .and_then(DistanceUnit::parse)
        .unwrap_or(DistanceUnit::Metric)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_codes_round_trip_in_web_order() {
        let codes: Vec<&str> = Language::all().iter().map(|l| l.as_str()).collect();
        assert_eq!(codes, ["en", "zh", "de", "es", "fr", "ja"]);
        for language in Language::all() {
            assert_eq!(Language::parse(language.as_str()), Some(language));
        }
        assert_eq!(Language::parse("pt"), None);
        assert_eq!(Language::parse("en-US"), None);
    }

    #[test]
    fn language_preference_falls_back_to_english() {
        assert_eq!(Language::from_preference(Some("ja")), Language::Ja);
        assert_eq!(Language::from_preference(Some("klingon")), Language::En);
        assert_eq!(Language::from_preference(None), Language::En);
    }

    #[test]
    fn endonyms_match_the_web_picker() {
        let endonyms: Vec<&str> = Language::all().iter().map(|l| l.endonym()).collect();
        assert_eq!(
            endonyms,
            [
                "English",
                "中文",
                "Deutsch",
                "Español",
                "Français",
                "日本語"
            ]
        );
    }

    /// The port must stay verbatim against the web's curated list.
    #[test]
    fn timezone_groups_match_the_web_list() {
        let groups = timezone_options();
        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].group_id, "settings.timezoneGroupAmericas");
        assert_eq!(groups[1].group_id, "settings.timezoneGroupEuropeAfrica");
        assert_eq!(groups[2].group_id, "settings.timezoneGroupAsiaPacific");
        assert_eq!(groups[0].options.len(), 14);
        assert_eq!(groups[1].options.len(), 17);
        assert_eq!(groups[2].options.len(), 16);
        assert_eq!(groups[0].options[0].value, "Pacific/Honolulu");
        assert_eq!(groups[0].options[0].label, "(UTC−10:00) Honolulu");
        assert_eq!(groups[1].options[0].value, "Atlantic/Reykjavik");
        assert_eq!(groups[2].options[15].value, "Pacific/Fiji");
    }

    #[test]
    fn curated_timezone_lookup_accepts_only_listed_zones() {
        assert!(is_curated_timezone("Europe/Berlin"));
        assert!(is_curated_timezone("America/Argentina/Buenos_Aires"));
        assert!(!is_curated_timezone("Mars/Olympus_Mons"));
        assert!(!is_curated_timezone(""));
    }

    #[test]
    fn every_curated_zone_resolves_through_chrono_tz() {
        for group in timezone_options() {
            for option in group.options {
                assert!(
                    option.value.parse::<chrono_tz::Tz>().is_ok(),
                    "{} is not a chrono-tz zone",
                    option.value
                );
            }
        }
    }

    #[test]
    fn unit_preference_round_trips_with_metric_fallback() {
        assert_eq!(
            unit_from_preference(Some("imperial")),
            DistanceUnit::Imperial
        );
        assert_eq!(unit_from_preference(Some("metric")), DistanceUnit::Metric);
        assert_eq!(unit_from_preference(Some("furlongs")), DistanceUnit::Metric);
        assert_eq!(unit_from_preference(None), DistanceUnit::Metric);
        assert_eq!(unit_options()[0].1, "km");
        assert_eq!(unit_options()[1].1, "mi");
    }
}
