//! Tessellate CAD features to EditableMesh for Bevy display.

use bevy::prelude::*;
use polyforge_core::{EditableMesh, Face};
use crate::feature::{CadFeature, SketchCurve, SketchPlane};

pub fn tessellate_features(features: &[CadFeature]) -> EditableMesh {
    let mut result = EditableMesh::default();
    // Collect sketches by id for Pad
    let sketches: Vec<_> = features
        .iter()
        .filter_map(|f| match f {
            CadFeature::Sketch { id, plane, curves, .. } => Some((*id, *plane, curves.clone())),
            _ => None,
        })
        .collect();

    for f in features {
        match f {
            CadFeature::Box {
                width,
                height,
                depth,
                ..
            } => {
                let m = box_mesh(*width, *height, *depth);
                append_mesh(&mut result, &m);
            }
            CadFeature::Cylinder {
                radius,
                height,
                segments,
                ..
            } => {
                let m = cylinder_mesh(*radius, *height, *segments);
                append_mesh(&mut result, &m);
            }
            CadFeature::Sphere {
                radius,
                segments,
                ..
            } => {
                let m = sphere_mesh(*radius, *segments);
                append_mesh(&mut result, &m);
            }
            CadFeature::Cone {
                radius,
                height,
                segments,
                ..
            } => {
                let m = cone_mesh(*radius, *height, *segments);
                append_mesh(&mut result, &m);
            }
            CadFeature::Pad {
                sketch_id,
                distance,
                ..
            } => {
                if let Some((_, plane, curves)) =
                    sketches.iter().find(|(id, _, _)| id == sketch_id)
                {
                    let m = pad_from_sketch(plane, curves, *distance);
                    append_mesh(&mut result, &m);
                }
            }
            CadFeature::Pocket { .. } => {
                // Phase 3 MVP: pocket requires boolean kernel — skip mesh until C4
            }
            CadFeature::Sketch { .. } => {
                // sketches alone don't add volume
            }
        }
    }
    if result.faces.is_empty() && result.vertices.is_empty() {
        // empty body: unit marker
        result = box_mesh(0.1, 0.1, 0.1);
    }
    result.recalculate_normals();
    result
}

fn append_mesh(dst: &mut EditableMesh, src: &EditableMesh) {
    let base = dst.vertices.len() as u32;
    dst.vertices.extend_from_slice(&src.vertices);
    dst.normals.extend_from_slice(&src.normals);
    dst.uvs.extend_from_slice(&src.uvs);
    for face in &src.faces {
        dst.faces.push(Face {
            indices: face.indices.iter().map(|i| i + base).collect(),
        });
    }
}

pub fn box_mesh(w: f32, h: f32, d: f32) -> EditableMesh {
    let hx = w * 0.5;
    let hy = h * 0.5;
    let hz = d * 0.5;
    let mut m = EditableMesh::default();
    let verts = [
        [-hx, -hy, -hz],
        [hx, -hy, -hz],
        [hx, hy, -hz],
        [-hx, hy, -hz],
        [-hx, -hy, hz],
        [hx, -hy, hz],
        [hx, hy, hz],
        [-hx, hy, hz],
    ];
    for v in verts {
        m.vertices.push(Vec3::new(v[0], v[1], v[2]));
        m.normals.push(Vec3::Y);
        m.uvs.push(Vec2::ZERO);
    }
    let faces = [
        [0, 1, 2, 3],
        [4, 7, 6, 5],
        [0, 4, 5, 1],
        [2, 6, 7, 3],
        [0, 3, 7, 4],
        [1, 5, 6, 2],
    ];
    for f in faces {
        m.faces.push(Face::quad(f[0], f[1], f[2], f[3]));
    }
    m.recalculate_normals();
    m
}

pub fn cylinder_mesh(radius: f32, height: f32, segments: u32) -> EditableMesh {
    let segments = segments.max(3);
    let mut m = EditableMesh::default();
    let hy = height * 0.5;
    // bottom center, top center
    m.vertices.push(Vec3::new(0.0, -hy, 0.0));
    m.normals.push(-Vec3::Y);
    m.uvs.push(Vec2::new(0.5, 0.5));
    m.vertices.push(Vec3::new(0.0, hy, 0.0));
    m.normals.push(Vec3::Y);
    m.uvs.push(Vec2::new(0.5, 0.5));
    for i in 0..segments {
        let a = (i as f32 / segments as f32) * std::f32::consts::TAU;
        let x = a.cos() * radius;
        let z = a.sin() * radius;
        m.vertices.push(Vec3::new(x, -hy, z));
        m.normals.push(Vec3::new(x, 0.0, z).normalize_or_zero());
        m.uvs.push(Vec2::new(i as f32 / segments as f32, 0.0));
        m.vertices.push(Vec3::new(x, hy, z));
        m.normals.push(Vec3::new(x, 0.0, z).normalize_or_zero());
        m.uvs.push(Vec2::new(i as f32 / segments as f32, 1.0));
    }
    for i in 0..segments {
        let i0 = 2 + i * 2;
        let i1 = 2 + ((i + 1) % segments) * 2;
        let b0 = i0;
        let b1 = i1;
        let t0 = i0 + 1;
        let t1 = i1 + 1;
        m.faces.push(Face::quad(b0, b1, t1, t0));
        m.faces.push(Face::triangle(0, b1, b0));
        m.faces.push(Face::triangle(1, t0, t1));
    }
    m.recalculate_normals();
    m
}

