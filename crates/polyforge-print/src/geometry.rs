//! 2D polygon helpers used by the slicer (offsetting, area, infill).
//!
//! These are deliberately dependency-free and operate on `Vec<Vec2>` in
//! millimetres. They correspond to `GeometryUtils.h` in ksengine.

use crate::types::{InfillPattern, Polygon2D, Polygons2D, Vec2, Vec3};

const EPS: f64 = 1e-10;

/// Signed area (shoelace). Positive for CCW winding.
pub fn polygon_area(verts: &[Vec2]) -> f64 {
    if verts.len() < 3 {
        return 0.0;
    }
    let mut area = 0.0;
    let n = verts.len();
    for i in 0..n {
        let j = (i + n - 1) % n;
        area += (verts[j].x + verts[i].x) * (verts[j].y - verts[i].y);
    }
    area * 0.5
}

/// Ray-casting point-in-polygon test.
pub fn point_in_polygon(pt: Vec2, poly: &[Vec2]) -> bool {
    if poly.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let pi = poly[i];
        let pj = poly[j];
        if (pi.y > pt.y) != (pj.y > pt.y) {
            let x_at = (pj.x - pi.x) * (pt.y - pi.y) / (pj.y - pi.y) + pi.x;
            if pt.x < x_at {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

/// Return a copy of `poly` with the winding flipped if needed so its signed
/// area is positive (CCW).
pub fn ensure_ccw(mut verts: Vec<Vec2>) -> Vec<Vec2> {
    if polygon_area(&verts) < 0.0 {
        verts.reverse();
    }
    verts
}

/// Offset every polygon of `input` inward (negative offset) or outward
/// (positive offset) using a miter join with a clamp to avoid spikes.
pub fn offset_polygons(input: &[Polygon2D], offset: f64) -> Polygons2D {
    input
        .iter()
        .filter_map(|poly| offset_polygon(&poly.vertices, offset))
        .collect()
}

fn offset_polygon(verts: &[Vec2], offset: f64) -> Option<Polygon2D> {
    if verts.len() < 3 {
        return None;
    }
    let n = verts.len();
    let mut out: Vec<Vec2> = Vec::with_capacity(n);

    for i in 0..n {
        let prev = verts[(i + n - 1) % n];
        let curr = verts[i];
        let next = verts[(i + 1) % n];

        let v1 = curr - prev;
        let v2 = next - curr;
        let len1 = v1.length();
        let len2 = v2.length();
        if len1 < EPS || len2 < EPS {
            continue;
        }
        let v1 = v1 / len1;
        let v2 = v2 / len2;

        // Inward normals for CCW polygons (left-hand normal).
        let n1 = Vec2::new(-v1.y, v1.x);
        let n2 = Vec2::new(-v2.y, v2.x);

        let bisector = n1 + n2;
        let blen = bisector.length();
        if blen < EPS {
            out.push(curr + n1 * offset);
            continue;
        }

        let dot = n1.dot(n2);
        let miter_limit = 2.0;
        let mut miter_factor = 1.0 / (0.5 * (1.0 + dot)).max(0.01);
        miter_factor = miter_factor.min(miter_limit);

        out.push(curr + (bisector / blen) * offset * miter_factor);
    }

    if out.len() >= 3 {
        Some(Polygon2D::new(out))
    } else {
        None
    }
}

/// Generate parallel-line infill clipped to the given boundary polygons.
///
/// Each resulting polygon is a thin quad (a single extrusion segment); the
/// G-code generator consumes their centre line.
pub fn generate_grid_infill(
    boundaries: &[Polygon2D],
    spacing: f64,
    angle_rad: f64,
    offset_x: f64,
    offset_y: f64,
) -> Polygons2D {
    if boundaries.is_empty() || spacing <= 0.0 {
        return Vec::new();
    }

    let bounds = match polygons_bounds(boundaries) {
        Some(b) => b,
        None => return Vec::new(),
    };

    let (cos_a, sin_a) = (angle_rad.cos(), angle_rad.sin());
    // Rotate into the line-aligned frame.
    let to_rot = |p: Vec2| Vec2::new(p.x * cos_a - p.y * sin_a, p.x * sin_a + p.y * cos_a);
    let to_world = |p: Vec2| Vec2::new(p.x * cos_a + p.y * sin_a, -p.x * sin_a + p.y * cos_a);

    let mut infill = Vec::new();

    let start_x = ((bounds.min.x - offset_x) / spacing).floor() * spacing + offset_x;
    let end_x = ((bounds.max.x - offset_x) / spacing).ceil() * spacing + offset_x;

    let mut x = start_x;
    while x <= end_x {
        let mut intersections: Vec<f64> = Vec::new();

        for poly in boundaries {
            let vs = &poly.vertices;
            if vs.len() < 3 {
                continue;
            }
            let n = vs.len();
            for i in 0..n {
                let p1 = to_rot(vs[i]);
                let p2 = to_rot(vs[(i + 1) % n]);
                if (p1.x <= x) != (p2.x <= x) && (p2.x - p1.x).abs() > EPS {
                    let t = (x - p1.x) / (p2.x - p1.x);
                    intersections.push(p1.y + t * (p2.y - p1.y));
                }
            }
        }

        intersections.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let mut k = 0;
        while k + 1 < intersections.len() {
            let y1 = intersections[k];
            let y2 = intersections[k + 1];
            k += 2;

            if y2 - y1 < spacing * 0.1 {
                continue;
            }

            let p1 = to_world(Vec2::new(x, y1 + offset_y));
            let p2 = to_world(Vec2::new(x, y2 + offset_y));
            let dir = (p2 - p1).normalize_or_zero();
            if dir == Vec2::ZERO {
                continue;
            }
            let half_width = spacing * 0.5;
            let perp = Vec2::new(-dir.y, dir.x) * half_width;

            infill.push(Polygon2D::new(vec![
                p1 + perp,
                p2 + perp,
                p2 - perp,
                p1 - perp,
            ]));
        }

        x += spacing;
    }

    infill
}

/// Convenience dispatcher for the configured infill pattern.
pub fn generate_infill(
    pattern: InfillPattern,
    boundaries: &[Polygon2D],
    spacing: f64,
    angle_rad: f64,
) -> Polygons2D {
    match pattern {
        InfillPattern::Concentric => offset_polygons(boundaries, -spacing * 0.5),
        // Gyroid is approximated with a denser grid for the MVP.
        InfillPattern::Grid
        | InfillPattern::Lines
        | InfillPattern::Triangles
        | InfillPattern::Gyroid => generate_grid_infill(boundaries, spacing, angle_rad, 0.0, 0.0),
    }
}

fn polygons_bounds(polys: &[Polygon2D]) -> Option<crate::types::BoundingBox> {
    let mut bb = crate::types::BoundingBox::empty();
    let mut any = false;
    for poly in polys {
        for v in &poly.vertices {
            bb.expand_point(Vec3::new(v.x, v.y, 0.0));
            any = true;
        }
    }
    any.then_some(bb)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(size: f64) -> Vec<Vec2> {
        vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(size, 0.0),
            Vec2::new(size, size),
            Vec2::new(0.0, size),
        ]
    }

    #[test]
    fn area_of_square() {
        let s = ensure_ccw(square(10.0));
        assert!((polygon_area(&s) - 100.0).abs() < 1e-6);
    }

    #[test]
    fn point_inside_square() {
        let s = square(10.0);
        assert!(point_in_polygon(Vec2::new(5.0, 5.0), &s));
        assert!(!point_in_polygon(Vec2::new(15.0, 5.0), &s));
    }

    #[test]
    fn inward_offset_shrinks() {
        let ccw = ensure_ccw(square(10.0));
        let poly = Polygon2D::new(ccw);
        let out = offset_polygons(&[poly], -1.0);
        assert_eq!(out.len(), 1);
        let area = polygon_area(&out[0].vertices).abs();
        assert!(area < 100.0 && area > 0.0);
    }

    #[test]
    fn grid_infill_has_segments() {
        let poly = Polygon2D::new(ensure_ccw(square(20.0)));
        let segs = generate_grid_infill(&[poly], 4.0, 0.0, 0.0, 0.0);
        assert!(!segs.is_empty());
    }
}
