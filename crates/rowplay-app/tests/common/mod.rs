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
                || line.contains("gate screenshot")
        })
        .collect::<Vec<_>>()
        .join("\n")
}
