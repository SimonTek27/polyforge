//! Basic transform operations on selected vertices.

use bevy::prelude::*;
use polyforge_core::{EditableMesh, Selection};
use crate::{OpResult, OpError};

/// Translate selected vertices by `delta`.
pub fn translate_vertices(
    mesh: &mut EditableMesh,
    selection: &Selection,
    delta: Vec3,
) -> OpResult {
    if selection.vertices.is_empty() {
        for v in &mut mesh.vertices {
            *v += delta;
        }
    } else {
        for &idx in &selection.vertices {
            if let Some(v) = mesh.vertices.get_mut(idx as usize) {
                *v += delta;
            }
        }
    }
    mesh.recalculate_normals();
    Ok(())
}

/// Translate with optional grid snap (applied to final positions).
pub fn translate_vertices_snapped(
    mesh: &mut EditableMesh,
    selection: &Selection,
    delta: Vec3,
    snap: Option<f32>,
) -> OpResult {
    let targets: Vec<u32> = if selection.vertices.is_empty() {
        (0..mesh.vertices.len() as u32).collect()
    } else {
        selection.vertices.iter().copied().collect()
    };

    for idx in targets {
        if let Some(v) = mesh.vertices.get_mut(idx as usize) {
            *v += delta;
            if let Some(s) = snap {
                if s > 1e-8 {
                    v.x = (v.x / s).round() * s;
                    v.y = (v.y / s).round() * s;
                    v.z = (v.z / s).round() * s;
                }
            }
        }
    }
    mesh.recalculate_normals();
    Ok(())
}

/// Set absolute positions for a list of (index, position) pairs.
pub fn set_vertex_positions(
    mesh: &mut EditableMesh,
    positions: &[(u32, Vec3)],
) -> OpResult {
    if positions.is_empty() {
        return Err(OpError::EmptySelection);
    }
    for &(idx, pos) in positions {
        if let Some(v) = mesh.vertices.get_mut(idx as usize) {
            *v = pos;
        }
    }
    mesh.recalculate_normals();
    Ok(())
}

/// Scale selected vertices around a pivot.
pub fn scale_vertices(
    mesh: &mut EditableMesh,
    selection: &Selection,
    scale: Vec3,
    pivot: Vec3,
) -> OpResult {
    let targets: Vec<u32> = if selection.vertices.is_empty() {
        (0..mesh.vertices.len() as u32).collect()
    } else {
        selection.vertices.iter().copied().collect()
    };

    if targets.is_empty() {
        return Err(OpError::EmptySelection);
    }

    for idx in targets {
        if let Some(v) = mesh.vertices.get_mut(idx as usize) {
            *v = pivot + (*v - pivot) * scale;
        }
    }
    mesh.recalculate_normals();
    Ok(())
}

/// Rotate selected vertices around a pivot by a quaternion.
pub fn rotate_vertices(
    mesh: &mut EditableMesh,
    selection: &Selection,
    rotation: Quat,
    pivot: Vec3,
) -> OpResult {
    let targets: Vec<u32> = if selection.vertices.is_empty() {
        (0..mesh.vertices.len() as u32).collect()
    } else {
        selection.vertices.iter().copied().collect()
    };

    if targets.is_empty() {
        return Err(OpError::EmptySelection);
    }

    for idx in targets {
        if let Some(v) = mesh.vertices.get_mut(idx as usize) {
            *v = pivot + rotation * (*v - pivot);
        }
    }
    mesh.recalculate_normals();
    Ok(())
}

/// Translate with proportional falloff (smooth sphere).
pub fn translate_proportional(
    mesh: &mut EditableMesh,
    center_verts: &std::collections::HashSet<u32>,
    delta: Vec3,
    radius: f32,
) {
    if radius <= 1e-6 || center_verts.is_empty() {
        for &idx in center_verts {
            if let Some(v) = mesh.vertices.get_mut(idx as usize) {
                *v += delta;
            }
        }
        return;
    }
    let mut pivot = Vec3::ZERO;
    for &idx in center_verts {
        pivot += mesh.vertices[idx as usize];
    }
    pivot /= center_verts.len() as f32;

    for (i, v) in mesh.vertices.iter_mut().enumerate() {
        let d = v.distance(pivot);
        if d > radius {
            continue;
        }
        let w = if center_verts.contains(&(i as u32)) {
            1.0
        } else {
            // smoothstep falloff
            let t = 1.0 - (d / radius);
            t * t * (3.0 - 2.0 * t)
        };
        *v += delta * w;
    }
}

/// Snap each selected vertex to the nearest non-selected vertex (within threshold).
pub fn snap_vertices_to_nearest(mesh: &mut EditableMesh, selection: &Selection, threshold: f32) {
    let selected = selection.effective_vertices(&mesh.faces);
    let others: Vec<(u32, Vec3)> = mesh
        .vertices
        .iter()
        .enumerate()
        .filter(|(i, _)| !selected.contains(&(*i as u32)))
        .map(|(i, v)| (i as u32, *v))
        .collect();
    for &si in &selected {
        let p = mesh.vertices[si as usize];
        let mut best = None;
        let mut best_d = threshold;
        for &(_, op) in &others {
            let d = p.distance(op);
            if d < best_d {
                best_d = d;
                best = Some(op);
            }
        }
        if let Some(target) = best {
            mesh.vertices[si as usize] = target;
        }
    }
}

/// Scale each selected vertex away from its own position... no-op identity.
/// Scale around each vertex's connected face centers average (individual origins).
pub fn scale_individual_origins(
    mesh: &mut EditableMesh,
    selection: &Selection,
    factor: f32,
) {
    let verts = selection.effective_vertices(&mesh.faces);
    // For individual origins: scale relative to each vertex's immediate neighbor centroid
    let edges = polyforge_core::extract_edges(mesh);
    let mut adj: std::collections::HashMap<u32, Vec<u32>> = std::collections::HashMap::new();
    for &(a, b) in &edges {
        adj.entry(a).or_default().push(b);
        adj.entry(b).or_default().push(a);
    }
    let mut deltas = Vec::new();
    for &vi in &verts {
        let p = mesh.vertices[vi as usize];
        let ns = adj.get(&vi).cloned().unwrap_or_default();
        if ns.is_empty() {
            continue;
        }
        let mut c = Vec3::ZERO;
        for &n in &ns {
            c += mesh.vertices[n as usize];
        }
        c /= ns.len() as f32;
        let new_p = c + (p - c) * factor;
        deltas.push((vi, new_p));
    }
    for (vi, p) in deltas {
        mesh.vertices[vi as usize] = p;
    }
}
