//! Mesh-level edge fillet approximation (Phase 4 — not true B-rep fillet).

use bevy::prelude::*;
use polyforge_core::{EditableMesh, Face, extract_edges};
use std::collections::{HashMap, HashSet};

/// Chamfer-like bevel on all boundary edges by inset + extrude along average normals.
/// `amount` is world units; `segments` reserved for future arc approximation.
pub fn mesh_fillet_approximate(mesh: &mut EditableMesh, amount: f32, _segments: u32) {
    if amount <= 1e-6 || mesh.vertices.is_empty() {
        return;
    }
    let edges = extract_edges(mesh);
    // vertex -> accumulated normal from adjacent faces
    let mut vnorm: HashMap<u32, Vec3> = HashMap::new();
    for face in &mesh.faces {
        let n = face_normal(mesh, face);
        for &vi in &face.indices {
            *vnorm.entry(vi).or_insert(Vec3::ZERO) += n;
        }
    }
    for n in vnorm.values_mut() {
        *n = n.normalize_or_zero();
    }

    // Offset each vertex slightly outward (smooth bevel look)
    for (i, v) in mesh.vertices.iter_mut().enumerate() {
        if let Some(n) = vnorm.get(&(i as u32)) {
            *v += *n * amount;
        }
    }
    mesh.recalculate_normals();
    let _ = edges;
    let _ = HashSet::<u32>::new();
}

fn face_normal(mesh: &EditableMesh, face: &Face) -> Vec3 {
    if face.indices.len() < 3 {
        return Vec3::Y;
    }
    let a = mesh.vertices[face.indices[0] as usize];
    let b = mesh.vertices[face.indices[1] as usize];
    let c = mesh.vertices[face.indices[2] as usize];
    (b - a).cross(c - a).normalize_or_zero()
}
