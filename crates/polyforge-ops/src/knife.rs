//! Knife: cut edges that intersect a cutting plane defined by two points + view normal.

use bevy::prelude::*;
use polyforge_core::{EditableMesh, Selection, Face, extract_edges};
use crate::{OpResult, OpError};
use std::collections::HashMap;

/// Cut all edges that cross the plane through `p0`, `p1` with normal `plane_n`.
/// Inserts vertices at intersections and splits faces.
pub fn knife_plane_cut(
    mesh: &mut EditableMesh,
    p0: Vec3,
    p1: Vec3,
    plane_n: Vec3,
) -> OpResult {
    let plane_n = plane_n.normalize_or_zero();
    if plane_n.length_squared() < 1e-8 {
        return Err(OpError::InvalidTopology);
    }
    // Plane also constrained roughly along p0-p1
    let cut_dir = (p1 - p0).normalize_or_zero();
    let edges = extract_edges(mesh);
    let mut edge_cuts: HashMap<(u32, u32), u32> = HashMap::new();

    for &(a, b) in &edges {
        let va = mesh.vertices[a as usize];
        let vb = mesh.vertices[b as usize];
        let da = (va - p0).dot(plane_n);
        let db = (vb - p0).dot(plane_n);
        if da * db >= 0.0 {
            continue; // same side
        }
        let t = da / (da - db);
        if !(0.0..=1.0).contains(&t) {
            continue;
        }
        let hit = va + (vb - va) * t;
        // Optional: restrict to segment near the cut line
        let _ = cut_dir;
        let ni = mesh.vertices.len() as u32;
        mesh.vertices.push(hit);
        mesh.normals.push(Vec3::Y);
        mesh.uvs.push(Vec2::ZERO);
        let key = if a < b { (a, b) } else { (b, a) };
        edge_cuts.insert(key, ni);
    }

    if edge_cuts.is_empty() {
        return Err(OpError::InvalidTopology);
    }

    // Split faces that contain cut edges
    let mut new_faces = Vec::new();
    for face in &mesh.faces {
        let n = face.indices.len();
        let mut split_pts = Vec::new();
        for i in 0..n {
            let a = face.indices[i];
            let b = face.indices[(i + 1) % n];
            let key = if a < b { (a, b) } else { (b, a) };
            if let Some(&c) = edge_cuts.get(&key) {
                split_pts.push((i, c));
            }
        }
        if split_pts.len() == 2 {
            // Split into two faces along the two cut points
            let (i0, c0) = split_pts[0];
            let (i1, c1) = split_pts[1];
            // Face A: from c0 along to c1
            let mut fa = vec![c0];
            let mut k = (i0 + 1) % n;
            while k != (i1 + 1) % n {
                fa.push(face.indices[k]);
                if k == i1 {
                    break;
                }
                k = (k + 1) % n;
            }
            fa.push(c1);
            // Face B
            let mut fb = vec![c1];
            k = (i1 + 1) % n;
            while k != (i0 + 1) % n {
                fb.push(face.indices[k]);
                if k == i0 {
                    break;
                }
                k = (k + 1) % n;
            }
            fb.push(c0);
            if fa.len() >= 3 {
                new_faces.push(Face { indices: fa });
            }
            if fb.len() >= 3 {
                new_faces.push(Face { indices: fb });
            }
        } else {
            // Insert cut verts into boundary
            let mut indices = Vec::new();
            for i in 0..n {
                let a = face.indices[i];
                indices.push(a);
                let b = face.indices[(i + 1) % n];
                let key = if a < b { (a, b) } else { (b, a) };
                if let Some(&c) = edge_cuts.get(&key) {
                    indices.push(c);
                }
            }
            new_faces.push(Face { indices });
        }
    }
    mesh.faces = new_faces;
    mesh.recalculate_normals();
    Ok(())
}

/// Convenience: knife cut on selection AABB mid plane along X.
pub fn knife_cut_x(mesh: &mut EditableMesh, selection: &Selection) -> OpResult {
    let verts = selection.effective_vertices(&mesh.faces);
    let pts: Vec<Vec3> = if verts.is_empty() {
        mesh.vertices.clone()
    } else {
        verts.iter().map(|&i| mesh.vertices[i as usize]).collect()
    };
    if pts.is_empty() {
        return Err(OpError::EmptySelection);
    }
    let mut min = pts[0];
    let mut max = pts[0];
    for p in &pts {
        min = min.min(*p);
        max = max.max(*p);
    }
    let mid = (min + max) * 0.5;
    let p0 = Vec3::new(mid.x, min.y, min.z);
    let p1 = Vec3::new(mid.x, max.y, max.z);
    knife_plane_cut(mesh, p0, p1, Vec3::X)
}
