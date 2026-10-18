//! Edit operations for vertices, edges and faces.

use bevy::prelude::*;
use polyforge_core::{EditableMesh, Selection, Face, topology::face_normal};
use crate::{OpResult, OpError};
use std::collections::{HashMap, HashSet};

// ─────────────────────────────────────────────────────────────
// Vertex operations
// ─────────────────────────────────────────────────────────────

/// Delete selected vertices and any faces that used them.
/// Remaining face indices are re-compacted; orphan verts cleaned up.
pub fn delete_vertices(mesh: &mut EditableMesh, selection: &Selection) -> OpResult {
    if selection.vertices.is_empty() {
        return Err(OpError::EmptySelection);
    }

    let to_delete: HashSet<u32> = selection.vertices.clone();

    // Keep faces that don't reference deleted verts
    let mut new_faces = Vec::new();
    for face in &mesh.faces {
        if face.indices.iter().any(|i| to_delete.contains(i)) {
            continue;
        }
        new_faces.push(face.clone());
    }
    mesh.faces = new_faces;

    // Compact vertices
    compact_mesh(mesh);
    mesh.recalculate_normals();
    Ok(())
}

/// Move selected vertices by delta (alias used by edit pipeline).
pub fn move_vertices(
    mesh: &mut EditableMesh,
    selection: &Selection,
    delta: Vec3,
) -> OpResult {
    if selection.vertices.is_empty() {
        return Err(OpError::EmptySelection);
    }
    for &idx in &selection.vertices {
        if let Some(v) = mesh.vertices.get_mut(idx as usize) {
            *v += delta;
        }
    }
    mesh.recalculate_normals();
    Ok(())
}

// ─────────────────────────────────────────────────────────────
// Edge operations
// ─────────────────────────────────────────────────────────────

/// Move selected edges by translating both endpoints.
pub fn move_edges(
    mesh: &mut EditableMesh,
    selection: &Selection,
    delta: Vec3,
) -> OpResult {
    if selection.edges.is_empty() {
        return Err(OpError::EmptySelection);
    }
    let mut moved = HashSet::new();
    for &(a, b) in &selection.edges {
        for idx in [a, b] {
            if moved.insert(idx) {
                if let Some(v) = mesh.vertices.get_mut(idx as usize) {
                    *v += delta;
                }
            }
        }
    }
    mesh.recalculate_normals();
    Ok(())
}

/// Delete selected edges by removing faces that contain them.
/// (Simple approach: any face using a selected edge is removed.)
pub fn delete_edges(mesh: &mut EditableMesh, selection: &Selection) -> OpResult {
    if selection.edges.is_empty() {
        return Err(OpError::EmptySelection);
    }

    let edge_set: HashSet<(u32, u32)> = selection.edges.clone();

    let mut new_faces = Vec::new();
    for face in &mesh.faces {
        let n = face.indices.len();
        let mut uses_edge = false;
        for i in 0..n {
            let a = face.indices[i];
            let b = face.indices[(i + 1) % n];
            let e = if a < b { (a, b) } else { (b, a) };
            if edge_set.contains(&e) {
                uses_edge = true;
                break;
            }
        }
        if !uses_edge {
            new_faces.push(face.clone());
        }
    }
    mesh.faces = new_faces;
    compact_mesh(mesh);
    mesh.recalculate_normals();
    Ok(())
}

