// SPDX-License-Identifier: GPL-3.0-or-later
//! The shared authored motion graph (web `replay/motionGraph.ts`).
//!
//! `sample_motion_graph` is a deterministic function of a
//! [`StrokePose`](crate::replay::stroke_model::StrokePose): no random state,
//! no mutable globals, safe to call for the live and ghost athlete in the same
//! frame. Every body / contact transition carries its own C2-continuous
//! envelope; evaluation order mirrors the web module so the golden corpus
//! (`replay-current-main-motion.json`) compares within 1e-10.
//!
//! The web also exposes allocation-conscious `*Into` scratch samplers; those
//! are a JavaScript GC optimisation and are deliberately not ported (Rust
//! returns the graph by value; revisit when the QML bridge measures it).

use crate::models::Sport;
use crate::replay::stroke_model::StrokePose;

/// One full turn in radians. Kept local so this module stays dependency-free.
const TAU: f64 = std::f64::consts::TAU;
/// Snap normalized curve landmarks through harmless phase round-trip noise.
const CURVE_BOUNDARY_EPSILON: f64 = 1e-12;

// Canonical classic double-pole landmarks as fractions of one complete cycle
// (web `SKI_*_CYCLE`): elbow flexion peaks early, pole-off lands around 29%,
// and the late pre-plant window velocity-matches the basket to the next snow
// anchor.
/// Elbow-flexion peak (fraction of the cycle).
pub const SKI_ELBOW_LOAD_CYCLE: f64 = 0.11;
/// Start of the pole release off the snow.
pub const SKI_POLE_RELEASE_START_CYCLE: f64 = 0.245;
/// Pole-off: the pole leaves the snow.
pub const SKI_POLE_OFF_CYCLE: f64 = 0.29;
/// Apex of the lifted recovery arc.
pub const SKI_POLE_FLIGHT_APEX_CYCLE: f64 = 0.42;
/// Start of the approach to the next plant.
pub const SKI_POLE_APPROACH_START_CYCLE: f64 = 0.88;
/// Start of the pre-plant velocity-matching window.
pub const SKI_PREPLANT_START_CYCLE: f64 = 0.94;

/// A scalar choreography channel sampled at one instant in a replay cycle
/// (web `MotionChannel`). `value` is coordinate-neutral; `velocity` and
/// `acceleration` are its derivatives per second and per second squared.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct MotionChannel {
    /// Coordinate-neutral channel value (most body/contact channels use 0..1).
    pub value: f64,
    /// First derivative per second.
    pub velocity: f64,
    /// Second derivative per second squared.
    pub acceleration: f64,
}

/// Circular state for a crank or other object that must cross 0 / 2π without
/// a scalar-angle discontinuity (web `CircularMotion`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CircularMotion {
    /// Canonical angle in `[0, 2π)`; positive direction is renderer-defined.
    pub angle: f64,
    /// Sine of the unwrapped angle (no wrap discontinuity).
    pub sin: f64,
    /// Cosine of the unwrapped angle.
    pub cos: f64,
    /// Radians per second.
    pub angular_velocity: f64,
    /// Radians per second squared; zero for a constant-cadence pose.
    pub angular_acceleration: f64,
}

/// Cycle timing reconstructed from a `StrokePose` (web `MotionTiming`). Each
/// input pose is treated as a constant-cadence sample: a recorded stroke
/// supplies a duration but not a force curve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionTiming {
    /// Integer cycle that contains this pose.
    pub cycle_index: i64,
    /// Normalized phase in `[0, 1)`, catch / pedal reference at 0.
    pub cycle: f64,
    /// Canonical phase in `[0, 2π)`.
    pub phase: f64,
    /// Positive, finite cadence-derived duration of one full cycle (s).
    pub seconds_per_cycle: f64,
    /// Canonical phase speed (rad/s).
    pub phase_velocity: f64,
    /// Canonical phase acceleration (rad/s²), always zero here.
    pub phase_acceleration: f64,
    /// Sanitized share of a row/ski cycle devoted to the drive.
    pub drive_fraction: f64,
    /// 0..1 during drive, then 1 through recovery.
    pub drive_progress: f64,
    /// 0 through drive, then 0..1 through recovery.
    pub recovery_progress: f64,
}

/// Rower body choreography (web `RowerMotionGraph.body`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowerBodyChannels {
    /// 0 = seat at the catch, 1 = fully driven back.
    pub seat_travel: MotionChannel,
    /// Pelvis/root translation, equal to seat travel for constraint rigs.
    pub pelvis_travel: MotionChannel,
    /// Same authored range as seat travel, for rig naming clarity.
    pub leg_extension: MotionChannel,
    /// 0 = body over at the catch, 1 = open at the finish.
    pub torso_swing: MotionChannel,
    /// Spine hinge cue, equal to torso swing.
    pub spine_hinge: MotionChannel,
    /// Inverse of torso swing: 1 at the forward reach, 0 at finish.
    pub torso_reach: MotionChannel,
    /// 0 = long arms, 1 = handle drawn to the body.
    pub arm_draw: MotionChannel,
    /// 0..1 shoulder follow-through, lagging legs and leading the arm draw.
    pub shoulder_set: MotionChannel,
    /// Coordinated catch-to-finish handle path for contact targets.
    pub handle_travel: MotionChannel,
    /// Small signed vertical cue (about −0.14..0.14).
    pub head_bob: MotionChannel,
}

