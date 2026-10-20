//! Bezier continuity helpers (Phase 14).

use bevy::prelude::*;
use crate::BezierSegment;

/// Enforce G1 at the joint between two consecutive cubic segments:
/// end of `a` matches start of `b`, and handles are collinear with proportional length option.
pub fn enforce_g1(a: &mut BezierSegment, b: &mut BezierSegment, keep_lengths: bool) {
    // Positional continuity G0
    let mid = (a.p3 + b.p0) * 0.5;
    a.p3 = mid;
    b.p0 = mid;

    // Incoming / outgoing handle directions
    let mut v_in = a.p3 - a.p2;
    let mut v_out = b.p1 - b.p0;
    let len_in = v_in.length();
    let len_out = v_out.length();
    if len_in < 1e-8 && len_out < 1e-8 {
        return;
    }
    // Average direction
    let dir = {
        let d = if len_in > 1e-8 { v_in.normalize() } else { Vec2::ZERO }
            + if len_out > 1e-8 {
                v_out.normalize()
            } else {
                Vec2::ZERO
            };
        if d.length_squared() < 1e-12 {
            return;
        }
        d.normalize()
    };
    let li = if keep_lengths && len_in > 1e-8 {
        len_in
    } else {
        len_in.max(len_out) * 0.5
    };
    let lo = if keep_lengths && len_out > 1e-8 {
        len_out
    } else {
        len_in.max(len_out) * 0.5
    };
    a.p2 = a.p3 - dir * li;
    b.p1 = b.p0 + dir * lo;
    let _ = (v_in, v_out);
}

/// Apply G1 across all consecutive pairs in a path's bezier list.
pub fn path_enforce_g1(beziers: &mut [BezierSegment], keep_lengths: bool) -> usize {
    if beziers.len() < 2 {
        return 0;
    }
    let mut n = 0;
    for i in 0..beziers.len() - 1 {
        // split borrow
        let (left, right) = beziers.split_at_mut(i + 1);
        enforce_g1(&mut left[i], &mut right[0], keep_lengths);
        n += 1;
    }
    // Close loop G1 if first.p0 near last.p3
    if beziers.len() >= 2 {
        let first_p0 = beziers[0].p0;
        let last_p3 = beziers[beziers.len() - 1].p3;
        if first_p0.distance(last_p3) < 0.05 {
            let len = beziers.len();
            let (left, right) = beziers.split_at_mut(len - 1);
            // treat last -> first
            let mut a = right[0].clone();
            let mut b = left[0].clone();
            enforce_g1(&mut a, &mut b, keep_lengths);
            right[0] = a;
            left[0] = b;
            n += 1;
        }
    }
    n
}

/// Chaikin-style corner smoothing on polyline points (1 iteration).
pub fn smooth_polyline(points: &[Vec2], closed: bool) -> Vec<Vec2> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let mut out = Vec::new();
    let n = points.len();
    let count = if closed { n } else { n - 1 };
    if !closed {
        out.push(points[0]);
    }
    for i in 0..count {
        let a = points[i];
        let b = points[(i + 1) % n];
        out.push(a.lerp(b, 0.25));
        out.push(a.lerp(b, 0.75));
    }
    if !closed {
        out.push(*points.last().unwrap());
    }
    out
}


/// Apply G1 only at the joint between segment `i` and `i+1` (and wrap for last→first if closed).
pub fn enforce_g1_at_joint(beziers: &mut [BezierSegment], joint: usize, keep_lengths: bool) -> bool {
    if beziers.len() < 2 {
        return false;
    }
    let n = beziers.len();
    if joint + 1 < n {
        let (left, right) = beziers.split_at_mut(joint + 1);
        enforce_g1(&mut left[joint], &mut right[0], keep_lengths);
        return true;
    }
    // joint is last segment: optional close with first
    if joint == n - 1 {
        let (left, right) = beziers.split_at_mut(n - 1);
        let mut a = right[0].clone();
        let mut b = left[0].clone();
        enforce_g1(&mut a, &mut b, keep_lengths);
        right[0] = a;
        left[0] = b;
        return true;
    }
    false
}

/// Given a handle drag on segment `seg`, fix the joint(s) that involve that segment.
pub fn enforce_g1_around_segment(beziers: &mut [BezierSegment], seg: usize, keep_lengths: bool) {
    if beziers.len() < 2 {
        return;
    }
    if seg > 0 {
        enforce_g1_at_joint(beziers, seg - 1, keep_lengths);
    }
    if seg + 1 < beziers.len() {
        enforce_g1_at_joint(beziers, seg, keep_lengths);
    } else if seg + 1 == beziers.len() {
        // closing joint
        enforce_g1_at_joint(beziers, seg, keep_lengths);
    }
}
