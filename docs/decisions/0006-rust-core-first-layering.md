# ADR 0006 — Rust core first: app → platform → core

Status: accepted (2026-09-11)

## Context

rowplay-studio's Core / Platform / Studio split kept its domain logic
cross-platform and testable on Linux while the UI stayed macOS-only. rowplay-qt
needs the same discipline: Qt Bridges is a beta, Qt is heavy to install in CI,
and parity fixtures are the test oracle.

## Decision

Cargo workspace with dependency direction **app → platform → core**:

| Crate | Depends on Qt? | Contents |
| --- | --- | --- |
| `rowplay-core` | no | pure domain: models, formatting, datetime, pace input, privacy, analytics, PBs, predictor, workout query, tags, demo library; later the replay engine, motion, motion graph, ghost pick, race gap/result, rival parsers, stroke pose, quality budgets, perf governor. No I/O beyond parsing byte slices. |
| `rowplay-platform` | no | services behind traits with mock implementations: `TokenStore` (keyring), `WorkoutCache` (rusqlite), `Concept2Client`, `PreferencesStore`, library loading, log sinks. |
| `rowplay-app` | yes | the binary: qtbridge backend objects, QML shell, Qt Quick 3D replay, Qt Graphs charts. |
| `rowplay-fixtures` | no (dev only) | loader for the golden parity fixtures under `tests/fixtures/`. |

- Everything testable without Qt lives in `core` or `platform` and runs on
  every CI push; the app builds on a three-OS matrix.
- Privacy invariants from Studio carry over: tokens only in the OS keychain
  via `keyring` (never in files, logs, fixtures or preferences); all logging of
  user data goes through `rowplay_core::privacy::redact`; parsers bound input
  length; share/export strips hardware-identifying metadata.
- Demo mode is first-class: `rowplay_core::demo` reproduces the web app's
  deterministic seeded workouts so the app is fully explorable without a
  Concept2 token.

## Consequences

- The parity oracle is `tests/fixtures/` (from rowplay-studio); every ported
  web helper gets a golden-fixture or web-test-derived test with a stated
  tolerance. When the web and Swift versions disagree, the web wins unless
  Studio's source map documents a deliberate deviation; every divergence is
  recorded in `docs/source-map.md`.
- `cargo build` / `cargo test` at the workspace root default to the Qt-free
  crates (`default-members`); `--workspace` or `-p rowplay-app` needs Qt.
