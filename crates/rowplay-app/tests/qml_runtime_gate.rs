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
//!
//! Hermeticity: the gate **defaults** to `offscreen` but keeps a
//! caller-provided `QT_QPA_PLATFORM` (the headless recipe uses xcb under
//! Xvfb). With an override it runs against a real window system and is
//! session-dependent — concurrent rendering apps have frozen gate walks
//! before (`docs/parity-coverage.md`, capture caveat). CI's display is
//! always clean; local runs after visual work are the risk case.
//! Recorded 2026-09-20: one per-commit-gate execution failed once and
//! passed unchanged on immediate re-run — cause **unproven**, recorded as
//! unexplained rather than dismissed as flake. Two facts narrow it: the
//! failing exec's environment had `QT_QPA_PLATFORM` unset (`.envrc` does
//! not set it), so the gate ran on the hermetic `offscreen` default and
//! the session-dependence above is *excluded* as that incident's cause;
//! and the failing step's output was not retained — the gate chain pipes
//! `cargo test` through a counting grep, which kept the failure count and
//! discarded the failing test's name and output, so a recurrence has
//! nothing to be compared against.

use std::collections::BTreeSet;
use std::fmt::Write as _;
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

/// Every app log line starts with the seconds since the app's first message,
/// so the walk can be timed from its log alone.
const MESSAGE_PATTERN: &str = "%{time process} %{if-category}%{category}: %{endif}%{message}";

/// The gate steps that open each phase of the walk (`Main.qml`'s gate timer).
const PHASES: [(u32, &str); 11] = [
    (1, "shell, languages, filters"),
    (24, "mock syncs"),
    (44, "sorting, dates, detail captures"),
    (52, "replay loads and captures"),
    (61, "tier cycling"),
    (66, "phase shots (row, ski)"),
    (77, "step-529 strip"),
    (79, "phase shots (bike)"),
    (84, "teardown"),
    (85, "bench (ROWPLAY_REPLAY_BENCH only)"),
    (200, "menus, settings over a replay, dialogs"),
];

/// Replay entry, request to first presented frame, under llvmpipe: measured
/// at 13.4-15.6 s (docs/roadmap.md). Over this the test warns, but does
/// not fail: shared CI runners have produced walks 4x and 30x slower.
const ENTRY_BUDGET_SECONDS: f64 = 30.0;

/// The walk's profile, `ROWPLAY_GATE_PROFILE`: `full` (the default and what
/// CI runs) or `quick` (the shell, all six languages, the member check, the
/// mock syncs and the first replay load). AGENTS.md, "Gate profiles".
fn gate_profile_is_quick() -> bool {
    match std::env::var("ROWPLAY_GATE_PROFILE").as_deref() {
        Err(_) | Ok("full") => false,
        Ok("quick") => true,
        Ok(other) => {
            panic!("unknown ROWPLAY_GATE_PROFILE {other:?}: use \"quick\" or \"full\"")
        }
    }
}

/// `(seconds, text)` for every log line that carries the pattern's stamp.
fn timed_lines(log: &str) -> Vec<(f64, &str)> {
    log.lines()
        .filter_map(|line| {
            let (stamp, rest) = line.trim_start().split_once(' ')?;
            Some((stamp.parse::<f64>().ok()?, rest))
        })
        .collect()
}

/// The log lines of a gate hold that gave up at its tick bound (`Main.qml`'s
/// gate timer: 60 ticks for a replay load or a scene settle, 40 for a grab).
/// A healthy walk has none, because frames keep arriving and every hold
/// releases early.
const BOUND_HITS: [&str; 3] = ["frames never settled", "load timed out", "grab timed out"];

