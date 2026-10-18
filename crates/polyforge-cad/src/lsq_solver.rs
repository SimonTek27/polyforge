//! Finite-difference Gauss–Newton sketch solver (Phase 32).
//! Complements the residual loop with a least-squares step on line endpoints.

use bevy::prelude::*;
use crate::feature::{SketchCurve, UserConstraint, UserConstraintKind};
use crate::constraints::{detect_hv_constraints, SketchConstraintKind};

fn collect_line_indices(curves: &[SketchCurve]) -> Vec<usize> {
    curves
        .iter()
        .enumerate()
        .filter_map(|(i, c)| matches!(c, SketchCurve::Line { .. }).then_some(i))
        .collect()
}

fn pack(curves: &[SketchCurve], lines: &[usize]) -> Vec<f32> {
    let mut v = Vec::with_capacity(lines.len() * 4);
    for &i in lines {
        if let Some(SketchCurve::Line { a, b }) = curves.get(i) {
            v.extend_from_slice(&[a.x, a.y, b.x, b.y]);
        }
    }
    v
}

fn unpack(curves: &mut [SketchCurve], lines: &[usize], v: &[f32]) {
    for (k, &i) in lines.iter().enumerate() {
        if let Some(SketchCurve::Line { a, b }) = curves.get_mut(i) {
            let o = k * 4;
            if o + 3 < v.len() {
                a.x = v[o];
                a.y = v[o + 1];
                b.x = v[o + 2];
                b.y = v[o + 3];
            }
        }
    }
}

fn residuals(curves: &[SketchCurve], user: &[UserConstraint]) -> Vec<f32> {
    let mut r = Vec::new();
    for c in detect_hv_constraints(curves, 1e-3) {
        if let Some(SketchCurve::Line { a, b }) = curves.get(c.curve_index) {
            match c.kind {
                SketchConstraintKind::Horizontal => r.push(a.y - b.y),
                SketchConstraintKind::Vertical => r.push(a.x - b.x),
                SketchConstraintKind::EqualLength => {}
            }
        }
    }
    for u in user {
        match u.kind {
            UserConstraintKind::Horizontal => {
                if let Some(SketchCurve::Line { a, b }) = curves.get(u.a) {
                    r.push(a.y - b.y);
                }
            }
            UserConstraintKind::Vertical => {
                if let Some(SketchCurve::Line { a, b }) = curves.get(u.a) {
                    r.push(a.x - b.x);
                }
            }
            UserConstraintKind::FixedLength => {
                if let Some(SketchCurve::Line { a, b }) = curves.get(u.a) {
                    r.push(a.distance(*b) - u.value);
                }
            }
            UserConstraintKind::EqualLength => {
                let lens: Vec<f32> = curves
                    .iter()
                    .filter_map(|c| {
                        if let SketchCurve::Line { a, b } = c {
                            Some(a.distance(*b))
                        } else {
                            None
                        }
                    })
                    .collect();
                if lens.len() >= 2 {
                    let avg = lens.iter().sum::<f32>() / lens.len() as f32;
                    for l in lens {
                        r.push(l - avg);
                    }
                }
            }
            UserConstraintKind::Parallel => {
                if let (Some(SketchCurve::Line { a: a0, b: b0 }), Some(SketchCurve::Line { a: a1, b: b1 })) =
                    (curves.get(u.a), curves.get(u.b))
                {
                    let d0 = *b0 - *a0;
                    let d1 = *b1 - *a1;
                    // cross product in 2d ~ 0 for parallel
                    r.push(d0.x * d1.y - d0.y * d1.x);
                }
            }
            UserConstraintKind::Coincident => {
                if let (Some(SketchCurve::Line { b: end_a, .. }), Some(SketchCurve::Line { a: start_b, .. })) =
                    (curves.get(u.a), curves.get(u.b))
                {
                    r.push(end_a.x - start_b.x);
                    r.push(end_a.y - start_b.y);
                }
            }
        }
    }
    r
}

