//! Sketch constraint solver (Phase 6–31).
//! Enforce + residual metric for iterative DOF reduction.

use bevy::prelude::*;
use crate::feature::{SketchCurve, UserConstraint, UserConstraintKind};
use crate::constraints::{SketchConstraint, SketchConstraintKind, detect_hv_constraints};

/// Solve with optional user constraints. Returns adjustments count.
pub fn solve_sketch(
    curves: &mut [SketchCurve],
    user: &[UserConstraint],
    iterations: u32,
) -> u32 {
    let mut total = 0u32;
    for _ in 0..iterations.max(1) {
        let mut changed = 0u32;
        for c in detect_hv_constraints(curves, 1e-4) {
            changed += enforce_auto(curves, &c);
        }
        for u in user {
            changed += enforce_user(curves, u);
        }
        if user.iter().any(|u| u.kind == UserConstraintKind::EqualLength) {
            changed += enforce_equal_lengths(curves);
        }
        total += changed;
        if changed == 0 {
            break;
        }
    }
    total
}

fn enforce_auto(curves: &mut [SketchCurve], c: &SketchConstraint) -> u32 {
    let Some(SketchCurve::Line { a, b }) = curves.get(c.curve_index).cloned() else {
        return 0;
    };
    match c.kind {
        SketchConstraintKind::Horizontal => {
            let mid_y = (a.y + b.y) * 0.5;
            if let Some(SketchCurve::Line { a, b }) = curves.get_mut(c.curve_index) {
                a.y = mid_y;
                b.y = mid_y;
            }
            1
        }
        SketchConstraintKind::Vertical => {
            let mid_x = (a.x + b.x) * 0.5;
            if let Some(SketchCurve::Line { a, b }) = curves.get_mut(c.curve_index) {
                a.x = mid_x;
                b.x = mid_x;
            }
            1
        }
        SketchConstraintKind::EqualLength => 0,
    }
}

fn enforce_user(curves: &mut [SketchCurve], u: &UserConstraint) -> u32 {
    match u.kind {
        UserConstraintKind::Horizontal => {
            if let Some(SketchCurve::Line { a, b }) = curves.get_mut(u.a) {
                let mid_y = (a.y + b.y) * 0.5;
                a.y = mid_y;
                b.y = mid_y;
                return 1;
            }
            0
        }
        UserConstraintKind::Vertical => {
            if let Some(SketchCurve::Line { a, b }) = curves.get_mut(u.a) {
                let mid_x = (a.x + b.x) * 0.5;
                a.x = mid_x;
                b.x = mid_x;
                return 1;
            }
            0
        }
        UserConstraintKind::FixedLength => {
            if let Some(SketchCurve::Line { a, b }) = curves.get_mut(u.a) {
                let d = *b - *a;
                let len = d.length();
                if len < 1e-8 || u.value <= 0.0 {
                    return 0;
                }
                let dir = d / len;
                *b = *a + dir * u.value;
                return 1;
            }
            0
        }
        UserConstraintKind::EqualLength => 0,
        UserConstraintKind::Parallel => {
            // Make direction of b match a (preserve length of b)
            let (dir_a, len_b, start_b) = {
                let Some(SketchCurve::Line { a, b }) = curves.get(u.a) else {
                    return 0;
                };
                let da = *b - *a;
                let la = da.length();
                if la < 1e-8 {
                    return 0;
                }
                let Some(SketchCurve::Line { a: a2, b: b2 }) = curves.get(u.b) else {
                    return 0;
                };
                let lb = (*b2 - *a2).length();
                (da / la, lb, *a2)
            };
            if let Some(SketchCurve::Line { a, b }) = curves.get_mut(u.b) {
                *a = start_b;
                *b = start_b + dir_a * len_b.max(1e-4);
                return 1;
            }
            0
        }
        UserConstraintKind::Coincident => {
            // Move start of b to end of a
            let end_a = {
                let Some(SketchCurve::Line { a: _, b }) = curves.get(u.a) else {
                    return 0;
                };
                *b
            };
            if let Some(SketchCurve::Line { a, b }) = curves.get_mut(u.b) {
                let delta = end_a - *a;
                if delta.length() < 1e-6 {
                    return 0;
                }
                *a += delta;
                *b += delta;
                return 1;
            }
            0
        }
    }
}

