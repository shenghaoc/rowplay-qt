// SPDX-License-Identifier: GPL-3.0-or-later
//! Compiles `qml/rowplay.qrc` into a binary Qt resource with `rcc --binary`,
//! and (Phase 4) compiles the `i18n/rowplay_<lang>.ts` catalogues with
//! `lrelease` into a second binary resource that places `qml_<lang>.qm` at
//! `:/qt/qml/RowPlay/i18n/`, where `QQmlApplicationEngine` picks translations
//! up automatically when `Qt.uiLanguage` changes.
//!
//! qtbridge's `include_bytes_qml!` embeds one file per call as a byte-literal
//! token stream, which does not scale to `.glb` assets; `rcc --binary` plus
//! `qtbridge::qresource::register_bytes` keeps the standard Qt resource
//! pipeline (and `qmllint`/`qmlls` working on plain directories). No C++ is
//! generated or compiled here. See docs/qt-bridges-notes.md.

use std::path::{Path, PathBuf};
use std::process::Command;

fn qmake() -> String {
    std::env::var("QMAKE").unwrap_or_else(|_| "qmake".to_owned())
}

fn qmake_query(key: &str) -> Option<PathBuf> {
    let output = Command::new(qmake()).arg("-query").arg(key).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if value.is_empty() {
        None
    } else {
        Some(PathBuf::from(value))
    }
}

/// Finds a Qt tool (`rcc`, `lrelease`) through the qmake install directories.
fn find_tool(env_override: &str, tool: &str) -> PathBuf {
    if let Some(path) = std::env::var_os(env_override) {
        return PathBuf::from(path);
    }
    let name = if cfg!(windows) {
        format!("{tool}.exe")
    } else {
        tool.to_owned()
    };
    for key in [
        "QT_INSTALL_LIBEXECS",
        "QT_INSTALL_BINS",
        "QT_HOST_LIBEXECS",
        "QT_HOST_BINS",
    ] {
        if let Some(dir) = qmake_query(key) {
            let candidate = dir.join(&name);
            if candidate.is_file() {
                return candidate;
            }
        }
    }
    panic!(
        "rowplay-app: could not find Qt's {tool} tool. Put qmake on PATH (or set QMAKE), \
         or point {env_override} at the {tool} executable."
    );
}

fn find_rcc() -> PathBuf {
    find_tool("ROWPLAY_RCC", "rcc")
}

fn find_lrelease() -> PathBuf {
    find_tool("ROWPLAY_LRELEASE", "lrelease")
}

fn rcc_binary(rcc: &Path, qrc: &Path, out: &Path) {
    let status = Command::new(rcc)
        .arg("--binary")
        .arg("--no-compress")
        .arg("-o")
        .arg(out)
        .arg(qrc)
        .status()
        .unwrap_or_else(|e| panic!("run rcc --binary on {}: {e}", qrc.display()));
    assert!(
        status.success(),
        "rcc --binary failed for {}",
        qrc.display()
    );
}

/// Compiles every `i18n/rowplay_<lang>.ts` with `lrelease` into
/// `OUT_DIR/i18n/qml_<lang>.qm`, then bundles them with `rcc` under the
/// prefix `QQmlApplicationEngine` searches when `Qt.uiLanguage` is set.
fn build_i18n_resources(rcc: &Path, out_dir: &Path) {
    println!("cargo::rerun-if-env-changed=ROWPLAY_LRELEASE");
    let i18n_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("i18n");
    let i18n_dir = i18n_dir.canonicalize().unwrap_or(i18n_dir);

    let mut languages = Vec::new();
    for entry in std::fs::read_dir(&i18n_dir).expect("read i18n/") {
        let path = entry.expect("i18n dir entry").path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let Some(language) = name
            .strip_prefix("rowplay_")
            .and_then(|rest| rest.strip_suffix(".ts"))
        else {
            continue;
        };
        println!("cargo::rerun-if-changed={}", path.display());
        languages.push(language.to_owned());
    }
    languages.sort();
    assert!(
        !languages.is_empty(),
        "no i18n/rowplay_<lang>.ts catalogues found; regenerate them with tools/convert-locales.mjs"
    );

    let lrelease = find_lrelease();
    let qm_dir = out_dir.join("i18n");
    std::fs::create_dir_all(&qm_dir).expect("create OUT_DIR/i18n");

    let mut qrc_entries = Vec::new();
    for language in &languages {
        let ts = i18n_dir.join(format!("rowplay_{language}.ts"));
        let qm = qm_dir.join(format!("qml_{language}.qm"));
        let status = Command::new(&lrelease)
            .arg(&ts)
            .arg("-qm")
            .arg(&qm)
            .status()
            .expect("run lrelease");
        assert!(status.success(), "lrelease failed for {}", ts.display());
        qrc_entries.push(format!("        <file>i18n/qml_{language}.qm</file>"));
    }

    // rcc resolves <file> paths relative to the .qrc location, so the
    // generated catalogue lives next to the compiled qm files in OUT_DIR.
    let qrc = out_dir.join("rowplay_i18n.qrc");
    std::fs::write(
        &qrc,
        format!(
            "<!DOCTYPE RCC>\n<RCC version=\"1.0\">\n    <qresource prefix=\"/qt/qml/RowPlay\">\n{}\n    </qresource>\n</RCC>\n",
            qrc_entries.join("\n")
        ),
    )
    .expect("write rowplay_i18n.qrc");
    rcc_binary(rcc, &qrc, &out_dir.join("rowplay_i18n.rcc"));
}

fn main() {
    println!("cargo::rerun-if-env-changed=QMAKE");
    println!("cargo::rerun-if-env-changed=ROWPLAY_RCC");
    let manifest_dir =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let qrc = manifest_dir
        .join("..")
        .join("..")
        .join("qml")
        .join("rowplay.qrc");
    let qrc = qrc.canonicalize().unwrap_or(qrc);
    println!("cargo::rerun-if-changed={}", qrc.display());

    let rcc = find_rcc();
    let listed = Command::new(&rcc)
        .arg("--list")
        .arg(&qrc)
        .output()
        .expect("run rcc --list");
    assert!(
        listed.status.success(),
        "rcc --list failed: {}",
        String::from_utf8_lossy(&listed.stderr)
    );
    for line in String::from_utf8_lossy(&listed.stdout).lines() {
        let path = Path::new(line.trim());
        if !path.as_os_str().is_empty() {
            println!("cargo::rerun-if-changed={}", path.display());
        }
    }

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    rcc_binary(&rcc, &qrc, &out_dir.join("rowplay.rcc"));
    build_i18n_resources(&rcc, &out_dir);
}
