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
    (
        "venues/procedural/snow-groomed-128.png",
        10630,
        "ec67239ff29c551d3f9386850f2a8d7cf4f9012bc78350111cb63afd6c7eb14a",
    ),
    (
        "venues/procedural/snow-groomed-64.png",
        3044,
        "3aac39a5ee7135b57f587a530d4168d566f716581b71ed9e0dcf81d5c827d067",
    ),
    (
        "venues/procedural/water-sheen-128.png",
        11533,
        "f1698cd276a55ed231439e30586ff2925c364c962a3f74d0f48fc73dc84029bf",
    ),
    (
        "venues/procedural/water-sheen-64.png",
        3494,
        "21f08366a14382e5afb0e9aed968ef7e7bb6ab73758eeec578e3e8d3af8c3b51",
    ),
    (
        "venues/rowplay-venue-bike-high.glb",
        1257000,
        "c9d602511d9e8fcec31e9c3ebf631ae52bf5df57827fde6e5e7f1eca03b637b9",
    ),
    (
        "venues/rowplay-venue-bike-high.json",
        17588,
        "fe5431a2090ea89b1901b8c7b8dedec8463c64f8d54d3f08105d504e7704adf5",
    ),
    (
        "venues/rowplay-venue-bike-low.glb",
        52668,
        "72edcdd4f7920f560f64fa8ea23e73fa71fa3eb7af67a30fb59b13506f6de70f",
    ),
    (
        "venues/rowplay-venue-bike-low.json",
        7370,
        "2881d536a6f81b28d4c463725b3a9f69a609e3ae195079ed1d7f9d3e1f9fdeda",
    ),
    (
        "venues/rowplay-venue-bike-medium.glb",
        435412,
        "8b6b21bce86f5ec9d2d82086645ba522d2c28bf544abd1327ae79a5f737ad490",
    ),
    (
        "venues/rowplay-venue-bike-medium.json",
        10197,
        "e54f0a3f62777d04b0ae05949c881eb57228b8a21ee16419b9151d3dda68d5c3",
    ),
    (
        "venues/rowplay-venue-bike-ultra.glb",
        1452864,
        "a41dae5364675539f9e59c667678c728b3724894d029ad94a9ce661e7bcbe958",
    ),
    (
        "venues/rowplay-venue-bike-ultra.json",
        20603,
        "1fee74c1eb748480dd91a4af656b76c84be32f11dc7207d6cea4172666fea827",
    ),
    (
        "venues/rowplay-venue-rower-high.glb",
        908748,
        "964a81298bb38fe54b6c3898a3db2e0cb92afbbe0b3ce2be3bcea89491abc7bb",
    ),
    (
        "venues/rowplay-venue-rower-high.json",
        91750,
        "128f7d5f8dacd65a232d56b54e5425545771cc28b7fd15bca3c85b364c1ecdd4",
    ),
    (
        "venues/rowplay-venue-rower-low.glb",
        386824,
        "c609412411136b5029c042ad05ba34dabb866daead0696c7f8bba3b862fa11a7",
    ),
    (
        "venues/rowplay-venue-rower-low.json",
        22285,
        "c9b27a7c57e09f042991b30835e96d243b8052a896f1a54aae95e12f0775846a",
    ),
    (
        "venues/rowplay-venue-rower-medium.glb",
        747096,
        "1996bf067bc2a7e9211f936197955f084c8073fa865c8d462be385e46f6adafc",
    ),
    (
        "venues/rowplay-venue-rower-medium.json",
        47293,
        "ecb28ed4301e808af1011d19c82a9b3a95af4ef49c214a578375ee5fb28b3893",
    ),
    (
        "venues/rowplay-venue-rower-ultra.glb",
        1122548,
        "3486eed1b6c637f08816c67759b5c7efb010f4c4fda34d43eeabdf8ae8c75c65",
    ),
    (
        "venues/rowplay-venue-rower-ultra.json",
        135099,
        "f5460a5b80f8f30cfb7340b47acff6356d0799df04a7cc9b6951180a63748e75",
    ),
    (
        "venues/rowplay-venue-skierg-high.glb",
        787976,
        "217fdfa0483f29efd7ab54c54ca4d623782b3d660e337689d81a0261ead75dfb",
    ),
    (
        "venues/rowplay-venue-skierg-high.json",
        111533,
        "76d3906e1e5144afabb3ae34ff42a4f8616d703c5e2e205ea4acc60d57727ee9",
    ),
    (
        "venues/rowplay-venue-skierg-low.glb",
        419116,
        "d3debeead44db6117d0327ee2db799b48c2c794de805f3065d34e35412b255bd",
    ),
    (
        "venues/rowplay-venue-skierg-low.json",
        18021,
        "39de8397cdc7e87b2243038c1f65d3277033ab1cf720b9ad7c8f21520b8ce695",
    ),
    (
        "venues/rowplay-venue-skierg-medium.glb",
        720856,
        "ca61b72400ec87842c90d576f7535cfda15f37996c95d3ef7d737bdf139cea99",
    ),
    (
        "venues/rowplay-venue-skierg-medium.json",
        57729,
        "b60e4e246340be1d404e85981f4e89d58b1aa2ddea9ffd6d79d4068db4580a23",
    ),
    (
        "venues/rowplay-venue-skierg-ultra.glb",
        1104968,
        "dee072effbaf6a0d5800b4562ff11553a0a96e47148a8331f87e223367d746a0",
    ),
    (
        "venues/rowplay-venue-skierg-ultra.json",
        163323,
        "70c9fc4842732e4f254435a92b9ee5bd0c222bf7dcc9e3dd54915e49e56fbb6c",
    ),
];