/// The walk's timing: seconds per phase, and for every gate step whose first
/// presented frame came more than a second later, that latency (replay
/// entry is step 52; the llvmpipe IBL bake shows up as each sport switch).
/// It also names the stall signatures: holds that ran out their tick bound
/// (no frames came, as for a covered window) and the longest silence in
/// the log. A silence well past a bound's nominal 18 s means the gate timer
/// itself ran slow (timer throttling). Returns the report and the entry
/// latency, if the log carried one.
fn walk_timing(log: &str, quick: bool) -> (String, Option<f64>) {
    let lines = timed_lines(log);
    let mut steps: Vec<(f64, u32)> = Vec::new();
    let mut latencies: Vec<(u32, f64)> = Vec::new();
    let mut step_now = 0;
    let mut bound_hits: Vec<u32> = Vec::new();
    let mut silence = (0.0_f64, 0_u32);
    let mut previous: Option<f64> = None;
    for &(at, text) in &lines {
        if let Some(before) = previous {
            if at - before > silence.0 {
                silence = (at - before, step_now);
            }
        }
        previous = Some(at);
        if BOUND_HITS.iter().any(|needle| text.contains(needle)) {
            bound_hits.push(step_now);
        }
        if let Some(rest) = text.split("gate frame after step ").nth(1) {
            if let Ok(step) = rest.trim().parse::<u32>() {
                if let Some(&(started, _)) = steps.iter().rev().find(|(_, n)| *n == step) {
                    latencies.push((step, at - started));
                }
            }
        } else if let Some(rest) = text.split("gate step ").nth(1) {
            if let Ok(step) = rest.trim().parse::<u32>() {
                steps.push((at, step));
                step_now = step;
            }
        }
    }
    let Some(&(end, _)) = lines.last() else {
        return (
            "gate timing: the log carried no timestamps".to_owned(),
            None,
        );
    };
    let profile = if quick { "quick" } else { "full" };
    let mut report = String::new();
    let _ = writeln!(
        report,
        "gate walk ({profile} profile): {end:.1} s of app log"
    );
    for (index, (first, name)) in PHASES.iter().enumerate() {
        let next = PHASES.get(index + 1).map_or(u32::MAX, |(step, _)| *step);
        let Some(&(start, _)) = steps.iter().find(|(_, n)| (*first..next).contains(n)) else {
            continue;
        };
        let stop = steps
            .iter()
            .find(|(_, n)| *n >= next)
            .map_or(end, |(at, _)| *at);
        let _ = writeln!(report, "  {name:40} {:7.1} s", stop - start);
    }
    let entry = latencies
        .iter()
        .find(|(step, _)| *step == 52)
        .map(|(_, seconds)| *seconds);
    if let Some(entry) = entry {
        let _ = writeln!(
            report,
            "  replay entry, step 52 to first frame: {entry:.1} s \
             (budget {ENTRY_BUDGET_SECONDS:.0} s)"
        );
    }
    let slow: Vec<String> = latencies
        .iter()
        .filter(|(_, seconds)| *seconds > 1.0)
        .map(|(step, seconds)| format!("{step} ({seconds:.1} s)"))
        .collect();
    if !slow.is_empty() {
        let _ = writeln!(
            report,
            "  steps whose first frame took over 1 s: {}",
            slow.join(", ")
        );
    }
    if !bound_hits.is_empty() {
        let hits: Vec<String> = bound_hits.iter().map(u32::to_string).collect();
        let _ = writeln!(
            report,
            "  holds that ran out their tick bound: {} (steps {})",
            hits.len(),
            hits.join(", ")
        );
    }
    let _ = writeln!(
        report,
        "  longest silence in the app log: {:.1} s, during step {}",
        silence.0, silence.1
    );
    (report, entry)
}

