// SPDX-License-Identifier: GPL-3.0-or-later
//! Shared animation helpers for the 2D and 3D replay (web `replay/motion.ts`).
//!
//! Everything is pure and frame-rate independent: phases advance by wall-clock
//! dt, smoothing uses exponential decay, and the particle / governor state
//! machines are unit-testable without a renderer.

use crate::models::Sport;

/// One full turn in radians.
const TAU: f64 = std::f64::consts::TAU;

/// Longest frame delta (s) the animation will integrate (web `MAX_DT`).
const MAX_DT: f64 = 0.1;

/// Distance (m) per full stroke/pedal animation cycle, per sport
/// (web `METERS_PER_CYCLE`).
#[must_use]
pub fn meters_per_cycle(sport: Sport) -> f64 {
    match sport {
        Sport::Rower => 11.0,
        Sport::Skierg => 8.0,
        Sport::Bike => 5.0,
    }
}

/// Convert a raw frame delta in milliseconds to clamped seconds (web `clampDt`).
#[must_use]
pub fn clamp_dt(ms: f64) -> f64 {
    if !ms.is_finite() || ms <= 0.0 {
        return 0.0;
    }
    (ms / 1000.0).min(MAX_DT)
}

/// Frame-rate independent smoothing factor for
/// `current += (target - current) * f` (web `dampFactor`).
#[must_use]
pub fn damp_factor(rate: f64, dt: f64) -> f64 {
    1.0 - (-rate * dt.max(0.0)).exp()
}

/// Cubic Hermite ramp on `[0, 1]` with unit rise and endpoint slopes `k`:
/// `h(t, k) = t²(3 − 2t) + k·t(1 − t)(1 − 2t)`.
///
/// `h(0, k) = 0`, `h(1, k) = 1` and `h′(0, k) = h′(1, k) = k`, so chaining
/// two halves with matched slopes is C1 — the building block of the stroke
/// warp below. (The motion graph keeps its own quintic ramps where C2-flat
/// endpoints are required.)
fn hermite(t: f64, k: f64) -> f64 {
    t * t * (3.0 - 2.0 * t) + k * t * (1.0 - t) * (1.0 - 2.0 * t)
}

/// Derivative of [`hermite`], `6t(1 − t) + k(1 − 6t + 6t²)`.
fn hermite_derivative(t: f64, k: f64) -> f64 {
    6.0 * t * (1.0 - t) + k * (1.0 - 6.0 * t + 6.0 * t * t)
}

/// Clamp `driveFrac` the way Studio does before it enters the warp.
fn sanitized_drive_frac(drive_frac: f64) -> f64 {
    if drive_frac.is_finite() {
        drive_frac.clamp(0.01, 0.99)
    } else {
        0.4
    }
}

