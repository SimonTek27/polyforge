//! Sync MaterialTable slots → Bevy StandardMaterial on active mesh.

use bevy::prelude::*;
use polyforge_core::{ActiveMesh, EditableMesh, MaterialTable};

/// When the material table or face slots change, update the object's StandardMaterial
/// from the active slot (Phase 2: one material per object; multi-material mesh later).
pub fn sync_materials_to_gpu(
    table: Res<MaterialTable>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    query: Query<(&EditableMesh, &MeshMaterial3d<StandardMaterial>), With<ActiveMesh>>,
) {
    if !table.is_changed() {
        return;
    }
    let slot = table
        .slots
        .get(table.active_slot as usize)
        .cloned()
        .unwrap_or_default();
    for (_mesh, mat_handle) in &query {
        if let Some(mut mat) = materials.get_mut(&mat_handle.0) {
            mat.base_color = Color::srgba(
                slot.base_color.x,
                slot.base_color.y,
                slot.base_color.z,
                slot.base_color.w,
            );
            mat.metallic = slot.metallic;
            mat.perceptual_roughness = slot.roughness;
        }
    }
}