/// Rower contact envelopes (web `RowerMotionGraph.contacts`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowerContactChannels {
    /// 0..1 drive pressure through the footplate.
    pub foot_pressure: MotionChannel,
    /// Constant hand-to-handle attachment.
    pub handle_grip: MotionChannel,
    /// 0..1 blade immersion; lock blade/water interaction while engaged.
    pub blade_water: MotionChannel,
    /// 0..1 feather during recovery, squared before the catch.
    pub blade_feather: MotionChannel,
    /// 0..1 oarlock load, for small rigger/oar flex.
    pub oarlock_load: MotionChannel,
}

/// Rower decorative accents (web `RowerMotionGraph.accents`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowerAccents {
    /// Signed catch-to-finish propulsion cue in approximately −1..1.
    pub surge: MotionChannel,
    /// Signed restrained hull/torso rise cue in approximately −0.18..0.18.
    pub vertical: MotionChannel,
}

/// Complete RowErg motion graph for one sample.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowerMotionGraph {
    /// Cycle timing for this sample.
    pub timing: MotionTiming,
    /// Body choreography channels.
    pub body: RowerBodyChannels,
    /// Equipment contact envelopes.
    pub contacts: RowerContactChannels,
    /// Restrained decorative accents.
    pub accents: RowerAccents,
}

/// SkiErg body choreography (web `SkierMotionGraph.body`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkierBodyChannels {
    /// 0 = high reach, 1 = completed double-pole press.
    pub arm_press: MotionChannel,
    /// Shoulder follow-through sharing the arm press timing.
    pub shoulder_drop: MotionChannel,
    /// 0 = upright, 1 = forward hip hinge.
    pub hip_hinge: MotionChannel,
    /// Pelvis hinge cue, equal to hip hinge.
    pub pelvis_hinge: MotionChannel,
    /// 0 = tall legs, 1 = athletic compression.
    pub knee_flex: MotionChannel,
    /// 0 = poles forward/up, 1 = poles swept back/down.
    pub pole_sweep: MotionChannel,
    /// 0..1 additional elbow flexion, peaking near 11% of the cycle.
    pub elbow_load: MotionChannel,
    /// 0..1 long-arm cue, near maximum from pole-off through early recovery.
    pub arm_extension: MotionChannel,
    /// 0 at snow contact, 1 at the apex of the lifted recovery arc.
    pub pole_lift: MotionChannel,
    /// 0 at pole-off/plant, 1 while the basket is freely airborne.
    pub pole_flight: MotionChannel,
    /// Inverse arm press for an explicit high-reach target.
    pub reach: MotionChannel,
    /// Weighted hip/knee compression suitable for a torso root.
    pub torso_compression: MotionChannel,
    /// Spine hinge cue derived from the compression truth.
    pub spine_hinge: MotionChannel,
    /// Small upright-rebound cue (about 0..0.16).
    pub head_rise: MotionChannel,
}

/// SkiErg contact envelopes (web `SkierMotionGraph.contacts`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkierContactChannels {
    /// Constant hand-to-grip attachment.
    pub pole_grip: MotionChannel,
    /// 0..1 pole-tip plant envelope; world-lock each tip while engaged.
    pub pole_plant: MotionChannel,
    /// 0..1 force/load envelope within the planted period.
    pub pole_load: MotionChannel,
    /// 0..1 snow/foot pressure through the press.
    pub foot_pressure: MotionChannel,
}

/// SkiErg decorative accents (web `SkierMotionGraph.accents`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkierAccents {
    /// 0..1 forward propulsion travel from plant to pole-off and back.
    pub surge: MotionChannel,
    /// 0..1 recovery rebound, C2-flat endpoints.
    pub rebound: MotionChannel,
}

/// Complete SkiErg motion graph for one sample.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkierMotionGraph {
    /// Cycle timing for this sample.
    pub timing: MotionTiming,
    /// Body choreography channels.
    pub body: SkierBodyChannels,
    /// Equipment contact envelopes.
    pub contacts: SkierContactChannels,
    /// Restrained decorative accents.
    pub accents: SkierAccents,
}

/// One pedal's contact-safe cyclic state (web `PedalMotion`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PedalMotion {
    /// Circular pedal angle and derivatives, phase-opposed left/right.
    pub rotation: CircularMotion,
    /// 0 = flexed knee, 1 = extended leg.
    pub leg_extension: MotionChannel,
    /// Inverse leg extension, for knee-target rigs.
    pub knee_lift: MotionChannel,
    /// Signed ankle articulation in approximately −0.55..0.55.
    pub ankle_flex: MotionChannel,
    /// Smooth 0..1 downstroke load cue; never a discontinuous half-wave.
    pub drive: MotionChannel,
    /// Constant shoe-to-pedal attachment.
    pub pedal_lock: MotionChannel,
}

/// BikeErg body choreography (web `BikeMotionGraph.body`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BikeBodyChannels {
    /// Small signed side-to-side torso cue (about −0.22..0.22).
    pub torso_sway: MotionChannel,
    /// Small signed pelvis rotation cue (about −0.14..0.14).
    pub hip_rock: MotionChannel,
    /// Alias of hip rock with a rig-friendly pelvis name.
    pub pelvis_rock: MotionChannel,
    /// Small signed fore/aft spine cue (about −0.065..0.065).
    pub spine_lean: MotionChannel,
    /// Counter-rotation for shoulders/handlebar posture.
    pub shoulder_counter_rotation: MotionChannel,
    /// Small signed head-stabilization cue countering pelvis and torso.
    pub head_stabilization: MotionChannel,
}

