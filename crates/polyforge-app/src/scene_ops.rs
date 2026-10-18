//! Multi-object, modifiers, collections (Phase 26).
use bevy::prelude::*;
use polyforge_core::*;
use polyforge_ops::*;

pub fn cycle_active_object_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut editor: ResMut<EditorState>,
    meshes: Query<Entity, (With<EditableObject>, With<EditableMesh>)>,
    active: Query<Entity, With<ActiveMesh>>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    // Ctrl+Tab: cycle active object (multi-object)
    if keys.just_pressed(KeyCode::Tab) && ctrl {
        let mut list: Vec<Entity> = meshes.iter().collect();
        if list.len() < 2 {
            info!("Ctrl+Tab: only one object in scene");
            return;
        }
        list.sort_by_key(|e| e.to_bits());
        let current = active.iter().next();
        let next = if let Some(cur) = current {
            let idx = list.iter().position(|e| *e == cur).unwrap_or(0);
            list[(idx + 1) % list.len()]
        } else {
            list[0]
        };
        for e in active.iter() {
            commands.entity(e).remove::<ActiveMesh>();
        }
        commands.entity(next).insert(ActiveMesh);
        info!("Active object → {:?}", next);
        return;
    }
    // Tab: Blender-like Edit ↔ Object mode toggle
    if keys.just_pressed(KeyCode::Tab) && !ctrl {
        if editor.selection_mode == SelectionMode::Object {
            editor.selection_mode = editor.edit_selection_mode;
            info!("Edit mode: {:?}", editor.selection_mode);
        } else {
            editor.edit_selection_mode = editor.selection_mode;
            editor.selection_mode = SelectionMode::Object;
            info!("Object mode");
        }
    }
}

pub fn separate_object_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut history: ResMut<History>,
    mut mesh_q: Query<(&mut EditableMesh, &mut Selection, &Transform), With<ActiveMesh>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !keys.just_pressed(KeyCode::KeyP) {
        return;
    }
    if keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight) {
        return;
    }
    let Ok((mut mesh, mut selection, transform)) = mesh_q.get_single_mut() else {
        return;
    };
    if selection.faces.is_empty() {
        return;
    }
    history.push(&mesh, &selection);
    match separate_faces(&mut mesh, &selection) {
        Ok(new_mesh) => {
            selection.clear();
            let bevy_mesh = meshes.add(new_mesh.to_bevy_mesh());
            let mat = materials.add(StandardMaterial {
                base_color: Color::srgb(0.35, 0.8, 0.55),
                metallic: 0.05,
                perceptual_roughness: 0.55,
                ..default()
            });
            let offset = transform.translation + Vec3::new(1.5, 0.0, 0.0);
            commands.spawn((
                Mesh3d(bevy_mesh),
                MeshMaterial3d(mat),
                Transform::from_translation(offset),
                EditableObject,
                MeshId(2),
                new_mesh,
                Selection::default(),
            ));
            info!("Separated selection into new object");
        }
        Err(e) => warn!("Separate failed: {e}"),
    }
}

pub fn join_active_objects_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut history: ResMut<History>,
    mut active_q: Query<(Entity, &mut EditableMesh, &mut Selection, &Transform), With<ActiveMesh>>,
    others: Query<(Entity, &EditableMesh, &Transform), (With<EditableObject>, Without<ActiveMesh>)>,
) {
    // Ctrl+J join all non-active into active
    if !(keys.just_pressed(KeyCode::KeyJ)
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight)))
    {
        return;
    }
    let Ok((_ae, mut mesh, mut selection, a_tf)) = active_q.get_single_mut() else {
        return;
    };
    history.push(&mesh, &selection);
    let mut to_despawn = Vec::new();
    for (e, other, o_tf) in others.iter() {
        let mut transformed = other.clone();
        let delta = o_tf.translation - a_tf.translation;
        for v in &mut transformed.vertices {
            *v += delta;
        }
        let _ = join_meshes(&mut mesh, &transformed);
        to_despawn.push(e);
    }
    for e in to_despawn {
        commands.entity(e).despawn();
    }
    info!("Joined objects into active mesh");
}


