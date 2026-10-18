//! Simplified Martinez-style polygon boolean via edge splits + winding (Phase 18).
//! Not a full industrial implementation; better than pure convex clip for many cases.

use bevy::prelude::*;
use crate::VectorPath;
use crate::boolean2d::point_in_poly;

#[derive(Clone, Copy, Debug)]
struct Seg {
    a: Vec2,
    b: Vec2,
}

fn segs(poly: &[Vec2]) -> Vec<Seg> {
    if poly.len() < 2 {
        return Vec::new();
    }
    let mut out = Vec::new();
    for i in 0..poly.len() {
        out.push(Seg {
            a: poly[i],
            b: poly[(i + 1) % poly.len()],
        });
    }
    out
}

fn orient(a: Vec2, b: Vec2, c: Vec2) -> f32 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

fn on_segment(a: Vec2, b: Vec2, p: Vec2) -> bool {
    p.x >= a.x.min(b.x) - 1e-6
        && p.x <= a.x.max(b.x) + 1e-6
        && p.y >= a.y.min(b.y) - 1e-6
        && p.y <= a.y.max(b.y) + 1e-6
}

fn segment_intersect(s1: Seg, s2: Seg) -> Option<Vec2> {
    let o1 = orient(s1.a, s1.b, s2.a);
    let o2 = orient(s1.a, s1.b, s2.b);
    let o3 = orient(s2.a, s2.b, s1.a);
    let o4 = orient(s2.a, s2.b, s1.b);
    if o1 * o2 < 0.0 && o3 * o4 < 0.0 {
        let d = (s1.a.x - s1.b.x) * (s2.a.y - s2.b.y) - (s1.a.y - s1.b.y) * (s2.a.x - s2.b.x);
        if d.abs() < 1e-12 {
            return None;
        }
        let t = ((s1.a.x - s2.a.x) * (s2.a.y - s2.b.y) - (s1.a.y - s2.a.y) * (s2.a.x - s2.b.x)) / d;
        return Some(Vec2::new(
            s1.a.x + t * (s1.b.x - s1.a.x),
            s1.a.y + t * (s1.b.y - s1.a.y),
        ));
    }
    None
}

fn split_seg_by_points(s: Seg, mut pts: Vec<Vec2>) -> Vec<Seg> {
    pts.push(s.a);
    pts.push(s.b);
    pts.sort_by(|p, q| {
        let dp = p.distance(s.a);
        let dq = q.distance(s.a);
        dp.partial_cmp(&dq).unwrap_or(std::cmp::Ordering::Equal)
    });
    pts.dedup_by(|a, b| a.distance(*b) < 1e-5);
    let mut out = Vec::new();
    for w in pts.windows(2) {
        if w[0].distance(w[1]) > 1e-5 {
            out.push(Seg { a: w[0], b: w[1] });
        }
    }
    out
}

fn all_split_edges(pa: &[Vec2], pb: &[Vec2]) -> Vec<Seg> {
    let sa = segs(pa);
    let sb = segs(pb);
    let mut result = Vec::new();
    for s in sa.iter().chain(sb.iter()) {
        let mut cuts = Vec::new();
        for o in sa.iter().chain(sb.iter()) {
            if let Some(ip) = segment_intersect(*s, *o) {
                if on_segment(s.a, s.b, ip) {
                    cuts.push(ip);
                }
            }
        }
        result.extend(split_seg_by_points(*s, cuts));
    }
    result
}

fn mid(s: Seg) -> Vec2 {
    (s.a + s.b) * 0.5
}

#[derive(Clone, Copy)]
pub enum BoolOp {
    Union,
    Intersection,
    Difference,
}

/// Martinez-inspired boolean using split edges and midpoint classification.
pub fn martinez_boolean(a: &VectorPath, b: &VectorPath, op: BoolOp) -> VectorPath {
    let pa = a.flattened(12);
    let pb = b.flattened(12);
    if pa.len() < 3 || pb.len() < 3 {
        return VectorPath {
            name: "empty".into(),
            points: Vec::new(),
            closed: true,
            beziers: Vec::new(),
            filled: true,
            holes: Vec::new(),
        };
    }
    let edges = all_split_edges(&pa, &pb);
    let mut keep_segs = Vec::new();
    for e in edges {
        let mpt = mid(e);
        let ia = point_in_poly(mpt, &pa);
        let ib = point_in_poly(mpt, &pb);
        let keep = match op {
            BoolOp::Union => ia || ib,
            BoolOp::Intersection => ia && ib,
            BoolOp::Difference => ia && !ib,
        };
        if keep {
            keep_segs.push(e);
        }
    }
    let rings = chain_all_rings(&keep_segs);
    // Phase 20: use largest ring as primary outline (holes deferred)
    let keep_pts = rings
        .into_iter()
        .max_by_key(|r| r.len())
        .unwrap_or_default();
    let tag = match op {
        BoolOp::Union => "MU",
        BoolOp::Intersection => "MI",
        BoolOp::Difference => "MD",
    };
    VectorPath {
        name: format!("{}_{}_{}", a.name, tag, b.name),
        points: keep_pts,
        closed: true,
        beziers: Vec::new(),
        filled: true,
        holes: Vec::new(),
    }
}

pub fn martinez_union(a: &VectorPath, b: &VectorPath) -> VectorPath {
    martinez_boolean(a, b, BoolOp::Union)
}
pub fn martinez_intersect(a: &VectorPath, b: &VectorPath) -> VectorPath {
    martinez_boolean(a, b, BoolOp::Intersection)
}
pub fn martinez_difference(a: &VectorPath, b: &VectorPath) -> VectorPath {
    martinez_boolean(a, b, BoolOp::Difference)
}

fn same_pt(a: Vec2, b: Vec2) -> bool {
    a.distance(b) < 1e-4
}

fn chain_all_rings(segs: &[Seg]) -> Vec<Vec<Vec2>> {
    let mut unused = segs.to_vec();
    let mut rings = Vec::new();
    let mut guard = 0;
    while !unused.is_empty() && guard < segs.len() + 16 {
        guard += 1;
        let ring = chain_one(&mut unused);
        if ring.len() >= 3 {
            rings.push(ring);
        } else if !unused.is_empty() {
            unused.remove(0);
        } else {
            break;
        }
    }
    rings
}

fn chain_one(unused: &mut Vec<Seg>) -> Vec<Vec2> {
    if unused.is_empty() {
        return Vec::new();
    }
    let first = unused.remove(0);
    let mut ring = vec![first.a, first.b];
    let mut guard = 0;
    while !unused.is_empty() && guard < 4096 {
        guard += 1;
        let tip = *ring.last().unwrap();
        if let Some(i) = unused.iter().position(|s| same_pt(s.a, tip) || same_pt(s.b, tip)) {
            let s = unused.remove(i);
            let next = if same_pt(s.a, tip) { s.b } else { s.a };
            if same_pt(next, ring[0]) {
                break;
            }
            ring.push(next);
        } else {
            break;
        }
    }
    ring
}