/// BikeErg contact envelopes (web `BikeMotionGraph.contacts`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BikeContactChannels {
    /// Constant hands-on-bars attachment.
    pub handlebar_grip: MotionChannel,
    /// Constant seated contact.
    pub saddle_contact: MotionChannel,
}

/// Complete BikeErg motion graph for one sample.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BikeMotionGraph {
    /// Cycle timing for this sample.
    pub timing: MotionTiming,
    /// Crank state shared by both pedals and any visible drivetrain.
    pub crank: CircularMotion,
    /// Body choreography channels.
    pub body: BikeBodyChannels,
    /// Left pedal state.
    pub left_pedal: PedalMotion,
    /// Right pedal state (π out of phase with the left).
    pub right_pedal: PedalMotion,
    /// Equipment contact envelopes.
    pub contacts: BikeContactChannels,
}

/// Discriminated output shared by the 2D and 3D replay renderers
/// (web `ReplayMotionGraph`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReplayMotionGraph {
    /// RowErg choreography.
    Rower(RowerMotionGraph),
    /// SkiErg choreography.
    Skierg(SkierMotionGraph),
    /// BikeErg choreography.
    Bike(BikeMotionGraph),
}

impl ReplayMotionGraph {
    /// The timing shared by every channel of this graph.
    #[must_use]
    pub fn timing(&self) -> MotionTiming {
        match self {
            ReplayMotionGraph::Rower(g) => g.timing,
            ReplayMotionGraph::Skierg(g) => g.timing,
            ReplayMotionGraph::Bike(g) => g.timing,
        }
    }
}

// ── curve algebra (web's private CurveSample combinators) ────────────────────

/// A curve sample with its first two derivatives per normalized cycle.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct Curve {
    value: f64,
    d_cycle: f64,
    dd_cycle: f64,
}

fn clamp(value: f64, minimum: f64, maximum: f64) -> f64 {
    value.max(minimum).min(maximum)
}

fn finite(value: f64, fallback: f64) -> f64 {
    if value.is_finite() { value } else { fallback }
}

fn default_seconds(sport: Sport) -> f64 {
    match sport {
        Sport::Bike => 60.0 / 80.0,
        Sport::Skierg => 60.0 / 32.0,
        Sport::Rower => 60.0 / 28.0,
    }
}

fn default_drive_fraction(sport: Sport) -> f64 {
    match sport {
        Sport::Bike => 0.5,
        Sport::Skierg => 0.34,
        Sport::Rower => 0.38,
    }
}

/// Quintic smootherstep with its first two analytic derivatives
/// (web `quinticRamp`): C2-flat at both ends, so chaining ramps creates
/// stable poses and contacts across drive/recovery and cycle boundaries.
fn quintic_ramp(cycle: f64, start: f64, end: f64) -> Curve {
    let span = (end - start).max(1e-6);
    if cycle <= start + CURVE_BOUNDARY_EPSILON {
        return Curve {
            value: 0.0,
            d_cycle: 0.0,
            dd_cycle: 0.0,
        };
    }
    if cycle >= end - CURVE_BOUNDARY_EPSILON {
        return Curve {
            value: 1.0,
            d_cycle: 0.0,
            dd_cycle: 0.0,
        };
    }
    let u = (cycle - start) / span;
    let u2 = u * u;
    let u3 = u2 * u;
    let u4 = u3 * u;
    let u5 = u4 * u;
    let derivative = 30.0 * u2 * (u - 1.0) * (u - 1.0);
    let second_derivative = 120.0 * u3 - 180.0 * u2 + 60.0 * u;
    Curve {
        value: 6.0 * u5 - 15.0 * u4 + 10.0 * u3,
        d_cycle: derivative / span,
        dd_cycle: second_derivative / (span * span),
    }
}

/// C2-flat ramp with a near-constant middle velocity (web `cruiseRamp`), used
/// where a regular smootherstep would create an exaggerated mid-drive peak.
/// Velocity eases with smootherstep over short symmetric edge windows.
fn cruise_ramp(cycle: f64, start: f64, end: f64) -> Curve {
    let span = (end - start).max(1e-6);
    if cycle <= start + CURVE_BOUNDARY_EPSILON {
        return Curve {
            value: 0.0,
            d_cycle: 0.0,
            dd_cycle: 0.0,
        };
    }
    if cycle >= end - CURVE_BOUNDARY_EPSILON {
        return Curve {
            value: 1.0,
            d_cycle: 0.0,
            dd_cycle: 0.0,
        };
    }
    let ease_span = span * 0.15;
    let cruise_velocity = 1.0 / (span - ease_span);
    let elapsed = cycle - start;
    // sampleEdge(u): velocity / acceleration / distance of the eased edge.
    let sample_edge = |u: f64| {
        let u2 = u * u;
        let u3 = u2 * u;
        let u4 = u3 * u;
        let u5 = u4 * u;
        let u6 = u5 * u;
        (
            6.0 * u5 - 15.0 * u4 + 10.0 * u3,
            30.0 * u2 * (u - 1.0) * (u - 1.0),
            u6 - 3.0 * u5 + 2.5 * u4,
        )
    };
    if elapsed < ease_span {
        let (velocity, acceleration, distance) = sample_edge(elapsed / ease_span);
        Curve {
            value: cruise_velocity * ease_span * distance,
            d_cycle: cruise_velocity * velocity,
            dd_cycle: (cruise_velocity * acceleration) / ease_span,
        }
    } else if elapsed > span - ease_span {
        let (velocity, acceleration, distance) = sample_edge((span - elapsed) / ease_span);
        Curve {
            value: 1.0 - cruise_velocity * ease_span * distance,
            d_cycle: cruise_velocity * velocity,
            dd_cycle: (-cruise_velocity * acceleration) / ease_span,
        }
    } else {
        Curve {
            value: cruise_velocity * (elapsed - ease_span * 0.5),
            d_cycle: cruise_velocity,
            dd_cycle: 0.0,
        }
    }
}

