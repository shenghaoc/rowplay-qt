// SPDX-License-Identifier: GPL-3.0-or-later
//! `rowplay-core` — the pure domain layer of rowplay-qt.
//!
//! Everything here is testable without Qt: Concept2 domain models, formatting,
//! logbook date-time helpers, analytics, personal bests, Paul's Law predictions,
//! the workout list query engine, workout tagging and the deterministic demo
//! library. The modules are ports of the rowplay web app (`src/lib/*.ts`), with
//! rowplay-studio (`Sources/RowPlayCore`) as the second reference; the web app
//! is canonical whenever the two disagree (see `docs/source-map.md`).
//!
//! Invariants:
//! - no Qt types, no file or network I/O (parsers only consume byte slices/strings);
//! - parsers bound their input length before scanning;
//! - user data that is logged goes through [`privacy::redact`].

#![forbid(unsafe_code)]

pub mod analytics;
pub mod concept2;
pub mod datetime;
pub mod demo;
pub mod formatting;
pub mod models;
pub mod num;
pub mod pace_input;
pub mod performance_predictor;
pub mod personal_bests;
pub mod privacy;
pub mod replay;
pub mod workout_query;
pub mod workout_tag;

pub use models::{
    DistanceUnit, HeartRateDetail, LoggingMetadata, Split, SplitIntervalType, Sport, Stroke,
    WeightClass, Workout, WorkoutDetail, WorkoutTargets,
};
