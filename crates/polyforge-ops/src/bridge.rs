//! Bridge two edge loops into a quad strip; extrude edges.

use bevy::prelude::*;
use polyforge_core::{EditableMesh, Selection, Face};
use crate::{OpResult, OpError};
use std::collections::{HashMap, HashSet};

/// Bridge two equal-length edge loops in the edge selection.
pub fn bridge_edge_loops(mesh: &mut EditableMesh, selection: &Selection) -> OpResult {
    let loops = find_edge_loops(&selection.edges);
    if loops.len() < 2 {
        return Err(OpError::InvalidTopology);
    }
    let mut loops = loops;
    loops.sort_by_key(|l| std::cmp::Reverse(l.len()));
    let a = loops[0].clone();
    let b = loops
        .iter()
        .skip(1)
        .find(|l| l.len() == a.len())
        .cloned()
        .ok_or(OpError::InvalidTopology)?;

    if a.len() < 3 {
        return Err(OpError::InvalidTopology);
    }

    let mut best_offset = 0usize;
    let mut best_dist = f32::MAX;
    for off in 0..b.len() {
        let d = mesh.vertices[a[0] as usize].distance_squared(mesh.vertices[b[off] as usize]);
        if d < best_dist {
            best_dist = d;
            best_offset = off;
        }
    }

    let center_a: Vec3 =
        a.iter().map(|&i| mesh.vertices[i as usize]).sum::<Vec3>() / a.len() as f32;
    let center_b: Vec3 =
        b.iter().map(|&i| mesh.vertices[i as usize]).sum::<Vec3>() / b.len() as f32;
    let dir = (center_b - center_a).normalize_or_zero();

    let mut b_ord: Vec<u32> = (0..b.len()).map(|i| b[(i + best_offset) % b.len()]).collect();
    let e0 = mesh.vertices[a[1] as usize] - mesh.vertices[a[0] as usize];
    let e1 = mesh.vertices[b_ord[0] as usize] - mesh.vertices[a[0] as usize];
    if e0.cross(e1).dot(dir) < 0.0 {
        b_ord.reverse();
        if let Some((i, _)) = b_ord.iter().enumerate().min_by(|(_, &x), (_, &y)| {
            mesh.vertices[x as usize]
                .distance_squared(mesh.vertices[a[0] as usize])
                .partial_cmp(
                    &mesh.vertices[y as usize].distance_squared(mesh.vertices[a[0] as usize]),
                )
                .unwrap_or(std::cmp::Ordering::Equal)
        }) {
            b_ord.rotate_left(i);
        }
    }

    let n = a.len();
    for i in 0..n {
        let a0 = a[i];
        let a1 = a[(i + 1) % n];
        let b0 = b_ord[i];
        let b1 = b_ord[(i + 1) % n];
        mesh.faces.push(Face::quad(a0, a1, b1, b0));
    }
    mesh.recalculate_normals();
    Ok(())
}

fn find_edge_loops(edges: &HashSet<(u32, u32)>) -> Vec<Vec<u32>> {
    let mut adj: HashMap<u32, Vec<u32>> = HashMap::new();
    for &(a, b) in edges {
        adj.entry(a).or_default().push(b);
        adj.entry(b).or_default().push(a);
    }
    let mut visited_edges: HashSet<(u32, u32)> = HashSet::new();
    let mut loops = Vec::new();

    for &start in adj.keys() {
        let neighbors = adj.get(&start).cloned().unwrap_or_default();
        for &n0 in &neighbors {
            let e0 = if start < n0 { (start, n0) } else { (n0, start) };
            if visited_edges.contains(&e0) {
                continue;
            }
            let mut loop_verts = vec![start];
            let mut prev = start;
            let mut cur = n0;
            visited_edges.insert(e0);
            let mut closed = false;
            for _ in 0..edges.len() + 2 {
                loop_verts.push(cur);
                if cur == start {
                    closed = true;
                    break;
                }
                let nexts = adj.get(&cur).cloned().unwrap_or_default();
                let mut advanced = false;
                for &n in &nexts {
                    if n == prev {
                        continue;
                    }
                    let e = if cur < n { (cur, n) } else { (n, cur) };
                    if visited_edges.contains(&e) {
                        continue;
                    }
                    visited_edges.insert(e);
                    prev = cur;
                    cur = n;
                    advanced = true;
                    break;
                }
                if !advanced {
                    break;
                }
            }
            if closed && loop_verts.len() > 3 {
                loop_verts.pop();
                loops.push(loop_verts);
            }
        }
    }
    loops
}

/// Extrude selected edges along +Y by `distance`, creating side quads.
pub fn extrude_edges(mesh: &mut EditableMesh, selection: &Selection, distance: f32) -> OpResult {
    if selection.edges.is_empty() {
        return Err(OpError::EmptySelection);
    }
    let mut map: HashMap<u32, u32> = HashMap::new();
    let verts: HashSet<u32> = selection.edges.iter().flat_map(|&(a, b)| [a, b]).collect();
    for &vi in &verts {
        let pos = mesh.vertices[vi as usize] + Vec3::Y * distance;
        let ni = mesh.vertices.len() as u32;
        mesh.vertices.push(pos);
        mesh.normals.push(Vec3::Y);
        mesh.uvs
            .push(mesh.uvs.get(vi as usize).copied().unwrap_or(Vec2::ZERO));
        map.insert(vi, ni);
    }
    for &(a, b) in &selection.edges {
        mesh.faces.push(Face::quad(a, b, map[&b], map[&a]));
    }
    mesh.recalculate_normals();
    Ok(())
}
