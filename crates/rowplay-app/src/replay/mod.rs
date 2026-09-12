// SPDX-License-Identifier: GPL-3.0-or-later
//! The Qt side of the 3D replay (Phase 5).
//!
//! [`assets`] resolves where the vendored packs are loaded from and carries
//! the build-time contract metadata; `backend::replay` adapts it to the
//! `Replay` QML singleton. All contract logic lives Qt-free in
//! `rowplay_viewmodel::replay`.

pub mod assets;
