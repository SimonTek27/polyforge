//! Paint-mode viewport interaction.
//!
//! Casts a ray from the cursor onto the active mesh, finds the hit face, maps
//! the hit point through the face's UVs and stamps the brush into the active
//! [`polyforge_paint`] layer.

use bevy::prelude::*;
use bevy_egui::EguiContexts;
use polyforge_core::{pick_face, ActiveMesh, AppMode, EditableMesh, PickHit, WorkspaceState};
use polyforge_paint::{projection, PaintState};

/// Result of a mesh pick.

/// Paint stroke state carried between frames.
#[derive(Default)]
pub struct PaintStroke {
    last: Option<(f32, f32)>,
    painting: bool,
}

pub fn paint_stroke_system(
    mut contexts: EguiContexts,
    mouse: Res<ButtonInput<MouseButton>>,
    workspace: Res<WorkspaceState>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mesh_q: Query<(&EditableMesh, &GlobalTransform), With<ActiveMesh>>,
    mut paint: ResMut<PaintState>,
    mut stroke: Local<PaintStroke>,
) {
    if workspace.mode != AppMode::Paint {
        stroke.last = None;
        stroke.painting = false;
        return;
    }

    if !mouse.pressed(MouseButton::Left) {
        stroke.last = None;
        stroke.painting = false;
        return;
    }

    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    if ctx.egui_wants_pointer_input() {
        stroke.last = None;
        stroke.painting = false;
        return;
    }

    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok((camera, cam_gt)) = camera_q.single() else {
        return;
    };
    let Ok(ray) = camera.viewport_to_world(cam_gt, cursor) else {
        return;
    };
    let Ok((mesh, global)) = mesh_q.single() else {
        stroke.last = None;
        return;
    };

    let world_from_local = global.to_matrix();
    let Some(PickHit::Face { index, distance }) = pick_face(&ray, mesh, &world_from_local) else {
        stroke.last = None;
        return;
    };
    let Some(face) = mesh.faces.get(index as usize) else {
        return;
    };
    if face.indices.len() < 3 {
        return;
    }

    // World hit point -> mesh-local space.
    let hit_world = ray.origin + *ray.direction * distance;
    let local_from_world = world_from_local.inverse();
    let hit_local = local_from_world.transform_point3(hit_world);

    let Some((u, v)) = face_uv_at(mesh, face.indices.as_slice(), hit_local) else {
        return;
    };

    let (px, py) = projection::uv_to_pixel(u, v, paint.document.width, paint.document.height);

    if let Some(last) = stroke.last {
        paint.stroke(last, (px, py));
    } else {
        paint.stamp(px, py);
    }
    stroke.last = Some((px, py));
    stroke.painting = true;
}

/// Interpolate the UV at `point` (mesh-local) across a face's triangle fan.
fn face_uv_at(mesh: &EditableMesh, indices: &[u32], point: Vec3) -> Option<(f32, f32)> {
    if indices.len() < 3 {
        return None;
    }
    let pos = |i: u32| mesh.vertices.get(i as usize).copied();
    let uv = |i: u32| mesh.uvs.get(i as usize).copied();

    let a = pos(indices[0])?;
    let uv_a = uv(indices[0]).unwrap_or(Vec2::ZERO);

    for k in 1..indices.len() - 1 {
        let b = pos(indices[k])?;
        let c = pos(indices[k + 1])?;
        let Some((wa, wb, wc)) = barycentric_3d(point, a, b, c) else {
            continue;
        };
        if wa < -1e-4 || wb < -1e-4 || wc < -1e-4 {
            continue;
        }
        let uv_b = uv(indices[k]).unwrap_or(Vec2::ZERO);
        let uv_c = uv(indices[k + 1]).unwrap_or(Vec2::ZERO);
        let uv = uv_a * wa + uv_b * wb + uv_c * wc;
        return Some((uv.x.clamp(0.0, 1.0), uv.y.clamp(0.0, 1.0)));
    }
    None
}

fn barycentric_3d(p: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Option<(f32, f32, f32)> {
    let v0 = b - a;
    let v1 = c - a;
    let v2 = p - a;
    let d00 = v0.dot(v0);
    let d01 = v0.dot(v1);
    let d11 = v1.dot(v1);
    let d20 = v2.dot(v0);
    let d21 = v2.dot(v1);
    let denom = d00 * d11 - d01 * d01;
    if denom.abs() < 1e-12 {
        return None;
    }
    let wb = (d11 * d20 - d01 * d21) / denom;
    let wc = (d00 * d21 - d01 * d20) / denom;
    let wa = 1.0 - wb - wc;
    Some((wa, wb, wc))
}

/// GPU-side paint texture and the material it was temporarily bound to.
#[derive(Resource)]
pub struct PaintTextureBinding {
    image: Option<Handle<bevy::image::Image>>,
    revision: u64,
    bound_material: Option<Handle<StandardMaterial>>,
    original_texture: Option<Handle<bevy::image::Image>>,
    original_color: Option<Color>,
}

impl Default for PaintTextureBinding {
    fn default() -> Self {
        Self {
            image: None,
            revision: u64::MAX,
            bound_material: None,
            original_texture: None,
            original_color: None,
        }
    }
}

/// Bind the composited paint canvas to the active mesh material while in Paint
/// mode, re-uploading only when `PaintState::revision` changes.
pub fn paint_texture_system(
    workspace: Res<WorkspaceState>,
    paint: Res<PaintState>,
    mut binding: ResMut<PaintTextureBinding>,
    mut images: ResMut<Assets<bevy::image::Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mesh_q: Query<&MeshMaterial3d<StandardMaterial>, With<ActiveMesh>>,
) {
    if workspace.mode != AppMode::Paint {
        if let Some(mat_handle) = binding.bound_material.take() {
            if let Some(mut material) = materials.get_mut(&mat_handle) {
                material.base_color_texture = binding.original_texture.take();
                if let Some(color) = binding.original_color.take() {
                    material.base_color = color;
                }
            }
            binding.image = None;
            binding.revision = u64::MAX;
        }
        return;
    }

    let Ok(material_handle) = mesh_q.single() else {
        return;
    };
    let material_handle = material_handle.0.clone();

    if binding.bound_material.as_ref() != Some(&material_handle) {
        if let Some(previous) = binding.bound_material.take() {
            if let Some(mut material) = materials.get_mut(&previous) {
                material.base_color_texture = binding.original_texture.take();
                if let Some(color) = binding.original_color.take() {
                    material.base_color = color;
                }
            }
        }
        if let Some(mut material) = materials.get_mut(&material_handle) {
            binding.original_texture = material.base_color_texture.clone();
            binding.original_color = Some(material.base_color);
        }
        binding.bound_material = Some(material_handle.clone());
        binding.revision = u64::MAX;
    }

    if binding.revision != paint.revision || binding.image.is_none() {
        let image = paint.document.composite().to_bevy_image();
        match binding.image.clone() {
            Some(handle) => {
                images.insert(&handle, image);
            }
            None => {
                binding.image = Some(images.add(image));
            }
        }
        binding.revision = paint.revision;
    }

    if let (Some(image), Some(mut material)) =
        (binding.image.clone(), materials.get_mut(&material_handle))
    {
        material.base_color_texture = Some(image);
        material.base_color = Color::WHITE;
    }
}

