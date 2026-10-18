//! Rip and fill the resulting gap with a face strip.

use bevy::prelude::*;
use polyforge_core::{EditableMesh, Selection, Face};
use crate::{OpResult, OpError, rip_vertices};
use std::collections::{HashMap, HashSet};

/// Rip selected vertices/edges then fill the opening with quads/triangles.
pub fn rip_fill(mesh: &mut EditableMesh, selection: &Selection, offset: Vec3) -> OpResult {
    if selection.vertices.is_empty() && selection.edges.is_empty() {
        return Err(OpError::EmptySelection);
    }

    let mut verts: HashSet<u32> = selection.vertices.clone();
    for &(a, b) in &selection.edges {
        verts.insert(a);
        verts.insert(b);
    }
    let original: Vec<u32> = verts.iter().copied().collect();

    // Record faces using these verts before rip
    let faces_before = mesh.faces.clone();
    rip_vertices(mesh, selection, offset)?;

    // After rip, map original -> new (last |original| verts added)
    // rip_vertices appends copies in iteration order of the HashSet — unstable.
    // Re-run mapping: for each original, find nearest new vertex in offset direction.
    let mut map: HashMap<u32, u32> = HashMap::new();
    for &oi in &original {
        let expected = mesh.vertices[oi as usize] + offset;
        // Original was not moved in current rip impl — copies are new indices
        // Find vertex near expected that isn't oi
        let mut best = None;
        let mut best_d = f32::MAX;
        for (i, v) in mesh.vertices.iter().enumerate() {
            if i as u32 == oi {
                continue;
            }
            let d = v.distance(expected);
            if d < best_d {
                best_d = d;
                best = Some(i as u32);
            }
        }
        if let Some(ni) = best {
            if best_d < offset.length() * 2.0 + 0.1 {
                map.insert(oi, ni);
            }
        }
    }

    // Fill: for each original edge between two ripped verts, create a quad
    // (a, b, map[b], map[a])
    let edges = polyforge_core::extract_edges(mesh);
    for &(a, b) in &edges {
        if verts.contains(&a) && verts.contains(&b) {
            if let (Some(&a2), Some(&b2)) = (map.get(&a), map.get(&b)) {
                mesh.faces.push(Face::quad(a, b, b2, a2));
            }
        }
    }
    let _ = faces_before;
    mesh.recalculate_normals();
    Ok(())
}
