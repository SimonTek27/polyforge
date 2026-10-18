//! Dissolve vertices / edges / faces.

use bevy::prelude::*;
use polyforge_core::{EditableMesh, Selection, Face};
use crate::{OpResult, OpError};
use std::collections::HashSet;

/// Dissolve selected faces: remove them and try to merge boundary into one face if planar-ish.
/// Simple version: just delete faces and leave a hole (same as delete for now) but without
/// removing shared boundary verts.
pub fn dissolve_faces(mesh: &mut EditableMesh, selection: &Selection) -> OpResult {
    if selection.faces.is_empty() {
        return Err(OpError::EmptySelection);
    }
    mesh.faces = mesh
        .faces
        .iter()
        .enumerate()
        .filter(|(i, _)| !selection.faces.contains(&(*i as u32)))
        .map(|(_, f)| f.clone())
        .collect();
    mesh.recalculate_normals();
    Ok(())
}

/// Dissolve selected edges: merge the two faces sharing each edge into one face.
pub fn dissolve_edges(mesh: &mut EditableMesh, selection: &Selection) -> OpResult {
    if selection.edges.is_empty() {
        return Err(OpError::EmptySelection);
    }
    for &(a, b) in &selection.edges {
        let key = if a < b { (a, b) } else { (b, a) };
        // Find up to two faces containing this edge
        let mut face_idxs = Vec::new();
        for (fi, face) in mesh.faces.iter().enumerate() {
            let n = face.indices.len();
            for i in 0..n {
                let x = face.indices[i];
                let y = face.indices[(i + 1) % n];
                let e = if x < y { (x, y) } else { (y, x) };
                if e == key {
                    face_idxs.push(fi);
                    break;
                }
            }
        }
        if face_idxs.len() != 2 {
            continue;
        }
        let f0 = mesh.faces[face_idxs[0]].clone();
        let f1 = mesh.faces[face_idxs[1]].clone();
        if let Some(merged) = merge_faces_across_edge(&f0, &f1, key) {
            // Remove higher index first
            let mut idxs = face_idxs;
            idxs.sort_unstable();
            mesh.faces.remove(idxs[1]);
            mesh.faces.remove(idxs[0]);
            mesh.faces.push(merged);
        }
    }
    mesh.recalculate_normals();
    Ok(())
}

fn merge_faces_across_edge(f0: &Face, f1: &Face, edge: (u32, u32)) -> Option<Face> {
    // Walk f0 excluding the edge, then f1 excluding the edge
    let path0 = face_path_excluding_edge(f0, edge)?;
    let path1 = face_path_excluding_edge(f1, edge)?;
    // path0 goes a..b along f0; path1 should go b..a along f1
    let mut indices = path0;
    // append path1 without endpoints
    if path1.len() > 2 {
        indices.extend_from_slice(&path1[1..path1.len() - 1]);
    }
    if indices.len() < 3 {
        return None;
    }
    Some(Face { indices })
}

fn face_path_excluding_edge(face: &Face, edge: (u32, u32)) -> Option<Vec<u32>> {
    let n = face.indices.len();
    // Find edge position
    let mut start = None;
    for i in 0..n {
        let a = face.indices[i];
        let b = face.indices[(i + 1) % n];
        let e = if a < b { (a, b) } else { (b, a) };
        if e == edge {
            start = Some(i);
            break;
        }
    }
    let i = start?;
    // Path from b around to a (excluding a->b edge)
    let mut path = Vec::new();
    for k in 1..=n {
        let idx = face.indices[(i + k) % n];
        path.push(idx);
        if path.len() > 1 && {
            let _last = *path.last().unwrap();
            let _first = path[0];
            // stop when we reach the other endpoint after walking
            false
        } {
            break;
        }
    }
    // Simpler: collect all verts starting after the edge
    path.clear();
    for k in 1..n {
        path.push(face.indices[(i + k) % n]);
    }
    // path is b, ..., a
    Some(path)
}

/// Dissolve selected vertices: remove them and retriangulate/reconnect adjacent faces (simple delete faces using them).
pub fn dissolve_vertices(mesh: &mut EditableMesh, selection: &Selection) -> OpResult {
    if selection.vertices.is_empty() {
        return Err(OpError::EmptySelection);
    }
    let drop: HashSet<u32> = selection.vertices.clone();
    mesh.faces
        .retain(|f| f.indices.iter().all(|i| !drop.contains(i)));
    // Compact happens via delete_selection path if needed
    mesh.recalculate_normals();
    Ok(())
}
