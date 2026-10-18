//! Fill: create a face from selected vertices or edges.

use bevy::prelude::*;
use polyforge_core::{EditableMesh, Selection, Face};
use crate::{OpResult, OpError};

/// Create a single face from selected vertices (order by selection iteration – best for 3–4 verts).
pub fn fill_from_vertices(mesh: &mut EditableMesh, selection: &Selection) -> OpResult {
    let mut indices: Vec<u32> = selection.vertices.iter().copied().collect();
    if indices.len() < 3 {
        return Err(OpError::EmptySelection);
    }
    // Sort by angle around centroid for a more stable polygon
    let mut center = Vec3::ZERO;
    for &i in &indices {
        center += mesh.vertices[i as usize];
    }
    center /= indices.len() as f32;

    // Project onto best-fit plane using average normal guess (XY if flat)
    indices.sort_by(|&a, &b| {
        let pa = mesh.vertices[a as usize] - center;
        let pb = mesh.vertices[b as usize] - center;
        let ang_a = pa.x.atan2(pa.z);
        let ang_b = pb.x.atan2(pb.z);
        ang_a.partial_cmp(&ang_b).unwrap_or(std::cmp::Ordering::Equal)
    });

    mesh.faces.push(Face { indices });
    mesh.recalculate_normals();
    Ok(())
}

/// Duplicate selected geometry as new vertices/faces offset slightly.
pub fn duplicate_selection(
    mesh: &mut EditableMesh,
    selection: &Selection,
    offset: Vec3,
) -> OpResult {
    let effective = selection.effective_vertices(&mesh.faces);
    if effective.is_empty() && selection.faces.is_empty() {
        return Err(OpError::EmptySelection);
    }

    // Map old vertex -> new vertex
    use std::collections::HashMap;
    let mut remap = HashMap::new();
    let verts: Vec<u32> = if !selection.faces.is_empty() {
        effective.into_iter().collect()
    } else {
        selection.vertices.iter().copied().collect()
    };

    for &old in &verts {
        let pos = mesh.vertices[old as usize] + offset;
        let new_i = mesh.vertices.len() as u32;
        mesh.vertices.push(pos);
        mesh.normals
            .push(mesh.normals.get(old as usize).copied().unwrap_or(Vec3::Y));
        mesh.uvs
            .push(mesh.uvs.get(old as usize).copied().unwrap_or(Vec2::ZERO));
        remap.insert(old, new_i);
    }

    if !selection.faces.is_empty() {
        for &fi in &selection.faces {
            if let Some(face) = mesh.faces.get(fi as usize) {
                let new_indices: Vec<u32> = face
                    .indices
                    .iter()
                    .filter_map(|i| remap.get(i).copied())
                    .collect();
                if new_indices.len() >= 3 {
                    mesh.faces.push(Face {
                        indices: new_indices,
                    });
                }
            }
        }
    }

    mesh.recalculate_normals();
    Ok(())
}
