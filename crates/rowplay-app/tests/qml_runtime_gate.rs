// SPDX-License-Identifier: GPL-3.0-or-later
//! QML runtime-error gate (Phase 4, spec R8).
//!
//! Launches the app in gate mode (`ROWPLAY_SMOKE_GATE=1`): demo data, no
//! token, offscreen platform. The shell walks every screen, flips through all
//! six languages and exercises the preference slots on a timer, then exits 0.
//! The test fails when stderr contains a QML `TypeError`, `ReferenceError`,
//! `Binding loop`, `Unable to assign`, `is not defined` or `was not placed in
//! the graphics scene` — the classes of silent breakage QML bindings and
//! dynamically created Quick 3D objects produce at runtime, which the
//! compiler and `qmllint` (no `.qmltypes` for Rust types, qt-bridges-notes
//! #9) cannot see.
//!
//! Unlike the 3D smoke screenshot test this needs no GL: the shell is pure
//! Qt Quick 2D, so `offscreen` renders it and the gate runs in every
//! `cargo test -p rowplay-app` invocation, including CI on all three OSes.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

mod common;

const FORBIDDEN_PATTERNS: [&str; 6] = [
    "TypeError",
    "ReferenceError",
    "Binding loop",
    "Unable to assign",
    "is not defined",
    // A Quick 3D object created with `createObject` under a 2D parent never
    // reaches the scene graph (Phase 5a: the rebuilt sky texture data).
    "was not placed in the graphics scene",
];

/// The Qt-bridge singletons whose members QML must resolve.
const SINGLETONS: [&str; 5] = ["Library", "Detail", "Settings", "Sync", "Replay"];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root")
}

/// Strips `//` line comments and `/* … */` blocks so commented-out examples
/// cannot produce false positives.
fn strip_comments(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let bytes = source.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
        } else if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                // Keep newlines so line-based diagnostics stay sensible.
                if bytes[i] == b'\n' {
                    out.push('\n');
                }
                i += 1;
            }
            i += 2;
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}

fn qml_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("read qml dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            qml_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "qml") {
            out.push(path);
        }
    }
}

/// Every `Singleton.member` reference in `qml/`, in stable order.
///
/// Signal-handler names (`onLibraryChanged`) and bare identifiers are not
/// member accesses and are skipped; chained accesses keep only the first hop
/// (`Library.tilesJson[0].labelId` → `Library.tilesJson`).
fn referenced_members() -> BTreeSet<String> {
    let mut files = Vec::new();
    qml_files(&repo_root().join("qml"), &mut files);
    assert!(!files.is_empty(), "no QML files found");

    let mut found = BTreeSet::new();
    for path in files {
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
        let source = strip_comments(&source);
        let bytes = source.as_bytes();
        for singleton in SINGLETONS {
            let mut start = 0;
            while let Some(hit) = source[start..].find(&format!("{singleton}.")) {
                let at = start + hit;
                // Must be a standalone identifier, not a suffix of a longer
                // one (e.g. `MyLibrary.` or `Synchroniser.`).
                let boundary =
                    at == 0 || !(bytes[at - 1].is_ascii_alphanumeric() || bytes[at - 1] == b'_');
                let mut end = at + singleton.len() + 1;
                while end < bytes.len()
                    && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_')
                {
                    end += 1;
                }
                let member = &source[at + singleton.len() + 1..end];
                if boundary && !member.is_empty() {
                    found.insert(format!("{singleton}.{member}"));
                }
                start = at + singleton.len() + 1;
            }
        }
    }
    found
}