/// Documentation that ships with the assets but carries no hash pin.
const UNPINNED_DOCS: &[&str] = &[
    "environments/README.md",
    "venues/MANIFEST.json",
    "venues/README.md",
];

/// The application icon set under `assets/icon/` (Phase 9): the web app's
/// icon vendored from rowplay `static/` (MIT) and the two platform icon files
/// `tools/package/gen-icons.py` derives from it. Same rule as the replay
/// assets: every byte pinned, nothing unlisted.
#[allow(clippy::unreadable_literal)]
const ICON_EXPECTED: &[(&str, u64, &str)] = &[
    (
        "rowplay-icon-512.png",
        11671,
        "d1f7f39db9793d207f776523a3b070179f23c281d17bba8e437be4f7ba9c2f85",
    ),
    (
        "rowplay-icon.svg",
        235,
        "e311af0126c298efe7a58a0baab82275fd5a18b36400163ef4b7ab240bd5e1e1",
    ),
    (
        "rowplay-qt.icns",
        68370,
        "b9af4984f0cc1405a65c7b003af6de28e13dc5b9b40477225eb7323692258742",
    ),
    (
        "rowplay-qt.ico",
        14465,
        "4ede6df19fb4b3ef0b9630755717ead3e16965e7c6b92042ba5a91cb6996349e",
    ),
];

fn assets_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
        .join("replay")
}

const AUTHORED: &[&str] = &[
    "authored/MANIFEST.json",
    "authored/blue-hour.hdr",
    "authored/blue-hour.ktx",
    "authored/buoy.glb",
    "authored/course.json",
    "authored/overcast.hdr",
    "authored/overcast.ktx",
    "authored/rowing-environment.blend",
    "authored/rowing-environment.glb",
    "authored/rowing-shell.glb",
    "authored/vegetation.json",
    "authored/water-normal.png",
];

/// The authored pack's plain-Git budget (ADR 0011). Blender Phase 3 raised it
/// from 4 to 6 MiB for the environment's `.blend` source and its outputs
/// (`docs/blender-audit.md`, "Phase 3").
const AUTHORED_PACK_BUDGET: u64 = 6 * 1024 * 1024;

