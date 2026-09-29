# ADR 0018 — Plasma integration through Qt and freedesktop, with Fusion in the portable AppImage

Status: accepted (2026-09-28; acceptance evidence and the harness added
2026-09-30). Refines ADR 0015's Linux decisions 5 and 6. ADR 0013's note on
Plasma's accent is corrected here.

## Context

ADR 0015 gives Linux Qt's Fusion style with "the system palette", and it
says the AppImage's palette comes from the desktop portal. ADR 0013 says
Plasma's accent reaches the app because "KDE's platform theme" sets
`QPalette::Accent`. Neither was tried on a Plasma desktop. This record
covers the work that did try: probes on Debian 13 (Plasma 6.3.6), which
were abandoned because that machine was unstable, and then a Fedora 44 KDE
Plasma machine, which is now the native Plasma test host:

- Fedora 44 KDE Plasma Desktop Edition on Wayland, Mesa 26.2.3 on Intel
  UHD 630 (hardware GL), the app on Qt 6.11.2 from aqt as CI and the
  AppImage use. The findings below were first measured on Plasma and KWin
  6.6.4 with KDE Frameworks 6.25.0, kernel 6.19.10 and host Qt 6.10.2. The
  host was then updated, and the acceptance run (below) re-measured every one
  of them on **Plasma and KWin 6.7.5, KDE Frameworks 6.30.0, kernel 7.2.7,
  xdg-desktop-portal 1.22.1 / KDE 6.7.5 and host Qt 6.11.2** (now the same
  version as the bundled Qt, which changes nothing: the bundled Qt is still a
  separate build from Fedora's). The update changed none of them.

The evidence is labelled by kind: **source** (read from the Qt 6.11.2 tags),
**native** (run on that Fedora Plasma Wayland session), **automated** (tests
and CI) and **Debian** (the earlier host, used only where it matters).

What the AppImage's Qt sees under Plasma:

- **Platform theme** (source, then native with `qt.qpa.theme` logging):
  outside Flatpak and Snap, with no `QT_QPA_PLATFORMTHEME` set, Qt asks the
  platform plugin for theme names. On `XDG_CURRENT_DESKTOP=KDE` they are
  `kde, generic`. The AppImage carries one theme plugin, the desktop
  portal's, keyed `xdgdesktopportal`; `aqt`'s Qt adds `gtk3`, which
  linuxdeploy does not deploy. No plugin matches `kde`, so Qt creates its
  built-in `QKdeTheme`, which reads `kdeglobals`. Plasma's own theme plugin
  (plasma-integration) belongs to the host's Qt and is never loaded. The
  portal theme is used only when it is forced or inside a sandbox, and even
  then it takes only `color-scheme` and `contrast` from the portal and
  delegates the palette to that same `QKdeTheme`. It does not read the
  portal's `accent-color`.
- **Palette** (native): the Breeze colours for Window, WindowText, Base,
  Button and Highlight. Under Breeze Light that is `#eff0f1`, `#232629`,
  `#ffffff`, `#fcfcfc` and `#3daee9`, with scheme Light and Noto Sans
  10 pt. **Accent is `#000000`.** `QKdeTheme` starts from `QPalette()`,
  which before the application palette exists is Qt's black palette. There
  `qt_ensure_default_accent_color` marks the accent as set, to black. The
  theme then writes Highlight from `[Colors:Selection] BackgroundNormal`
  but never writes Accent, so Qt's documented default, the accent following
  the highlight, never applies. On `main` before this change,
  `Theme.qml` accepted opaque black as the system accent: under Plasma the
  scrubber's fill and the focus ring were black. The portal theme gives the
  same black.
- **Where Plasma's accent lands** (native, with a sandboxed
  `XDG_CONFIG_HOME` so the desktop was untouched): with only `[General]
  AccentColor` changed, Qt reports the same palette as before. With only
  `[Colors:Selection]` changed, the highlight follows it (`#f67400`).
  Plasma's accent reaches Qt's KDE theme through the highlight alone.
- **Contrast** (source and native): `QKdeTheme` has no
  `contrastPreference()`. xdg-desktop-portal-kde 6.6.4 supplies
  `color-scheme`, `reduced-motion` and `accent-color` but no `contrast`.
  Fedora's Plasma ships no high-contrast colour scheme. Qt reports
  `NoPreference`.