/// Warp a continuous stroke phase so the drive is quick and the recovery slow
/// (web `warpStrokePhase`).
///
/// Input and output are radians with the catch at multiples of 2π; the drive
/// occupies the first `driveFrac` of each cycle and is remapped onto the first
/// half (0..π) of the output, so `cos(warped)` swings +1 (catch) → −1
/// (finish) fast and eases back through the long recovery.
///
/// **Deliberate divergence (roadmap Phase 2):** the web and Studio versions
/// are piecewise *linear* — C0 at the drive/recovery seam, where the phase
/// velocity jumps by ~`0.5/f ÷ 0.5/(1−f)` (≈2× on SkiErg) and the jump is
/// visible in playback. This port composes each half from a cubic Hermite
/// ramp instead, with the endpoint slope chosen as twice the half's width:
///
/// ```text
/// w(u) = 0.5 · h(u / f, 2f)                     for u <  f   (drive)
///      = 0.5 + 0.5 · h((u − f)/(1 − f), 2(1−f)) for u ≥ f   (recovery)
/// ```
///
/// The contract is unchanged — cycle boundaries map to themselves, the end of
/// the drive maps to half a cycle, the map is monotonic and the drive is
/// faster on average — and the slope choice buys three extra properties:
/// `dw/du` equals 1 at the catch and at the finish from both sides, so the
/// warp is C1 at the seam **and periodic across the cycle boundary**; it is
/// monotonic for every `f` in `[0.01, 0.99]` because the Hermite endpoint
/// slopes never exceed `2·0.99 < 3`, the monotone-cubic ceiling; and it
/// degenerates to the exact identity at `f = 0.5` (since `h(t, 1) = t`), like
/// the web version. `warp_stroke_phase_rate` exposes the analytic derivative
/// and the tests prove the continuity.
#[must_use]
pub fn warp_stroke_phase(phase: f64, drive_frac: f64) -> f64 {
    if !phase.is_finite() {
        return 0.0;
    }
    let cycles = (phase / TAU).floor();
    let u = phase / TAU - cycles; // 0..1 within the cycle
    let f = sanitized_drive_frac(drive_frac);
    let w = if u < f {
        0.5 * hermite(u / f, 2.0 * f)
    } else {
        0.5 + 0.5 * hermite((u - f) / (1.0 - f), 2.0 * (1.0 - f))
    };
    (cycles + w) * TAU
}

/// Analytic derivative of [`warp_stroke_phase`] with respect to the input
/// phase (output radians per input radian, i.e. `dw/du`). Equals 1 at the
/// catch, at the finish and on both sides of the drive/recovery seam, which
/// is what makes the warp C1 and periodic.
#[must_use]
pub fn warp_stroke_phase_rate(phase: f64, drive_frac: f64) -> f64 {
    if !phase.is_finite() {
        return 0.0;
    }
    let cycles = (phase / TAU).floor();
    let u = phase / TAU - cycles;
    let f = sanitized_drive_frac(drive_frac);
    // d((cycles + w)·TAU)/d(phase) = w′(u), since du/dphase = 1/TAU cancels
    // the outer TAU; the chain rule folds the half-width into the endpoint
    // slope: dw/du = h′(·, 2f)/(2f) on the drive and
    // h′(·, 2(1−f))/(2(1−f)) on recovery.
    if u < f {
        hermite_derivative(u / f, 2.0 * f) / (2.0 * f)
    } else {
        hermite_derivative((u - f) / (1.0 - f), 2.0 * (1.0 - f)) / (2.0 * (1.0 - f))
    }
}

/// Hull surge offset for a warped stroke phase (web `strokeSurge`): the shell
/// checks (sits back) at the catch, accelerates through the drive and coasts
/// forward into the finish. Returns −1..1.
#[must_use]
pub fn stroke_surge(warped_phase: f64) -> f64 {
    -warped_phase.cos()
}

/// Count the catches (phase crossing a 2π boundary) between two stroke phases
/// (web `catchEvents`, default `max_cycles = 2`).
#[must_use]
pub fn catch_events(prev_phase: f64, next_phase: f64) -> u32 {
    catch_events_up_to(prev_phase, next_phase, 2)
}

/// [`catch_events`] with an explicit catch ceiling; jumps larger than
/// `max_cycles` (seeks) report 0 so a scrub doesn't fire a burst of splashes.
#[must_use]
pub fn catch_events_up_to(prev_phase: f64, next_phase: f64, max_cycles: u32) -> u32 {
    if !prev_phase.is_finite()
        || !next_phase.is_finite()
        || max_cycles == 0
        || next_phase <= prev_phase
    {
        return 0;
    }
    if next_phase - prev_phase > f64::from(max_cycles) * TAU {
        return 0;
    }
    let crossings = (next_phase / TAU).floor() - (prev_phase / TAU).floor();
    if crossings <= 0.0 || crossings > f64::from(max_cycles) {
        return 0;
    }
    crossings as u32
}

