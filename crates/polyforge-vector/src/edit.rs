//! Vector path node / Bezier handle editing (Phase 12–13).

use bevy::prelude::*;
use polyforge_core::{AppMode, WorkspaceState};
use crate::{VectorPath, VectorWorkspace};


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
