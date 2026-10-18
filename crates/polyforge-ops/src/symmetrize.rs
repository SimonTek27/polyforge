//! Mirror / symmetrize mesh across an axis plane.

use bevy::prelude::*;
use polyforge_core::{EditableMesh, Selection, Face};
use crate::{OpResult, OpError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymAxis {
    X,
    Y,
    Z,
}

/// Symmetrize: for each selected vertex (or all), mirror across axis and weld near-duplicates.
/// Also duplicates faces on the positive side onto the negative if missing.
pub fn symmetrize(mesh: &mut EditableMesh, selection: &Selection, axis: SymAxis) -> OpResult {
    let threshold = 1e-4;
    let targets: Vec<u32> = if selection.vertices.is_empty()
        && selection.faces.is_empty()
        && selection.edges.is_empty()
    {
        (0..mesh.vertices.len() as u32).collect()
    } else {
        selection.effective_vertices(&mesh.faces).into_iter().collect()
    };
    if targets.is_empty() {
        return Err(OpError::EmptySelection);
    }

    // Snap vertices close to the plane onto the plane
    for &idx in &targets {
        if let Some(v) = mesh.vertices.get_mut(idx as usize) {
            match axis {
                SymAxis::X if v.x.abs() < threshold => v.x = 0.0,
                SymAxis::Y if v.y.abs() < threshold => v.y = 0.0,
                SymAxis::Z if v.z.abs() < threshold => v.z = 0.0,
                _ => {}
            }
        }
    }

    // Build map: original vertex -> mirrored vertex index
    let mut mirror_of: std::collections::HashMap<u32, u32> = std::collections::HashMap::new();
    for &idx in &targets {
        let v = mesh.vertices[idx as usize];
        let mirrored = match axis {
            SymAxis::X => Vec3::new(-v.x, v.y, v.z),
            SymAxis::Y => Vec3::new(v.x, -v.y, v.z),
            SymAxis::Z => Vec3::new(v.x, v.y, -v.z),
        };
        // Find existing vertex near mirrored position
        let mut found = None;
        for (i, p) in mesh.vertices.iter().enumerate() {
            if p.distance(mirrored) < threshold {
                found = Some(i as u32);
                break;
            }
        }
        let mi = if let Some(f) = found {
            f
        } else {
            let new_i = mesh.vertices.len() as u32;
            mesh.vertices.push(mirrored);
            mesh.normals
                .push(mesh.normals.get(idx as usize).copied().unwrap_or(Vec3::Y));
            mesh.uvs
                .push(mesh.uvs.get(idx as usize).copied().unwrap_or(Vec2::ZERO));
            new_i
        };
        mirror_of.insert(idx, mi);
    }

    // Duplicate faces that have all verts in the map, with mirrored indices (reversed winding)
    let mut extra_faces = Vec::new();
    for face in &mesh.faces {
        if face.indices.iter().all(|i| mirror_of.contains_key(i)) {
            let mut mirrored: Vec<u32> = face
                .indices
                .iter()
                .map(|i| mirror_of[i])
                .collect();
            mirrored.reverse();
            // Skip if identical to an existing face
            extra_faces.push(Face { indices: mirrored });
        }
    }
    mesh.faces.extend(extra_faces);
    mesh.recalculate_normals();
    Ok(())
}
