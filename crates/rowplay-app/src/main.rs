// SPDX-License-Identifier: GPL-3.0-or-later
//! rowplay-qt desktop application entry point.
//!
//! Phase 0: registers the Rust backend objects with the QML engine, mounts the
//! QML module compiled into the binary and shows the stack smoke-test window.

#![forbid(unsafe_code)]

mod smoke;

use std::process::ExitCode;

use qtbridge::QApp;

/// The `qml/` tree, compiled to a binary Qt resource by `build.rs`.
static RESOURCES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/rowplay.rcc"));

fn main() -> ExitCode {
    let mut app = QApp::new();
    app.application_name("rowplay-qt");
    // The registered data must outlive the application: a `static` guarantees that.
    assert!(
        qtbridge::qresource::register_bytes(RESOURCES),
        "failed to register the QML resource bundle"
    );
    let code = app
        .register::<smoke::SmokeBackend>()
        .add_import_path("qrc:/qt/qml")
        .load_qml_from_file("qrc:/qt/qml/RowPlay/Main.qml")
        .run();
    ExitCode::from(u8::try_from(code.clamp(0, 255)).unwrap_or(1))
}
