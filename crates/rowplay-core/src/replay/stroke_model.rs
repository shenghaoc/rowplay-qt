// SPDX-License-Identifier: GPL-3.0-or-later
//! The renderer-neutral stroke pose model (web `replay/strokeModel.ts`).
//!
//! `build_stroke_timeline` + `stroke_pose_at` are the canonical web pipeline
//! used by the renderers. The exported `stroke-pose-parity.json` fixture
//! predates the web's 2026-07 amplitude / progress rework and was verified
//! against Studio's frame-based `computeAtTime`, so that entry point
//! ([`compute_at_time`]) is ported alongside; see `docs/source-map.md`.

use crate::models::{Sport, Stroke};
use crate::replay::engine::Frame;
use crate::replay::motion::warp_stroke_phase;

const TAU: f64 = std::f64::consts::TAU;

/// One normalized stroke row on the replay timeline (web `StrokeTimelineEntry`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StrokeTimelineEntry {
    /// Stroke-row index (0-based after anchor skipping).
    pub index: usize,
    /// Integrated cycle position at the entry bounds (synthetic timelines).
    pub start_cycle: f64,
    /// End of the integrated cycle span.
    pub end_cycle: f64,
    /// Seconds since workout start at the row's catch.
    pub start_t: f64,
    /// Seconds since workout start at the row's finish.
    pub end_t: f64,
    /// Cumulative metres at the row's catch.
    pub start_d: f64,
    /// Cumulative metres at the row's finish.
    pub end_d: f64,
    /// Stored sec/500 m pace.
    pub pace: f64,
    /// Strokes per minute (rpm on the BikeErg).
    pub spm: f64,
    /// Heart rate in bpm, when recorded.
    pub hr: Option<f64>,
    /// Watts, derived from pace when not reported.
    pub watts: f64,
}

/// Stroke aggregates used to normalise pose intensity / fatigue
/// (web `StrokeTimeline`).
#[derive(Debug, Clone, PartialEq)]
pub struct StrokeTimeline {
    /// Machine family the strokes belong to.
    pub sport: Sport,
    /// True when each row is a genuine per-stroke sample.
    pub real: bool,
    /// Normalised stroke rows, anchor rows skipped.
    pub entries: Vec<StrokeTimelineEntry>,
    /// Total seconds covered by the timeline.
    pub duration: f64,
    /// Total metres covered by the timeline.
    pub distance: f64,
    /// Median positive watts (0 when none).
    pub median_watts: f64,
    /// Peak positive watts.
    pub peak_watts: f64,
    /// Median metres per stroke.
    pub median_dps: f64,
    /// Median positive heart rate (0 when none).
    pub median_hr: f64,
    /// Maximum heart rate.
    pub max_hr: f64,
}

/// Per-frame pose state (web `StrokePose`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StrokePose {
    /// Stroke-row index: cycle counter and catch-transition key.
    pub index: usize,
    /// Continuous phase in radians; one full cycle per modeled stroke.
    pub phase: f64,
    /// Phase warped with an inferred drive/recovery split.
    pub warped_phase: f64,
    /// 0..1 within the current stroke cycle.
    pub cycle_frac: f64,
    /// Estimated drive share of the stroke cycle; not a force curve.
    pub drive_frac: f64,
    /// Whether the pose sits inside the drive window.
    pub drive: bool,
    /// 0..1 progress through the drive (1 through recovery).
    pub drive_progress: f64,
    /// 0..1 progress through the recovery (0 through drive).
    pub recovery_progress: f64,
    /// Modelled duration of this stroke in seconds.
    pub stroke_seconds: f64,
    /// Modelled distance of this stroke in metres.
    pub stroke_meters: f64,
    /// Stroke rate (spm, or rpm on the BikeErg).
    pub rate: f64,
    /// Watts, derived from pace when not reported.
    pub watts: f64,
    /// Normalised intensity 0..1.
    pub intensity: f64,
    /// Amplitude multiplier (restrained 0.94..1.06 in the web pipeline).
    pub amplitude: f64,
    /// Normalised fatigue 0..1.
    pub fatigue: f64,
    /// True for poses built from genuine per-stroke data.
    pub real: bool,
}