/// The timing summary reads its numbers from the log alone; a synthetic log
/// with a starved settle checks every line of it without launching the app.
#[test]
fn walk_timing_reads_phases_entry_and_stall_signatures() {
    let log = "\
     0.500 qml: gate profile: full
     0.500 qml: gate step 1
     0.520 qml: gate frame after step 1
     1.000 qml: gate step 24
    10.000 qml: gate step 44
    12.000 qml: gate step 52
    25.500 qml: gate frame after step 52
    26.000 qml: gate step 53
    26.100 qml: gate scene: settling replay-row from 184 frames
    44.100 qml: gate scene: frames never settled, grabbing anyway
    45.000 qml: gate step 84
    46.000 qml: gate step 85
    47.000 qml: gate bench: done
";
    let (report, entry) = walk_timing(log, false);
    assert_eq!(entry, Some(13.5), "{report}");
    for expected in [
        "gate walk (full profile): 47.0 s of app log",
        "mock syncs                                   9.0 s",
        "replay loads and captures                   33.0 s",
        "replay entry, step 52 to first frame: 13.5 s (budget 30 s)",
        "steps whose first frame took over 1 s: 52 (13.5 s)",
        "holds that ran out their tick bound: 1 (steps 53)",
        "longest silence in the app log: 18.0 s, during step 53",
    ] {
        assert!(
            report.contains(expected),
            "missing {expected:?} in\n{report}"
        );
    }
}

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