pub fn draw_circle_brush_system(
    circle: Res<CircleSelectState>,
    mut gizmos: Gizmos,
    camera_q: Query<(&Camera, &GlobalTransform)>,
) {
    if !circle.active {
        return;
    }
    let Ok((camera, cam_tf)) = camera_q.single() else { return };
    // Approximate circle in world at a plane in front of camera
    let origin = cam_tf.translation() + cam_tf.forward() * 5.0;
    // Screen-space radius is approximate: draw a ring of fixed world radius
    let r = 0.35 * (circle.radius_px / 40.0);
    let color = Color::srgba(0.2, 0.7, 1.0, 0.8);
    let segments = 32;
    let right = *cam_tf.right();
    let up = *cam_tf.up();
    let mut prev = origin + right * r;
    for i in 1..=segments {
        let a = (i as f32 / segments as f32) * std::f32::consts::TAU;
        let p = origin + right * (a.cos() * r) + up * (a.sin() * r);
        gizmos.line(prev, p, color);
        prev = p;
    }
    let _ = camera;
}


pub fn lasso_select_system(
    mut lasso: ResMut<LassoState>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut cursor: MessageReader<CursorMoved>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    mut mesh_q: Query<(&EditableMesh, &mut Selection, &Transform), With<ActiveMesh>>,
    editor: Res<EditorState>,
) {
    // Ctrl+L toggle lasso
    if keys.just_pressed(KeyCode::KeyL)
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
    {
        lasso.active = !lasso.active;
        lasso.clear();
        info!("Lasso select: {}", lasso.active);
    }
    if !lasso.active {
        return;
    }
    for e in cursor.read() {
        if mouse.pressed(MouseButton::Left) {
            if lasso.points.last().map(|p| p.distance(e.position) > 4.0).unwrap_or(true) {
                lasso.points.push(e.position);
            }
        }
    }
    if mouse.just_released(MouseButton::Left) && lasso.points.len() >= 3 {
        lasso.closed = true;
        let Ok((camera, cam_tf)) = camera_q.single() else { return };
        let Ok((mesh, mut selection, transform)) = mesh_q.get_single_mut() else { return };
        if !(keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight)) {
            selection.clear();
        }
        for (i, v) in mesh.vertices.iter().enumerate() {
            let world = transform.transform_point(*v);
            if let Ok(screen) = camera.world_to_viewport(cam_tf, world) {
                if lasso.contains(screen) {
                    match editor.selection_mode {
                        SelectionMode::Face | SelectionMode::Cell => {
                            for (fi, face) in mesh.faces.iter().enumerate() {
                                if face.indices.contains(&(i as u32)) {
                                    selection.faces.insert(fi as u32);
                                }
                            }
                        }
                        SelectionMode::Edge => {
                            for &(a, b) in &polyforge_core::extract_edges(mesh) {
                                if a == i as u32 || b == i as u32 {
                                    selection.select_edge(a, b);
                                }
                            }
                        }
                        _ => {
                            selection.vertices.insert(i as u32);
                        }
                    }
                }
            }
        }
        lasso.clear();
        lasso.active = false;
    }
}

pub fn draw_lasso_system(lasso: Res<LassoState>, mut gizmos: Gizmos, camera_q: Query<(&Camera, &GlobalTransform)>) {
    if !lasso.active || lasso.points.len() < 2 {
        return;
    }
    let Ok((camera, cam_tf)) = camera_q.single() else { return };
    // Draw in a plane in front of camera by unprojecting is hard without depth;
    // skip world gizmos - egui overlay handles it
    let _ = (camera, cam_tf, &mut gizmos);
}

pub fn mirror_modifier_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut history: ResMut<History>,
    mut q: Query<(Entity, &mut EditableMesh, &mut Selection, Option<&mut ModifierStack>), With<ActiveMesh>>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let alt = keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight);
    if !keys.just_pressed(KeyCode::KeyM) || !ctrl {
        return;
    }
    let Ok((entity, mut mesh, mut selection, stack)) = q.get_single_mut() else {
        return;
    };
    if shift && !alt {
        // add mirror modifier
        let mut st = stack.map(|s| s.clone()).unwrap_or_default();
        st.mirror = Some(MirrorModifier::default());
        commands.entity(entity).insert(st);
        info!("Mirror modifier X added (Ctrl+Alt+M to apply)");
    } else if alt {
        history.push(&mesh, &selection);
        if let Some(mut st) = stack {
            apply_mirror(&mut mesh, &mut st);
            selection.clear();
            info!("Mirror modifier applied");
        } else {
            // apply one-shot mirror via symmetrize path
            let mut st = ModifierStack {
                mirror: Some(MirrorModifier::default()),
            };
            apply_mirror(&mut mesh, &mut st);
            info!("Mirror applied (no stack)");
        }
    }
}