pub fn sphere_mesh(radius: f32, segments: u32) -> EditableMesh {
    let segments = segments.max(4);
    let rings = segments;
    let mut m = EditableMesh::default();
    for j in 0..=rings {
        let v = j as f32 / rings as f32;
        let phi = v * std::f32::consts::PI;
        for i in 0..=segments {
            let u = i as f32 / segments as f32;
            let theta = u * std::f32::consts::TAU;
            let x = radius * phi.sin() * theta.cos();
            let y = radius * phi.cos();
            let z = radius * phi.sin() * theta.sin();
            m.vertices.push(Vec3::new(x, y, z));
            m.normals.push(Vec3::new(x, y, z).normalize_or_zero());
            m.uvs.push(Vec2::new(u, v));
        }
    }
    let cols = segments + 1;
    for j in 0..rings {
        for i in 0..segments {
            let a = j * cols + i;
            let b = a + cols;
            m.faces.push(Face::quad(a, a + 1, b + 1, b));
        }
    }
    m.recalculate_normals();
    m
}

pub fn cone_mesh(radius: f32, height: f32, segments: u32) -> EditableMesh {
    let segments = segments.max(3);
    let mut m = EditableMesh::default();
    let hy = height * 0.5;
    m.vertices.push(Vec3::new(0.0, -hy, 0.0)); // base center
    m.normals.push(-Vec3::Y);
    m.uvs.push(Vec2::splat(0.5));
    m.vertices.push(Vec3::new(0.0, hy, 0.0)); // tip
    m.normals.push(Vec3::Y);
    m.uvs.push(Vec2::splat(0.5));
    for i in 0..segments {
        let a = (i as f32 / segments as f32) * std::f32::consts::TAU;
        m.vertices
            .push(Vec3::new(a.cos() * radius, -hy, a.sin() * radius));
        m.normals.push(Vec3::new(a.cos(), 0.0, a.sin()));
        m.uvs.push(Vec2::new(i as f32 / segments as f32, 0.0));
    }
    for i in 0..segments {
        let b0 = 2 + i;
        let b1 = 2 + (i + 1) % segments;
        m.faces.push(Face::triangle(0, b1, b0));
        m.faces.push(Face::triangle(1, b0, b1));
    }
    m.recalculate_normals();
    m
}

fn pad_from_sketch(plane: &SketchPlane, curves: &[SketchCurve], distance: f32) -> EditableMesh {
    // Use first Rect or Circle or bounding box of lines
    let mut min = Vec2::splat(f32::MAX);
    let mut max = Vec2::splat(f32::MIN);
    let mut circle: Option<(Vec2, f32)> = None;
    for c in curves {
        match c {
            SketchCurve::Rect { min: a, max: b } => {
                min = min.min(*a);
                max = max.max(*b);
            }
            SketchCurve::Circle { center, radius } => {
                circle = Some((*center, *radius));
            }
            SketchCurve::Line { a, b } => {
                min = min.min(*a).min(*b);
                max = max.max(*a).max(*b);
            }
        }
    }
    if let Some((c, r)) = circle {
        let mut m = cylinder_mesh(r, distance, 32);
        // cylinder is Y-up; map plane
        orient_mesh(&mut m, plane);
        offset_mesh(&mut m, plane, c, distance);
        return m;
    }
    if min.x < max.x {
        let w = (max.x - min.x).max(0.01);
        let d = (max.y - min.y).max(0.01);
        let mut m = box_mesh(w, distance, d);
        orient_mesh(&mut m, plane);
        let center = (min + max) * 0.5;
        offset_mesh(&mut m, plane, center, distance);
        return m;
    }
    box_mesh(0.5, distance, 0.5)
}

fn orient_mesh(m: &mut EditableMesh, plane: &SketchPlane) {
    // default extrusion along Y in box/cylinder; remap
    match plane {
        SketchPlane::XY => {
            // extrude along Z: swap Y and Z
            for v in &mut m.vertices {
                let y = v.y;
                v.y = v.z;
                v.z = y;
            }
        }
        SketchPlane::XZ => {
            // already Y up for pad
        }
        SketchPlane::YZ => {
            for v in &mut m.vertices {
                let x = v.x;
                v.x = v.y;
                v.y = x;
            }
        }
    }
}

fn offset_mesh(m: &mut EditableMesh, plane: &SketchPlane, center: Vec2, _dist: f32) {
    let offset = match plane {
        SketchPlane::XY => Vec3::new(center.x, center.y, 0.0),
        SketchPlane::XZ => Vec3::new(center.x, 0.0, center.y),
        SketchPlane::YZ => Vec3::new(0.0, center.x, center.y),
    };
    for v in &mut m.vertices {
        *v += offset;
    }
}
