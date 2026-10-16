//! Basic transform operations on selected vertices.

use bevy::prelude::*;
use polyforge_core::{EditableMesh, Selection};
use crate::{OpResult, OpError};

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
