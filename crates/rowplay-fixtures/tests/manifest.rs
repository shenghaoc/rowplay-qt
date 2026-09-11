// SPDX-License-Identifier: GPL-3.0-or-later
//! The vendored fixtures must match `tests/fixtures/manifest.json` byte for byte.

use std::collections::BTreeSet;

use serde::Deserialize;

#[derive(Deserialize)]
struct Manifest {
    schema: String,
    fixtures: Vec<Entry>,
}

#[derive(Deserialize)]
struct Entry {
    path: String,
    bytes: usize,
    sha256: String,
    source: Source,
}

#[derive(Deserialize)]
struct Source {
    repository: String,
    commit: String,
    path: String,
}

#[test]
fn every_fixture_matches_its_manifest_entry() {
    let manifest: Manifest = rowplay_fixtures::load_json("manifest.json").expect("manifest");
    assert_eq!(manifest.schema, "rowplay-qt.fixtures.manifest.v1");
    assert!(!manifest.fixtures.is_empty());
    let mut seen = BTreeSet::new();
    for entry in &manifest.fixtures {
        let bytes = rowplay_fixtures::read_bytes(&entry.path)
            .unwrap_or_else(|e| panic!("{}: {e}", entry.path));
        assert_eq!(bytes.len(), entry.bytes, "{}: size", entry.path);
        assert_eq!(
            rowplay_fixtures::sha256_hex(&bytes),
            entry.sha256,
            "{}: sha256",
            entry.path
        );
        assert_eq!(
            entry.source.repository,
            "https://github.com/shenghaoc/rowplay-studio"
        );
        assert_eq!(entry.source.commit.len(), 40, "{}: commit", entry.path);
        assert!(
            entry.source.path.ends_with(&entry.path),
            "{}: source path",
            entry.path
        );
        assert!(seen.insert(entry.path.clone()), "{}: duplicate", entry.path);
    }
    // Every file on disk is listed (no unrecorded fixture can sneak in).
    let dir = rowplay_fixtures::fixtures_dir();
    let mut on_disk = BTreeSet::new();
    for entry in walk(&dir) {
        let rel = entry
            .strip_prefix(&dir)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        if rel != "manifest.json" && rel != "PROVENANCE.md" {
            on_disk.insert(rel);
        }
    }
    assert_eq!(on_disk, seen, "fixtures on disk differ from the manifest");
}

fn walk(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir).expect("read fixtures dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            files.extend(walk(&path));
        } else {
            files.push(path);
        }
    }
    files
}

#[test]
fn missing_fixtures_report_a_clear_error() {
    let err = rowplay_fixtures::read_bytes("does-not-exist.json").unwrap_err();
    assert!(matches!(err, rowplay_fixtures::FixtureError::NotFound(_)));
    assert!(err.to_string().contains("does-not-exist.json"));
}
