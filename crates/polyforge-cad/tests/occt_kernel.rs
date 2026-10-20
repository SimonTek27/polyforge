//! Integration tests for the OCCT-backed kernel (`--features cad_kernel`).
//!
//! They compile only with the feature and skip at runtime when OCCT cannot be
//! initialised (no native library / missing DLLs), so they stay usable in
//! default builds and non-Windows sandboxes.

#![cfg(feature = "cad_kernel")]

use polyforge_cad::{CadFeature, CadFeatureId, SketchCurve, SketchPlane};
use polyforge_core::EditableMesh;

fn occt() -> bool {
    polyforge_occt_sys::init()
}

fn box_feature(id: u64) -> CadFeature {
    CadFeature::Box {
        id: CadFeatureId(id),
        width: 1.0,
        height: 1.0,
        depth: 1.0,
    }
}

fn tessellated(features: &[CadFeature]) -> Option<EditableMesh> {
    let shape = polyforge_cad::evaluate_shape(features)?;
    let (vertices, faces) = polyforge_occt_sys::triangulate(&shape, 0.05)?;
    Some(polyforge_cad::mesh_from_triangles(vertices, faces))
}

fn bounds(mesh: &EditableMesh) -> (bevy::math::Vec3, bevy::math::Vec3) {
    let mut min = bevy::math::Vec3::splat(f32::MAX);
    let mut max = bevy::math::Vec3::splat(f32::MIN);
    for v in &mesh.vertices {
        min = min.min(*v);
        max = max.max(*v);
    }
    (min, max)
}

#[test]
fn box_tessellates_to_unit_cube() {
    if !occt() {
        eprintln!("OCCT unavailable; skipping");
        return;
    }

    let mesh = tessellated(&[box_feature(1)]).expect("box solid");
    let (min, max) = bounds(&mesh);
    let size = max - min;

    assert!(mesh.faces.len() >= 12, "expected >=12 triangles");
    assert!((size.x - 1.0).abs() < 0.01, "x extent {}", size.x);
    assert!((size.y - 1.0).abs() < 0.01, "y extent {}", size.y);
    assert!((size.z - 1.0).abs() < 0.01, "z extent {}", size.z);
}

#[test]
fn fillet_feature_evaluates() {
    if !occt() {
        eprintln!("OCCT unavailable; skipping");
        return;
    }

    let features = [
        box_feature(1),
        CadFeature::Fillet {
            id: CadFeatureId(2),
            radius: 0.1,
        },
    ];

    let mesh = tessellated(&features).expect("filleted solid");
    assert!(!mesh.faces.is_empty());
}

#[test]
fn pocket_cut_adds_geometry() {
    if !occt() {
        eprintln!("OCCT unavailable; skipping");
        return;
    }

    let plain = tessellated(&[box_feature(1)]).expect("box solid");

    let sketch = CadFeature::Sketch {
        id: CadFeatureId(10),
        plane: SketchPlane::XY,
        curves: vec![SketchCurve::Rect {
            min: bevy::math::Vec2::new(-0.2, -0.2),
            max: bevy::math::Vec2::new(0.2, 0.2),
        }],
        constraints: Vec::new(),
    };
    let pocket = CadFeature::Pocket {
        id: CadFeatureId(11),
        sketch_id: CadFeatureId(10),
        distance: 2.0,
    };

    let cut = tessellated(&[box_feature(1), sketch, pocket]).expect("cut solid");

    assert!(
        cut.faces.len() > plain.faces.len(),
        "pocket should add faces ({} vs {})",
        cut.faces.len(),
        plain.faces.len()
    );
}

#[test]
fn step_roundtrip() {
    if !occt() {
        eprintln!("OCCT unavailable; skipping");
        return;
    }

    let path = std::env::temp_dir().join("polyforge_occt_roundtrip.step");
    let features = [box_feature(1)];

    polyforge_cad::export_step_occt(&features, &path).expect("STEP export");

    let text = std::fs::read_to_string(&path).expect("read STEP");
    assert!(text.starts_with("ISO-10303-21;"), "not an ISO-10303-21 file");

    let imported = polyforge_cad::import_step_occt(&path).expect("STEP import");
    assert!(!imported.vertices.is_empty());
    assert!(!imported.faces.is_empty());

    let _ = std::fs::remove_file(&path);
}
