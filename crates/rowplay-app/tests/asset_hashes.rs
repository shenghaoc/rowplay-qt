// SPDX-License-Identifier: GPL-3.0-or-later
//! Pins the vendored replay assets (Phase 5a, spec R1.4).
//!
//! Every byte under `assets/replay/` is a byte-for-byte copy of a reviewed
//! upstream artifact; the table below (regenerable with
//! `tools/vendor-replay-assets.py --emit-rust`) asserts size and SHA-256 per
//! file, and the walk at the end fails on any file the table does not know,
//! so a silent asset swap — or an unreviewed addition — fails CI. Qt-free on
//! purpose: this runs in the default `cargo test`.

use std::path::{Path, PathBuf};

use rowplay_fixtures::sha256_hex;

/// (path relative to `assets/replay/`, byte count, SHA-256).
// Emitted verbatim by `tools/vendor-replay-assets.py --emit-rust`; the byte
// counts stay as the script prints them so the table is never hand-edited.
#[allow(clippy::unreadable_literal)]
const EXPECTED: &[(&str, u64, &str)] = &[
    (
        "environments/aerial-grass-rock/aerial-grass-rock-diffuse-512.jpg",
        99466,
        "0ba2f50ac5472ea90f9a7110ae8805402592997e0f0c6e5f0ac33c8f3a3e2e34",
    ),
    (
        "environments/aerial-grass-rock/aerial-grass-rock-normal-gl-512.jpg",
        105694,
        "7186ea4554c8a657869af2e5441503126b1060d4008798c2526bf229d58af653",
    ),
    (
        "environments/aerial-grass-rock/aerial-grass-rock-roughness-512.jpg",
        76118,
        "dc7c447546f9b9439b57d1a1ee730f6785c1d6189891ae2d9211949982f4e8fb",
    ),
    (
        "environments/bark-brown-01/bark-brown-01-diffuse-512.jpg",
        123431,
        "3a5c636bf35244e6bf436b5a5da999f29efffa04882c24d42387f80ec9e21049",
    ),
    (
        "environments/bark-brown-01/bark-brown-01-normal-gl-512.jpg",
        140928,
        "572f31a0c68bd0ecd3b1680ae939cb669db901419850525edf0261caf5143170",
    ),
    (
        "environments/bark-brown-01/bark-brown-01-roughness-512.jpg",
        44822,
        "c8392e995cb7040fc022fba4099dcbced67662a3a52ba99c4b7452c627e4b7c0",
    ),
    (
        "environments/brown-planks-03/brown-planks-03-diffuse-512.jpg",
        98228,
        "3b53a63afb2a1ce6ee1fa45e6c7bab5b0241cb5f03f267e8c223b3e7694b66c2",
    ),
    (
        "environments/brown-planks-03/brown-planks-03-normal-gl-512.jpg",
        40437,
        "6ef4b139a08ef9f0bf75e84c0182176d5866fa2e4364c0ba5685b45e16834f37",
    ),
    (
        "environments/brown-planks-03/brown-planks-03-roughness-512.jpg",
        46319,
        "edc3f1996678a5d3ba5a306180ff6b4a0d0364b9cbdcf4e2ef0362cafbf0e3ee",
    ),
    (
        "environments/brushed-concrete-2/brushed-concrete-2-diffuse-512.jpg",
        61922,
        "e0552acd803f63c2e055aac8c096f5a1dddc4668cd3008017bcdc0f46a2e173f",
    ),
    (
        "environments/brushed-concrete-2/brushed-concrete-2-normal-gl-512.jpg",
        68254,
        "18a8e69e29c18478b07ac00cdb4ceada80f3be3a5fdfd4a0b3b05824ba2fd7b4",
    ),
    (
        "environments/brushed-concrete-2/brushed-concrete-2-roughness-512.jpg",
        41777,
        "500e543fa6bf4a14a43233352ff6273b326ab63f0e49b5d0af14edb729e43f55",
    ),
    (
        "environments/cobblestone-floor-03/cobblestone-floor-03-diffuse-512.jpg",
        100997,
        "999a8745fbf6996860e9c64995728c97185032389201931b29ff6f80691ff0e9",
    ),
    (
        "environments/cobblestone-floor-03/cobblestone-floor-03-normal-gl-512.jpg",
        83551,
        "947b46c737dbfe5890e264636ec437be09665a8a227a11c8b8228eec4089c335",
    ),
    (
        "environments/cobblestone-floor-03/cobblestone-floor-03-roughness-512.jpg",
        49569,
        "daeab61257a199ae69efd7ecd278f7555ed5a68ef0ba4f43fa9f7e8c492e7281",
    ),
    (
        "environments/concrete-floor-painted/concrete-floor-painted-diffuse-512.jpg",
        113161,
        "19c89793bd4a07d5f233799a0a372da2faa58b9ecf50d47220e762ba9101dd1e",
    ),
    (
        "environments/concrete-floor-painted/concrete-floor-painted-normal-gl-512.jpg",
        73925,
        "20253d5ec67097b4ee522afd787e926396add5c55a2565bc40fe3917ccecb66a",
    ),
    (
        "environments/concrete-floor-painted/concrete-floor-painted-roughness-512.jpg",
        114214,
        "0ba04433f0a61569d4e1d19eef1e947364ad43280b92ebb0e4c7bf5cd98cc613",
    ),
    (
        "environments/dry-river-pebbles/dry-river-pebbles-diffuse-512.jpg",
        116529,
        "717c0d224a024e72909504dbbe9e26f85914894067b29c909a5241df0b526662",
    ),
    (
        "environments/dry-river-pebbles/dry-river-pebbles-normal-gl-512.jpg",
        146796,
        "a35aeba24d6e3744771053a5e9074d29a89afbee185bfc6917dc87245085db6d",
    ),
    (
        "environments/dry-river-pebbles/dry-river-pebbles-roughness-512.jpg",
        69869,
        "cebf7ff77629c6ea308bfc46358314a515aeb59d076153a58712a87159cd1470",
    ),
    (
        "environments/forest-leaves-04/forest-leaves-04-diffuse-512.jpg",
        146693,
        "0c4a19d3e4f81ac2bd28475d1c0c15e0b951cf68e6b35a72ec48bdd790331b47",
    ),
    (
        "environments/forest-leaves-04/forest-leaves-04-normal-gl-512.jpg",
        173272,
        "d3b4bed87b801470ec1567bb582a618883ce06e6051298db38731a28362c4644",
    ),
    (
        "environments/forest-leaves-04/forest-leaves-04-roughness-512.jpg",
        121895,
        "001bfa94e88d2debe0b3f91c8aa72577a6d4db4e79392ba9d0354b1c86d5b925",
    ),
    (
        "environments/forrest-ground-01/forrest-ground-01-diffuse-512.jpg",
        130009,
        "655b2425d9b18d150c4a7854002e9c71ed1c321f72a8b2f929436ea557af0acb",
    ),
    (
        "environments/forrest-ground-01/forrest-ground-01-normal-gl-512.jpg",
        152007,
        "07b8bb3fa23ceb43b6f5ebdccdb49786f78222ab16a33941597c577b87e25938",
    ),
    (
        "environments/forrest-ground-01/forrest-ground-01-roughness-512.jpg",
        63945,
        "632e6bc9faf39d25afae2af708524e7a30e4daf05eba6eee7f6b66923e79958d",
    ),
    (
        "environments/leafy-grass/leafy-grass-diffuse-512.jpg",
        130499,
        "92cd6d767e9502ab4d8b1aed49b6f40e16d4b9c761020e833681fabe371506d4",
    ),
    (
        "environments/leafy-grass/leafy-grass-normal-gl-512.jpg",
        150504,
        "946fc165aa732befa8bbfe55073c66d1662c2463998bc24cfeb37afd32f8afaa",
    ),
    (
        "environments/leafy-grass/leafy-grass-roughness-512.jpg",
        40551,
        "6cca720bf619d284bedd2dac4be5a6126a3abfa566fc410312efdb9f0392391d",
    ),
    (
        "environments/rock-01/rock-01-diffuse-512.jpg",
        120693,
        "1ee06fb1752c9eab2b21d7e89e7912efd5981b923e72a6e30f5ce4c3e3bc5898",
    ),
    (
        "environments/rock-01/rock-01-normal-gl-512.jpg",
        110358,
        "6025fd703dd261b8edcbb39b21223a850f291b1b8e7564f80d7037b48160b087",
    ),
    (
        "environments/rock-01/rock-01-roughness-512.jpg",
        99142,
        "511ba59e965cdc61ee16d30cfc5b5e19cec2e99986945cc48cad0e14767766d0",
    ),
    (
        "environments/snow-02/snow-diffuse-512.jpg",
        62514,
        "0ae627f87a222d82dfe2f311ef1ba427432fdc58268d4cbcfbbff7764f4f6492",
    ),
    (
        "environments/snow-02/snow-normal-gl-512.jpg",
        156254,
        "5bc9092efa6c2d73cd6b0f04c9ed790b4c3beeed4400938911eb405918a5cc0e",
    ),
    (
        "environments/snow-02/snow-roughness-512.jpg",
        42246,
        "507717de6130d18c2c057ffbc4ae1575a32a5ea26e338deff6827032c4d5399f",
    ),
    (
        "environments/wood-floor/wood-floor-diffuse-512.jpg",
        74574,
        "763103fd5fb60cc18b1f3764bff98fbb666ab0c74512ec90f8f12626f59cb50e",
    ),
    (
        "environments/wood-floor/wood-floor-normal-gl-512.jpg",
        26004,
        "0096fb6be668196d968459efd93cd482e16a6f62fd8b86807346930fb6a58613",
    ),
    (
        "environments/wood-floor/wood-floor-roughness-512.jpg",
        66641,
        "6a639e0d23c6133df3a7906a8aa0ebdadfe9dba0472221dc21d7dc16fe2bcdc4",
    ),
    (
        "rowplay-athlete-v4.contract.json",
        27593,
        "62dd814ba6113ca57419aac42905233f8e07589846be5c3a25bbadcf55d63dd3",
    ),
    (
        "rowplay-athlete-v4.glb",
        4584320,
        "a564a4dbd4922e2ba76ef21a23f5bf0eb1b0180846548f9d7110e55ffd8f760e",
    ),
    (
        "rowplay-rigs-v3.glb",
        733864,
        "31418f4808b30fa786830129b0b637fc025b6e5ddbb539d848fc8cab74806925",
    ),
];