- **Icons** (source and native): the icon loader starts with the system
  theme `breeze`. Qt Quick's `IconImage` loads an icon-theme file for
  `icon.name` before it considers `icon.source`. The source only takes the
  place of the platform icon engine (SF Symbols, Segoe). So on Linux a
  button with both a name and a source already shows the theme's icon when
  the theme has it and the source otherwise. ADR 0015 says the opposite.
  Measured with one static button per name, the theme icon won exactly
  where Breeze had the name. The names the toolbar used included a category
  icon (`preferences-system`) and a MIME-type icon (`x-office-calendar`).
  Breeze draws those in full colour, and the style's tint turned the
  Settings button into a solid silhouette: 294 dark pixels in its 48 px
  square, against 111 for our glyph.
- **Identity** (source and native): the app never named its desktop entry.
  Qt then uses the executable's name as the Wayland `app_id` and as X11's
  `_KDE_NET_WM_DESKTOP_FILE` / `_GTK_APPLICATION_ID`. KWin recorded
  `desktopFileName=rowplay-qt` for the AppImage (`rowplay-app` under Cargo).
  No desktop entry has that name: the shipped one is
  `io.github.shenghaoc.rowplay.desktop`, which is also the AppStream
  component and the macOS bundle identifier.
- **Host KDE libraries and the bundled Qt** (Debian): running host KDE
  binaries with the app's Qt on the loader path failed on an undefined
  symbol, a LayerShellQt one among them. Loading the host's
  `org.kde.desktop` Quick Controls style into the bundled Qt partly
  instantiated it. It warned that the Kirigami platform plugin was missing,
  drew incompletely, and drew no progress bar. Those modules are built
  against the host's Qt, not the app's.
- **Services** (source and native): the app opens no URL, file dialog or
  notification. The one desktop service it uses on Linux is the Secret
  Service for the Concept2 token (`keyring`), which on Fedora Plasma is
  `ksecretd`. The opt-in round trip passed there. The application menu is a
  `Qt.labs.platform` `MenuBar` on macOS only; elsewhere it is an in-window
  `Menu`.

## Decision

1. **The portable AppImage keeps Fusion and the platform theme's palette.**
   `org.kde.desktop` is rejected for it. The bundled Qt 6.11.2 cannot use
   the host's KDE modules, which are built for another Qt: the mismatch was
   seen as undefined symbols and incomplete rendering. Copying them into
   the AppImage, or bundling a second KDE Frameworks stack to obtain Breeze
   controls, would give up the AppImage's self-containment for a look. A
   package built on a distribution's own Qt and KDE Frameworks could use
   `org.kde.desktop`, because there both come from the same build. That is
   a possible future package, not this one.
2. **The accent is read by Qt's palette rules, never by desktop.**
   `Theme.resolveAccent(accent, highlight)` uses the palette's accent
   unless it is one of Qt's own defaults: Fusion's `#308cc6`, or the
   `#000000` of the uninitialised palette a theme starts from. For either,
   it uses the highlight, which is Qt's documented default for an unset
   accent, unless the highlight is Fusion's default too. Otherwise the brand
   blue applies, as before. On macOS and Windows the platform sets the
   accent, so nothing changes there. Under Plasma the accent is the
   selection colour, Plasma's accent. The gate checks the rule on fixed
   palettes on every platform, and checks the palette it runs under against
   the same rule. No test names a desktop or a colour a desktop happens to
   use. The black accent is a Qt defect in `QKdeTheme`, recorded in
   `docs/qt-bridges-notes.md` for upstream.
3. **High contrast is what Qt reports.** Plasma reports no contrast
   preference to Qt 6.11.2, so the app's high-contrast variant engages
   under Plasma only through `ROWPLAY_FORCE_CONTRAST=high`. It is not
   guessed from the palette.
4. **Command icons: the theme's standard action icon, else ours.** On Linux
   `Glyphs.platformNames` gives only names from the freedesktop Icon Naming
   Specification's standard actions: `view-refresh`, `go-previous`,
   `system-search`, `view-sort-ascending`, `media-playback-start` and
   `media-playback-pause`. Every compliant theme draws those as monochrome
   action glyphs. The Settings, sidebar, menu, calendar and "more" glyphs
   have no standard action name, so they keep our SVG on every desktop.
   Every button keeps its SVG source as the fallback, so a desktop with no
   icon theme, as on CI under Xvfb, draws our glyphs throughout. No custom
   machinery: this is Qt's own precedence. The product's content (the PM5
   metric graphics, the sport badges, the replay, the charts) never takes
   theme icons.
