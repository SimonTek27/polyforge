//! Merge / weld vertices.

use bevy::prelude::*;
use polyforge_core::{EditableMesh, Selection};
use crate::{OpResult, OpError};
use std::collections::HashMap;

/// Merge vertices that are within `distance` of each other.
/// If selection is non-empty, only selected vertices are candidates.
pub fn merge_by_distance(
    mesh: &mut EditableMesh,
    selection: &Selection,
    distance: f32,
) -> OpResult {
    let distance = distance.max(1e-6);
    let dist_sq = distance * distance;

    let candidates: Vec<u32> = if selection.vertices.is_empty() {
        (0..mesh.vertices.len() as u32).collect()
    } else {
        selection.vertices.iter().copied().collect()
    };

    if candidates.len() < 2 {
        return Err(OpError::EmptySelection);
    }

    // Union-find
    let mut parent: HashMap<u32, u32> = candidates.iter().map(|&i| (i, i)).collect();

    fn find(parent: &mut HashMap<u32, u32>, x: u32) -> u32 {
        let mut root = x;
        while parent[&root] != root {
            root = parent[&root];
        }
        let mut x = x;
        while parent[&x] != root {
            let next = parent[&x];
            parent.insert(x, root);
            x = next;
        }
        root
    }

    fn union(parent: &mut HashMap<u32, u32>, a: u32, b: u32) {
        let ra = find(parent, a);
        let rb = find(parent, b);
        if ra != rb {
            parent.insert(rb, ra);
        }
    }

    for i in 0..candidates.len() {
        for j in (i + 1)..candidates.len() {
            let a = candidates[i];
            let b = candidates[j];
            let pa = mesh.vertices[a as usize];
            let pb = mesh.vertices[b as usize];
            if pa.distance_squared(pb) <= dist_sq {
                union(&mut parent, a, b);
            }
        }
    }

    // Average positions per cluster
    let mut clusters: HashMap<u32, Vec<u32>> = HashMap::new();
    for &c in &candidates {
        let root = find(&mut parent, c);
        clusters.entry(root).or_default().push(c);
    }

    let mut remap: HashMap<u32, u32> = HashMap::new();
    for (_, members) in &clusters {
        let mut avg = Vec3::ZERO;
        for &m in members {
            avg += mesh.vertices[m as usize];
        }
        avg /= members.len() as f32;
        let keep = members[0];
        mesh.vertices[keep as usize] = avg;
        for &m in members {
            remap.insert(m, keep);
        }
    }

    // Remap face indices
    for face in &mut mesh.faces {
        for idx in &mut face.indices {
            if let Some(&new_i) = remap.get(idx) {
                *idx = new_i;
            }
        }
        // Remove degenerate consecutive duplicates
        face.indices.dedup();
        while face.indices.len() >= 2
            && face.indices.first() == face.indices.last()
        {
            face.indices.pop();
        }
    }

    mesh.faces.retain(|f| f.indices.len() >= 3);

    // Compact unreferenced
    compact_unused(mesh);
    mesh.recalculate_normals();
    Ok(())
}

fn compact_unused(mesh: &mut EditableMesh) {
    use std::collections::HashSet;
    let mut used = HashSet::new();
    for face in &mesh.faces {
        for &i in &face.indices {
            used.insert(i);
        }
    }
    if used.len() == mesh.vertices.len() {
        return;
    }
    let mut old_to_new = HashMap::new();
    let mut nv = Vec::new();
    let mut nn = Vec::new();
    let mut nu = Vec::new();
    for (i, v) in mesh.vertices.iter().enumerate() {
        let i = i as u32;
        if used.contains(&i) {
            old_to_new.insert(i, nv.len() as u32);
            nv.push(*v);
            nn.push(mesh.normals.get(i as usize).copied().unwrap_or(Vec3::Y));
            nu.push(mesh.uvs.get(i as usize).copied().unwrap_or(Vec2::ZERO));
        }
    }
    for face in &mut mesh.faces {
        for idx in &mut face.indices {
            *idx = old_to_new[idx];
        }
    }
    mesh.vertices = nv;
    mesh.normals = nn;
    mesh.uvs = nu;
}

/// Merge selected vertices to their average center.
pub fn merge_at_center(mesh: &mut EditableMesh, selection: &Selection) -> OpResult {
    if selection.vertices.len() < 2 {
        return Err(OpError::EmptySelection);
    }
    let mut avg = Vec3::ZERO;
    for &i in &selection.vertices {
        avg += mesh.vertices[i as usize];
    }
    avg /= selection.vertices.len() as f32;

    let keep = *selection.vertices.iter().next().unwrap();
    mesh.vertices[keep as usize] = avg;

    let mut remap = HashMap::new();
    for &i in &selection.vertices {
        remap.insert(i, keep);
    }

    for face in &mut mesh.faces {
        for idx in &mut face.indices {
            if let Some(&n) = remap.get(idx) {
                *idx = n;
            }
        }
        face.indices.dedup();
    }
    mesh.faces.retain(|f| f.indices.len() >= 3);
    compact_unused(mesh);
    mesh.recalculate_normals();
    Ok(())
}
