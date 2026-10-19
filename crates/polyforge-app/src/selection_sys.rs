//! Pointer selection: drag, marquee, click, circle, lasso (Phase 26).
use bevy::prelude::*;
use polyforge_core::*;
use crate::gizmo_sys::DRAG_THRESHOLD_PX;
use polyforge_ops::{move_selection, select_edge_loop};

#[derive(Default)]
pub struct PointerGesture {
    /// Left button is held.
    pressed: bool,
    start_pos: Vec2,
    /// True once movement exceeded the threshold.
    dragging: bool,
    /// What we decided this gesture is.
    kind: GestureKind,
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum GestureKind {
    #[default]
    None,
    /// Screen-space marquee.
    Marquee,
    /// Free move of selected vertices in the camera plane.
    VertexDrag,
}

pub fn pointer_input_system(
    mut gesture: Local<PointerGesture>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    gizmo: Res<GizmoState>,
    editor: Res<EditorState>,
    mut marquee: ResMut<MarqueeState>,
    mut drag: ResMut<VertexDragState>,
    mut mesh_q: Query<(&mut EditableMesh, &mut Selection, &Transform), With<ActiveMesh>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Ok((camera, cam_gt)) = camera_q.single() else {
        return;
    };

    let additive = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);

    // ── Press ──
    if mouse.just_pressed(MouseButton::Left) {
        // Gizmo handles own interaction
        if gizmo.hovered_axis != GizmoAxis::None || gizmo.is_dragging {
            gesture.pressed = false;
            return;
        }

        gesture.pressed = true;
        gesture.start_pos = cursor;
        gesture.dragging = false;
        gesture.kind = GestureKind::None;
        marquee.cancel();
        drag.active = false;
    }

    // ── Hold / move ──
    if gesture.pressed && mouse.pressed(MouseButton::Left) {
        let delta = cursor - gesture.start_pos;
        if !gesture.dragging && delta.length() >= DRAG_THRESHOLD_PX {
            gesture.dragging = true;

            // Decide gesture type
            let can_vertex_drag = matches!(
                editor.active_tool,
                Tool::Move | Tool::Select
            ) && gizmo.mode == GizmoMode::Translate
                || editor.active_tool == Tool::Move;

            let has_selection = mesh_q
                .single()
                .map(|(mesh, sel, _)| !sel.effective_vertices(&mesh.faces).is_empty())
                .unwrap_or(false);

            // Prefer vertex drag when Move tool + something selected
            if editor.active_tool == Tool::Move && has_selection {
                gesture.kind = GestureKind::VertexDrag;
                drag.active = true;
                drag.start_screen = gesture.start_pos;
                drag.last_screen = gesture.start_pos;
            } else if editor.active_tool == Tool::Select
                || editor.active_tool == Tool::Move && !has_selection
            {
                gesture.kind = GestureKind::Marquee;
                marquee.begin(gesture.start_pos, additive);
            } else if can_vertex_drag && has_selection {
                gesture.kind = GestureKind::VertexDrag;
                drag.active = true;
                drag.start_screen = gesture.start_pos;
                drag.last_screen = gesture.start_pos;
            } else {
                gesture.kind = GestureKind::Marquee;
                marquee.begin(gesture.start_pos, additive);
            }
        }

        match gesture.kind {
            GestureKind::Marquee => {
                marquee.update(cursor);
            }
            GestureKind::VertexDrag => {
                apply_vertex_drag(
                    &mut mesh_q,
                    &mut drag,
                    cursor,
                    &cam_gt,
                    &editor,
                );
            }
            _ => {}
        }
    }

    // ── Release ──
    if mouse.just_released(MouseButton::Left) && gesture.pressed {
        gesture.pressed = false;

        match gesture.kind {
            GestureKind::Marquee => {
                if let Some(rect) = marquee.end() {
                    apply_marquee_selection(
                        &mut mesh_q,
                        &camera,
                        cam_gt,
                        &rect,
                        &editor,
                        marquee.additive,
                    );
                }
            }
            GestureKind::VertexDrag => {
                drag.active = false;
            }
            GestureKind::None => {
                // Treat as a click selection
                let alt_held = keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight);
                apply_click_selection(
                    &mut mesh_q,
                    &camera,
                    cam_gt,
                    cursor,
                    &editor,
                    additive,
                    alt_held,
                    &keys,
                );
            }
        }

