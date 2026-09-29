# KDE Plasma integration — design

One pull request, `kde/plasma-integration`, on `main`. ADR 0018 holds the
findings and the decisions. This file says where each one lives.

## Hosts and evidence

- Native: Fedora 44 KDE Plasma 6.6.4, Wayland, Intel UHD 630 on Mesa, and
  the app's Qt 6.11.2 from aqt, as in CI. The gate walks run on hardware GL
  (`QT_QPA_PLATFORM=wayland`, `QSG_RHI_BACKEND=opengl`, no
  `LIBGL_ALWAYS_SOFTWARE`).
- Generic Linux: Xvfb + `xcb`, as CI runs it. It is not Plasma evidence.
- Packages: `tools/package/linux.sh` unchanged, run in `ubuntu:24.04`. On a
  Fedora 44 host its output crashes (ADR 0018, "Consequences").
- Probes: small QML files run with the aqt Qt's own `qml` tool, with the
  loader path set to that Qt only. Qt's `qt.qpa.theme*` and
  `qt.gui.icon.loader` logging. A KWin script that prints the windows'
  `desktopFileName`. `xprop` under Xvfb. Evidence goes to the git-ignored
  `artifacts/kde/`.

## Identity (`crates/rowplay-app/src/main.rs`)

`set_desktop_identity()` calls cxx-qt-lib's static
`QGuiApplication::set_desktop_file_name(APP_ID)` on Linux, right after
`QApp::new()`. That is before any window, because Qt reads the name when a
window is created.
- cxx-qt-lib is a direct dependency, pinned to the release qtbridge 0.3.0
  locks (`=0.10.0`). One lockfile line changes, an edge; no package is added.
- A unit test pins `APP_ID` to the desktop entry, the AppStream `<id>` and
  `<launchable>`, and `Info.plist`.

## Accent (`qml/RowPlay/Theme.qml`)

`resolveAccent(accent, highlight)` is a pure function, and
`systemAccent` / `systemAccentAvailable` are built on it. `accentColor`
keeps its high-contrast and brand-blue branches. The Qt defaults it
recognises are Fusion's `#308cc6` and the uninitialised palette's
`#000000`. `Main.qml`'s gate logs five fixed cases and the system palette
(`gate accent`), and `qml_runtime_gate.rs`'s `assert_accent_rule` checks
both against the same rule.

## Icons (`qml/RowPlay/Glyphs.qml`)

Only the `other` (Linux) column of `platformNames` changes: the Settings,
sidebar, menu, calendar and "more" names are cleared, and the standard
action names stay. `iconSource` is unchanged, so every button keeps its SVG
fallback. `CommandButton` and the sort button are the only users.

## Acceptance harness (`tools/kde/`)

`acceptance.py` orchestrates the stages (README there lists them); `kdeacc/`
holds the parsers and engines, each unit-tested without a desktop. The visual
contract lives in `expected-visual-diff.json` (regions, ring bands, `must_change`,
a native-hardware noise profile), derived by `derive_rules.py`. Appearance and
identity are transactions (`kdeacc/plasma.py`). The gate's keyboard contract is
in `Main.qml` (`gateKeyContractShell`, `gateKeyContractShortcuts`,
`gateKeyContractReplay`, `gateQuit`) with `GateKeys.qml`'s helper, asserted by
`qml_runtime_gate.rs`.

## Not built

Global menu, portal calls, notifications, tray, MPRIS, KIO, KConfig, KI18n,
Kirigami, and `org.kde.desktop` in the AppImage: ADR 0018, "Not adopted".