pub fn setup_plugin_commands(registry: Res<CommandRegistry>, mut macros: ResMut<MacroRegistry>) {
    register_builtin_commands(&registry);
    register_builtin_macros(&mut macros);
    info!("Plugin commands: {:?}", registry.list());
    info!("Macros: {:?}", macros.macros.iter().map(|m| &m.name).collect::<Vec<_>>());
}

pub fn plugin_command_hotkeys(
    keys: Res<ButtonInput<KeyCode>>,
    registry: Res<CommandRegistry>,
    mut world_opt: ParamSet<(Query<&mut Selection>, )>,
) {
    // F9 runs selection.clear_all via registry needs World - use exclusive system pattern simplified
    let _ = (keys, registry, world_opt);
}

pub fn interactive_knife_system(
    mut knife: ResMut<KnifeToolState>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    windows: Query<&Window>,
    mut history: ResMut<History>,
    mut mesh_q: Query<(&mut EditableMesh, &mut Selection, &Transform), With<ActiveMesh>>,
    mut gizmos: Gizmos,
) {
    // Toggle interactive knife with Ctrl+K
    if keys.just_pressed(KeyCode::KeyK)
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
    {
        knife.active = !knife.active;
        knife.reset_points();
        info!("Interactive knife stroke: {}", knife.active);
    }
    if !knife.active {
        return;
    }

    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    let Ok((camera, cam_tf)) = camera_q.single() else { return };
    let Ok(ray) = camera.viewport_to_world(cam_tf, cursor) else { return };

    // Project onto plane through mesh center, facing camera
    let center = {
        if let Ok((mesh, _, transform)) = mesh_q.get_single() {
            transform.transform_point(mesh.center())
        } else {
            Vec3::ZERO
        }
    };
    let plane_n = *cam_tf.forward();
    let denom = ray.direction.dot(plane_n);
    if denom.abs() < 1e-6 {
        return;
    }
    let tparam = (center - ray.origin).dot(plane_n) / denom;
    let hit = ray.origin + *ray.direction * tparam;

    // Drag accumulates stroke
    if mouse.pressed(MouseButton::Left) {
        knife.push_world(hit);
    }

    // Draw stroke
    let col = Color::srgb(1.0, 0.3, 0.2);
    for w in knife.world_points.windows(2) {
        gizmos.line(w[0], w[1], col);
    }

    // Release: cut along each segment
    if mouse.just_released(MouseButton::Left) && knife.world_points.len() >= 2 {
        let points = knife.world_points.clone();
        let view_n = *cam_tf.forward();
        if let Ok((mut mesh, mut selection, transform)) = mesh_q.get_single_mut() {
            history.push(&mesh, &selection);
            let inv = transform.compute_matrix().inverse();
            for seg in points.windows(2) {
                let a = inv.transform_point3(seg[0]);
                let b = inv.transform_point3(seg[1]);
                let pn = inv.transform_vector3((seg[1] - seg[0]).cross(view_n).normalize_or_zero());
                let _ = knife_plane_cut(&mut mesh, a, b, pn);
            }
            selection.clear();
            info!("Knife stroke applied ({} segments)", points.len().saturating_sub(1));
        }
        knife.reset_points();
        // keep active for multi-cuts until Ctrl+K again
    }

    // Enter confirms and exits
    if keys.just_pressed(KeyCode::Enter) {
        knife.active = false;
        knife.reset_points();
    }
}

pub fn array_modifier_hotkeys(
    keys: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut history: ResMut<History>,
    mut q: Query<(Entity, &mut EditableMesh, &mut Selection, Option<&mut ModifierStack>), With<ActiveMesh>>,
) {
    let ctrl = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let alt = keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight);
    // Ctrl+Shift+Y = add array
    if keys.just_pressed(KeyCode::KeyY) && ctrl && shift && !alt {
        let Ok((entity, _, _, stack)) = q.get_single_mut() else { return };
        let mut st = stack.map(|s| s.clone()).unwrap_or_default();
        st.array = Some(ArrayModifier::default());
        commands.entity(entity).insert(st);
        info!("Array modifier added (Ctrl+Alt+Y to apply all)");
    }
    // Ctrl+Alt+Y = apply all modifiers
    if keys.just_pressed(KeyCode::KeyY) && ctrl && alt {
        let Ok((_, mut mesh, mut selection, stack)) = q.get_single_mut() else { return };
        history.push(&mesh, &selection);
        if let Some(mut st) = stack {
            apply_all_modifiers(&mut mesh, &mut st);
        }
        selection.clear();
        info!("All modifiers applied");
    }
}


