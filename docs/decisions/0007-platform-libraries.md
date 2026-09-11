# ADR 0007 — Platform libraries: ureq, keyring, rusqlite, directories

Status: accepted (2026-09-12)

## Context

Phase 3 gives `rowplay-platform` its production implementations: an HTTP client
for the Concept2 Logbook, an OS-keychain token store, a local workout cache and
a preferences file. ADR 0006 fixed the layering (`app → platform → core`) but
left the external crates open. The constraints were:

- no Qt, and **no tokio**: the app is a Qt event loop, not an async runtime, and
  a second reactor inside a Qt process is a liability;
- the token must never leave the OS credential store, and never reach another
  host over a redirect;
- `rowplay-core` stays dependency-light (its parsers only need `serde`, which
  it already has);
- the MSRV in `rust-toolchain.toml` and CI is 1.87, and CI runs Ubuntu, macOS
  and Windows plus an MSRV job.

## Decision

Four crates, all in `rowplay-platform` only:

| Crate | Version | Role | Why this one |
| --- | --- | --- | --- |
| `ureq` | `3.4.1`, `default-features = false`, `features = ["rustls"]` | blocking Concept2 client | Blocking I/O is what a Qt worker thread wants; `rustls` avoids an OpenSSL build dependency on every platform; `cookies`, `json`, `charset` and the proxy features stay off because the client needs none of them. Its `max_redirects(0)` plus a hand-written policy is what makes the token-leak rules ours. |
| `keyring` | `3.6.3`, `default-features = false`, features `apple-native`, `windows-native`, `sync-secret-service`, `crypto-rust` | Concept2 token store | keyring 4 requires Rust 1.88, above the pinned MSRV. keyring 3's backends are `cfg`-gated dependencies, so naming all three in one declaration is safe on every target. It has **no default features**: without one it silently substitutes an in-memory mock store, which would lose the token on quit, so `the_default_credential_store_is_not_the_mock` fails the build's tests if the features are ever dropped. `crypto-rust` encrypts secrets in transit over the session bus without pulling OpenSSL. |
| `rusqlite` | `0.40.2`, `features = ["bundled"]` | workout cache | The only mature Rust SQLite binding, and `bundled` compiles SQLite itself so Windows and macOS CI need no system library or `sqlite3` headers. |
| `directories` | `6.0.0` | data/config paths | Follows each platform's convention (XDG, `Library/Application Support`, `%APPDATA%`) instead of hard-coding one. |

Supporting choices:

- `tempfile` is a **dev-dependency only** (caches, preferences and the
  permission tests use throwaway directories).
- `serde_json` moves from `rowplay-core`'s dev-dependencies to its dependencies,
  because the Concept2 mapper's contract is "byte slices in, core models out".
  No other crate is added to core.
- `chrono` also gains a `rowplay-platform` dependency edge (it was already a
  workspace dependency used by core) for the sync timestamps.
- **`url` is deliberately not used.** It is a real dependency tree today
  (`idna` → ICU4X) for a job we need only in a restricted form: comparing the
  origin of a `Location` header against a URL we built ourselves. `HttpUri`
  covers exactly that, and the redirect policy fails closed on anything it
  cannot affirmatively classify, so the security boundary does not depend on
  parsing breadth.

## Consequences

- Linux builds need the Secret Service client library (`libdbus-1-dev` plus
  `pkg-config`, or `dbus-devel`/`pkgconf` on RHEL) for `libdbus-sys`. CI installs
  it in the Qt-free and MSRV jobs and in the Linux app job; macOS and Windows
  need nothing extra.
- Tests that touch the OS keychain are opt-in via `ROWPLAY_KEYRING_TESTS=1`,
  because CI has no Secret Service and macOS prompts. Everything else uses the
  in-memory mock, and no default-run test opens a socket: the HTTP rules are
  tested against a local `TcpListener`.
- keyring's version, like `qtbridge`'s, is a deliberate pin: bumping it means
  re-checking the backend features and the mock-store guard.
- `ureq`'s transport errors are reduced to a static class before they reach a
  `Concept2Error`, so no error string can carry request detail. That costs some
  diagnostic detail; it is the right trade for a client that holds a bearer
  token.
- The dependency set stays out of `rowplay-core`, so the parity test oracle
  keeps building with the minimum toolchain.
