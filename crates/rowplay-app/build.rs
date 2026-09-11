// SPDX-License-Identifier: GPL-3.0-or-later
//! Compiles `qml/rowplay.qrc` into a binary Qt resource with `rcc --binary`.
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

fn find_rcc() -> PathBuf {
    if let Some(path) = std::env::var_os("ROWPLAY_RCC") {
        return PathBuf::from(path);
    }
    let name = if cfg!(windows) { "rcc.exe" } else { "rcc" };
    for key in [
        "QT_INSTALL_LIBEXECS",
        "QT_INSTALL_BINS",
        "QT_HOST_LIBEXECS",
        "QT_HOST_BINS",
    ] {
        if let Some(dir) = qmake_query(key) {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return candidate;
            }
        }
    }
    panic!(
        "rowplay-app: could not find Qt's rcc tool. Put qmake on PATH (or set QMAKE), \
         or point ROWPLAY_RCC at the rcc executable."
    );
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

    let out = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR")).join("rowplay.rcc");
    let status = Command::new(&rcc)
        .arg("--binary")
        .arg("--no-compress")
        .arg("-o")
        .arg(&out)
        .arg(&qrc)
        .status()
        .expect("run rcc --binary");
    assert!(
        status.success(),
        "rcc --binary failed for {}",
        qrc.display()
    );
}
