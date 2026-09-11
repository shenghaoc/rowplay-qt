# Phase 3 — Platform: design

## Module map

| Module | Web source | Studio source | Rust |
| --- | --- | --- | --- |
| `rowplay_core::concept2` | `src/lib/server/concept2.ts` (raw shapes, `mapResult`, `mapStrokes`, `mapSplits`, `mapHeartRate`, `mapTargets`, `mapMetadata`, `synthStrokes`, `getWorkout`) | `Concept2/Concept2Models.swift`, `Concept2/Concept2Mapper.swift` | `crates/rowplay-core/src/concept2.rs` |
| `rowplay_platform::concept2` | `Concept2Client` (fetch, `/api` prefix, headers, `Accept`) | `Concept2/Concept2Endpoint.swift`, `Concept2/Concept2Error.swift`, `Concept2/HTTPTransport.swift`, `Concept2/URLSessionConcept2Client.swift` | `crates/rowplay-platform/src/concept2/mod.rs`, `…/concept2/http.rs` |
| `rowplay_platform::token_store` | — | `Sync/TokenStore.swift`, `Platform/KeychainTokenStore.swift` | `crates/rowplay-platform/src/token_store.rs` |
| `rowplay_platform::workout_cache` | — | `Sync/WorkoutCache.swift`, `Storage/SQLiteWorkoutCache.swift`, `Storage/SQLiteWorkoutCacheMigration.swift` | `crates/rowplay-platform/src/workout_cache/{mod,sqlite}.rs` |
| `rowplay_platform::preferences` | `safeStorage` usage in `replayRenderer.ts` | `Platform/AppPreferences.swift` | `crates/rowplay-platform/src/preferences.rs` |
| `rowplay_platform::sync` | `data.ts` `SyncState` | `Sync/WorkoutSyncCoordinator.swift`, `Sync/SyncStateTracker.swift` | `crates/rowplay-platform/src/sync.rs` |
| `rowplay_platform::paths` | — | `Concept2SyncController.defaultCachePath` | `crates/rowplay-platform/src/paths.rs` |

Web names survive in snake_case (`map_workout`, `map_strokes`, `map_splits`,
`synth_strokes`, `assemble_detail`, `list_results`, `result_detail`) so
`docs/source-map.md` stays greppable.

## Core mapper

`rowplay-core` stays pure: the concept2 module has no I/O and consumes
`&[u8]`. It gains `serde_json` (promoted from a dev-dependency) because the
mapper must "parse byte slices into existing core models"; nothing else in the
dependency story changes.

```rust
pub const MAX_PAYLOAD_BYTES: usize = 25 * 1024 * 1024;

SummaryResponse, DetailResponse, StrokesResponse, RawResult, RawSplit,
RawStroke, RawHeartRate, RawHeartRateValue, RawWorkout, RawTargets,
RawMetadata, Pagination, Meta

impl RawResult { fn from_slice(&[u8]) -> Result<Self, Concept2PayloadError> }
impl SummaryResponse { fn from_slice(..) } // + Detail/Strokes

map_workout(&RawResult) -> Workout
map_workout_with_metadata(&RawResult, Option<&RawMetadata>) -> Workout
map_heart_rate_value(Option<&RawHeartRateValue>) -> Option<HeartRateDetail>
map_targets(Option<&RawTargets>, Sport) -> Option<WorkoutTargets>
map_metadata(Option<&RawMetadata>) -> Option<LoggingMetadata>
map_split_type(Option<&str>) -> Option<SplitIntervalType>
map_strokes(&[RawStroke], Sport) -> Vec<Stroke>
map_splits(&RawResult) -> Vec<Split>
synth_strokes(&Workout, &[Split]) -> Vec<Stroke>
assemble_detail(&DetailResponse, Option<&[RawStroke]>) -> WorkoutDetail
```

`map_strokes` keeps the running `t`/`d` offset state from the web: a stroke whose
normalised time or distance goes backwards means the monitor reset its counter
for a new interval, so the previous value is added to the offset. This is what
makes the replays of interval pieces monotonic, and it is asserted by
`rower-interval.fixture.json` (`_index` 5 restarts at the previous rep's
`rawT`/`rawD`).

`assemble_detail` is the web `getWorkout`:

```text
workout = map_workout_with_metadata(raw, metadata)
strokes = stroke_data ? map_strokes(raw_strokes, sport) : []
splits  = map_splits(raw)
synthesised = strokes.is_empty()
strokes = synthesised ? synth_strokes(&workout, &splits) : strokes
is_interval = !intervals.is_empty()
has_stroke_data = workout.has_stroke_data && !synthesised
```

The `has_stroke_data` clearing matters: when the strokes are synthesised from
splits the pose model must treat the workout as split-derived, otherwise it
renders one full stroke cycle per synthesised point (≈4 catches across a 2K
instead of ≈221).

## HTTP client

`Concept2HttpClient` wraps a `ureq::Agent` built once:

```rust
Agent::config_builder()
    .http_status_as_error(false)       // we classify statuses ourselves
    .max_redirects(0)                  // …and follow them by hand
    .timeout_per_call(Some(30s))       // per request
    .timeout_global(Some(300s))        // per call (belt and braces)
    .max_response_header_size(64 * 1024)
    .user_agent("rowplay-qt/<version>")
    .build()
```

No cookies (the `cookies` feature is off) and no HTTP cache (ureq has none; the
old response cache was removed upstream). TLS is rustls, the default provider.

A fetch is: build the URI → check scheme → until the redirect budget is spent,
call the agent, and on a 3xx read `Location` and re-check the target. The body
is read with an explicit limit (`body_mut().with_config().limit(25 MiB)`), which
surfaces `Error::BodyExceedsLimit`. Statuses are classified by
`map_status(status, retry_after)`.

### URI and redirect policy

`http::HttpUri` is a deliberately small absolute-URI type — we never need
query-string encoding, userinfo, or relative references beyond joining a
`Location` onto a URL we built ourselves, and keeping it local avoids pulling
`url`/ICU into the desktop build:

```rust
struct HttpUri { scheme: String, host: String, port: u16, path_and_query: String }
```

- parse: `scheme://authority/path?query`; `scheme` is `http` or `https`
  (lower-cased), the authority may carry a port (defaults 443/80) and must not
  carry userinfo, IPv6 hosts keep their brackets, and the path is normalised to
  start with `/`.
- `resolve(location)`: absolute (`scheme://…`), protocol-relative (`//host/…`),
  absolute path (`/…`) and relative path forms are supported; anything with
  control characters or whitespace is rejected.
- `origin_matches(other)`: lower-cased host equality plus effective port
  equality.

`redirect_target(base, location)` is the whole policy, in one fail-closed place:

```text
target = base.resolve(location)?
if base.is_secure() && !target.is_secure()            -> InsecureRedirectBlocked
if !(target.is_secure() || target.is_loopback())      -> InsecureRedirectBlocked
if !base.origin_matches(&target)                      -> InsecureRedirectBlocked
```

`is_loopback()` is `localhost`, `127.0.0.1` or `::1` (case-insensitive), which
is the same exemption the initial request uses, so the local-server tests can
exercise an allowed same-origin redirect without TLS. `is_secure()` is `https`.
A scheme change is therefore only possible as a loopback `http` → `https`
upgrade on the same host and port; the origin comparison follows Studio (host +
port, case-insensitive) rather than RFC 6454's full origin.

### Error taxonomy

`Concept2Error` keeps the Phase 1 variants (`Unauthorized`, `RateLimited`,
`NotFound`, `Transport`, `Decode`) and adds what Studio's contract needs:

| Variant | Source |
| --- | --- |
| `Unauthorized` / `Forbidden` / `RateLimited { retry_after_secs }` / `Http { status }` | status mapping |
| `InsecureConnection` | initial URI is plain `http` off-loopback |
| `InsecureRedirectBlocked` | cross-host, downgrade or non-secure redirect |
| `InvalidUrl` | base URL or `Location` that cannot be parsed |
| `Timeout` | `ureq::Error::Timeout` |
| `BodyTooLarge` | response body over the cap |
| `Decode` / `Transport` | payload and transport failures, redacted text |
| `NotFound(i64)` | domain "unknown result id" (mock) |

`Display` for `Transport`/`Decode` carries redacted text only; `SecretToken` has
no `Display` and a `REDACTED` `Debug`, the request URL never contains the token
(it is a header), and the client logs through `PrivacySafeLogger`.

### Trait extension

