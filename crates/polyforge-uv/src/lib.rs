//! # polyforge-uv
//! UV mapping, unwrapping, and island tools.

use bevy::prelude::*;
use polyforge_core::EditableMesh;

pub mod islands;
pub mod follow_quads;
pub use islands::*;
pub use follow_quads::*;

pub struct PolyForgeUvPlugin;

impl Plugin for PolyForgeUvPlugin {
    fn build(&self, _app: &mut App) {}
}

/// Simple planar projection onto XY plane.
pub fn project_planar_xy(mesh: &mut EditableMesh) {
    for (i, v) in mesh.vertices.iter().enumerate() {
        let uv = Vec2::new(v.x + 0.5, v.y + 0.5);
        if i < mesh.uvs.len() {
            mesh.uvs[i] = uv;
        } else {
            mesh.uvs.push(uv);
        }
    }
}

/// Simple box mapping.
pub fn project_box(mesh: &mut EditableMesh) {
    for (i, v) in mesh.vertices.iter().enumerate() {
        let abs = v.abs();
        let uv = if abs.x >= abs.y && abs.x >= abs.z {
            Vec2::new(v.z, v.y)
        } else if abs.y >= abs.x && abs.y >= abs.z {
            Vec2::new(v.x, v.z)
        } else {
            Vec2::new(v.x, v.y)
        };
        let uv = uv * 0.5 + Vec2::splat(0.5);
        if i < mesh.uvs.len() {
            mesh.uvs[i] = uv;
        } else {
            mesh.uvs.push(uv);
        }
    }
}

/// Angle-based unwrap approximation using box projection + seam awareness.
pub fn unwrap_smart(mesh: &mut EditableMesh) {
    if mesh.vertices.is_empty() {
        return;
    }
    project_box(mesh);
    pack_uv_islands(mesh, 0.02);
}