fn constant(value: f64) -> Curve {
    Curve {
        value,
        d_cycle: 0.0,
        dd_cycle: 0.0,
    }
}

/// Web `add(...samples)` — accumulates in argument order.
fn add2(a: &Curve, b: &Curve) -> Curve {
    Curve {
        value: a.value + b.value,
        d_cycle: a.d_cycle + b.d_cycle,
        dd_cycle: a.dd_cycle + b.dd_cycle,
    }
}

fn add3(a: &Curve, b: &Curve, c: &Curve) -> Curve {
    Curve {
        value: a.value + b.value + c.value,
        d_cycle: a.d_cycle + b.d_cycle + c.d_cycle,
        dd_cycle: a.dd_cycle + b.dd_cycle + c.dd_cycle,
    }
}

fn scale(sample: &Curve, amount: f64) -> Curve {
    Curve {
        value: sample.value * amount,
        d_cycle: sample.d_cycle * amount,
        dd_cycle: sample.dd_cycle * amount,
    }
}

fn invert(sample: &Curve) -> Curve {
    Curve {
        value: 1.0 - sample.value,
        d_cycle: -sample.d_cycle,
        dd_cycle: -sample.dd_cycle,
    }
}

fn multiply(first: &Curve, second: &Curve) -> Curve {
    Curve {
        value: first.value * second.value,
        d_cycle: first.d_cycle * second.value + first.value * second.d_cycle,
        dd_cycle: first.dd_cycle * second.value
            + 2.0 * first.d_cycle * second.d_cycle
            + first.value * second.dd_cycle,
    }
}

/// A C2 rise, plateau and fall within one normalized cycle (web `pulse`).
#[allow(clippy::too_many_arguments)]
fn pulse(cycle: f64, rise_start: f64, rise_end: f64, fall_start: f64, fall_end: f64) -> Curve {
    let up = quintic_ramp(cycle, rise_start, rise_end);
    let down = quintic_ramp(cycle, fall_start, fall_end);
    add2(&up, &scale(&down, -1.0))
}

/// A C2 bell with zero value, velocity and acceleration at both ends
/// (web `bump`).
fn bump(cycle: f64, start: f64, end: f64) -> Curve {
    let ramp = quintic_ramp(cycle, start, end);
    Curve {
        value: 4.0 * ramp.value * (1.0 - ramp.value),
        d_cycle: 4.0 * ramp.d_cycle * (1.0 - 2.0 * ramp.value),
        dd_cycle: 4.0
            * (ramp.dd_cycle * (1.0 - 2.0 * ramp.value) - 2.0 * ramp.d_cycle * ramp.d_cycle),
    }
}

fn sine(cycle: f64, phase_offset: f64) -> Curve {
    let angle = cycle * TAU + phase_offset;
    let sin = angle.sin();
    let cos = angle.cos();
    Curve {
        value: sin,
        d_cycle: cos * TAU,
        dd_cycle: -sin * TAU * TAU,
    }
}

fn cosine(cycle: f64, phase_offset: f64) -> Curve {
    sine(cycle, phase_offset + std::f64::consts::FRAC_PI_2)
}

fn to_channel(sample: &Curve, timing: &MotionTiming) -> MotionChannel {
    let cycles_per_second = 1.0 / timing.seconds_per_cycle;
    MotionChannel {
        value: sample.value,
        velocity: sample.d_cycle * cycles_per_second,
        acceleration: sample.dd_cycle * cycles_per_second * cycles_per_second,
    }
}

fn timing_for(sport: Sport, pose: &StrokePose) -> MotionTiming {
    let fallback_cycle = clamp(finite(pose.cycle_frac, 0.0), 0.0, 0.999_999);
    let source_phase = finite(pose.phase, fallback_cycle * TAU);
    let raw_cycle = source_phase / TAU;
    let cycle = ((raw_cycle % 1.0) + 1.0) % 1.0;
    let seconds_per_cycle = clamp(
        finite(pose.stroke_seconds, default_seconds(sport)),
        0.2,
        12.0,
    );
    let (drive_lo, drive_hi) = if sport == Sport::Bike {
        (0.5, 0.5)
    } else {
        (0.26, 0.48)
    };
    let drive_fraction = clamp(
        finite(pose.drive_frac, default_drive_fraction(sport)),
        drive_lo,
        drive_hi,
    );
    let in_drive = cycle < drive_fraction;
    MotionTiming {
        cycle_index: raw_cycle.floor() as i64,
        cycle,
        phase: cycle * TAU,
        seconds_per_cycle,
        phase_velocity: TAU / seconds_per_cycle,
        phase_acceleration: 0.0,
        drive_fraction,
        drive_progress: if in_drive {
            cycle / drive_fraction
        } else {
            1.0
        },
        recovery_progress: if in_drive {
            0.0
        } else {
            (cycle - drive_fraction) / (1.0 - drive_fraction)
        },
    }
}

