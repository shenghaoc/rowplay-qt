// SPDX-License-Identifier: GPL-3.0-or-later
//! Replay presentation logic (Phase 5): the V3 asset contract reader
//! ([`glb`]), the material-role → theme-key palette resolver ([`materials`]),
//! the venue palette bridge for sky, ground and lanes ([`palette`]) and the
//! equipment anchor table from the asset README ([`anchors`]).
//!
//! Everything here is Qt-free data and validation: the app crate's `Replay`
//! singleton adapts it to QML, and no hex colour or anchor coordinate is
//! written in QML (Phase 5a spec R3/R4/R5).

pub mod anchors;
pub mod athlete;
pub mod camera;
pub mod course;
pub mod frame;
pub mod glb;
pub mod hud;
pub mod materials;
pub mod palette;
pub mod pose;