#[test]
fn authored_assets_match_manifest_and_budgets() {
    let dir = assets_dir().join("authored");
    let manifest: serde_json::Value = serde_json::from_slice(
        &std::fs::read(dir.join("MANIFEST.json")).expect("authored manifest"),
    )
    .expect("manifest JSON");
    let files = manifest["files"].as_object().expect("file pins");
    assert_eq!(files.len(), AUTHORED.len() - 1);
    let mut total = 0;
    for (name, pin) in files {
        assert!(AUTHORED.contains(&format!("authored/{name}").as_str()));
        let bytes = std::fs::read(dir.join(name)).expect("authored asset");
        assert_eq!(bytes.len() as u64, pin["bytes"].as_u64().unwrap(), "{name}");
        assert_eq!(
            sha256_hex(&bytes),
            pin["sha256"].as_str().unwrap(),
            "{name}"
        );
        total += bytes.len() as u64;
    }
    assert!(
        total <= AUTHORED_PACK_BUDGET,
        "authored pack exceeds {AUTHORED_PACK_BUDGET} bytes"
    );
    assert!(manifest["buoyTriangles"].as_u64().unwrap() <= 5000);
    assert!(manifest["textureMax"].as_u64().unwrap() <= 1024);
    // Blender Phase 2: the shell and both oars as drawn (build.rs recounts
    // them from the GLB itself through rowing_shell::validate_rowing_shell).
    let shell = &manifest["shell"];
    assert_eq!(shell["budget"].as_u64(), Some(60_000));
    assert!(shell["triangles"]["rendered"].as_u64().unwrap() <= 60_000);
    let course_text = std::fs::read_to_string(dir.join("course.json")).unwrap();
    let course: Vec<serde_json::Value> = serde_json::from_str(&course_text).unwrap();
    assert_eq!(course.len(), 256);
    // tools/blender/build_all.py writes one buoy per line, so a moved buoy
    // is a one-line diff.
    assert_eq!(
        course_text.lines().count(),
        course.len() + 2,
        "one buoy per line"
    );
    for entry in course {
        let p = entry["position"].as_array().unwrap();
        assert_eq!(p.len(), 3);
        assert_eq!(p[1].as_f64(), Some(0.0));
        let radius = p[0].as_f64().unwrap().hypot(p[2].as_f64().unwrap());
        assert!((radius - 23.1).abs() < 1e-5 || (radius - 33.3).abs() < 1e-5);
    }
}

/// Blender Phase 3: the environment is exported from the committed `.blend`
/// (`tools/blender/export_environment.py`). The manifest records which source
/// the GLB and the placements came from, and their budgets; the placements
/// keep the invariants the scene relies on.
#[test]
fn the_environment_matches_its_source_and_budgets() {
    const VARIANTS: [&str; 6] = [
        "tree-broadleaf-a",
        "tree-broadleaf-b",
        "tree-conifer",
        "tree-poplar",
        "shrub",
        "reeds",
    ];
    let dir = assets_dir().join("authored");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.join("MANIFEST.json")).unwrap()).unwrap();
    let environment = &manifest["environment"];
    // The outputs were exported from exactly the committed source.
    assert_eq!(
        environment["source"].as_str(),
        Some("rowing-environment.blend")
    );
    let blend = std::fs::read(dir.join("rowing-environment.blend")).unwrap();
    assert_eq!(
        environment["sourceSha256"].as_str(),
        Some(sha256_hex(&blend).as_str()),
        "rowing-environment.blend changed without a re-export (make blender-environment)"
    );
    let budget = &environment["budget"];
    let triangles = environment["triangles"].as_object().unwrap();
    for part in ["terrain", "far-bank", "woodland"] {
        assert!(
            triangles[part].as_u64().unwrap() <= budget[part].as_u64().unwrap(),
            "{part} over budget"
        );
    }
    for variant in VARIANTS {
        assert!(triangles[variant].as_u64().unwrap() <= budget["variant"].as_u64().unwrap());
    }
    assert_eq!(triangles.len(), 3 + VARIANTS.len());
    for (key, limit) in [
        ("terrain", 16_000),
        ("far-bank", 8_000),
        ("woodland", 6_000),
        ("variant", 800),
    ] {
        assert_eq!(
            budget[key].as_u64(),
            Some(limit),
            "the {key} budget moved; update docs/blender-audit.md"
        );
    }

    // One instance per line, so a moved tree is a one-line diff; sorted by
    // variant and then tier, so each tier is a prefix of its variant's list
    // (the scene's instanceCountOverride relies on it).
    let text = std::fs::read_to_string(dir.join("vegetation.json")).unwrap();
    let placements: Vec<serde_json::Value> = serde_json::from_str(&text).unwrap();
    assert_eq!(
        text.lines().count(),
        placements.len() + 2,
        "one instance per line"
    );
    let key = |e: &serde_json::Value| {
        (
            e["variant"].as_str().unwrap().to_owned(),
            e["tier"].as_u64().unwrap(),
        )
    };
    assert!(
        placements.windows(2).all(|w| key(&w[0]) <= key(&w[1])),
        "vegetation.json must be sorted by variant, then tier"
    );
    let tiers = ["low", "medium", "high", "ultra"];
    let mut shown = [0_u64; 4];
    let mut drawn = [0_u64; 4];
    for variant in VARIANTS {
        let mut counts = [0_u64; 4];
        for entry in placements.iter().filter(|e| e["variant"] == variant) {
            let tier = usize::try_from(entry["tier"].as_u64().unwrap()).unwrap();
            for count in &mut counts[tier..] {
                *count += 1;
            }
            let p = entry["position"].as_array().unwrap();
            let radius = p[0].as_f64().unwrap().hypot(p[2].as_f64().unwrap());
            // Clear of the course: the outer buoy ring is at 33.3 m and the
            // quay at 36.2 m or beyond; only reeds stand in the shallows.
            let floor = if variant == "reeds" { 35.0 } else { 38.2 };
            assert!(radius >= floor, "{variant} at r {radius:.2} m");
            let scale = entry["scale"].as_f64().unwrap();
            assert!((0.3..=2.0).contains(&scale));
        }
        assert!(
            counts[0] > 0,
            "{variant} has no Low instance: an empty instance table is not drawn"
        );
        let recorded: Vec<u64> = environment["instances"][variant]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c.as_u64().unwrap())
            .collect();
        assert_eq!(recorded, counts, "{variant} tier counts");
        for (i, count) in counts.iter().enumerate() {
            shown[i] += count;
            drawn[i] += count * triangles[variant].as_u64().unwrap();
        }
    }
    for (i, tier) in tiers.iter().enumerate() {
        assert_eq!(environment["shown"][tier].as_u64(), Some(shown[i]));
        assert_eq!(environment["drawnTriangles"][tier].as_u64(), Some(drawn[i]));
        assert!(shown[i] <= budget["instances"][tier].as_u64().unwrap());
        assert!(drawn[i] <= budget["drawn"][tier].as_u64().unwrap());
    }
    assert_eq!(placements.len() as u64, shown[3]);
}

