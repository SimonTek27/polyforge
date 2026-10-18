//! Extrude operation (faces / edges).

use bevy::prelude::*;
use polyforge_core::{EditableMesh, Selection, Face, topology::face_normal};
use crate::{OpResult, OpError};

/// Simple face extrude along normal by `distance`.
pub fn extrude_faces(
    mesh: &mut EditableMesh,
    selection: &Selection,
    distance: f32,
) -> OpResult {
    if selection.faces.is_empty() {
        return Err(OpError::EmptySelection);
    }

    let selected: Vec<usize> = selection.faces.iter().map(|&i| i as usize).collect();

    for &face_idx in &selected {
        if face_idx >= mesh.faces.len() {
            continue;
        }
        let face = mesh.faces[face_idx].clone();
        let normal = face_normal(mesh, &face);

        let mut new_indices = Vec::new();
        for &old_idx in &face.indices {
            let pos = mesh.vertices[old_idx as usize] + normal * distance;
            let new_idx = mesh.vertices.len() as u32;
            mesh.vertices.push(pos);
            mesh.normals.push(normal);
            mesh.uvs
                .push(mesh.uvs.get(old_idx as usize).copied().unwrap_or(Vec2::ZERO));
            new_indices.push(new_idx);
        }

        mesh.faces[face_idx] = Face {
            indices: new_indices.clone(),
        };

        let n = face.indices.len();
        for i in 0..n {
            let a = face.indices[i];
            let b = face.indices[(i + 1) % n];
            let c = new_indices[(i + 1) % n];
            let d = new_indices[i];
            mesh.faces.push(Face::quad(a, b, c, d));
        }
    }

    mesh.recalculate_normals();
    Ok(())
}
