//! Scene project v2 save/load + auto-save.

use bevy::prelude::*;
use polyforge_core::{
    AppSettings, EditableMesh, ActiveMesh, EditableObject, Selection, MeshId, ObjectName,
};
use polyforge_cad::{CadBody, CadDisplayMesh, CadFeature, CadWorkspace, tessellate_features};
use polyforge_vector::VectorPath;
use polyforge_io::{
    SceneProject, MeshDataV2, NamedMeshData, CadBodyData, VectorPathData, save_scene_project, load_scene_project,
    SCENE_VERSION,
};

#[derive(Resource)]
pub struct AutoSaveTimer {
    pub timer: Timer,
}

impl Default for AutoSaveTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(120.0, TimerMode::Repeating),
        }
    }
}

fn build_project(
    mesh_q: &Query<&EditableMesh, With<ActiveMesh>>,
    all_meshes: &Query<(&EditableMesh, &Transform, Option<&ObjectName>), With<EditableObject>>,
    cad_q: &Query<(&CadBody, &Transform)>,
    vec_q: &Query<&VectorPath>,
    name: &str,
) -> SceneProject {
    let mesh = mesh_q.iter().next().map(MeshDataV2::from);
    let meshes: Vec<_> = all_meshes
        .iter()
        .map(|(m, tf, n)| NamedMeshData {
            name: n.map(|x| x.0.clone()).unwrap_or_else(|| "Mesh".into()),
            translation: tf.translation.to_array(),
            mesh: MeshDataV2::from(m),
        })
        .collect();
    let cad_bodies: Vec<_> = cad_q
        .iter()
        .map(|(b, tf)| CadBodyData {
            name: b.name.clone(),
            translation: tf.translation.to_array(),
            features_json: serde_json::to_string(&b.features).unwrap_or_default(),
        })
        .collect();
    let vector_paths: Vec<_> = vec_q
        .iter()
        .map(|p| VectorPathData {
            name: p.name.clone(),
            points: p.flattened(8).iter().map(|v| [v.x, v.y]).collect(),
            closed: p.closed,
        })
        .collect();
    SceneProject {
        version: SCENE_VERSION,
        name: name.into(),
        mesh,
        meshes,
        cad_bodies,
        vector_paths,
    }
}

