//! Object-mode transform: move/rotate/scale whole objects (not mesh verts).

use bevy::prelude::*;
use bevy::input::mouse::MouseMotion;
use polyforge_core::{GizmoState, EditorState, SelectionMode, EditableObject, ActiveMesh};

/// Drag whole active object when in Object selection mode with gizmo grab.
pub fn object_mode_transform_system(
    mut gizmo: ResMut<GizmoState>,
    editor: Res<EditorState>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut motion: MessageReader<MouseMotion>,
    keys: Res<ButtonInput<KeyCode>>,
    mut q: Query<&mut Transform, (With<EditableObject>, With<ActiveMesh>)>,
) {
    if editor.selection_mode != SelectionMode::Object {
        return;
    }
    if !gizmo.dragging {
        return;
    }
    if !mouse.pressed(MouseButton::Left) {
        gizmo.dragging = false;
        return;
    }

    let mut delta = Vec2::ZERO;
    for e in motion.read() {
        delta += e.delta;
    }
    if delta.length_squared() < 1e-8 {
        return;
    }

    let Ok(mut tf) = q.get_single_mut() else {
        return;
    };

    let sens = 0.01;
    match gizmo.mode {
        GizmoMode::Translate => {
            let mut d = Vec3::ZERO;
            match gizmo.active_axis {
                GizmoAxis::X => d.x = delta.x * sens,
                GizmoAxis::Y => d.y = -delta.y * sens,
                GizmoAxis::Z => d.z = delta.x * sens,
                GizmoAxis::None | GizmoAxis::View => {
                    d.x = delta.x * sens;
                    d.y = -delta.y * sens;
                }
            }
            // Axis keys override
            if keys.pressed(KeyCode::KeyX) {
                d = Vec3::X * delta.x * sens;
            } else if keys.pressed(KeyCode::KeyY) {
                d = Vec3::Y * -delta.y * sens;
            } else if keys.pressed(KeyCode::KeyZ) {
                d = Vec3::Z * delta.x * sens;
            }
            tf.translation += d;
            gizmo.origin = tf.translation;
        }
        GizmoMode::Rotate => {
            let angle = delta.x * 0.01;
            let axis = match gizmo.active_axis {
                GizmoAxis::X => Vec3::X,
                GizmoAxis::Y => Vec3::Y,
                GizmoAxis::Z => Vec3::Z,
                _ => Vec3::Y,
            };
            tf.rotate(Quat::from_axis_angle(axis, angle));
        }
        GizmoMode::Scale => {
            let f = 1.0 + delta.y * -0.01;
            let s = match gizmo.active_axis {
                GizmoAxis::X => Vec3::new(f, 1.0, 1.0),
                GizmoAxis::Y => Vec3::new(1.0, f, 1.0),
                GizmoAxis::Z => Vec3::new(1.0, 1.0, f),
                _ => Vec3::splat(f),
            };
            tf.scale *= s;
        }
    }
}
