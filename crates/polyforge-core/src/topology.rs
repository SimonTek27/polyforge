//! Topology helpers: edges, adjacency, connected face regions (cells).

use super::mesh::{EditableMesh, Face};
use std::collections::{HashMap, HashSet, VecDeque};

/// Returns all unique edges of the mesh as (min, max) pairs.
pub fn extract_edges(mesh: &EditableMesh) -> Vec<(u32, u32)> {
    let mut edges = Vec::new();
    for face in &mesh.faces {
        let n = face.indices.len();
        for i in 0..n {
            let a = face.indices[i];
            let b = face.indices[(i + 1) % n];
            let edge = if a < b { (a, b) } else { (b, a) };
            if !edges.contains(&edge) {
                edges.push(edge);
            }
        }
    }
    edges
}

/// Simple face normal.
pub fn face_normal(mesh: &EditableMesh, face: &Face) -> bevy::math::Vec3 {
    if face.indices.len() < 3 {
        return bevy::math::Vec3::ZERO;
    }
    let v0 = mesh.vertices[face.indices[0] as usize];
    let v1 = mesh.vertices[face.indices[1] as usize];
    let v2 = mesh.vertices[face.indices[2] as usize];
    (v1 - v0).cross(v2 - v0).normalize_or_zero()
}

/// Build adjacency: for each edge, the list of face indices that use it.
pub fn edge_to_faces(mesh: &EditableMesh) -> HashMap<(u32, u32), Vec<u32>> {
    let mut map: HashMap<(u32, u32), Vec<u32>> = HashMap::new();
    for (fi, face) in mesh.faces.iter().enumerate() {
        let n = face.indices.len();
        for i in 0..n {
            let a = face.indices[i];
            let b = face.indices[(i + 1) % n];
            let edge = if a < b { (a, b) } else { (b, a) };
            map.entry(edge).or_default().push(fi as u32);
        }
    }
    map
}

/// For each face, the list of neighbouring face indices (share at least one edge).
pub fn face_adjacency(mesh: &EditableMesh) -> Vec<Vec<u32>> {
    let e2f = edge_to_faces(mesh);
    let mut adj: Vec<HashSet<u32>> = (0..mesh.faces.len()).map(|_| HashSet::new()).collect();

    for faces in e2f.values() {
        for i in 0..faces.len() {
            for j in (i + 1)..faces.len() {
                let a = faces[i] as usize;
                let b = faces[j] as usize;
                adj[a].insert(faces[j]);
                adj[b].insert(faces[i]);
            }
        }
    }

    adj.into_iter()
        .map(|s| s.into_iter().collect())
        .collect()
}

/// Flood-fill all faces connected to `seed` through shared edges.
/// Returns the set of face indices in the connected region (a "cell island").
pub fn select_linked_faces(mesh: &EditableMesh, seed: u32) -> HashSet<u32> {
    let adj = face_adjacency(mesh);
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();

    if (seed as usize) >= mesh.faces.len() {
        return visited;
    }

    queue.push_back(seed);
    visited.insert(seed);

    while let Some(f) = queue.pop_front() {
        if let Some(neighbours) = adj.get(f as usize) {
            for &n in neighbours {
                if visited.insert(n) {
                    queue.push_back(n);
                }
            }
        }
    }

    visited
}

/// Expand an existing face selection to all linked faces (union of regions).
pub fn expand_linked_faces(mesh: &EditableMesh, seeds: &HashSet<u32>) -> HashSet<u32> {
    let mut result = HashSet::new();
    for &seed in seeds {
        result.extend(select_linked_faces(mesh, seed));
    }
    result
}

/// Faces that share an edge with any of the selected faces (one-ring grow).
pub fn grow_face_selection(mesh: &EditableMesh, selected: &HashSet<u32>) -> HashSet<u32> {
    let adj = face_adjacency(mesh);
    let mut grown = selected.clone();
    for &f in selected {
        if let Some(neighbours) = adj.get(f as usize) {
            for &n in neighbours {
                grown.insert(n);
            }
        }
    }
    grown
}

/// Shrink face selection by removing boundary faces (one-ring shrink).
pub fn shrink_face_selection(mesh: &EditableMesh, selected: &HashSet<u32>) -> HashSet<u32> {
    let adj = face_adjacency(mesh);
    selected
        .iter()
        .copied()
        .filter(|&f| {
            // Keep face only if all its neighbours are also selected
            if let Some(neighbours) = adj.get(f as usize) {
                !neighbours.is_empty() && neighbours.iter().all(|n| selected.contains(n))
            } else {
                false
            }
        })
        .collect()
}

/// Select edge ring: edges "parallel" across quads from a seed edge.
pub fn select_edge_ring(mesh: &EditableMesh, a: u32, b: u32) -> HashSet<(u32, u32)> {
    let start = if a < b { (a, b) } else { (b, a) };
    let mut result = HashSet::new();
    let e2f = edge_to_faces(mesh);
    let mut visited = HashSet::new();
    let mut queue = vec![start];
    visited.insert(start);

    while let Some(edge) = queue.pop() {
        result.insert(edge);
        if let Some(faces) = e2f.get(&edge) {
            for &fi in faces {
                let face = &mesh.faces[fi as usize];
                if face.indices.len() != 4 {
                    continue;
                }
                // Adjacent edges sharing a vertex but not opposite — ring steps to left/right
                // For ring: take edges that share a vertex with seed and are not the opposite edge
                if let Some(side) = ring_next_edges(face, edge) {
                    for e in side {
                        if visited.insert(e) {
                            queue.push(e);
                        }
                    }
                }
            }
        }
    }
    result
}

fn ring_next_edges(face: &Face, edge: (u32, u32)) -> Option<[(u32, u32); 2]> {
    let v = &face.indices;
    if v.len() != 4 {
        return None;
    }
    for i in 0..4 {
        let a = v[i];
        let b = v[(i + 1) % 4];
        let e = if a < b { (a, b) } else { (b, a) };
        if e == edge {
            // Left and right edges (not opposite)
            let left = {
                let x = v[(i + 3) % 4];
                let y = a;
                if x < y { (x, y) } else { (y, x) }
            };
            let right = {
                let x = b;
                let y = v[(i + 2) % 4];
                if x < y { (x, y) } else { (y, x) }
            };
            return Some([left, right]);
        }
    }
    None
}

/// Select faces with similar normal to seed (dot > threshold).
pub fn select_similar_faces(mesh: &EditableMesh, seed: u32, threshold: f32) -> HashSet<u32> {
    let mut result = HashSet::new();
    if seed as usize >= mesh.faces.len() {
        return result;
    }
    let n0 = face_normal(mesh, &mesh.faces[seed as usize]);
    for (i, face) in mesh.faces.iter().enumerate() {
        let n = face_normal(mesh, face);
        if n.dot(n0) >= threshold {
            result.insert(i as u32);
        }
    }
    result
}
