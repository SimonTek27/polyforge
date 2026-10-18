//! Approximate solid boolean on tessellated meshes (Phase 4 placeholder for B-rep).

use bevy::prelude::*;
use polyforge_core::{EditableMesh, Face};

/// Union: append meshes (no intersection removal — visual union of solids).
pub fn mesh_union(a: &EditableMesh, b: &EditableMesh) -> EditableMesh {
    let mut out = a.clone();
    let base = out.vertices.len() as u32;
    out.vertices.extend_from_slice(&b.vertices);
    out.normals.extend_from_slice(&b.normals);
    out.uvs.extend_from_slice(&b.uvs);
    for face in &b.faces {
        out.faces.push(Face {
            indices: face.indices.iter().map(|i| i + base).collect(),
        });
    }
    out.recalculate_normals();
    out
}

/// Cut approximation: keep only vertices of `a` outside an AABB expanded from `b`.
/// Crude; real Pocket needs a kernel.
pub fn mesh_cut_aabb(a: &EditableMesh, b: &EditableMesh) -> EditableMesh {
    let (min, max) = bounds(b);
    let mut keep_v = vec![true; a.vertices.len()];
    for (i, v) in a.vertices.iter().enumerate() {
        if v.x >= min.x && v.x <= max.x && v.y >= min.y && v.y <= max.y && v.z >= min.z && v.z <= max.z
        {
            keep_v[i] = false;
        }
    }
    // Remap faces that still have 3+ kept verts
    let mut map = vec![None; a.vertices.len()];
    let mut out = EditableMesh::default();
    for (i, &keep) in keep_v.iter().enumerate() {
        if keep {
            map[i] = Some(out.vertices.len() as u32);
            out.vertices.push(a.vertices[i]);
            out.normals
                .push(a.normals.get(i).copied().unwrap_or(Vec3::Y));
            out.uvs.push(a.uvs.get(i).copied().unwrap_or(Vec2::ZERO));
        }
    }
    for face in &a.faces {
        let idx: Vec<u32> = face
            .indices
            .iter()
            .filter_map(|i| map[*i as usize])
            .collect();
        if idx.len() >= 3 {
            out.faces.push(Face { indices: idx });
        }
    }
    out.recalculate_normals();
    out
}

fn bounds(m: &EditableMesh) -> (Vec3, Vec3) {
    if m.vertices.is_empty() {
        return (Vec3::ZERO, Vec3::ZERO);
    }
    let mut min = m.vertices[0];
    let mut max = m.vertices[0];
    for v in &m.vertices {
        min = min.min(*v);
        max = max.max(*v);
    }
    (min, max)
}