#[test]
fn shell_walk_produces_no_qml_runtime_errors() {
    // The walk saves per-screen captures here; on a fresh CI checkout the
    // (git-ignored) directory does not exist and every grabToImage save
    // silently fails (saveToFile returns false, the walk logs FAILED and
    // continues). The smoke test creates its own artifact directory; this
    // one must too, or the capture assertions below are unrunnable.
    if let Some(dir) = std::env::var_os("ROWPLAY_SMOKE_SCREENSHOT_DIR") {
        std::fs::create_dir_all(&dir).expect("create screenshot directory");
    }
    let mut command = Command::new(env!("CARGO_BIN_EXE_rowplay-app"));
    command
        .env("ROWPLAY_SMOKE_GATE", "1")
        // On Windows Qt routes logging to OutputDebugString when stderr is a
        // pipe, which would blind both the error scan and the walk's own
        // console.log probes; force stderr everywhere (no-op on Unix).
        .env("QT_FORCE_STDERR_LOGGING", "1")
        // The walk ends with an end-to-end sync against the deterministic
        // mock client: worker thread, mpsc events, cross-thread invoker,
        // cache writes and the completion path — no token, no network.
        .env("ROWPLAY_SYNC_MOCK", "1")
        // Keep a caller-provided platform (e.g. xcb under Xvfb); default to
        // offscreen, which is enough for the 2D shell.
        .env(
            "QT_QPA_PLATFORM",
            std::env::var_os("QT_QPA_PLATFORM").unwrap_or_else(|| "offscreen".into()),
        )
        .env("LANG", "C.UTF-8")
        // The member checklist is scanned from qml/ right here, so a property
        // reference added to any screen is probed on the next gate run without
        // anybody maintaining a hand-written list.
        .env(
            "ROWPLAY_GATE_MEMBER_CHECK",
            referenced_members()
                .into_iter()
                .collect::<Vec<_>>()
                .join(","),
        );
    // Hermetic state: the walk syncs and clears the cache, so it must never
    // touch the developer's real logbook data directory.
    let temp = std::env::temp_dir().join(format!("rowplay-gate-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp);
    command.env("ROWPLAY_DATA_DIR", &temp);
    let output = command.output().expect("launch rowplay-app in gate mode");
    let _ = std::fs::remove_dir_all(&temp);

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Scan both streams: console.log may land on either depending on the
    // platform and Qt version.
    let combined = format!("{stdout}\n{stderr}");
    for pattern in FORBIDDEN_PATTERNS {
        for line in combined.lines() {
            assert!(
                !line.contains(pattern),
                "QML runtime error ({pattern}) during the gate walk:\n{line}\n\n\
                 full stderr:\n{stderr}"
            );
        }
    }
    assert!(
        output.status.success(),
        "gate walk exited with {}\nstderr:\n{stderr}",
        output.status
    );
    // The gate logs its language probes; their presence proves the walk ran
    // and that translations loaded and retranslated live instead of falling
    // back to message ids.
    assert!(
        combined.contains("gate i18n en: Dashboard"),
        "translations did not load (qsTrId fell back to ids)\noutput:\n{combined}"
    );
    assert!(
        combined.contains("gate i18n zh:") && !combined.contains("gate i18n zh: nav.dashboard"),
        "live language switch to zh did not retranslate\noutput:\n{combined}"
    );
    // Singleton properties: every member QML references must resolve on the
    // qtbridge object. A missing registration reads as `undefined` and
    // produces no QML warning at all, so this is the only thing that catches
    // it.
    let members = referenced_members();
    assert!(
        members.len() > 50,
        "member scan found only {} references — the scanner is stale",
        members.len()
    );
    assert!(
        !combined.contains("gate members unresolved:"),
        "QML references singleton members that do not resolve (a missing \
         qproperty! registration reads as `undefined` with no QML error):\n{}",
        combined
            .lines()
            .filter(|line| line.contains("gate members"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert!(
        combined.contains("gate members:") && combined.contains("resolved"),
        "the member check did not run\noutput:\n{combined}"
    );

    // The mock sync must complete and land in the cache, not the demo data.
    assert!(
        combined.contains("gate sync: sync.done"),
        "end-to-end mock sync did not complete\noutput:\n{combined}"
    );
    assert!(
        combined.contains("library 17 cache"),
        "mock sync did not populate the cache with the 17 demo workouts\noutput:\n{combined}"
    );
    // Incremental behaviour, end to end through the worker thread: the first
    // sync saves everything, the second skips everything (fetching no
    // details) and reports the web's "caught up" result, and a full re-sync
    // ignores the cache again.
    assert!(
        combined.contains("gate resync: sync.incrementalDone added 0 skipped 17"),
        "an incremental re-sync over an unchanged library must skip every \
         detail\noutput:\n{combined}"
    );
    assert!(
        combined.contains("gate full: sync.done added 17 skipped 0"),
        "a full re-sync must re-download every detail\noutput:\n{combined}"
    );

    // Phase 5a spec R6.1/R6.2: when this walk ran with a screenshot
    // directory, each sport's replay capture must be a real render — loaded
    // equipment, not a blank or single-colour frame. This must live in the
    // same test as the walk: a separate #[test] runs in its own process
    // concurrently and reads the directory before the walk has saved the
    // captures (CI Linux, fresh checkout, fails deterministically).
    if let Some(dir) = std::env::var_os("ROWPLAY_SMOKE_SCREENSHOT_DIR") {
        for sport in ["row", "ski", "bike"] {
            let ppm = Path::new(&dir).join(format!("replay-{sport}.ppm"));
            let bytes = std::fs::read(&ppm).unwrap_or_else(|error| {
                panic!(
                    "read {}: {error} — the gate walk must reach the replay route \
                     and save the per-sport captures\n\napp log:\n{}",
                    ppm.display(),
                    common::gate_log_lines(&combined)
                )
            });
            let (width, height, pixels) = common::parse_ppm(&bytes);
            common::assert_rendered(width, height, pixels, &format!("replay-{sport}"));
        }
    }
}
