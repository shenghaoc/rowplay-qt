// SPDX-License-Identifier: GPL-3.0-or-later
//! Shared helpers for the app's integration tests (not a test crate itself).

/// Parses a binary P6 PPM into `(width, height, rgb bytes)`.
///
/// The gate's `grabScreen` saves a PPM twin next to every PNG exactly because
/// this format parses without an image dependency.
pub fn parse_ppm(bytes: &[u8]) -> (usize, usize, &[u8]) {
    // P6\n<w> <h>\n255\n<rgb...> — header fields separated by any whitespace.
    let mut fields = Vec::new();
    let mut i = 0;
    while fields.len() < 4 && i < bytes.len() {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let start = i;
        while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        fields.push(std::str::from_utf8(&bytes[start..i]).expect("ascii header"));
    }
    assert_eq!(fields[0], "P6", "expected a binary PPM");
    let width: usize = fields[1].parse().expect("width");
    let height: usize = fields[2].parse().expect("height");
    assert_eq!(fields[3], "255");
    (width, height, &bytes[i + 1..])
}

/// Distinct RGB colours plus the share of the most common one, sampled every
/// `step`-th pixel to bound the set size.
pub fn colour_diversity(pixels: &[u8], step: usize) -> (usize, f64) {
    use std::collections::HashMap;
    let mut counts: HashMap<[u8; 3], usize> = HashMap::new();
    let mut total = 0usize;
    let mut at = 0usize;
    while at + 3 <= pixels.len() {
        *counts
            .entry([pixels[at], pixels[at + 1], pixels[at + 2]])
            .or_default() += 1;
        total += 1;
        at += 3 * step.max(1);
    }
    let most_common = counts.values().copied().max().unwrap_or(0);
    let share = if total == 0 {
        1.0
    } else {
        most_common as f64 / total as f64
    };
    (counts.len(), share)
}

/// Asserts a rendered screenshot is neither blank nor single-coloured.
pub fn assert_rendered(width: usize, height: usize, pixels: &[u8], what: &str) {
    assert!(
        width >= 320 && height >= 200,
        "{what}: unexpected size {width}x{height}"
    );
    assert!(
        pixels.len() >= width * height * 3,
        "{what}: truncated pixel buffer"
    );
    let (distinct, top_share) = colour_diversity(pixels, 4);
    assert!(
        distinct >= 64,
        "{what}: only {distinct} distinct colours — the render looks blank"
    );
    assert!(
        top_share < 0.9,
        "{what}: one colour covers {:.0}% of the frame — the scene did not render",
        top_share * 100.0
    );
}

/// Asserts that shadows are present on the ground plane.
///
/// The chase camera tracks the equipment in the viewport centre, and the key
/// light casts shadows onto the ground beneath it. The assertion samples
/// ground luminance in the centre band (where shadows fall) and compares
/// against the far right edge (unshaded ground at the same height). The
/// actual darkening under llvmpipe is 8–12%; the 5% threshold gives headroom
/// above dithering noise while catching a genuine shadow-map failure.
///
/// The sidebar occupies the left ~27% of the frame, so "far edge" samples
/// from the right edge. The ground plane fills the lower portion of the
/// replay captures. If the 5c chase camera changes the framing, these
/// fractions need updating alongside the camera.
#[allow(dead_code)]
pub fn assert_shadows(width: usize, height: usize, pixels: &[u8], what: &str) {
    if pixels.len() < width * height * 3 {
        return; // truncated — the rendered check already caught this
    }
    // Sample regions derived from the 5b chase camera's framing at t=0 for
    // the demo workouts (1001/1003/1004): the equipment sits in the viewport
    // centre, the ground plane fills the lower portion, and the sidebar
    // occupies the left ~27%. If the chase camera or the demo workouts
    // change, re-derive these fractions from the new captures rather than
    // lowering the threshold — a threshold change masks a real regression.
    let centre_luma = region_luminance(pixels, width, height, 0.40, 0.60, 0.55, 0.75);
    let edge_luma = region_luminance(pixels, width, height, 0.90, 0.98, 0.55, 0.75);
    if edge_luma > 10.0 {
        let margin = (edge_luma - centre_luma) / edge_luma;
        assert!(
            margin > 0.05,
            "{what}: no shadow detected on the ground plane — \
             centre luminance {centre_luma:.1} vs edge {edge_luma:.1} \
             (margin {:.1}%, need >5%)",
            margin * 100.0
        );
    }
}

