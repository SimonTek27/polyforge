//! Loop cut: insert an edge ring across a strip of quads.

use bevy::prelude::*;
use polyforge_core::{EditableMesh, Face};
use crate::{OpResult, OpError};
use std::collections::{HashMap, HashSet};

/// Insert a loop cut starting from edge `(a, b)` at parameter `t` in 0..1 (0.5 = center).
/// Works best on pure quad strips; triangles are skipped gracefully.
pub fn loop_cut_from_edge(
    mesh: &mut EditableMesh,
    edge_a: u32,
    edge_b: u32,
    t: f32,
) -> OpResult {
    let t = t.clamp(0.05, 0.95);
    let start = if edge_a < edge_b {
        (edge_a, edge_b)
    } else {
        (edge_b, edge_a)
    };

    // Walk the quad strip in both directions from the starting edge
    let mut edges_to_cut: Vec<(u32, u32)> = Vec::new();
    let mut visited_edges: HashSet<(u32, u32)> = HashSet::new();

    // Build face list per edge
    let mut edge_faces: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
    for (fi, face) in mesh.faces.iter().enumerate() {
        let n = face.indices.len();
        for i in 0..n {
            let a = face.indices[i];
            let b = face.indices[(i + 1) % n];
            let e = if a < b { (a, b) } else { (b, a) };
            edge_faces.entry(e).or_default().push(fi);
        }
    }

    if !edge_faces.contains_key(&start) {
        return Err(OpError::InvalidTopology);
    }

    // BFS along opposite edges of quads
    let mut queue = vec![start];
    visited_edges.insert(start);

    while let Some(edge) = queue.pop() {
        edges_to_cut.push(edge);
        if let Some(faces) = edge_faces.get(&edge) {
            for &fi in faces {
                let face = &mesh.faces[fi];
                if face.indices.len() != 4 {
                    continue; // only quads form clean loops
                }
                // Find opposite edge
                if let Some(opp) = opposite_edge_in_quad(face, edge) {
                    if visited_edges.insert(opp) {
                        queue.push(opp);
                    }
                }
            }
        }
    }

    if edges_to_cut.is_empty() {
        return Err(OpError::InvalidTopology);
    }

    // Create midpoint vertices for each edge
    let mut edge_mid: HashMap<(u32, u32), u32> = HashMap::new();
    for &(a, b) in &edges_to_cut {
        let pa = mesh.vertices[a as usize];
        let pb = mesh.vertices[b as usize];
        let pos = pa.lerp(pb, t);
        let new_idx = mesh.vertices.len() as u32;
        mesh.vertices.push(pos);
        mesh.normals.push(Vec3::Y);
        let uva = mesh.uvs.get(a as usize).copied().unwrap_or(Vec2::ZERO);
        let uvb = mesh.uvs.get(b as usize).copied().unwrap_or(Vec2::ZERO);
        mesh.uvs.push(uva.lerp(uvb, t));
        edge_mid.insert((a, b), new_idx);
    }

    // Split each affected quad into two quads
    let mut new_faces = Vec::new();
    let affected: HashSet<usize> = edges_to_cut
        .iter()
        .filter_map(|e| edge_faces.get(e))
        .flatten()
        .copied()
        .collect();

    for (fi, face) in mesh.faces.iter().enumerate() {
        if !affected.contains(&fi) || face.indices.len() != 4 {
            new_faces.push(face.clone());
            continue;
        }

        // Find which two opposite edges were cut
        let mut mids = Vec::new();
        let n = 4;
        for i in 0..n {
            let a = face.indices[i];
            let b = face.indices[(i + 1) % n];
            let e = if a < b { (a, b) } else { (b, a) };
            if let Some(&mid) = edge_mid.get(&e) {
                mids.push((i, mid));
            }
        }

        if mids.len() == 2 {
            // Split quad into two quads along the midpoints
            let (i0, m0) = mids[0];
            let (i1, m1) = mids[1];
            // Order around the face
            let v = &face.indices;
            // Path A: from m0 along to m1 one way
            let mut path_a = vec![m0];
            let mut i = (i0 + 1) % 4;
            while i != i1 {
                path_a.push(v[i]);
                i = (i + 1) % 4;
            }
            path_a.push(m1);

            let mut path_b = vec![m1];
            i = (i1 + 1) % 4;
            while i != i0 {
                path_b.push(v[i]);
                i = (i + 1) % 4;
            }
            path_b.push(m0);

            if path_a.len() >= 3 {
                new_faces.push(Face {
                    indices: path_a,
                });
            }
            if path_b.len() >= 3 {
                new_faces.push(Face {
                    indices: path_b,
                });
            }
        } else {
            // Fallback: keep original
            new_faces.push(face.clone());
        }
    }

    mesh.faces = new_faces;
    mesh.recalculate_normals();
    Ok(())
}

fn opposite_edge_in_quad(face: &Face, edge: (u32, u32)) -> Option<(u32, u32)> {
    if face.indices.len() != 4 {
        return None;
    }
    let v = &face.indices;
    for i in 0..4 {
        let a = v[i];
        let b = v[(i + 1) % 4];
        let e = if a < b { (a, b) } else { (b, a) };
        if e == edge {
            // Opposite edge is two steps ahead
            let c = v[(i + 2) % 4];
            let d = v[(i + 3) % 4];
            return Some(if c < d { (c, d) } else { (d, c) });
        }
    }
    None
}

/// Convenience: loop-cut the first selected edge at center.
pub fn loop_cut_selection(
    mesh: &mut EditableMesh,
    selection: &polyforge_core::Selection,
    t: f32,
) -> OpResult {
    let edge = selection
        .edges
        .iter()
        .next()
        .copied()
        .ok_or(OpError::EmptySelection)?;
    loop_cut_from_edge(mesh, edge.0, edge.1, t)
}

/// Select the full edge loop starting from `(a, b)`.
pub fn select_edge_loop(mesh: &EditableMesh, a: u32, b: u32) -> HashSet<(u32, u32)> {
    let start = if a < b { (a, b) } else { (b, a) };
    let mut result = HashSet::new();

    let mut edge_faces: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
    for (fi, face) in mesh.faces.iter().enumerate() {
        let n = face.indices.len();
        for i in 0..n {
            let x = face.indices[i];
            let y = face.indices[(i + 1) % n];
            let e = if x < y { (x, y) } else { (y, x) };
            edge_faces.entry(e).or_default().push(fi);
        }
    }

    let mut visited = HashSet::new();
    let mut queue = vec![start];
    visited.insert(start);

    while let Some(edge) = queue.pop() {
        result.insert(edge);
        if let Some(faces) = edge_faces.get(&edge) {
            for &fi in faces {
                let face = &mesh.faces[fi];
                if face.indices.len() != 4 {
                    continue;
                }
                if let Some(opp) = opposite_edge_in_quad(face, edge) {
                    if visited.insert(opp) {
                        queue.push(opp);
                    }
                }
            }
        }
    }

    result
}
