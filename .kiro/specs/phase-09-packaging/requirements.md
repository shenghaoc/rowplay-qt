# Phase 9 — Packaging: requirements

Ship installable builds of the Phases 0–8 app on the three platforms the
project targets, produced by CI from a tag, without weakening any invariant
the earlier phases established. Decision record: ADR 0012.

## R0 — What Phase 9 is (and is not)

- R0.1 Packaging is tooling, not code: no hand-written C++ (ADR 0001), no
  new Rust dependency (ADR 0007), no runtime-downloaded asset (AGENTS.md
  P1). The two Rust changes it does make are a release-safe launch probe
  (R3) and macOS link flags (R2.4).
- R0.2 The packaged Qt is the pinned Qt: the same aqt 6.11.2 archives CI
  builds against. A package built against any other Qt is out of scope.
- R0.3 Packaging proves *start, load, render, exit* on each platform. It
  does not close the visual-verification gap on macOS and Windows (the
  gate's pixel assertions run on the Linux leg only, note #17); that gap is
  recorded, not hidden (R6).

## R1 — Artifacts

- R1.1 macOS: `rowplay-qt.app` (arm64) and a compressed `.dmg`; the bundle
  is self-contained (no Mach-O references the Qt install by path) and
  carries Info.plist, the `.icns`, and the licence texts.
- R1.2 Windows: an Inno Setup installer (`…-setup.exe`, per-user by
  default, all-users on request) and a portable `.zip`; both hold
  `rowplay-qt.exe`, the deployed Qt tree, the licence texts and the VC++
  runtime installer.
- R1.3 Linux: an x86_64 AppImage with the desktop entry, icon, AppStream
  metadata and licence texts; runs under X11 and Wayland.
- R1.4 Every artifact has a `.sha256` sidecar; a tag build also publishes a
  combined `SHA256SUMS`.
- R1.5 File names carry the Cargo package version and the platform/arch:
  `rowplay-qt-<version>-<os>-<arch>.<ext>`.

## R2 — Build inputs and provenance

- R2.1 The application icon is the web app's own (`static/icon-512.png`,
  `favicon.svg`, MIT, at the pinned commit), vendored under `assets/icon/`
  with a provenance row per file in `ASSET_PROVENANCE.md`.
- R2.2 The `.icns` / `.ico` are generated from it by a committed script and
  committed; all four files are SHA-256-pinned by `asset_hashes.rs`, and any
  unlisted file under `assets/icon/` fails that test.
- R2.3 Third-party packaging tools that are downloaded (`linuxdeploy`, its
  Qt plugin) are pinned by release tag *and* SHA-256, verified before use.
- R2.4 macOS binaries carry `LC_RPATH` entries for the Qt install and
  `@executable_path/../Frameworks`, emitted by `build.rs`, so neither tree
  builds nor the bundle depend on a `DYLD_*` variable (closes the workaround
  in note #10).
- R2.5 Version, identifiers and names come from one place each:
  `Cargo.toml` for the version, `io.github.shenghaoc.rowplay` for the
  bundle / desktop / AppStream id, `rowplay` for the display name,
  `rowplay-qt` for the executable.

## R3 — Launch verification

- R3.1 `ROWPLAY_EXIT_AFTER_FRAMES=N` makes the shell quit with status 0
  after N rendered frames, in every build (read with plain `env::var`, not
  the debug-only `test_env`), because the release bundle carries no gate
  hooks and needs a probe. It can only shorten a run; it exposes and alters
  no data. Documented next to `ROWPLAY_DATA_DIR` in AGENTS.md.
- R3.2 A shared check (`tools/package/launch-check.py`) starts the deployed
  binary from an environment stripped of `PATH`, `DYLD_*`, `LD_LIBRARY_PATH`,
  `QT_*` and `QML*`, with a temporary `ROWPLAY_DATA_DIR`, and fails on a
  non-zero exit, a timeout, or a loader / QML failure signature on stderr.
- R3.3 The check is proven to bite before it is trusted: fed a bundle with
  its platform plugin removed and one with `QtQuick3D.framework` removed, it
  must fail both (measured, recorded in the PR).
- R3.4 Each package script runs the check on its own output, on the runner
  that built it, before producing the distributable.

## R4 — Workflow and release

- R4.1 `.github/workflows/release.yml` packages all three platforms on
  every pull request that touches a packaging input, on manual dispatch,
  and on a `v*` tag.
- R4.2 A tag build attaches every artifact and `SHA256SUMS` to a **draft**
  GitHub release; publishing stays a human action.
- R4.3 The workflow needs no secret. Signing is ad-hoc (macOS) / absent
  (Windows) unless the author supplies an identity
  (`ROWPLAY_MAC_SIGN_IDENTITY`); notarisation is a documented follow-up.
- R4.4 The existing CI matrix keeps building the app on all three OSes; the
  macOS test step runs without `DYLD_FALLBACK_FRAMEWORK_PATH` so R2.4 stays
  exercised.

## R5 — Documentation (P1)

- R5.1 ADR 0012 records the route and the alternatives; the ADR index is
  complete (it had stopped at 0009 while 0010 and 0011 existed).
- R5.2 README: how to install each package (including the unsigned-app
  steps), how to build one locally, and a corrected Status section (it
  still said "Phase 5a is delivered in this PR").
- R5.3 AGENTS.md: the packaging commands, the new QA hook, and the rule
  that release artifacts come only from the workflow.
- R5.4 `docs/qt-bridges-notes.md` #10 records the rpath fix and #5 the
  version consequence; `tools/README.md` lists the scripts;
  `docs/roadmap.md` marks the phase.

## R6 — Known gaps (recorded, not closed)

- R6.1 macOS and Windows rendering is verified by launch only, not by the
  gate's visual assertions (Linux leg only, note #17). A human look at the
  packaged app on each OS is the remaining check before a public release.
- R6.2 macOS x86_64 and Linux aarch64 are not packaged (qtbridge's support
  statement, note #8).
- R6.3 The macOS bundle is ~215 MB because `macdeployqt` copies the whole
  `QtQuick` QML tree; pruning is a follow-up with a launch check per pruned
  module.
- R6.4 Flatpak is deferred (ADR 0012); the AppDir already has the Flathub
  shape.
- R6.5 No in-app version display (note #5).
