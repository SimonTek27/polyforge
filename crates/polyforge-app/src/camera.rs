//! Orbit camera (Phase 26).

use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;
use polyforge_core::{GizmoState, MarqueeState, VertexDragState};

#[derive(Component)]
pub struct OrbitCamera {
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
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

    let Ok((mut transform, mut orbit)) = query.single_mut() else {
        return;
    };

    let rotating =
        mouse.pressed(MouseButton::Middle) || (mouse.pressed(MouseButton::Right) && !mouse.pressed(MouseButton::Left));

    let mut delta = Vec2::ZERO;
    for ev in motion.read() {
        delta += ev.delta;
    }

    if rotating && delta != Vec2::ZERO {
        orbit.yaw -= delta.x * 0.005;
        orbit.pitch = (orbit.pitch - delta.y * 0.005).clamp(-1.54, 1.54);
    }

    for ev in scroll.read() {
        let factor = (-ev.y * 0.1).exp();
        orbit.distance = (orbit.distance * factor).clamp(0.5, 1000.0);
    }

    let (sin_yaw, cos_yaw) = orbit.yaw.sin_cos();
    let (sin_pitch, cos_pitch) = orbit.pitch.sin_cos();
    let offset = Vec3::new(
        orbit.distance * cos_pitch * sin_yaw,
        orbit.distance * sin_pitch,
        orbit.distance * cos_pitch * cos_yaw,
    );

    transform.translation = offset;
    transform.look_at(Vec3::ZERO, Vec3::Y);
}
