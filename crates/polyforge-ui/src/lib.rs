//! # polyforge-ui
//!
//! Immediate-mode UI for the editor using bevy_egui.
//! All user-visible strings go through [`polyforge_core::I18n`].

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use polyforge_cad::{
    CadBody, CadWorkspace, feature_params, apply_feature_params,
    mesh_fillet_approximate, solve_body_sketches, solve_body_sketches_lsq, estimate_sketch_dof, constraint_residual,
    add_constraint_to_last_sketch, UserConstraint, UserConstraintKind,
};
use polyforge_vector::{VectorPath, VectorWorkspace};
use polyforge_core::{
    ActiveMesh, EditableMesh, EditableObject, EditorState, GizmoMode, GizmoState, I18n, Locale,
    MarqueeState, Msg, ObjectName, Selection, SelectionMode, Tool, CommandRegistry,
    MaterialTable, UvEditorState,
    WorkspaceState, AppMode, AppSettings, Units,
};

pub struct PolyForgeUiPlugin;

impl Plugin for PolyForgeUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin::default())
            .add_systems(bevy_egui::EguiPrimaryContextPass, editor_ui_main_panel)
            .add_systems(bevy_egui::EguiPrimaryContextPass, editor_ui_windows)
            .add_systems(bevy_egui::EguiPrimaryContextPass, editor_ui_marquee)
            .add_systems(Update, sync_window_title);
    }
}

/// Keep the OS window title in sync with the active locale.
fn sync_window_title(i18n: Res<I18n>, mut windows: Query<&mut Window>) {
    if !i18n.is_changed() && !i18n.is_added() {
        return;
    }
    let title = i18n.t(Msg::AppTitle);
    for mut window in &mut windows {
        if window.title != title {
            window.title = title.to_string();
        }
    }
}

// ============================================================================
// Main panel (central) with embedded side panels
// ============================================================================

