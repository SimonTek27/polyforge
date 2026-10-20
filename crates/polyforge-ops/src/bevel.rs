//! Edge bevel / chamfer.

use bevy::prelude::*;
use polyforge_core::{EditableMesh, Selection, Face};
use crate::{OpResult, OpError};
use std::collections::{HashMap, HashSet};

/// Bevel selected edges by offsetting endpoints along the edge by `amount` (0..0.5).
/// Creates a new face strip along each beveled edge (1 segment).
pub fn bevel_edges(
    mesh: &mut EditableMesh,
    selection: &Selection,
    amount: f32,
) -> OpResult {
    if selection.edges.is_empty() {
        return Err(OpError::EmptySelection);
    }
    let amount = amount.clamp(0.01, 0.49);

    // For each selected edge, create two new vertices near the endpoints
    // and replace the edge with a bevel face.
    let edges: Vec<(u32, u32)> = selection.edges.iter().copied().collect();
    let mut replacements: HashMap<(u32, u32), (u32, u32)> = HashMap::new();

    for &(a, b) in &edges {
        let pa = mesh.vertices[a as usize];
        let pb = mesh.vertices[b as usize];
        let dir = (pb - pa).normalize_or_zero();
        let len = pa.distance(pb);
        let off = len * amount;

        let p_near_a = pa + dir * off;
        let p_near_b = pb - dir * off;

        let ia = mesh.vertices.len() as u32;
        mesh.vertices.push(p_near_a);
        mesh.normals.push(mesh.normals.get(a as usize).copied().unwrap_or(Vec3::Y));
        mesh.uvs
            .push(mesh.uvs.get(a as usize).copied().unwrap_or(Vec2::ZERO));

        let ib = mesh.vertices.len() as u32;
        mesh.vertices.push(p_near_b);
        mesh.normals.push(mesh.normals.get(b as usize).copied().unwrap_or(Vec3::Y));
        mesh.uvs
            .push(mesh.uvs.get(b as usize).copied().unwrap_or(Vec2::ZERO));

        let key = if a < b { (a, b) } else { (b, a) };
        // Store ordered as (vertex near a, vertex near b) relative to (min,max) key
        if a < b {
            replacements.insert(key, (ia, ib));
        } else {
            replacements.insert(key, (ib, ia));
        }

        // Bevel face (quad along the edge)
        mesh.faces.push(Face::quad(a, ia, ib, b));
    }

    // Update existing faces: replace edge endpoints with bevel vertices where needed
    // Simple approach: for each face edge that was beveled, insert the new verts
    let mut new_faces = Vec::new();
    let bevel_face_start = mesh.faces.len() - edges.len(); // faces we just added

    for (fi, face) in mesh.faces.iter().enumerate() {
        if fi >= bevel_face_start {
            new_faces.push(face.clone());
            continue;
        }

        let n = face.indices.len();
        let mut new_idx = Vec::new();
        for i in 0..n {
            let a = face.indices[i];
            let b = face.indices[(i + 1) % n];
            new_idx.push(a);
            let key = if a < b { (a, b) } else { (b, a) };
            if let Some(&(va, vb)) = replacements.get(&key) {
                // Insert the bevel vertex closer to a
                if a < b {
                    new_idx.push(va);
                } else {
                    new_idx.push(vb);
                }
            }
        }
        if new_idx.len() >= 3 {
            new_faces.push(Face { indices: new_idx });
        }
    }

    mesh.faces = new_faces;
    mesh.recalculate_normals();
    Ok(())
}
