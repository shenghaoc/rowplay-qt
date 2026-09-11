// SPDX-License-Identifier: GPL-3.0-or-later
//! `rowplay-platform` — non-UI services for rowplay-qt.
//!
//! Every external boundary is a trait with a production implementation (later
//! phases: `keyring`, `rusqlite`, an HTTP client) and an in-memory mock that
//! tests and demo mode use today. Nothing here depends on Qt.
//!
//! | Boundary | Trait | Production (Phase 3) | Mock |
//! | --- | --- | --- | --- |
//! | Concept2 token | [`token_store::TokenStore`] | OS keychain via `keyring` | [`token_store::InMemoryTokenStore`] |
//! | Workout cache | [`workout_cache::WorkoutCache`] | `rusqlite` | [`workout_cache::InMemoryWorkoutCache`] |
//! | Concept2 API | [`concept2::Concept2Client`] | HTTPS client | [`concept2::MockConcept2Client`] |
//! | Preferences | [`preferences::PreferencesStore`] | settings file | [`preferences::InMemoryPreferencesStore`] |
//!
//! Privacy invariants carried over from rowplay-studio:
//! - tokens live only in the token store; [`token_store::SecretToken`] cannot be
//!   displayed, serialised or logged, and preferences have no token field;
//! - everything logged goes through `rowplay_core::privacy::redact`;
//! - cache failures propagate — the library never silently falls back to demo data.

#![forbid(unsafe_code)]

pub mod concept2;
pub mod library;
pub mod logging;
pub mod preferences;
pub mod token_store;
pub mod workout_cache;