fn editor_ui_main_panel(
    mut contexts: EguiContexts,
    mut editor: ResMut<EditorState>,
    mut gizmo: ResMut<GizmoState>,
    mut i18n: ResMut<I18n>,
    marquee: Res<MarqueeState>,
    selection_q: Query<(&EditableMesh, &Selection), With<ActiveMesh>>,
    mut workspace: ResMut<WorkspaceState>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return; };

    // Use a full-screen window instead of CentralPanel (egui 0.36 API change)
    egui::Window::new("PolyForge")
        .id(egui::Id::new("polyforge_main_window"))
        .default_size(egui::vec2(f32::INFINITY, f32::INFINITY))
        .resizable(false)
        .collapsible(false)
        .title_bar(false)
        .anchor(egui::Align2::LEFT_TOP, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
        // ── Marquee rectangle overlay ──
        if let Some(rect) = marquee.rect() {
            let painter = ui.painter();
            let screen_rect = egui::Rect::from_min_max(
                egui::pos2(rect.min.x, rect.min.y),
                egui::pos2(rect.max.x, rect.max.y),
            );
            painter.rect(
                screen_rect,
                0.0,
                egui::Color32::from_rgba_unmultiplied(80, 140, 255, 40),
                egui::Stroke::new(1.5, egui::Color32::from_rgb(80, 140, 255)),
                egui::StrokeKind::Inside,
            );
        }

        // ── Top menu bar ──
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button(i18n.t(Msg::MenuFile), |ui| {
                let _ = ui.button(i18n.t(Msg::MenuNewScene));
                let _ = ui.button(i18n.t(Msg::MenuOpen));
                let _ = ui.button(i18n.t(Msg::MenuSave));
                ui.separator();
                let _ = ui.button(i18n.t(Msg::MenuQuit));
            });
            ui.menu_button(i18n.t(Msg::MenuEdit), |ui| {
                let _ = ui.button(i18n.t(Msg::MenuUndo));
                let _ = ui.button(i18n.t(Msg::MenuRedo));
            });
            ui.menu_button(i18n.t(Msg::MenuView), |ui| {
                let _ = ui.button(i18n.t(Msg::MenuResetCamera));
            });
            ui.menu_button(i18n.t(Msg::MenuHelp), |ui| {
                let _ = ui.button(i18n.t(Msg::MenuAbout));
            });
        });

        ui.add_space(4.0);

        // ── Left mode selector ──
        egui::Panel::left("mode_selector")
            .exact_size(88.0)
            .resizable(false)
            .show(ui, |ui| {
            ui.set_max_width(88.0);
            ui.vertical_centered(|ui| {
                ui.add_space(8.0);
                ui.label(egui::RichText::new("Mode").small().weak());
                ui.add_space(4.0);
                for mode in [AppMode::Poly, AppMode::Cad, AppMode::Vector] {
                    let selected = workspace.mode == mode;
                    let text = match mode {
                        AppMode::Poly => "Poly",
                        AppMode::Cad => "CAD",
                        AppMode::Vector => "Vector",
                    };
                    if ui.selectable_label(selected, text).clicked() {
                        workspace.mode = mode;
                    }
                    if mode == AppMode::Cad || mode == AppMode::Vector {
                        if selected {
                            ui.small("(soon)");
                        }
                    }
                }
                ui.add_space(16.0);
                ui.separator();
                ui.add_space(8.0);
                if ui.add_sized([72.0, 28.0], egui::Button::new("⚙ Settings")).clicked() {
                    workspace.show_settings = !workspace.show_settings;
                    workspace.show_help = false;
                }
                if ui.add_sized([72.0, 28.0], egui::Button::new("? Help")).clicked() {
                    workspace.show_help = !workspace.show_help;
                    workspace.show_settings = false;
                }
                if ui.add_sized([72.0, 28.0], egui::Button::new("🖨 Print")).clicked() {
                    workspace.show_print = !workspace.show_print;
                }
            });
        });

        // ── Right properties panel ──
        egui::Panel::right("properties")
            .exact_size(240.0)
            .resizable(false)
            .show(ui, |ui| {
            ui.heading(i18n.t(Msg::Properties));
            ui.separator();

            ui.label(i18n.t(Msg::SelectionMode));
            ui.horizontal(|ui| {
                mode_button(ui, &mut editor.selection_mode, SelectionMode::Vertex, i18n.t(Msg::ModeVertex));
                mode_button(ui, &mut editor.selection_mode, SelectionMode::Edge, i18n.t(Msg::ModeEdge));
                mode_button(ui, &mut editor.selection_mode, SelectionMode::Face, i18n.t(Msg::ModeFace));
                mode_button(ui, &mut editor.selection_mode, SelectionMode::Cell, i18n.t(Msg::ModeCell));
                mode_button(ui, &mut editor.selection_mode, SelectionMode::Object, i18n.t(Msg::ModeObject));
            });

            if let Ok((_mesh, sel)) = selection_q.single() {
                ui.label(i18n.selected_counts(sel.vertices.len(), sel.edges.len(), sel.faces.len()));
            }

            ui.separator();
            ui.label(i18n.t(Msg::GizmoMode));
            ui.horizontal(|ui| {
                if ui.selectable_label(gizmo.mode == GizmoMode::Translate, i18n.t(Msg::ToolMove)).clicked() {
                    gizmo.mode = GizmoMode::Translate;
                    editor.active_tool = Tool::Move;
                }
                if ui.selectable_label(gizmo.mode == GizmoMode::Rotate, i18n.t(Msg::ToolRotate)).clicked() {
                    gizmo.mode = GizmoMode::Rotate;
                    editor.active_tool = Tool::Rotate;
                }
                if ui.selectable_label(gizmo.mode == GizmoMode::Scale, i18n.t(Msg::ToolScale)).clicked() {
                    gizmo.mode = GizmoMode::Scale;
                    editor.active_tool = Tool::Scale;
                }
            });

            ui.separator();
            ui.checkbox(&mut editor.snap_enabled, i18n.t(Msg::Snap));
            if editor.snap_enabled {
                ui.add(egui::Slider::new(&mut editor.snap_value, 0.01..=1.0).text(i18n.t(Msg::SnapValue)));
            }

            ui.separator();
            ui.label(format!("{}: {}", i18n.t(Msg::ToolLabel), i18n.tool_name(editor.active_tool)));
            ui.label(format!("{}: {:?}", i18n.t(Msg::GizmoLabel), gizmo.mode));

            ui.separator();
            ui.label(i18n.t(Msg::Language));
            ui.horizontal(|ui| {
                if ui.selectable_label(i18n.locale == Locale::English, Locale::English.display_name()).clicked() {
                    i18n.set_locale(Locale::English);
                }
            });

            ui.separator();
            ui.label(i18n.t(Msg::EditKeys));
            ui.small(i18n.t(Msg::HintModes));
            ui.small(i18n.t(Msg::HintTransform));
            ui.small(i18n.t(Msg::HintExtrudeInset));
            ui.small(i18n.t(Msg::HintLoopBevel));
            ui.small(i18n.t(Msg::HintMergeDup));
            ui.small(i18n.t(Msg::HintSplitDelete));
            ui.small(i18n.t(Msg::HintEdgeLoop));
            ui.small(i18n.t(Msg::HintLinkedGrow));
            ui.small(i18n.t(Msg::HintUndo));
            ui.small(i18n.t(Msg::HintTriangulate));
            ui.small(i18n.t(Msg::HintSymExport));
        });

        // ── Left toolbar ──
        egui::Panel::left("toolbar")
            .exact_size(72.0)
            .resizable(false)
            .show(ui, |ui| {
            ui.heading(i18n.t(Msg::Tools));
            ui.separator();

            tool_button(ui, &mut editor.active_tool, Tool::Select, i18n.t(Msg::ToolSelect), i18n.t(Msg::TipSelect));
            tool_button(ui, &mut editor.active_tool, Tool::Move, i18n.t(Msg::ToolMove), i18n.t(Msg::TipMove));
            tool_button(ui, &mut editor.active_tool, Tool::Rotate, i18n.t(Msg::ToolRotate), i18n.t(Msg::TipRotate));
            tool_button(ui, &mut editor.active_tool, Tool::Scale, i18n.t(Msg::ToolScale), i18n.t(Msg::TipScale));
            ui.separator();
            tool_button(ui, &mut editor.active_tool, Tool::Extrude, i18n.t(Msg::ToolExtrude), i18n.t(Msg::TipExtrude));
            tool_button(ui, &mut editor.active_tool, Tool::Inset, i18n.t(Msg::ToolInset), i18n.t(Msg::TipInset));
            tool_button(ui, &mut editor.active_tool, Tool::Bevel, i18n.t(Msg::ToolBevel), i18n.t(Msg::TipBevel));
            tool_button(ui, &mut editor.active_tool, Tool::CreatePrimitive, i18n.t(Msg::ToolPrimitive), i18n.t(Msg::TipPrimitive));

            match editor.active_tool {
                Tool::Move => gizmo.mode = GizmoMode::Translate,
                Tool::Rotate => gizmo.mode = GizmoMode::Rotate,
                Tool::Scale => gizmo.mode = GizmoMode::Scale,
                _ => {}
            }
        });

        // ── Status bar (bottom) ──
        egui::Panel::bottom("status_bar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("Mode: {:?}", editor.selection_mode));
                ui.separator();
                ui.label(format!("Tool: {}", i18n.tool_name(editor.active_tool)));
                ui.separator();
                let snap = match editor.snap_mode {
                    1 => "Vert", 2 => "Edge", 3 => "Face", 4 => "Grid", _ => "Off",
                };
                ui.label(format!("Snap: {snap}"));
                if let Ok((mesh, sel)) = selection_q.single() {
                    ui.separator();
                    ui.label(format!(
                        "V {}  E {}  F {}  | sel {}/{}/{}",
                        mesh.vertices.len(),
                        polyforge_core::extract_edges(mesh).len(),
                        mesh.faces.len(),
                        sel.vertices.len(),
                        sel.edges.len(),
                        sel.faces.len(),
                    ));
                }
            });
        });
    });
}

