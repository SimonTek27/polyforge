//! Raycasting against editable mesh elements (vertices, edges, faces).

use bevy::prelude::*;
use super::mesh::{EditableMesh, Face};
use super::topology::extract_edges;

/// Result of a mesh pick.
#[derive(Debug, Clone, Copy)]
pub enum PickHit {
    Vertex { index: u32, distance: f32 },
    Edge { a: u32, b: u32, distance: f32 },
    Face { index: u32, distance: f32 },
}

impl PickHit {
    pub fn distance(&self) -> f32 {
        match self {
            PickHit::Vertex { distance, .. }
            | PickHit::Edge { distance, .. }
            | PickHit::Face { distance, .. } => *distance,
        }
    }
}

/// Maximum screen-space (world) distance to consider a vertex/edge "hit".
/// Scaled by camera distance outside this module.
pub const DEFAULT_VERTEX_THRESHOLD: f32 = 0.12;
pub const DEFAULT_EDGE_THRESHOLD: f32 = 0.10;

/// Find the closest vertex to the ray within `threshold` world units.
pub fn pick_vertex(
    ray: &Ray3d,
    mesh: &EditableMesh,
    world_from_local: &Mat4,
    threshold: f32,
) -> Option<PickHit> {
    let mut best: Option<(u32, f32)> = None;

    for (i, &local_pos) in mesh.vertices.iter().enumerate() {
        let world_pos = world_from_local.transform_point3(local_pos);
        let dist = distance_ray_to_point(ray, world_pos);
        if dist < threshold {
            if best.map_or(true, |(_, d)| dist < d) {
                best = Some((i as u32, dist));
            }
        }
    }

    best.map(|(index, distance)| PickHit::Vertex { index, distance })
}

/// Find the closest edge to the ray within `threshold` world units.
pub fn pick_edge(
    ray: &Ray3d,
    mesh: &EditableMesh,
    world_from_local: &Mat4,
    threshold: f32,
) -> Option<PickHit> {
    let edges = extract_edges(mesh);
    let mut best: Option<(u32, u32, f32)> = None;

    for &(a, b) in &edges {
        let wa = world_from_local.transform_point3(mesh.vertices[a as usize]);
        let wb = world_from_local.transform_point3(mesh.vertices[b as usize]);
        let dist = distance_ray_to_segment(ray, wa, wb);
        if dist < threshold {
            if best.map_or(true, |(_, _, d)| dist < d) {
                best = Some((a, b, dist));
            }
        }
    }

    best.map(|(a, b, distance)| PickHit::Edge { a, b, distance })
}

/// Find the closest face intersected by the ray (triangle/quad tests).
pub fn pick_face(
    ray: &Ray3d,
    mesh: &EditableMesh,
    world_from_local: &Mat4,
) -> Option<PickHit> {
    let mut best: Option<(u32, f32)> = None;

    for (face_idx, face) in mesh.faces.iter().enumerate() {
        if let Some(t) = ray_intersects_face(ray, mesh, face, world_from_local) {
            if t > 0.0 && best.map_or(true, |(_, d)| t < d) {
                best = Some((face_idx as u32, t));
            }
        }
    }

    best.map(|(index, distance)| PickHit::Face { index, distance })
}

/// Ray–face intersection. Triangulates n-gons with a simple fan.
fn ray_intersects_face(
    ray: &Ray3d,
    mesh: &EditableMesh,
    face: &Face,
    world_from_local: &Mat4,
) -> Option<f32> {
    if face.indices.len() < 3 {
        return None;
    }

    let mut closest: Option<f32> = None;

    // Fan triangulation from first vertex
    for i in 1..face.indices.len() - 1 {
        let i0 = face.indices[0] as usize;
        let i1 = face.indices[i] as usize;
        let i2 = face.indices[i + 1] as usize;

        let v0 = world_from_local.transform_point3(mesh.vertices[i0]);
        let v1 = world_from_local.transform_point3(mesh.vertices[i1]);
        let v2 = world_from_local.transform_point3(mesh.vertices[i2]);

        if let Some(t) = ray_triangle_intersect(ray, v0, v1, v2) {
            if closest.map_or(true, |c| t < c) {
                closest = Some(t);
            }
        }
    }

    closest
}

/// Möller–Trumbore ray-triangle intersection.
/// Returns distance along ray if hit.
fn ray_triangle_intersect(ray: &Ray3d, v0: Vec3, v1: Vec3, v2: Vec3) -> Option<f32> {
    const EPSILON: f32 = 1e-6;

    let edge1 = v1 - v0;
    let edge2 = v2 - v0;
    let dir = *ray.direction;
    let h = dir.cross(edge2);
    let a = edge1.dot(h);

    if a > -EPSILON && a < EPSILON {
        return None; // parallel
    }

    let f = 1.0 / a;
    let s = ray.origin - v0;
    let u = f * s.dot(h);
    if u < 0.0 || u > 1.0 {
        return None;
    }

    let q = s.cross(edge1);
    let v = f * dir.dot(q);
    if v < 0.0 || u + v > 1.0 {
        return None;
    }

    let t = f * edge2.dot(q);
    if t > EPSILON {
        Some(t)
    } else {
        None
    }
}

/// Shortest distance from a ray to a point in world space.
pub fn distance_ray_to_point(ray: &Ray3d, point: Vec3) -> f32 {
    let to_point = point - ray.origin;
    let projected = to_point.dot(*ray.direction).max(0.0);
    let closest = ray.origin + *ray.direction * projected;
    closest.distance(point)
}

/// Shortest distance from a ray to a line segment.
pub fn distance_ray_to_segment(ray: &Ray3d, a: Vec3, b: Vec3) -> f32 {
    let ab = b - a;
    let len_sq = ab.length_squared();
    if len_sq < 1e-12 {
        return distance_ray_to_point(ray, a);
    }

    let dir = *ray.direction;
    // Solve for closest points between two skew lines
    let ao = a - ray.origin;
    let cross = dir.cross(ab);
    let denom = cross.length_squared();

    if denom < 1e-10 {
        // Nearly parallel – distance from origin to the infinite line, then clamp
        let t = ao.dot(dir).max(0.0);
        let point_on_ray = ray.origin + dir * t;
        let u = (point_on_ray - a).dot(ab) / len_sq;
        let u_clamped = u.clamp(0.0, 1.0);
        let point_on_seg = a + ab * u_clamped;
        return point_on_ray.distance(point_on_seg);
    }

    let t = ao.cross(ab).dot(cross) / denom; // along ray
    let u = ao.cross(dir).dot(cross) / denom; // along segment

    let t_clamped = t.max(0.0);
    let u_clamped = u.clamp(0.0, 1.0);

    let point_on_ray = ray.origin + dir * t_clamped;
    let point_on_seg = a + ab * u_clamped;
    point_on_ray.distance(point_on_seg)
}
