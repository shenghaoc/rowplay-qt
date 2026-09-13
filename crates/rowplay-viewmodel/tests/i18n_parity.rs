// SPDX-License-Identifier: GPL-3.0-or-later
//! Qt-free i18n parity checks (Phase 4, spec R7.5), run in CI:
//!
//! 1. all six committed `i18n/rowplay_*.ts` catalogues have exactly the web
//!    `en` key set (908 web + 1 desktop supplement = 909 ids) and identical
//!    `<source>` texts and `{name}`
//!    placeholder sets per id;
//! 2. every translation id used by a literal `Tr.t("…")` call in `qml/`
//!    exists in that key set;
//! 3. when the pinned web checkout (`reference/rowplay`) and Node are
//!    available, `tools/convert-locales.mjs --check` confirms the committed
//!    catalogues match a fresh regeneration (skipped in CI, which has
//!    neither).
//!
//! The `.ts` files are generated deterministically by
//! `tools/convert-locales.mjs`, so a small hand-rolled scanner is enough —
//! no XML dependency in the Qt-free workspace.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

const LANGUAGES: [&str; 6] = ["en", "zh", "de", "es", "fr", "ja"];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root")
}

fn unescape(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

/// Extracts `(id, oldsource, translation)` triples from a generated `.ts`
/// file. `<source>` is empty in the canonical ID-based shape (lrelease only
/// embeds the id lookup then); the English text lives in `<oldsource>`.
/// Values may span lines (the docs markdown), so the scanner works on the
/// whole text between `<message …>` and `</message>`.
fn parse_ts(path: &Path) -> Vec<(String, String, String)> {
    let text =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let mut messages = Vec::new();
    let mut rest = text.as_str();
    while let Some(start) = rest.find("<message id=\"") {
        rest = &rest[start + "<message id=\"".len()..];
        let id_end = rest.find('"').expect("message id closes");
        let id = rest[..id_end].to_owned();
        rest = &rest[id_end..];
        let end = rest.find("</message>").expect("message closes");
        let block = &rest[..end];
        rest = &rest[end..];

        let source = extract_element(block, "source").unwrap_or_default();
        assert!(
            source.is_empty(),
            "{}: <source> must be empty for id-based lookup (got {source:?})",
            path.display()
        );
        let oldsource = extract_element(block, "oldsource").unwrap_or_default();
        let translation = extract_element(block, "translation").unwrap_or_default();
        messages.push((unescape(&id), unescape(&oldsource), unescape(&translation)));
    }
    messages
}

fn extract_element(block: &str, name: &str) -> Option<String> {
    let open = format!("<{name}>");
    let close = format!("</{name}>");
    let start = block.find(&open)? + open.len();
    let end = start + block[start..].find(&close)?;
    Some(block[start..end].to_owned())
}

/// `{name}` placeholder tokens in a message, web `interpolate` syntax.
fn placeholders(text: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        rest = &rest[open + 1..];
        match rest.find('}') {
            Some(close) => {
                let name = &rest[..close];
                if !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                    found.insert(name.to_owned());
                }
                rest = &rest[close + 1..];
            }
            None => break,
        }
    }
    found
}

fn all_qml_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("read qml dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            all_qml_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "qml") {
            out.push(path);
        }
    }
}

#[test]
fn every_catalogue_has_exactly_the_english_key_set() {
    let i18n = repo_root().join("i18n");
    let english: BTreeMap<String, String> = parse_ts(&i18n.join("rowplay_en.ts"))
        .into_iter()
        .map(|(id, source, _)| (id, source))
        .collect();
    // 908 web keys + 1 desktop supplement (settings.reduceMotion, added by
    // tools/convert-locales.mjs DESKTOP_SUPPLEMENT because the web has no
    // reduce-motion control).
    assert_eq!(
        english.len(),
        909,
        "expected 908 web keys + 1 desktop supplement = 909"
    );

    for language in LANGUAGES {
        let path = i18n.join(format!("rowplay_{language}.ts"));
        let messages = parse_ts(&path);
        assert_eq!(
            messages.len(),
            english.len(),
            "{language}: message count diverges from en"
        );
        for (id, oldsource, translation) in &messages {
            let Some(english_source) = english.get(id) else {
                panic!("{language}: id not in the en key set: {id}");
            };
            assert_eq!(
                oldsource, english_source,
                "{language}: <oldsource> for {id} must equal the English text"
            );
            assert!(
                !translation.is_empty(),
                "{language}: empty translation for {id} (missing keys must be \
                 filled with the English value, matching the web fallback)"
            );
            // Placeholder parity is one-directional, like the web: a
            // translation may drop a placeholder (the web fr
            // `dashboard.emptyTrend` hardcodes "Une seule séance" instead of
            // "{n} session" — see docs/source-map.md), but a placeholder the
            // English source lacks would never be substituted and would show
            // as a literal "{name}" bug.
            let extra = placeholders(translation)
                .difference(&placeholders(oldsource))
                .cloned()
                .collect::<Vec<_>>();
            assert!(
                extra.is_empty(),
                "{language}: translation for {id} has placeholders absent \
                 from the English source: {extra:?}"
            );
        }
    }
}

#[test]
fn every_translation_id_used_in_qml_exists() {
    let i18n = repo_root().join("i18n");
    let ids: BTreeSet<String> = parse_ts(&i18n.join("rowplay_en.ts"))
        .into_iter()
        .map(|(id, _, _)| id)
        .collect();

    // Literal `Tr.t("some.id"` / `.t("some.id"` call sites. Dynamic ids are
    // not allowed (spec R7.5): the scanner only recognises literals, and a
    // dynamic call would silently escape the check — hence the strict
    // follow-character assertion below.
    let pattern = regex::Regex::new(r#"\.t\(\s*"([A-Za-z0-9_.]+)"\s*[,)]"#).expect("regex");
    let mut qml_files = Vec::new();
    all_qml_files(&repo_root().join("qml"), &mut qml_files);
    assert!(!qml_files.is_empty(), "no QML files found");

    let mut checked = 0usize;
    for path in qml_files {
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        for captures in pattern.captures_iter(&text) {
            let id = &captures[1];
            assert!(
                ids.contains(id),
                "{}: Tr.t id \"{id}\" is not in the web key set",
                path.display()
            );
            checked += 1;
        }
    }
    assert!(
        checked > 0,
        "no Tr.t(…) ids found in qml/ — the scanner pattern is stale"
    );
}

#[test]
fn committed_catalogues_match_a_fresh_regeneration() {
    let root = repo_root();
    let locales = root.join("reference/rowplay/src/lib/locales");
    if !locales.is_dir() {
        eprintln!("skipping regeneration check: no reference/rowplay checkout (CI)");
        return;
    }
    let node = Command::new("node").arg("--version").output();
    if node.is_err() {
        eprintln!("skipping regeneration check: node is not installed");
        return;
    }
    let status = Command::new("node")
        .arg(root.join("tools/convert-locales.mjs"))
        .arg("--check")
        .current_dir(&root)
        .status()
        .expect("run convert-locales.mjs --check");
    assert!(
        status.success(),
        "i18n/rowplay_*.ts are stale: rerun `node tools/convert-locales.mjs`"
    );
}