pub fn collection_visibility_system(
    collections: Query<(Entity, &Collection)>,
    members: Query<(Entity, Option<&InCollection>, Option<&ObjectParent>), With<EditableObject>>,
    mut visibility: Query<&mut Visibility>,
    keys: Res<ButtonInput<KeyCode>>,
    mut col_q: Query<&mut Collection>,
) {
    // H toggles active collection visibility when Shift held
    if keys.just_pressed(KeyCode::KeyH)
        && (keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight))
    {
        for mut c in &mut col_q {
            c.visible = !c.visible;
            info!("Collection '{}' visible={}", c.name, c.visible);
        }
    }

    // Build map collection entity -> visible
    let mut vis_map = std::collections::HashMap::new();
    for (e, c) in &collections {
        vis_map.insert(e, c.visible);
    }

    for (entity, in_col, _parent) in &members {
        let show = if let Some(InCollection(ce)) = in_col {
            vis_map.get(ce).copied().unwrap_or(true)
        } else {
            true
        };
        if let Ok(mut vis) = visibility.get_mut(entity) {
            *vis = if show {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
}

pub fn queue_plugin_hotkeys(
    keys: Res<ButtonInput<KeyCode>>,
    mut pending: ResMut<PendingPluginCommand>,
    registry: Res<CommandRegistry>,
) {
    // F9: selection.clear_all
    if keys.just_pressed(KeyCode::F9) {
        pending.0 = Some("selection.clear_all".into());
    }
    // F10: mesh.recalc_normals
    if keys.just_pressed(KeyCode::F10) {
        pending.0 = Some("mesh.recalc_normals".into());
    }
    // F8: list
    if keys.just_pressed(KeyCode::F8) {
        info!("Plugins: {:?}", registry.list());
    }
}

pub fn queue_macro_hotkeys(
    keys: Res<ButtonInput<KeyCode>>,
    macros: Res<MacroRegistry>,
    mut pending: ResMut<PendingPluginCommand>,
) {
    // F7: run first macro steps sequentially via pending (one step per frame simplified: push all)
    if keys.just_pressed(KeyCode::F7) {
        if let Some(m) = macros.get("prep.export") {
            info!("Running macro {} ({} steps)", m.name, m.steps.len());
            // Queue first step; remaining via MacroRunner state
            if let Some(first) = m.steps.first() {
                pending.0 = Some(first.clone());
            }
        }
    }
}


pub fn execute_pending_plugin_command(
    mut pending: ResMut<PendingPluginCommand>,
    registry: Res<CommandRegistry>,
    mut sel_q: Query<&mut Selection>,
    mut mesh_q: Query<&mut EditableMesh>,
) {
    let Some(name) = pending.0.take() else { return };
    match name.as_str() {
        "selection.clear_all" => {
            for mut sel in &mut sel_q {
                sel.clear();
            }
            info!("plugin: selection.clear_all");
        }
        "mesh.recalc_normals" => {
            for mut mesh in &mut mesh_q {
                mesh.recalculate_normals();
            }
            info!("plugin: mesh.recalc_normals");
        }
        other => {
            info!("plugin command '{other}' (registry has {} cmds)", registry.list().len());
        }
    }
}



pub fn parent_transform_system(
    parents: Query<&GlobalTransform, (With<EditableObject>, Without<ObjectParent>)>,
    mut children: Query<(&ObjectParent, &ObjectLocal, &mut Transform)>,
) {
    for (ObjectParent(pe), local, mut tf) in &mut children {
        let Ok(pgt) = parents.get(*pe) else { continue };
        let parent_tf = pgt.compute_transform();
        tf.translation = parent_tf.translation + parent_tf.rotation * (local.translation * parent_tf.scale);
        tf.rotation = parent_tf.rotation * local.rotation;
        tf.scale = parent_tf.scale * local.scale;
    }
}
