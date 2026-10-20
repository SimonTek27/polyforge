//! Vector path node / Bezier handle editing (Phase 12–13).

use bevy::prelude::*;
use polyforge_core::{AppMode, WorkspaceState};
use crate::{VectorPath, VectorWorkspace, enforce_g1_around_segment};

#[derive(Clone, Copy, Debug)]
pub enum DragTarget {
    Point(usize),
    Bezier { seg: usize, handle: u8 }, // 0=p0 1=p1 2=p2 3=p3
}

/// Cycle active vector path with `[` / `]` in VectorMode.
pub fn cycle_active_path_system(
    keys: Res<ButtonInput<KeyCode>>, // snap
    workspace: Res<WorkspaceState>,
    mut vws: ResMut<VectorWorkspace>,
    q: Query<Entity, With<VectorPath>>,
) {
    if workspace.mode != AppMode::Vector {
        return;
    }
    let entities: Vec<Entity> = q.iter().collect();
    if entities.is_empty() {
        vws.active_path = None;
        return;
    }
    if keys.just_pressed(KeyCode::BracketRight) {
        let next = match vws.active_path.and_then(|e| entities.iter().position(|x| *x == e)) {
            Some(i) => entities[(i + 1) % entities.len()],
            None => entities[0],
        };
        vws.active_path = Some(next);
        info!("Active vector path → {next:?}");
    }
    if keys.just_pressed(KeyCode::BracketLeft) {
        let next = match vws.active_path.and_then(|e| entities.iter().position(|x| *x == e)) {
            Some(i) => entities[(i + entities.len() - 1) % entities.len()],
            None => entities[0],
        };
        vws.active_path = Some(next);
        info!("Active vector path → {next:?}");
    }
}

/// Delete active path with Delete / Backspace in VectorMode (no Ctrl).
pub fn delete_active_path_system(
    keys: Res<ButtonInput<KeyCode>>, // snap
    workspace: Res<WorkspaceState>,
    mut vws: ResMut<VectorWorkspace>,
    mut commands: Commands,
) {
    if workspace.mode != AppMode::Vector {
        return;
    }
    let del = keys.just_pressed(KeyCode::Delete) || keys.just_pressed(KeyCode::Backspace);
    if !del {
        return;
    }
    if keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight) {
        return;
    }
    if let Some(e) = vws.active_path.take() {
        commands.entity(e).despawn();
        info!("Deleted active vector path {e:?}");
    }
}

fn cursor_on_xz(
    windows: &Query<&Window>,
    camera_q: &Query<(&Camera, &GlobalTransform)>,
) -> Option<Vec2> {
    let window = windows.single().ok()?;
    let cursor = window.cursor_position()?;
    let (camera, cam_tf) = camera_q.single().ok()?;
    let ray = camera.viewport_to_world(cam_tf, cursor).ok()?;
    let t = -ray.origin.y / ray.direction.y;
    if !t.is_finite() {
        return None;
    }
    let hit = ray.origin + *ray.direction * t;
    Some(Vec2::new(hit.x, hit.z))
}

