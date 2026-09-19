# ADR 0012 — Packaging: macdeployqt bundle + DMG, windeployqt + Inno Setup, linuxdeploy AppImage; ad-hoc signing by default; Flatpak deferred

Status: accepted (2026-09-19)

## Context

Phases 0–8 produce one binary per platform that only runs next to a Qt 6.11
install (`qmake` on `PATH`, `DYLD_FALLBACK_FRAMEWORK_PATH` on macOS —
docs/qt-bridges-notes.md #10). A public portfolio project with a rigorous
parity audit and no installer is a strange artifact; Phase 9 ships one.
Everything the app needs at runtime is already inside the binary — QML,
translations, the balsam-converted packs and the venue textures are compiled
into `.rcc` blobs by `build.rs` — so the whole packaging surface is Qt's own
frameworks, plugins and QML modules, plus an icon and the platform manifests.

Constraints that shaped the choice:

- **No hand-written C++ and no new Rust dependencies** (ADR 0001, 0007). The
  packaging has to be tooling, not code.
- **The pinned Qt is the only Qt.** qtbridge 0.2.0 is pinned exactly and CI
  builds against aqt's Qt 6.11.2 archives on all three OSes (ci.yml). A
  package built against any other Qt build is untested.
- **Release builds carry no test hooks** (`backend::test_env` is compiled
  out), so the packaged binary cannot be driven through the runtime gate.
- **Signing identities are the author's**, not the repository's; the
  pipeline must work without them.
- The Linux CI leg is the only place the gate's visual assertions run
  (`grabToImage` is black on one macOS/Metal host, note #17); packaging must
  not claim more than it can check.

## Decision

One script per platform under `tools/package/`, one workflow
(`.github/workflows/release.yml`), one shared launch check.

| Platform | Route | Artifact |
| --- | --- | --- |
| macOS (arm64) | assemble `rowplay-qt.app` (Info.plist template, `.icns`, licences) → `macdeployqt -qmldir=qml` → otool scan → launch check → `hdiutil` UDZO | `rowplay-qt-<v>-macos-arm64.dmg` |
| Windows (x64) | stage `rowplay-qt.exe` + licences → `windeployqt --qmldir qml --compiler-runtime` → launch check → Inno Setup 6 (`packaging/windows/rowplay-qt.iss`) + `Compress-Archive` | `…-windows-x86_64-setup.exe`, `…-windows-x86_64.zip` |
| Linux (x86_64) | AppDir (desktop entry, icon, AppStream metainfo, licences) → `linuxdeploy --plugin qt --output appimage` with `QML_SOURCES_PATHS=qml` and the Wayland platform plugins → launch check under Xvfb | `…-linux-x86_64.AppImage` |

- **Qt's own deployment tools** (`macdeployqt`, `windeployqt`) and the
  de-facto standard for AppImages (`linuxdeploy` + `linuxdeploy-plugin-qt`)
  do the dependency walking. They read the same aqt Qt the binary was built
  against, so the package is the tested configuration. `linuxdeploy` and its
  plugin are fetched at pinned release tags and verified by SHA-256 in
  `linux.sh` before they run.
- **The QML is compiled into the binary**, so all three tools are pointed at
  `qml/` as the import source (`-qmldir`, `--qmldir`, `QML_SOURCES_PATHS`).
  The generated `RowPlay.ReplayAssets` module (build-time balsam output) only
  imports `QtQuick3D`, which `qml/` already imports, so nothing is missed.
- **Every package is launch-checked on its own runner**
  (`tools/package/launch-check.py`): the deployed binary runs from an
  environment stripped of `PATH`, `DYLD_*`, `LD_LIBRARY_PATH`, `QT_*` and
  `QML*`, with `ROWPLAY_EXIT_AFTER_FRAMES=30`, and must exit 0 with no
  loader / QML failure signature on stderr. `ROWPLAY_EXIT_AFTER_FRAMES` is
  read with plain `env::var` in every build — the one override that cannot
  alter or expose data — precisely so the *release* bundle has a probe. The
  check was fed two known-bad bundles before it was trusted (platform plugin
  removed → exit −6 with the signature; `QtQuick3D.framework` removed → QML
  load failure that never exits, caught by the timeout).
- **macOS `rpath`s come from `build.rs`**: `-Wl,-rpath,<QT_INSTALL_LIBS>`
  and `-Wl,-rpath,@executable_path/../Frameworks` on Apple targets, which
  retires the `DYLD_FALLBACK_FRAMEWORK_PATH` workaround for tree builds and
  gives the bundle its search path. `macdeployqt` drops the absolute entry
  (measured: the deployed binary carries only `@executable_path/../
  Frameworks`), and `macos.sh` scans every Mach-O in the bundle for the Qt
  install path so the build machine's Qt can never be a hidden input.
- **Signing is ad-hoc by default.** macOS: `macdeployqt`'s default ad-hoc
  signature; `ROWPLAY_MAC_SIGN_IDENTITY` switches to a Developer ID with
  hardened runtime and a secure timestamp (notarisation-ready; the
  `notarytool` submission is a follow-up once the author has an identity).
  Windows: unsigned (SmartScreen will warn). Both are documented in the
  README as the "open anyway" steps. The pipeline never needs a secret.
- **Releases are drafts.** A `v*` tag runs the three package jobs and
  `gh release create --draft` with every artifact and a combined
  `SHA256SUMS`; publishing is a human click. Pull requests that touch a
  packaging input run the package jobs without the release job, so the
  scripts are exercised before a tag ever is.
- **Identifiers.** Bundle id / desktop id / AppStream id:
  `io.github.shenghaoc.rowplay` (a domain the author controls). The
  `directories` triple in `rowplay_platform::paths` stays
  `com` / `rowplay` / `rowplay-qt`; changing it moves users' data
  directories, which is its own decision before 1.0, not a packaging
  side-effect. Bundle/display name `rowplay`, executable `rowplay-qt`.
- **Icon.** The web app's `static/icon-512.png` and `favicon.svg` (MIT,
  rowplay, commit `173c6fa`) vendored under `assets/icon/` with provenance;
  `tools/package/gen-icons.py` (Pillow) derives the committed `.icns` and
  `.ico`, all four SHA-256-pinned by `asset_hashes.rs` like the replay
  assets. Nothing is downloaded at build time (ADR 0002 / 0009 spirit).
- **Version** comes from `Cargo.toml` (`cargo metadata`) into Info.plist,
  the Inno script, the AppStream release entry and every file name;
  `QApp` has no version setter (note #5), so nothing in the running app
  reads it yet.

## Alternatives considered

- **Flatpak.** The KDE runtime has a `qt6.11` branch, so it is feasible, but
  a Flatpak builds against the runtime's Qt, not the pinned aqt archives,
  and publishing means a Flathub submission with its own review cadence.
  Deferred: the AppImage ships the tested Qt today; a Flatpak is a follow-up
  that should start from the same AppDir layout (desktop entry, metainfo and
  icon are already in the Flathub-required form).
- **cargo-bundle / cargo-packager / tauri-bundler.** Rust-side bundlers do
  not walk Qt's plugin and QML module graph; they would need the Qt tools
  underneath anyway.
- **Static Qt.** Would remove the whole dependency walk but requires a
  static Qt build, which aqt does not ship and which qtbridge has never
  been built against.
- **NSIS / WiX for Windows.** Inno Setup is preinstalled on the
  `windows-2025` runner image (6.7.1), needs no extra step, and its script
  is a readable text file; WiX is also present but heavier for a single
  directory tree.
- **Trimming the macdeployqt output.** The bundle is 215 MB because
  `macdeployqt` copies the whole `QtQuick` QML tree (every Controls style,
  Dialogs, Particles, VectorImage, XR…). Pruning is deferred to a follow-up
  with a launch check per pruned module; only the QtSql driver plugins are
  removed now, because the ODBC / PostgreSQL / Mimer drivers link libraries
  that exist on no user's machine and nothing in rowplay-qt uses QtSql.

## Consequences

- `cargo run -p rowplay-app` and `cargo test -p rowplay-app` on macOS no
  longer need `DYLD_FALLBACK_FRAMEWORK_PATH`; ci.yml's macOS test step and
  the README drop it (note #10 records the fix).
- Three new runtime inputs become review items: `packaging/**`,
  `tools/package/**`, `assets/icon/**` (pinned). A Qt bump must be
  re-packaged and launch-checked on all three OSes.
- The launch check proves *start, load, render frames, exit* — not pixels.
  macOS and Windows rendering is still unverified by the gate's visual
  assertions (Linux-only, note #17); the Phase 9 spec records this as a
  known gap, not a solved one.
- Only the architectures qtbridge lists are packaged (note #8): macOS
  arm64, Windows x64, Linux x86_64. macOS x86_64 and Linux aarch64 are
  follow-ups.
- The `.metainfo.xml` is CC0-1.0 (the AppStream specification admits only
  permissive metadata licences); every other new file is
  GPL-3.0-or-later. `LICENSES/CC0-1.0.txt` already exists for the Poly
  Haven maps.