#[allow(dead_code)]
fn region_luminance(
    pixels: &[u8],
    width: usize,
    height: usize,
    x0_frac: f64,
    x1_frac: f64,
    y0_frac: f64,
    y1_frac: f64,
) -> f64 {
    let x0 = (width as f64 * x0_frac) as usize;
    let x1 = (width as f64 * x1_frac) as usize;
    let y0 = (height as f64 * y0_frac) as usize;
    let y1 = (height as f64 * y1_frac) as usize;
    let mut sum = 0.0_f64;
    let mut count = 0usize;
    // Sample every 3rd pixel to bound cost.
    for y in (y0..y1).step_by(3) {
        for x in (x0..x1).step_by(3) {
            let at = (y * width + x) * 3;
            if at + 2 < pixels.len() {
                let r = f64::from(pixels[at]);
                let g = f64::from(pixels[at + 1]);
                let b = f64::from(pixels[at + 2]);
                sum += 0.299 * r + 0.587 * g + 0.114 * b;
                count += 1;
            }
        }
    }
    if count == 0 { 0.0 } else { sum / count as f64 }
}

/// The app's gate progress lives in its captured stdout/stderr, invisible in
/// a CI log; a failing capture assertion must carry the relevant lines so
/// the run is diagnosable without a re-push.
// Only the gate test calls this, but common/ compiles into the smoke test
// binary too, where it would be dead code.
#[allow(dead_code)]
pub fn gate_log_lines(combined: &str) -> String {
    combined
        .lines()
        .filter(|line| {
            line.contains("gate ")
                || line.contains("replay scene rules")
                || line.contains("replay equipment")
                || line.contains("replay textures")
                || line.contains("gate screenshot")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The 3D viewport bounds within a full-window capture.
///
/// The sidebar occupies the left ~27% and the toolbar/transport bars the top
/// and bottom, so the Quick 3D viewport is the right ~two-thirds of the
/// middle.
#[allow(dead_code)]
pub fn viewport_bounds(width: usize, height: usize) -> (usize, usize, usize, usize) {
    (
        (width as f64 * 0.30) as usize,
        (height as f64 * 0.12) as usize,
        (width as f64 * 0.95) as usize,
        (height as f64 * 0.90) as usize,
    )
}

/// Asserts the 3D viewport region itself rendered something.
///
/// [`assert_rendered`] samples the whole window, so a black Quick 3D viewport
/// still passes on the strength of the sidebar and chrome. That masks a
/// capture path returning no scene at all: on macOS/Metal `grabToImage` yields
/// a black viewport while the live window renders correctly, so a phase-shot
/// baseline silently becomes black frames. Sampling the viewport region makes
/// a blank 3D area a failure on any platform.
#[allow(dead_code)]
pub fn assert_viewport_rendered(width: usize, height: usize, pixels: &[u8], what: &str) {
    let (x0, y0, x1, y1) = viewport_bounds(width, height);
    assert!(
        x1 > x0 && y1 > y0 && x1 <= width && y1 <= height,
        "{what}: bad viewport bounds"
    );
    let mut samples: Vec<[u8; 3]> = Vec::new();
    for y in (y0..y1).step_by(7) {
        for x in (x0..x1).step_by(7) {
            let at = (y * width + x) * 3;
            samples.push([pixels[at], pixels[at + 1], pixels[at + 2]]);
        }
    }
    let mut set = std::collections::BTreeSet::new();
    for sample in &samples {
        set.insert(*sample);
    }
    let distinct = set.len();
    let mut sum = [0u32; 3];
    for sample in &samples {
        for (channel, value) in sample.iter().enumerate() {
            sum[channel] += u32::from(*value);
        }
    }
    let n = samples.len() as u32;
    let mean = [sum[0] / n, sum[1] / n, sum[2] / n];
    let luminance = (mean[0] * 3 + mean[1] * 6 + mean[2]) / 10;
    assert!(
        distinct >= 16,
        "{what}: the 3D viewport has only {distinct} distinct colours (mean RGB {mean:?}) — \
         it did not render (the sidebar alone satisfies the whole-window check)"
    );
    assert!(
        luminance >= 8,
        "{what}: the 3D viewport is essentially black (mean RGB {mean:?}) — it did not render"
    );
}