fn residual_norm(r: &[f32]) -> f32 {
    r.iter().map(|x| x * x).sum::<f32>().sqrt()
}

/// One Gauss–Newton step with finite-difference Jacobian.
pub fn gauss_newton_step(curves: &mut [SketchCurve], user: &[UserConstraint], eps: f32) -> f32 {
    let lines = collect_line_indices(curves);
    if lines.is_empty() {
        return 0.0;
    }
    let x = pack(curves, &lines);
    let n = x.len();
    let r0 = residuals(curves, user);
    if r0.is_empty() {
        return 0.0;
    }
    let m = r0.len();
    // J is m x n
    let mut j = vec![0.0f32; m * n];
    let mut x_pert = x.clone();
    for col in 0..n {
        x_pert[col] = x[col] + eps;
        unpack(curves, &lines, &x_pert);
        let rp = residuals(curves, user);
        unpack(curves, &lines, &x); // restore before next
        x_pert[col] = x[col];
        for row in 0..m {
            let a = r0.get(row).copied().unwrap_or(0.0);
            let b = rp.get(row).copied().unwrap_or(0.0);
            j[row * n + col] = (b - a) / eps;
        }
    }
    // Solve (J^T J + λI) dx = -J^T r  (damped)
    let lambda = 1e-4f32;
    let mut jtj = vec![0.0f32; n * n];
    let mut jtr = vec![0.0f32; n];
    for i in 0..n {
        for k in 0..n {
            let mut s = 0.0;
            for row in 0..m {
                s += j[row * n + i] * j[row * n + k];
            }
            if i == k {
                s += lambda;
            }
            jtj[i * n + k] = s;
        }
        let mut s = 0.0;
        for row in 0..m {
            s += j[row * n + i] * r0[row];
        }
        jtr[i] = -s;
    }
    // Gaussian elimination (small n)
    let mut a = jtj;
    let mut b = jtr;
    for col in 0..n {
        // pivot
        let mut piv = col;
        for r in col + 1..n {
            if a[r * n + col].abs() > a[piv * n + col].abs() {
                piv = r;
            }
        }
        if a[piv * n + col].abs() < 1e-12 {
            continue;
        }
        if piv != col {
            for k in 0..n {
                a.swap(col * n + k, piv * n + k);
            }
            b.swap(col, piv);
        }
        let diag = a[col * n + col];
        for r in col + 1..n {
            let f = a[r * n + col] / diag;
            for k in col..n {
                a[r * n + k] -= f * a[col * n + k];
            }
            b[r] -= f * b[col];
        }
    }
    let mut dx = vec![0.0f32; n];
    for i in (0..n).rev() {
        let mut s = b[i];
        for k in i + 1..n {
            s -= a[i * n + k] * dx[k];
        }
        let diag = a[i * n + i];
        dx[i] = if diag.abs() > 1e-12 { s / diag } else { 0.0 };
    }
    let mut x_new = x;
    for i in 0..n {
        x_new[i] += dx[i];
    }
    unpack(curves, &lines, &x_new);
    residual_norm(&residuals(curves, user))
}

/// Multi-step LSQ solve. Returns final residual norm.
pub fn solve_sketch_lsq(
    curves: &mut [SketchCurve],
    user: &[UserConstraint],
    steps: u32,
) -> f32 {
    let mut r = residual_norm(&residuals(curves, user));
    for _ in 0..steps.max(1) {
        r = gauss_newton_step(curves, user, 1e-5);
        if r < 1e-5 {
            break;
        }
    }
    r
}

/// Body-level LSQ pass on all sketches.
pub fn solve_body_sketches_lsq(body: &mut crate::CadBody) -> f32 {
    let mut last = 0.0f32;
    for f in &mut body.features {
        if let crate::CadFeature::Sketch {
            curves,
            constraints,
            ..
        } = f
        {
            last = solve_sketch_lsq(curves, constraints, 8);
            body.dirty = true;
        }
    }
    last
}
