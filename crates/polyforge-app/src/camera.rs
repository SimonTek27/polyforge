//! Orbit camera (Phase 26).
use bevy::prelude::*;
use bevy::input::mouse::{MouseMotion, MouseWheel};
use polyforge_core::{GizmoState, VertexDragState, MarqueeState};

pub struct OrbitCamera {
    distance: f32,
    yaw: f32,
    pitch: f32,
}

pub fn orbit_camera_system(
    mut query: Query<(&mut Transform, &mut OrbitCamera)>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut motion: MessageReader<MouseMotion>,
    mut scroll: MessageReader<MouseWheel>,
    gizmo: Res<GizmoState>,
    drag: Res<VertexDragState>,
    marquee: Res<MarqueeState>,
) {
    if gizmo.is_dragging || drag.active || marquee.active {
        return;
    }

    let Ok(window) = query.single() else { return; };
    let window = window.0; // Get the window from the query

    let Ok(window) = query.get_single() else { return; };
    let window = window.0; // Get the window from the query

    let Ok(window) = window.single() else { return; };
    let cursor = window.cursor_position();

    if let Some(cursor) = cursor {
        let center = window.physical_size().as_vec2() * 0.5;
        let delta = cursor - center;

        // Yaw (horizontal rotation)
        let yaw = -delta.x * 0.002;
        // Pitch (vertical rotation)
        let pitch = -delta.y * 0.002;

        let mut cam = query.single_mut().0;
        cam.translation = Vec3::new(0.0, 0.0, 5.0);
        cam.look_at(Vec3::ZERO, Vec3::Y);
    }

    // Handle scroll zoom
    for scroll_event in scroll.read() {
        // Zoom logic here
    }
}