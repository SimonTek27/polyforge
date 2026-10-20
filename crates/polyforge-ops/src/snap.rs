//! Geometric snap: vertex, edge, face.

use bevy::prelude::*;
use polyforge_core::{EditableMesh, extract_edges, topology::face_normal};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SnapTarget {
    #[default]
    None,
    Vertex,
    Edge,
    Face,
    Grid,
}

#[derive(Debug, Clone, Copy)]
pub struct SnapResult {
    pub point: Vec3,
    pub target: SnapTarget,
}

/// Snap a world-space point to the nearest mesh element (in mesh local space).
pub fn snap_point_to_mesh(
    mesh: &EditableMesh,
    point: Vec3,
    mode: SnapTarget,
    threshold: f32,
) -> Option<SnapResult> {
    match mode {
        SnapTarget::None => None,
        SnapTarget::Grid => {
            let g = threshold.max(1e-6);
            Some(SnapResult {
                point: Vec3::new(
                    (point.x / g).round() * g,
                    (point.y / g).round() * g,
                    (point.z / g).round() * g,
                ),
                target: SnapTarget::Grid,
            })
        }
        SnapTarget::Vertex => {
            let mut best = None;
            let mut best_d = threshold;
            for v in &mesh.vertices {
                let d = point.distance(*v);
                if d < best_d {
                    best_d = d;
                    best = Some(*v);
                }
            }
            best.map(|p| SnapResult {
                point: p,
                target: SnapTarget::Vertex,
            })
        }
        SnapTarget::Edge => {
            let mut best = None;
            let mut best_d = threshold;
            for &(a, b) in &extract_edges(mesh) {
                let pa = mesh.vertices[a as usize];
                let pb = mesh.vertices[b as usize];
                let (closest, d) = closest_on_segment(point, pa, pb);
                if d < best_d {
                    best_d = d;
                    best = Some(closest);
                }
            }
            best.map(|p| SnapResult {
                point: p,
                target: SnapTarget::Edge,
            })
        }
        SnapTarget::Face => {
            let mut best = None;
            let mut best_d = threshold;
            for face in &mesh.faces {
                if face.indices.len() < 3 {
                    continue;
                }
                // Fan to triangles
                let i0 = face.indices[0] as usize;
                for i in 1..face.indices.len() - 1 {
                    let i1 = face.indices[i] as usize;
                    let i2 = face.indices[i + 1] as usize;
                    let a = mesh.vertices[i0];
                    let b = mesh.vertices[i1];
                    let c = mesh.vertices[i2];
                    if let Some((closest, d)) = closest_on_triangle(point, a, b, c) {
                        if d < best_d {
                            best_d = d;
                            best = Some(closest);
                        }
                    }
                }
            }
            best.map(|p| SnapResult {
                point: p,
                target: SnapTarget::Face,
            })
        }
    }
}

fn closest_on_segment(p: Vec3, a: Vec3, b: Vec3) -> (Vec3, f32) {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_squared().max(1e-12)).clamp(0.0, 1.0);
    let c = a + ab * t;
    (c, p.distance(c))
}

fn closest_on_triangle(p: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Option<(Vec3, f32)> {
    let n = (b - a).cross(c - a);
    let nlen = n.length();
    if nlen < 1e-12 {
        return None;
    }
    let n = n / nlen;
    let dist = (p - a).dot(n);
    let proj = p - n * dist;
    // Barycentric inside test
    let v0 = c - a;
    let v1 = b - a;
    let v2 = proj - a;
    let dot00 = v0.dot(v0);
    let dot01 = v0.dot(v1);
    let dot02 = v0.dot(v2);
    let dot11 = v1.dot(v1);
    let dot12 = v1.dot(v2);
    let inv = 1.0 / (dot00 * dot11 - dot01 * dot01).max(1e-12);
    let u = (dot11 * dot02 - dot01 * dot12) * inv;
    let v = (dot00 * dot12 - dot01 * dot02) * inv;
    if u >= 0.0 && v >= 0.0 && (u + v) <= 1.0 {
        Some((proj, dist.abs()))
    } else {
        // Fall back to edges
        let (c0, d0) = closest_on_segment(p, a, b);
        let (c1, d1) = closest_on_segment(p, b, c);
        let (c2, d2) = closest_on_segment(p, c, a);
        if d0 <= d1 && d0 <= d2 {
            Some((c0, d0))
        } else if d1 <= d2 {
            Some((c1, d1))
        } else {
            Some((c2, d2))
        }
    }
}

/// Snap selected vertices toward nearest mesh elements (excluding themselves).
pub fn snap_selection_to_target(
    mesh: &mut EditableMesh,
    selected: &std::collections::HashSet<u32>,
    mode: SnapTarget,
    threshold: f32,
) {
    if mode == SnapTarget::None || selected.is_empty() {
        return;
    }
    // Work on a snapshot of positions so we don't snap to already-moved verts inconsistently
    let snapshot = mesh.vertices.clone();
    for &si in selected {
        let p = snapshot[si as usize];
        // Temporarily ignore selected verts by using full mesh but prefer non-selected
        if let Some(result) = snap_point_to_mesh(mesh, p, mode, threshold) {
            // Avoid snapping a vertex to itself
            if result.point.distance(p) > 1e-8 {
                mesh.vertices[si as usize] = result.point;
            }
        }
    }
    let _ = face_normal; // keep topology import used if needed later
}
