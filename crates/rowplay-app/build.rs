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

use std::fmt::Write as _;
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

/// Validates the vendored V3 rig pack against the asset contract at build
/// time (Phase 5a spec R2) and writes the name → material-role map the QML
/// material walker and the gate need into `OUT_DIR/replay_assets_meta.json`.
///
/// The scene loads through `balsam`-generated QML components (see
/// [`build_replay_balsam`]), because a `RuntimeLoader` scene is not
/// addressable from QML — no objectNames, no traversable `children`
/// (docs/qt-bridges-notes.md). The contract is enforced here on the exact
/// bytes `balsam` converts; a drift fails the build with the named slot.
/// Development (`ROWPLAY_REPLAY_ASSETS`) re-validates an on-disk pack at
/// startup, so editing the asset surfaces immediately.
fn build_replay_asset_meta(manifest_dir: &Path, out_dir: &Path) {
    let assets = manifest_dir
        .join("..")
        .join("..")
        .join("assets")
        .join("replay");
    let rigs = assets.join("rowplay-rigs-v3.glb");
    println!("cargo::rerun-if-changed={}", rigs.display());
    let bytes =
        std::fs::read(&rigs).unwrap_or_else(|error| panic!("read {}: {error}", rigs.display()));
    let library = rowplay_viewmodel::replay::glb::validate_v3(&bytes)
        .unwrap_or_else(|error| panic!("vendored V3 rig pack fails its contract: {error}"));

    let mut mesh_roles = serde_json::Map::new();
    for entry in &library.mesh_roles {
        mesh_roles.insert(
            entry.name.clone(),
            serde_json::json!({
                "role": entry.role,
                "template": entry.template,
                "slot": entry.slot,
            }),
        );
    }
    let meta = serde_json::json!({
        "byteLength": library.byte_length,
        "assetMode": "balsam",
        "templates": library.manifest.templates.iter().map(|template| serde_json::json!({
            "template": template.template,
            "partCount": template.part_count,
            "materialRoles": template.material_roles,
        })).collect::<Vec<_>>(),
        "leaves": library.leaves.iter().map(|leaf| serde_json::json!({
            "slot": leaf.slot,
            "materialRole": leaf.material_role,
        })).collect::<Vec<_>>(),
        "meshRoles": mesh_roles,
    });
    std::fs::write(
        out_dir.join("replay_assets_meta.json"),
        serde_json::to_string_pretty(&meta).expect("serialize meta"),
    )
    .expect("write replay_assets_meta.json");
}

/// Converts the vendored packs with Qt's `balsam` into QML components and
/// bundles them (plus their meshes) as a generated module
/// `RowPlay.ReplayAssets` inside a third binary resource.
///
/// `balsam`'s components name every node (`objectName`) and every Joint of
/// the athlete's skin with the contract's bone names, which is what makes
/// both the runtime material walk (role by node name) and Phase 5b posing
/// possible at all; a `RuntimeLoader` scene exposes none of that to QML.
/// Balsam also collapses each pack into its neutral placeholder material and
/// drops the `replayAsset*` extras — harmless, because roles are re-applied
/// from the validated JSON map at runtime (docs/qt-bridges-notes.md).
fn build_replay_balsam(manifest_dir: &Path, out_dir: &Path, rcc: &Path) {
    println!("cargo::rerun-if-env-changed=ROWPLAY_BALSAM");
    let balsam = find_tool("ROWPLAY_BALSAM", "balsam");
    let assets = manifest_dir
        .join("..")
        .join("..")
        .join("assets")
        .join("replay");

    // (source GLB, generated component, module subdirectory, exported type)
    const PACKS: [(&str, &str, &str, &str); 2] = [
        ("rowplay-rigs-v3.glb", "Rowplay_rigs_v3.qml", "rigs", "Rigs"),
        (
            "rowplay-athlete-v4.glb",
            "Rowplay_athlete_v4.qml",
            "athlete",
            "Athlete",
        ),
    ];

    let module_root = out_dir.join("replay-balsam");
    let _ = std::fs::remove_dir_all(&module_root);
    std::fs::create_dir_all(&module_root).expect("create replay-balsam dir");

    // Files land in OUT_DIR/replay-balsam/… but the module must sit at
    // /qt/qml/RowPlay/ReplayAssets/… for the import URI to resolve, so every
    // entry carries an alias with the module-relative resource path.
    let mut qmldir = String::from("module RowPlay.ReplayAssets\n");
    let mut qrc_entries = Vec::new();
    for (source, component, subdir, exported) in PACKS {
        let pack_out = module_root.join(subdir);
        std::fs::create_dir_all(&pack_out).expect("create balsam pack dir");
        // The V4 pack carries its three authored cycle clips; balsam would
        // turn them into auto-running `Timeline`s (plus `.qad` keyframe
        // files), but the athlete is posed from Rust — 5b evaluates the
        // clips itself (spec D2) — so Qt's animation system must not touch
        // the joints. Strip the animation component at conversion time.
        //
        // balsam is a QGuiApplication: with no display it dies initialising
        // the default platform plugin (xcb on a headless CI builder) before
        // converting anything. Conversion never needs a real window, so pin
        // the offscreen plugin — it ships with every Qt build.
        let status = Command::new(&balsam)
            .env("QT_QPA_PLATFORM", "offscreen")
            .arg("--removeComponentAnimations")
            .arg("-o")
            .arg(&pack_out)
            .arg(assets.join(source))
            .status()
            .unwrap_or_else(|error| panic!("run balsam on {source}: {error}"));
        assert!(status.success(), "balsam failed on {source}");
        let generated = pack_out.join(component);
        assert!(
            generated.is_file(),
            "balsam produced no {component} for {source}"
        );
        writeln!(qmldir, "{exported} 1.0 {subdir}/{component}").expect("format qmldir");
        qrc_entries.push(format!(
            "        <file alias=\"RowPlay/ReplayAssets/{subdir}/{component}\">replay-balsam/{subdir}/{component}</file>"
        ));
        let meshes = pack_out.join("meshes");
        if meshes.is_dir() {
            let mut files: Vec<PathBuf> = std::fs::read_dir(&meshes)
                .expect("read meshes dir")
                .map(|entry| entry.expect("mesh entry").path())
                .collect();
            files.sort();
            for file in files {
                let name = file.file_name().expect("mesh name").to_string_lossy();
                qrc_entries.push(format!(
                    "        <file alias=\"RowPlay/ReplayAssets/{subdir}/meshes/{name}\">replay-balsam/{subdir}/meshes/{name}</file>"
                ));
            }
        }
    }
    std::fs::write(module_root.join("qmldir"), qmldir).expect("write ReplayAssets qmldir");
    qrc_entries.insert(
        0,
        "        <file alias=\"RowPlay/ReplayAssets/qmldir\">replay-balsam/qmldir</file>"
            .to_owned(),
    );

    // rcc resolves <file> paths relative to the .qrc location, so the
    // generated resource lives next to the balsam output in OUT_DIR.
    let qrc = out_dir.join("rowplay_replay.qrc");
    std::fs::write(
        &qrc,
        format!(
            "<!DOCTYPE RCC>\n<RCC version=\"1.0\">\n    <qresource prefix=\"/qt/qml\">\n{}\n    </qresource>\n</RCC>\n",
            qrc_entries.join("\n")
        ),
    )
    .expect("write rowplay_replay.qrc");
    rcc_binary(rcc, &qrc, &out_dir.join("rowplay_replay.rcc"));
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
    build_replay_balsam(&manifest_dir, &out_dir, &rcc);
    build_replay_asset_meta(&manifest_dir, &out_dir);
}
