// SPDX-License-Identifier: GPL-3.0-or-later
//! Self-test for `common::assert_viewport_rendered`: a frame that is black in
//! the 3D viewport but colourful in the sidebar passed the old whole-window
//! check, which is exactly how a black phase-shot baseline went unnoticed
//! (docs/qt-bridges-notes.md #17).

// Test binaries each include the shared module but use different subsets.
#[allow(dead_code)]
mod common;

#[test]
fn black_viewport_fails_and_a_rendered_one_passes() {
    let (w, h) = (1200usize, 800usize);
    let mut frame = vec![0u8; w * h * 3];
    // Sidebar band: colourful, so the whole-window check is satisfied.
    for y in 0..h {
        for x in 0..(w * 3 / 10) {
            let at = (y * w + x) * 3;
            let v = ((x * 7 + y * 13) % 256) as u8;
            frame[at] = v;
            frame[at + 1] = 255 - v;
            frame[at + 2] = v / 2;
        }
    }
    // The blind spot: whole-window passes, viewport check must fail.
    common::assert_rendered(w, h, &frame, "sidebar-only");
    let caught = std::panic::catch_unwind(|| {
        common::assert_viewport_rendered(w, h, &frame, "sidebar-only");
    });
    assert!(caught.is_err(), "a black viewport must fail the check");

    // A genuinely rendered viewport passes.
    let (x0, y0, x1, y1) = common::viewport_bounds(w, h);
    for y in y0..y1 {
        for x in x0..x1 {
            let at = (y * w + x) * 3;
            frame[at] = ((x * 3 + y) % 200) as u8;
            frame[at + 1] = ((x + y * 3) % 220) as u8;
            frame[at + 2] = 180;
        }
    }
    common::assert_viewport_rendered(w, h, &frame, "rendered");
}
