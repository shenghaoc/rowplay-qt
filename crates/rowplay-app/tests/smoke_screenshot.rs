// SPDX-License-Identifier: GPL-3.0-or-later
//! Headless stack smoke test: launches the app, renders the Quick 3D scene for
//! a few frames and checks the screenshot has real content.
//!
//! Qt Quick 3D needs a real RHI backend, so this test only runs when the
//! caller opts in with `ROWPLAY_QT_SMOKE=1` and provides a platform that can
//! create an OpenGL context, for example on Linux:
//!
//! ```sh
//! QT_QPA_PLATFORM=xcb QSG_RHI_BACKEND=opengl LIBGL_ALWAYS_SOFTWARE=1 \
//!   ROWPLAY_QT_SMOKE=1 xvfb-run -a cargo test -p rowplay-app
//! ```
//!
//! Without the opt-in the test passes trivially and says so.

use std::collections::HashSet;
use std::path::PathBuf;
use std::process::Command;

mod common;
use common::parse_ppm;

#[test]
fn renders_the_smoke_scene_headlessly() {
    if std::env::var_os("ROWPLAY_QT_SMOKE").is_none() {
        eprintln!(
            "skipping: set ROWPLAY_QT_SMOKE=1 (and a GL-capable QT_QPA_PLATFORM) to run the screenshot test"
        );
        return;
    }
    let out_dir = std::env::var_os("ROWPLAY_SMOKE_ARTIFACT_DIR").map_or_else(
        || std::env::temp_dir().join(format!("rowplay-smoke-{}", std::process::id())),
        PathBuf::from,
    );
    std::fs::create_dir_all(&out_dir).expect("create artifact dir");
    let png = out_dir.join("smoke.png");
    let ppm = out_dir.join("smoke.ppm");
    let _ = std::fs::remove_file(&png);
    let _ = std::fs::remove_file(&ppm);

    let status = Command::new(env!("CARGO_BIN_EXE_rowplay-app"))
        .env("ROWPLAY_SMOKE_SCREENSHOT", &png)
        .env("ROWPLAY_SMOKE_EXIT_AFTER_FRAMES", "24")
        .status()
        .expect("launch rowplay-app");
    assert!(status.success(), "rowplay-app exited with {status}");

    let png_bytes = std::fs::read(&png).expect("screenshot PNG written");
    assert!(png_bytes.starts_with(b"\x89PNG"), "not a PNG");
    assert!(
        png_bytes.len() > 4096,
        "PNG suspiciously small: {} bytes",
        png_bytes.len()
    );

    let ppm_bytes = std::fs::read(&ppm).expect("screenshot PPM written");
    let (width, height, pixels) = parse_ppm(&ppm_bytes);
    common::assert_rendered(width, height, pixels, "smoke");
    assert!(
        width >= 320 && height >= 200,
        "unexpected size {width}x{height}"
    );
    assert!(pixels.len() >= width * height * 3);
    let colours: HashSet<[u8; 3]> = pixels.chunks_exact(3).map(|p| [p[0], p[1], p[2]]).collect();
    assert!(
        colours.len() >= 256,
        "only {} distinct colours: the 3D scene did not render",
        colours.len()
    );
    // The sky occupies the top rows: it must read as blue, not the software fallback's flat clear colour.
    let top = &pixels[(width * 8 + width / 2) * 3..][..3];
    assert!(top[2] > top[0], "top-centre pixel {top:?} is not sky blue");
    eprintln!(
        "smoke screenshot {width}x{height} with {} colours at {}",
        colours.len(),
        png.display()
    );
}
