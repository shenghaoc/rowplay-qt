# Phase 3 — Platform: requirements

The web app (`shenghaoc/rowplay`, pinned `011e830`) is canonical; rowplay-studio
(pinned `3d406a5`) is the second reference and supplied the Concept2 golden
fixtures. When they disagree the web wins unless Studio's source map documents a
deviation; every choice the Rust port makes is recorded in `docs/source-map.md`.

Everything lands behind the traits that Phase 1 already defined in
`crates/rowplay-platform`: `Concept2Client`, `TokenStore`, `WorkoutCache` and
`PreferencesStore`. The mocks stay; Phase 3 adds production implementations.
No Qt, no tokio, no UI, no OAuth flow.

Scope of the phase:

- the raw Logbook payload types and the mapper live in `rowplay-core` (pure, no
  I/O: byte slices in, existing core models out);
- everything that touches the network, the OS keychain, SQLite or the file
  system lives in `rowplay-platform`;
- the sync coordinator is synchronous and cancellable so Phase 4 can run it on
  a worker thread.

## R1: Concept2 raw-payload mapper (`rowplay-core::concept2`)

- **R1.1** The API envelope and payload types are ported as serde types with the
  wire field names: `SummaryResponse` (`data`, `meta.pagination.total_pages`),
  `DetailResponse` (`data`, `metadata`), `StrokesResponse` (`data`), `RawResult`,
  `RawSplit`, `RawStroke`, `RawHeartRate`/`RawHeartRateValue` (number or
  object), `RawTargets`, `RawMetadata`, `RawWorkout`.
- **R1.2** `map_workout` ports the web `mapResult` field for field: `time` in
  tenths → seconds, pace = `time / (distance / 500)` (0 when distance is 0),
  `heart_rate` (`average`/`min`/`max`/`ending`/`rest`/`recovery`), and
  `stroke_rate`, `stroke_count`, `calories_total`, `wattminutes_total`,
  `drag_factor`, `workout_type`, `comments`, `timezone`, `date_utc`,
  `weight_class`, `privacy`, `source`, `verified`, `rest_time` (tenths →
  seconds), `rest_distance`, `targets`, `metadata`, `has_stroke_data`.
  Absent optional values stay `None` (the web keeps `undefined`; Studio
  substituted `JustRow` / `verified = true` defaults — web wins).
- **R1.3** `map_strokes` ports the web `mapStrokes`: `t` in tenths → seconds,
  `d` in decimetres → metres, `p` in tenths → seconds per 500 m with the
  BikeErg divisor of 2, interval `t`/`d` resets accumulated into running
  offsets, `spm` and optional `hr` carried, watts from
  `pace_to_watts_for_sport`, and the as-logged `raw_t` / `raw_d` kept.
- **R1.4** `map_splits` ports the web `mapSplits` (splits then intervals,
  0-based `index`, `time` in tenths → seconds, pace, `spm`, heart-rate detail
  and scalar `hr`, calories, watt-minutes, interval `type`, rest fields,
  `machine` as a `Sport`, `is_rest = distance == 0 && time > 0`).
- **R1.5** `map_heart_rate`, `map_heart_rate_value`, `map_targets`,
  `map_metadata` and `map_split_type` port the web helpers, including the
  "empty object means `None`" rule for heart rate and targets, and the BikeErg
  divisor on target pace.
- **R1.6** `assemble_detail` ports the web `getWorkout`: `is_interval` from a
  non-empty `workout.intervals`, and when no strokes were returned it
  synthesises them from the splits — or from a 60-step summary segment when
  there are none — then clears `has_stroke_data` so the pose model treats the
  workout as split-derived.
- **R1.7** Parsing is fallible and bounded: every `from_slice` entry point
  rejects payloads over 25 MiB and reports a typed error without echoing the
  payload.

## R2: Concept2 HTTP client (`rowplay-platform::concept2`)

- **R2.1** `Concept2HttpClient` implements the extended trait with blocking
  `ureq` (rustls) against base `https://log.concept2.com`:
  `GET /api/users/me/results?page=&number=`,
  `GET /api/users/me/results/{id}?include=metadata` and
  `GET /api/users/me/results/{id}/strokes`.
- **R2.2** Every request carries `Authorization: Bearer <token>` (bring your own
  token; there is no OAuth flow) and
  `Accept: application/vnd.c2logbook.v1+json`.
- **R2.3** HTTPS only: a plain `http` base is rejected with
  `InsecureConnection` unless the host is `localhost`, `127.0.0.1` or `::1`, so
  tests can use a local server.
- **R2.4** Automatic redirects are off. The client follows at most 3 redirects
  by hand and only same-origin HTTPS ones (host compared case-insensitively,
  effective ports compared). Cross-host redirects and HTTPS→HTTP downgrades are
  rejected with `InsecureRedirectBlocked`, and the token is never sent to
  another host.
- **R2.5** Timeouts: 30 s per request and 300 s overall for one logical fetch
  (including its redirect chain). No cookies and no HTTP cache.
- **R2.6** Any response body over 25 MiB is rejected as `BodyTooLarge`.
- **R2.7** Status mapping: 401 → `Unauthorized`, 403 → `Forbidden`,
  429 → `RateLimited` (with `Retry-After` seconds when the header is a plain
  integer), every other non-2xx → `Http { status }`. Undecodable bodies →
  `Decode`, transport failures → `Transport`, timeouts → `Timeout`.
- **R2.8** The token never appears in `Display`/`Debug` output, error strings or
  logs. The client holds a `SecretToken`, logs through `PrivacySafeLogger`, and
  a test asserts the token is absent from every error text the client can
  produce.
