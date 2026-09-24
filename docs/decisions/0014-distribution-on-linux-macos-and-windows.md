# ADR 0014 — Distribution on Linux, macOS and Windows; Windows ships CI-verified only

Status: accepted (2026-09-24)

## Context

[ADR 0012](0012-packaging-route.md) chose the packaging route on all three
platforms and a draft release "with every artifact and a combined
`SHA256SUMS`". When Phase 9 landed (#35), a narrower distribution policy
went into the roadmap, the README, the Phase 9 spec and `release.yml`
instead. Under it the Linux AppImage was the only distributed artifact. The
macOS `.dmg` and the Windows installer and portable `.zip` were built and
launch-checked in CI but not attached to releases. Their rendering
verification, code signing and notarisation were closed as won't-do, because
the author shipped on Linux.

The author has now decided that rowplay-qt is distributed on Linux, macOS
and Windows. The author has no Windows machine.

What each platform has behind it on 2026-09-24:

- **Linux:** CI (build, tests, the QML runtime-error gate walk under Xvfb +
  Mesa with its visual assertions on the replay's 3D captures) and local
  runs by hand under Xvfb + Mesa, with the AppImage opened on a real
  machine before the first release (Phase 9, T9).
- **macOS:** CI (build, tests, the gate walk on the `offscreen` platform),
  the package's launch check, and a native check on the author's Mac
  (#66). The native check ran the gate walk in a real window, 3D replay
  included, read the chords, accent, contrast preference, dialog order and
  system font from Qt, and built and launch-checked the package.
- **Windows:** CI only (the app's build, clippy and tests, the gate walk on
  `offscreen` among them) and the package's launch check (#81). Nobody has
  looked at it.

## Decision

1. **All three platforms are distributed.** A release attaches:
   - the Linux AppImage;
   - the macOS disk image with `rowplay-qt.app`;
   - the Windows installer and portable `.zip`.

   Each keeps its `.sha256` sidecar, and one `SHA256SUMS` covers exactly the
   attached files. Releases stay drafts that the author publishes (ADR 0012),
   and every artifact still comes from `release.yml` only.
2. **Windows builds ship verified by CI only**, and the release notes say
   so. "CI-verified" means:
   - **covered:** for the tagged commit, `ci.yml`'s Windows leg builds the
     app and passes clippy and the app crate's tests, the QML runtime-error
     gate walk among them (on `offscreen`, which proves every screen loads
     and walks without a QML runtime error). `release.yml`'s Windows
     package, the deployed installer tree, starts from a clean environment,
     renders 30 frames and exits 0 (`tools/package/launch-check.py`).
     `ci.yml` doesn't run on tags and `release.yml` doesn't wait for it. So
     until the workflow change (5) gates the draft release on a passing
     `CI` run, the author tags only a commit whose `CI` run passed;
   - **not covered:** the Qt-free crates' tests (core, platform, viewmodel,
     fixtures), which `ci.yml` runs on Linux only. On Windows the platform
     layer's keychain backend, its guard against keyring's in-memory mock
     and its paths are compiled but never tested;
   - **also not covered:** anything a person would see or do. That includes
     rendering, fonts and text-size scaling, the accent and contrast theme,
     keyboard and mouse behaviour, the 3D replay on real GPUs, the
     installer's own screens and uninstall, and SmartScreen. The gate's
     visual assertions run on the Linux leg only and read the replay's 3D
     captures. #81 lists what a person with Windows should check; until
     then, reports from Windows users are the only verification.
3. **macOS is checked by a person on a Mac** before a release is published:
   the packaged app is opened and looked at. The gate walk can also be run
   in a real window there, where it captures every screen, 3D included, and
   `cargo test` with `QT_QPA_PLATFORM=cocoa QSG_RHI_BACKEND=metal` runs its
   visual assertions too (qt-bridges-notes #17).
4. **Signing is unchanged, and the gaps are stated.** ADR 0012's defaults
   stand:
   - macOS: ad-hoc signature, not notarised, so Gatekeeper refuses the first
     launch;
   - Windows: unsigned, so SmartScreen warns.

   The README says how to open each. Developer ID signing with
   notarisation, and Windows code signing, are open decisions for the author
   (below); none is required to distribute.
5. **The release workflow follows separately.** `release.yml`'s release job
   attaches only the AppImage today, and it doesn't check that the tagged
   commit passed CI. Two things change the workflow, on a path that runs the
   paid package matrix, so they land only with the author's approval:
   - attaching the macOS and Windows packages;
   - gating the draft release on the tagged commit's `CI` run.

   Until then, the policy is decided but a tagged release still carries the
   AppImage alone. The macOS and Windows packages are the tagged run's
   workflow artifacts, which need a GitHub sign-in to download and expire
   after 90 days, and the README says so.

## Open decisions for the author

- **macOS Developer ID and notarisation:** an Apple Developer Program
  membership. That gives a Developer ID Application certificate (exported
  with its key as a `.p12` for CI) and `notarytool` credentials (an App Store
  Connect API key, or an Apple ID with an app-specific password and the team
  ID), all stored as repository secrets. `macos.sh` already signs with
  `ROWPLAY_MAC_SIGN_IDENTITY` (hardened runtime, secure timestamp). It still
  needs a `notarytool submit --wait` and a `stapler staple` of the `.app` and
  the `.dmg`.
- **Windows code signing:** a code-signing certificate (an OV or EV
  certificate from a CA, or a cloud service such as Azure Trusted Signing).
  `windows.ps1` needs a `signtool sign` step for the executable and the
  installer, and CI needs the secrets. Until a certificate builds
  reputation, SmartScreen may still warn on a signed installer.

## Consequences

- The README, the roadmap's Phase 9 record, AGENTS.md, the Phase 9 spec and
  qt-bridges-notes #17 now say "three platforms", with Windows marked
  CI-verified only.
- A Windows regression that CI cannot see can reach users. The release notes
  say so, and #81 is the place for Windows reports.
- Gatekeeper and SmartScreen warnings are part of installing until signing
  is decided.
- The macOS "black viewport" of note #17 was the gate test's `offscreen`
  default, not Metal. In a real window the same host captures the replay,
  so macOS pixels can be checked locally.
