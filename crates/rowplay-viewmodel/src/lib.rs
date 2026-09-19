// SPDX-License-Identifier: GPL-3.0-or-later
//! `rowplay-viewmodel` — the Qt-free UI logic layer of rowplay-qt.
//!
//! Sits between the Qt shell (`rowplay-app`) and the domain layers: every
//! filtering, sorting, tiling, chart-series and display-string decision lives
//! here so it is testable with plain `cargo test`, and the qtbridge objects in
//! the app crate stay thin adapters (Phase 4 ground rule). All user-visible
//! numbers and dates are formatted in Rust through `rowplay-core::formatting`
//! and [`dates`]; QML never formats metrics.
//!
//! Sources: rowplay-studio `Views/*` (the views being ported) and the rowplay
//! web app (`src/lib/datetime.ts`, `src/lib/timezoneOptions.ts`,
//! `src/lib/i18n.ts`) which stays canonical; see `docs/source-map.md`.

#![forbid(unsafe_code)]

pub mod dashboard;
pub mod dates;
pub mod detail;
pub mod library;
pub mod live;
pub mod nav;
pub mod replay;
pub mod role;
pub mod settings;
pub mod strokes;
