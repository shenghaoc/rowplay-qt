// SPDX-License-Identifier: GPL-3.0-or-later
//! Analytic two-bone and rigid-contact solves (Studio `ReplayTwoBoneSolver`;
//! the web's `solvePositionTowardTarget` / `solveRigidContactPoint3D`).
//!
//! Pure vector maths, no allocation, every result finite. `solve3d` places a
//! two-segment chain's middle joint and end point for a target, bending in
//! the plane of a hint; `solve_rigid_contact3d` finds the point on a rigid
//! contact sphere (a planted pole basket) closest to a preferred hand while
//! staying inside the arm's reach annulus.

/// Solver epsilon (Studio `epsilon`).
pub const EPSILON: f64 = 1e-9;

/// A two-bone solution: the middle joint and the end point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TwoBoneSolution {
    /// Middle joint position.
    pub joint: [f64; 3],
    /// End (effector) position, the target when reachable.
    pub end: [f64; 3],
}

/// Place a two-segment chain from `root` toward `target` with segment lengths
/// `first` and `second`, bending toward `bend_hint` (Studio `solve3D`).
#[must_use]
pub fn solve3d(
    root: [f64; 3],
    target: [f64; 3],
    first_length: f64,
    second_length: f64,
    bend_hint: [f64; 3],
) -> TwoBoneSolution {
    let root = finite(root);
    let target = finite(target);
    let first = segment_length(first_length);
    let second = segment_length(second_length);
    let delta = sub(target, root);
    let distance = length(delta);
    if first + second <= EPSILON {
        return TwoBoneSolution {
            joint: root,
            end: root,
        };
    }
    let candidate = if distance > EPSILON {
        scale(delta, 1.0 / distance)
    } else {
        [1.0, 0.0, 0.0]
    };
    let direction = if candidate.iter().all(|v| v.is_finite()) {
        candidate
    } else {
        [1.0, 0.0, 0.0]
    };
    let hint = perpendicular_hint(bend_hint, direction);
    if distance <= EPSILON && (first - second).abs() <= EPSILON {
        return TwoBoneSolution {
            joint: add(root, scale(hint, first)),
            end: root,
        };
    }
    let solve_distance = clamped_reach(distance, first, second);
    let end = if solve_distance == distance {
        target
    } else {
        add(root, scale(direction, solve_distance))
    };
    let safe_distance = solve_distance.max(EPSILON);
    let along =
        (first * first - second * second + safe_distance * safe_distance) / (2.0 * safe_distance);
    let perpendicular = (first * first - along * along).max(0.0).sqrt();
    TwoBoneSolution {
        joint: finite(add(
            add(root, scale(direction, along)),
            scale(hint, perpendicular),
        )),
        end: finite(end),
    }
}

