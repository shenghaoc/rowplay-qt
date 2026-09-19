# Phase 9 — Packaging: design

## Shape

```
cargo build --release ─▶ platform skeleton ─▶ Qt deployment tool ─▶ self-containment check
                                                                          │
   .dmg / installer+zip / AppImage  ◀── checksum ◀── launch-check.py ◀────┘
```

Three scripts, one per platform, each a linear list of the steps above with
the "done" criteria in the header comment; one Python launch check shared by
all three; one workflow that runs the scripts on their native runners and,
on a tag, drafts the release. Nothing packaging-specific lives in the
crates except the launch probe and the Apple rpaths.

## Where the runtime surface comes from

`build.rs` already compiles every non-Qt runtime input into the binary:
`qml/` (rcc), `i18n/` (lrelease + rcc), the balsam-converted rig / athlete /
venue components and meshes (rcc), the venue textures (rcc), and the
validated asset metadata (JSON in `OUT_DIR`, `include_str!`). The packaging
surface is therefore exactly Qt's frameworks, plugins and QML modules. All
three deployment tools discover QML modules by scanning import statements,
and the shell's QML is not on disk in the package, so each tool is pointed
at the `qml/` source tree (`-qmldir`, `--qmldir`, `QML_SOURCES_PATHS`). The
generated `RowPlay.ReplayAssets` module (balsam output, not under `qml/`)
imports only `QtQuick3D`, which `qml/RowPlay/Replay/*.qml` already imports.

## macOS (`tools/package/macos.sh`)

1. `cargo build --release -p rowplay-app`. `build.rs::emit_apple_rpaths`
   adds `LC_RPATH` entries for `QT_INSTALL_LIBS` and
   `@executable_path/../Frameworks` (bins and the crate's test harness).
2. Skeleton: `Contents/MacOS/rowplay-qt`, `Contents/Info.plist` from
   `packaging/macos/Info.plist` with `@VERSION@` (cargo metadata) and
   `@MIN_MACOS@` (the Qt install's `QMAKE_MACOSX_DEPLOYMENT_TARGET`, read
   from `mkspecs/qconfig.pri`) substituted, `Resources/rowplay-qt.icns`,
   `PkgInfo`, `Resources/licenses/`.
3. `macdeployqt <app> -qmldir=qml -libpath=<QT_INSTALL_LIBS>
   -always-overwrite`, plus `-codesign=<id> -hardened-runtime -timestamp`
   when `ROWPLAY_MAC_SIGN_IDENTITY` is set (ad-hoc otherwise). It copies
   the frameworks, plugins and the whole `QtQuick` QML tree, rewrites the
   load commands and drops the absolute rpath (measured).
4. `PlugIns/sqldrivers` is removed: the ODBC / PostgreSQL / Mimer drivers
   link libraries that exist on no user's machine (they are the `otool`
   errors in the deploy log) and nothing in rowplay-qt uses QtSql.
5. Self-containment: every executable / `.dylib` in the bundle is scanned
   with `otool -l`; any reference to `QT_INSTALL_LIBS` fails the build.
6. `launch-check.py` on `Contents/MacOS/rowplay-qt` under cocoa (the only
   platform plugin `macdeployqt` deploys; a window flashes for ~2 s).
7. `hdiutil create -format UDZO` and `shasum -a 256`.

## Windows (`tools/package/windows.ps1`)

1. `cargo build --release -p rowplay-app` (MSVC).
2. Stage `rowplay-qt.exe`, `rowplay-qt.ico`, `LICENSE`, `licenses/`.
3. `windeployqt --release --compiler-runtime --qmldir qml` copies the DLLs,
   plugins, QML modules and `vc_redist.x64.exe` (the Rust and Qt binaries
   link the VC++ runtime dynamically).
4. `launch-check.py` on the staged exe (the `windows` platform).
5. `ISCC.exe` compiles `packaging/windows/rowplay-qt.iss` (`/DAppVersion`,
   `/DStageDir`, `/DOutDir`): fixed `AppId` GUID, per-user default with an
   all-users dialog, optional desktop icon, runs `vc_redist` quietly when it
   was staged, offers to launch. `Compress-Archive` makes the portable zip.
6. `Get-FileHash` sidecars.

## Linux (`tools/package/linux.sh`)