// ============================================================================
// Floating windows
// ============================================================================

fn editor_ui_windows(
    mut contexts: EguiContexts,
    mut i18n: ResMut<I18n>,
    objects_q: Query<(Entity, Option<&ObjectName>), With<EditableObject>>,
    active_q: Query<Entity, With<ActiveMesh>>,
    registry: Res<CommandRegistry>,
    mut materials_table: ResMut<MaterialTable>,
    mut uv_state: ResMut<UvEditorState>,
    mut mesh_uv_q: Query<(&mut EditableMesh, &Selection), With<ActiveMesh>>,
    mut workspace: ResMut<WorkspaceState>,
    mut settings: ResMut<AppSettings>,
    mut commands: Commands,
    mut cad_ws: ResMut<CadWorkspace>,
    mut cad_bodies: Query<(Entity, &mut CadBody, Option<&mut polyforge_core::EditableMesh>)>,
    mut vector_ws: ResMut<VectorWorkspace>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return; };

    // Settings window
    if workspace.show_settings {
        egui::Window::new("Settings")
            .collapsible(false)
            .resizable(true)
            .default_width(320.0)
            .show(ctx, |ui| {
                ui.heading("Preferences");
                ui.separator();
                ui.label("Units");
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut settings.units, Units::Metric, "Metric (mm)");
                    ui.selectable_value(&mut settings.units, Units::Imperial, "Imperial (in)");
                });
                ui.add(egui::Slider::new(&mut settings.orbit_sensitivity, 0.2..=3.0).text("Orbit sensitivity"));
                ui.checkbox(&mut settings.show_grid, "Show grid");
                ui.checkbox(&mut settings.auto_save, "Auto-save (planned)");
                ui.separator();
                ui.label("Language");
                ui.horizontal(|ui| {
                    if ui.button("English").clicked() {
                        i18n.set_locale(Locale::English);
                    }
                });
                if ui.button("Close").clicked() {
                    workspace.show_settings = false;
                }
            });
    }

    // Help window
    if workspace.show_help {
        egui::Window::new("Help")
            .collapsible(false)
            .default_width(360.0)
            .show(ctx, |ui| {
                ui.heading("PolyForge");
                ui.label("GPL-3.0 · Bevy engine");
                ui.separator();
                ui.label("Modes");
                ui.small("PolyMode — polygon mesh (Phase 1–2)");
                ui.small("CadMode — solids / sketcher (Phases 3–20)");
                ui.small("VectorMode — 2D paths (planned)");
                ui.separator();
                ui.label("Essentials");
                ui.small("Tab Edit↔Object · G/R/S transform · 1/2/3 select modes");
                ui.small("Ctrl+S save · Ctrl+Z undo · F8 plugins");
                ui.separator();
                if ui.button("Close").clicked() {
                    workspace.show_help = false;
                }
            });
    }

    // CAD / Vector placeholder banner
    if workspace.mode != AppMode::Poly {
        egui::Area::new(egui::Id::new("mode_banner"))
            .anchor(egui::Align2::CENTER_TOP, [0.0, 40.0])
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "{} — not implemented yet (after Phase 2)",
                        workspace.mode.label()
                    ))
                    .color(egui::Color32::from_rgb(255, 180, 80)),
                );
            });
    }

    // Materials window
    egui::Window::new("Materials")
        .default_width(200.0)
        .show(ctx, |ui| {
            ui.heading("Slots");
            let slot_count = materials_table.slots.len();
            for i in 0..slot_count {
                // Read slot name immutably first
                let name = materials_table.slots[i].name.clone();
                let selected = materials_table.active_slot == i as u32;
                let mut clicked = false;
                ui.horizontal(|ui| {
                    if ui.selectable_label(selected, format!("{i}: {}", name)).clicked() {
                        clicked = true;
                    }
                });
                if clicked {
                    materials_table.active_slot = i as u32;
                }
                // Now mutably borrow the slot for editing
                let slot = &mut materials_table.slots[i];
                let mut col = [
                    slot.base_color.x,
                    slot.base_color.y,
                    slot.base_color.z,
                ];
                ui.horizontal(|ui| {
                    ui.add(egui::Slider::new(&mut col[0], 0.0..=1.0).text("R"));
                    ui.add(egui::Slider::new(&mut col[1], 0.0..=1.0).text("G"));
                    ui.add(egui::Slider::new(&mut col[2], 0.0..=1.0).text("B"));
                });
                slot.base_color.x = col[0];
                slot.base_color.y = col[1];
                slot.base_color.z = col[2];
                ui.add(egui::Slider::new(&mut slot.metallic, 0.0..=1.0).text("Metal"));
                ui.add(egui::Slider::new(&mut slot.roughness, 0.0..=1.0).text("Rough"));
                ui.separator();
            }
            if ui.button("Add slot").clicked() {
                let n = materials_table.slots.len();
                materials_table.slots.push(polyforge_core::MaterialSlot {
                    name: format!("Material.{n}"),
                    ..Default::default()
                });
            }
            if ui.button("Assign active to selection").clicked() {
                if let Ok((mut mesh, sel)) = mesh_uv_q.single_mut() {
                    for &f in &sel.faces {
                        mesh.set_face_material(f, materials_table.active_slot);
                    }
                }
            }
        });

    // Outliner
    egui::Window::new("Outliner")
        .default_width(180.0)
        .show(ctx, |ui| {
            ui.heading("Objects");
            let active = active_q.iter().next();
            for (e, name) in objects_q.iter() {
                let label = name.map(|n| n.0.as_str()).unwrap_or("Object");
                let is_active = active == Some(e);
                if ui.selectable_label(is_active, format!("{label}")).clicked() {
                    for ae in active_q.iter() {
                        commands.entity(ae).remove::<ActiveMesh>();
                    }
                    commands.entity(e).insert(ActiveMesh);
                }
            }
            ui.separator();
            ui.heading("Plugins");
            for cmd in registry.list() {
                ui.small(cmd);
            }
        });

    // UV Editor window
    egui::Window::new("UV Editor")
        .default_width(280.0)
        .default_height(300.0)
        .show(ctx, |ui| {
            ui.label("UV layout — drag to move selected UVs");
            ui.horizontal(|ui| {
                if ui.button("Select all UV").clicked() {
                    if let Ok((mesh, _)) = mesh_uv_q.single() {
                        uv_state.selected = (0..mesh.uvs.len() as u32).collect();
                    }
                }
                if ui.button("Clear UV sel").clicked() { uv_state.clear(); }
                if ui.button("Pack islands").clicked() {
                    if let Ok((mut mesh, _)) = mesh_uv_q.single_mut() {
                        polyforge_uv::pack_uv_islands(&mut mesh, 0.02);
                    }
                }
                if ui.button("Average islands").clicked() {
                    if let Ok((mut mesh, _)) = mesh_uv_q.single_mut() {
                        polyforge_uv::average_uv_islands(&mut mesh, &uv_state.selected);
                    }
                }
                if ui.button("Select island").clicked() {
                    if let Ok((mesh, _)) = mesh_uv_q.single() {
                        uv_state.selected = polyforge_uv::expand_uv_islands(&mesh, &uv_state.selected);
                    }
                }
                if ui.button("Follow active quads").clicked() {
                    if let Ok((mut mesh, sel)) = mesh_uv_q.single_mut() {
                        if let Some(&seed) = sel.faces.iter().next() {
                            polyforge_uv::follow_active_quads(&mut mesh, seed);
                        }
                    }
                }
                if ui.button("Unwrap smart").clicked() {
                    if let Ok((mut mesh, _)) = mesh_uv_q.single_mut() {
                        polyforge_uv::unwrap_smart(&mut mesh);
                    }
                }
            });

            let (response, painter) = ui.allocate_painter(
                egui::vec2(260.0, 240.0),
                egui::Sense::click_and_drag(),
            );
            let rect = response.rect;
            painter.rect_filled(rect, 0.0, egui::Color32::from_gray(28));
            painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, egui::Color32::GRAY), egui::StrokeKind::Inside);

            let to_screen = |uv: bevy::math::Vec2| -> egui::Pos2 {
                egui::pos2(
                    rect.left() + uv.x.clamp(0.0, 1.0) * rect.width(),
                    rect.bottom() - uv.y.clamp(0.0, 1.0) * rect.height(),
                )
            };
            let to_uv = |pos: egui::Pos2| -> bevy::math::Vec2 {
                bevy::math::Vec2::new(
                    ((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0),
                    (1.0 - (pos.y - rect.top()) / rect.height()).clamp(0.0, 1.0),
                )
            };

            if let Ok((mut mesh, sel)) = mesh_uv_q.single_mut() {
                // face edges
                for (fi, face) in mesh.faces.iter().enumerate() {
                    let selected = sel.faces.contains(&(fi as u32));
                    let col = if selected {
                        egui::Color32::from_rgb(255, 180, 60)
                    } else {
                        egui::Color32::from_rgb(100, 140, 200)
                    };
                    let n = face.indices.len();
                    for i in 0..n {
                        let a = face.indices[i] as usize;
                        let b = face.indices[(i + 1) % n] as usize;
                        let ua = mesh.uvs.get(a).copied().unwrap_or(bevy::math::Vec2::ZERO);
                        let ub = mesh.uvs.get(b).copied().unwrap_or(bevy::math::Vec2::ZERO);
                        painter.line_segment(
                            [to_screen(ua), to_screen(ub)],
                            egui::Stroke::new(1.0, col),
                        );
                    }
                }
                // points
                for (i, uv) in mesh.uvs.iter().enumerate() {
                    let selected = uv_state.selected.contains(&(i as u32));
                    let col = if selected {
                        egui::Color32::YELLOW
                    } else {
                        egui::Color32::WHITE
                    };
                    painter.circle_filled(to_screen(*uv), if selected { 3.5 } else { 2.0 }, col);
                }
                // click select
                if response.clicked() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let mut best = None;
                        let mut best_d = 8.0_f32;
                        for (i, uv) in mesh.uvs.iter().enumerate() {
                            let d = to_screen(*uv).distance(pos);
                            if d < best_d {
                                best_d = d;
                                best = Some(i as u32);
                            }
                        }
                        if let Some(i) = best {
                            if ui.input(|i| i.modifiers.shift) {
                                if !uv_state.selected.remove(&i) {
                                    uv_state.selected.insert(i);
                                }
                            } else {
                                uv_state.selected.clear();
                                uv_state.selected.insert(i);
                            }
                        }
                    }
                }
                // drag move
                if response.dragged() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        if let Some(last) = uv_state.last_cursor {
                            let delta = to_uv(pos) - to_uv(egui::pos2(last.x, last.y));
                            for &i in &uv_state.selected {
                                if let Some(uv) = mesh.uvs.get_mut(i as usize) {
                                    *uv = (*uv + delta).clamp(bevy::math::Vec2::ZERO, bevy::math::Vec2::ONE);
                                }
                            }
                        }
                        // Store as bevy::math::Vec2 (convert from egui::Pos2)
                        uv_state.last_cursor = Some(bevy::math::Vec2::new(pos.x, pos.y));
                        uv_state.dragging = true;
                    }
                } else {
                    uv_state.dragging = false;
                    uv_state.last_cursor = None;
                }
            }
        });

    // CAD Mode panels
    if workspace.mode == AppMode::Cad {
        egui::Window::new("CAD Task Panel")
            .default_width(280.0)
            .show(ctx, |ui| {
                ui.heading("Parameters");
                let mut fillet = false;
                if let Some(ae) = cad_ws.active_body {
                    if let Ok((_, mut body, mesh)) = cad_bodies.get_mut(ae) {
                        ui.label(&body.name);
                        let mut apply_idx: Option<usize> = None;
                        let mut apply_vals: Option<Vec<f32>> = None;
                        for fi in 0..body.features.len() {
                            let params = feature_params(&body.features[fi]);
                            ui.separator();
                            ui.label(&params.label);
                            let mut vals: Vec<f32> = params.values.iter().map(|(_, v)| *v).collect();
                            for (i, (name, _)) in params.values.iter().enumerate() {
                                ui.add(egui::Slider::new(&mut vals[i], 0.01..=5.0).text(name));
                            }
                            if ui.button(format!("Apply #{fi}")).clicked() {
                                apply_idx = Some(fi);
                                apply_vals = Some(vals);
                            }
                        }
                        // Apply params after the loop to avoid borrow conflict
                        if let (Some(fi), Some(vals)) = (apply_idx, apply_vals) {
                            apply_feature_params(&mut body.features[fi], &vals);
                            body.dirty = true;
                        }
                        if ui.button("Delete last feature").clicked() {
                            let idx = body.features.len().saturating_sub(1);
                            body.remove_feature_at(idx);
                        }
                        if ui.button("Fillet mesh 0.05").clicked() {
                            fillet = true;
                        }
                        if ui.button("Solve constraints").clicked() {
                            let n = solve_body_sketches(&mut body);
                            bevy::log::info!("Solver adjusted {n} segments");
                        }
                        if ui.button("Solve LSQ (Gauss–Newton)").clicked() {
                            let r = solve_body_sketches_lsq(&mut body);
                            bevy::log::info!("LSQ residual {r:.6}");
                        }
                        if let Some(polyforge_cad::CadFeature::Sketch { curves, constraints, .. }) =
                            body.features.iter().rev().find(|f| matches!(f, polyforge_cad::CadFeature::Sketch { .. }))
                        {
                            let res = constraint_residual(curves, constraints);
                            ui.small(format!("Constraint residual: {res:.5}"));
                        }
                        if let Some(polyforge_cad::CadFeature::Sketch { curves, constraints, .. }) =
                            body.features.iter().rev().find(|f| matches!(f, polyforge_cad::CadFeature::Sketch { .. }))
                        {
                            let dof = estimate_sketch_dof(curves.len(), constraints.len());
                            ui.small(format!("Sketch DOF ≈ {dof} (curves {}, cns {})", curves.len(), constraints.len()));
                        }
                        ui.add(egui::Slider::new(&mut cad_ws.selected_feature, 0..=32).text("Feature #"));
                        ui.add(egui::Slider::new(&mut cad_ws.selected_curve, 0..=16).text("Curve #"));
                        if ui.button("Delete feature #").clicked() {
                            body.remove_feature_at(cad_ws.selected_feature);
                        }
                        let ci = cad_ws.selected_curve;
                        ui.horizontal(|ui| {
                            if ui.button("Add H").clicked() {
                                add_constraint_to_last_sketch(
                                    &mut body,
                                    UserConstraint { kind: UserConstraintKind::Horizontal, a: ci, b: 0, value: 0.0 },
                                );
                            }
                            if ui.button("Add V").clicked() {
                                add_constraint_to_last_sketch(
                                    &mut body,
                                    UserConstraint { kind: UserConstraintKind::Vertical, a: ci, b: 0, value: 0.0 },
                                );
                            }
                            if ui.button("Equal len").clicked() {
                                add_constraint_to_last_sketch(
                                    &mut body,
                                    UserConstraint { kind: UserConstraintKind::EqualLength, a: ci, b: ci.saturating_add(1), value: 0.0 },
                                );
                            }
                        });
                        if ui.button(format!("Fixed len 1.0 on #{ci}")).clicked() {
                            add_constraint_to_last_sketch(
                                &mut body,
                                UserConstraint { kind: UserConstraintKind::FixedLength, a: ci, b: 0, value: 1.0 },
                            );
                        }
                        ui.horizontal(|ui| {
                            if ui.button("Parallel #,#+1").clicked() {
                                add_constraint_to_last_sketch(
                                    &mut body,
                                    UserConstraint { kind: UserConstraintKind::Parallel, a: ci, b: ci.saturating_add(1), value: 0.0 },
                                );
                            }
                            if ui.button("Coincident #→#+1").clicked() {
                                add_constraint_to_last_sketch(
                                    &mut body,
                                    UserConstraint { kind: UserConstraintKind::Coincident, a: ci, b: ci.saturating_add(1), value: 0.0 },
                                );
                            }
                        });
                        if fillet {
                            if let Some(mut mesh) = mesh {
                                mesh_fillet_approximate(&mut mesh, 0.05, 1);
                                body.dirty = false;
                            }
                        }
                    }
                } else {
                    ui.small("Select a body in the tree");
                }
            });
    }

    // VectorMode panel
    if workspace.mode == AppMode::Vector {
        egui::Window::new("VectorMode")
            .default_width(200.0)
            .show(ctx, |ui| {
                ui.heading("VectorMode");
                ui.separator();
                ui.small("2D paths on XY plane");
                if ui.button("Add rectangle path").clicked() {
                    let path = VectorPath::rect(
                        "Rect",
                        bevy::math::Vec2::new(-0.5, -0.3),
                        bevy::math::Vec2::new(0.5, 0.3),
                    ).with_fill();
                    let e = commands.spawn((path, Name::new("VectorRect"))).id();
                    vector_ws.active_path = Some(e);
                }
                if ui.button("Add triangle path").clicked() {
                    let path = VectorPath {
                        name: "Triangle".into(),
                        points: vec![
                            bevy::math::Vec2::new(0.0, 0.5),
                            bevy::math::Vec2::new(-0.5, -0.3),
                            bevy::math::Vec2::new(0.5, -0.3),
                        ],
                        closed: true,
                        beziers: Vec::new(),
                        filled: true,
                        holes: Vec::new(),
                    };
                    let e = commands.spawn((path, Name::new("VectorTri"))).id();
                    vector_ws.active_path = Some(e);
                }
                ui.small("Drawn as cyan gizmos (Y=0 plane)");
                ui.separator();
                ui.checkbox(&mut vector_ws.auto_g1, "Auto G1 on handle drag");
                ui.small("MMB handles · U/J/D/Y · G/M · A toggle auto-G1 · Clear X");
                if ui.button("Add bezier curve").clicked() {
                    use polyforge_vector::BezierSegment;
                    let path = VectorPath {
                        name: "Bezier".into(),
                        points: Vec::new(),
                        closed: false,
                        beziers: vec![BezierSegment {
                            p0: bevy::math::Vec2::new(-0.6, 0.0),
                            p1: bevy::math::Vec2::new(-0.2, 0.6),
                            p2: bevy::math::Vec2::new(0.2, -0.6),
                            p3: bevy::math::Vec2::new(0.6, 0.0),
                        }],
                        filled: false,
                        holes: Vec::new(),
                    };
                    let e = commands.spawn((path, Name::new("VectorBezier"))).id();
                    vector_ws.active_path = Some(e);
                }
                ui.small("Ctrl+Shift+E export SVG");
                ui.small("Ctrl+Shift+I import SVG");
                ui.small("→ artifacts/polyforge_vectors.svg");
            });
    }
}