```rust
pub trait Concept2Client: Send + Sync {
    fn list_results(&self, page: u32, per_page: u32) -> Result<ResultsPage, Concept2Error>;
    fn result_detail(&self, id: i64) -> Result<WorkoutDetail, Concept2Error>;
    fn strokes(&self, id: i64, sport: Sport) -> Result<Vec<Stroke>, Concept2Error>;
}
```

`per_page` is new (Studio's `fetchWorkouts(page:perPage:)`; the sync coordinator
uses the API maximum of 250, as the web does). `strokes` is the explicit
endpoint the task calls out; `result_detail` still assembles the full detail
itself, exactly like Studio's `fetchWorkoutDetail`.

## Token store

```rust
KeyringTokenStore { entry: keyring::Entry, logger: PrivacySafeLogger<'static> }
```

Service `com.rowplay-qt.concept2-token`, account `default` (Studio's service
name with the Qt app's identifier). `Entry::get_password` returning
`keyring::Error::NoEntry` is `Ok(None)`; anything else is
`TokenStoreError::Unavailable(redact(err))`. `delete_credential` treats
`NoEntry` as success, so `clear` is idempotent.

Features are enabled for all three targets at once because keyring's backends are
`cfg`-gated dependencies — `apple-native` pulls `security-framework` only on
macOS, `windows-native` pulls `windows-sys` only on Windows, and
`sync-secret-service` pulls `dbus-secret-service` only on Linux/BSD:

```toml
keyring = { version = "3.6.3", default-features = false, features = [
  "apple-native", "windows-native", "sync-secret-service", "crypto-rust",
] }
```

`crypto-rust` encrypts secrets in transit over the session bus without adding an
OpenSSL dependency. keyring 3 has **no default features**: without a backend
feature it silently substitutes its in-memory mock store, which would lose the
token on quit. `keyring::default::default_credential_builder().persistence()`
returns `EntryOnly` for the mock and `UntilDelete` for every native store, so a
default-run unit test asserts it is not `EntryOnly` and CI fails if the features
are ever dropped.

`sync-secret-service` links `libdbus-1` through `libdbus-sys`, so Linux builds
need `libdbus-1-dev` (Debian/Ubuntu) or `dbus-devel` (RHEL). The keychain tests
are opt-in (`ROWPLAY_KEYRING_TESTS=1`) because CI has no Secret Service and
macOS would prompt.

## Workout cache

`rusqlite` with `bundled` compiles SQLite itself, so Windows CI needs no system
library. `Connection` is `Send` but not `Sync`, so the cache holds
`Mutex<Connection>`.

Schema (version 1), mirroring Studio's table while keeping `date` as the
logbook string the model uses:

```sql
CREATE TABLE IF NOT EXISTS workouts (
    id INTEGER PRIMARY KEY,
    sport TEXT NOT NULL,
    date TEXT NOT NULL,
    workout_type TEXT,
    distance REAL NOT NULL,
    time REAL NOT NULL,
    pace REAL NOT NULL,
    stroke_rate REAL,
    stroke_count REAL,
    heart_rate_avg REAL,
    calories_total REAL,
    watt_minutes REAL,
    drag_factor REAL,
    comments TEXT,
    source TEXT,
    verified INTEGER,
    has_stroke_data INTEGER NOT NULL DEFAULT 0,
    is_interval INTEGER NOT NULL DEFAULT 0,
    detail_json TEXT NOT NULL,
    updated_at REAL NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_workouts_date ON workouts (date DESC);
```

The summary columns exist so `list_workouts` never has to decode JSON; a test
round-trips details and asserts the columns and the JSON agree. `save_details`
upserts (`INSERT … ON CONFLICT(id) DO UPDATE`) inside one transaction.
Migrations run in a transaction and are recorded with
`PRAGMA user_version = 1`; running them twice is a no-op, and a zero-byte file
migrates cleanly.

Files and directories come from `paths`: `directories::ProjectDirs` data
directory, `workouts.sqlite`, created with mode `0700` for the directory and
`0600` for the file on Unix (`DirBuilderExt::mode`, `OpenOptionsExt::mode`, plus
a defensive `set_permissions` after opening). WAL is deliberately not enabled so
SQLite does not create `-wal`/`-shm` siblings with default permissions.

## Preferences

`FilePreferencesStore { path, logger }` in the platform config directory as
`preferences.json`. `save` serialises to JSON, writes `<name>.tmp` in the same
directory with mode `0600`, then `fs::rename`s it over the target (atomic on one
filesystem) and removes the temporary file if anything fails. `load` returns
defaults for a missing file (one info line) or a corrupt file (one warning
line); `serde`'s `#[serde(default)]` plus unknown-field tolerance means a partial
file keeps its known values and everything else falls back. `Preferences` still
has no token field.

## Sync

```rust
WorkoutSyncCoordinator<'a> { client: &'a dyn Concept2Client, cache: &'a dyn WorkoutCache,
                             per_page: u32, logger: PrivacySafeLogger<'static> }
SyncStateTracker<'a>       { cache: &'a dyn WorkoutCache, state: Mutex<SyncState>,
                             logger: PrivacySafeLogger<'static> }
WorkoutSyncResult { fetched_count, saved_count, failed_count, started_at, finished_at }
SyncProgress { completed, total }
SyncOutcome { result, cancelled }
WorkoutSyncError { ClientFailed(String), CacheFailed(String), MappingFailed(String) }
```

`sync_all()` is `sync_all_with(&AtomicBool::new(false), &mut |_| {})`, so there is
one code path. The algorithm is Studio's `syncAll` in order:

1. `cache.migrate()` — failure is `CacheFailed`;
2. page summaries from page 1 while `page <= total_pages`, recording
   `fetched_count` — any failure is logged redacted and returned as
   `ClientFailed`;
3. for each summary: check the cancel flag → `SyncOutcome { cancelled: true }`;
   fetch the detail, then save it. A save failure counts as failed and
   continues; a fetch failure counts as failed and continues **unless** it is
   401/403/429, which aborts with `ClientFailed`;
4. report counts and the start/finish instants.

`should_abort_sync` mirrors Studio's `shouldAbortSync`: `Unauthorized`,
`Forbidden`, `RateLimited`, and defensively `Http { status }` for those three
codes.

Times are `chrono::DateTime<Utc>` (`Utc::now()`), matching Studio's `Date` and
giving Phase 4 a real instant to format. Tests assert ordering and counts rather
than wall-clock values.

`SyncStateTracker` keeps Studio's transitions, with the state behind a `Mutex`
so Phase 4 can read it from the bridge thread:

| Call | Effect |
| --- | --- |
| `refresh_workout_count` | cache `list_workouts().len()`, warn on failure and keep the old count |
| `sync_started` | `in_progress = true`, clear `last_error`/`last_error_date` |
| `sync_completed` | `in_progress = false`, `last_sync_date = now`, refresh count |
| `sync_failed` | `in_progress = false`, redacted `last_error` + date, refresh count (a partial sync still shows its workouts) |

## Tests

- `crates/rowplay-core/tests/parity.rs`: the four Concept2 fixtures drive
  `parse_detail_response` + `parse_strokes_response` + `assemble_detail`; the
  fixture's `expected.result` (sport, time, distance, pace), `expected.strokes`
  (by `_index`, including `rawT`/`rawD` for the interval case) and
  `expected.splits` are compared with a 1e-9 tolerance.
- `rowplay-core::concept2` unit tests cover the unit conversions, the BikeErg
  divisor, the offset accumulation, the empty-object rules, target pace and the
  synthesised-stroke fallbacks.
- `rowplay-platform::concept2::http` unit tests cover URI parsing, `resolve`,
  and every branch of `redirect_target` (including the downgrade, which needs no
  server).
- `rowplay-platform::concept2::http::tests::server` runs a tiny
  `std::net::TcpListener` server in a thread: status mapping (200/401/403/429/
  404/500), an allowed same-origin redirect, a blocked cross-host redirect, a
  429 `Retry-After`, a request timeout, an oversized body, and a token-absence
  assertion over every error string. No test dials the real Logbook API.
- The cache, preferences, token store and sync suites use `tempfile` and the
  in-memory mocks; the real keychain is behind `ROWPLAY_KEYRING_TESTS=1`.

## Non-goals

No QML, no Qt, no OAuth (bring-your-own-token only), no live PM5/BLE work, no
Phase 4 controller (`Concept2SyncController`'s UI-facing orchestration is the
app layer's job), no worker thread or cancellation handle beyond the
`AtomicBool`, no analytics, no token rotation, no on-disk HTTP cache.
