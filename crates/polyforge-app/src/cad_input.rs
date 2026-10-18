
//! CadMode input: spawn primitives when AppMode::Cad.

use bevy::prelude::*;
use polyforge_core::{AppMode, WorkspaceState};
use polyforge_cad::{CadWorkspace, spawn_cad_box, CadBody, add_cylinder_to_active, write_step_like, tessellate_features};

pub fn cad_hotkey_system(
    keys: Res<ButtonInput<KeyCode>>,
    workspace: Res<WorkspaceState>,
    mut cad_ws: ResMut<CadWorkspace>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut bodies: Query<&mut CadBody>,
) {
    if workspace.mode != AppMode::Cad {
        return;
    }
    // B = box body
    if keys.just_pressed(KeyCode::KeyB)
        && !keys.pressed(KeyCode::ControlLeft)
        && !keys.pressed(KeyCode::ControlRight)
    {
        spawn_cad_box(
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut cad_ws,
            "CadBox",
            1.0,
            1.0,
            1.0,
        );
        info!("CadMode: spawned box body");
    }
    // C = cylinder feature on active or new body
    if keys.just_pressed(KeyCode::KeyC)
        && !keys.pressed(KeyCode::ControlLeft)
        && !keys.pressed(KeyCode::ShiftLeft)
    {
        if let Some(e) = cad_ws.active_body {
            if let Ok(mut body) = bodies.get_mut(e) {
                add_cylinder_to_active(&mut body, 0.4, 1.2);
                info!("CadMode: added cylinder feature");
                return;
            }
        }
        let e = spawn_cad_box(
            &mut commands,
            &mut meshes,
            &mut materials,
            &mut cad_ws,
            "CadCyl",
            0.01,
            0.01,
            0.01,
        );
        if let Ok(mut body) = bodies.get_mut(e) {
            body.features.clear();
            add_cylinder_to_active(&mut body, 0.5, 1.0);
        }
    }

    // Ctrl+Shift+P → STEP-like export of active body
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if keys.just_pressed(KeyCode::KeyP) && ctrl && shift {
        if let Some(e) = cad_ws.active_body {
            if let Ok(body) = bodies.get(e) {
                let mesh = tessellate_features(&body.features);
                let path = std::path::Path::new("/home/workdir/artifacts/polyforge_export.step");
                match write_step_like(&mesh, &body.name, path) {
                    Ok(()) => info!("STEP-like export → {}", path.display()),
                    Err(err) => warn!("STEP export failed: {err}"),
                }
            }
        }
    }
}