        gesture.kind = GestureKind::None;
        gesture.dragging = false;
    }
}

pub fn apply_vertex_drag(
    mesh_q: &mut Query<(&mut EditableMesh, &mut Selection, &Transform), With<ActiveMesh>>,
    drag: &mut VertexDragState,
    cursor: Vec2,
    cam_gt: &GlobalTransform,
    editor: &EditorState,
) {
    let screen_delta = cursor - drag.last_screen;
    drag.last_screen = cursor;

    if screen_delta.length_squared() < 1e-6 {
        return;
    }

    let Ok((mut mesh, selection, transform)) = mesh_q.single_mut() else {
        return;
    };

    // Move in the camera-facing plane
    let dist = cam_gt.translation().distance(transform.translation).max(0.5);
    let sensitivity = dist * 0.0015;
    let right = cam_gt.right();
    let up = cam_gt.up();
    let mut world_delta = right * screen_delta.x * sensitivity + up * -screen_delta.y * sensitivity;

    // Optional grid snap on the delta
    if editor.snap_enabled && editor.snap_value > 1e-8 {
        let s = editor.snap_value;
        world_delta.x = (world_delta.x / s).round() * s;
        world_delta.y = (world_delta.y / s).round() * s;
        world_delta.z = (world_delta.z / s).round() * s;
    }

    // Works for vertices, edges and faces via effective selection
    let _ = move_selection(&mut mesh, &selection, world_delta);
}

pub fn apply_marquee_selection(
    mesh_q: &mut Query<(&mut EditableMesh, &mut Selection, &Transform), With<ActiveMesh>>,
    camera: &Camera,
    cam_gt: &GlobalTransform,
    rect: &MarqueeRect,
    editor: &EditorState,
    additive: bool,
) {
    let Ok((mesh, mut selection, transform)) = mesh_q.single_mut() else {
        return;
    };

    let world_from_local = transform.to_matrix();

    match editor.selection_mode {
        SelectionMode::Vertex | SelectionMode::Object => {
            let mut hits = Vec::new();
            for (i, &local) in mesh.vertices.iter().enumerate() {
                let world = world_from_local.transform_point3(local);
                if let Ok(screen) = camera.world_to_viewport(cam_gt, world) {
                    if rect.contains(screen) {
                        hits.push(i as u32);
                    }
                }
            }
            if editor.selection_mode == SelectionMode::Object && !hits.is_empty() {
                selection.select_all_vertices(mesh.vertices.len());
            } else {
                selection.set_vertices_from_iter(hits, additive);
            }
        }
        SelectionMode::Edge => {
            let mut hits = Vec::new();
            for (a, b) in extract_edges(&mesh) {
                let wa = world_from_local.transform_point3(mesh.vertices[a as usize]);
                let wb = world_from_local.transform_point3(mesh.vertices[b as usize]);
                // Edge is selected if both endpoints are inside, or the midpoint
                let mid = (wa + wb) * 0.5;
                let a_in = camera
                    .world_to_viewport(cam_gt, wa)
                    .map(|s| rect.contains(s))
                    .unwrap_or(false);
                let b_in = camera
                    .world_to_viewport(cam_gt, wb)
                    .map(|s| rect.contains(s))
                    .unwrap_or(false);
                let mid_in = camera
                    .world_to_viewport(cam_gt, mid)
                    .map(|s| rect.contains(s))
                    .unwrap_or(false);
                if (a_in && b_in) || mid_in {
                    hits.push((a, b));
                }
            }
            selection.set_edges_from_iter(hits, additive);
        }
        SelectionMode::Face | SelectionMode::Cell => {
            let mut hits = Vec::new();
            for (fi, face) in mesh.faces.iter().enumerate() {
                let mut center = Vec3::ZERO;
                let mut count = 0u32;
                for &vi in &face.indices {
                    if let Some(&v) = mesh.vertices.get(vi as usize) {
                        center += world_from_local.transform_point3(v);
                        count += 1;
                    }
                }
                if count == 0 {
                    continue;
                }
                center /= count as f32;
                if let Ok(screen) = camera.world_to_viewport(cam_gt, center) {
                    if rect.contains(screen) {
                        hits.push(fi as u32);
                    }
                }
            }
            // Cell mode: after marquee, expand each hit to its linked region
            if editor.selection_mode == SelectionMode::Cell && !hits.is_empty() {
                let mut seeds = std::collections::HashSet::new();
                for h in hits {
                    seeds.insert(h);
                }
                let linked = polyforge_core::expand_linked_faces(&mesh, &seeds);
                selection.set_faces_from_iter(linked, additive);
            } else {
                selection.set_faces_from_iter(hits, additive);
            }
        }
    }
}

