//! Spin: rotate selection around an axis, creating intermediate geometry.

use bevy::prelude::*;
use polyforge_core::{EditableMesh, Selection, Face};
use crate::{OpResult, OpError};
use std::collections::HashMap;
use std::f32::consts::TAU;

/// Spin selected vertices around axis through `origin` in direction `axis`,
/// creating `steps` copies and bridging quads between them.
pub fn spin_selection(
    mesh: &mut EditableMesh,
    selection: &Selection,
    origin: Vec3,
    axis: Vec3,
    angle: f32,
    steps: u32,
) -> OpResult {
    let axis = axis.normalize_or_zero();
    if axis.length_squared() < 1e-8 || steps == 0 {
        return Err(OpError::InvalidTopology);
    }
    let verts: Vec<u32> = selection.effective_vertices(&mesh.faces).into_iter().collect();
    if verts.is_empty() {
        return Err(OpError::EmptySelection);
    }

    // rings[step][i] = vertex index for verts[i] at that step
    let mut rings: Vec<Vec<u32>> = Vec::new();
    // step 0 = originals
    rings.push(verts.clone());

    for s in 1..=steps {
        let t = s as f32 / steps as f32;
        let a = angle * t;
        let mut ring = Vec::with_capacity(verts.len());
        for &vi in &verts {
            let p = mesh.vertices[vi as usize];
            let rotated = rotate_around(p, origin, axis, a);
            let ni = mesh.vertices.len() as u32;
            mesh.vertices.push(rotated);
            mesh.normals
                .push(mesh.normals.get(vi as usize).copied().unwrap_or(Vec3::Y));
            mesh.uvs
                .push(mesh.uvs.get(vi as usize).copied().unwrap_or(Vec2::ZERO));
            ring.push(ni);
        }
        rings.push(ring);
    }

    // Bridge consecutive rings along selection edges
    let edges = polyforge_core::extract_edges(mesh);
    let vert_set: std::collections::HashSet<u32> = verts.iter().copied().collect();
    let mut index_of: HashMap<u32, usize> = HashMap::new();
    for (i, &v) in verts.iter().enumerate() {
        index_of.insert(v, i);
    }
    for &(a, b) in &edges {
        if !(vert_set.contains(&a) && vert_set.contains(&b)) {
            continue;
        }
        let ia = index_of[&a];
        let ib = index_of[&b];
        for s in 0..steps as usize {
            let a0 = rings[s][ia];
            let b0 = rings[s][ib];
            let a1 = rings[s + 1][ia];
            let b1 = rings[s + 1][ib];
            mesh.faces.push(Face::quad(a0, b0, b1, a1));
        }
    }
    mesh.recalculate_normals();
    let _ = TAU;
    Ok(())
}

fn rotate_around(p: Vec3, origin: Vec3, axis: Vec3, angle: f32) -> Vec3 {
    let v = p - origin;
    let c = angle.cos();
    let s = angle.sin();
    // Rodrigues
    v * c + axis.cross(v) * s + axis * axis.dot(v) * (1.0 - c) + origin
}