/// Documentation that ships with the assets but carries no hash pin.
const UNPINNED_DOCS: &[&str] = &["environments/README.md"];

fn assets_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
        .join("replay")
}

fn walk(dir: &Path, prefix: &str, out: &mut Vec<String>) {
    for entry in std::fs::read_dir(dir).expect("read assets/replay") {
        let path = entry.expect("dir entry").path();
        let rel = format!(
            "{prefix}{}",
            path.file_name().expect("file name").to_string_lossy()
        );
        if path.is_dir() {
            walk(&path, &format!("{rel}/"), out);
        } else {
            out.push(rel);
        }
    }
}

#[test]
fn vendored_assets_match_the_reviewed_hashes() {
    let dir = assets_dir();
    for (rel, size, digest) in EXPECTED {
        let path = dir.join(rel);
        let bytes = std::fs::read(&path)
            .unwrap_or_else(|error| panic!("vendored asset missing: {rel} ({error})"));
        assert_eq!(
            bytes.len() as u64,
            *size,
            "{rel}: byte count changed; refresh with tools/vendor-replay-assets.py"
        );
        assert_eq!(
            sha256_hex(&bytes),
            *digest,
            "{rel}: SHA-256 changed; a silent asset swap is a review event"
        );
    }
}

#[test]
fn no_unpinned_file_under_assets_replay() {
    let mut present = Vec::new();
    walk(&assets_dir(), "", &mut present);
    present.sort();
    let mut unknown: Vec<&String> = present
        .iter()
        .filter(|rel| {
            !EXPECTED.iter().any(|&(known, _, _)| known == rel.as_str())
                && !UNPINNED_DOCS.contains(&rel.as_str())
        })
        .collect();
    unknown.sort();
    assert!(
        unknown.is_empty(),
        "unpinned files under assets/replay: {unknown:?}; add them to the \
         expectation table and ASSET_PROVENANCE.md, or delete them"
    );
    assert_eq!(
        present.len(),
        EXPECTED.len() + UNPINNED_DOCS.len(),
        "asset inventory drift"
    );
    // ADR 0009: assets stay in plain Git until this ceiling. Exceeding it is
    // the signal to revisit the storage decision (Git LFS or split packs)
    // with an ADR, before Phase 6 venues grow the tree silently.
    const LFS_TRIPWIRE_BYTES: u64 = 50 * 1024 * 1024;
    let mut total = 0;
    for (rel, size, _) in EXPECTED {
        total += size;
        assert!(
            *size <= LFS_TRIPWIRE_BYTES,
            "{rel} is over the per-file tripwire ({size} B); ADR 0009 must be \
             revisited before vendoring a file this large"
        );
    }
    assert!(
        total <= LFS_TRIPWIRE_BYTES,
        "assets/replay totals {total} B, over the ADR 0009 plain-Git \
         tripwire ({LFS_TRIPWIRE_BYTES} B); write a follow-up ADR (Git LFS \
         or split packs) before vendoring more"
    );
}