/// Blender Phase 3: the water normal's tile is the one the scene repeats.
/// `RowingWater.qml` draws Qt's 100-unit Rectangle scaled by 60 (6000 m) and
/// repeats the texture `scaleU` times, so one tile is 6000 / scaleU metres.
#[test]
fn the_water_tile_matches_the_scene() {
    let dir = assets_dir().join("authored");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.join("MANIFEST.json")).unwrap()).unwrap();
    let water = &manifest["water"];
    let tile = water["tile"].as_f64().unwrap();
    let size = water["size"].as_u64().unwrap();
    assert!(size <= manifest["textureMax"].as_u64().unwrap());
    let qml =
        std::fs::read_to_string(assets_dir().join("../../qml/RowPlay/Replay/RowingWater.qml"))
            .unwrap();
    let number = |needle: &str| -> f64 {
        let start = qml.find(needle).unwrap_or_else(|| panic!("{needle} moved")) + needle.len();
        qml[start..]
            .split(|c: char| !(c.is_ascii_digit() || c == '.'))
            .find(|s| !s.is_empty())
            .unwrap()
            .parse()
            .unwrap()
    };
    assert!(qml.contains("scale: Qt.vector3d(60, 60, 1)"));
    let (u, v) = (number("scaleU:"), number("scaleV:"));
    assert!((u - v).abs() < f64::EPSILON);
    assert!(
        (6000.0 / u - tile).abs() < 1e-9,
        "RowingWater repeats a {} m tile, the generator draws {tile} m",
        6000.0 / u
    );
    // The PNG's own header: width and height at offsets 16 and 20.
    let png = std::fs::read(dir.join("water-normal.png")).unwrap();
    let width = u32::from_be_bytes(png[16..20].try_into().unwrap());
    let height = u32::from_be_bytes(png[20..24].try_into().unwrap());
    assert_eq!((u64::from(width), u64::from(height)), (size, size));
}