fn enforce_equal_lengths(curves: &mut [SketchCurve]) -> u32 {
    let mut lengths = Vec::new();
    for c in curves.iter() {
        if let SketchCurve::Line { a, b } = c {
            lengths.push(a.distance(*b));
        }
    }
    if lengths.len() < 2 {
        return 0;
    }
    let avg = lengths.iter().sum::<f32>() / lengths.len() as f32;
    if avg < 1e-6 {
        return 0;
    }
    let mut changed = 0u32;
    for c in curves.iter_mut() {
        if let SketchCurve::Line { a, b } = c {
            let d = *b - *a;
            let len = d.length();
            if len < 1e-8 {
                continue;
            }
            if (len - avg).abs() > 1e-4 {
                let dir = d / len;
                *b = *a + dir * avg;
                changed += 1;
            }
        }
    }
    changed
}

pub fn solve_body_sketches(body: &mut crate::CadBody) -> u32 {
    let mut n = 0;
    for f in &mut body.features {
        if let crate::CadFeature::Sketch {
            curves,
            constraints,
            ..
        } = f
        {
            let (adj, _r) = solve_sketch_until(curves, constraints, 24, 1e-4);
            n += adj;
            body.dirty = true;
        }
    }
    n
}

pub fn add_constraint_to_last_sketch(
    body: &mut crate::CadBody,
    cns: UserConstraint,
) -> bool {
    for f in body.features.iter_mut().rev() {
        if let crate::CadFeature::Sketch { constraints, .. } = f {
            constraints.push(cns);
            body.dirty = true;
            return true;
        }
    }
    false
}


/// Sum of constraint residuals (lower is better). Phase 31.
pub fn constraint_residual(curves: &[SketchCurve], user: &[UserConstraint]) -> f32 {
    let mut r = 0.0f32;
    for c in detect_hv_constraints(curves, 1e-3) {
        if let Some(SketchCurve::Line { a, b }) = curves.get(c.curve_index) {
            match c.kind {
                SketchConstraintKind::Horizontal => r += (a.y - b.y).abs(),
                SketchConstraintKind::Vertical => r += (a.x - b.x).abs(),
                SketchConstraintKind::EqualLength => {}
            }
        }
    }
    for u in user {
        match u.kind {
            UserConstraintKind::Horizontal => {
                if let Some(SketchCurve::Line { a, b }) = curves.get(u.a) {
                    r += (a.y - b.y).abs();
                }
            }
            UserConstraintKind::Vertical => {
                if let Some(SketchCurve::Line { a, b }) = curves.get(u.a) {
                    r += (a.x - b.x).abs();
                }
            }
            UserConstraintKind::FixedLength => {
                if let Some(SketchCurve::Line { a, b }) = curves.get(u.a) {
                    r += (a.distance(*b) - u.value).abs();
                }
            }
            UserConstraintKind::EqualLength => {
                let mut lens = Vec::new();
                for c in curves {
                    if let SketchCurve::Line { a, b } = c {
                        lens.push(a.distance(*b));
                    }
                }
                if lens.len() >= 2 {
                    let avg = lens.iter().sum::<f32>() / lens.len() as f32;
                    for l in lens {
                        r += (l - avg).abs();
                    }
                }
            }
            UserConstraintKind::Parallel => {
                if let (Some(SketchCurve::Line { a: a0, b: b0 }), Some(SketchCurve::Line { a: a1, b: b1 })) =
                    (curves.get(u.a), curves.get(u.b))
                {
                    let d0 = (*b0 - *a0).normalize_or_zero();
                    let d1 = (*b1 - *a1).normalize_or_zero();
                    r += 1.0 - d0.dot(d1).abs();
                }
            }
            UserConstraintKind::Coincident => {
                if let (Some(SketchCurve::Line { a: _, b: end_a }), Some(SketchCurve::Line { a: start_b, b: _ })) =
                    (curves.get(u.a), curves.get(u.b))
                {
                    r += end_a.distance(*start_b);
                }
            }
        }
    }
    r
}

/// Solve until residual below tol or max iterations. Returns (adjustments, final residual).
pub fn solve_sketch_until(
    curves: &mut [SketchCurve],
    user: &[UserConstraint],
    max_iters: u32,
    tol: f32,
) -> (u32, f32) {
    let mut total = 0u32;
    let mut last_r = constraint_residual(curves, user);
    for _ in 0..max_iters.max(1) {
        let n = solve_sketch(curves, user, 1);
        total += n;
        let r = constraint_residual(curves, user);
        if r < tol || (n == 0 && (last_r - r).abs() < tol * 0.1) {
            return (total, r);
        }
        last_r = r;
    }
    (total, constraint_residual(curves, user))
}
