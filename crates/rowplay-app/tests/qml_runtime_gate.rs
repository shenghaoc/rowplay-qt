// SPDX-License-Identifier: GPL-3.0-or-later
//! QML runtime-error gate (Phase 4, spec R8).
//!
//! Launches the app in gate mode (`ROWPLAY_SMOKE_GATE=1`): demo data, no
//! token, offscreen platform. The shell walks every screen, flips through all
//! six languages and exercises the preference slots on a timer, then exits 0.
//! The test fails when stderr contains a QML `TypeError`, `ReferenceError`,
//! `Binding loop`, `Unable to assign` or `is not defined` — the classes of
//! silent breakage QML bindings produce at runtime, which the compiler and
//! `qmllint` (no `.qmltypes` for Rust types, qt-bridges-notes #9) cannot see.
//!
//! Unlike the 3D smoke screenshot test this needs no GL: the shell is pure
//! Qt Quick 2D, so `offscreen` renders it and the gate runs in every
//! `cargo test -p rowplay-app` invocation, including CI on all three OSes.

use std::process::Command;

const FORBIDDEN_PATTERNS: [&str; 5] = [
    "TypeError",
    "ReferenceError",
    "Binding loop",
    "Unable to assign",
    "is not defined",
];

#[test]
fn shell_walk_produces_no_qml_runtime_errors() {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rowplay-app"));
    command
        .env("ROWPLAY_SMOKE_GATE", "1")
        // On Windows Qt routes logging to OutputDebugString when stderr is a
        // pipe, which would blind both the error scan and the walk's own
        // console.log probes; force stderr everywhere (no-op on Unix).
        .env("QT_FORCE_STDERR_LOGGING", "1")
        // The walk ends with an end-to-end sync against the deterministic
        // mock client: worker thread, mpsc events, cross-thread invoker,
        // cache writes and the completion path — no token, no network.
        .env("ROWPLAY_SYNC_MOCK", "1")
        // Keep a caller-provided platform (e.g. xcb under Xvfb); default to
        // offscreen, which is enough for the 2D shell.
        .env(
            "QT_QPA_PLATFORM",
            std::env::var_os("QT_QPA_PLATFORM").unwrap_or_else(|| "offscreen".into()),
        )
        .env("LANG", "C.UTF-8");
    let output = command.output().expect("launch rowplay-app in gate mode");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Scan both streams: console.log may land on either depending on the
    // platform and Qt version.
    let combined = format!("{stdout}\n{stderr}");
    for pattern in FORBIDDEN_PATTERNS {
        for line in combined.lines() {
            assert!(
                !line.contains(pattern),
                "QML runtime error ({pattern}) during the gate walk:\n{line}\n\n\
                 full stderr:\n{stderr}"
            );
        }
    }
    assert!(
        output.status.success(),
        "gate walk exited with {}\nstderr:\n{stderr}",
        output.status
    );
    // The gate logs its language probes; their presence proves the walk ran
    // and that translations loaded and retranslated live instead of falling
    // back to message ids.
    assert!(
        combined.contains("gate i18n en: Dashboard"),
        "translations did not load (qsTrId fell back to ids)\noutput:\n{combined}"
    );
    assert!(
        combined.contains("gate i18n zh:") && !combined.contains("gate i18n zh: nav.dashboard"),
        "live language switch to zh did not retranslate\noutput:\n{combined}"
    );
    // The mock sync must complete and land in the cache, not the demo data.
    assert!(
        combined.contains("gate sync: sync.done"),
        "end-to-end mock sync did not complete\noutput:\n{combined}"
    );
    assert!(
        combined.contains("library 17 cache"),
        "mock sync did not populate the cache with the 17 demo workouts\noutput:\n{combined}"
    );
}
