//! 2D boolean operations on closed polygon paths (Phase 9).
//! Uses even-odd / winding tests and Sutherland–Hodgman clipping for intersection.

use bevy::prelude::*;
use crate::VectorPath;

/// Point-in-polygon (ray cast).
pub fn point_in_poly(pt: Vec2, poly: &[Vec2]) -> bool {
    if poly.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let pi = poly[i];
        let pj = poly[j];
        if ((pi.y > pt.y) != (pj.y > pt.y))
            && (pt.x < (pj.x - pi.x) * (pt.y - pi.y) / (pj.y - pi.y + 1e-12) + pi.x)
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn cross(o: Vec2, a: Vec2, b: Vec2) -> f32 {
    (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x)
}

/// Convex clip subject against one half-plane (Sutherland–Hodgman edge).


fn line_intersect(p1: Vec2, p2: Vec2, p3: Vec2, p4: Vec2) -> Option<Vec2> {
    let d = (p1.x - p2.x) * (p3.y - p4.y) - (p1.y - p2.y) * (p3.x - p4.x);
    if d.abs() < 1e-12 {
        return None;
    }
    let t = ((p1.x - p3.x) * (p3.y - p4.y) - (p1.y - p3.y) * (p3.x - p4.x)) / d;
    Some(Vec2::new(
        p1.x + t * (p2.x - p1.x),
        p1.y + t * (p2.y - p1.y),
    ))
}

/// Clip polygon against one half-plane defined by edge a->b (keep left side).
fn clip_edge_fixed(poly: &[Vec2], a: Vec2, b: Vec2) -> Vec<Vec2> {
    if poly.len() < 3 {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let pi = poly[i];
        let pj = poly[j];
        // Keep points on the left side of a->b (cross(b-a, p-a) >= 0)
        let inside_i = cross(a, b, pi) >= -1e-12;
        let inside_j = cross(a, b, pj) >= -1e-12;

        if inside_i != inside_j {
            if let Some(ip) = line_intersect(pi, pj, a, b) {
                out.push(ip);
            }
        }
        if inside_i {
            out.push(pi);
        }
        j = i;
    }
    out
}

/// Clip subject polygon to clip polygon (requires **convex** clip).
pub fn intersect_convex(subject: &[Vec2], clip: &[Vec2]) -> Vec<Vec2> {
    if subject.len() < 3 || clip.len() < 3 {
        return Vec::new();
    }
    let mut out = subject.to_vec();
    for i in 0..clip.len() {
        let a = clip[i];
        let b = clip[(i + 1) % clip.len()];
        out = clip_edge_fixed(&out, a, b);
        if out.is_empty() {
            break;
        }
    }
    out
}

/// Union of two closed paths.
/// Uses boundary points of both plus a coarse grid of interior samples, then convex hull
/// when shapes are simple; for concave results keeps outer ring approximation via hull.
pub fn union_paths(a: &VectorPath, b: &VectorPath) -> VectorPath {
    let pa = a.flattened(8);
    let pb = b.flattened(8);
    let mut pts = pa.clone();
    pts.extend(pb.iter().copied());
    // Grid samples inside either polygon (non-convex aware sampling)
    let (min, max) = bounds_of([&pa[..], &pb[..]]);
    let steps = 12u32;
    for iy in 0..=steps {
        for ix in 0..=steps {
            let t = Vec2::new(ix as f32 / steps as f32, iy as f32 / steps as f32);
            let p = min + (max - min) * t;
            if point_in_poly(p, &pa) || point_in_poly(p, &pb) {
                pts.push(p);
            }
        }
    }
    // Prefer contour of occupied grid cells (can be non-convex); fallback hull
    let contour = union_contour(&pa, &pb, 24);
    let points = if contour.len() >= 3 {
        contour
    } else {
        convex_hull(&pts)
    };
    VectorPath {
        name: format!("{}_U_{}", a.name, b.name),
        points,
        closed: true,
        beziers: Vec::new(),
        filled: true,
        holes: Vec::new(),
    }
}

/// March around occupied grid cells to build a simple outer ring (Phase 11).
fn union_contour(pa: &[Vec2], pb: &[Vec2], steps: u32) -> Vec<Vec2> {
    let (min, max) = bounds_of([pa, pb]);
    let steps = steps.max(4);
    let mut grid = vec![false; ((steps + 1) * (steps + 1)) as usize];
    let idx = |ix: u32, iy: u32| -> usize { (iy * (steps + 1) + ix) as usize };
    for iy in 0..=steps {
        for ix in 0..=steps {
            let t = Vec2::new(ix as f32 / steps as f32, iy as f32 / steps as f32);
            let p = min + (max - min) * t;
            if point_in_poly(p, pa) || point_in_poly(p, pb) {
                grid[idx(ix, iy)] = true;
            }
        }
    }
    // Collect boundary cell centers (cell occupied, neighbor empty or border)
    let mut boundary = Vec::new();
    for iy in 0..=steps {
        for ix in 0..=steps {
            if !grid[idx(ix, iy)] {
                continue;
            }
            let mut edge = ix == 0 || iy == 0 || ix == steps || iy == steps;
            if !edge {
                for (dx, dy) in [(-1i32, 0), (1, 0), (0, -1), (0, 1)] {
                    let nx = ix as i32 + dx;
                    let ny = iy as i32 + dy;
                    if nx < 0 || ny < 0 || nx > steps as i32 || ny > steps as i32 {
                        edge = true;
                        break;
                    }
                    if !grid[idx(nx as u32, ny as u32)] {
                        edge = true;
                        break;
                    }
                }
            }
            if edge {
                let t = Vec2::new(ix as f32 / steps as f32, iy as f32 / steps as f32);
                boundary.push(min + (max - min) * t);
            }
        }
    }
    if boundary.len() < 3 {
        return boundary;
    }
    // Order by angle around centroid for a closed ring
    let c = boundary.iter().fold(Vec2::ZERO, |a, p| a + *p) / boundary.len() as f32;
    boundary.sort_by(|a, b| {
        let aa = (a.y - c.y).atan2(a.x - c.x);
        let bb = (b.y - c.y).atan2(b.x - c.x);
        aa.partial_cmp(&bb).unwrap_or(std::cmp::Ordering::Equal)
    });
    boundary
}

fn bounds_of(polys: [&[Vec2]; 2]) -> (Vec2, Vec2) {
    let mut min = Vec2::splat(f32::MAX);
    let mut max = Vec2::splat(f32::MIN);
    for poly in polys {
        for p in poly {
            min = min.min(*p);
            max = max.max(*p);
        }
    }
    if !min.is_finite() {
        (Vec2::ZERO, Vec2::ONE)
    } else {
        (min, max)
    }
}

pub fn intersect_paths(a: &VectorPath, b: &VectorPath) -> VectorPath {
    let pa = a.flattened(8);
    let pb = b.flattened(8);
    // Try convex clip first
    let inter = intersect_convex(&pa, &pb);
    if inter.len() >= 3 {
        return VectorPath {
            name: format!("{}_I_{}", a.name, b.name),
            points: inter,
            closed: true,
            beziers: Vec::new(),
            filled: true,
            holes: Vec::new(),
        };
    }
    // Phase 15: grid-based intersection for non-convex
    let pts = grid_classify(&pa, &pb, 28, |ia, ib| ia && ib);
    VectorPath {
        name: format!("{}_I_{}", a.name, b.name),
        points: angle_sort_ring(pts),
        closed: true,
        beziers: Vec::new(),
        filled: true,
        holes: Vec::new(),
    }
}

fn grid_classify(
    pa: &[Vec2],
    pb: &[Vec2],
    steps: u32,
    pred: impl Fn(bool, bool) -> bool,
) -> Vec<Vec2> {
    let (min, max) = bounds_of([pa, pb]);
    let mut pts = Vec::new();
    for iy in 0..=steps {
        for ix in 0..=steps {
            let t = Vec2::new(ix as f32 / steps as f32, iy as f32 / steps as f32);
            let p = min + (max - min) * t;
            if pred(point_in_poly(p, pa), point_in_poly(p, pb)) {
                pts.push(p);
            }
        }
    }
    for p in pa {
        if pred(true, point_in_poly(*p, pb)) {
            pts.push(*p);
        }
    }
    for p in pb {
        if pred(point_in_poly(*p, pa), true) {
            pts.push(*p);
        }
    }
    pts
}

fn angle_sort_ring(mut pts: Vec<Vec2>) -> Vec<Vec2> {
    if pts.len() < 3 {
        return pts;
    }
    let c = pts.iter().fold(Vec2::ZERO, |a, p| a + *p) / pts.len() as f32;
    pts.sort_by(|a, b| {
        let aa = (a.y - c.y).atan2(a.x - c.x);
        let bb = (b.y - c.y).atan2(b.x - c.x);
        aa.partial_cmp(&bb).unwrap_or(std::cmp::Ordering::Equal)
    });
    pts
}

/// Difference A − B: keep vertices of A outside B + intersection edges (approx).
pub fn difference_paths(a: &VectorPath, b: &VectorPath) -> VectorPath {
    let pa = a.flattened(8);
    let pb = b.flattened(8);
    let pts: Vec<Vec2> = pa
        .into_iter()
        .filter(|p| !point_in_poly(*p, &pb))
        .collect();
    VectorPath {
        name: format!("{}_D_{}", a.name, b.name),
        points: pts,
        closed: a.closed,
        beziers: Vec::new(),
        filled: a.filled,
        holes: Vec::new(),
    }
}

fn convex_hull(points: &[Vec2]) -> Vec<Vec2> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let mut pts = points.to_vec();
    pts.sort_by(|a, b| {
        a.x
            .partial_cmp(&b.x)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal))
    });
    pts.dedup();
    if pts.len() < 3 {
        return pts;
    }
    let mut lower = Vec::new();
    for p in &pts {
        while lower.len() >= 2
            && cross(lower[lower.len() - 2], lower[lower.len() - 1], *p) <= 0.0
        {
            lower.pop();
        }
        lower.push(*p);
    }
    let mut upper = Vec::new();
    for p in pts.iter().rev() {
        while upper.len() >= 2
            && cross(upper[upper.len() - 2], upper[upper.len() - 1], *p) <= 0.0
        {
            upper.pop();
        }
        upper.push(*p);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}