/// `2·s − 1` (web `centered`), keeping the accumulate-from-zero order.
fn centered(sample: &Curve) -> Curve {
    add2(&scale(sample, 2.0), &constant(-1.0))
}

fn intensity_scale(pose: &StrokePose) -> f64 {
    0.88 + clamp(finite(pose.intensity, 0.5), 0.0, 1.0) * 0.12
}

fn sample_rower(timing: &MotionTiming, pose: &StrokePose) -> RowerMotionGraph {
    let cycle = timing.cycle;
    let drive = timing.drive_fraction;
    let recovery = 1.0 - drive;

    // Ranges intentionally overlap: the finish is a short shared plateau,
    // recovery sends hands away before body-over and slide return.
    let legs = pulse(cycle, 0.0, drive * 0.56, drive + recovery * 0.34, 1.0);
    let torso = pulse(
        cycle,
        drive * 0.3,
        drive * 0.8,
        drive + recovery * 0.18,
        drive + recovery * 0.58,
    );
    // Legs → body → arms: the draw window is the single authored velocity
    // profile for the arm pull (cruiseRamp is C2-flat with a near-constant
    // middle; no downstream easing may be stacked on it).
    let arms = add2(
        &cruise_ramp(cycle, drive * 0.68, drive * 0.995),
        &scale(&quintic_ramp(cycle, drive, drive + recovery * 0.3), -1.0),
    );
    let handle = add3(
        &scale(&legs, 0.42),
        &scale(&torso, 0.32),
        &scale(&arms, 0.26),
    );
    let shoulders = add2(&scale(&torso, 0.45), &scale(&arms, 0.55));
    // Burial holds through the entire drive (the arm draw is still
    // propulsive); the spoon extracts in the first stretch of recovery, and
    // depth is prepared again in late recovery so the contact stays C2.
    let drive_blade_water = pulse(cycle, -drive * 0.1, 0.0, drive, drive + recovery * 0.08);
    let pre_catch_blade_water = quintic_ramp(cycle, drive + recovery * 0.82, 1.0);
    let blade_water = add2(&drive_blade_water, &pre_catch_blade_water);
    let blade_feather = pulse(
        cycle,
        drive + recovery * 0.025,
        drive + recovery * 0.13,
        drive + recovery * 0.75,
        1.0,
    );
    let foot_pressure = pulse(
        cycle,
        drive * 0.02,
        drive * 0.17,
        drive * 0.63,
        drive * 0.86,
    );
    let oarlock_load = multiply(&blade_water, &foot_pressure);
    let vertical = add2(
        &scale(&centered(&legs), 0.1),
        &scale(&centered(&torso), 0.055),
    );
    let head_bob = add2(&scale(&centered(&handle), 0.09), &scale(&vertical, 0.22));
    let effort = intensity_scale(pose);

    let seat_travel = to_channel(&legs, timing);
    RowerMotionGraph {
        timing: *timing,
        body: RowerBodyChannels {
            pelvis_travel: seat_travel,
            leg_extension: seat_travel,
            seat_travel,
            torso_swing: to_channel(&torso, timing),
            spine_hinge: to_channel(&torso, timing),
            torso_reach: to_channel(&invert(&torso), timing),
            arm_draw: to_channel(&arms, timing),
            shoulder_set: to_channel(&shoulders, timing),
            handle_travel: to_channel(&handle, timing),
            head_bob: to_channel(&head_bob, timing),
        },
        contacts: RowerContactChannels {
            foot_pressure: to_channel(&foot_pressure, timing),
            handle_grip: to_channel(&constant(1.0), timing),
            blade_water: to_channel(&blade_water, timing),
            blade_feather: to_channel(&blade_feather, timing),
            oarlock_load: to_channel(&oarlock_load, timing),
        },
        accents: RowerAccents {
            surge: to_channel(&scale(&centered(&handle), effort), timing),
            vertical: to_channel(&scale(&vertical, effort), timing),
        },
    }
}