- **R2.9** `result_detail` follows Studio's shape — strokes are fetched only
  when `stroke_data` is true, and a failed stroke fetch is non-fatal (falls back
  to synthesised strokes) — and the trait additionally exposes the stroke fetch
  directly.

## R3: Token store (`rowplay-platform::token_store`)

- **R3.1** `KeyringTokenStore` implements `TokenStore` with the `keyring` crate
  and a native backend per target: macOS Keychain (`apple-native`), Windows
  Credential Manager (`windows-native`) and Linux Secret Service
  (`sync-secret-service`, with the pure-Rust encryption backend).
- **R3.2** The backends are enabled explicitly; no target is allowed to fall
  back to keyring's in-memory mock store. A default-run test fails when the
  crate's default credential builder is the mock.
- **R3.3** `load` returns `None` for a missing entry (never an error), `save`
  replaces, and `clear` is idempotent. Backend failures become
  `TokenStoreError::Unavailable` with a redacted message; a stored value that
  fails `SecretToken` validation is an `Invalid` error, never a token echo.
- **R3.4** Tests that touch the real keychain run only when
  `ROWPLAY_KEYRING_TESTS=1`; every other test uses `InMemoryTokenStore`.

## R4: Workout cache (`rowplay-platform::workout_cache`)

- **R4.1** `SqliteWorkoutCache` implements `WorkoutCache` with `rusqlite`
  (`bundled`, so no system SQLite is required on Windows).
- **R4.2** The schema mirrors Studio's `SQLiteWorkoutCache`: one `workouts`
  table keyed by Concept2 result id holding the summary columns plus the full
  `WorkoutDetail` JSON, an index on the date, and a version tracked by
  `PRAGMA user_version` (currently 1). `date` is stored as the logbook string
  because the model keeps it verbatim.
- **R4.3** The database lives in the platform data directory found with the
  `directories` crate; on Unix the directory is created with mode `0700` and the
  database file with mode `0600`.
- **R4.4** `migrate` is idempotent and safe on an empty (zero-byte) file;
  `save_details` upserts in one transaction; `list_workouts` is newest first;
  `details` omits unknown ids; `clear` empties the table. Failures are typed
  `CacheError`s that never carry payloads.

## R5: Preferences (`rowplay-platform::preferences`)

- **R5.1** `FilePreferencesStore` persists `Preferences` as JSON in the platform
  config directory (the `directories` crate).
- **R5.2** Writes are atomic: a temporary file in the same directory is renamed
  over the target, and on Unix the file is `0600`.
- **R5.3** Reads are tolerant: unknown fields are ignored, a missing file yields
  defaults, and a corrupt file yields defaults plus one redacted log line. The
  store never panics and never holds a token (`Preferences` has no token field).

## R6: Sync (`rowplay-platform::sync`)

- **R6.1** `WorkoutSyncCoordinator` ports Studio's semantics exactly: migrate
  the cache, page through all summaries, then fetch and save each detail.
- **R6.2** Individual detail-fetch or save failures are counted and the sync
  continues; 401, 403 and 429 abort it. A fundamental summary-fetch failure
  aborts with `ClientFailed`; cache migration/save failures are typed
  (`CacheFailed`).
- **R6.3** `WorkoutSyncResult` reports fetched, saved and failed counts plus the
  start and finish instants.
- **R6.4** The sync is synchronous and cancellable: an `AtomicBool` is checked
  between requests and a progress callback reports `(completed, total)`.
  Cancellation returns an `Ok` outcome flagged `cancelled`, not an error.
- **R6.5** `SyncStateTracker` ports Studio's state machine: `refresh_workout_count`,
  `sync_started` (clears the previous error), `sync_completed` (stamps the date,
  refreshes the count) and `sync_failed` (redacted message + timestamp, then
  refresh so a partial sync still shows its workouts).

## R7: Dependencies and boundaries

- **R7.1** New crates go only into `rowplay-platform`: `ureq` (rustls, no
  cookies, no json feature), `keyring`, `rusqlite` (`bundled`), `directories`,
  plus `tempfile` as a dev-dependency. `rowplay-core` gains only `serde_json`
  (promoted from a dev-dependency) for the payload parser.
- **R7.2** No Qt, no tokio, no OAuth, no UI. `unsafe_code` stays forbidden and
  the pinned toolchain in `rust-toolchain.toml` is unchanged.
- **R7.3** ADR `0007-platform-libraries` records the choices and the boundaries;
  `AGENTS.md` lists the new dependencies.

## R8: Fixtures, CI and validation

- **R8.1** The `#[ignore]`d Concept2 mapper parity test is enabled and passes for
  all four fixtures in `tests/fixtures/Concept2/` (tenths of seconds,
  decimetres, the BikeErg pace divisor and interval offsets). Fixtures are not
  edited.
- **R8.2** The HTTP rules are tested against a local `std::net::TcpListener`
  server with no real network access: every status mapping, an allowed
  same-origin redirect, a blocked cross-host redirect, a blocked downgrade, the
  30 s timeout, the 25 MiB body cap, and the token absent from error text.
- **R8.3** No test in the default run touches the network or the real keychain.
  Demo mode keeps working with no token and no network.
- **R8.4** CI gains the Linux build dependency for the Secret Service backend
  (`libdbus-1-dev` / `pkg-config`) and the README notes `dbus-devel` for RHEL.
- **R8.5** `cargo fmt --all -- --check`,
  `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace` (Qt-free members) and `git diff --check` pass.