// ============================================================================
// Marquee overlay
// ============================================================================

fn editor_ui_marquee(
    mut contexts: EguiContexts,
    marquee: Res<MarqueeState>,
) {
    let Ok(ctx) = contexts.ctx_mut() else { return; };
    if let Some(rect) = marquee.rect() {
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("marquee_overlay"),
        ));
        let screen_rect = egui::Rect::from_min_max(
            egui::pos2(rect.min.x, rect.min.y),
            egui::pos2(rect.max.x, rect.max.y),
        );
        painter.rect(
            screen_rect,
            0.0,
            egui::Color32::from_rgba_unmultiplied(80, 140, 255, 40),
            egui::Stroke::new(1.5, egui::Color32::from_rgb(80, 140, 255)),
            egui::StrokeKind::Inside,
        );
    }
}

// ============================================================================
// Helper functions
// ============================================================================

fn tool_button(
    ui: &mut egui::Ui,
    current: &mut Tool,
    tool: Tool,
    label: &str,
    tooltip: &str,
) {
    let selected = *current == tool;
    let response = ui.selectable_label(selected, label);
    if response.clicked() {
        *current = tool;
    }
    response.on_hover_text(tooltip);
}

fn mode_button(
    ui: &mut egui::Ui,
    current: &mut SelectionMode,
    mode: SelectionMode,
    label: &str,
) {
    let selected = *current == mode;
    if ui.selectable_label(selected, label).clicked() {
        *current = mode;
    }
}