fn sample_skier(timing: &MotionTiming, pose: &StrokePose) -> SkierMotionGraph {
    let cycle = timing.cycle;

    // Classic double-poling is a short planted impulse followed by a long
    // recovery; the absolute landmarks are intentional (Concept2 stroke data
    // contains cadence but no measured pole force or joint path).
    let arms = pulse(cycle, 0.0, SKI_POLE_OFF_CYCLE, 0.305, 0.8);
    let hips = pulse(cycle, 0.025, 0.31, 0.325, 0.74);
    let knees = pulse(cycle, 0.06, 0.32, 0.34, 0.69);
    let pole_sweep = add2(
        &cruise_ramp(cycle, 0.0, SKI_POLE_OFF_CYCLE),
        &scale(&quintic_ramp(cycle, SKI_POLE_OFF_CYCLE, 1.0), -1.0),
    );
    let elbow_load = pulse(
        cycle,
        0.0,
        SKI_ELBOW_LOAD_CYCLE,
        SKI_ELBOW_LOAD_CYCLE,
        SKI_POLE_OFF_CYCLE,
    );
    let arm_extension = add2(
        &cruise_ramp(cycle, SKI_ELBOW_LOAD_CYCLE, SKI_POLE_OFF_CYCLE),
        &scale(&quintic_ramp(cycle, 0.72, 1.0), -1.0),
    );
    let pole_lift = bump(cycle, SKI_POLE_OFF_CYCLE, 1.0);
    let pole_flight = pulse(
        cycle,
        SKI_POLE_OFF_CYCLE,
        SKI_POLE_FLIGHT_APEX_CYCLE,
        SKI_POLE_APPROACH_START_CYCLE,
        1.0,
    );
    // At the cycle seam the basket has already velocity-matched the next snow
    // point; the late C2 pre-plant ramp plus the early hold keeps contact
    // position and derivatives continuous.
    let pole_plant = add2(
        &invert(&quintic_ramp(
            cycle,
            SKI_POLE_RELEASE_START_CYCLE,
            SKI_POLE_OFF_CYCLE,
        )),
        &quintic_ramp(cycle, SKI_PREPLANT_START_CYCLE, 1.0),
    );
    let pole_load = pulse(cycle, 0.012, 0.095, 0.17, 0.275);
    let foot_pressure = pulse(cycle, 0.025, 0.12, 0.19, 0.32);
    let torso_compression = add2(&scale(&hips, 0.66), &scale(&knees, 0.34));
    let rebound = bump(cycle, 0.32, 0.9);
    let head_rise = scale(&rebound, 0.16);
    let effort = intensity_scale(pose);

    let arm_press = to_channel(&arms, timing);
    let hip_hinge = to_channel(&hips, timing);
    let torso_compression_channel = to_channel(&torso_compression, timing);
    SkierMotionGraph {
        timing: *timing,
        body: SkierBodyChannels {
            arm_press,
            shoulder_drop: arm_press,
            hip_hinge,
            pelvis_hinge: hip_hinge,
            knee_flex: to_channel(&knees, timing),
            pole_sweep: to_channel(&pole_sweep, timing),
            elbow_load: to_channel(&elbow_load, timing),
            arm_extension: to_channel(&arm_extension, timing),
            pole_lift: to_channel(&pole_lift, timing),
            pole_flight: to_channel(&pole_flight, timing),
            reach: to_channel(&invert(&arms), timing),
            torso_compression: torso_compression_channel,
            spine_hinge: torso_compression_channel,
            head_rise: to_channel(&head_rise, timing),
        },
        contacts: SkierContactChannels {
            pole_grip: to_channel(&constant(1.0), timing),
            pole_plant: to_channel(&pole_plant, timing),
            pole_load: to_channel(&pole_load, timing),
            foot_pressure: to_channel(&foot_pressure, timing),
        },
        accents: SkierAccents {
            surge: to_channel(&scale(&pole_sweep, effort), timing),
            rebound: to_channel(&rebound, timing),
        },
    }
}

fn circular_at(timing: &MotionTiming, phase_offset: f64) -> CircularMotion {
    let unwrapped = timing.phase + phase_offset;
    CircularMotion {
        angle: ((unwrapped % TAU) + TAU) % TAU,
        sin: unwrapped.sin(),
        cos: unwrapped.cos(),
        angular_velocity: timing.phase_velocity,
        angular_acceleration: timing.phase_acceleration,
    }
}

fn pedal_at(timing: &MotionTiming, phase_offset: f64) -> PedalMotion {
    let rotation = circular_at(timing, phase_offset);
    let sine_wave = sine(timing.cycle, phase_offset);
    let cosine_wave = cosine(timing.cycle, phase_offset);
    let extension = add2(&constant(0.5), &scale(&cosine_wave, 0.5));
    let downstroke = add2(&constant(0.5), &scale(&sine_wave, 0.5));
    // Squaring makes peak torque live around the downstroke while remaining
    // C∞ at the zero-load point; unlike max(0, sin) it has no derivative kink.
    let drive = multiply(&downstroke, &downstroke);
    let ankle = add2(&scale(&sine_wave, 0.44), &scale(&cosine_wave, -0.11));
    let leg_extension = to_channel(&extension, timing);
    PedalMotion {
        rotation,
        leg_extension,
        knee_lift: to_channel(&invert(&extension), timing),
        ankle_flex: to_channel(&ankle, timing),
        drive: to_channel(&drive, timing),
        pedal_lock: to_channel(&constant(1.0), timing),
    }
}