/// `Font.TabularNumbers` is not a Qt 6.11 enum. It reads as `undefined`, and
/// a font object (`font: ({ …, features: Font.TabularNumbers })`) drops the
/// undefined member without a word, so the text loses its tabular figures
/// where the walk below cannot see it (docs/qt-bridges-notes.md). Tabular
/// digits are the OpenType tag: `features: { "tnum": 1 }`.
#[test]
fn qml_names_no_font_tabular_numbers_enum() {
    let mut files = Vec::new();
    qml_files(&repo_root().join("qml"), &mut files);
    assert!(!files.is_empty(), "no QML files found");
    let offenders: Vec<String> = files
        .iter()
        .filter(|path| {
            let source = std::fs::read_to_string(path)
                .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
            strip_comments(&source).contains("Font.TabularNumbers")
        })
        .map(|path| path.display().to_string())
        .collect();
    assert!(
        offenders.is_empty(),
        "Font.TabularNumbers is undefined in Qt 6.11 and a font object drops it \
         silently; use `features: {{ \"tnum\": 1 }}`:\n{}",
        offenders.join("\n")
    );
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
    let quick = gate_profile_is_quick();
    let mut command = Command::new(env!("CARGO_BIN_EXE_rowplay-app"));
    command
        .env("ROWPLAY_SMOKE_GATE", "1")
        .env("QT_MESSAGE_PATTERN", MESSAGE_PATTERN)
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
    // Keep the whole log and the walk's timing before any assertion can
    // fail: a slow or failing walk is only diagnosable from them (CI
    // uploads the log; a passing walk used to leave nothing behind).
    if let Some(dir) = std::env::var_os("ROWPLAY_SMOKE_ARTIFACT_DIR") {
        std::fs::create_dir_all(&dir).expect("create artifact directory");
        std::fs::write(Path::new(&dir).join("gate-log.txt"), &combined).expect("save the gate log");
    }
    let (timing, entry) = walk_timing(&combined, quick);
    eprintln!("{timing}");
    if let Some(entry) = entry {
        if entry > ENTRY_BUDGET_SECONDS {
            eprintln!(
                "::warning title=Replay entry over budget::the first replay entry took \
                 {entry:.1} s, over the {ENTRY_BUDGET_SECONDS:.0} s llvmpipe budget \
                 (docs/roadmap.md); informational, not a failure"
            );
        }
    }
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

    // Settings opened over a replay close it on the way in: Back then lands
    // on the workout with no replay left presented, and Replay opens again
    // instead of being refused as already presented (block reason 4).
    assert!(
        combined.contains("gate settings over the replay: screen 1 closed"),
        "settings opened over a replay must close it, and Back return to the \
         workout\n\napp log:\n{}",
        common::gate_log_lines(&combined)
    );
    assert!(
        combined.contains("gate replay reopened: screen 3 block 0"),
        "Replay must open again after settings closed it\n\napp log:\n{}",
        common::gate_log_lines(&combined)
    );

    // Phase 5b: structural equipment inventory check. The scene logs
    // "replay equipment: N of M" after each applySceneRules. The expected
    // counts are fixed here from the V3 contract's anchor table — the app's
    // own count is the thing under test, so the oracle must be external.
    //
    //   RowErg:  1 boat + 2 oars + 2 blades + 1 seat = 6
    //   SkiErg:  2 skis + 2×3 pole-parts             = 8
    //   BikeErg: 1 frame + 1 drivetrain + 2 wheels    = 4
    let expected_equipment: [(&str, usize); 3] = [("row", 6), ("ski", 8), ("bike", 4)];
    for (sport, expected) in expected_equipment {
        // "replay equipment row: 5 of 5" — filter to this sport's lines.
        let sport_needle = format!("replay equipment {sport}:");
        let ok = combined
            .lines()
            .filter(|line| line.contains(&sport_needle))
            .all(|line| {
                let rest = line.split(&sport_needle).nth(1).unwrap_or("");
                let present: usize = rest
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                present >= expected
            });
        assert!(
            ok,
            "replay equipment inventory for {sport}: expected at least {expected} \
             nodes in every \"{sport_needle}\" log line\n\napp log:\n{}",
            common::gate_log_lines(&combined)
        );
    }

    // Phase 7: finger grip contacts. The scene logs
    // "replay grip <sport>: N/M digit contacts" from the applied table after
    // each applySceneRules; the vendored closures are full-contact (5 digits
    // × 2 hands), so every line must read N == M == 10, and a FAILED line
    // anywhere fails the walk — an unsolved hand must fail loudly.
    assert!(
        !combined.contains("replay grip FAILED"),
        "a sport's finger helpers did not resolve\n\napp log:\n{}",
        common::gate_log_lines(&combined)
    );
    // The quick profile loads one replay (the rower); the full walk loads
    // all three sports.
    let sports: &[&str] = if quick {
        &["row"]
    } else {
        &["row", "ski", "bike"]
    };
    for sport in sports {
        let grip_needle = format!("replay grip {sport}:");
        let lines: Vec<&str> = combined
            .lines()
            .filter(|line| line.contains(&grip_needle))
            .collect();
        assert!(
            !lines.is_empty(),
            "no \"{grip_needle}\" log line — the grip walk did not run for {sport}\n\napp log:\n{}",
            common::gate_log_lines(&combined)
        );
        for line in lines {
            let rest = line.split(&grip_needle).nth(1).unwrap_or("");
            let count = rest.split_whitespace().next().unwrap_or("");
            let (contacted, total) = count
                .split_once('/')
                .and_then(|(n, m)| Some((n.parse::<usize>().ok()?, m.parse::<usize>().ok()?)))
                .unwrap_or((usize::MAX, 0));
            assert!(
                contacted == total && total == 10,
                "replay grip contacts for {sport}: expected 10/10 digit contacts, \
                 got \"{count}\" in {line}\n\napp log:\n{}",
                common::gate_log_lines(&combined)
            );
        }
    }
    // Phase 5c: per-tier texture set assertion. The gate cycles through all
    // four quality tiers on the rower scene. The expected texture set counts
    // are from the environments README table, hardcoded here as the oracle.
    let expected_textures: [(&str, usize); 4] =
        [("low", 0), ("medium", 0), ("high", 8), ("ultra", 8)];
    for (tier, expected_count) in expected_textures {
        let tier_needle = format!("replay textures {tier}:");
        let ok = combined
            .lines()
            .filter(|l| l.contains(&tier_needle))
            .all(|l| {
                let rest = l.split(&tier_needle).nth(1).unwrap_or("");
                let count: usize = rest
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(usize::MAX);
                count == expected_count
            });
        assert!(
            ok,
            "replay texture inventory for tier {tier}: expected {expected_count} \
             sets in every \"{tier_needle}\" log line\n\napp log:\n{}",
            common::gate_log_lines(&combined)
        );
    }

    // Phase 6b: venue inventory. The scene logs one line per venue load with
    // the structural counts, and one per tier texture readback. The oracle is
    // the vendored contracts themselves (external to the app), keyed by
    // (sport, tier): every venue line must exactly match the tier it was
    // served at, a FAILED line anywhere fails the walk — a venue that does
    // not load must fail the gate, not fall back to the bare ground plane.
    let venue_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
        .join("replay")
        .join("venues");
    let venue_inventory = |sport: &str| -> Vec<[usize; 4]> {
        ["low", "medium", "high", "ultra"]
            .iter()
            .map(|tier| {
                let text = std::fs::read_to_string(
                    venue_dir.join(format!("rowplay-venue-{sport}-{tier}.json")),
                )
                .unwrap_or_else(|error| {
                    panic!("read vendored venue contract {sport}/{tier}: {error}")
                });
                let contract: serde_json::Value =
                    serde_json::from_str(&text).expect("vendored venue contract is valid JSON");
                let inventory = &contract["inventory"];
                [
                    inventory["nodes"].as_u64().expect("nodes") as usize,
                    contract["instancing"]
                        .as_object()
                        .map_or(0, serde_json::Map::len),
                    inventory["instances"].as_u64().expect("instances") as usize,
                    inventory["materials"].as_u64().expect("materials") as usize,
                ]
            })
            .collect()
    };
    assert!(
        !combined.contains("replay venue FAILED"),
        "a venue failed to load; there is no silent fallback\n\napp log:\n{}",
        common::gate_log_lines(&combined)
    );
    for (sport, tag) in [("rower", "row"), ("skierg", "ski"), ("bike", "bike")] {
        let expected = venue_inventory(sport);
        let needle = format!("replay venue {tag}:");
        let lines: Vec<&str> = combined
            .lines()
            .filter(|line| line.contains(&needle))
            .collect();
        // Every venue the walk loads must match a tier inventory, and every
        // sport the profile loads must load one (the quick walk loads only
        // the rower).
        assert!(
            (quick && tag != "row") || !lines.is_empty(),
            "no venue log for {sport}; the walk must load a venue per sport\n\napp log:\n{}",
            common::gate_log_lines(&combined)
        );
        // The first rower entry has its venue: it must load before the first
        // sport switch (step 55, SkiErg). Until #84 it came only from the
        // later rower + ghost entry, after a switch had set a plan.
        if tag == "row" {
            let first_venue = combined.lines().position(|line| line.contains(&needle));
            let first_switch = combined
                .lines()
                .position(|line| line.trim_end().ends_with("gate step 55"));
            assert!(
                matches!((first_venue, first_switch), (Some(venue), Some(switch)) if venue < switch)
                    || (first_venue.is_some() && first_switch.is_none()),
                "the first rower entry has no venue: none loaded before the first sport switch\n\napp log:\n{}",
                common::gate_log_lines(&combined)
            );
        }
        for line in &lines {
            let rest = line.split(&needle).nth(1).unwrap_or("");
            let numbers: Vec<usize> = rest
                .split(|c: char| !c.is_ascii_digit())
                .filter(|part| !part.is_empty())
                .filter_map(|part| part.parse().ok())
                .collect();
            assert!(
                expected.iter().any(|inv| inv == numbers.as_slice()),
                "{sport} venue load {numbers:?} matches no tier inventory {expected:?}\n\n\
                 app log:\n{}",
                common::gate_log_lines(&combined)
            );
        }
    }

    // Per-tier venue texture counts. The tier cycle runs on the rower scene;
    // ski and bike log once each at the walk's default tier (medium). The
    // expected counts are the texture bindings the contract carries at that
    // tier (Low binds none — the environments README rule; Ultra's normal
    // bindings only exist in the Ultra contracts, matching the resolver's
    // normalMaps flag). Texture *objects* are shared across variants by
    // source; the logged count is the bindings a walk applied.
    let venue_texture_counts = |sport: &str| -> Vec<(String, usize)> {
        ["low", "medium", "high", "ultra"]
            .iter()
            .map(|tier| {
                let text = std::fs::read_to_string(
                    venue_dir.join(format!("rowplay-venue-{sport}-{tier}.json")),
                )
                .expect("venue contract");
                let contract: serde_json::Value =
                    serde_json::from_str(&text).expect("venue contract JSON");
                let bindings: usize = contract["materials"]
                    .as_object()
                    .expect("materials")
                    .values()
                    .filter_map(|material| material["textures"].as_object())
                    .map(serde_json::Map::len)
                    .sum();
                (tier.to_string(), bindings)
            })
            .collect()
    };
    for (sport, tag) in [("rower", "row"), ("skierg", "ski"), ("bike", "bike")] {
        let expected = venue_texture_counts(sport);
        let needle = format!("replay venue {tag} textures ");
        for line in combined.lines().filter(|line| line.contains(&needle)) {
            let rest = line.split(&needle).nth(1).unwrap_or("");
            let mut parts = rest.split_whitespace();
            let tier = parts.next().unwrap_or("").trim_end_matches(':');
            let count: usize = parts
                .next()
                .and_then(|c| c.parse().ok())
                .unwrap_or(usize::MAX);
            let expected_count = expected.iter().find(|(name, _)| name == tier).map_or_else(
                || panic!("{tag}: unknown venue tier {tier}"),
                |(_, count)| *count,
            );
            assert_eq!(
                count,
                expected_count,
                "{sport} venue textures at {tier}: expected {expected_count} unique \
                 sources\n\napp log:\n{}",
                common::gate_log_lines(&combined)
            );
        }
    }

    // Phase 5a spec R6.1/R6.2: when this walk ran with a screenshot
    // directory, each sport's replay capture must be a real render — loaded
    // equipment, not a blank or single-colour frame. This must live in the
    // same test as the walk: a separate #[test] runs in its own process
    // concurrently and reads the directory before the walk has saved the
    // captures (CI Linux, fresh checkout, fails deterministically).
    if let Some(dir) = std::env::var_os("ROWPLAY_SMOKE_SCREENSHOT_DIR") {
        for sport in sports {
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
            let label = format!("replay-{sport}");
            common::assert_rendered(width, height, pixels, &label);
            // Phase 5b: shadows must be visible on the ground plane.
            // Only checked under a real GL backend — the offscreen QPA
            // does not render View3D content, so its captures are flat.
            if std::env::var("QSG_RHI_BACKEND").is_ok() {
                common::assert_shadows(width, height, pixels, &label);
            }
        }
    }

    // Phase 7 T8 baseline: with ROWPLAY_PHASE_SHOTS=1 the walk captures
    // catch / mid-drive / finish / mid-recovery per sport (ghost loaded),
    // at deterministic mid-workout stroke fractions. Every shot must be a
    // real render — the visual baseline future budget/weight changes
    // compare against.
    if !quick && std::env::var("ROWPLAY_PHASE_SHOTS").is_ok() {
        if let Some(dir) = std::env::var_os("ROWPLAY_SMOKE_SCREENSHOT_DIR") {
            for sport in ["row", "ski", "bike"] {
                for phase in ["catch", "middrive", "finish", "midrecovery"] {
                    let name = format!("phase-{sport}-{phase}");
                    let ppm = Path::new(&dir).join(format!("{name}.ppm"));
                    let bytes = std::fs::read(&ppm).unwrap_or_else(|error| {
                        panic!(
                            "read {}: {error} — the phase-shot walk must save {name}\n\napp log:\n{}",
                            ppm.display(),
                            common::gate_log_lines(&combined)
                        )
                    });
                    let (width, height, pixels) = common::parse_ppm(&bytes);
                    common::assert_rendered(width, height, pixels, &name);
                    // The whole-window check passes on a black 3D viewport
                    // (the sidebar supplies the colours), which is exactly how
                    // a capture path that returns no scene went unnoticed:
                    // macOS/Metal `grabToImage` yields a black viewport while
                    // the live window renders correctly (qt-bridges-notes #16),
                    // so assert the viewport region itself — under a real GL
                    // capture backend, like the shadow check.
                    if std::env::var("QSG_RHI_BACKEND").is_ok() {
                        common::assert_viewport_rendered(width, height, pixels, &name);
                    }
                }
            }
        }
    }
}