1. `linuxdeploy-x86_64.AppImage` (`1-alpha-20251107-1`) and
   `linuxdeploy-plugin-qt-x86_64.AppImage` (`1-alpha-20250213-1`) fetched
   into `tools/package/.cache/` (git-ignored) and verified against the
   SHA-256 measured on 2026-09-19; a mismatch aborts.
2. `cargo build --release -p rowplay-app`.
3. AppDir: `usr/bin/rowplay-qt`, the desktop entry
   (`packaging/linux/io.github.shenghaoc.rowplay.desktop`), the 512 px icon
   under `hicolor`, the AppStream metainfo (`@VERSION@` / `@DATE@`
   substituted), `usr/share/licenses/rowplay-qt/`.
4. `linuxdeploy --appdir AppDir --plugin qt --output appimage` with
   `APPIMAGE_EXTRACT_AND_RUN=1` (FUSE-less runners), `QML_SOURCES_PATHS=qml`,
   `EXTRA_PLATFORM_PLUGINS` = the two Wayland platform plugins and
   `EXTRA_QT_PLUGINS` = the three `wayland-*` plugin directories they
   dlopen, `VERSION` for the file name.
5. `launch-check.py` on the AppImage (extract-and-run) with the xcb
   platform; under `xvfb-run` with `QSG_RHI_BACKEND=opengl` and
   `LIBGL_ALWAYS_SOFTWARE=1` when there is no display (CI).
6. `sha256sum` sidecar.

## The launch probe and the launch check

`SettingsBackend.exit_after_frames` is read with plain `std::env::var`
(unlike the gate hooks, which `test_env` compiles out of release builds)
and exposed as the constant property `Settings.exitAfterFrames`.
`Main.qml` runs a `FrameAnimation` while it is positive and calls
`Qt.quit()` when `currentFrame` reaches it — an idle shell renders no
frames, so the driver is what makes "N frames" reachable. The gate's
member check picks the new property up automatically (it scans `qml/` for
`Settings.*` references).

`launch-check.py` keeps only the variables a process needs to find its home
directory and a window system (`HOME`, `DISPLAY`, `XDG_RUNTIME_DIR`,
`SystemRoot`, …), cuts `PATH` to the system directories, sets
`ROWPLAY_EXIT_AFTER_FRAMES`, `QT_FORCE_STDERR_LOGGING=1` (Windows routes
qWarning to OutputDebugString when stderr is a pipe) and a temporary
`ROWPLAY_DATA_DIR`, and runs the binary with a 60 s timeout. Failure = exit
≠ 0, timeout, or any of the loader / QML signatures on stderr (`Library not
loaded`, `error while loading shared libraries`, `is not installed`, `is
not a type`, `Could not find the Qt platform plugin`, `ReferenceError`,
`TypeError`, …). The timeout is not a nicety: a QML load failure leaves
`QApp::run()` blocking with no window and no exit (note #16), so "never
exits" is one of the two failure modes it must catch.

## Workflow (`.github/workflows/release.yml`)

`package` matrix: `ubuntu-24.04` / `macos-26` / `windows-2025`, each with the
same Qt install step as `ci.yml` (including the Windows aqt pin), the Linux
apt runtime for Xvfb + Mesa + `libdbus`, then the platform script and an
artifact upload of `dist/`. `release` (tags only, needs `package`) downloads
everything, concatenates the `.sha256` files into `SHA256SUMS` and runs
`gh release create --draft --generate-notes`. Triggers: `push` on `v*`,
`workflow_dispatch`, and `pull_request` filtered to the packaging inputs.

## Identifiers and provenance

| Thing | Value | Source of truth |
| --- | --- | --- |
| Version | `0.1.0` | `Cargo.toml` (`cargo metadata`) |
| Bundle / desktop / AppStream id | `io.github.shenghaoc.rowplay` | ADR 0012 |
| Display name / executable | `rowplay` / `rowplay-qt` | Info.plist, `.desktop`, `.iss` |
| Data directories | `com` / `rowplay` / `rowplay-qt` | `rowplay_platform::paths` (unchanged, see ADR 0012) |
| Icon | rowplay `static/icon-512.png` + `favicon.svg` @ `173c6fa` | `ASSET_PROVENANCE.md`, `ICON_EXPECTED` |
| Inno `AppId` | `{EAABF5B7-1D6F-40A2-A67F-B66FCFB1F775}` | `rowplay-qt.iss` (fixed forever) |
