
use bevy::prelude::*;
use polyforge_core::{AppMode, WorkspaceState};
use polyforge_vector::{VectorPath, VectorWorkspace, write_svg_file, read_svg_file, union_paths, intersect_paths, difference_paths, xor_paths, path_enforce_g1, smooth_polyline, martinez_union, martinez_intersect, martinez_difference};

pub fn vector_io_system(
    keys: Res<ButtonInput<KeyCode>>,
    workspace: Res<WorkspaceState>,
    paths: Query<&VectorPath>,
    mut paths_mut: Query<&mut VectorPath>,
    path_entities: Query<Entity, With<VectorPath>>,
    mut vws: ResMut<VectorWorkspace>,
    mut commands: Commands,
) {
    if workspace.mode != AppMode::Vector {
        return;
    }
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    // Ctrl+Shift+E export SVG
    if keys.just_pressed(KeyCode::KeyE) && ctrl && shift {
        let list: Vec<VectorPath> = paths.iter().cloned().collect();
        let path = std::path::Path::new("/home/workdir/artifacts/polyforge_vectors.svg");
        match write_svg_file(&list, path) {
            Ok(()) => info!("Exported SVG {} paths → {}", list.len(), path.display()),
            Err(e) => warn!("SVG export failed: {e}"),
        }
    }
    // Ctrl+Shift+I import SVG
    if keys.just_pressed(KeyCode::KeyI) && ctrl && shift {
        let path = std::path::Path::new("/home/workdir/artifacts/polyforge_vectors.svg");
        match read_svg_file(path) {
            Ok(imported) => {
                for p in imported {
                    let name = p.name.clone();
                    commands.spawn((p, Name::new(name)));
                }
                info!("Imported SVG paths");
            }
            Err(e) => warn!("SVG import failed: {e}"),
        }
    }
    // Boolean 2D on first two paths
    if ctrl && shift {
        let list: Vec<VectorPath> = paths.iter().cloned().collect();
        // Clear all vector paths
        if keys.just_pressed(KeyCode::KeyX) {
            for e in path_entities.iter() {
                commands.entity(e).despawn();
            }
            info!("Cleared all vector paths");
        }
        if list.len() >= 2 {
            if keys.just_pressed(KeyCode::KeyU) {
                let u = union_paths(&list[0], &list[1]);
                let n = u.name.clone();
                commands.spawn((u, Name::new(n)));
                info!("Vector union");
            }
            if keys.just_pressed(KeyCode::KeyJ) {
                // I is import; use J for intersect
                let inter = intersect_paths(&list[0], &list[1]);
                let n = inter.name.clone();
                commands.spawn((inter, Name::new(n)));
                info!("Vector intersect");
            }
            if keys.just_pressed(KeyCode::KeyD) {
                let d = difference_paths(&list[0], &list[1]);
                let n = d.name.clone();
                commands.spawn((d, Name::new(n)));
                info!("Vector difference");
            }
            if keys.just_pressed(KeyCode::KeyY) {
                let x = xor_paths(&list[0], &list[1]);
                let n = x.name.clone();
                commands.spawn((x, Name::new(n)));
                info!("Vector XOR");
            }
            // Martinez-style (edge split)
            if keys.just_pressed(KeyCode::Digit1) {
                let r = martinez_union(&list[0], &list[1]);
                let n = r.name.clone();
                commands.spawn((r, Name::new(n)));
                info!("Martinez union");
            }
            if keys.just_pressed(KeyCode::Digit2) {
                let r = martinez_intersect(&list[0], &list[1]);
                let n = r.name.clone();
                commands.spawn((r, Name::new(n)));
                info!("Martinez intersect");
            }
            if keys.just_pressed(KeyCode::Digit3) {
                let r = martinez_difference(&list[0], &list[1]);
                let n = r.name.clone();
                commands.spawn((r, Name::new(n)));
                info!("Martinez difference");
            }
        }
        // Toggle auto-G1
        if keys.just_pressed(KeyCode::KeyA) {
            vws.auto_g1 = !vws.auto_g1;
            info!("auto_g1 = {}", vws.auto_g1);
        }
        // G1 on active path
        if keys.just_pressed(KeyCode::KeyG) {
            if let Some(e) = vws.active_path {
                if let Ok(mut path) = paths_mut.get_mut(e) {
                    let n = path_enforce_g1(&mut path.beziers, true);
                    info!("G1 enforced on {n} joints");
                }
            }
        }
        // Smooth polyline on active
        if keys.just_pressed(KeyCode::KeyM) {
            if let Some(e) = vws.active_path {
                if let Ok(mut path) = paths_mut.get_mut(e) {
                    if path.points.len() >= 3 {
                        path.points = smooth_polyline(&path.points, path.closed);
                        info!("Polyline smoothed");
                    }
                }
            }
        }
    }
}
