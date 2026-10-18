//! Bridge between PolyForge's editable mesh and the slicer's triangle soup.
//!
//! Bevy (and PolyForge) use Y-up; slicers expect Z-up with the model sitting
//! on the build plate. The conversion also fans n-gons into triangles.

use crate::types::{MeshTriangles, Triangle, Vec3};
use bevy::prelude::Vec3 as BevyVec3;
use polyforge_core::EditableMesh;

/// Convert a Bevy Y-up position to slicer Z-up space (millimetres).
pub fn to_print_space(v: BevyVec3) -> Vec3 {
    Vec3::new(v.x as f64, -(v.z as f64), v.y as f64)
}

/// Triangulate an `EditableMesh` into slicer-space triangles.
pub fn mesh_to_triangles(mesh: &EditableMesh) -> MeshTriangles {
    let mut tris = Vec::new();
    for face in &mesh.faces {
        if face.indices.len() < 3 {
            continue;
        }
        let i0 = face.indices[0] as usize;
        let Some(a) = mesh.vertices.get(i0).copied() else {
            continue;
        };
        let a = to_print_space(a);

        for w in face.indices[1..].windows(2) {
            let (i1, i2) = (w[0] as usize, w[1] as usize);
            let (Some(b), Some(c)) = (
                mesh.vertices.get(i1).copied(),
                mesh.vertices.get(i2).copied(),
            ) else {
                continue;
            };
            tris.push(Triangle::new(a, to_print_space(b), to_print_space(c)));
        }
    }
    tris
}

#[cfg(test)]
mod tests {
    use super::*;
    use polyforge_core::EditableMesh;

    #[test]
    fn cube_converts_to_12_triangles() {
        let mesh = EditableMesh::cube();
        let tris = mesh_to_triangles(&mesh);
        assert_eq!(tris.len(), 12);
    }

    #[test]
    fn y_up_maps_to_z_up() {
        let up = to_print_space(BevyVec3::new(0.0, 5.0, 0.0));
        assert!((up.z - 5.0).abs() < 1e-12);
        assert!(up.x.abs() < 1e-12 && up.y.abs() < 1e-12);
    }
}
