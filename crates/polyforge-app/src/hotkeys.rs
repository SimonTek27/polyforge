//! Edit-mode and poly hotkeys (Phase 27).
use bevy::prelude::*;
use polyforge_core::*;
use polyforge_ops::*;
use polyforge_io::{write_obj_file, write_gltf_file, read_gltf_file, save_project, load_project, write_stl_ascii, write_stl_binary};

pub fn hotkey_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut gizmo: ResMut<GizmoState>,
    mut editor: ResMut<EditorState>,
    mut history: ResMut<History>,
    mut mesh_q: Query<(&mut EditableMesh, &mut Selection), With<ActiveMesh>>,
) {
    // ── Transform tools ──
    if keys.just_pressed(KeyCode::KeyG) {
        gizmo.mode = GizmoMode::Translate;
        editor.active_tool = Tool::Move;
    }
    if keys.just_pressed(KeyCode::KeyR)
        && !keys.pressed(KeyCode::ControlLeft)
        && !keys.pressed(KeyCode::ControlRight)
    {
        gizmo.mode = GizmoMode::Rotate;
        editor.active_tool = Tool::Rotate;
    }
    // Scale: only if Ctrl not held (Ctrl+S often means save)
    if keys.just_pressed(KeyCode::KeyS)
        && !keys.pressed(KeyCode::ControlLeft)
        && !keys.pressed(KeyCode::ControlRight)
    {
        gizmo.mode = GizmoMode::Scale;
        editor.active_tool = Tool::Scale;
    }

    // ── Selection modes: 1=V 2=E 3=F 4=Cell 5=Object ──
    if keys.just_pressed(KeyCode::Digit1) {
        editor.selection_mode = SelectionMode::Vertex;
        editor.active_tool = Tool::Select;
        if let Ok((_, mut sel)) = mesh_q.single_mut() {
            sel.edges.clear();
            sel.faces.clear();
        }
    }
    if keys.just_pressed(KeyCode::Digit2) {
        editor.selection_mode = SelectionMode::Edge;
        editor.active_tool = Tool::Select;
        if let Ok((_, mut sel)) = mesh_q.single_mut() {
            sel.vertices.clear();
            sel.faces.clear();
        }
    }
    if keys.just_pressed(KeyCode::Digit3) {
        editor.selection_mode = SelectionMode::Face;
        editor.active_tool = Tool::Select;
        if let Ok((_, mut sel)) = mesh_q.single_mut() {
            sel.vertices.clear();
            sel.edges.clear();
        }
    }
    if keys.just_pressed(KeyCode::Digit4) {
        editor.selection_mode = SelectionMode::Cell;
        editor.active_tool = Tool::Select;
        if let Ok((_, mut sel)) = mesh_q.single_mut() {
            sel.vertices.clear();
            sel.edges.clear();
        }
    }
    if keys.just_pressed(KeyCode::Digit5) {
        editor.selection_mode = SelectionMode::Object;
        editor.active_tool = Tool::Select;
    }

    // ── Select all / clear ──
    if keys.just_pressed(KeyCode::KeyA)
        && !keys.pressed(KeyCode::AltLeft)
        && !keys.pressed(KeyCode::AltRight)
    {
        if let Ok((mesh, mut selection)) = mesh_q.single_mut() {
            match editor.selection_mode {
                SelectionMode::Vertex => selection.select_all_vertices(mesh.vertices.len()),
                SelectionMode::Face | SelectionMode::Cell => {
                    selection.select_all_faces(mesh.faces.len())
                }
                SelectionMode::Edge => {
                    selection.clear();
                    for (a, b) in extract_edges(&mesh) {
                        selection.select_edge(a, b);
                    }
                }
                SelectionMode::Object => selection.select_all_vertices(mesh.vertices.len()),
            }
        }
    }

    // L → select linked cells (connected face region)
    if keys.just_pressed(KeyCode::KeyL) {
        if let Ok((mesh, mut selection)) = mesh_q.single_mut() {
            if editor.selection_mode.uses_faces() && !selection.faces.is_empty() {
                let linked = polyforge_core::expand_linked_faces(&mesh, &selection.faces);
                selection.faces = linked;
            }
        }
    }

    // = or NumpadAdd → grow cell/face selection by one ring
    if keys.just_pressed(KeyCode::Equal) || keys.just_pressed(KeyCode::NumpadAdd) {
        if let Ok((mesh, mut selection)) = mesh_q.single_mut() {
            if editor.selection_mode.uses_faces() && !selection.faces.is_empty() {
                selection.faces = polyforge_core::grow_face_selection(&mesh, &selection.faces);
            }
        }
    }

    // - or NumpadSubtract → shrink cell/face selection
    if keys.just_pressed(KeyCode::Minus) || keys.just_pressed(KeyCode::NumpadSubtract) {
        if let Ok((mesh, mut selection)) = mesh_q.single_mut() {
            if editor.selection_mode.uses_faces() && !selection.faces.is_empty() {
                selection.faces = polyforge_core::shrink_face_selection(&mesh, &selection.faces);
            }
        }
    }

    if keys.just_pressed(KeyCode::Escape)
        || (keys.just_pressed(KeyCode::KeyA)
            && (keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight)))
    {
        if let Ok((_, mut selection)) = mesh_q.single_mut() {
            selection.clear();
        }
    }

    // ── Edit operations ──
    // X or Delete → delete selection (vertex / edge / face)
    if (keys.just_pressed(KeyCode::KeyX)
        && !(keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight)))
        || keys.just_pressed(KeyCode::Delete)
    {
        if let Ok((mut mesh, mut selection)) = mesh_q.single_mut() {
            history.push(&mesh, &selection);
            let _ = delete_selection(&mut mesh, &mut selection);
        }
    }

    // E → extrude faces or edges (not Ctrl+Shift+E export)
    if keys.just_pressed(KeyCode::KeyE)
        && !(keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
    {
        if let Ok((mut mesh, selection)) = mesh_q.single_mut() {
            if !selection.faces.is_empty() {
                history.push(&mesh, &selection);
                let _ = extrude_faces(&mut mesh, &selection, 0.25);
            } else if !selection.edges.is_empty() {
                history.push(&mesh, &selection);
                let _ = extrude_edges(&mut mesh, &selection, 0.25);
            }
        }
    }

    // I → inset selected faces (not Ctrl+I invert)
    if keys.just_pressed(KeyCode::KeyI)
        && !keys.pressed(KeyCode::ControlLeft)
        && !keys.pressed(KeyCode::ControlRight)
    {
        if let Ok((mut mesh, selection)) = mesh_q.single_mut() {
            if !selection.faces.is_empty() {
                history.push(&mesh, &selection);
                let _ = inset_faces(&mut mesh, &selection, 0.3);
            }
        }
    }

    // Y → split selected edges (not Ctrl+Y redo)
    if keys.just_pressed(KeyCode::KeyY)
        && !keys.pressed(KeyCode::ControlLeft)
        && !keys.pressed(KeyCode::ControlRight)
    {
        if let Ok((mut mesh, selection)) = mesh_q.single_mut() {
            if !selection.edges.is_empty() {
                history.push(&mesh, &selection);
                let _ = split_edges(&mut mesh, &selection);
            }
        }
    }

    // N → nudge faces along normal
    if keys.just_pressed(KeyCode::KeyN) {
        if let Ok((mut mesh, selection)) = mesh_q.single_mut() {
            if !selection.faces.is_empty() {
                history.push(&mesh, &selection);
                let _ = move_faces_along_normal(&mut mesh, &selection, 0.1);
            }
        }
    }

    // Ctrl+R → loop cut (from selected edge)
    if keys.just_pressed(KeyCode::KeyR)
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
    {
        if let Ok((mut mesh, selection)) = mesh_q.single_mut() {
            history.push(&mesh, &selection);
            let _ = loop_cut_selection(&mut mesh, &selection, 0.5);
        }
    }

    // Ctrl+B → bevel selected edges
    if keys.just_pressed(KeyCode::KeyB)
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
    {
        if let Ok((mut mesh, selection)) = mesh_q.single_mut() {
            if !selection.edges.is_empty() {
                history.push(&mesh, &selection);
                let _ = bevel_edges(&mut mesh, &selection, 0.15);
            }
        }
    }

    // M → merge by distance
    if keys.just_pressed(KeyCode::KeyM) {
        if let Ok((mut mesh, selection)) = mesh_q.single_mut() {
            history.push(&mesh, &selection);
            let _ = merge_by_distance(&mut mesh, &selection, 0.05);
        }
    }

    // F → fill face from selected vertices
    if keys.just_pressed(KeyCode::KeyF) {
        if let Ok((mut mesh, selection)) = mesh_q.single_mut() {
            if selection.vertices.len() >= 3 {
                history.push(&mesh, &selection);
                let _ = fill_from_vertices(&mut mesh, &selection);
            }
        }
    }

    // Shift+D → duplicate selection
    if keys.just_pressed(KeyCode::KeyD)
        && (keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight))
    {
        if let Ok((mut mesh, selection)) = mesh_q.single_mut() {
            history.push(&mesh, &selection);
            let _ = duplicate_selection(&mut mesh, &selection, Vec3::new(0.3, 0.0, 0.0));
        }
    }

    // Ctrl+Z → undo
    if keys.just_pressed(KeyCode::KeyZ)
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
        && !(keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight))
    {
        if let Ok((mut mesh, mut selection)) = mesh_q.single_mut() {
            if let Some(snap) = history.undo(&mesh, &selection) {
                history.suppress = true;
                *mesh = snap.mesh;
                *selection = snap.selection;
                history.suppress = false;
            }
        }
    }

    // Ctrl+Shift+Z or Ctrl+Y → redo
    if (keys.just_pressed(KeyCode::KeyZ)
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
        && (keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight)))
        || (keys.just_pressed(KeyCode::KeyY)
            && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
            && !keys.pressed(KeyCode::ShiftLeft)
            && !keys.pressed(KeyCode::ShiftRight))
    {
        if let Ok((mut mesh, mut selection)) = mesh_q.single_mut() {
            if let Some(snap) = history.redo(&mesh, &selection) {
                history.suppress = true;
                *mesh = snap.mesh;
                *selection = snap.selection;
                history.suppress = false;
            }
        }
    }

    // Ctrl+Shift+T → STL export
    if keys.just_pressed(KeyCode::KeyT)
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
        && (keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight))
    {
        if let Ok((mesh, _)) = mesh_q.single() {
            let ascii = std::path::Path::new("/home/workdir/artifacts/polyforge_export.stl");
            let bin = std::path::Path::new("/home/workdir/artifacts/polyforge_export_bin.stl");
            match write_stl_ascii(&mesh, ascii) {
                Ok(()) => info!("STL ASCII → {}", ascii.display()),
                Err(e) => warn!("STL ASCII failed: {e}"),
            }
            match write_stl_binary(&mesh, bin) {
                Ok(()) => info!("STL binary → {}", bin.display()),
                Err(e) => warn!("STL binary failed: {e}"),
            }
        }
    }
    // Ctrl+T → triangulate (no Shift)
    else if keys.just_pressed(KeyCode::KeyT)
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
    {
        if let Ok((mut mesh, selection)) = mesh_q.single_mut() {
            history.push(&mesh, &selection);
            let _ = triangulate_faces(&mut mesh, &selection);
        }
    }

    // Ctrl+Shift+D → subdivide (D alone with Ctrl+Shift; Shift+D is duplicate without Ctrl)
    // U or W (Blender parity) → subdivide
    if keys.just_pressed(KeyCode::KeyU)
        || (keys.just_pressed(KeyCode::KeyW)
            && !keys.pressed(KeyCode::ShiftLeft)
            && !keys.pressed(KeyCode::ShiftRight)
            && !keys.pressed(KeyCode::ControlLeft)
            && !keys.pressed(KeyCode::ControlRight))
    {
        if let Ok((mut mesh, selection)) = mesh_q.single_mut() {
            history.push(&mesh, &selection);
            let _ = subdivide_faces(&mut mesh, &selection);
        }
    }

    // Ctrl+I → invert selection
    if keys.just_pressed(KeyCode::KeyI)
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
    {
        if let Ok((mesh, mut selection)) = mesh_q.single_mut() {
            match editor.selection_mode {
                SelectionMode::Vertex | SelectionMode::Object => {
                    selection.invert_vertices(mesh.vertices.len());
                }
                SelectionMode::Face | SelectionMode::Cell => {
                    selection.invert_faces(mesh.faces.len());
                }
                SelectionMode::Edge => {
                    let edges = extract_edges(&mesh);
                    selection.invert_edges(&edges);
                }
            }
        }
    }

    // Ctrl+Shift+X → symmetrize X
    if keys.just_pressed(KeyCode::KeyX)
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
        && (keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight))
    {
        if let Ok((mut mesh, selection)) = mesh_q.single_mut() {
            history.push(&mesh, &selection);
            let _ = symmetrize(&mut mesh, &selection, SymAxis::X);
        }
    }

    // Ctrl+Shift+E → export OBJ to artifacts
    if keys.just_pressed(KeyCode::KeyE)
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
        && (keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight))
    {
        if let Ok((mesh, _)) = mesh_q.single() {
            let path = std::path::Path::new("/home/workdir/artifacts/polyforge_export.obj");
            match write_obj_file(&mesh, path) {
                Ok(()) => info!("Exported OBJ to {}", path.display()),
                Err(e) => warn!("OBJ export failed: {e}"),
            }
        }
    }

    // Ctrl+Shift+B → bridge edge loops
    if keys.just_pressed(KeyCode::KeyB)
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
        && (keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight))
    {
        if let Ok((mut mesh, selection)) = mesh_q.single_mut() {
            history.push(&mesh, &selection);
            let _ = bridge_edge_loops(&mut mesh, &selection);
        }
    }

    // Ctrl+Shift+F → solidify
    if keys.just_pressed(KeyCode::KeyF)
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
        && (keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight))
    {
        if let Ok((mut mesh, selection)) = mesh_q.single_mut() {
            history.push(&mesh, &selection);
            let _ = solidify(&mut mesh, &selection, 0.1);
        }
    }

    // Ctrl+Delete → dissolve faces (or edges)
    if keys.just_pressed(KeyCode::Delete)
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
    {
        if let Ok((mut mesh, mut selection)) = mesh_q.single_mut() {
            history.push(&mesh, &selection);
            if !selection.faces.is_empty() {
                let _ = dissolve_faces(&mut mesh, &selection);
            } else if !selection.edges.is_empty() {
                let _ = dissolve_edges(&mut mesh, &selection);
            }
            selection.clear();
        }
    }

    // Shift+G → select similar faces
    if keys.just_pressed(KeyCode::KeyG)
        && (keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight))
        && !(keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
    {
        if let Ok((mesh, mut selection)) = mesh_q.single_mut() {
            if let Some(&seed) = selection.faces.iter().next() {
                selection.faces = select_similar_faces(&mesh, seed, 0.95);
            }
        }
    }

    // Ctrl+R without already handled as loop cut - edge ring via Alt+R
    if keys.just_pressed(KeyCode::KeyR)
        && (keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight))
    {
        if let Ok((mesh, mut selection)) = mesh_q.single_mut() {
            if let Some(&(a, b)) = selection.edges.iter().next() {
                let ring = select_edge_ring(&mesh, a, b);
                selection.clear();
                for e in ring {
                    selection.select_edge(e.0, e.1);
                }
            }
        }
    }

    // Ctrl+E (no shift) → mark UV seams on selected edges
    if keys.just_pressed(KeyCode::KeyE)
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
        && !(keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight))
    {
        if let Ok((mut mesh, selection)) = mesh_q.single_mut() {
            history.push(&mesh, &selection);
            for &(a, b) in &selection.edges {
                mesh.toggle_seam(a, b);
            }
            info!("Seams: {} edges marked", mesh.seams.len());
        }
    }

    // Ctrl+S → save project
    if keys.just_pressed(KeyCode::KeyS)
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
    {
        if let Ok((mesh, _)) = mesh_q.single() {
            let path = std::path::Path::new("/home/workdir/artifacts/polyforge_project.json");
            match save_project(&mesh, path, "PolyForge Scene") {
                Ok(()) => info!("Saved project to {}", path.display()),
                Err(e) => warn!("Save failed: {e}"),
            }
        }
    }

    // Ctrl+O → load project
    if keys.just_pressed(KeyCode::KeyO)
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
    {
        if let Ok((mut mesh, mut selection)) = mesh_q.single_mut() {
            let path = std::path::Path::new("/home/workdir/artifacts/polyforge_project.json");
            match load_project(path) {
                Ok((name, loaded)) => {
                    history.push(&mesh, &selection);
                    *mesh = loaded;
                    selection.clear();
                    info!("Loaded project '{name}'");
                }
                Err(e) => warn!("Load failed: {e}"),
            }
        }
    }

    // Shift+A → add plane primitive (merged into current mesh as separate island)
    if keys.just_pressed(KeyCode::KeyA)
        && (keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight))
        && !(keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
        && !(keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight))
    {
        if let Ok((mut mesh, mut selection)) = mesh_q.single_mut() {
            history.push(&mesh, &selection);
            let prim = create_plane(1.0);
            let base = mesh.vertices.len() as u32;
            for v in &prim.vertices {
                mesh.vertices.push(*v + Vec3::new(1.5, 0.0, 0.0));
            }
            for n in &prim.normals {
                mesh.normals.push(*n);
            }
            for uv in &prim.uvs {
                mesh.uvs.push(*uv);
            }
            selection.clear();
            for face in &prim.faces {
                let indices: Vec<u32> = face.indices.iter().map(|i| i + base).collect();
                let fi = mesh.faces.len() as u32;
                mesh.faces.push(polyforge_core::Face { indices: indices.clone() });
                selection.faces.insert(fi);
            }
            mesh.recalculate_normals();
        }
    }

    // P → separate selected faces into a new object
    if keys.just_pressed(KeyCode::KeyP)
        && !(keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
    {
        // handled by separate_system via event flag - inline spawn needs Commands
    }

    // V → rip vertices
    if keys.just_pressed(KeyCode::KeyV) {
        if let Ok((mut mesh, selection)) = mesh_q.single_mut() {
            history.push(&mesh, &selection);
            let _ = rip_vertices(&mut mesh, &selection, Vec3::new(0.15, 0.0, 0.0));
        }
    }

    // GG slide: double-tap G is hard; use Alt+G for slide
    if keys.just_pressed(KeyCode::KeyG)
        && (keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight))
    {
        if let Ok((mut mesh, selection)) = mesh_q.single_mut() {
            history.push(&mesh, &selection);
            let _ = slide_vertices(&mut mesh, &selection, 0.15);
        }
    }

    // O → toggle proportional edit
    if keys.just_pressed(KeyCode::KeyO)
        && !(keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
    {
        // handled in proportional_toggle with ResMut
    }

    // C → toggle circle select
    if keys.just_pressed(KeyCode::KeyC) {
        // handled in circle_select_system
    }

    // Ctrl+Shift+G → export glTF
    if keys.just_pressed(KeyCode::KeyG)
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
        && (keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight))
    {
        if let Ok((mesh, _)) = mesh_q.single() {
            let path = std::path::Path::new("/home/workdir/artifacts/polyforge_export.gltf");
            match write_gltf_file(&mesh, path) {
                Ok(()) => info!("Exported glTF to {}", path.display()),
                Err(e) => warn!("glTF export failed: {e}"),
            }
        }
    }

    // Alt+S → snap selected verts to nearest
    if keys.just_pressed(KeyCode::KeyS)
        && (keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight))
    {
        if let Ok((mut mesh, selection)) = mesh_q.single_mut() {
            history.push(&mesh, &selection);
            snap_vertices_to_nearest(&mut mesh, &selection, 0.5);
        }
    }

    // Shift+T → add torus
    if keys.just_pressed(KeyCode::KeyT)
        && (keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight))
        && !(keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
    {
        if let Ok((mut mesh, mut selection)) = mesh_q.single_mut() {
            history.push(&mesh, &selection);
            let prim = create_torus(16, 8, 0.6, 0.2);
            let base = mesh.vertices.len() as u32;
            for v in &prim.vertices {
                mesh.vertices.push(*v + Vec3::new(0.0, 0.0, 1.8));
            }
            for n in &prim.normals {
                mesh.normals.push(*n);
            }
            for uv in &prim.uvs {
                mesh.uvs.push(*uv);
            }
            selection.clear();
            for face in &prim.faces {
                let indices: Vec<u32> = face.indices.iter().map(|i| i + base).collect();
                let fi = mesh.faces.len() as u32;
                mesh.faces.push(polyforge_core::Face { indices });
                selection.faces.insert(fi);
            }
            mesh.recalculate_normals();
        }
    }

    // K → knife cut X mid-plane
    if keys.just_pressed(KeyCode::KeyK)
        && !keys.pressed(KeyCode::ControlLeft)
        && !keys.pressed(KeyCode::ControlRight)
    {
        if let Ok((mut mesh, selection)) = mesh_q.single_mut() {
            history.push(&mesh, &selection);
            let _ = knife_cut_x(&mut mesh, &selection);
        }
    }

    // Shift+W → spin 90° Y 4 steps
    if keys.just_pressed(KeyCode::KeyW)
        && (keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight))
    {
        if let Ok((mut mesh, selection)) = mesh_q.single_mut() {
            history.push(&mesh, &selection);
            let verts = selection.effective_vertices(&mesh.faces);
            let mut origin = Vec3::ZERO;
            for &i in &verts {
                origin += mesh.vertices[i as usize];
            }
            if !verts.is_empty() {
                origin /= verts.len() as f32;
            }
            let _ = spin_selection(
                &mut mesh,
                &selection,
                origin,
                Vec3::Y,
                std::f32::consts::FRAC_PI_2,
                4,
            );
        }
    }

    // Ctrl+Shift+O → import glTF
    if keys.just_pressed(KeyCode::KeyO)
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
        && (keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight))
    {
        if let Ok((mut mesh, mut selection)) = mesh_q.single_mut() {
            let path = std::path::Path::new("/home/workdir/artifacts/polyforge_export.gltf");
            match read_gltf_file(path) {
                Ok(loaded) => {
                    history.push(&mesh, &selection);
                    *mesh = loaded;
                    selection.clear();
                    info!("Imported glTF");
                }
                Err(e) => warn!("glTF import failed: {e}"),
            }
        }
    }

    // Ctrl+Alt+S → scale individual origins
    if keys.just_pressed(KeyCode::KeyS)
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
        && (keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight))
    {
        if let Ok((mut mesh, selection)) = mesh_q.single_mut() {
            history.push(&mesh, &selection);
            scale_individual_origins(&mut mesh, &selection, 1.1);
        }
    }

    // Shift+V or Alt+V → rip fill (Blender Alt+V)
    if keys.just_pressed(KeyCode::KeyV)
        && ((keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight))
            || (keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight)))
    {
        if let Ok((mut mesh, selection)) = mesh_q.single_mut() {
            history.push(&mesh, &selection);
            let _ = rip_fill(&mut mesh, &selection, Vec3::new(0.15, 0.0, 0.0));
        }
    }

    // Ctrl+Shift+` cycle snap mode (vert/edge/face/grid)
    if keys.just_pressed(KeyCode::Backquote)
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
    {
        editor.snap_mode = (editor.snap_mode + 1) % 5;
        editor.snap_enabled = editor.snap_mode != 0;
        let names = ["none", "vertex", "edge", "face", "grid"];
        info!("Snap mode: {}", names[editor.snap_mode as usize]);
    }

    // Apply geometric snap to selection
    if keys.just_pressed(KeyCode::KeyS)
        && editor.snap_enabled
        && (keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight))
        && !(keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
        && !(keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight))
    {
        if let Ok((mut mesh, selection)) = mesh_q.single_mut() {
            history.push(&mesh, &selection);
            let mode = match editor.snap_mode {
                1 => SnapTarget::Vertex,
                2 => SnapTarget::Edge,
                3 => SnapTarget::Face,
                4 => SnapTarget::Grid,
                _ => SnapTarget::None,
            };
            let thr = if mode == SnapTarget::Grid {
                editor.snap_value
            } else {
                editor.snap_value.max(0.15)
            };
            let verts = selection.effective_vertices(&mesh.faces);
            snap_selection_to_target(&mut mesh, &verts, mode, thr);
        }
    }

    // Shift+Q → screw
    if keys.just_pressed(KeyCode::KeyQ)
        && (keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight))
    {
        if let Ok((mut mesh, selection)) = mesh_q.single_mut() {
            history.push(&mesh, &selection);
            let verts = selection.effective_vertices(&mesh.faces);
            let mut origin = Vec3::ZERO;
            for &i in &verts {
                origin += mesh.vertices[i as usize];
            }
            if !verts.is_empty() {
                origin /= verts.len() as f32;
            }
            let _ = screw_selection(
                &mut mesh,
                &selection,
                origin,
                Vec3::Y,
                std::f32::consts::TAU,
                1.0,
                8,
            );
        }
    }

    // Assign material slot 1 to selected faces: Ctrl+1
    if keys.just_pressed(KeyCode::Digit1)
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
        && (keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight))
    {
        // digit1 already mode vertex - use KeyN with Ctrl for material?
    }

    // Ctrl+Shift+1 / 2 assign face material slots
    if keys.just_pressed(KeyCode::Digit1)
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
        && (keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight))
    {
        if let Ok((mut mesh, selection)) = mesh_q.single_mut() {
            for &f in &selection.faces {
                mesh.set_face_material(f, 1);
            }
            info!("Assigned material slot 1 to {} faces", selection.faces.len());
        }
    }
    if keys.just_pressed(KeyCode::Digit2)
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
        && (keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight))
    {
        if let Ok((mut mesh, selection)) = mesh_q.single_mut() {
            for &f in &selection.faces {
                mesh.set_face_material(f, 2);
            }
            info!("Assigned material slot 2 to {} faces", selection.faces.len());
        }
    }
}


// ─────────────────────────────────────────────────────────────
// Gizmo origin
// ─────────────────────────────────────────────────────────────

