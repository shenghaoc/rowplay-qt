// SPDX-License-Identifier: GPL-3.0-or-later
//! rowplay-qt desktop application entry point.
//!
//! Phase 4: registers the `RowPlay` QML singletons (Library, Detail, Settings,
//! Sync — thin adapters over `rowplay-viewmodel` and `rowplay-platform`),
//! mounts the QML module and the compiled translations, and shows the app
//! shell. The Phase 0 smoke window stays available for the stack screenshot
//! test (`ROWPLAY_SMOKE_SCREENSHOT` / `ROWPLAY_SMOKE_EXIT_AFTER_FRAMES`).

#![forbid(unsafe_code)]

mod backend;
mod smoke;

use std::process::ExitCode;

use qtbridge::QApp;

/// The `qml/` tree, compiled to a binary Qt resource by `build.rs`.
static RESOURCES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/rowplay.rcc"));
/// The `i18n/` catalogues, compiled by `build.rs` with `lrelease` to
/// `:/qt/qml/RowPlay/i18n/qml_<lang>.qm`, where `QQmlApplicationEngine`
/// picks them up when `Qt.uiLanguage` changes.
static I18N_RESOURCES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/rowplay_i18n.rcc"));

fn main() -> ExitCode {
    let mut app = QApp::new();
    app.application_name("rowplay-qt");
    // The registered data must outlive the application: a `static` guarantees that.
    assert!(
        qtbridge::qresource::register_bytes(RESOURCES),
        "failed to register the QML resource bundle"
    );
    assert!(
        qtbridge::qresource::register_bytes(I18N_RESOURCES),
        "failed to register the i18n resource bundle"
    );

    // The Phase 0 smoke scene keeps its own window so the stack screenshot
    // test exercises the same QML it always has.
    let smoke_scene = std::env::var_os("ROWPLAY_SMOKE_SCREENSHOT").is_some()
        || std::env::var_os("ROWPLAY_SMOKE_EXIT_AFTER_FRAMES").is_some();
    let root = if smoke_scene {
        "qrc:/qt/qml/RowPlay/SmokeMain.qml"
    } else {
        "qrc:/qt/qml/RowPlay/Main.qml"
    };

    let code = app
        .register::<smoke::SmokeBackend>()
        .register::<backend::library::LibraryBackend>()
        .register::<backend::detail::DetailBackend>()
        .register::<backend::settings::SettingsBackend>()
        .register::<backend::sync::SyncBackend>()
        .add_import_path("qrc:/qt/qml")
        .load_qml_from_file(root)
        .run();
    ExitCode::from(u8::try_from(code.clamp(0, 255)).unwrap_or(1))
}
