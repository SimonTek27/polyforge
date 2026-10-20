//! Face subdivide and triangulate.

use bevy::prelude::*;
use polyforge_core::{EditableMesh, Selection, Face};
use crate::{OpResult, OpError};
use std::collections::HashMap;

/// Triangulate selected faces (or all faces if selection empty).
/// Quads → 2 tris; n-gons → fan from first vertex.
pub fn triangulate_faces(mesh: &mut EditableMesh, selection: &Selection) -> OpResult {
    let targets: Vec<usize> = if selection.faces.is_empty() {
        (0..mesh.faces.len()).collect()
    } else {
        selection.faces.iter().map(|&i| i as usize).collect()
    };
    if targets.is_empty() {
        return Err(OpError::EmptySelection);
    }

    let mut new_faces = Vec::new();
    let target_set: std::collections::HashSet<usize> = targets.into_iter().collect();

    for (fi, face) in mesh.faces.iter().enumerate() {
        if !target_set.contains(&fi) || face.indices.len() <= 3 {
            new_faces.push(face.clone());
            continue;
        }
        let idx = &face.indices;
        // Fan triangulation
        for i in 1..idx.len() - 1 {
            new_faces.push(Face::triangle(idx[0], idx[i], idx[i + 1]));
        }
    }
    mesh.faces = new_faces;
    mesh.recalculate_normals();
    Ok(())
}

/// Subdivide selected faces once (Catmull-Clark-like for quads: face point + edge mids).
/// Simple scheme: each edge gets a midpoint; each face becomes quads (or tris for triangles).
pub fn subdivide_faces(mesh: &mut EditableMesh, selection: &Selection) -> OpResult {
    let targets: Vec<usize> = if selection.faces.is_empty() {
        (0..mesh.faces.len()).collect()
    } else {
        selection.faces.iter().map(|&i| i as usize).collect()
    };
    if targets.is_empty() && mesh.faces.is_empty() {
        return Err(OpError::EmptySelection);
    }

    let target_set: std::collections::HashSet<usize> = targets.into_iter().collect();

    // Midpoint cache for edges
    let mut edge_mid: HashMap<(u32, u32), u32> = HashMap::new();
    let mut mid = |mesh: &mut EditableMesh, a: u32, b: u32, edge_mid: &mut HashMap<(u32, u32), u32>| -> u32 {
        let key = if a < b { (a, b) } else { (b, a) };
        if let Some(&i) = edge_mid.get(&key) {
            return i;
        }
        let pa = mesh.vertices[a as usize];
        let pb = mesh.vertices[b as usize];
        let pos = (pa + pb) * 0.5;
        let new_i = mesh.vertices.len() as u32;
        mesh.vertices.push(pos);
        mesh.normals.push(Vec3::Y);
        let uva = mesh.uvs.get(a as usize).copied().unwrap_or(Vec2::ZERO);
        let uvb = mesh.uvs.get(b as usize).copied().unwrap_or(Vec2::ZERO);
        mesh.uvs.push((uva + uvb) * 0.5);
        edge_mid.insert(key, new_i);
        new_i
    };

    let mut new_faces = Vec::new();

    let faces_snapshot = mesh.faces.clone();
    for (fi, face) in faces_snapshot.iter().enumerate() {
        if !target_set.contains(&fi) {
            new_faces.push(face.clone());
            continue;
        }
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
        let center_idx = mesh.vertices.len() as u32;
        mesh.vertices.push(center);
        mesh.normals.push(Vec3::Y);
        mesh.uvs.push(Vec2::splat(0.5));

        // Edge mids
        let mut mids = Vec::with_capacity(n);
        for i in 0..n {
            let a = face.indices[i];
            let b = face.indices[(i + 1) % n];
            mids.push(mid(mesh, a, b, &mut edge_mid));
        }

        // For each original corner: quad (corner, mid_next, center, mid_prev)
        // Actually: (v_i, mid_i, center, mid_{i-1})
        for i in 0..n {
            let v = face.indices[i];
            let m_prev = mids[(i + n - 1) % n];
            let m_next = mids[i];
            new_faces.push(Face::quad(v, m_next, center_idx, m_prev));
        }
    }

    mesh.faces = new_faces;
    mesh.recalculate_normals();
    Ok(())
}