/// The point on the sphere of radius `contact_length` around `contact_center`
/// closest to `preferred` while inside the `[minimum_reach, maximum_reach]`
/// annulus around `root` (Studio `solveRigidContact3D`). The flag is `false`
/// when the two constraints do not intersect; the point then still keeps the
/// rigid contact radius and minimises the residual deterministically.
#[must_use]
pub fn solve_rigid_contact3d(
    root: [f64; 3],
    preferred: [f64; 3],
    contact_center: [f64; 3],
    contact_length: f64,
    minimum_reach: f64,
    maximum_reach: f64,
) -> ([f64; 3], bool) {
    let root = finite(root);
    let center = finite(contact_center);
    let preferred = finite(preferred);
    let radius = segment_length(contact_length);
    let reach_a = segment_length(minimum_reach);
    let reach_b = segment_length(maximum_reach);
    let minimum = reach_a.min(reach_b);
    let maximum = reach_a.max(reach_b);

    let mut preferred_direction = sub(preferred, center);
    let mut preferred_length = length(preferred_direction);
    if preferred_length <= EPSILON {
        preferred_direction = sub(root, center);
        preferred_length = length(preferred_direction);
    }
    if preferred_length <= EPSILON {
        preferred_direction = [1.0, 0.0, 0.0];
        preferred_length = 1.0;
    }
    let candidate = add(
        center,
        scale(preferred_direction, radius / preferred_length),
    );
    let candidate_reach = length(sub(candidate, root));
    if candidate_reach >= minimum - EPSILON && candidate_reach <= maximum + EPSILON {
        return (finite(candidate), true);
    }

    let boundary = if candidate_reach > maximum {
        maximum
    } else {
        minimum
    };
    let mut root_delta = sub(root, center);
    let center_distance = length(root_delta);
    let intersects = center_distance > EPSILON
        && center_distance <= radius + boundary + EPSILON
        && center_distance + radius.min(boundary) + EPSILON >= radius.max(boundary);
    if intersects {
        root_delta = scale(root_delta, 1.0 / center_distance);
        let along = (radius * radius - boundary * boundary + center_distance * center_distance)
            / (2.0 * center_distance);
        let circle_center = add(center, scale(root_delta, along));
        let circle_radius = (radius * radius - along * along).max(0.0).sqrt();
        let plane_direction = perpendicular_hint(sub(preferred, circle_center), root_delta);
        return (
            finite(add(circle_center, scale(plane_direction, circle_radius))),
            true,
        );
    }
    if center_distance > EPSILON {
        return (
            finite(add(center, scale(root_delta, radius / center_distance))),
            false,
        );
    }
    (
        finite(add(
            center,
            scale(preferred_direction, radius / preferred_length),
        )),
        false,
    )
}

/// Clamp a chain's reach into what two segments can span.
#[must_use]
pub fn clamped_reach(distance: f64, first: f64, second: f64) -> f64 {
    (first - second).abs().max((first + second).min(distance))
}

fn segment_length(value: f64) -> f64 {
    if value.is_finite() {
        value.abs().max(0.0)
    } else {
        0.0
    }
}

/// The unit component of `raw_hint` perpendicular to `direction`, with a
/// deterministic axis fallback when the hint is (anti)parallel or empty.
#[must_use]
pub fn perpendicular_hint(raw_hint: [f64; 3], direction: [f64; 3]) -> [f64; 3] {
    let mut hint = finite(raw_hint);
    hint = sub(hint, scale(direction, dot(hint, direction)));
    let mut hint_length = length(hint);
    if hint_length <= EPSILON {
        let abs = [direction[0].abs(), direction[1].abs(), direction[2].abs()];
        let axis = if abs[0] <= abs[1] && abs[0] <= abs[2] {
            [1.0, 0.0, 0.0]
        } else if abs[1] <= abs[2] {
            [0.0, 1.0, 0.0]
        } else {
            [0.0, 0.0, 1.0]
        };
        hint = sub(axis, scale(direction, dot(axis, direction)));
        hint_length = length(hint);
    }
    if hint_length <= EPSILON || !hint_length.is_finite() {
        return [0.0, 1.0, 0.0];
    }
    scale(hint, 1.0 / hint_length)
}

/// Replace non-finite components by zero.
#[must_use]
pub fn finite(value: [f64; 3]) -> [f64; 3] {
    [
        if value[0].is_finite() { value[0] } else { 0.0 },
        if value[1].is_finite() { value[1] } else { 0.0 },
        if value[2].is_finite() { value[2] } else { 0.0 },
    ]
}

/// `a + b`.
#[must_use]
pub fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

/// `a − b`.
#[must_use]
pub fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

/// `v · s`.
#[must_use]
pub fn scale(v: [f64; 3], s: f64) -> [f64; 3] {
    [v[0] * s, v[1] * s, v[2] * s]
}

