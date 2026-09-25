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

/// Self-test for `common::assert_shadows`: identical twins (a shadow map
/// that rendered nothing), a shadow too small to see and one too faint to
/// matter must fail; a real shadow passes in a light and in a dark scene,
/// because it is measured against the same pixels unshadowed.
#[test]
fn shadow_twins_fail_without_a_shadow_and_pass_with_one() {
    let (w, h) = (400usize, 200usize);
    let fails = |with: &[u8], without: &[u8]| {
        std::panic::catch_unwind(|| common::assert_shadows(w, h, with, without, "twins")).is_err()
    };
    // `share` of the rows, from the top, keep `kept` of their luminance.
    let shade = |ground: u8, share: f64, kept: f64| {
        let mut frame = vec![ground; w * h * 3];
        for y in 0..(h as f64 * share) as usize {
            for value in &mut frame[y * w * 3..(y + 1) * w * 3] {
                *value = (f64::from(ground) * kept) as u8;
            }
        }
        frame
    };
    for ground in [200u8, 60] {
        let unshadowed = shade(ground, 0.0, 1.0);
        assert!(fails(&unshadowed, &unshadowed), "identical twins must fail");
        assert!(
            fails(&shade(ground, 0.005, 0.5), &unshadowed),
            "a shadow over 0.5 % of the capture must fail"
        );
        common::assert_shadows(w, h, &shade(ground, 0.2, 0.8), &unshadowed, "shadow");
    }
    // Darker by more than the noise delta, but by only 3.5 %: too faint.
    let unshadowed = shade(200, 0.0, 1.0);
    assert!(
        fails(&shade(200, 0.2, 0.965), &unshadowed),
        "a 3.5 % shadow must fail"
    );
}
