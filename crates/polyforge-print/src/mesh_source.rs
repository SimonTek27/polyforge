//! Bridge between PolyForge's editable mesh and the slicer's triangle soup.
//!
//! Bevy (and PolyForge) use Y-up; slicers expect Z-up with the model sitting
//! on the build plate. The conversion also fans n-gons into triangles.

use crate::profile::PrinterProfile;
use crate::types::{BoundingBox, MeshTriangles, Triangle, Vec3};
use bevy::prelude::Vec3 as BevyVec3;
use polyforge_core::EditableMesh;

/// Convert a Bevy Y-up position to slicer Z-up space (millimetres).
pub fn to_print_space(v: BevyVec3) -> Vec3 {
    Vec3::new(v.x as f64, -(v.z as f64), v.y as f64)
}

/// Triangulate an `EditableMesh` into slicer-space triangles.
pub fn mesh_to_triangles(mesh: &EditableMesh) -> MeshTriangles {
    mesh_to_triangles_scaled(mesh, 1.0)
}

/// Triangulate an `EditableMesh` into slicer space, multiplying every
/// coordinate by `scale` (scene units -> millimetres).
pub fn mesh_to_triangles_scaled(mesh: &EditableMesh, scale: f32) -> MeshTriangles {
    let mut tris = Vec::new();
    for face in &mesh.faces {
        if face.indices.len() < 3 {
            continue;
        }
        let i0 = face.indices[0] as usize;
        let Some(a) = mesh.vertices.get(i0).copied() else {
            continue;
        };
        let a = to_print_space(a * scale);

        for w in face.indices[1..].windows(2) {
            let (i1, i2) = (w[0] as usize, w[1] as usize);
            let (Some(b), Some(c)) = (
                mesh.vertices.get(i1).copied(),
                mesh.vertices.get(i2).copied(),
            ) else {
                continue;
            };
            tris.push(Triangle::new(
                a,
                to_print_space(b * scale),
                to_print_space(c * scale),
            ));
        }
    }
    tris
}

/// Triangulate and place the model on the printer bed: centred in X/Y and
/// resting with its lowest point at Z = 0.
pub fn mesh_to_bed_triangles(
    mesh: &EditableMesh,
    scale: f32,
    printer: &PrinterProfile,
) -> MeshTriangles {
    let mut tris = mesh_to_triangles_scaled(mesh, scale);
    if tris.is_empty() {
        return tris;
    }

    let mut bb = BoundingBox::empty();
    for tri in &tris {
        for p in tri.v {
            bb.expand_point(p);
        }
    }

    let center = bb.center();
    let dx = printer.build_volume_x * 0.5 - center.x;
    let dy = printer.build_volume_y * 0.5 - center.y;
    let dz = -bb.min.z;

    for tri in &mut tris {
        for p in &mut tri.v {
            p.x += dx;
            p.y += dy;
            p.z += dz;
        }
        tri.normal = (tri.v[1] - tri.v[0])
            .cross(tri.v[2] - tri.v[0])
            .normalize_or_zero();
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

    #[test]
    fn bed_placement_centres_and_drops_to_zero() {
        let mesh = EditableMesh::cube();
        let printer = PrinterProfile::create_builtin("generic_marlin").unwrap();
        let tris = mesh_to_bed_triangles(&mesh, 100.0, &printer);

        let mut bb = BoundingBox::empty();
        for tri in &tris {
            for p in tri.v {
                bb.expand_point(p);
            }
        }

        assert!((bb.min.z).abs() < 1e-9);
        assert!((bb.center().x - printer.build_volume_x * 0.5).abs() < 1e-9);
        assert!((bb.center().y - printer.build_volume_y * 0.5).abs() < 1e-9);
        assert!((bb.size().z - 100.0).abs() < 1e-6);
    }

    #[test]
    fn editable_mesh_slices_and_exports() {
        use crate::gcode::generate_gcode;
        use crate::slicer::SlicerEngine;

        let mesh = EditableMesh::cube();
        let printer = PrinterProfile::create_builtin("prusa_mk4").unwrap();
        let settings = printer.default_settings.clone();

        let tris = mesh_to_bed_triangles(&mesh, 100.0, &printer);
        let slice = SlicerEngine::new().slice(&tris, &settings, &printer);
        assert!(slice.success, "slice errors: {:?}", slice.errors);
        assert!(slice.total_layers > 100);
        assert!(slice.filament_used > 0.0);

        let gcode = generate_gcode(&slice, &printer, &settings);
        assert!(gcode.contains("G28") || gcode.contains("G21"));
        assert!(gcode.contains("M82") || gcode.contains("M83"));
    }
}