/// Dot product.
#[must_use]
pub fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Cross product.
#[must_use]
pub fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// Euclidean length.
#[must_use]
pub fn length(v: [f64; 3]) -> f64 {
    dot(v, v).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: [f64; 3], b: [f64; 3]) -> bool {
        length(sub(a, b)) < 1e-9
    }

    #[test]
    fn a_reachable_target_bends_toward_the_hint_and_lands_exactly() {
        let solution = solve3d([0.0; 3], [1.0, 0.0, 0.0], 0.8, 0.8, [0.0, 1.0, 0.0]);
        assert!(close(solution.end, [1.0, 0.0, 0.0]));
        // Isosceles: the joint sits above the midpoint, 0.8 from each end.
        assert!((solution.joint[0] - 0.5).abs() < 1e-9);
        assert!(solution.joint[1] > 0.0);
        assert!((length(sub(solution.joint, [0.0; 3])) - 0.8).abs() < 1e-9);
        assert!((length(sub(solution.end, solution.joint)) - 0.8).abs() < 1e-9);
        // Flipping the hint flips the bend side.
        let other = solve3d([0.0; 3], [1.0, 0.0, 0.0], 0.8, 0.8, [0.0, -1.0, 0.0]);
        assert!(other.joint[1] < 0.0);
    }

    #[test]
    fn out_of_reach_targets_straighten_the_chain_along_the_ray() {
        let solution = solve3d([0.0; 3], [5.0, 0.0, 0.0], 0.5, 0.4, [0.0, 0.0, 1.0]);
        assert!(close(solution.end, [0.9, 0.0, 0.0]));
        assert!(close(solution.joint, [0.5, 0.0, 0.0]));
        // Too close: the chain folds to its minimum reach.
        let folded = solve3d([0.0; 3], [0.01, 0.0, 0.0], 0.5, 0.4, [0.0, 0.0, 1.0]);
        assert!((length(sub(folded.end, [0.0; 3])) - 0.1).abs() < 1e-9);
        assert_eq!(clamped_reach(3.0, 1.0, 1.0), 2.0);
        assert_eq!(clamped_reach(0.0, 1.0, 0.4), 0.6);
    }

    #[test]
    fn a_hint_parallel_to_the_chain_falls_back_deterministically() {
        let hint = perpendicular_hint([1.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
        assert!((length(hint) - 1.0).abs() < 1e-12);
        assert!(dot(hint, [1.0, 0.0, 0.0]).abs() < 1e-12);
        let degenerate = solve3d([0.0; 3], [0.0; 3], 0.0, 0.0, [0.0; 3]);
        assert_eq!(degenerate.joint, [0.0; 3]);
        let nan = solve3d([f64::NAN; 3], [1.0, 0.0, 0.0], 1.0, 1.0, [0.0, 1.0, 0.0]);
        assert!(nan.joint.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn a_rigid_contact_keeps_its_radius_and_prefers_the_hand() {
        // A 1.37 m pole planted 0.9 m ahead and below the shoulder.
        let shoulder = [0.0, 1.3, 0.0];
        let basket = [0.0, 0.0, 0.9];
        let preferred = [0.0, 0.7, 0.3];
        let (point, feasible) = solve_rigid_contact3d(shoulder, preferred, basket, 1.37, 0.0, 0.94);
        assert!(feasible);
        assert!(
            (length(sub(point, basket)) - 1.37).abs() < 1e-9,
            "rigid pole length"
        );
        assert!(
            length(sub(point, shoulder)) <= 0.94 + 1e-9,
            "inside the arm's reach"
        );
        // The reachable candidate straight toward the preferred hand is kept
        // when it lies inside the annulus.
        let near = [0.0, 0.5, 0.2];
        let (kept, _) = solve_rigid_contact3d(shoulder, near, [0.0, 0.5, 0.2 + 0.4], 0.4, 0.0, 2.0);
        assert!(close(kept, near));
        // Impossible constraints report infeasible but stay on the sphere.
        let (far, ok) =
            solve_rigid_contact3d([10.0, 0.0, 0.0], [10.0, 1.0, 0.0], [0.0; 3], 1.0, 0.0, 0.5);
        assert!(!ok);
        assert!((length(far) - 1.0).abs() < 1e-9);
    }
}
