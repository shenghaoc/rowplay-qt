# Phase 3 — Platform: tasks

Spec first, then one small commit per deliverable.

- [x] Write the Phase 3 spec (requirements, design, tasks) and commit it before
      any code.
- [x] `rowplay-core::concept2`: raw payload / envelope serde types and
      `from_slice` parsers bounded to 25 MiB.
- [x] `rowplay-core::concept2`: `map_workout`, `map_heart_rate(_value)`,
      `map_targets`, `map_metadata`, `map_split_type`.
- [x] `rowplay-core::concept2`: `map_strokes` (tenths, decimetres, BikeErg
      divisor, interval offsets, `raw_t`/`raw_d`), `map_splits`,
      `synth_strokes`, `assemble_detail`.
- [x] Enable the `#[ignore]`d Concept2 mapper parity test against the four
      fixtures in `tests/fixtures/Concept2/` (fixtures unchanged).
- [x] `rowplay-platform::concept2::http`: `HttpUri`, `resolve`, `redirect_target`
      policy with the loopback exemption.
- [x] `rowplay-platform::concept2::http`: `Concept2HttpClient` (ureq, rustls, no
      cookies/cache, 30 s / 300 s timeouts, 25 MiB body cap, status mapping,
      3 manual redirects, `SecretToken` + `PrivacySafeLogger`).
- [x] Extend `Concept2Client` with `per_page` and the strokes endpoint; update
      the mock and its tests.
- [x] HTTP tests: local `TcpListener` server covering every status mapping, the
      allowed same-origin redirect, the blocked cross-host redirect, the blocked
      downgrade, the timeout, the body cap and token-free error text.
- [x] `rowplay-platform::token_store`: `KeyringTokenStore` with explicit native
      backends per target, plus the "default store is not the mock" test.
- [x] Opt-in keychain round-trip tests behind `ROWPLAY_KEYRING_TESTS=1`,
      including a second-`Entry` load that proves the value really persisted.
- [x] `rowplay-platform::paths`: `directories`-based data/config paths with
      `0700`/`0600` permission helpers on Unix.
- [x] `rowplay-platform::workout_cache`: `SqliteWorkoutCache` (bundled rusqlite,
      Studio's schema, `PRAGMA user_version` migrations, upsert in a
      transaction) with a `tempfile` test for every trait method and an empty
      file.
- [x] `rowplay-platform::preferences`: `FilePreferencesStore` with atomic
      temp-file-and-rename writes, tolerant reads and corrupt-file fallback.
- [x] `rowplay-platform::sync`: `WorkoutSyncCoordinator`, `SyncStateTracker`,
      result / progress / outcome / error types; cancellable via `AtomicBool`
      with a progress callback.
- [x] Sync tests: paging, abort on 401/403/429, continue on other failures,
      counts and timestamps, cancellation, tracker transitions.
- [x] ADR `0007-platform-libraries` (ureq, keyring, rusqlite, directories,
      tempfile, `url` deliberately not used, `serde_json` in core).
- [x] Update `AGENTS.md` (new dependencies and boundaries), `README.md`
      (`dbus-devel` for RHEL, `ROWPLAY_KEYRING_TESTS`), `docs/roadmap.md`
      (Phase 3 delivered) and `docs/source-map.md` (Rust columns + divergence
      rows).
- [x] CI: add `libdbus-1-dev` / `pkg-config` to the Linux apt lists.
- [x] Validation: `cargo fmt --all -- --check`,
      `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`, `git diff --check origin/main...HEAD`.

## Deferred

- The Qt app crate (`rowplay-app`) is unchanged in this phase; the Qt-free
  members are the ones that run here (no Qt on the development host). CI covers
  the app build on the three-OS matrix.
- `Concept2SyncController` (Studio's UI-facing orchestration: token + cache +
  client composition, status messages, disconnect) belongs to the app layer in
  Phase 4; Phase 3 ships the coordinator, tracker and services it composes.
- On-disk cache queries for the Phase 4 library sidebar (SQL-level filtering and
  sorting over the mirrored summary columns).
