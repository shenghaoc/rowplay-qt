## Scope

<!-- Which phase or fix; link the .kiro spec. -->

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets -- -D warnings` (add `--workspace` where Qt is installed)
- [ ] `cargo test --workspace` (or the Qt-free default members, stating which)
- [ ] `git diff --check`
- [ ] App builds where Qt is available (`cargo build -p rowplay-app`); smoke screenshot attached for QML/3D changes

## Toolchain

<!-- rustc, cargo, Qt (`qmake -query QT_VERSION`), OS. -->

## Documentation (P1 review item)

- [ ] `docs/source-map.md` updated for every ported or changed helper, including divergences from the web app or Studio
- [ ] `docs/roadmap.md` status and the phase's `.kiro/specs/` tasks updated
- [ ] `docs/qt-bridges-notes.md` updated with any Qt Bridges friction, bug or missing feature (with a minimal repro)
- [ ] ADR added under `docs/decisions/` for any architectural decision, in particular anything that would need C++
- [ ] `ASSET_PROVENANCE.md` / `tests/fixtures/PROVENANCE.md` updated for vendored assets or fixtures
- [ ] Every new source file carries the SPDX header

## Deferred, known gaps, Qt Bridges issues

<!-- What was left out and why; every qtbridge problem hit. -->