/// Drag polyline nodes or Bezier handles with Middle Mouse (Phase 13).
pub fn drag_path_node_system(
    workspace: Res<WorkspaceState>,
    vws: Res<VectorWorkspace>,  // auto_g1 read
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut motion: MessageReader<bevy::input::mouse::MouseMotion>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    windows: Query<&Window>,
    mut paths: Query<&mut VectorPath>,
    mut drag: Local<Option<DragTarget>>,
) {
    if workspace.mode != AppMode::Vector {
        *drag = None;
        return;
    }
    let Some(active) = vws.active_path else {
        *drag = None;
        return;
    };
    let Ok(mut path) = paths.get_mut(active) else {
        return;
    };

    if mouse.just_released(MouseButton::Middle) {
        *drag = None;
        return;
    }

    if mouse.just_pressed(MouseButton::Middle) {
        let Some(p2) = cursor_on_xz(&windows, &camera_q) else {
            return;
        };
        let mut best_d = 0.35_f32;
        let mut target: Option<DragTarget> = None;

        for (i, pt) in path.points.iter().enumerate() {
            let d = pt.distance(p2);
            if d < best_d {
                best_d = d;
                target = Some(DragTarget::Point(i));
            }
        }
        for (si, seg) in path.beziers.iter().enumerate() {
            for (hi, pt) in [seg.p0, seg.p1, seg.p2, seg.p3].iter().enumerate() {
                let d = pt.distance(p2);
                if d < best_d {
                    best_d = d;
                    target = Some(DragTarget::Bezier {
                        seg: si,
                        handle: hi as u8,
                    });
                }
            }
        }
        *drag = target;
    }

    if mouse.pressed(MouseButton::Middle) {
        if let Some(target) = *drag {
            let mut delta = Vec2::ZERO;
            for e in motion.read() {
                delta += e.delta;
            }
            if delta.length_squared() > 0.0 {
                let d = Vec2::new(delta.x * 0.01, -delta.y * 0.01);
                let do_snap = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
                let snap_pt = |p: &mut Vec2| {
                    if do_snap {
                        let g = 0.1;
                        p.x = (p.x / g).round() * g;
                        p.y = (p.y / g).round() * g;
                    }
                };
                match target {
                    DragTarget::Point(i) => {
                        if let Some(p) = path.points.get_mut(i) {
                            *p += d;
                            snap_pt(p);
                        }
                    }
                    DragTarget::Bezier { seg, handle } => {
                        if let Some(s) = path.beziers.get_mut(seg) {
                            match handle {
                                0 => { s.p0 += d; snap_pt(&mut s.p0); }
                                1 => { s.p1 += d; snap_pt(&mut s.p1); }
                                2 => { s.p2 += d; snap_pt(&mut s.p2); }
                                3 => { s.p3 += d; snap_pt(&mut s.p3); }
                                _ => {}
                            }
                        }
                        if vws.auto_g1 && path.beziers.len() >= 2 {
                            enforce_g1_around_segment(&mut path.beziers, seg, true);
                        }
                    }
                }
            }
        }
    } else {
        motion.clear();
    }
}

/// Highlight active path nodes and Bezier handles.
pub fn draw_active_path_nodes(
    mut gizmos: Gizmos,
    workspace: Res<WorkspaceState>,
    vws: Res<VectorWorkspace>,  // auto_g1 read
    q: Query<(Entity, &VectorPath)>,
) {
    if workspace.mode != AppMode::Vector {
        return;
    }
    let Some(active) = vws.active_path else {
        return;
    };
    for (e, path) in &q {
        if e != active {
            continue;
        }
        let col = Color::srgb(1.0, 0.9, 0.2);
        let handle_col = Color::srgb(1.0, 0.4, 0.8);
        for p in path.flattened(8) {
            gizmos.sphere(Vec3::new(p.x, 0.04, p.y), 0.03, col);
        }
        for seg in &path.beziers {
            // control polygon
            gizmos.line(
                Vec3::new(seg.p0.x, 0.05, seg.p0.y),
                Vec3::new(seg.p1.x, 0.05, seg.p1.y),
                handle_col,
            );
            gizmos.line(
                Vec3::new(seg.p2.x, 0.05, seg.p2.y),
                Vec3::new(seg.p3.x, 0.05, seg.p3.y),
                handle_col,
            );
            for (p, r) in [
                (seg.p0, 0.045),
                (seg.p1, 0.035),
                (seg.p2, 0.035),
                (seg.p3, 0.045),
            ] {
                gizmos.sphere(Vec3::new(p.x, 0.06, p.y), r, handle_col);
            }
        }
        for p in &path.points {
            gizmos.sphere(Vec3::new(p.x, 0.05, p.y), 0.04, col);
        }
    }
}