/// Fixed-capacity droplet pool with no allocation after construction (web
/// `ParticlePool`).
///
/// Coordinates are caller-defined: the 2D renderer spawns in CSS pixels with
/// y down, the 3D renderer in metres with y up — gravity's sign is up to the
/// caller.
pub struct ParticlePool {
    capacity: usize,
    alive: usize,
    x: Vec<f64>,
    y: Vec<f64>,
    z: Vec<f64>,
    vx: Vec<f64>,
    vy: Vec<f64>,
    vz: Vec<f64>,
    /// Remaining life (s).
    life: Vec<f64>,
    /// Initial life (s), for fade fractions.
    ttl: Vec<f64>,
    size: Vec<f64>,
}

impl ParticlePool {
    /// Allocate a pool that never allocates again.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        ParticlePool {
            capacity,
            alive: 0,
            x: vec![0.0; capacity],
            y: vec![0.0; capacity],
            z: vec![0.0; capacity],
            vx: vec![0.0; capacity],
            vy: vec![0.0; capacity],
            vz: vec![0.0; capacity],
            life: vec![0.0; capacity],
            ttl: vec![0.0; capacity],
            size: vec![0.0; capacity],
        }
    }

    /// Maximum number of live droplets.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Particles in `[0, alive)` are live; the rest is scratch.
    #[must_use]
    pub fn alive(&self) -> usize {
        self.alive
    }

    /// Spawn one droplet; silently dropped when the pool is full.
    pub fn spawn(
        &mut self,
        x: f64,
        y: f64,
        z: f64,
        vx: f64,
        vy: f64,
        vz: f64,
        life: f64,
        size: f64,
    ) {
        if self.alive >= self.capacity {
            return;
        }
        let i = self.alive;
        self.alive += 1;
        self.x[i] = x;
        self.y[i] = y;
        self.z[i] = z;
        self.vx[i] = vx;
        self.vy[i] = vy;
        self.vz[i] = vz;
        self.life[i] = life;
        self.ttl[i] = life;
        self.size[i] = size;
    }

    /// Integrate gravity + velocity and expire dead droplets
    /// (swap-remove with the last live particle).
    pub fn update(&mut self, dt: f64, gx: f64, gy: f64, gz: f64) {
        let mut i = 0;
        while i < self.alive {
            self.life[i] -= dt;
            if self.life[i] <= 0.0 {
                let last = self.alive - 1;
                self.alive = last;
                if i != last {
                    self.x[i] = self.x[last];
                    self.y[i] = self.y[last];
                    self.z[i] = self.z[last];
                    self.vx[i] = self.vx[last];
                    self.vy[i] = self.vy[last];
                    self.vz[i] = self.vz[last];
                    self.life[i] = self.life[last];
                    self.ttl[i] = self.ttl[last];
                    self.size[i] = self.size[last];
                }
                continue;
            }
            self.vx[i] += gx * dt;
            self.vy[i] += gy * dt;
            self.vz[i] += gz * dt;
            self.x[i] += self.vx[i] * dt;
            self.y[i] += self.vy[i] * dt;
            self.z[i] += self.vz[i] * dt;
            i += 1;
        }
    }

    /// Fraction of life remaining for particle `i`, 1 → 0.
    #[must_use]
    pub fn fade(&self, i: usize) -> f64 {
        let t = self.ttl[i];
        if t > 0.0 { self.life[i] / t } else { 0.0 }
    }

    /// Drop every particle without freeing the storage.
    pub fn clear(&mut self) {
        self.alive = 0;
    }
}

/// Adaptive degradation ladder (web `PerfGovernor`, Studio
/// `ReplayPerformanceGovernor`).
///
/// Watches frame deltas while playing and steps a `level` counter up
/// (0 = full quality) when frames run persistently over budget. Levels are
/// sticky: stepping back up would oscillate on hardware that is right at the
/// edge. The governor first calibrates to the display's observed steady
/// cadence (median + headroom, capped at 2× the floor) so refresh-capped
/// displays are not punished.
pub struct PerfGovernor {
    floor_budget_ms: f64,
    window: u32,
    grace_frames: u32,
    maximum_level: u32,
    calibration: Vec<f64>,
    cal_count: usize,
    budget: Option<f64>,
    ema_ms: f64,
    over: u32,
    grace: u32,
    level: u32,
}

