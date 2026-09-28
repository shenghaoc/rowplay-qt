// SPDX-License-Identifier: GPL-3.0-or-later
//! rowplay-qt desktop application entry point.
//!
//! Registers the `RowPlay` QML singletons (Library, Detail, Settings, Sync,
//! Live, Replay — thin adapters over `rowplay-viewmodel` and
//! `rowplay-platform`), mounts the QML module and the compiled translations,
//! and shows the app shell. The Phase 0 smoke window stays available for the
//! stack screenshot test (`ROWPLAY_SMOKE_SCREENSHOT` /
//! `ROWPLAY_SMOKE_EXIT_AFTER_FRAMES`).

#![forbid(unsafe_code)]

mod backend;
mod replay;
mod smoke;

use std::process::ExitCode;

use qtbridge::QApp;

/// The freedesktop application ID: the desktop entry's file name without
/// `.desktop`, the AppStream component ID and the macOS bundle identifier
/// (`packaging/`). ADR 0018. Read at run time on Linux only; the tests pin
/// it to `packaging/` on every platform.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
const APP_ID: &str = "io.github.shenghaoc.rowplay";

/// The `qml/` tree, compiled to a binary Qt resource by `build.rs`.
static RESOURCES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/rowplay.rcc"));
/// The `i18n/` catalogues, compiled by `build.rs` with `lrelease` to
/// `:/qt/qml/RowPlay/i18n/qml_<lang>.qm`, where `QQmlApplicationEngine`
/// picks them up when `Qt.uiLanguage` changes.
static I18N_RESOURCES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/rowplay_i18n.rcc"));
/// The balsam-converted replay packs (Phase 5a): the generated
/// `RowPlay.ReplayAssets` module with the rig and athlete components and
/// their meshes. See `build.rs` for why the scene loads through balsam
/// instead of `RuntimeLoader`.
static REPLAY_RESOURCES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/rowplay_replay.rcc"));
/// The venue surface textures (Phase 6b): the Poly Haven set derivatives and
/// the bake's procedural maps, under `/qt/qml/RowPlay/Environments/…`. Bound
/// to venue materials only at High and Ultra (the environments README tiers).
static ENVIRONMENT_RESOURCES: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/rowplay_environments.rcc"));

/// Names the desktop entry that launches this process (ADR 0018). Without it
/// Qt falls back to the executable's name: the Wayland `app_id` and X11's
/// `_KDE_NET_WM_DESKTOP_FILE` / `_GTK_APPLICATION_ID` read `rowplay-qt`
/// (`rowplay-app` under Cargo), which names no desktop entry, so a task
/// manager or dock has to guess which launcher, icon and pin a window
/// belongs to. Qt reads the name when it creates a window, so it is set
/// before any QML loads. Linux only: macOS takes the identity from the
/// bundle's `Info.plist` and Windows from the executable, and neither
/// platform reads this value.
fn set_desktop_identity() {
    #[cfg(target_os = "linux")]
    cxx_qt_lib::QGuiApplication::set_desktop_file_name(&cxx_qt_lib::QString::from(APP_ID));
}

fn main() -> ExitCode {
    let mut app = QApp::new();
    app.application_name("rowplay-qt");
    set_desktop_identity();
    // The registered data must outlive the application: a `static` guarantees that.
    assert!(
        qtbridge::qresource::register_bytes(RESOURCES),
        "failed to register the QML resource bundle"
    );
    assert!(
        qtbridge::qresource::register_bytes(I18N_RESOURCES),
        "failed to register the i18n resource bundle"
    );
    assert!(
        qtbridge::qresource::register_bytes(REPLAY_RESOURCES),
        "failed to register the replay asset bundle"
    );
    assert!(
        qtbridge::qresource::register_bytes(ENVIRONMENT_RESOURCES),
        "failed to register the environment texture bundle"
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
        .register::<backend::live::LiveBackend>()
        .register::<backend::replay::ReplayBackend>()
        .add_import_path("qrc:/qt/qml")
        .load_qml_from_file(root)
        .run();
    ExitCode::from(u8::try_from(code.clamp(0, 255)).unwrap_or(1))
}

#[cfg(test)]
mod tests {
    use super::APP_ID;

    fn packaging(path: &str) -> String {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../packaging/");
        std::fs::read_to_string(format!("{root}{path}"))
            .unwrap_or_else(|error| panic!("read packaging/{path}: {error}"))
    }

    /// The name Qt hands the window system must name the desktop entry the
    /// AppImage ships, the AppStream component and the macOS bundle.
    #[test]
    fn the_desktop_identity_matches_the_packaging() {
        let desktop = packaging(&format!("linux/{APP_ID}.desktop"));
        assert!(
            desktop.lines().any(|line| line == format!("Icon={APP_ID}")),
            "the desktop entry's icon is not {APP_ID}"
        );
        let metainfo = packaging(&format!("linux/{APP_ID}.metainfo.xml"));
        assert!(metainfo.contains(&format!("<id>{APP_ID}</id>")));
        assert!(metainfo.contains(&format!(
            "<launchable type=\"desktop-id\">{APP_ID}.desktop</launchable>"
        )));
        assert!(packaging("macos/Info.plist").contains(&format!("<string>{APP_ID}</string>")));
    }
}