/// Split each selected edge at its midpoint, inserting a new vertex.
pub fn split_edges(mesh: &mut EditableMesh, selection: &Selection) -> OpResult {
    if selection.edges.is_empty() {
        return Err(OpError::EmptySelection);
    }

    // Map old edge -> new midpoint vertex index
    let mut midpoints: HashMap<(u32, u32), u32> = HashMap::new();

    for &(a, b) in &selection.edges {
        let key = if a < b { (a, b) } else { (b, a) };
        if midpoints.contains_key(&key) {
            continue;
        }
        let pa = mesh.vertices[a as usize];
        let pb = mesh.vertices[b as usize];
        let mid = (pa + pb) * 0.5;
        let new_idx = mesh.vertices.len() as u32;
        mesh.vertices.push(mid);
        mesh.normals.push(Vec3::Y);
        let uv_a = mesh.uvs.get(a as usize).copied().unwrap_or(Vec2::ZERO);
        let uv_b = mesh.uvs.get(b as usize).copied().unwrap_or(Vec2::ZERO);
        mesh.uvs.push((uv_a + uv_b) * 0.5);
        midpoints.insert(key, new_idx);
    }

    // Rebuild faces: replace any selected edge with two edges through midpoint
    let mut new_faces = Vec::new();
    for face in &mesh.faces {
        let n = face.indices.len();
        let mut new_indices = Vec::new();
        for i in 0..n {
            let a = face.indices[i];
            let b = face.indices[(i + 1) % n];
            new_indices.push(a);
            let key = if a < b { (a, b) } else { (b, a) };
            if let Some(&mid) = midpoints.get(&key) {
                new_indices.push(mid);
            }
        }
        // Only keep faces that still form a polygon
        if new_indices.len() >= 3 {
            new_faces.push(Face {
                indices: new_indices,
            });
        }
    }
    mesh.faces = new_faces;
    mesh.recalculate_normals();
    Ok(())
}

// ─────────────────────────────────────────────────────────────
// Face operations
// ─────────────────────────────────────────────────────────────

/// Translate selected faces along their average normal (or arbitrary delta).
pub fn move_faces(
    mesh: &mut EditableMesh,
    selection: &Selection,
    delta: Vec3,
) -> OpResult {
    if selection.faces.is_empty() {
        return Err(OpError::EmptySelection);
    }

    let mut moved = HashSet::new();
    for &fi in &selection.faces {
        if let Some(face) = mesh.faces.get(fi as usize) {
            for &vi in &face.indices {
                if moved.insert(vi) {
                    if let Some(v) = mesh.vertices.get_mut(vi as usize) {
                        *v += delta;
                    }
                }
            }
        }
    }
    mesh.recalculate_normals();
    Ok(())
}

/// Move selected faces along their own normals by `distance`.
pub fn move_faces_along_normal(
    mesh: &mut EditableMesh,
    selection: &Selection,
    distance: f32,
) -> OpResult {
    if selection.faces.is_empty() {
        return Err(OpError::EmptySelection);
    }

    // Per-vertex accumulated offset (average if shared)
    let mut offsets: HashMap<u32, (Vec3, u32)> = HashMap::new();

    for &fi in &selection.faces {
        if let Some(face) = mesh.faces.get(fi as usize) {
            let n = face_normal(mesh, face);
            for &vi in &face.indices {
                let entry = offsets.entry(vi).or_insert((Vec3::ZERO, 0));
                entry.0 += n * distance;
                entry.1 += 1;
            }
        }
    }

    for (vi, (sum, count)) in offsets {
        if let Some(v) = mesh.vertices.get_mut(vi as usize) {
            *v += sum / count as f32;
        }
    }
    mesh.recalculate_normals();
    Ok(())
}

/// Delete selected faces.
pub fn delete_faces_op(mesh: &mut EditableMesh, selection: &Selection) -> OpResult {
    if selection.faces.is_empty() {
        return Err(OpError::EmptySelection);
    }
    let mut new_faces = Vec::new();
    for (i, face) in mesh.faces.iter().enumerate() {
        if !selection.faces.contains(&(i as u32)) {
            new_faces.push(face.clone());
        }
    }
    mesh.faces = new_faces;
    compact_mesh(mesh);
    mesh.recalculate_normals();
    Ok(())
}

