//! Recompute dirty CadBody feature stacks and sync display meshes to GPU.

use bevy::prelude::*;
use polyforge_core::{EditableMesh, EditableObject, ActiveMesh, MeshId, Selection};
use crate::{CadBody, CadDisplayMesh, CadFeature, CadWorkspace, tessellate_features};

/// Rebuild CadDisplayMesh / EditableMesh when body.dirty.
pub fn cad_recompute_system(mut q: Query<(&mut CadBody, &mut CadDisplayMesh, Option<&mut EditableMesh>)>) {
    for (mut body, mut display, editable) in &mut q {
        if !body.dirty {
            continue;
        }
        let mesh = tessellate_features(&body.features);
        display.0 = mesh.clone();
        if let Some(mut em) = editable {
            *em = mesh;
        }
        body.dirty = false;
        info!("CadBody '{}' recomputed ({} features)", body.name, body.features.len());
    }
}

/// Ensure CadBody entities that have Mesh3d get GPU updates when EditableMesh changes.
pub fn cad_sync_display_mesh_system(
    mut meshes: ResMut<Assets<Mesh>>,
    q: Query<(&EditableMesh, &Mesh3d), (With<CadBody>, Changed<EditableMesh>)>,
) {
    for (editable, mesh3d) in &q {
        if let Some(mut gpu) = meshes.get_mut(&mesh3d.0) {
            *gpu = editable.to_bevy_mesh();
        }
    }
}

static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

pub fn next_feature_id() -> crate::CadFeatureId {
    crate::CadFeatureId(NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
}

/// Spawn a new CAD body with a single box feature.
pub fn spawn_cad_box(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    workspace: &mut CadWorkspace,
    name: &str,
    w: f32,
    h: f32,
    d: f32,
) -> Entity {
    use crate::{CadFeature, CadFeatureId};
    let id = next_feature_id();
    let mut body = CadBody::new(name);
    body.push_feature(CadFeature::Box {
        id,
        width: w,
        height: h,
        depth: d,
    });
    let mesh = tessellate_features(&body.features);
    body.dirty = false;
    let handle = meshes.add(mesh.to_bevy_mesh());
    let mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.65, 0.75),
        metallic: 0.1,
        perceptual_roughness: 0.45,
        ..default()
    });
    let e = commands
        .spawn((
            Mesh3d(handle),
            MeshMaterial3d(mat),
            Transform::from_translation(Vec3::new(2.0, 0.0, 0.0)),
            EditableObject,
            MeshId(100),
            mesh.clone(),
            Selection::default(),
            body,
            CadDisplayMesh(mesh),
            Name::new(name.to_string()),
            polyforge_core::ObjectName(name.to_string()),
        ))
        .id();
    workspace.active_body = Some(e);
    e
}

pub fn add_cylinder_to_active(
    body: &mut CadBody,
    radius: f32,
    height: f32,
) {
    body.push_feature(CadFeature::Cylinder {
        id: next_feature_id(),
        radius,
        height,
        segments: 32,
    });
}


pub fn spawn_cad_pad_rect(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    workspace: &mut CadWorkspace,
    name: &str,
    width: f32,
    depth: f32,
    height: f32,
) -> Entity {
    let mut body = CadBody::new(name);
    body.add_pad_rect(width, depth, height);
    let mesh = tessellate_features(&body.features);
    body.dirty = false;
    let handle = meshes.add(mesh.to_bevy_mesh());
    let mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.45, 0.7, 0.55),
        metallic: 0.05,
        perceptual_roughness: 0.5,
        ..default()
    });
    let e = commands
        .spawn((
            Mesh3d(handle),
            MeshMaterial3d(mat),
            Transform::from_translation(Vec3::new(2.5, 0.0, 1.5)),
            EditableObject,
            MeshId(101),
            mesh.clone(),
            Selection::default(),
            body,
            CadDisplayMesh(mesh),
            Name::new(name.to_string()),
            polyforge_core::ObjectName(name.to_string()),
        ))
        .id();
    workspace.active_body = Some(e);
    e
}