impl PerfGovernor {
    /// Construct with the web constructor's options; `max_level` is required
    /// there and here.
    #[must_use]
    pub fn new(
        budget_ms: f64,
        window: u32,
        grace_frames: u32,
        max_level: u32,
        calibration_frames: usize,
    ) -> Self {
        let calibration_frames = calibration_frames.max(1);
        PerfGovernor {
            floor_budget_ms: if budget_ms.is_finite() && budget_ms > 0.0 {
                budget_ms
            } else {
                22.0
            },
            window: window.max(1),
            grace_frames,
            maximum_level: max_level,
            calibration: vec![0.0; calibration_frames],
            cal_count: 0,
            budget: None,
            ema_ms: 0.0,
            over: 0,
            grace: 0,
            level: 0,
        }
    }

    /// Current degradation level (0 = full quality).
    #[must_use]
    pub fn level(&self) -> u32 {
        self.level
    }

    /// The calibrated working budget, or the floor while calibrating.
    #[must_use]
    pub fn active_budget_ms(&self) -> f64 {
        self.budget.unwrap_or(self.floor_budget_ms)
    }

    /// Whether the calibration window has filled and the budget is final.
    #[must_use]
    pub fn is_calibrated(&self) -> bool {
        self.cal_count == self.calibration.len()
    }

    /// Feed one frame delta in milliseconds. Returns the new level when a
    /// step-down fires, else `None`. Deltas over 250 ms (tab switches, GC
    /// stalls), non-positive and non-finite values are ignored.
    pub fn sample(&mut self, dt_ms: f64) -> Option<u32> {
        if !dt_ms.is_finite() || dt_ms <= 0.0 || dt_ms > 250.0 {
            return None;
        }
        if self.cal_count < self.calibration.len() {
            self.calibration[self.cal_count] = dt_ms;
            self.cal_count += 1;
            if self.cal_count == self.calibration.len() {
                self.finish_calibration();
            }
            return None;
        }
        if self.grace > 0 {
            self.grace -= 1;
            return None;
        }
        if self.level >= self.maximum_level {
            return None;
        }
        // The EMA grows from the calibrated median, so a lone spike can never
        // push it over budget — only a sustained run of slow frames can.
        self.ema_ms = self.ema_ms * 0.9 + dt_ms * 0.1;
        if self.ema_ms > self.active_budget_ms() {
            self.over += 1;
            if self.over >= self.window {
                self.level += 1;
                self.over = 0;
                self.ema_ms = 0.0;
                self.grace = self.grace_frames;
                return Some(self.level);
            }
        } else {
            self.over = 0;
        }
        None
    }

    /// Clear calibration and degradation while keeping the constructor policy.
    pub fn reset(&mut self) {
        self.calibration.fill(0.0);
        self.cal_count = 0;
        self.budget = None;
        self.ema_ms = 0.0;
        self.over = 0;
        self.grace = 0;
        self.level = 0;
    }