pub fn apply_click_selection(
    mesh_q: &mut Query<(&mut EditableMesh, &mut Selection, &Transform), With<ActiveMesh>>,
    camera: &Camera,
    cam_gt: &GlobalTransform,
    cursor: Vec2,
    editor: &EditorState,
    additive: bool,
    alt_held: bool,
    keys: &Res<ButtonInput<KeyCode>>,
) {
    let Ok(ray) = camera.viewport_to_world(cam_gt, cursor) else {
        return;
    };
    let Ok((mesh, mut selection, transform)) = mesh_q.single_mut() else {
        return;
    };

    let world_from_local = transform.to_matrix();
    let cam_dist = cam_gt.translation().distance(transform.translation);
    let vert_threshold = (DEFAULT_VERTEX_THRESHOLD * cam_dist * 0.15).clamp(0.04, 0.35);
    let edge_threshold = (DEFAULT_EDGE_THRESHOLD * cam_dist * 0.15).clamp(0.03, 0.25);

    let hit: Option<PickHit> = match editor.selection_mode {
        SelectionMode::Vertex => pick_vertex(&ray, &mesh, &world_from_local, vert_threshold),
        SelectionMode::Edge => pick_edge(&ray, &mesh, &world_from_local, edge_threshold),
        SelectionMode::Face | SelectionMode::Cell => {
            pick_face(&ray, &mesh, &world_from_local)
        }
        SelectionMode::Object => {
            if pick_face(&ray, &mesh, &world_from_local).is_some() {
                selection.select_all_vertices(mesh.vertices.len());
                return;
            }
            None
        }
    };

    match hit {
        Some(PickHit::Vertex { index, .. }) => {
            if additive {
                selection.toggle_vertex(index);
            } else {
                selection.set_vertex(index);
            }
        }
        Some(PickHit::Edge { a, b, .. }) => {
            let ctrl_held = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);
            if alt_held && ctrl_held {
                // Ctrl+Alt+click: edge ring (Blender-like)
                let ring = select_edge_ring(&mesh, a, b);
                if additive {
                    for e in ring {
                        selection.select_edge(e.0, e.1);
                    }
                } else {
                    selection.clear();
                    for e in ring {
                        selection.select_edge(e.0, e.1);
                    }
                }
            } else if alt_held {
                // Alt+click: select full edge loop (Blender-like)
                let loop_edges = select_edge_loop(&mesh, a, b);
                if additive {
                    for e in loop_edges {
                        selection.select_edge(e.0, e.1);
                    }
                } else {
                    selection.clear();
                    for e in loop_edges {
                        selection.select_edge(e.0, e.1);
                    }
                }
            } else if additive {
                selection.toggle_edge(a, b);
            } else {
                selection.set_edge(a, b);
            }
        }
        Some(PickHit::Face { index, .. }) => {
            if editor.selection_mode == SelectionMode::Cell {
                // Cell mode: select the entire connected face region
                let region = polyforge_core::select_linked_faces(&mesh, index);
                if additive {
                    for f in region {
                        selection.toggle_face(f);
                    }
                } else {
                    selection.clear();
                    selection.faces = region;
                }
            } else if additive {
                selection.toggle_face(index);
            } else {
                selection.set_face(index);
            }
        }
        None => {
            if !additive {
                selection.clear();
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────
// Selection visual feedback
// ─────────────────────────────────────────────────────────────

pub fn draw_selection_system(
    mut gizmos: Gizmos,
    query: Query<(&EditableMesh, &Selection, &Transform), With<ActiveMesh>>,
    editor: Res<EditorState>,
) {
    let Ok((mesh, selection, transform)) = query.single() else {
        return;
    };

    let world_from_local = transform.to_matrix();
    let sel_color = if editor.selection_mode == SelectionMode::Cell {
        Color::srgb(0.2, 0.85, 0.9) // cyan for cells
    } else {
        Color::srgb(1.0, 0.55, 0.1) // orange for V/E/F
    };
    let face_color = if editor.selection_mode == SelectionMode::Cell {
        Color::srgba(0.2, 0.85, 0.9, 0.35)
    } else {
        Color::srgba(1.0, 0.6, 0.1, 0.35)
    };

    for &idx in &selection.vertices {
        if let Some(&local) = mesh.vertices.get(idx as usize) {
            let world = world_from_local.transform_point3(local);
            let s = 0.05;
            gizmos.line(world - Vec3::X * s, world + Vec3::X * s, sel_color);
            gizmos.line(world - Vec3::Y * s, world + Vec3::Y * s, sel_color);
            gizmos.line(world - Vec3::Z * s, world + Vec3::Z * s, sel_color);
        }
    }

    for &(a, b) in &selection.edges {
        if let (Some(&va), Some(&vb)) = (
            mesh.vertices.get(a as usize),
            mesh.vertices.get(b as usize),
        ) {
            let wa = world_from_local.transform_point3(va);
            let wb = world_from_local.transform_point3(vb);
            gizmos.line(wa, wb, sel_color);
        }
    }

    for &fi in &selection.faces {
        if let Some(face) = mesh.faces.get(fi as usize) {
            let n = face.indices.len();
            for i in 0..n {
                let ia = face.indices[i] as usize;
                let ib = face.indices[(i + 1) % n] as usize;
                if let (Some(&va), Some(&vb)) = (mesh.vertices.get(ia), mesh.vertices.get(ib)) {
                    let wa = world_from_local.transform_point3(va);
                    let wb = world_from_local.transform_point3(vb);
                    gizmos.line(wa, wb, sel_color);
                }
            }
            if n >= 3 {
                let i0 = face.indices[0] as usize;
                for i in 2..n {
                    let ia = face.indices[i - 1] as usize;
                    let ib = face.indices[i] as usize;
                    if let (Some(&v0), Some(&va), Some(&vb)) = (
                        mesh.vertices.get(i0),
                        mesh.vertices.get(ia),
                        mesh.vertices.get(ib),
                    ) {
                        let w0 = world_from_local.transform_point3(v0);
                        let wa = world_from_local.transform_point3(va);
                        let wb = world_from_local.transform_point3(vb);
                        gizmos.line(w0, wa, face_color);
                        gizmos.line(wa, wb, face_color);
                        gizmos.line(wb, w0, face_color);
                    }
                }
            }
        }
    }

    // UV seams (magenta)
    let seam_color = Color::srgb(0.95, 0.2, 0.55);
    for &(a, b) in &mesh.seams {
        if let (Some(&va), Some(&vb)) = (
            mesh.vertices.get(a as usize),
            mesh.vertices.get(b as usize),
        ) {
            let wa = world_from_local.transform_point3(va);
            let wb = world_from_local.transform_point3(vb);
            gizmos.line(wa, wb, seam_color);
        }
    }

    if editor.selection_mode == SelectionMode::Vertex {
        let dim = Color::srgba(0.7, 0.7, 0.7, 0.45);
        let s = 0.03;
        for (i, &local) in mesh.vertices.iter().enumerate() {
            if !selection.vertices.contains(&(i as u32)) {
                let world = world_from_local.transform_point3(local);
                gizmos.line(world - Vec3::X * s, world + Vec3::X * s, dim);
                gizmos.line(world - Vec3::Y * s, world + Vec3::Y * s, dim);
                gizmos.line(world - Vec3::Z * s, world + Vec3::Z * s, dim);
            }
        }
    }

    if editor.selection_mode == SelectionMode::Edge {
        let dim = Color::srgba(0.5, 0.5, 0.55, 0.4);
        for (a, b) in extract_edges(&mesh) {
            if !selection.edges.contains(&(a.min(b), a.max(b))) {
                if let (Some(&va), Some(&vb)) = (
                    mesh.vertices.get(a as usize),
                    mesh.vertices.get(b as usize),
                ) {
                    let wa = world_from_local.transform_point3(va);
                    let wb = world_from_local.transform_point3(vb);
                    gizmos.line(wa, wb, dim);
                }
            }
        }
    }

    // Face / Cell mode: draw all face outlines so the user can aim
    if editor.selection_mode.uses_faces() {
        let dim = if editor.selection_mode == SelectionMode::Cell {
            Color::srgba(0.3, 0.7, 0.75, 0.4)
        } else {
            Color::srgba(0.45, 0.5, 0.6, 0.35)
        };
        for (fi, face) in mesh.faces.iter().enumerate() {
            if selection.faces.contains(&(fi as u32)) {
                continue;
            }
            let n = face.indices.len();
            for i in 0..n {
                let ia = face.indices[i] as usize;
                let ib = face.indices[(i + 1) % n] as usize;
                if let (Some(&va), Some(&vb)) = (mesh.vertices.get(ia), mesh.vertices.get(ib)) {
                    let wa = world_from_local.transform_point3(va);
                    let wb = world_from_local.transform_point3(vb);
                    gizmos.line(wa, wb, dim);
                }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────
// GPU mesh sync
// ─────────────────────────────────────────────────────────────


pub fn proportional_and_circle_hotkeys(
    keys: Res<ButtonInput<KeyCode>>,
    mut prop: ResMut<ProportionalEdit>,
    mut circle: ResMut<CircleSelectState>,
) {
    if keys.just_pressed(KeyCode::KeyO)
        && !keys.pressed(KeyCode::ControlLeft)
        && !keys.pressed(KeyCode::ControlRight)
    {
        prop.enabled = !prop.enabled;
        info!("Proportional edit: {}", if prop.enabled { "ON" } else { "OFF" });
    }
    if keys.just_pressed(KeyCode::KeyC)
        && !keys.pressed(KeyCode::ControlLeft)
        && !keys.pressed(KeyCode::ControlRight)
    {
        circle.active = !circle.active;
        info!("Circle select: {}", if circle.active { "ON" } else { "OFF" });
    }
}



pub fn circle_select_system(
    circle: ResMut<CircleSelectState>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_q: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut mesh_q: Query<(&EditableMesh, &mut Selection, &Transform), With<ActiveMesh>>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if !circle.active {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    let Ok((camera, cam_gt)) = camera_q.single() else { return };
    if !mouse.pressed(MouseButton::Left) {
        return;
    }
    let additive = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let Ok((mesh, mut selection, transform)) = mesh_q.single_mut() else { return };
    // Paint-select vertices near cursor in screen space
    for (i, v) in mesh.vertices.iter().enumerate() {
        let world = transform.transform_point(*v);
        if let Ok(screen) = camera.world_to_viewport(cam_gt, world) {
            if screen.distance(cursor) <= circle.radius_px {
                if additive {
                    selection.vertices.insert(i as u32);
                } else {
                    selection.vertices.insert(i as u32);
                }
            }
        }
    }
}