5. **The desktop identity is named at start-up.** `rowplay-app` calls
   `QGuiApplication::setDesktopFileName("io.github.shenghaoc.rowplay")` on
   Linux, before any window exists:
   - The Wayland `app_id`, `_KDE_NET_WM_DESKTOP_FILE` and
     `_GTK_APPLICATION_ID` then name the shipped desktop entry, so a task
     manager or dock matches the window to its launcher, icon and pin.
   - X11's `WM_CLASS` is unchanged (`rowplay-qt` as the class, which the
     entry's `StartupWMClass` names).
   - qtbridge's `QApp` does not expose the setter, so `rowplay-app` depends
     directly on cxx-qt-lib, the exact release qtbridge 0.3.0 already locks.
     That makes the dependency explicit, not something reached through
     qtbridge. It adds no C++ and no new package to the lockfile.
   - The identity lives only in `rowplay-app`'s bootstrap, never in core,
     platform or view-model code.
   - A unit test pins the ID to the desktop entry, the AppStream metadata
     and `Info.plist`. macOS and Windows take their identity from the
     bundle and the executable and never read this value.
6. **Services stay Qt's and freedesktop's.** The Secret Service through
   `keyring` is the only desktop service the app needs. No portal call,
   notification or KIO is added for a feature the app does not have.
7. **Menus:** no global-menu work. Plasma's global menu reaches a Qt
   `MenuBar` over DBusMenu, and the app has no menu bar on Linux, only an
   in-window menu. Building one only for the global menu would add a menu
   hierarchy the product does not have. This stays open until the product
   has a menu bar on Linux.

**Not adopted, and why:**

- `org.kde.desktop` / qqc2-desktop-style in the AppImage: decision 1.
- Kirigami: KDE's component framework and navigation model, a second
  visual system and a KDE Frameworks dependency.
- KConfig: the app's preferences are its own JSON file
  (`FilePreferencesStore`); `kdeglobals` is already read by Qt's KDE theme.
- KI18n: the six web locales are the string source (`i18n/`, `Tr.t`).
- KIO: the app opens no URL or file; `QDesktopServices` and the portal
  would serve if it ever does.
- KStatusNotifierItem / a tray icon, MPRIS, KNotifications: the app has no
  background activity, media session or notification to show.
- Plasma widgets (plasmoids): the app is a window, not a panel applet.
- Loading the host's KDE QML modules, Frameworks or Breeze style into the
  bundled Qt, or copying them into the AppImage: they are built against
  another Qt (the undefined-symbol and incomplete-rendering findings in the
  context), and would give up self-containment for a look. Not tried again
  on Fedora, where the finding was already settled.
- Detecting Plasma, by `XDG_CURRENT_DESKTOP` or anything else, in product
  code or tests: the accent, contrast and icon rules are Qt's and
  freedesktop's, and hold on any desktop that implements them. The
  acceptance harness is a Plasma tool and may name Plasma; the app and its
  tests do not.

## Acceptance

The decisions above are checked by a repeatable harness, `tools/kde/acceptance.py`
(`tools/kde/README.md`), and not by a one-off manual walk. Its evidence directory
is local and git-ignored; what it measured on 2026-09-30, from a clean branch
worktree on post-#144 `main` (046c2e3), with every stage passing:

- **Native gates** (Wayland, hardware GL): the debug quick, release quick
  (#144's, no `BorrowError`) and full gates pass; the full walk takes 115.5 s
  with 70 captures, a 0.7 s replay entry and a 2.48 % shadow check. `main`'s
  own full walk takes the same 115.5 s.
- **Visual contract:** 40 of 70 captures exceed the generic noise bound, but
  every changed pixel is confined to a recorded region; **0 unexpected
  pixels**. The 30 replay captures change only in the scrubber's fill
  (`#000000` to the accent), the 9 toolbar screens only in the 16x16 Settings
  icon, and `detail-nostrokes` in the sidebar list's focus ring (a band, no
  interior). The other 30 are noise. Accent-vs-default: 6 of 14 captures
  change, only in the Replay button, a switch, the selected row and the focus
  ring. Native hardware captures carry delta-1 dithering the Xvfb bound was
  not measured on (two same-tree runs: 2D up to 450 px, 3D up to 76 px, never
  above delta 1), so the rules file holds a named noise profile; the global
  bounds are unchanged.
- **Appearance**, on the live desktop and then restored (kdeglobals'
  SHA-256, scheme, portal keys, font and what Qt reports all equal the
  snapshot): a loud accent (`plasma-apply-colorscheme --accent-color #f67400`)
  moves Qt's Highlight `#3daee9` to `#f89d4c` while Accent stays `#000000`, and
  `Theme.accentColor` and the focus ring follow; Breeze Dark reaches Qt
  (`colorScheme` Dark, portal `color-scheme` 1); the general font at 150 %
  (10 to 15 pt) reaches a fresh Qt process and the full gate passes.
