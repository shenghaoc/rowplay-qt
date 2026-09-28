# KDE Plasma integration — requirements

The owner's request (2026-09-27): make rowplay-qt a good citizen of a KDE
Plasma desktop through Qt and freedesktop interfaces, without turning it
into a KDE application, and without giving up the AppImage's
self-containment. The decision record is ADR 0018. Probes began on Debian 13
(Plasma 6.3.6), whose machine was unstable. Fedora 44 KDE Plasma is the
native test host.

## R0 — Scope and invariants

- R0.1 No KDE Frameworks, Kirigami, KConfig, KI18n, KIO, tray icon, MPRIS or
  other KDE-specific dependency, unless it solves a demonstrated problem that
  Qt or freedesktop cannot.
- R0.2 No desktop detection (`XDG_CURRENT_DESKTOP`, a Breeze colour, …) in
  product code or tests. The rules are Qt's and freedesktop's.
- R0.3 Nothing in `rowplay-core`, `rowplay-platform` or `rowplay-viewmodel`.
  Desktop integration lives in `rowplay-app`'s bootstrap and the QML.
- R0.4 No C++, no `unsafe`, no new string. The AppImage bundles nothing
  from the host's KDE or Qt.
- R0.5 macOS, Windows and the generic Linux path (CI under Xvfb) behave as
  before, or the difference is explained.
- R0.6 Host KDE tools are never run with the app's Qt on the loader path,
  and host KDE QML modules are never loaded into the bundled Qt.

## R1 — Baseline on the native host

- R1.1 The host's environment is recorded (release, kernel, session, Plasma,
  KWin, Frameworks, host Qt, Mesa, GPU, the app's Qt).
- R1.2 The quick and full gate walks pass natively on Plasma Wayland on the
  unchanged tree, with system health recorded after each.
- R1.3 The unchanged AppImage is built as the release runner builds it. Its
  size, SHA-256 and inventory are recorded.

## R2 — What Qt sees

- R2.1 The platform theme the AppImage's Qt actually selects under Plasma
  is established from the source and by a run.
- R2.2 The palette roles, accent, colour scheme, contrast preference, font
  and icon theme Qt reports are recorded.
- R2.3 A change of Plasma's accent and scheme is followed to what Qt
  reports, live and in a fresh process.

## R3 — Identity

- R3.1 The window names the shipped desktop entry: the Wayland `app_id`,
  `_KDE_NET_WM_DESKTOP_FILE` and `_GTK_APPLICATION_ID` are
  `io.github.shenghaoc.rowplay`.
- R3.2 X11's `WM_CLASS` still matches the entry's `StartupWMClass`.
- R3.3 One task-manager entry with the app's icon; pinning and relaunching
  keep one identity.

## R4 — Palette and accent

- R4.1 A valid platform accent is used. An accent that is one of Qt's own
  defaults means "unset", and then the highlight is used, which is Qt's
  documented default, unless it is a default too. Otherwise the brand blue
  applies.
- R4.2 The rule is tested on fixed palettes on every platform, and on the
  palette each run happens under.
- R4.3 High contrast follows Qt's contrast preference only.

## R5 — Command icons

- R5.1 A standard command shows the icon theme's standard action icon where
  the theme has it, and the app's own SVG otherwise. Qt's precedence, with
  no machinery of our own.
- R5.2 Only freedesktop standard action names are used. Content (metric
  graphics, sport badges, the replay, charts) never takes theme icons.

## R6 — Services and menus

- R6.1 Only services the app uses are audited: the Secret Service works
  under Plasma. No portal, notification or KIO is added without a feature
  that needs it.
- R6.2 Global-menu support is recorded, not built, while the product has no
  Linux menu bar.

## R7 — Acceptance

- R7.1 Native Fedora Plasma Wayland: the launcher entry, the icon, the task
  manager and pinning, light and dark, a non-default accent, 100 % and
  150 % text, keyboard traversal and focus, dialogs, menus, the screens,
  the replay HUD and every command icon.
- R7.2 The full gate walk natively; CI on every platform.
- R7.3 The AppImage is rebuilt as in R1.3. Its size delta and inventory are
  compared, it is launch-checked, and it is run natively.
- R7.4 Every claim is labelled by evidence kind: source, automated, native
  Fedora Plasma, or historical Debian.