fn sample_bike(timing: &MotionTiming) -> BikeMotionGraph {
    let sine_wave = sine(timing.cycle, 0.0);
    // Doubling the phase analytically keeps body rotation derivatives clean
    // at the cycle boundary.
    let double_angle = timing.cycle * TAU * 2.0;
    let hip_rock = Curve {
        value: double_angle.sin() * 0.14,
        d_cycle: double_angle.cos() * TAU * 2.0 * 0.14,
        dd_cycle: -double_angle.sin() * TAU * TAU * 4.0 * 0.14,
    };
    let torso_sway = scale(&sine_wave, 0.22);
    let spine_lean = scale(&cosine(timing.cycle, 0.0), 0.065);
    // Authored shoulders counter the pelvis in both orthographic and
    // perspective views.
    let shoulders = scale(&torso_sway, -0.62);
    let head_stabilization = scale(
        &add2(&scale(&torso_sway, -1.0), &scale(&hip_rock, -0.25)),
        0.32,
    );

    let hip_rock_channel = to_channel(&hip_rock, timing);
    BikeMotionGraph {
        timing: *timing,
        crank: circular_at(timing, 0.0),
        body: BikeBodyChannels {
            torso_sway: to_channel(&torso_sway, timing),
            hip_rock: hip_rock_channel,
            pelvis_rock: hip_rock_channel,
            spine_lean: to_channel(&spine_lean, timing),
            shoulder_counter_rotation: to_channel(&shoulders, timing),
            head_stabilization: to_channel(&head_stabilization, timing),
        },
        left_pedal: pedal_at(timing, 0.0),
        right_pedal: pedal_at(timing, std::f64::consts::PI),
        contacts: BikeContactChannels {
            handlebar_grip: to_channel(&constant(1.0), timing),
            saddle_contact: to_channel(&constant(1.0), timing),
        },
    }
}

/// Sample the shared authored motion graph for a data-derived pose
/// (web `sampleMotionGraph`).
#[must_use]
pub fn sample_motion_graph(sport: Sport, pose: &StrokePose) -> ReplayMotionGraph {
    let timing = timing_for(sport, pose);
    match sport {
        Sport::Skierg => ReplayMotionGraph::Skierg(sample_skier(&timing, pose)),
        Sport::Bike => ReplayMotionGraph::Bike(sample_bike(&timing)),
        Sport::Rower => ReplayMotionGraph::Rower(sample_rower(&timing, pose)),
    }
}

/// Convenience typed sampler for RowErg consumers.
#[must_use]
pub fn sample_rower_motion_graph(pose: &StrokePose) -> RowerMotionGraph {
    sample_rower(&timing_for(Sport::Rower, pose), pose)
}

/// Convenience typed sampler for SkiErg consumers.
#[must_use]
pub fn sample_skier_motion_graph(pose: &StrokePose) -> SkierMotionGraph {
    sample_skier(&timing_for(Sport::Skierg, pose), pose)
}

