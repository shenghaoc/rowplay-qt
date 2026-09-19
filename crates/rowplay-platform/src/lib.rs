// SPDX-License-Identifier: GPL-3.0-or-later
//! `rowplay-platform` — non-UI services for rowplay-qt.
//!
//! Every external boundary is a trait with a production implementation and an
//! in-memory mock that tests and demo mode use. Nothing here depends on Qt, and
//! nothing here depends on tokio: the HTTP client is blocking.
//!
//! | Boundary | Trait | Production | Test / demo |
//! | --- | --- | --- | --- |
//! | Concept2 token | [`token_store::TokenStore`] | [`token_store::KeyringTokenStore`] (OS keychain) | [`token_store::InMemoryTokenStore`] |
//! | Workout cache | [`workout_cache::WorkoutCache`] | [`workout_cache::SqliteWorkoutCache`] (`rusqlite`) | [`workout_cache::InMemoryWorkoutCache`], [`workout_cache::FailingWorkoutCache`] |
//! | Concept2 API | [`concept2::Concept2Client`] | [`concept2::Concept2HttpClient`] (`ureq` + rustls) | [`concept2::MockConcept2Client`] |
//! | Preferences | [`preferences::PreferencesStore`] | [`preferences::FilePreferencesStore`] (JSON) | [`preferences::InMemoryPreferencesStore`] |
//! | Sync | [`sync::WorkoutSyncCoordinator`] | synchronous, cancellable | [`concept2::MockConcept2Client`] + in-memory cache |
//! | Live poll | [`live::poll_recent`] | page-1 only, never touches `fully_synced` | scripted client + in-memory cache |
//! | Paths | — | [`paths`] (`directories`) | — |
//!
//! Privacy invariants carried over from rowplay-studio:
//! - tokens live only in the token store; [`token_store::SecretToken`] cannot be
//!   displayed, serialised or logged, and preferences have no token field;
//! - everything logged goes through `rowplay_core::privacy::redact`;
//! - cache failures propagate — the library never silently falls back to demo data;
//! - the Concept2 client only talks HTTPS (loopback excepted), only follows
//!   same-origin redirects, and never puts the token in a URL, error or log line.

#![forbid(unsafe_code)]

pub mod concept2;
pub mod library;
pub mod live;
pub mod logging;
pub mod paths;
pub mod preferences;
pub mod sync;
pub mod token_store;
pub mod workout_cache;
