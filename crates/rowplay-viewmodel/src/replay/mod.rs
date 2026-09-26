// SPDX-License-Identifier: GPL-3.0-or-later
//! Replay presentation logic (Phase 5): the V3 asset contract reader
//! ([`glb`]), the material-role → theme-key palette resolver ([`materials`]),
//! the venue palette bridge for sky, ground and lanes ([`palette`]) and the
//! equipment anchor table from the asset README ([`anchors`]). Phase 6a adds
//! the baked-venue reader and contract validator ([`venue`]); Blender Phase 2
//! the authored rowing shell's gate against the V3 contract
//! ([`rowing_shell`]); Blender Phase 3 the authored rowing environment's
//! gate ([`environment`]).
//!
//! Everything here is Qt-free data and validation: the app crate's `Replay`
//! singleton adapts it to QML, and no hex colour or anchor coordinate is
//! written in QML (Phase 5a spec R3/R4/R5).

pub mod anchors;
pub mod athlete;
pub mod camera;
pub mod course;
pub mod environment;
pub mod equipment;
pub mod frame;
pub mod glb;
pub mod grip;
pub mod hud;
pub mod materials;
pub mod palette;
pub mod pose;
pub mod rowing_shell;
pub mod tier;
pub mod venue;
pub mod venue_runtime;