- **The exact AppImage** launched through a desktop entry: KWin reports
  `desktopFileName=io.github.shenghaoc.rowplay` on a native Wayland window;
  one launch is one window, a second is a second window with the same
  association; the app's own Qt log shows platform theme `kde`, icon theme
  `breeze` and themed lookups of the standard command icons; an AT-SPI walk
  moves focus out of the date fields (the list resets, 17 to 5 matching, without
  aborting) and toggles Play and Pause; it closes cleanly with no core dump.
- **Generic Linux** (Xvfb, every desktop variable removed): no KDE theme is
  created (`generic`, icon theme `hicolor`), Fusion is the style, the palette
  falls back by the same rule to the brand blue, the 70 captures match `main`'s
  within capture-diff's own bounds, and X11's `_KDE_NET_WM_DESKTOP_FILE` and
  `_GTK_APPLICATION_ID` are the ID while `WM_CLASS` is unchanged.
- **Packages**, both built by `linux.sh` unchanged in `ubuntu:24.04`, each with
  its own target directory: post-#144 `main` 69,585,400 bytes (SHA-256
  `5d7e24af...`), the branch 69,593,592 bytes (`6ac11160...`), 2,319 files
  each, only `usr/bin/rowplay-qt` differing (+20,552 bytes; the AppImage grows
  8,192 because squashfs pads to blocks). Neither bundles a KDE Frameworks,
  Kirigami, Plasma, Breeze QML, KConfig, KI18n or KIO file, and no ELF file
  needs a KDE library; the only platform-theme plugin is the desktop portal's.
  The earlier "+4,096 bytes" was measured before #144 and is not comparable.

What running it found, beyond the decisions:

- **A locked screen starves the gate.** Nobody touches the machine during a
  run, so Plasma's screen locker engaged after five idle minutes, and a locked
  session stops frame callbacks to every window: random gates ran out every
  hold. The harness holds `org.freedesktop.ScreenSaver.Inhibit`, keeps the
  window frontmost with a KWin script, and fails a starved or locked run by name.
- **Large text turns the sidebar into a modal drawer**, at 150 % text a 1200 px
  window is the medium width class. The walk's date-range step assumed the
  sidebar beside the content and failed there; it now opens the drawer first.
  The walk had only ever run at the default text size.
- **Shortcuts fire only in an active window.** Xvfb has no window manager, so
  the shortcut contract is skipped there (and says so); under Wayland and
  offscreen a skip is a failure. Platform chords differ (Preferences is
  Ctrl+Shift+, under KDE; Ctrl+Q does not exist offscreen), and the gate reads
  them from the shortcuts.

Key presses cannot be injected into the exact packaged AppImage on this
session without a privileged or interactive channel (no synthesizer tool is
installed, `/dev/uinput` is root-only, the RemoteDesktop portal needs a consent
dialog each session), and none is added. The chords are covered where key
events are real, in the gate (`GateKeys.qml`, on the release code path, through
`qml_runtime_gate.rs`), and the exact AppImage is driven through AT-SPI focus
and actions. Pinning to the Task Manager is the user's panel and stays an
optional manual smoke.

## Consequences

- Under Plasma the accent-derived colours, the focus ring and the
  scrubber, follow the user's accent. On macOS, Windows and the generic
  Linux path, where CI runs under Xvfb with no theme, the accent resolves
  as before.
- On Plasma the toolbar mixes Breeze's standard action icons with the
  app's own glyphs for the commands the specification has no action name
  for. That is the price of theming the standard commands without guessing
  theme-specific names.
- A high-contrast Plasma setup gets no automatic high-contrast variant
  until Qt reports one.
- The AppImage's inventory is unchanged: no KDE, Kirigami or host Qt file
  is bundled. The one new dependency is a direct edge to a crate the app
  already linked.
- **Portal and desktop services need no code** (read from the source by
  the harness): the app opens no URL, file dialog or notification, has no tray
  icon or MPRIS service, and creates its only menu bar on macOS; the one service
  it uses is the Secret Service through `keyring` (ksecretd on Plasma), whose
  opt-in round trip passes there.
- **Contrast stays a platform limitation.** Qt reports `NoPreference` on
  Plasma 6.7.5 and the portal's `contrast` key is 0; nothing is inferred
  from the palette.
- **Packaging-host limitation** (native, local only): `tools/package/linux.sh`
  on a Fedora 44 host produces an AppImage that crashes before `main`.
  linuxdeploy's pinned tools rewrite `RUNPATH` on Fedora's RELR-packed
  system libraries (`libpcre2-8` crashed in `_init`), and `NO_STRIP=1` does
  not help. This is not a runtime defect of the app. Local package
  comparisons are built in an `ubuntu:24.04` container, the release
  runner's OS, and release artifacts come from `release.yml` as before
  (ADR 0012).
