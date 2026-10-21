//! Paint-mode viewport interaction.
//!
//! Casts a ray from the cursor onto the active mesh, finds the hit face, maps
//! the hit point through the face's UVs and stamps the brush into the active
//! [`polyforge_paint`] layer.

use bevy::prelude::*;
use bevy_egui::EguiContexts;
use polyforge_core::{pick_face, ActiveMesh, AppMode, EditableMesh, PickHit, WorkspaceState};
use polyforge_paint::{projection, PaintChannel, PaintState, PaintTool};

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
    keys: Res<ButtonInput<KeyCode>>,
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

    let alt = keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight);
    if alt {
        let x = px.clamp(0.0, paint.document.width as f32 - 1.0) as u32;
        let y = py.clamp(0.0, paint.document.height as f32 - 1.0) as u32;
        if paint.tool == PaintTool::Clone {
            paint.clone_source = Some((x as f32, y as f32));
        } else {
            paint.pick_color(x, y);
        }
        stroke.last = None;
        stroke.painting = false;
        return;
    }

    if paint.tool == PaintTool::Clone {
        if let Some(source) = paint.clone_source.take() {
            paint.clone_offset = (source.0 - px, source.1 - py);
        }
    }

    if paint.tool == PaintTool::Fill {
        if !stroke.painting {
            let x = px.clamp(0.0, paint.document.width as f32 - 1.0) as u32;
            let y = py.clamp(0.0, paint.document.height as f32 - 1.0) as u32;
            paint.fill_region(x, y);
        }
        stroke.last = Some((px, py));
        stroke.painting = true;
        return;
    }

    match stroke.last {
        Some(last) => paint.stroke(last, (px, py)),
        None => {
            paint.begin_edit();
            paint.stamp(px, py);
        }
    }
    stroke.last = Some((px, py));
    stroke.painting = true;
}

/// Undo/redo hotkeys for Paint mode (Ctrl+Z, Ctrl+Shift+Z, Ctrl+Y).
pub fn paint_history_system(
    mut contexts: EguiContexts,
    keys: Res<ButtonInput<KeyCode>>,
    workspace: Res<WorkspaceState>,
    mut paint: ResMut<PaintState>,
) {
    if workspace.mode != AppMode::Paint {
        return;
    }
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    if ctx.egui_wants_keyboard_input() {
        return;
    }
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    if ctrl {
        let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
        if keys.just_pressed(KeyCode::KeyZ) {
            if shift {
                paint.redo();
            } else {
                paint.undo();
            }
        } else if keys.just_pressed(KeyCode::KeyY) {
            paint.redo();
        }
        return;
    }

    if keys.just_pressed(KeyCode::KeyB) {
        paint.tool = PaintTool::Brush;
    } else if keys.just_pressed(KeyCode::KeyE) {
        paint.tool = PaintTool::Eraser;
    } else if keys.just_pressed(KeyCode::KeyC) {
        paint.tool = PaintTool::Clone;
    } else if keys.just_pressed(KeyCode::KeyF) {
        paint.tool = PaintTool::Fill;
    } else if keys.just_pressed(KeyCode::BracketLeft) {
        paint.brush.size = (paint.brush.size * 0.85).max(1.0);
    } else if keys.just_pressed(KeyCode::BracketRight) {
        paint.brush.size = (paint.brush.size * 1.15).min(512.0);
    }
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

/// Material fields captured before Paint mode overrides them.
#[derive(Clone)]
struct MaterialSnapshot {
    base_color: Color,
    base_color_texture: Option<Handle<bevy::image::Image>>,
    metallic_roughness_texture: Option<Handle<bevy::image::Image>>,
    normal_map_texture: Option<Handle<bevy::image::Image>>,
    emissive: LinearRgba,
    emissive_texture: Option<Handle<bevy::image::Image>>,
}

/// GPU-side PBR texture set and the material it was temporarily bound to.
#[derive(Resource, Default)]
pub struct PaintTextureBinding {
    /// Base colour, ORM, normal and emissive images.
    images: [Option<Handle<bevy::image::Image>>; 4],
    revision: u64,
    bound_material: Option<Handle<StandardMaterial>>,
    original: Option<MaterialSnapshot>,
}

fn upsert(
    images: &mut Assets<bevy::image::Image>,
    slot: &mut Option<Handle<bevy::image::Image>>,
    image: bevy::image::Image,
) {
    match slot {
        Some(handle) => {
            let _ = images.insert(handle, image);
        }
        None => {
            *slot = Some(images.add(image));
        }
    }
}

fn restore_material(
    materials: &mut Assets<StandardMaterial>,
    handle: &Handle<StandardMaterial>,
    snapshot: &MaterialSnapshot,
) {
    if let Some(mut material) = materials.get_mut(handle) {
        material.base_color = snapshot.base_color;
        material.base_color_texture = snapshot.base_color_texture.clone();
        material.metallic_roughness_texture = snapshot.metallic_roughness_texture.clone();
        material.normal_map_texture = snapshot.normal_map_texture.clone();
        material.emissive = snapshot.emissive;
        material.emissive_texture = snapshot.emissive_texture.clone();
    }
}

/// Bind the composited PBR channels to the active mesh material while in Paint
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
            if let Some(snapshot) = binding.original.take() {
                restore_material(&mut materials, &mat_handle, &snapshot);
            }
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
            if let Some(snapshot) = binding.original.take() {
                restore_material(&mut materials, &previous, &snapshot);
            }
        }
        if let Some(material) = materials.get_mut(&material_handle) {
            binding.original = Some(MaterialSnapshot {
                base_color: material.base_color,
                base_color_texture: material.base_color_texture.clone(),
                metallic_roughness_texture: material.metallic_roughness_texture.clone(),
                normal_map_texture: material.normal_map_texture.clone(),
                emissive: material.emissive,
                emissive_texture: material.emissive_texture.clone(),
            });
        }
        binding.bound_material = Some(material_handle.clone());
        binding.revision = u64::MAX;
    }

    if binding.revision != paint.revision || binding.images[0].is_none() {
        let base = paint
            .composite_channel(PaintChannel::BaseColor)
            .to_bevy_image();
        let orm = paint.export_orm().to_bevy_image_linear();
        let normal = paint
            .normal
            .composite()
            .to_bevy_image_linear();
        let emissive = paint.emissive.composite().to_bevy_image();

        let [base_slot, orm_slot, normal_slot, emissive_slot] = &mut binding.images;
        upsert(&mut images, base_slot, base);
        upsert(&mut images, orm_slot, orm);
        upsert(&mut images, normal_slot, normal);
        upsert(&mut images, emissive_slot, emissive);
        binding.revision = paint.revision;
    }

    if let Some(mut material) = materials.get_mut(&material_handle) {
        material.base_color_texture = binding.images[0].clone();
        material.metallic_roughness_texture = binding.images[1].clone();
        material.normal_map_texture = binding.images[2].clone();
        material.emissive_texture = binding.images[3].clone();
        material.base_color = Color::WHITE;
        material.emissive = LinearRgba::WHITE;
    }
}