/// ADR 0002: the common assets authored here are MIT, whatever the licence of
/// the repository or of the scripts that generate them. Every file in the
/// authored pack needs its own MIT row in ASSET_PROVENANCE.md, and no row may
/// name a file the pack does not have.
#[test]
fn every_authored_asset_has_an_mit_provenance_row() {
    let provenance = std::fs::read_to_string(assets_dir().join("../../ASSET_PROVENANCE.md"))
        .expect("read ASSET_PROVENANCE.md");
    let mut rows = std::collections::BTreeMap::new();
    for line in provenance.lines() {
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        if let Some(path) = cells.get(1).and_then(|cell| {
            cell.strip_prefix("`assets/replay/")
                .and_then(|rest| rest.strip_suffix('`'))
                .filter(|rel| rel.starts_with("authored/"))
        }) {
            let licence = cells.get(2).copied().unwrap_or_default().to_owned();
            assert!(
                rows.insert(path.to_owned(), licence).is_none(),
                "{path} has two provenance rows"
            );
        }
    }
    for rel in AUTHORED {
        let licence = rows
            .get(*rel)
            .unwrap_or_else(|| panic!("assets/replay/{rel} has no row in ASSET_PROVENANCE.md"));
        assert_eq!(licence, "MIT", "assets/replay/{rel} must be MIT (ADR 0002)");
    }
    let listed: Vec<_> = rows
        .keys()
        .filter(|rel| !AUTHORED.contains(&rel.as_str()))
        .collect();
    assert!(
        listed.is_empty(),
        "provenance rows for missing files: {listed:?}"
    );
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
                && !AUTHORED.contains(&rel.as_str())
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
        EXPECTED.len() + UNPINNED_DOCS.len() + AUTHORED.len(),
        "asset inventory drift"
    );
    // ADR 0009 set a 50 MB tripwire when the next asset family's size was
    // unknown. Phase 6a measured the baked venues (ADR 0011): the whole
    // `assets/replay/` tree is ~18.3 MiB, two orders of magnitude below
    // GitHub's per-file limits, so assets stay in plain Git (no LFS) and the
    // total ceiling is raised to 100 MB. The next doubling is again a
    // decision point.
    const LFS_TRIPWIRE_BYTES: u64 = 100 * 1024 * 1024;
    let mut total = 0;
    for rel in AUTHORED {
        total += std::fs::metadata(assets_dir().join(rel)).unwrap().len();
    }
    for (rel, size, _) in EXPECTED {
        total += size;
        assert!(
            *size <= LFS_TRIPWIRE_BYTES,
            "{rel} is over the per-file tripwire ({size} B); ADR 0011 (which \
             raised ADR 0009's ceiling) must be revisited before vendoring a \
             file this large"
        );
    }
    assert!(
        total <= LFS_TRIPWIRE_BYTES,
        "assets/replay totals {total} B, over the ADR 0011 plain-Git \
         tripwire ({LFS_TRIPWIRE_BYTES} B); write a follow-up ADR (Git LFS \
         or split packs) before vendoring more"
    );
}

/// The icon set is pinned like the replay assets and, like them, admits no
/// unlisted file: a stray export in `assets/icon/` would otherwise ship in
/// nobody's package and drift from ASSET_PROVENANCE.md unnoticed.
#[test]
fn icon_assets_match_their_pins_and_nothing_else_is_present() {
    let dir = assets_dir().parent().expect("assets/").join("icon");
    for (rel, size, sha) in ICON_EXPECTED {
        let path = dir.join(rel);
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        assert_eq!(bytes.len() as u64, *size, "{rel}: size");
        assert_eq!(sha256_hex(&bytes), *sha, "{rel}: SHA-256");
    }
    let mut present = Vec::new();
    walk(&dir, "", &mut present);
    present.sort();
    let unknown: Vec<&String> = present
        .iter()
        .filter(|rel| {
            !ICON_EXPECTED
                .iter()
                .any(|&(known, _, _)| known == rel.as_str())
        })
        .collect();
    assert!(
        unknown.is_empty(),
        "unpinned files under assets/icon: {unknown:?}; add them to ICON_EXPECTED and \
         ASSET_PROVENANCE.md, or delete them"
    );
    assert_eq!(present.len(), ICON_EXPECTED.len(), "icon inventory drift");
}
