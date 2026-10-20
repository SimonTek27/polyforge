//! Solidify: give thickness to a surface mesh.

use bevy::prelude::*;
use polyforge_core::{EditableMesh, Selection, Face, topology::face_normal};
use crate::{OpResult, OpError};
use std::collections::HashMap;

/// Extrude all (or selected) faces inward/outward by `thickness`, creating side walls.
pub fn solidify(mesh: &mut EditableMesh, selection: &Selection, thickness: f32) -> OpResult {
    if mesh.faces.is_empty() {
        return Err(OpError::EmptySelection);
    }

    let face_indices: Vec<usize> = if selection.faces.is_empty() {
        (0..mesh.faces.len()).collect()
    } else {
        selection.faces.iter().map(|&i| i as usize).collect()
    };

    // Per-vertex offset = average of adjacent face normals * thickness
    let mut accum: HashMap<u32, (Vec3, u32)> = HashMap::new();
    for &fi in &face_indices {
        let face = &mesh.faces[fi];
        let n = face_normal(mesh, face);
        for &vi in &face.indices {
            let e = accum.entry(vi).or_insert((Vec3::ZERO, 0));
            e.0 += n;
            e.1 += 1;
        }
    }

    let mut vert_map: HashMap<u32, u32> = HashMap::new();
    for (vi, (sum, count)) in &accum {
        let n = (*sum / *count as f32).normalize_or_zero();
        let pos = mesh.vertices[*vi as usize] + n * thickness;
        let ni = mesh.vertices.len() as u32;
        mesh.vertices.push(pos);
        mesh.normals.push(-n);
        mesh.uvs
            .push(mesh.uvs.get(*vi as usize).copied().unwrap_or(Vec2::ZERO));
        vert_map.insert(*vi, ni);
    }

    // Original faces become outer; add inner reversed + sides
    let mut new_faces = Vec::new();
    for (fi, face) in mesh.faces.iter().enumerate() {
        if !face_indices.contains(&fi) {
            new_faces.push(face.clone());
            continue;
        }
        // outer (keep)
        new_faces.push(face.clone());
        // inner (reversed, mapped)
        if face.indices.iter().all(|i| vert_map.contains_key(i)) {
            let mut inner: Vec<u32> = face.indices.iter().map(|i| vert_map[i]).collect();
            inner.reverse();
            new_faces.push(Face { indices: inner });
            // sides
            let n = face.indices.len();
            for i in 0..n {
                let a = face.indices[i];
                let b = face.indices[(i + 1) % n];
                let a2 = vert_map[&a];
                let b2 = vert_map[&b];
                new_faces.push(Face::quad(a, b, b2, a2));
            }
        }
    }
    mesh.faces = new_faces;
    mesh.recalculate_normals();
    Ok(())
}
