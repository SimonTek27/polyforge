//! Rip vertices along selected edges / selection.

use bevy::prelude::*;
use polyforge_core::{EditableMesh, Selection};
use crate::{OpResult, OpError};
use std::collections::{HashMap, HashSet};

/// Rip: duplicate selected vertices and reassign faces on one side to the copies,
/// offset slightly along view or +X.
pub fn rip_vertices(mesh: &mut EditableMesh, selection: &Selection, offset: Vec3) -> OpResult {
    if selection.vertices.is_empty() && selection.edges.is_empty() {
        return Err(OpError::EmptySelection);
    }

    let mut verts: HashSet<u32> = selection.vertices.clone();
    for &(a, b) in &selection.edges {
        verts.insert(a);
        verts.insert(b);
    }

    // Duplicate vertices
    let mut map: HashMap<u32, u32> = HashMap::new();
    for &vi in &verts {
        let pos = mesh.vertices[vi as usize] + offset;
        let ni = mesh.vertices.len() as u32;
        mesh.vertices.push(pos);
        mesh.normals
            .push(mesh.normals.get(vi as usize).copied().unwrap_or(Vec3::Y));
        mesh.uvs
            .push(mesh.uvs.get(vi as usize).copied().unwrap_or(Vec2::ZERO));
        map.insert(vi, ni);
    }

    // For each face that uses any ripped vertex, if the face is "on the positive X of center",
    // remap to duplicated verts. Heuristic: faces whose centroid is on the +offset side.
    let axis = offset.normalize_or_zero();
    for face in &mut mesh.faces {
        let uses = face.indices.iter().any(|i| map.contains_key(i));
        if !uses {
            continue;
        }
        let mut c = Vec3::ZERO;
        for &i in &face.indices {
            c += mesh.vertices.get(i as usize).copied().unwrap_or(Vec3::ZERO);
        }
        c /= face.indices.len().max(1) as f32;
        // Use original positions approximation: if face normal points with offset, remap
        if c.dot(axis) >= 0.0 || axis == Vec3::ZERO {
            for idx in &mut face.indices {
                if let Some(&ni) = map.get(idx) {
                    *idx = ni;
                }
            }
        }
    }
    mesh.recalculate_normals();
    Ok(())
}

/// Simple edge/vertex slide: move selected verts along connected edge direction by factor.
pub fn slide_vertices(mesh: &mut EditableMesh, selection: &Selection, factor: f32) -> OpResult {
    let verts = selection.effective_vertices(&mesh.faces);
    if verts.is_empty() {
        return Err(OpError::EmptySelection);
    }

    // For each vertex, average direction toward neighbors not in selection
    let edges = polyforge_core::extract_edges(mesh);
    let mut adj: HashMap<u32, Vec<u32>> = HashMap::new();
    for &(a, b) in &edges {
        adj.entry(a).or_default().push(b);
        adj.entry(b).or_default().push(a);
    }

    let mut deltas: HashMap<u32, Vec3> = HashMap::new();
    for &vi in &verts {
        let pos = mesh.vertices[vi as usize];
        let mut dir = Vec3::ZERO;
        let mut count = 0u32;
        if let Some(ns) = adj.get(&vi) {
            for &n in ns {
                if !verts.contains(&n) {
                    dir += mesh.vertices[n as usize] - pos;
                    count += 1;
                }
            }
        }
        if count > 0 {
            deltas.insert(vi, (dir / count as f32) * factor);
        }
    }
    for (vi, d) in deltas {
        mesh.vertices[vi as usize] += d;
    }
    mesh.recalculate_normals();
    Ok(())
}