/// Normalisation aggregates for the Studio frame-based pose path
/// (Studio `ReplayStrokePoseContext`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PoseContext {
    /// Machine family the pose belongs to.
    pub sport: Sport,
    /// Peak positive watts of the trace.
    pub peak_watts: f64,
    /// Median positive watts of the trace.
    pub median_watts: f64,
    /// Median distance per stroke (metres).
    pub median_dps: f64,
    /// Maximum heart rate of the trace.
    pub max_hr: f64,
}

fn clamp(v: f64, lo: f64, hi: f64) -> f64 {
    v.max(lo).min(hi)
}

fn finite(v: f64, fallback: f64) -> f64 {
    if v.is_finite() { v } else { fallback }
}

fn default_rate(sport: Sport) -> f64 {
    match sport {
        Sport::Bike => 80.0,
        Sport::Skierg => 32.0,
        Sport::Rower => 28.0,
    }
}

/// Web-parity median: non-finite values discarded, middle (or average of the
/// two middle) of the sorted rest.
fn median(values: &mut Vec<f64>, fallback: f64) -> f64 {
    values.retain(|v| v.is_finite());
    if values.is_empty() {
        return fallback;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = values.len() / 2;
    if values.len().is_multiple_of(2) {
        (values[mid - 1] + values[mid]) / 2.0
    } else {
        values[mid]
    }
}

/// Seconds per stroke from a rate, clamped to safe per-sport ranges (web
/// `secondsFromRate`; JS `spm || base` treats 0 and NaN as "unset").
fn seconds_from_rate(spm: f64, sport: Sport) -> f64 {
    let base = default_rate(sport);
    let (lo, hi) = if sport == Sport::Bike {
        (25.0, 130.0)
    } else {
        (10.0, 60.0)
    };
    let raw = if spm.is_nan() || spm == 0.0 {
        base
    } else {
        spm
    };
    60.0 / clamp(raw, lo, hi)
}

/// Metres covered in `seconds` at `pace` (sec/500 m, normalised for BikeErg
/// by the time it reaches the replay model).
fn meters_from_pace(seconds: f64, pace: f64) -> f64 {
    if !seconds.is_finite() || seconds <= 0.0 || !pace.is_finite() || pace <= 0.0 {
        return 0.0;
    }
    (seconds / pace) * 500.0
}

/// Estimated drive share of the cycle, biased by rate, power and duration
/// (web `driveFraction`).
fn drive_fraction(sport: Sport, seconds: f64, rate: f64, intensity: f64) -> f64 {
    if sport == Sport::Bike {
        return 0.5;
    }
    // Guard against NaN from corrupt sensor data (web-only hardening).
    let safe_intensity = if intensity.is_finite() {
        intensity
    } else {
        0.5
    };
    let base = if sport == Sport::Skierg { 0.34 } else { 0.38 };
    let rate_base = if sport == Sport::Skierg { 32.0 } else { 28.0 };
    let rate_bias = clamp((rate - rate_base) / 40.0, -0.12, 0.12);
    let power_bias = (safe_intensity - 0.5) * 0.08;
    let duration_bias = clamp((2.0 - seconds) / 8.0, -0.06, 0.06);
    clamp(base + rate_bias + power_bias + duration_bias, 0.28, 0.46)
}

fn default_drive_fraction(sport: Sport) -> f64 {
    match sport {
        Sport::Bike => 0.5,
        Sport::Skierg => 0.34,
        Sport::Rower => 0.38,
    }
}

/// Normalise one raw stroke row into a timeline entry (web `normalizeEntry`).
fn normalize_entry(
    stroke: &Stroke,
    index: usize,
    start_t: f64,
    start_d: f64,
    sport: Sport,
) -> StrokeTimelineEntry {
    let mut end_t = finite(stroke.t, start_t);
    if end_t <= start_t {
        end_t = start_t + seconds_from_rate(stroke.spm, sport);
    }
    let mut end_d = finite(stroke.d, start_d);
    if end_d < start_d {
        end_d = start_d + meters_from_pace(end_t - start_t, stroke.pace);
    }
    // Second pass: zero-distance real rows (e.g. rest strokes with d
    // unchanged); pace is re-checked because it may be valid even when d has
    // not advanced.
    if end_d == start_d {
        end_d += meters_from_pace(end_t - start_t, stroke.pace);
    }
    StrokeTimelineEntry {
        index,
        start_cycle: 0.0,
        end_cycle: 0.0,
        start_t,
        end_t,
        start_d,
        end_d,
        pace: finite(stroke.pace, 0.0),
        spm: finite(stroke.spm, 0.0),
        hr: stroke.hr,
        watts: finite(stroke.watts, 0.0),
    }
}

/// Concept2 interval boundaries can repeat the current cumulative coordinate;
/// those rows are anchors, not strokes (web `isNonAdvancingAnchor`).
fn is_non_advancing_anchor(stroke: &Stroke, start_t: f64, start_d: f64) -> bool {
    if !stroke.t.is_finite() || !stroke.d.is_finite() {
        return false;
    }
    (stroke.t - start_t).abs() < 1e-6 && (stroke.d - start_d).abs() < 1e-6
}

/// Build the normalised replay timeline (web `buildStrokeTimeline`).
#[must_use]
pub fn build_stroke_timeline(strokes: &[Stroke], sport: Sport, real: bool) -> StrokeTimeline {
    if strokes.is_empty() {
        return StrokeTimeline {
            sport,
            entries: Vec::new(),
            real,
            duration: 0.0,
            distance: 0.0,
            median_watts: 0.0,
            peak_watts: 0.0,
            median_dps: 0.0,
            median_hr: 0.0,
            max_hr: 0.0,
        };
    }
    let mut start_t = 0.0;
    let mut start_d = 0.0;
    let mut start_cycle = 0.0_f64;
    let mut entries: Vec<StrokeTimelineEntry> = Vec::new();
    for stroke in strokes {
        if is_non_advancing_anchor(stroke, start_t, start_d) {
            continue;
        }
        let normalized = normalize_entry(stroke, entries.len(), start_t, start_d, sport);
        let cycle_span = if real {
            1.0
        } else {
            (normalized.end_t - normalized.start_t).max(0.0)
                / seconds_from_rate(normalized.spm, sport)
        };
        let entry = StrokeTimelineEntry {
            start_cycle,
            end_cycle: start_cycle + cycle_span,
            ..normalized
        };
        start_t = entry.end_t;
        start_d = entry.end_d;
        start_cycle = entry.end_cycle;
        entries.push(entry);
    }

    let mut watts = Vec::new();
    let mut dps = Vec::new();
    let mut hrs = Vec::new();
    let mut peak_watts = 0.0_f64;
    let mut max_hr = 0.0_f64;
    for e in &entries {
        if e.watts > 0.0 {
            watts.push(e.watts);
            if e.watts > peak_watts {
                peak_watts = e.watts;
            }
        }
        let d = e.end_d - e.start_d;
        if d > 0.0 {
            dps.push(d);
        }
        if let Some(h) = e.hr {
            if h > 0.0 {
                hrs.push(h);
                if h > max_hr {
                    max_hr = h;
                }
            }
        }
    }

    let dps_fallback = match sport {
        Sport::Bike => 5.0,
        Sport::Skierg => 8.0,
        Sport::Rower => 11.0,
    };
    StrokeTimeline {
        sport,
        real: real && !entries.is_empty(),
        duration: entries.last().map_or(0.0, |e| e.end_t),
        distance: entries.last().map_or(0.0, |e| e.end_d),
        median_watts: median(&mut watts, 0.0),
        peak_watts,
        median_dps: median(&mut dps, dps_fallback),
        median_hr: median(&mut hrs, 0.0),
        max_hr,
        entries,
    }
}

/// First entry whose `end_t` is after `t` (web `entryAt`).
fn entry_at(entries: &[StrokeTimelineEntry], t: f64) -> Option<&StrokeTimelineEntry> {
    if entries.is_empty() {
        return None;
    }
    if t <= entries[0].start_t {
        return Some(&entries[0]);
    }
    let mut lo = 0_usize;
    let mut hi = entries.len() - 1;
    while lo < hi {
        let mid = (lo + hi) / 2;
        if t < entries[mid].end_t {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    entries.get(lo)
}

/// Which amplitude law a pose is assembled with: the current web pipeline's
/// restrained decorative scale, or the livelier Studio formula that the
/// exported pose fixture was verified against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AmplitudeLaw {
    /// `clamp(0.94 + i·0.12, 0.94, 1.06)` — decorative surge/bob only.
    Web,
    /// `clamp(0.78 + i·0.44 + f·0.08, 0.72, 1.32)` — Studio / pre-#171 web.
    Studio,
}

fn amplitude(law: AmplitudeLaw, intensity: f64, fatigue: f64) -> f64 {
    match law {
        AmplitudeLaw::Web => clamp(0.94 + intensity * 0.12, 0.94, 1.06),
        AmplitudeLaw::Studio => clamp(0.78 + intensity * 0.44 + fatigue * 0.08, 0.72, 1.32),
    }
}

#[allow(clippy::too_many_arguments)]
fn make_pose(
    index: usize,
    cycle_frac: f64,
    stroke_seconds: f64,
    stroke_meters: f64,
    rate: f64,
    watts: f64,
    intensity: f64,
    fatigue: f64,
    drive_frac: f64,
    real: bool,
    law: AmplitudeLaw,
) -> StrokePose {
    let cycle_frac = clamp(cycle_frac, 0.0, 0.999_999);
    let phase = (index as f64 + cycle_frac) * TAU;
    let drive = cycle_frac < drive_frac;
    let drive_progress = if drive { cycle_frac / drive_frac } else { 1.0 };
    let recovery_progress = if drive {
        0.0
    } else {
        (cycle_frac - drive_frac) / (1.0 - drive_frac)
    };
    StrokePose {
        index,
        phase,
        warped_phase: warp_stroke_phase(phase, drive_frac),
        cycle_frac,
        drive_frac,
        drive,
        drive_progress: clamp(drive_progress, 0.0, 1.0),
        recovery_progress: clamp(recovery_progress, 0.0, 1.0),
        stroke_seconds,
        stroke_meters,
        rate,
        watts,
        intensity: clamp(intensity, 0.0, 1.0),
        amplitude: amplitude(law, clamp(intensity, 0.0, 1.0), clamp(fatigue, 0.0, 1.0)),
        fatigue: clamp(fatigue, 0.0, 1.0),
        real,
    }
}

/// Intensity / fatigue blend shared by both pose paths (identical formulas in
/// the web pipeline and Studio's port; only the `progress` source differs).
#[allow(clippy::too_many_arguments)]
fn intensity_and_fatigue(
    watts: f64,
    meters: f64,
    rate: f64,
    sport: Sport,
    peak_watts: f64,
    median_watts: f64,
    median_dps: f64,
    max_hr: f64,
    hr: Option<f64>,
    median_hr: f64,
    progress: f64,
) -> (f64, f64) {
    let watts_norm = if peak_watts > 0.0 {
        watts / peak_watts
    } else if median_watts > 0.0 {
        watts / median_watts
    } else {
        0.35
    };
    let dps_norm = if median_dps > 0.0 {
        meters / median_dps
    } else {
        1.0
    };
    // Rate ceilings tuned to elite-sprint peaks (bike ~120 rpm, row/ski ~36).
    let rate_ceiling = if sport == Sport::Bike { 120.0 } else { 36.0 };
    let rate_norm = rate / rate_ceiling;
    let intensity = clamp(
        watts_norm * 0.55 + clamp(dps_norm / 1.45, 0.0, 1.0) * 0.3 + rate_norm * 0.15,
        0.0,
        1.0,
    );

    let hr_value = hr.unwrap_or(0.0);
    let hr_fatigue = if hr_value > 0.0 && max_hr > 0.0 {
        clamp(
            (hr_value - (median_hr - 5.0).max(0.0)) / (max_hr - median_hr + 10.0).max(20.0),
            0.0,
            1.0,
        )
    } else {
        0.0
    };
    let fatigue = clamp(
        hr_fatigue * 0.65 + progress * 0.25 + (intensity - 0.75).max(0.0) * 0.1,
        0.0,
        1.0,
    );
    (intensity, fatigue)
}

/// Synthetic pose for timelines without real per-row cycles
/// (web `syntheticPoseAt`).
fn synthetic_pose_at(timeline: &StrokeTimeline, t: f64) -> StrokePose {
    let sport = timeline.sport;
    let entry = entry_at(&timeline.entries, t);
    let rate = if let Some(e) = entry {
        if e.spm.is_finite() && e.spm != 0.0 {
            e.spm
        } else {
            default_rate(sport)
        }
    } else {
        default_rate(sport)
    };
    let entry_progress = entry.map_or(0.0, |e| {
        clamp(
            (t.max(0.0) - e.start_t) / (e.end_t - e.start_t).max(0.05),
            0.0,
            1.0,
        )
    });
    let cycle = if let Some(e) = entry {
        e.start_cycle + (e.end_cycle - e.start_cycle) * entry_progress
    } else {
        t.max(0.0) / seconds_from_rate(rate, sport)
    };
    let phase = cycle * TAU;
    let watts = entry.map_or(0.0, |e| e.watts);
    let intensity_raw = if timeline.peak_watts > 0.0 {
        watts / timeline.peak_watts
    } else {
        rate / 45.0
    };
    let cycle_frac = (((phase / TAU) % 1.0) + 1.0) % 1.0;
    let stroke_meters = entry.map_or(
        match sport {
            Sport::Bike => 5.0,
            Sport::Skierg => 8.0,
            Sport::Rower => 11.0,
        },
        |e| (e.end_d - e.start_d).max(0.0),
    );
    make_pose(
        cycle.max(0.0).floor() as usize,
        cycle_frac,
        seconds_from_rate(rate, sport),
        stroke_meters,
        rate,
        watts,
        clamp(intensity_raw, 0.0, 1.0),
        0.0,
        default_drive_fraction(sport),
        false,
        AmplitudeLaw::Web,
    )
}

/// Sample the pose at replay time `t` (web `strokePoseAt`).
#[must_use]
pub fn stroke_pose_at(timeline: &StrokeTimeline, t: f64) -> StrokePose {
    if !timeline.real || timeline.entries.is_empty() {
        return synthetic_pose_at(timeline, t);
    }
    let Some(entry) = entry_at(&timeline.entries, t.max(0.0)) else {
        return fallback_stroke_pose(timeline.sport, 0.0, 0.0);
    };

    let seconds = (entry.end_t - entry.start_t).max(0.05);
    let meters = (entry.end_d - entry.start_d).max(0.0);
    let cycle_frac = clamp((t.max(0.0) - entry.start_t) / seconds, 0.0, 0.999_999);
    let (intensity, fatigue) = intensity_and_fatigue(
        entry.watts,
        meters,
        entry.spm,
        timeline.sport,
        timeline.peak_watts,
        timeline.median_watts,
        timeline.median_dps,
        timeline.max_hr,
        entry.hr,
        timeline.median_hr,
        if timeline.duration > 0.0 {
            clamp(entry.end_t / timeline.duration, 0.0, 1.0)
        } else {
            0.0
        },
    );
    let drive_frac = drive_fraction(timeline.sport, seconds, entry.spm, intensity);

    make_pose(
        entry.index,
        cycle_frac,
        seconds,
        meters,
        entry.spm,
        entry.watts,
        intensity,
        fatigue,
        drive_frac,
        true,
        AmplitudeLaw::Web,
    )
}

/// Synthetic pose for workouts without stroke data (web `fallbackStrokePose`).
#[must_use]
pub fn fallback_stroke_pose(sport: Sport, phase: f64, rate: f64) -> StrokePose {
    let safe_phase = finite(phase, 0.0);
    let intensity = clamp(
        rate / if sport == Sport::Bike { 120.0 } else { 40.0 },
        0.0,
        1.0,
    );
    let cycle_frac = (((safe_phase / TAU) % 1.0) + 1.0) % 1.0;
    make_pose(
        (safe_phase.max(0.0) / TAU) as usize,
        cycle_frac,
        seconds_from_rate(rate, sport),
        match sport {
            Sport::Bike => 5.0,
            Sport::Skierg => 8.0,
            Sport::Rower => 11.0,
        },
        rate,
        0.0,
        intensity,
        0.0,
        default_drive_fraction(sport),
        false,
        AmplitudeLaw::Web,
    )
}

/// Studio's frame-based pose path (`ReplayStrokePose.computeAtTime`), kept for
/// parity with the exported `stroke-pose-parity.json` fixture: progress is
/// the frame's own time fraction and the amplitude follows the livelier
/// pre-#171 law. Renderers use [`stroke_pose_at`] instead.
#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn compute_at_time(
    frame: &Frame,
    stroke_start_time: f64,
    stroke_end_time: f64,
    stroke_start_distance: f64,
    stroke_end_distance: f64,
    stroke_index: usize,
    context: &PoseContext,
    median_hr: f64,
    duration: f64,
) -> StrokePose {
    let sport = context.sport;
    let rate = finite(frame.spm, default_rate(sport)).max(0.0);
    let stroke_duration = (stroke_end_time - stroke_start_time).max(0.05);
    let t = finite(frame.t, stroke_start_time);
    let raw_cycle_frac = (t - stroke_start_time) / stroke_duration;
    let cycle_frac = clamp(raw_cycle_frac, 0.0, 0.999_999);
    let meters = (stroke_end_distance - stroke_start_distance).max(0.0);
    let watts = frame.watts.max(0.0);
    let progress = if duration > 0.0 {
        clamp(frame.progress, 0.0, 1.0)
    } else {
        0.0
    };
    let (intensity, fatigue) = intensity_and_fatigue(
        watts,
        meters,
        rate,
        sport,
        context.peak_watts,
        context.median_watts,
        context.median_dps,
        context.max_hr,
        frame.hr,
        median_hr,
        progress,
    );
    let drive_frac = drive_fraction(sport, stroke_duration, rate, intensity);
    make_pose(
        stroke_index,
        cycle_frac,
        stroke_duration,
        meters,
        rate,
        watts,
        intensity,
        fatigue,
        drive_frac,
        true,
        AmplitudeLaw::Studio,
    )
}

/// Freeze a pose for reduced motion: zero out repetitive articulation while
/// preserving spatial state (Studio `ReplayStrokePose.reducedMotion`).
#[must_use]
pub fn reduced_motion(pose: &StrokePose) -> StrokePose {
    let mut frozen = *pose;
    frozen.phase = 0.0;
    frozen.warped_phase = 0.0;
    frozen.cycle_frac = 0.0;
    frozen.drive = false;
    frozen.drive_progress = 0.0;
    frozen.recovery_progress = 0.0;
    frozen
}

/// Count stroke-row catch transitions without firing on seeks or backward
/// scrubs (web `catchTransitions`, default `max_catches = 3`).
#[must_use]
pub fn catch_transitions(prev: Option<&StrokePose>, next: Option<&StrokePose>) -> u32 {
    catch_transitions_up_to(prev, next, 3)
}

/// [`catch_transitions`] with an explicit ceiling.
#[must_use]
pub fn catch_transitions_up_to(
    prev: Option<&StrokePose>,
    next: Option<&StrokePose>,
    max_catches: u32,
) -> u32 {
    let (Some(prev), Some(next)) = (prev, next) else {
        return 0;
    };
    if next.index <= prev.index {
        return 0;
    }
    let diff = (next.index - prev.index) as u32;
    if diff <= max_catches { diff } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stroke(t: f64, d: f64, spm: f64, watts: f64, hr: Option<f64>) -> Stroke {
        Stroke {
            t,
            d,
            pace: 120.0,
            spm,
            hr,
            watts,
            raw_t: None,
            raw_d: None,
        }
    }

    #[test]
    fn maps_irregular_intervals_to_one_cycle_per_row() {
        let timeline = build_stroke_timeline(
            &[
                stroke(1.8, 10.0, 30.0, 180.0, Some(150.0)),
                stroke(4.4, 24.0, 23.0, 210.0, Some(150.0)),
                stroke(6.1, 36.0, 35.0, 260.0, Some(150.0)),
            ],
            Sport::Rower,
            true,
        );
        let first = stroke_pose_at(&timeline, 0.9);
        let second = stroke_pose_at(&timeline, 2.6);
        let third = stroke_pose_at(&timeline, 5.2);
        assert_eq!(first.index, 0);
        assert!((first.cycle_frac - 0.5).abs() < 0.01);
        assert_eq!(second.index, 1);
        assert!((second.stroke_seconds - 2.6).abs() < 0.01);
        assert_eq!(third.index, 2);
        assert!((third.stroke_seconds - 1.7).abs() < 0.01);
    }

    #[test]
    fn normalizes_interval_reset_rows() {
        let timeline = build_stroke_timeline(
            &[
                stroke(5.0, 52.0, 24.0, 150.0, None),
                stroke(10.0, 105.0, 25.0, 170.0, None),
                stroke(2.0, 22.0, 30.0, 220.0, None),
            ],
            Sport::Skierg,
            true,
        );
        let third = &timeline.entries[2];
        assert_eq!(third.start_t, 10.0);
        assert!(third.end_t > 10.0);
        assert!(third.end_d > third.start_d);
        assert_eq!(stroke_pose_at(&timeline, 10.5).index, 2);
    }

    #[test]
    fn skips_non_advancing_anchors() {
        let timeline = build_stroke_timeline(
            &[
                stroke(0.0, 0.0, 30.0, 0.0, None),
                stroke(2.0, 10.0, 30.0, 180.0, None),
                stroke(2.0, 10.0, 30.0, 180.0, None),
                stroke(4.0, 21.0, 30.0, 190.0, None),
            ],
            Sport::Rower,
            true,
        );
        assert_eq!(timeline.entries.len(), 2);
        assert_eq!(
            timeline.entries.iter().map(|e| e.index).collect::<Vec<_>>(),
            vec![0, 1]
        );
        assert_eq!(timeline.duration, 4.0);
        assert_eq!(timeline.distance, 21.0);
        assert_eq!(stroke_pose_at(&timeline, 2.0).index, 1);
    }

    #[test]
    fn infers_bikeerg_distance_from_normalized_pace() {
        let timeline = build_stroke_timeline(
            &[
                stroke(0.0, 0.0, 60.0, 0.0, None),
                Stroke {
                    t: 2.0,
                    d: 0.0,
                    pace: 100.0,
                    spm: 60.0,
                    hr: None,
                    watts: 200.0,
                    raw_t: None,
                    raw_d: None,
                },
            ],
            Sport::Bike,
            true,
        );
        assert_eq!(timeline.entries.len(), 1);
        let e = &timeline.entries[0];
        assert!((e.end_d - e.start_d - 10.0).abs() < 1e-10);
    }

    #[test]
    fn keeps_split_fallback_synthetic_and_finite() {
        let timeline = build_stroke_timeline(
            &[
                stroke(60.0, 250.0, 0.0, 0.0, None),
                stroke(120.0, 500.0, 0.0, 0.0, None),
            ],
            Sport::Rower,
            false,
        );
        let pose = stroke_pose_at(&timeline, 45.0);
        assert!(!pose.real);
        assert!(pose.phase.is_finite());
        assert!(pose.rate > 0.0);
    }

    #[test]
    fn integrates_synthetic_phase_across_changing_rates() {
        let timeline = build_stroke_timeline(
            &[
                stroke(60.0, 250.0, 20.0, 100.0, None),
                stroke(120.0, 500.0, 30.0, 120.0, None),
            ],
            Sport::Rower,
            false,
        );
        let before = stroke_pose_at(&timeline, 59.999);
        let boundary = stroke_pose_at(&timeline, 60.0);
        let after = stroke_pose_at(&timeline, 60.001);
        assert!((boundary.phase - 20.0 * TAU).abs() < 1e-8);
        assert!(boundary.phase - before.phase > 0.0 && boundary.phase - before.phase < 0.01);
        assert!(after.phase - boundary.phase > 0.0 && after.phase - boundary.phase < 0.01);
    }

    #[test]
    fn keeps_one_real_cycle_per_api_row_for_every_sport() {
        for sport in [Sport::Rower, Sport::Skierg, Sport::Bike] {
            let timeline = build_stroke_timeline(
                &[
                    stroke(0.0, 0.0, 30.0, 0.0, None),
                    stroke(2.0, 10.0, 30.0, 180.0, None),
                    stroke(4.0, 20.0, 30.0, 190.0, None),
                ],
                sport,
                true,
            );
            assert_eq!(stroke_pose_at(&timeline, 1.0).index, 0);
            assert_eq!(stroke_pose_at(&timeline, 2.0).index, 1);
            assert!((stroke_pose_at(&timeline, 2.0).phase - TAU).abs() < 1e-8);
        }
    }

    #[test]
    fn changes_timing_and_cues_for_harder_late_rows() {
        let timeline = build_stroke_timeline(
            &[
                stroke(3.0, 8.0, 18.0, 90.0, Some(120.0)),
                stroke(5.0, 25.0, 42.0, 420.0, Some(178.0)),
            ],
            Sport::Rower,
            true,
        );
        let low = stroke_pose_at(&timeline, 1.5);
        let high = stroke_pose_at(&timeline, 4.0);
        assert!(high.drive_frac > low.drive_frac);
        assert!(high.amplitude > low.amplitude);
        assert!(high.fatigue > low.fatigue);
    }

    #[test]
    fn detects_catch_transitions_without_firing_on_seeks() {
        let timeline = build_stroke_timeline(
            &[
                stroke(2.0, 10.0, 30.0, 180.0, None),
                stroke(4.0, 20.0, 30.0, 185.0, None),
                stroke(6.0, 30.0, 30.0, 190.0, None),
                stroke(8.0, 40.0, 30.0, 195.0, None),
            ],
            Sport::Rower,
            true,
        );
        let before = stroke_pose_at(&timeline, 3.99);
        let after = stroke_pose_at(&timeline, 4.0);
        let seek = stroke_pose_at(&timeline, 8.0);
        assert_eq!(catch_transitions(Some(&before), Some(&after)), 1);
        assert_eq!(catch_transitions_up_to(Some(&before), Some(&seek), 1), 0);
        assert_eq!(catch_transitions(Some(&after), Some(&before)), 0);
    }

    #[test]
    fn builds_a_bike_fallback_with_symmetric_drive() {
        let pose = fallback_stroke_pose(Sport::Bike, std::f64::consts::PI, 90.0);
        assert_eq!(pose.drive_frac, 0.5);
        assert_eq!(pose.rate, 90.0);
    }

    #[test]
    fn aggregates_watts_dps_and_hr_across_the_timeline() {
        let timeline = build_stroke_timeline(
            &[
                stroke(2.0, 11.0, 28.0, 160.0, Some(140.0)),
                stroke(4.0, 23.0, 30.0, 220.0, Some(160.0)),
                stroke(6.0, 36.0, 32.0, 280.0, Some(178.0)),
            ],
            Sport::Rower,
            true,
        );
        assert_eq!(timeline.median_watts, 220.0);
        assert_eq!(timeline.peak_watts, 280.0);
        assert_eq!(timeline.median_dps, 12.0);
        assert_eq!(timeline.median_hr, 160.0);
        assert_eq!(timeline.max_hr, 178.0);
    }

    #[test]
    fn falls_back_to_defaults_without_watts_or_hr() {
        let timeline = build_stroke_timeline(
            &[
                Stroke {
                    t: 2.0,
                    d: 11.0,
                    pace: 120.0,
                    spm: 28.0,
                    hr: None,
                    watts: 0.0,
                    raw_t: None,
                    raw_d: None,
                },
                Stroke {
                    t: 4.0,
                    d: 22.0,
                    pace: 120.0,
                    spm: 28.0,
                    hr: None,
                    watts: 0.0,
                    raw_t: None,
                    raw_d: None,
                },
            ],
            Sport::Rower,
            true,
        );
        assert_eq!(timeline.median_watts, 0.0);
        assert_eq!(timeline.peak_watts, 0.0);
        assert_eq!(timeline.median_hr, 0.0);
        assert_eq!(timeline.max_hr, 0.0);
        assert_eq!(timeline.median_dps, 11.0);
    }

    #[test]
    fn web_pose_amplitude_is_restrained() {
        // Current web law (post #171): decorative only, 0.94..1.06.
        let pose = fallback_stroke_pose(Sport::Rower, 0.0, 40.0); // intensity 1
        assert!((pose.amplitude - 1.06).abs() < 1e-12);
        let pose = fallback_stroke_pose(Sport::Rower, 0.0, 0.0); // intensity 0
        assert!((pose.amplitude - 0.94).abs() < 1e-12);
    }

    #[test]
    fn reduced_motion_freezes_the_cycle() {
        let pose = fallback_stroke_pose(Sport::Rower, 2.5 * TAU, 30.0);
        let frozen = reduced_motion(&pose);
        assert_eq!(frozen.phase, 0.0);
        assert_eq!(frozen.warped_phase, 0.0);
        assert_eq!(frozen.cycle_frac, 0.0);
        assert!(!frozen.drive);
        assert_eq!(frozen.rate, pose.rate);
    }
}
