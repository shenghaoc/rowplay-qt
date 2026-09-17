// SPDX-License-Identifier: GPL-3.0-or-later
//! Every float literal in every committed fixture must read back bit-identical
//! through the parser the tests use.
//!
//! The property, not a symptom: a fixture's numbers are the bytes the
//! generator wrote, and whatever double the generator held, the decimal text
//! on disk has exactly one nearest double — so the parser used by the parity
//! tests must produce it. Asserting this over the committed fixtures (rather
//! than over specific literals) means the pin survives a serialiser change, a
//! new fixture format and new fixtures: any float-bearing file that fails to
//! round-trip fails this test by construction.
//!
//! The comparison oracle is Rust's std `str::parse::<f64>()`, which is
//! correctly rounded (Eisel-Lemire). serde_json equals it only with the
//! `float_roundtrip` feature: the default fast parser computes
//! significand·/÷10^k in one rounding and lands on the neighbouring double
//! for a fraction of long literals — measured at 1709/19007 in the rig-phase
//! corpus, 1954/19485 motion, 232/1469 stroke-model and three more (4283
//! literals total; the mis-rounded class needs ~16+ significant digits, which
//! is why the short-decimal Concept2 payloads parse identically either way).
//! Every parity comparison in this repo ran on those perturbed inputs until
//! the feature was enabled; the first visible failure was a stroke-model
//! sample whose `cycleFrac` arrived exactly at the 0.5 drive seam instead of
//! just below it. If this test fails, the feature was dropped and the parity
//! suites are feeding the ports silently perturbed inputs again.

use std::path::{Path, PathBuf};

/// Lex JSON text and yield every literal that denotes a float (has a
/// fractional part or an exponent). Integer literals are skipped: they are
/// exactly representable (or uniformly rounded) under any parser, and the
/// property under test is about the fast path's decimal scaling.
fn float_literals(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut literals = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => {
                // Skip the string (SHA-256 hex, ids, schema names) including
                // escaped quotes, so digits inside it are never lexed.
                i += 1;
                while i < bytes.len() && bytes[i] != b'"' {
                    if bytes[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
                i += 1;
            }
            c if c == b'-' || c.is_ascii_digit() => {
                let start = i;
                if bytes[i] == b'-' {
                    i += 1;
                }
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
                let mut is_float = false;
                if i < bytes.len() && bytes[i] == b'.' {
                    is_float = true;
                    i += 1;
                    while i < bytes.len() && bytes[i].is_ascii_digit() {
                        i += 1;
                    }
                }
                if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
                    let mut j = i + 1;
                    if j < bytes.len() && (bytes[j] == b'+' || bytes[j] == b'-') {
                        j += 1;
                    }
                    if j < bytes.len() && bytes[j].is_ascii_digit() {
                        is_float = true;
                        i = j;
                        while i < bytes.len() && bytes[i].is_ascii_digit() {
                            i += 1;
                        }
                    }
                }
                if is_float {
                    literals.push(&text[start..i]);
                }
            }
            _ => i += 1,
        }
    }
    literals
}

fn json_files_under(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(path) = stack.pop() {
        let entries: Vec<PathBuf> = std::fs::read_dir(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
            .map(|entry| entry.expect("dir entry").path())
            .collect();
        for entry in entries {
            if entry.is_dir() {
                stack.push(entry);
            } else if entry.extension().is_some_and(|ext| ext == "json") {
                files.push(entry);
            }
        }
    }
    files.sort();
    files
}

#[test]
fn every_fixture_float_literal_parses_to_its_nearest_double() {
    let mut checked = 0_usize;
    for path in json_files_under(&rowplay_fixtures::fixtures_dir()) {
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        for literal in float_literals(&text) {
            let via_serde: f64 = serde_json::from_str(literal)
                .unwrap_or_else(|e| panic!("{}: {literal}: {e}", path.display()));
            let nearest: f64 = literal
                .parse()
                .unwrap_or_else(|e| panic!("{}: {literal}: {e}", path.display()));
            assert_eq!(
                via_serde.to_bits(),
                nearest.to_bits(),
                "{}: literal {literal} parses 1 ULP off its nearest double — \
                 serde_json's float_roundtrip feature is not in effect",
                path.display()
            );
            checked += 1;
        }
    }
    // The scan must actually exercise the corpora; a fixtures-dir relocation
    // that silently scans nothing would otherwise pass vacuously.
    assert!(
        checked > 10_000,
        "only {checked} float literals found under the fixtures dir — the scan is broken"
    );
}