    fn finish_calibration(&mut self) {
        let mut sorted = self.calibration.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        // Web/Studio parity: an even window uses the upper middle observation.
        let median = sorted[sorted.len() >> 1];
        let cal_budget_cap = self.floor_budget_ms * 2.0;
        self.budget = Some(cal_budget_cap.min(self.floor_budget_ms.max(median * 1.6)));
        self.ema_ms = median;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_dt_converts_and_clamps() {
        assert!((clamp_dt(16.7) - 0.0167).abs() < 1e-4);
        assert!((clamp_dt(5000.0) - 0.1).abs() < 1e-9);
        assert_eq!(clamp_dt(0.0), 0.0);
        assert_eq!(clamp_dt(-5.0), 0.0);
        assert_eq!(clamp_dt(f64::NAN), 0.0);
        assert_eq!(clamp_dt(f64::INFINITY), 0.0);
    }

    #[test]
    fn damp_factor_is_frame_rate_independent() {
        assert_eq!(damp_factor(8.0, 0.0), 0.0);
        assert!((damp_factor(8.0, 10.0) - 1.0).abs() < 1e-6);
        let rate = 6.0;
        let full = damp_factor(rate, 1.0 / 30.0);
        let half = damp_factor(rate, 1.0 / 60.0);
        assert!((1.0 - (1.0 - half) * (1.0 - half) - full).abs() < 1e-10);
        assert_eq!(damp_factor(8.0, -1.0), 0.0);
    }

    #[test]
    fn warp_maps_cycle_boundaries_onto_themselves() {
        assert!((warp_stroke_phase(0.0, 0.4)).abs() < 1e-10);
        assert!((warp_stroke_phase(TAU, 0.4) - TAU).abs() < 1e-10);
        assert!((warp_stroke_phase(3.0 * TAU, 0.4) - 3.0 * TAU).abs() < 1e-10);
    }

    #[test]
    fn warp_maps_the_end_of_the_drive_to_half_a_cycle() {
        assert!((warp_stroke_phase(0.4 * TAU, 0.4) - std::f64::consts::PI).abs() < 1e-10);
        assert!((warp_stroke_phase(0.3 * TAU, 0.3) - std::f64::consts::PI).abs() < 1e-10);
    }

    #[test]
    fn warp_is_monotonic_within_a_cycle() {
        // The sanitized range including its extremes: the Hermite endpoint
        // slope k = 2·max(f, 1−f) never exceeds 1.98 < 3 (the monotone-cubic
        // ceiling), so every f in [0.01, 0.99] is covered by this set.
        for f in [0.01, 0.2, 0.3, 0.4, 0.5, 0.8, 0.99] {
            let mut prev = -1.0;
            let mut u = 0.0;
            while u <= 1.0 {
                let w = warp_stroke_phase(u * TAU, f);
                assert!(w >= prev, "not monotonic at u={u} f={f}");
                prev = w;
                u += 0.001;
            }
        }
    }

    #[test]
    fn warp_is_the_identity_at_a_symmetric_split() {
        // h(t, 1) = t, so f = 0.5 reproduces the unwarped phase exactly, like
        // the web version at its only symmetric split.
        let mut u = 0.0;
        while u <= 1.0 {
            let warped = warp_stroke_phase(u * TAU, 0.5);
            assert!(
                (warped - u * TAU).abs() <= 1e-12,
                "identity drift at u={u}: {warped} != {}",
                u * TAU
            );
            u += 0.001;
        }
    }

    #[test]
    fn warp_rate_is_one_at_the_catch_the_finish_and_the_seam() {
        // dw/du = h′(0, k)/(half width) = h′(1, k)/(half width) = 1 at u = 0,
        // u = f (either branch) and u → 1: the warp is C1 and periodic.
        for f in [0.01, 0.2, 0.3, 0.34, 0.4, 0.5, 0.8, 0.99] {
            let rate = |phase: f64| warp_stroke_phase_rate(phase, f);
            // Exact knots: both branch evaluations resolve to 1 exactly.
            assert!(
                (rate(0.0) - 1.0).abs() <= 1e-12,
                "f={f}: dw/du at the catch {}",
                rate(0.0)
            );
            assert!(
                (rate(f * TAU) - 1.0).abs() <= 1e-9,
                "f={f}: dw/du at the seam {}",
                rate(f * TAU)
            );
            // Just off the knots the Hermite's second derivative bends the
            // rate, amplified by 1/half-width² at the sanitized extremes
            // (≈5e-6 at f = 0.01), so the band is loose here.
            let h = 1e-9;
            assert!(
                (rate(f * TAU - h) - 1.0).abs() <= 1e-4 && (rate(f * TAU + h) - 1.0).abs() <= 1e-4,
                "f={f}: dw/du beside the seam {} / {}",
                rate(f * TAU - h),
                rate(f * TAU + h)
            );
            // The finish check shares the loose band: at f = 0.99 the
            // recovery half is 0.01 wide, so the same 1/half-width²
            // amplification applies (≈5e-6 at h = 1e-9).
            assert!(
                (rate(TAU - h) - 1.0).abs() <= 1e-4,
                "f={f}: dw/du at 1− {}",
                rate(TAU - h)
            );
        }
        // Numerical one-sided quotients agree for the mid-range splits.
        for f in [0.2, 0.3, 0.34, 0.4, 0.5, 0.8] {
            let seam = f * TAU;
            let h = 1e-6;
            let left = (warp_stroke_phase(seam, f) - warp_stroke_phase(seam - h, f)) / h;
            let right = (warp_stroke_phase(seam + h, f) - warp_stroke_phase(seam, f)) / h;
            assert!(
                (left - 1.0).abs() <= 1e-4 && (right - 1.0).abs() <= 1e-4,
                "f={f}: numeric dw/du at the seam {left} / {right}"
            );
        }
    }

    #[test]
    fn warp_traverses_the_drive_faster_than_the_recovery() {
        let f = 0.4;
        let drive_rate = warp_stroke_phase(f * TAU, f) / (f * TAU);
        let recovery_rate =
            (warp_stroke_phase(TAU, f) - warp_stroke_phase(f * TAU, f)) / ((1.0 - f) * TAU);
        assert!(drive_rate > recovery_rate);
    }

    #[test]
    fn warp_guards_non_finite_phase_and_drive_fraction() {
        assert_eq!(warp_stroke_phase(f64::NAN, 0.4), 0.0);
        assert!(warp_stroke_phase(0.5, f64::NAN).is_finite());
        assert!(warp_stroke_phase(0.5, 0.0).is_finite());
        assert!(warp_stroke_phase(0.5, 1.0).is_finite());
    }

    // ---- the Phase 2 headline: C1 continuity at the seam --------------------

    #[test]
    fn warp_is_c1_continuous_at_the_drive_recovery_seam() {
        // Left/right numerical derivatives agree at the seam for a sweep of
        // drive fractions and cycles. The web/Studio piecewise-linear map has
        // a slope ratio of (1−f)/f there (1.5× at 0.4, ≈1.9× at 0.34) — this
        // test fails against that version.
        for &f in &[0.26, 0.3, 0.34, 0.38, 0.4, 0.46, 0.5] {
            for cycle in 0..3 {
                let seam = (f64::from(cycle) + f) * TAU;
                // h is bounded below by the cancellation floor of the
                // numerical difference quotient near a ~14 rad seam.
                for h in [1e-3, 1e-4, 1e-5, 1e-6] {
                    let left = (warp_stroke_phase(seam, f) - warp_stroke_phase(seam - h, f)) / h;
                    let right = (warp_stroke_phase(seam + h, f) - warp_stroke_phase(seam, f)) / h;
                    // One-sided quotients carry an O(h) bias from the
                    // Hermite's second derivative, so the band scales with it.
                    let tolerance = 1e-6 + 5.0 * h;
                    assert!(
                        (left - right).abs() < tolerance,
                        "f={f} cycle={cycle} h={h}: d−={left} d+={right}"
                    );
                    // And the analytic derivative agrees with both sides.
                    let analytic = warp_stroke_phase_rate(seam, f);
                    assert!(
                        (left - analytic).abs() < tolerance && (right - analytic).abs() < tolerance,
                        "f={f} cycle={cycle}: analytic={analytic} left={left} right={right}"
                    );
                }
            }
        }
    }

    #[test]
    fn warp_is_c1_continuous_at_the_cycle_boundary() {
        for &f in &[0.26, 0.34, 0.4, 0.5] {
            for cycle in 1..3 {
                let seam = f64::from(cycle) * TAU;
                let h = 1e-6;
                let left = (warp_stroke_phase(seam, f) - warp_stroke_phase(seam - h, f)) / h;
                let right = (warp_stroke_phase(seam + h, f) - warp_stroke_phase(seam, f)) / h;
                assert!(
                    (left - right).abs() < 1e-6 + 100.0 * h,
                    "f={f} cycle={cycle}: d−={left} d+={right}"
                );
            }
        }
    }

    #[test]
    fn warp_derivative_is_continuous_across_a_dense_sweep() {
        // Velocity jump between consecutive sweep steps stays bounded by the
        // sweep step's own curvature scale: no discontinuity anywhere in the
        // cycle, including the seam and the catch.
        let f = 0.34; // SkiErg: where the C0 jump was visible
        let steps = 20_000;
        let dt_u = 1.0 / f64::from(steps);
        let rate_at = |u: f64| warp_stroke_phase_rate(u * TAU, f);
        let mut max_jump: f64 = 0.0;
        for i in 0..steps {
            let u = (f64::from(i) + 0.5) * dt_u;
            max_jump = max_jump.max((rate_at(u + dt_u) - rate_at(u)).abs());
        }
        // The per-u derivative peaks around h′(0.5, k)/2f ≈ (1.5−f)/2f ≈ 1.7;
        // any C0 kink would produce a jump comparable to the full range.
        assert!(max_jump < 1e-3, "max derivative jump {max_jump}");
    }

    #[test]
    fn stroke_surge_checks_at_the_catch_and_peaks_at_the_finish() {
        assert_eq!(stroke_surge(0.0), -1.0);
        assert_eq!(stroke_surge(std::f64::consts::PI), 1.0);
        let mut p = 0.0;
        while p < TAU {
            let s = stroke_surge(p);
            assert!((-1.0..=1.0).contains(&s));
            p += 0.1;
        }
    }

    #[test]
    fn catch_events_reports_and_suppresses_correctly() {
        assert_eq!(catch_events(0.9 * TAU, 1.1 * TAU), 1);
        assert_eq!(catch_events(0.2 * TAU, 0.8 * TAU), 0);
        assert_eq!(catch_events(0.0, 50.0 * TAU), 0);
        assert_eq!(catch_events(2.0 * TAU, TAU), 0);
        assert_eq!(catch_events_up_to(0.5 * TAU, 2.6 * TAU, 3), 2);
    }

    #[test]
    fn particle_pool_spawns_integrates_and_expires() {
        let mut p = ParticlePool::new(2);
        p.spawn(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0);
        p.spawn(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0);
        p.spawn(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0);
        assert_eq!(p.alive(), 2);

        let mut p = ParticlePool::new(1);
        p.spawn(0.0, 0.0, 0.0, 10.0, 0.0, 0.0, 1.0, 1.0);
        p.update(0.5, 0.0, -4.0, 0.0);
        assert!((p.x[0] - 5.0).abs() < 1e-5);
        assert!((p.vy[0] - (-2.0)).abs() < 1e-5);
        assert!((p.y[0] - (-1.0)).abs() < 1e-5);

        let mut p = ParticlePool::new(3);
        p.spawn(1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.1, 1.0);
        p.spawn(2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 9.0, 1.0);
        p.spawn(3.0, 0.0, 0.0, 0.0, 0.0, 0.0, 9.0, 1.0);
        p.update(0.2, 0.0, 0.0, 0.0);
        assert_eq!(p.alive(), 2);
        let mut xs = [p.x[0], p.x[1]];
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        assert_eq!(xs, [2.0, 3.0]);

        let mut p = ParticlePool::new(1);
        p.spawn(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0);
        assert_eq!(p.fade(0), 1.0);
        p.update(0.75, 0.0, 0.0, 0.0);
        assert!((p.fade(0) - 0.25).abs() < 1e-5);
        p.clear();
        assert_eq!(p.alive(), 0);
    }

    fn calibrate(g: &mut PerfGovernor, frames: usize, dt: f64) {
        for _ in 0..frames {
            g.sample(dt);
        }
    }

    #[test]
    fn governor_stays_at_level_zero_within_budget() {
        let mut g = PerfGovernor::new(22.0, 10, 90, 3, 30);
        for _ in 0..200 {
            assert_eq!(g.sample(16.0), None);
        }
        assert_eq!(g.level(), 0);
    }

    #[test]
    fn governor_steps_down_after_sustained_slow_frames() {
        let mut g = PerfGovernor::new(22.0, 10, 5, 3, 30);
        calibrate(&mut g, 30, 16.0);
        let mut stepped = None;
        for _ in 0..80 {
            stepped = g.sample(40.0);
            if stepped.is_some() {
                break;
            }
        }
        assert_eq!(stepped, Some(1));
        assert_eq!(g.level(), 1);
    }

    #[test]
    fn governor_tolerates_a_refresh_capped_display() {
        let mut g = PerfGovernor::new(22.0, 10, 5, 3, 30);
        for _ in 0..600 {
            g.sample(33.4);
        }
        assert_eq!(g.level(), 0);
    }

    #[test]
    fn governor_still_degrades_when_a_capped_display_slows() {
        let mut g = PerfGovernor::new(22.0, 10, 5, 3, 30);
        calibrate(&mut g, 30, 33.4);
        let mut stepped = None;
        for _ in 0..100 {
            stepped = g.sample(80.0);
            if stepped.is_some() {
                break;
            }
        }
        assert_eq!(stepped, Some(1));
    }

    #[test]
    fn governor_ignores_single_spikes_and_tab_switches() {
        let mut g = PerfGovernor::new(22.0, 10, 90, 3, 30);
        calibrate(&mut g, 30, 16.0);
        g.sample(200.0);
        for _ in 0..100 {
            g.sample(10.0);
        }
        assert_eq!(g.level(), 0);

        let mut g = PerfGovernor::new(22.0, 5, 0, 2, 30);
        calibrate(&mut g, 30, 16.0);
        for _ in 0..100 {
            g.sample(1000.0);
        }
        assert_eq!(g.level(), 0);
    }

    #[test]
    fn governor_respects_grace_and_max_level() {
        let mut g = PerfGovernor::new(22.0, 5, 50, 3, 30);
        calibrate(&mut g, 30, 16.0);
        while g.level() == 0 {
            g.sample(40.0);
        }
        for _ in 0..50 {
            g.sample(40.0);
        }
        assert_eq!(g.level(), 1);

        let mut g = PerfGovernor::new(22.0, 5, 0, 2, 30);
        calibrate(&mut g, 30, 16.0);
        for _ in 0..500 {
            g.sample(40.0);
        }
        assert_eq!(g.level(), 2);
    }

    #[test]
    fn governor_degrades_on_an_already_overloaded_device() {
        // Calibration at 100 ms/frame: the budget is capped at 2× the floor
        // (44 ms), so sustained 100 ms frames still trigger a step-down.
        let mut g = PerfGovernor::new(22.0, 5, 5, 3, 30);
        calibrate(&mut g, 30, 100.0);
        assert!((g.active_budget_ms() - 44.0).abs() < 1e-9);
        let mut stepped = None;
        for _ in 0..50 {
            stepped = g.sample(100.0);
            if stepped.is_some() {
                break;
            }
        }
        assert_eq!(stepped, Some(1));
    }

    #[test]
    fn meters_per_cycle_covers_every_sport() {
        assert!(meters_per_cycle(Sport::Rower) > 0.0);
        assert!(meters_per_cycle(Sport::Skierg) > 0.0);
        assert!(meters_per_cycle(Sport::Bike) > 0.0);
    }
}
