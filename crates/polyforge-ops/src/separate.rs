//! Separate selection into a new mesh; join meshes.

use bevy::prelude::*;
use polyforge_core::{EditableMesh, Selection, Face};
use crate::OpError;
use std::collections::{HashMap, HashSet};

/// Extract selected faces into a new mesh (vertices compacted).
/// Original mesh keeps the remaining faces.
pub fn separate_faces(
    mesh: &mut EditableMesh,
    selection: &Selection,
) -> Result<EditableMesh, OpError> {
    if selection.faces.is_empty() {
        return Err(OpError::EmptySelection);
    }

    let mut used: HashSet<u32> = HashSet::new();
    for &fi in &selection.faces {
        if let Some(face) = mesh.faces.get(fi as usize) {
            for &vi in &face.indices {
                used.insert(vi);
            }
        }
    }

    let mut old_to_new: HashMap<u32, u32> = HashMap::new();
    let mut new_mesh = EditableMesh::default();
    for &old in &used {
        let ni = new_mesh.vertices.len() as u32;
        old_to_new.insert(old, ni);
        new_mesh.vertices.push(mesh.vertices[old as usize]);
        new_mesh.normals.push(
            mesh.normals
                .get(old as usize)
                .copied()
                .unwrap_or(Vec3::Y),
        );
        new_mesh
            .uvs
            .push(mesh.uvs.get(old as usize).copied().unwrap_or(Vec2::ZERO));
    }

    for &fi in &selection.faces {
        if let Some(face) = mesh.faces.get(fi as usize) {
            let indices: Vec<u32> = face
                .indices
                .iter()
                .map(|i| old_to_new[i])
                .collect();
            new_mesh.faces.push(Face { indices });
        }
    }
    new_mesh.recalculate_normals();

    // Remove separated faces from original (high index first)
    let mut faces_sorted: Vec<u32> = selection.faces.iter().copied().collect();
    faces_sorted.sort_unstable_by(|a, b| b.cmp(a));
    for fi in faces_sorted {
        if (fi as usize) < mesh.faces.len() {
            mesh.faces.remove(fi as usize);
        }
    }
    mesh.recalculate_normals();
    Ok(new_mesh)
}

/// Append `other` into `mesh` (vertex offset).
pub fn join_meshes(mesh: &mut EditableMesh, other: &EditableMesh) -> Result<(), OpError> {
    let base = mesh.vertices.len() as u32;
    mesh.vertices.extend_from_slice(&other.vertices);
    mesh.normals.extend_from_slice(&other.normals);
    mesh.uvs.extend_from_slice(&other.uvs);
    while mesh.normals.len() < mesh.vertices.len() {
        mesh.normals.push(Vec3::Y);
    }
    while mesh.uvs.len() < mesh.vertices.len() {
        mesh.uvs.push(Vec2::ZERO);
    }
    for face in &other.faces {
        let indices: Vec<u32> = face.indices.iter().map(|i| i + base).collect();
        mesh.faces.push(Face { indices });
    }
    for &(a, b) in &other.seams {
        mesh.seams.insert((a + base, b + base));
    }
    mesh.recalculate_normals();
    Ok(())
}
