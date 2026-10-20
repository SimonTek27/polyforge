//! UV island detection, pack, and average.

use bevy::prelude::*;
use polyforge_core::EditableMesh;
use std::collections::{HashMap, HashSet, VecDeque};

/// Connected UV islands via face adjacency across non-seam edges.
pub fn find_uv_islands(mesh: &EditableMesh) -> Vec<HashSet<u32>> {
    let mut face_adj: HashMap<u32, Vec<u32>> = HashMap::new();
    // Edge -> faces
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
    for faces in e2f.values() {
        if faces.len() == 2 {
            face_adj.entry(faces[0]).or_default().push(faces[1]);
            face_adj.entry(faces[1]).or_default().push(faces[0]);
        }
    }

    let mut visited = HashSet::new();
    let mut islands = Vec::new();
    for fi in 0..mesh.faces.len() as u32 {
        if visited.contains(&fi) {
            continue;
        }
        let mut island_faces = HashSet::new();
        let mut q = VecDeque::new();
        q.push_back(fi);
        visited.insert(fi);
        while let Some(f) = q.pop_front() {
            island_faces.insert(f);
            if let Some(ns) = face_adj.get(&f) {
                for &n in ns {
                    if visited.insert(n) {
                        q.push_back(n);
                    }
                }
            }
        }
        let mut verts = HashSet::new();
        for &f in &island_faces {
            for &vi in &mesh.faces[f as usize].indices {
                verts.insert(vi);
            }
        }
        if !verts.is_empty() {
            islands.push(verts);
        }
    }
    islands
}

/// Pack islands into unit square with padding.
pub fn pack_uv_islands(mesh: &mut EditableMesh, padding: f32) {
    ensure_uvs(mesh);
    let islands = find_uv_islands(mesh);
    if islands.is_empty() {
        return;
    }

    // Normalize each island to its own bbox, then place in a grid
    let cols = (islands.len() as f32).sqrt().ceil() as usize;
    let cell = 1.0 / cols as f32;
    let pad = padding.clamp(0.0, cell * 0.4);

    for (idx, island) in islands.iter().enumerate() {
        let mut min = Vec2::splat(f32::MAX);
        let mut max = Vec2::splat(f32::MIN);
        for &vi in island {
            if let Some(uv) = mesh.uvs.get(vi as usize) {
                min = min.min(*uv);
                max = max.max(*uv);
            }
        }
        let size = (max - min).max(Vec2::splat(1e-6));
        let col = idx % cols;
        let row = idx / cols;
        let origin = Vec2::new(col as f32 * cell + pad, row as f32 * cell + pad);
        let scale = (cell - 2.0 * pad) / size.max_element().max(1e-6);

        for &vi in island {
            if let Some(uv) = mesh.uvs.get_mut(vi as usize) {
                *uv = origin + (*uv - min) * scale;
            }
        }
    }
}

/// Average UVs of selected vertices within each island (relax).
pub fn average_uv_islands(mesh: &mut EditableMesh, selected: &HashSet<u32>) {
    ensure_uvs(mesh);
    let islands = find_uv_islands(mesh);
    for island in islands {
        let targets: Vec<u32> = island.intersection(selected).copied().collect();
        if targets.is_empty() {
            continue;
        }
        // Laplacian: each selected UV = average of neighbors in island
        let edges = polyforge_core::extract_edges(mesh);
        let mut adj: HashMap<u32, Vec<u32>> = HashMap::new();
        for &(a, b) in &edges {
            if island.contains(&a) && island.contains(&b) {
                adj.entry(a).or_default().push(b);
                adj.entry(b).or_default().push(a);
            }
        }
        let mut updates = Vec::new();
        for &vi in &targets {
            if let Some(ns) = adj.get(&vi) {
                if ns.is_empty() {
                    continue;
                }
                let mut sum = Vec2::ZERO;
                for &n in ns {
                    sum += mesh.uvs.get(n as usize).copied().unwrap_or(Vec2::ZERO);
                }
                updates.push((vi, sum / ns.len() as f32));
            }
        }
        for (vi, uv) in updates {
            if let Some(slot) = mesh.uvs.get_mut(vi as usize) {
                *slot = uv;
            }
        }
    }
}

/// Select all UV verts in islands touched by `seed` selection.
pub fn expand_uv_islands(mesh: &EditableMesh, seed: &HashSet<u32>) -> HashSet<u32> {
    let mut out = HashSet::new();
    for island in find_uv_islands(mesh) {
        if island.iter().any(|v| seed.contains(v)) {
            out.extend(island);
        }
    }
    out
}

fn ensure_uvs(mesh: &mut EditableMesh) {
    while mesh.uvs.len() < mesh.vertices.len() {
        mesh.uvs.push(Vec2::ZERO);
    }
}
