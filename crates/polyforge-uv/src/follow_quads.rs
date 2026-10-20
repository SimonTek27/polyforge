//! Follow Active Quads — propagate UVs across a quad strip from a seed face.

use bevy::prelude::*;
use polyforge_core::EditableMesh;
use std::collections::{HashMap, HashSet, VecDeque};

/// Unwrap by following quads from `seed_face`, copying UV deltas across shared edges.
pub fn follow_active_quads(mesh: &mut EditableMesh, seed_face: u32) {
    if seed_face as usize >= mesh.faces.len() {
        return;
    }
    while mesh.uvs.len() < mesh.vertices.len() {
        mesh.uvs.push(Vec2::ZERO);
    }

    // Ensure seed has reasonable UVs: project seed face to unit square
    {
        let face = &mesh.faces[seed_face as usize];
        if face.indices.len() >= 3 {
            for (i, &vi) in face.indices.iter().enumerate() {
                let t = i as f32 / face.indices.len().max(1) as f32;
                if let Some(uv) = mesh.uvs.get_mut(vi as usize) {
                    // place on a simple ring if still zero-ish
                    if uv.length_squared() < 1e-8 {
                        *uv = Vec2::new(t.cos() * 0.3 + 0.5, t.sin() * 0.3 + 0.5);
                    }
                }
            }
            // For quads, use corners of unit square
            if face.indices.len() == 4 {
                let corners = [
                    Vec2::new(0.0, 0.0),
                    Vec2::new(1.0, 0.0),
                    Vec2::new(1.0, 1.0),
                    Vec2::new(0.0, 1.0),
                ];
                for (i, &vi) in face.indices.iter().enumerate() {
                    if let Some(uv) = mesh.uvs.get_mut(vi as usize) {
                        *uv = corners[i % 4];
                    }
                }
            }
        }
    }

    // Build edge -> faces (non-seam)
    let mut e2f: HashMap<(u32, u32), Vec<u32>> = HashMap::new();
    for (fi, face) in mesh.faces.iter().enumerate() {
        let n = face.indices.len();
        for i in 0..n {
            let a = face.indices[i];
            let b = face.indices[(i + 1) % n];
            let e = if a < b { (a, b) } else { (b, a) };
            if mesh.seams.contains(&e) {
                continue;
            }
            e2f.entry(e).or_default().push(fi as u32);
        }
    }

    let mut visited = HashSet::new();
    let mut q = VecDeque::new();
    q.push_back(seed_face);
    visited.insert(seed_face);

    while let Some(fi) = q.pop_front() {
        let face = mesh.faces[fi as usize].clone();
        if face.indices.len() != 4 {
            // still expand to neighbors
            expand_neighbors(&face, fi, &e2f, &mut visited, &mut q);
            continue;
        }
        // For each edge, find neighboring quad and assign UVs by edge correspondence
        let n = 4;
        for i in 0..n {
            let a = face.indices[i];
            let b = face.indices[(i + 1) % n];
            let e = if a < b { (a, b) } else { (b, a) };
            let Some(faces) = e2f.get(&e) else { continue };
            for &nf in faces {
                if nf == fi || visited.contains(&nf) {
                    continue;
                }
                let nface = mesh.faces[nf as usize].clone();
                if nface.indices.len() != 4 {
                    visited.insert(nf);
                    q.push_back(nf);
                    continue;
                }
                // Find edge orientation in neighbor
                if let Some((j, rev)) = find_edge_orient(&nface, a, b) {
                    let ua = mesh.uvs[a as usize];
                    let ub = mesh.uvs[b as usize];
                    // Opposite edge verts on neighbor
                    let c = nface.indices[(j + 2) % 4];
                    let d = nface.indices[(j + 3) % 4];
                    // Extrude UV across edge: offset = perpendicular in UV of (ub-ua)
                    let edge_uv = ub - ua;
                    let perp = Vec2::new(-edge_uv.y, edge_uv.x);
                    // Place c,d based on orientation
                    if rev {
                        // neighbor edge is b->a
                        if let Some(uv) = mesh.uvs.get_mut(c as usize) {
                            *uv = ua + perp;
                        }
                        if let Some(uv) = mesh.uvs.get_mut(d as usize) {
                            *uv = ub + perp;
                        }
                    } else {
                        if let Some(uv) = mesh.uvs.get_mut(d as usize) {
                            *uv = ua + perp;
                        }
                        if let Some(uv) = mesh.uvs.get_mut(c as usize) {
                            *uv = ub + perp;
                        }
                    }
                }
                visited.insert(nf);
                q.push_back(nf);
            }
        }
    }
}

fn expand_neighbors(
    face: &polyforge_core::Face,
    fi: u32,
    e2f: &HashMap<(u32, u32), Vec<u32>>,
    visited: &mut HashSet<u32>,
    q: &mut VecDeque<u32>,
) {
    let n = face.indices.len();
    for i in 0..n {
        let a = face.indices[i];
        let b = face.indices[(i + 1) % n];
        let e = if a < b { (a, b) } else { (b, a) };
        if let Some(faces) = e2f.get(&e) {
            for &nf in faces {
                if nf != fi && visited.insert(nf) {
                    q.push_back(nf);
                }
            }
        }
    }
}

fn find_edge_orient(face: &polyforge_core::Face, a: u32, b: u32) -> Option<(usize, bool)> {
    let n = face.indices.len();
    for i in 0..n {
        let x = face.indices[i];
        let y = face.indices[(i + 1) % n];
        if x == a && y == b {
            return Some((i, false));
        }
        if x == b && y == a {
            return Some((i, true));
        }
    }
    None
}
