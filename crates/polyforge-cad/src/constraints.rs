//! Lightweight sketch constraints (Phase 5 — no full geometric solver).

use bevy::prelude::*;
use crate::feature::SketchCurve;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SketchConstraintKind {
    Horizontal,
    Vertical,
    EqualLength,
}

#[derive(Debug, Clone)]
pub struct SketchConstraint {
    pub kind: SketchConstraintKind,
    pub curve_index: usize,
}

/// Analyse curves and return auto-detected H/V constraints for display.
pub fn detect_hv_constraints(curves: &[SketchCurve], tol: f32) -> Vec<SketchConstraint> {
    let mut out = Vec::new();
    for (i, c) in curves.iter().enumerate() {
        if let SketchCurve::Line { a, b } = c {
            let d = *b - *a;
            if d.x.abs() < tol {
                out.push(SketchConstraint {
                    kind: SketchConstraintKind::Vertical,
                    curve_index: i,
                });
            } else if d.y.abs() < tol {
                out.push(SketchConstraint {
                    kind: SketchConstraintKind::Horizontal,
                    curve_index: i,
                });
            }
        }
    }
    out
}

/// Snap a segment endpoint to enforce H or V relative to start.
pub fn enforce_hv(start: Vec2, mut end: Vec2, tol: f32) -> Vec2 {
    let d = end - start;
    if d.x.abs() < tol {
        end.x = start.x;
    } else if d.y.abs() < tol {
        end.y = start.y;
    }
    end
}


/// Rough DOF estimate: 2 * line_count - constraint slots (Phase 29).
pub fn estimate_sketch_dof(curve_count: usize, constraint_count: usize) -> i32 {
    let free = (curve_count as i32) * 2;
    (free - constraint_count as i32).max(0)
}