/// Symmetric difference (XOR) via grid: cells in exactly one polygon.
pub fn xor_paths(a: &VectorPath, b: &VectorPath) -> VectorPath {
    let pa = a.flattened(8);
    let pb = b.flattened(8);
    let (min, max) = bounds_of([&pa[..], &pb[..]]);
    let steps = 28u32;
    let mut pts = Vec::new();
    for iy in 0..=steps {
        for ix in 0..=steps {
            let t = Vec2::new(ix as f32 / steps as f32, iy as f32 / steps as f32);
            let p = min + (max - min) * t;
            let ia = point_in_poly(p, &pa);
            let ib = point_in_poly(p, &pb);
            if ia != ib {
                pts.push(p);
            }
        }
    }
    // also keep boundary points of each that lie outside the other
    for p in &pa {
        if !point_in_poly(*p, &pb) {
            pts.push(*p);
        }
    }
    for p in &pb {
        if !point_in_poly(*p, &pa) {
            pts.push(*p);
        }
    }
    let contour = if pts.len() >= 3 {
        // angle sort around centroid
        let c = pts.iter().fold(Vec2::ZERO, |a, p| a + *p) / pts.len() as f32;
        pts.sort_by(|a, b| {
            let aa = (a.y - c.y).atan2(a.x - c.x);
            let bb = (b.y - c.y).atan2(b.x - c.x);
            aa.partial_cmp(&bb).unwrap_or(std::cmp::Ordering::Equal)
        });
        pts
    } else {
        pts
    };
    VectorPath {
        name: format!("{}_X_{}", a.name, b.name),
        points: contour,
        closed: true,
        beziers: Vec::new(),
        filled: true,
        holes: Vec::new(),
    }
}


/// Even-odd style: inside outer and outside all holes.
pub fn point_in_path_with_holes(pt: Vec2, outer: &[Vec2], holes: &[Vec<Vec2>]) -> bool {
    if !point_in_poly(pt, outer) {
        return false;
    }
    for h in holes {
        if point_in_poly(pt, h) {
            return false;
        }
    }
    true
}