pub fn scene_hotkeys(
    keys: Res<ButtonInput<KeyCode>>,
    mesh_q: Query<&EditableMesh, With<ActiveMesh>>,
    all_meshes: Query<(&EditableMesh, &Transform, Option<&ObjectName>), With<EditableObject>>,
    cad_q: Query<(&CadBody, &Transform)>,
    vec_q: Query<&VectorPath>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cad_ws: ResMut<CadWorkspace>,
    mut active_mesh_q: Query<&mut EditableMesh, With<ActiveMesh>>,
    existing_cad: Query<Entity, With<CadBody>>,
    existing_vec: Query<Entity, With<VectorPath>>,
    existing_poly: Query<Entity, (With<EditableObject>, Without<CadBody>, Without<ActiveMesh>)>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);

    if keys.just_pressed(KeyCode::KeyS) && ctrl && shift {
        let path = std::path::Path::new("/home/workdir/artifacts/polyforge_scene.json");
        let project = build_project(&mesh_q, &all_meshes, &cad_q, &vec_q, "PolyForge Scene");
        match save_scene_project(&project, path) {
            Ok(()) => info!("Scene saved → {}", path.display()),
            Err(e) => warn!("Scene save failed: {e}"),
        }
    }

    // Ctrl+Shift+L → load scene
    if keys.just_pressed(KeyCode::KeyL) && ctrl && shift {
        let path = std::path::Path::new("/home/workdir/artifacts/polyforge_scene.json");
        match load_scene_project(path) {
            Ok(project) => {
                // Despawn previous CAD/vector to avoid duplicates (Phase 8)
                for e in existing_cad.iter() {
                    commands.entity(e).despawn();
                }
                for e in existing_vec.iter() {
                    commands.entity(e).despawn();
                }
                // Phase 11: despawn non-active poly objects to avoid stacking on multi-mesh load
                for e in existing_poly.iter() {
                    commands.entity(e).despawn();
                }
                cad_ws.active_body = None;
                // Restore active mesh (legacy field)
                if let Some(md) = &project.mesh {
                    if let Ok(mut mesh) = active_mesh_q.single_mut() {
                        *mesh = md.to_editable();
                    }
                }
                // Phase 10: restore all named meshes as separate objects
                // First entry can refresh active; extras are spawned
                for (i, nm) in project.meshes.iter().enumerate() {
                    let editable = nm.mesh.to_editable();
                    if i == 0 {
                        if let Ok(mut mesh) = active_mesh_q.single_mut() {
                            *mesh = editable.clone();
                        }
                        continue;
                    }
                    let handle = meshes.add(editable.to_bevy_mesh());
                    let mat = materials.add(StandardMaterial {
                        base_color: Color::srgb(0.7, 0.6, 0.5),
                        metallic: 0.05,
                        perceptual_roughness: 0.55,
                        ..default()
                    });
                    commands.spawn((
                        Mesh3d(handle),
                        MeshMaterial3d(mat),
                        Transform::from_translation(Vec3::from_array(nm.translation)),
                        EditableObject,
                        MeshId(300 + i as u64),
                        editable,
                        Selection::default(),
                        Name::new(nm.name.clone()),
                        ObjectName(nm.name.clone()),
                    ));
                }
                // Spawn CAD bodies
                for cad in &project.cad_bodies {
                    let features: Vec<CadFeature> =
                        serde_json::from_str(&cad.features_json).unwrap_or_default();
                    let mut body = CadBody::new(&cad.name);
                    body.features = features;
                    body.dirty = true;
                    let mesh = tessellate_features(&body.features);
                    body.dirty = false;
                    let handle = meshes.add(mesh.to_bevy_mesh());
                    let mat = materials.add(StandardMaterial {
                        base_color: Color::srgb(0.55, 0.65, 0.75),
                        metallic: 0.1,
                        perceptual_roughness: 0.45,
                        ..default()
                    });
                    let e = commands
                        .spawn((
                            Mesh3d(handle),
                            MeshMaterial3d(mat),
                            Transform::from_translation(Vec3::from_array(cad.translation)),
                            EditableObject,
                            MeshId(200),
                            mesh.clone(),
                            Selection::default(),
                            body,
                            CadDisplayMesh(mesh),
                            Name::new(cad.name.clone()),
                            ObjectName(cad.name.clone()),
                        ))
                        .id();
                    cad_ws.active_body = Some(e);
                }
                // Spawn vectors
                for v in &project.vector_paths {
                    let path = VectorPath {
                        name: v.name.clone(),
                        points: v.points.iter().map(|p| Vec2::new(p[0], p[1])).collect(),
                        closed: v.closed,
                        beziers: Vec::new(),
                        filled: false,
                        holes: Vec::new(),
                    };
                    commands.spawn((path, Name::new(v.name.clone())));
                }
                info!(
                    "Scene loaded: {} meshes, {} cad, {} vectors",
                    project.meshes.len(),
                    project.cad_bodies.len(),
                    project.vector_paths.len()
                );
            }
            Err(e) => warn!("Scene load failed: {e}"),
        }
    }
}

pub fn auto_save_system(
    time: Res<Time>,
    settings: Res<AppSettings>,
    mut timer: ResMut<AutoSaveTimer>,
    mesh_q: Query<&EditableMesh, With<ActiveMesh>>,
    all_meshes: Query<(&EditableMesh, &Transform, Option<&ObjectName>), With<EditableObject>>,
    cad_q: Query<(&CadBody, &Transform)>,
    vec_q: Query<&VectorPath>,
) {
    if !settings.auto_save {
        return;
    }
    timer.timer.tick(time.delta());
    if !timer.timer.just_finished() {
        return;
    }
    let path = std::path::Path::new("/home/workdir/artifacts/polyforge_autosave.json");
    let project = build_project(&mesh_q, &all_meshes, &cad_q, &vec_q, "AutoSave");
    if save_scene_project(&project, path).is_ok() {
        info!("Auto-saved scene");
    }
}
