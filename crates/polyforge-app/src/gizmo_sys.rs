//! Gizmo interaction & draw (Phase 26).
use bevy::prelude::*;
use bevy::input::mouse::{MouseButton, MouseMotion};
use polyforge_core::{GizmoState, GizmoMode, GizmoAxis, draw_gizmo, EditorState, EditableMesh, Selection, ActiveMesh, MarqueeState, VertexDragState, ProportionalEdit};
use polyforge_ops::{move_selection, translate_proportional, translate_vertices_snapped, rotate_vertices, scale_vertices};

/// Distance in pixels before a drag/marquee begins.
pub const DRAG_THRESHOLD_PX: f32 = 4.0;

pub fn update_gizmo_origin(
    mut gizmo: ResMut<GizmoState>,
    query: Query<(&EditableMesh, &Selection, &Transform), With<ActiveMesh>>,
    camera_q: Query<&Transform, With<Camera3d>>,
) {
    let Ok((mesh, selection, transform)) = query.single() else {
        return;
    };

    let effective = selection.effective_vertices(&mesh.faces);
    let local_center = if effective.is_empty() {
        mesh.center()
    } else {
        let mut sum = Vec3::ZERO;
        let mut count = 0u32;
        for &idx in &effective {
            if let Some(v) = mesh.vertices.get(idx as usize) {
                sum += *v;
                count += 1;
            }
        }
        if count == 0 {
            mesh.center()
        } else {
            sum / count as f32
        }
    };

    gizmo.origin = transform.transform_point(local_center);

    if let Ok(cam_tf) = camera_q.single() {
        let dist = cam_tf.translation.distance(gizmo.origin);
        gizmo.size = (dist * 0.15).clamp(0.4, 4.0);
    }
}

// ─────────────────────────────────────────────────────────────
// Gizmo axis interaction
// ─────────────────────────────────────────────────────────────

pub fn gizmo_interaction_system(
    mut gizmo: ResMut<GizmoState>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut motion: MessageReader<MouseMotion>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut mesh_q: Query<(&mut EditableMesh, &Selection, &Transform), With<ActiveMesh>>,
    editor: Res<EditorState>,
    marquee: Res<MarqueeState>,
    drag: Res<VertexDragState>,
    prop: Res<ProportionalEdit>,
) {
    // Don't fight with marquee or free drag
    if marquee.active || drag.active {
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

    if !gizmo.is_dragging {
        gizmo.hovered_axis = pick_gizmo_axis(&ray, &gizmo);
    }

    if mouse.just_pressed(MouseButton::Left) && gizmo.hovered_axis != GizmoAxis::None {
        gizmo.is_dragging = true;
        gizmo.active_axis = gizmo.hovered_axis;
        gizmo.drag_start_screen = cursor;
        gizmo.drag_start_origin = gizmo.origin;
    }

    if mouse.just_released(MouseButton::Left) && gizmo.is_dragging {
        gizmo.reset_drag();
    }

    if !gizmo.is_dragging {
        return;
    }

    let mut total_delta = Vec2::ZERO;
    for ev in motion.read() {
        total_delta += ev.delta;
    }
    if total_delta == Vec2::ZERO {
        return;
    }

    let Ok((mut mesh, selection, transform)) = mesh_q.single_mut() else {
        return;
    };

    let effective = selection.effective_vertices(&mesh.faces);
    if effective.is_empty() && selection.is_empty() {
        return;
    }
    let mut tmp_sel = Selection::default();
    tmp_sel.vertices = if effective.is_empty() {
        (0..mesh.vertices.len() as u32).collect()
    } else {
        effective
    };

    let sensitivity = 0.01 * gizmo.size;

    match gizmo.mode {
        GizmoMode::Translate => {
            let mut world_delta = match gizmo.active_axis {
                GizmoAxis::X => Vec3::X * total_delta.x * sensitivity,
                GizmoAxis::Y => Vec3::Y * -total_delta.y * sensitivity,
                GizmoAxis::Z => Vec3::Z * total_delta.x * sensitivity,
                _ => {
                    let right = cam_gt.right();
                    let up = cam_gt.up();
                    right * total_delta.x * sensitivity + up * -total_delta.y * sensitivity
                }
            };
            if editor.snap_enabled && editor.snap_value > 1e-8 {
                let s = editor.snap_value;
                world_delta.x = (world_delta.x / s).round() * s;
                world_delta.y = (world_delta.y / s).round() * s;
                world_delta.z = (world_delta.z / s).round() * s;
            }
            // Prefer semantic move (faces/edges/verts); fall back to vertex list
            if !selection.faces.is_empty() || !selection.edges.is_empty() {
                let _ = move_selection(&mut mesh, selection, world_delta);
            } else {
                if prop.enabled {
                    let verts = tmp_sel.effective_vertices(&mesh.faces);
                    translate_proportional(&mut mesh, &verts, world_delta, prop.radius);
                } else {
                    let _ = translate_vertices_snapped(&mut mesh, &tmp_sel, world_delta, None);
                }
            }
            gizmo.origin += world_delta;
        }
        GizmoMode::Rotate => {
            let angle = total_delta.x * 0.01;
            let axis = match gizmo.active_axis {
                GizmoAxis::X => Vec3::X,
                GizmoAxis::Y => Vec3::Y,
                GizmoAxis::Z => Vec3::Z,
                _ => cam_gt.forward().as_vec3(),
            };
            let rot = Quat::from_axis_angle(axis, angle);
            let pivot = transform.to_matrix().inverse().transform_point3(gizmo.origin);
            let _ = rotate_vertices(&mut mesh, &tmp_sel, rot, pivot);
        }
        GizmoMode::Scale => {
            let factor = 1.0 + total_delta.x * 0.005;
            let scale = match gizmo.active_axis {
                GizmoAxis::X => Vec3::new(factor, 1.0, 1.0),
                GizmoAxis::Y => Vec3::new(1.0, factor, 1.0),
                GizmoAxis::Z => Vec3::new(1.0, 1.0, factor),
                _ => Vec3::splat(factor),
            };
            let pivot = transform.to_matrix().inverse().transform_point3(gizmo.origin);
            let _ = scale_vertices(&mut mesh, &tmp_sel, scale, pivot);
        }
    }
}

pub fn pick_gizmo_axis(ray: &Ray3d, gizmo: &GizmoState) -> GizmoAxis {
    let o = gizmo.origin;
    let s = gizmo.size;
    let threshold = s * 0.12;
    let axes = [
        (GizmoAxis::X, Vec3::X),
        (GizmoAxis::Y, Vec3::Y),
        (GizmoAxis::Z, Vec3::Z),
    ];
    let mut best = GizmoAxis::None;
    let mut best_dist = threshold;
    for &(axis, dir) in &axes {
        let dist = polyforge_core::distance_ray_to_segment(ray, o, o + dir * s);
        if dist < best_dist {
            best_dist = dist;
            best = axis;
        }
    }
    best
}

pub fn draw_gizmo_system(mut gizmos: Gizmos, state: Res<GizmoState>) {
    match state.mode {
        GizmoMode::Translate | GizmoMode::Rotate | GizmoMode::Scale => {
            draw_gizmo(&mut gizmos, &state);
        }
    }
}

// ─────────────────────────────────────────────────────────────
// Pointer input: marquee area select + click select + free vertex drag
// ─────────────────────────────────────────────────────────────



