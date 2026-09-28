# KDE Plasma integration — tasks

Ticked in the pull request that delivers them (`kde/plasma-integration`).

- [x] T1 Baseline on Fedora 44 Plasma Wayland, `main` unchanged (R1).
  - [x] T1.1 Environment recorded: Fedora 44, kernel 6.19.10, Wayland,
    Plasma, KWin 6.6.4, KF 6.25.0, host Qt 6.10.2, Mesa 26.2.3, Intel UHD
    630, the app's Qt 6.11.2.
  - [x] T1.2 Quick gate walk natively: passed, 32.8 s, 14 captures. Full
    walk with phase shots and close-ups: passed, 118.2 s, 70 captures.
    Healthy afterwards.
  - [x] T1.3 Baseline AppImage (ubuntu:24.04): 69,532,152 bytes, 2,319
    entries, no KDE, Kirigami or host Qt file.
- [x] T2 What Qt sees (R2).
  - [x] T2.1 Theme selection: `kde, generic`, then qtbase's `QKdeTheme`
    (source and native). The portal theme only when forced or sandboxed.
  - [x] T2.2 The palette: Breeze roles, Accent `#000000`, Light,
    NoPreference, Noto Sans 10 pt, icon theme `breeze`.
  - [x] T2.3 Plasma's accent reaches Qt through `[Colors:Selection]` as the
    highlight, not through `AccentColor`. Measured with a sandboxed
    `kdeglobals`; Breeze Dark reports Dark.
  - [ ] T2.4 The same through System Settings, live and in fresh
    processes. The first run recorded no change at all (kdeglobals was
    never written in its window), so it is repeated in T6.
- [x] T3 Identity (R3.1, R3.2).
  - [x] T3.1 `setDesktopFileName` in `rowplay-app`'s bootstrap; cxx-qt-lib
    as a direct, exact pin; qt-bridges-notes #23.
  - [x] T3.2 Unit test pinning the ID to `packaging/`.
  - [x] T3.3 KWin records `desktopFileName=io.github.shenghaoc.rowplay`
    (native Wayland). Under Xvfb, `WM_CLASS` is `rowplay-app, rowplay-qt`
    and the desktop-file properties are the ID.
- [x] T4 Accent (R4).
  - [x] T4.1 `Theme.resolveAccent`, with the highlight for an unset
    accent.
  - [x] T4.2 The gate's `gate accent` cases and the system palette checked
    in `qml_runtime_gate.rs`. Natively: `#000000 #3daee9 -> #3daee9`.
  - [x] T4.3 Contrast: NoPreference under Plasma, documented (ADR 0018).
- [x] T5 Icons (R5).
  - [x] T5.1 Qt's precedence, a theme file before the source, confirmed in
    `qquickiconimage.cpp` and natively with static buttons.
  - [x] T5.2 The Linux names limited to the standard actions. The Settings
    silhouette is gone: 294 dark pixels before, 111 after.
- [ ] T6 Native acceptance on Fedora Plasma Wayland (R7.1, R7.2).
- [ ] T7 AppImage rebuilt, compared and run natively (R7.3).
- [x] T8 Docs: ADR 0018; ADR 0015 decisions 5 and 6 and ADR 0013's Plasma
  note amended; `design-system.md`, `qt-bridges-notes.md`, `roadmap.md`,
  AGENTS.md, and `linux.sh`'s header.