/// Inset selected faces by a factor (0..1). Creates a smaller face and side walls.
pub fn inset_faces(
    mesh: &mut EditableMesh,
    selection: &Selection,
    amount: f32,
) -> OpResult {
    if selection.faces.is_empty() {
        return Err(OpError::EmptySelection);
    }

    let amount = amount.clamp(0.01, 0.95);
    let selected: Vec<usize> = selection.faces.iter().map(|&i| i as usize).collect();

    for &face_idx in &selected {
        if face_idx >= mesh.faces.len() {
            continue;
        }
        let face = mesh.faces[face_idx].clone();
        let n = face.indices.len();
        if n < 3 {
            continue;
        }

        // Face center
        let mut center = Vec3::ZERO;
        for &vi in &face.indices {
            center += mesh.vertices[vi as usize];
        }
        center /= n as f32;

        // New inset vertices
        let mut new_indices = Vec::new();
        for &old_idx in &face.indices {
            let pos = mesh.vertices[old_idx as usize].lerp(center, amount);
            let new_idx = mesh.vertices.len() as u32;
            mesh.vertices.push(pos);
            mesh.normals.push(mesh.normals.get(old_idx as usize).copied().unwrap_or(Vec3::Y));
            mesh.uvs
                .push(mesh.uvs.get(old_idx as usize).copied().unwrap_or(Vec2::ZERO));
            new_indices.push(new_idx);
        }

        // Replace original face with inset face
        mesh.faces[face_idx] = Face {
            indices: new_indices.clone(),
        };

        // Side faces (quads between original and inset)
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

// ─────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────

/// Remove unreferenced vertices and reindex faces.
fn compact_mesh(mesh: &mut EditableMesh) {
    let mut used = HashSet::new();
    for face in &mesh.faces {
        for &i in &face.indices {
            used.insert(i);
        }
    }

    if used.len() == mesh.vertices.len() {
        return; // nothing to compact
    }

    let mut old_to_new: HashMap<u32, u32> = HashMap::new();
    let mut new_verts = Vec::new();
    let mut new_normals = Vec::new();
    let mut new_uvs = Vec::new();

    for (i, v) in mesh.vertices.iter().enumerate() {
        let i = i as u32;
        if used.contains(&i) {
            let new_i = new_verts.len() as u32;
            old_to_new.insert(i, new_i);
            new_verts.push(*v);
            new_normals.push(mesh.normals.get(i as usize).copied().unwrap_or(Vec3::Y));
            new_uvs.push(mesh.uvs.get(i as usize).copied().unwrap_or(Vec2::ZERO));
        }
    }

    for face in &mut mesh.faces {
        for idx in &mut face.indices {
            *idx = old_to_new[idx];
        }
    }

    mesh.vertices = new_verts;
    mesh.normals = new_normals;
    mesh.uvs = new_uvs;
}

/// Unified delete based on what is selected.
pub fn delete_selection(mesh: &mut EditableMesh, selection: &mut Selection) -> OpResult {
    let result = if !selection.faces.is_empty() {
        delete_faces_op(mesh, selection)
    } else if !selection.edges.is_empty() {
        delete_edges(mesh, selection)
    } else if !selection.vertices.is_empty() {
        delete_vertices(mesh, selection)
    } else {
        Err(OpError::EmptySelection)
    };
    if result.is_ok() {
        selection.clear();
    }
    result
}

/// Unified move: applies delta to whatever is selected.
pub fn move_selection(
    mesh: &mut EditableMesh,
    selection: &Selection,
    delta: Vec3,
) -> OpResult {
    if !selection.faces.is_empty() {
        move_faces(mesh, selection, delta)
    } else if !selection.edges.is_empty() {
        move_edges(mesh, selection, delta)
    } else if !selection.vertices.is_empty() {
        move_vertices(mesh, selection, delta)
    } else {
        // Fall back: move everything if empty (object-like)
        for v in &mut mesh.vertices {
            *v += delta;
        }
        mesh.recalculate_normals();
        Ok(())
    }
}