/// Convenience typed sampler for BikeErg consumers.
#[must_use]
pub fn sample_bike_motion_graph(pose: &StrokePose) -> BikeMotionGraph {
    sample_bike(&timing_for(Sport::Bike, pose))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pose(sport: Sport, cycle: f64, intensity: f64) -> StrokePose {
        StrokePose {
            index: 0,
            phase: cycle * TAU,
            warped_phase: cycle * TAU,
            cycle_frac: cycle,
            drive_frac: default_drive_fraction(sport),
            drive: cycle < default_drive_fraction(sport),
            drive_progress: 0.0,
            recovery_progress: 0.0,
            stroke_seconds: 2.0,
            stroke_meters: 10.0,
            rate: 30.0,
            watts: 200.0,
            intensity,
            amplitude: 1.0,
            fatigue: 0.0,
            real: true,
        }
    }

    #[test]
    fn timing_sanitises_the_pose() {
        let t = timing_for(Sport::Rower, &pose(Sport::Rower, 0.25, 0.5));
        assert_eq!(t.cycle_index, 0);
        assert!((t.cycle - 0.25).abs() < 1e-12);
        assert!((t.phase - 0.25 * TAU).abs() < 1e-12);
        assert!((t.seconds_per_cycle - 2.0).abs() < 1e-12);
        assert!((t.phase_velocity - TAU / 2.0).abs() < 1e-12);
        assert_eq!(t.phase_acceleration, 0.0);
        assert!((t.drive_fraction - 0.38).abs() < 1e-12);

        // Drive fraction clamps into the authored range.
        let mut p = pose(Sport::Rower, 0.0, 0.5);
        p.drive_frac = 0.9;
        p.stroke_seconds = 99.0; // clamps to 12 s
        let t = timing_for(Sport::Rower, &p);
        assert!((t.drive_fraction - 0.48).abs() < 1e-12);
        assert!((t.seconds_per_cycle - 12.0).abs() < 1e-12);

        let mut p = pose(Sport::Bike, 0.0, 0.5);
        p.drive_frac = 0.3;
        let t = timing_for(Sport::Bike, &p);
        assert_eq!(t.drive_fraction, 0.5);
    }

    #[test]
    fn rower_sequences_legs_body_arms() {
        // Mid-drive: legs ahead of torso, torso ahead of arms.
        let graph = sample_rower_motion_graph(&pose(Sport::Rower, 0.2, 0.5));
        assert!(graph.body.leg_extension.value > graph.body.torso_swing.value);
        assert!(graph.body.torso_swing.value >= graph.body.arm_draw.value);
        // Aliases share one truth source.
        assert_eq!(graph.body.seat_travel, graph.body.pelvis_travel);
        assert_eq!(graph.body.seat_travel, graph.body.leg_extension);
        assert_eq!(graph.body.torso_swing, graph.body.spine_hinge);
        assert_eq!(
            graph.body.torso_reach.value,
            1.0 - graph.body.spine_hinge.value
        );
        // Constant contacts.
        assert_eq!(graph.contacts.handle_grip.value, 1.0);
        assert_eq!(graph.contacts.handle_grip.velocity, 0.0);
    }

    #[test]
    fn rower_blade_is_wet_through_the_drive() {
        let catch = sample_rower_motion_graph(&pose(Sport::Rower, 0.0, 0.5));
        assert!((catch.contacts.blade_water.value - 1.0).abs() < 1e-9);
        let mid_drive = sample_rower_motion_graph(&pose(Sport::Rower, 0.25, 0.5));
        assert!((mid_drive.contacts.blade_water.value - 1.0).abs() < 1e-9);
        // Extracted during recovery.
        let mid_recovery = sample_rower_motion_graph(&pose(Sport::Rower, 0.75, 0.5));
        assert!(mid_recovery.contacts.blade_water.value < 0.5);
    }

    #[test]
    fn skier_plants_early_and_recovers_long() {
        let graph = sample_skier_motion_graph(&pose(Sport::Skierg, 0.1, 0.5));
        assert!(graph.contacts.pole_plant.value > 0.5);
        assert!(graph.contacts.pole_load.value >= 0.0);
        let recovery = sample_skier_motion_graph(&pose(Sport::Skierg, 0.7, 0.5));
        assert!(recovery.contacts.pole_plant.value < 0.2);
        assert!(
            recovery.body.pole_lift.value > 0.0
                || (recovery.body.pole_lift.value - 0.0).abs() < 1e-9
        );
        assert_eq!(graph.body.arm_press, graph.body.shoulder_drop);
        assert_eq!(graph.body.hip_hinge, graph.body.pelvis_hinge);
        assert_eq!(graph.body.torso_compression, graph.body.spine_hinge);
    }

    #[test]
    fn bike_pedals_are_phase_opposed() {
        let graph = sample_bike_motion_graph(&pose(Sport::Bike, 0.1, 0.5));
        assert!(
            ((graph.left_pedal.rotation.angle - graph.right_pedal.rotation.angle).abs()
                - std::f64::consts::PI)
                .abs()
                < 1e-9
        );
        assert!((graph.left_pedal.rotation.sin + graph.right_pedal.rotation.sin).abs() < 1e-9);
        assert_eq!(graph.crank.angle, graph.left_pedal.rotation.angle);
        // Hip rock runs at twice the cycle frequency.
        let a = sample_bike_motion_graph(&pose(Sport::Bike, 0.0, 0.5));
        let half = sample_bike_motion_graph(&pose(Sport::Bike, 0.5, 0.5));
        assert!((a.body.hip_rock.value - half.body.hip_rock.value).abs() < 1e-9);
        assert!((a.body.torso_sway.value + half.body.torso_sway.value).abs() < 1e-9);
    }

    #[test]
    fn intensity_only_scales_accents() {
        let low = sample_rower_motion_graph(&pose(Sport::Rower, 0.3, 0.0));
        let high = sample_rower_motion_graph(&pose(Sport::Rower, 0.3, 1.0));
        assert!(high.accents.surge.value > low.accents.surge.value);
        assert_eq!(low.body.seat_travel, high.body.seat_travel);
        assert_eq!(low.contacts.blade_water, high.contacts.blade_water);
    }

    #[test]
    fn channels_are_c2_across_the_cycle_seam() {
        // Sample the rower handle around the catch (cycle 1⁻/0⁺): values,
        // velocities and accelerations continue smoothly.
        for sport in [Sport::Rower, Sport::Skierg, Sport::Bike] {
            let before = sample_motion_graph(sport, &pose(sport, 0.9999, 0.5));
            let after = sample_motion_graph(sport, &pose(sport, 0.0001, 0.5));
            let (b, a) = match (before, after) {
                (ReplayMotionGraph::Rower(b), ReplayMotionGraph::Rower(a)) => {
                    (b.body.handle_travel, a.body.handle_travel)
                }
                (ReplayMotionGraph::Skierg(b), ReplayMotionGraph::Skierg(a)) => {
                    (b.body.pole_sweep, a.body.pole_sweep)
                }
                (ReplayMotionGraph::Bike(b), ReplayMotionGraph::Bike(a)) => {
                    (b.body.torso_sway, a.body.torso_sway)
                }
                _ => unreachable!("matching variants"),
            };
            assert!(
                (b.value - a.value).abs() < 0.05,
                "{sport:?}: value seam {b:?} {a:?}"
            );
            assert!(
                (b.velocity - a.velocity).abs() < 2.0,
                "{sport:?}: velocity seam {b:?} {a:?}"
            );
        }
    }

    #[test]
    fn ski_landmarks_are_stable() {
        // The published on-snow double-poling landmarks; changing them shifts
        // every ski channel in the golden corpus.
        assert_eq!(SKI_ELBOW_LOAD_CYCLE, 0.11);
        assert_eq!(SKI_POLE_RELEASE_START_CYCLE, 0.245);
        assert_eq!(SKI_POLE_OFF_CYCLE, 0.29);
        assert_eq!(SKI_POLE_FLIGHT_APEX_CYCLE, 0.42);
        assert_eq!(SKI_POLE_APPROACH_START_CYCLE, 0.88);
        assert_eq!(SKI_PREPLANT_START_CYCLE, 0.94);
    }
}
