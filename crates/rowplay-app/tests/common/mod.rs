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

/// A luminance change a shadow must exceed: the 3D capture noise bound's
/// channel delta (AGENTS.md, "Working efficiently").
const SHADOW_NOISE_DELTA: f64 = 4.0;

/// Asserts that the key light's shadow renders, from two grabs of the same
/// frame: `shadowed` as the tier sets the light, and `unshadowed` with the
/// light's shadow off.
///
/// Every pixel is compared with itself unshadowed, the local ground or
/// surface under it, so the check measures the shadow's relative contrast
/// and does not depend on the scheme's palette. The shadow must darken at
/// least 1 % of the capture by more than the noise delta, and those pixels
/// must lose more than 5 % of their luminance on average: a shadow map that
/// fails renders the twins alike.
///
/// It replaced a check that compared the capture's centre with its right
/// edge in the Medium captures, which have no shadows at all (the tier rules
/// cast them at High and Ultra only). Once the replay hid the sidebar the
/// centre sampled the athlete and hull, so it compared their albedo with the
/// water's: the light scheme passed and the dark one, whose water is about
/// as dark as the athlete, failed at 4.9 % (Metal).
#[allow(dead_code)]
pub fn assert_shadows(width: usize, height: usize, shadowed: &[u8], unshadowed: &[u8], what: &str) {
    let n = width * height;
    assert!(
        shadowed.len() >= n * 3 && unshadowed.len() >= n * 3,
        "{what}: truncated pixel buffer"
    );
    let luma = |pixels: &[u8], i: usize| {
        0.299 * f64::from(pixels[i * 3])
            + 0.587 * f64::from(pixels[i * 3 + 1])
            + 0.114 * f64::from(pixels[i * 3 + 2])
    };
    let (mut darker, mut lost, mut base) = (0usize, 0.0_f64, 0.0_f64);
    for i in 0..n {
        let (with, without) = (luma(shadowed, i), luma(unshadowed, i));
        if without - with > SHADOW_NOISE_DELTA {
            darker += 1;
            lost += without - with;
            base += without;
        }
    }
    let area = darker as f64 / n as f64;
    let margin = if base > 0.0 { lost / base } else { 0.0 };
    // Printed on a pass too: the figures per platform are the check's record
    // (docs/qt-bridges-notes.md, "Three gate assertions").
    eprintln!(
        "{what}: the key light's shadow darkens {:.2} % of the capture ({darker} px) \
         by {:.2} % (need at least 1 % and more than 5 %)",
        area * 100.0,
        margin * 100.0
    );
    assert!(
        area >= 0.01,
        "{what}: the key light's shadow darkens only {:.2} % of the capture \
         ({darker} px), need at least 1 % — the shadow map did not render",
        area * 100.0
    );
    assert!(
        margin > 0.05,
        "{what}: the shadow takes {:.1} % of the luminance where it falls, \
         need more than 5 %",
        margin * 100.0
    );
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
