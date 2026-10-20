//! # polyforge-ui
//!
//! Immediate-mode UI for the editor using bevy_egui.
//! All user-visible strings go through [`polyforge_core::I18n`].

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiPrimaryContextPass};
use polyforge_cad::{ActiveKernel, AssemblyMate, MateKind, sort_by_hierarchy, 
    CadBody, CadWorkspace, spawn_cad_box, spawn_cad_pad_rect, feature_params, apply_feature_params,
    mesh_fillet_approximate, SketchToolState, SketchPlane, solve_body_sketches, solve_body_sketches_lsq, estimate_sketch_dof, constraint_residual,
    add_constraint_to_last_sketch, UserConstraint, UserConstraintKind,
};
use polyforge_vector::{VectorPath, VectorWorkspace, union_paths, intersect_paths, difference_paths};
use polyforge_bridge::{ArtCraftIntegration, launch_vectorcraft, launch_photocraft};
use polyforge_paint::{BlendMode as PaintBlendMode, PaintState, PaintTool, Rgba};
use polyforge_core::{
    ActiveMesh, EditableMesh, EditableObject, EditorState, GizmoMode, GizmoState, I18n, Locale,
    MarqueeState, Msg, ObjectName, Selection, SelectionMode, Tool, CommandRegistry,
    MaterialTable, UvEditorState, extract_edges,
    WorkspaceState, AppMode, AppSettings, Units, CommandLineState,
};

pub struct PolyForgeUiPlugin;

impl Plugin for PolyForgeUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EguiPlugin::default())
            .add_systems(
                EguiPrimaryContextPass,
                (editor_ui_system, editor_windows_system),
            )
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


/// Draw a mode icon matching `assets/icons/mode_*.svg` (vector paths, not emoji).
fn paint_mode_icon(painter: &egui::Painter, rect: egui::Rect, mode: AppMode, color: egui::Color32) {
    let s = rect.width().min(rect.height());
    let c = rect.center();
    let stroke = egui::Stroke::new((s * 0.08).clamp(1.2, 2.0), color);
    let to = |x: f32, y: f32| {
        // SVG viewBox 0..24 → rect
        egui::pos2(
            rect.min.x + (x / 24.0) * s + (rect.width() - s) * 0.5,
            rect.min.y + (y / 24.0) * s + (rect.height() - s) * 0.5,
        )
    };
    match mode {
        AppMode::Poly => {
            // Cube wireframe (mode_poly.svg)
            let pts = [
                to(12.0, 3.0),
                to(20.0, 7.5),
                to(20.0, 16.5),
                to(12.0, 21.0),
                to(4.0, 16.5),
                to(4.0, 7.5),
            ];
            painter.line_segment([pts[0], pts[1]], stroke);
            painter.line_segment([pts[1], pts[2]], stroke);
            painter.line_segment([pts[2], pts[3]], stroke);
            painter.line_segment([pts[3], pts[4]], stroke);
            painter.line_segment([pts[4], pts[5]], stroke);
            painter.line_segment([pts[5], pts[0]], stroke);
            let mid = to(12.0, 12.0);
            painter.line_segment([pts[0], mid], stroke);
            painter.line_segment([mid, pts[3]], stroke);
            painter.line_segment([mid, pts[1]], stroke);
            painter.line_segment([mid, pts[5]], stroke);
        }
        AppMode::Cad => {
            // Set square + circle (mode_cad.svg)
            painter.line_segment([to(5.0, 5.0), to(5.0, 19.0)], stroke);
            painter.line_segment([to(5.0, 19.0), to(19.0, 19.0)], stroke);
            painter.line_segment([to(5.0, 5.0), to(19.0, 19.0)], stroke);
            painter.circle_stroke(to(16.5, 7.5), s * 0.09, stroke);
        }
        AppMode::Paint => {
            // Brush diamond (mode_paint.svg)
            painter.line_segment([to(14.5, 4.5), to(19.5, 9.5)], stroke);
            painter.line_segment([to(19.5, 9.5), to(10.0, 19.0)], stroke);
            painter.line_segment([to(10.0, 19.0), to(5.0, 19.0)], stroke);
            painter.line_segment([to(5.0, 19.0), to(5.0, 14.0)], stroke);
            painter.line_segment([to(5.0, 14.0), to(14.5, 4.5)], stroke);
            painter.line_segment([to(12.5, 6.5), to(17.5, 11.5)], stroke);
        }
        AppMode::Vector => {
            // Pen / Bézier (mode_vector.svg)
            let p0 = to(5.0, 17.0);
            let p1 = to(12.0, 8.0);
            let p2 = to(19.0, 17.0);
            // quadratic-ish polyline
            painter.line_segment([p0, to(8.0, 11.0)], stroke);
            painter.line_segment([to(8.0, 11.0), p1], stroke);
            painter.line_segment([p1, to(16.0, 11.0)], stroke);
            painter.line_segment([to(16.0, 11.0), p2], stroke);
            let r = s * 0.08;
            painter.circle_filled(p0, r, color);
            painter.circle_filled(p1, r, color);
            painter.circle_filled(p2, r, color);
            painter.line_segment([p1, to(12.0, 4.0)], stroke);
            painter.circle_stroke(to(12.0, 3.5), s * 0.05, stroke);
        }
    }
}

fn mode_icon_button(ui: &mut egui::Ui, mode: AppMode, selected: bool, accent: egui::Color32) -> egui::Response {
    let size = egui::vec2(56.0, 28.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let bg = if selected {
        accent
    } else if resp.hovered() {
        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 20)
    } else {
        egui::Color32::from_rgba_unmultiplied(0, 0, 0, 40)
    };
    ui.painter().rect_filled(rect, 4.0, bg);
    // icon box left
    let icon_rect = egui::Rect::from_min_size(
        egui::pos2(rect.min.x + 4.0, rect.min.y + 4.0),
        egui::vec2(20.0, 20.0),
    );
    let icon_color = if selected {
        egui::Color32::WHITE
    } else {
        match mode {
            AppMode::Poly => egui::Color32::from_rgb(126, 184, 255),
            AppMode::Cad => egui::Color32::from_rgb(94, 224, 200),
            AppMode::Paint => egui::Color32::from_rgb(201, 160, 255),
            AppMode::Vector => egui::Color32::from_rgb(232, 192, 96),
        }
    };
    paint_mode_icon(ui.painter(), icon_rect, mode, icon_color);
    // label
    let text_pos = egui::pos2(rect.min.x + 26.0, rect.center().y);
    ui.painter().text(
        text_pos,
        egui::Align2::LEFT_CENTER,
        mode.short(),
        egui::FontId::proportional(12.5),
        if selected {
            egui::Color32::WHITE
        } else {
            egui::Color32::from_rgb(200, 200, 210)
        },
    );
    resp
}

#[derive(bevy::ecs::system::SystemParam)]
struct EditorUiParams<'w, 's> {
    contexts: EguiContexts<'w, 's>,
    editor: ResMut<'w, EditorState>,
    gizmo: ResMut<'w, GizmoState>,
    i18n: ResMut<'w, I18n>,
    marquee: Res<'w, MarqueeState>,
    selection_q: Query<
        'w,
        's,
        (&'static EditableMesh, &'static Selection),
        (With<ActiveMesh>, Without<CadBody>),
    >,
    workspace: ResMut<'w, WorkspaceState>,
    settings: ResMut<'w, AppSettings>,
    commands: Commands<'w, 's>,
    cad_ws: ResMut<'w, CadWorkspace>,
    cad_bodies: Query<
        'w,
        's,
        (
            Entity,
            &'static mut CadBody,
            Option<&'static mut polyforge_core::EditableMesh>,
        ),
    >,
    meshes: ResMut<'w, Assets<Mesh>>,
    std_materials: ResMut<'w, Assets<StandardMaterial>>,
    vector_ws: ResMut<'w, VectorWorkspace>,
    sketch_tool: ResMut<'w, SketchToolState>,
    cmdline: ResMut<'w, CommandLineState>,
    artcraft: Res<'w, ArtCraftIntegration>,
    mates_q: Query<'w, 's, (Entity, &'static mut AssemblyMate)>,
    names_q: Query<'w, 's, &'static polyforge_core::ObjectName>,
    paint: ResMut<'w, PaintState>,
    preview_tex: Local<'s, Option<egui::TextureHandle>>,
    preview_rev: Local<'s, u64>,
}

fn editor_ui_system(params: EditorUiParams) {
    let EditorUiParams {
        mut contexts,
        mut editor,
        mut gizmo,
        mut i18n,
        marquee,
        selection_q,
        mut workspace,
        mut settings,
        mut commands,
        mut cad_ws,
        mut cad_bodies,
        mut meshes,
        mut std_materials,
        mut vector_ws,
        mut sketch_tool,
        mut cmdline,
        artcraft,
        mut mates_q,
        names_q,
        mut paint,
        mut preview_tex,
        mut preview_rev,
    } = params;

    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    let mut viewport_ui = egui::Ui::new(
        ctx.clone(),
        egui::Id::new("polyforge_viewport"),
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::background())
            .max_rect(ctx.viewport_rect()),
    );

    // ── Marquee rectangle overlay ──
    if let Some(rect) = marquee.rect() {
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("marquee_overlay"),
        ));
        let screen_rect = egui::Rect::from_min_max(
            egui::pos2(rect.min.x, rect.min.y),
            egui::pos2(rect.max.x, rect.max.y),
        );
        painter.rect_filled(
            screen_rect,
            0.0,
            egui::Color32::from_rgba_unmultiplied(80, 140, 255, 40),
        );
        painter.rect_stroke(
            screen_rect,
            0.0,
            egui::Stroke::new(1.5, egui::Color32::from_rgb(80, 140, 255)),
            egui::StrokeKind::Inside,
        );
    }


    // Settings window
    if workspace.show_settings {
        egui::Window::new("Settings")
            .collapsible(false)
            .resizable(true)
            .default_width(320.0)
            .show(ctx, |ui| {
                ui.heading("Preferences");
                ui.separator();
                ui.label("ArtCraft companions");
                if ui.button("Open PhotoCraft…").clicked() {
                    if let Err(e) = launch_photocraft(&artcraft.vendor_root) {
                        bevy::log::warn!("{e}");
                    }
                }
                if ui.button("Open VectorCraft…").clicked() {
                    if let Err(e) = launch_vectorcraft(&artcraft.vendor_root) {
                        bevy::log::warn!("{e}");
                    }
                }
                ui.small("Sync: ./scripts/sync_artcraft.sh");
                ui.separator();
                ui.label("Units");
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut settings.units, Units::Metric, "Metric (mm)");
                    ui.selectable_value(&mut settings.units, Units::Imperial, "Imperial (in)");
                });
                ui.add(
                    egui::Slider::new(&mut settings.orbit_sensitivity, 0.2..=3.0)
                        .text("Orbit sensitivity"),
                );
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
                ui.label("Three workspaces, three UX targets:");
                ui.small("• Poly → Blender mesh modeling");
                ui.small("• CAD → CadCraft (command line, feature tree, 3D ribbon)");
                ui.small("• Paint → PhotoCraft (textures / raster)");
                ui.small("• Vector → VectorCraft (pen, pathfinder, layers)");
                ui.small("Titlebar: mode · ribbon · ? · ⚙");
                ui.separator();
                ui.label("ArtCraft upstream (auto-synced)");
                ui.small(format!(
                    "VectorCraft pin: {} ({})",
                    if artcraft.vectorcraft.pin_sha.len() >= 12 { &artcraft.vectorcraft.pin_sha[..12] } else { "run sync" },
                    artcraft.vectorcraft.pin_ref
                ));
                ui.small(format!(
                    "PhotoCraft pin: {} ({})",
                    if artcraft.photocraft.pin_sha.len() >= 12 { &artcraft.photocraft.pin_sha[..12] } else { "run sync" },
                    artcraft.photocraft.pin_ref
                ));
                ui.small("./scripts/sync_artcraft.sh");
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


    // CadMode panels
    if workspace.mode == AppMode::Cad {
        egui::Panel::right("cad_tree")
            .default_size(240.0)
            .show(&mut viewport_ui, |ui| {
                ui.heading("Model Tree");
                ui.small("CadCraft feature history");
                ui.separator();
                for (e, body, _) in cad_bodies.iter() {
                    let active = cad_ws.active_body == Some(e);
                    if ui.selectable_label(active, &body.name).clicked() {
                        cad_ws.active_body = Some(e);
                    }
                    for f in &body.features {
                        ui.small(format!("  • {}", f.label()));
                    }
                }
                ui.separator();
                ui.label("Create");
                if ui.button("Box body").clicked() {
                    spawn_cad_box(
                        &mut commands,
                        &mut meshes,
                        &mut std_materials,
                        &mut cad_ws,
                        "Box",
                        1.0,
                        1.0,
                        1.0,
                    );
                }
                if ui.button("Pad rectangle").clicked() {
                    spawn_cad_pad_rect(
                        &mut commands,
                        &mut meshes,
                        &mut std_materials,
                        &mut cad_ws,
                        "Pad",
                        1.0,
                        0.8,
                        0.4,
                    );
                }
                ui.separator();
                ui.label("Sketcher");
                let sk_label = if sketch_tool.active { "Sketch: ON (Enter commit)" } else { "Sketch: OFF (S)" };
                if ui.button(sk_label).clicked() {
                    sketch_tool.active = !sketch_tool.active;
                    if !sketch_tool.active {
                        sketch_tool.clear();
                    }
                }
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut sketch_tool.plane, SketchPlane::XY, "XY");
                    ui.selectable_value(&mut sketch_tool.plane, SketchPlane::XZ, "XZ");
                    ui.selectable_value(&mut sketch_tool.plane, SketchPlane::YZ, "YZ");
                });
                ui.small("Click points · H/V auto-snap · close near start");
                ui.separator();
                ui.label("Assembly tree (click source, then target)");
                {
                    let body_list: Vec<_> = cad_bodies.iter().map(|(e, b, _)| (e, b.name.clone())).collect();
                    let mate_pairs: Vec<_> = mates_q.iter().map(|(e, m)| (e, m.parent)).collect();
                    let ents: Vec<_> = body_list.iter().map(|(e, _)| *e).collect();
                    let ordered = sort_by_hierarchy(&ents, &mate_pairs);
                    // Two-click reparent: first click = child, second = parent
                    ui.ctx().data_mut(|d| {
                        let _ = d.get_temp_mut_or_default::<Option<Entity>>(egui::Id::new("asm_drag_src"));
                    });
                    let mut pending_parent: Option<Entity> = None;
                    let mut set_src: Option<Entity> = None;
                    let mut clear_src = false;
                    for (e, depth) in ordered {
                        let name = body_list
                            .iter()
                            .find(|(be, _)| *be == e)
                            .map(|(_, n)| n.as_str())
                            .unwrap_or("?");
                        let indent = "  ".repeat(depth as usize);
                        let branch = if depth > 0 { "└ " } else { "• " };
                        let src = ui.ctx().data(|d| {
                            d.get_temp::<Option<Entity>>(egui::Id::new("asm_drag_src"))
                                .and_then(|x| x)
                        });
                        let is_src = src == Some(e);
                        let label = format!(
                            "{indent}{branch}{name}  (d{depth}){}",
                            if is_src { "  [src]" } else { "" }
                        );
                        let resp = ui.selectable_label(cad_ws.active_body == Some(e) || is_src, label);
                        if resp.clicked() {
                            cad_ws.active_body = Some(e);
                            if let Some(child) = src {
                                if child != e {
                                    pending_parent = Some(e);
                                } else {
                                    clear_src = true;
                                }
                            } else {
                                set_src = Some(e);
                            }
                        }
                        if resp.secondary_clicked() {
                            clear_src = true;
                        }
                        for (me, m) in mates_q.iter() {
                            if me == e {
                                let pname = names_q.get(m.parent).map(|n| n.0.as_str()).unwrap_or("?");
                                ui.small(format!("{indent}    ↳ {pname}"));
                            }
                        }
                    }
                    if clear_src {
                        ui.ctx().data_mut(|d| {
                            d.insert_temp(egui::Id::new("asm_drag_src"), None::<Entity>);
                        });
                    } else if let Some(s) = set_src {
                        ui.ctx().data_mut(|d| {
                            d.insert_temp(egui::Id::new("asm_drag_src"), Some(s));
                        });
                    }
                    if let Some(parent) = pending_parent {
                        let child = ui.ctx().data(|d| {
                            d.get_temp::<Option<Entity>>(egui::Id::new("asm_drag_src"))
                                .and_then(|x| x)
                        });
                        if let Some(child) = child {
                            let mut cyclic = false;
                            let mut cur = parent;
                            for _ in 0..64 {
                                if cur == child {
                                    cyclic = true;
                                    break;
                                }
                                if let Some((_, m)) = mates_q.iter().find(|(me, _)| *me == cur) {
                                    cur = m.parent;
                                } else {
                                    break;
                                }
                            }
                            if !cyclic {
                                commands.entity(child).insert(AssemblyMate {
                                    parent,
                                    kind: MateKind::CoincidentOrigin,
                                    offset: bevy::math::Vec3::ZERO,
                                });
                                bevy::log::info!("Reparent {child:?} → {parent:?}");
                            }
                            ui.ctx().data_mut(|d| {
                                d.insert_temp(egui::Id::new("asm_drag_src"), None::<Entity>);
                            });
                        }
                    }
                }
                ui.small("Click body = source, click another = parent (right-click clears)");
                if ui.button("Mate: coincident to previous").clicked() {
                    let ents: Vec<_> = cad_bodies.iter().map(|(e, _, _)| e).collect();
                    if ents.len() >= 2 {
                        let parent = ents[ents.len() - 2];
                        let child = ents[ents.len() - 1];
                        commands.entity(child).insert(AssemblyMate {
                            parent,
                            kind: MateKind::CoincidentOrigin,
                            offset: bevy::math::Vec3::ZERO,
                        });
                    }
                }
                if ui.button("Mate: offset Y+0.5").clicked() {
                    let ents: Vec<_> = cad_bodies.iter().map(|(e, _, _)| e).collect();
                    if ents.len() >= 2 {
                        let parent = ents[ents.len() - 2];
                        let child = ents[ents.len() - 1];
                        commands.entity(child).insert(AssemblyMate {
                            parent,
                            kind: MateKind::OffsetAxis,
                            offset: bevy::math::Vec3::new(0.0, 0.5, 0.0),
                        });
                    }
                }
                for (e, mut mate) in mates_q.iter_mut() {
                    ui.horizontal(|ui| {
                        ui.label(format!("{e:?}"));
                        ui.add(egui::DragValue::new(&mut mate.offset.x).speed(0.01).prefix("X "));
                        ui.add(egui::DragValue::new(&mut mate.offset.y).speed(0.01).prefix("Y "));
                        ui.add(egui::DragValue::new(&mut mate.offset.z).speed(0.01).prefix("Z "));
                    });
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut mate.kind, MateKind::CoincidentOrigin, "Coincident");
                        ui.selectable_value(&mut mate.kind, MateKind::OffsetAxis, "OffsetAxis");
                    });
                    if ui.button(format!("Remove mate {e:?}")).clicked() {
                        commands.entity(e).remove::<AssemblyMate>();
                    }
                }
                ui.separator();
                ui.label("Reparent");
                if ui.button("Reparent active → previous body").clicked() {
                    if let Some(child) = cad_ws.active_body {
                        let ents: Vec<_> = cad_bodies.iter().map(|(e, _, _)| e).collect();
                        if let Some(pos) = ents.iter().position(|e| *e == child) {
                            if pos > 0 {
                                let parent = ents[pos - 1];
                                if parent != child {
                                    commands.entity(child).insert(AssemblyMate {
                                        parent,
                                        kind: MateKind::CoincidentOrigin,
                                        offset: bevy::math::Vec3::ZERO,
                                    });
                                }
                            }
                        }
                    }
                }
                if ui.button("Reparent active → first body").clicked() {
                    if let Some(child) = cad_ws.active_body {
                        let ents: Vec<_> = cad_bodies.iter().map(|(e, _, _)| e).collect();
                        if let Some(&parent) = ents.first() {
                            if parent != child {
                                commands.entity(child).insert(AssemblyMate {
                                    parent,
                                    kind: MateKind::OffsetAxis,
                                    offset: bevy::math::Vec3::new(0.0, 0.25, 0.0),
                                });
                            }
                        }
                    }
                }

            });

        // Task panel
        egui::Window::new("CAD Task Panel")
            .default_width(280.0)
            .show(ctx, |ui| {
                ui.heading("Parameters");
                let mut fillet = false;
                if let Some(ae) = cad_ws.active_body {
                    if let Ok((_, mut body, mesh)) = cad_bodies.get_mut(ae) {
                        ui.label(&body.name);
                        for fi in 0..body.features.len() {
                            let params = feature_params(&body.features[fi]);
                            ui.separator();
                            ui.label(&params.label);
                            let mut vals: Vec<f32> = params.values.iter().map(|(_, v)| *v).collect();
                            for (i, (name, _)) in params.values.iter().enumerate() {
                                ui.add(egui::Slider::new(&mut vals[i], 0.01..=5.0).text(name));
                            }
                            if ui.button(format!("Apply #{fi}")).clicked() {
                                apply_feature_params(&mut body.features[fi], &vals);
                                body.dirty = true;
                            }
                        }
                        if ui.button("Delete last feature").clicked() {
                            let last = body.features.len().saturating_sub(1);
                            body.remove_feature_at(last);
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
                        // DOF estimate on last sketch
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
                                    UserConstraint {
                                        kind: UserConstraintKind::Horizontal,
                                        a: ci,
                                        b: 0,
                                        value: 0.0,
                                    },
                                );
                            }
                            if ui.button("Add V").clicked() {
                                add_constraint_to_last_sketch(
                                    &mut body,
                                    UserConstraint {
                                        kind: UserConstraintKind::Vertical,
                                        a: ci,
                                        b: 0,
                                        value: 0.0,
                                    },
                                );
                            }
                            if ui.button("Equal len").clicked() {
                                add_constraint_to_last_sketch(
                                    &mut body,
                                    UserConstraint {
                                        kind: UserConstraintKind::EqualLength,
                                        a: ci,
                                        b: ci.saturating_add(1),
                                        value: 0.0,
                                    },
                                );
                            }
                        });
                        if ui.button(format!("Fixed len 1.0 on #{ci}")).clicked() {
                            add_constraint_to_last_sketch(
                                &mut body,
                                UserConstraint {
                                    kind: UserConstraintKind::FixedLength,
                                    a: ci,
                                    b: 0,
                                    value: 1.0,
                                },
                            );
                        }
                        ui.horizontal(|ui| {
                            if ui.button("Parallel #,#+1").clicked() {
                                add_constraint_to_last_sketch(
                                    &mut body,
                                    UserConstraint {
                                        kind: UserConstraintKind::Parallel,
                                        a: ci,
                                        b: ci.saturating_add(1),
                                        value: 0.0,
                                    },
                                );
                            }
                            if ui.button("Coincident #→#+1").clicked() {
                                add_constraint_to_last_sketch(
                                    &mut body,
                                    UserConstraint {
                                        kind: UserConstraintKind::Coincident,
                                        a: ci,
                                        b: ci.saturating_add(1),
                                        value: 0.0,
                                    },
                                );
                            }
                        });
                        if fillet {
                            if let Some(mut mesh) = mesh {
                                mesh_fillet_approximate(&mut mesh, 0.05, 1);
                                body.dirty = false; // mesh edited directly
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
        egui::Panel::right("vector_panel")
            .default_size(200.0)
            .show(&mut viewport_ui, |ui| {
                ui.heading("Layers & Appearance");
                ui.small("VectorCraft-style panels");
                ui.separator();
                ui.label("Artboard · XY plane");
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
                ui.separator();
                ui.small("[ ] cycle path · Del delete active");
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
                ui.separator();
                ui.label("Upstream VectorCraft");
                if ui.button("Open VectorCraft…").on_hover_text("Launch vendored or PATH binary").clicked() {
                    if let Err(e) = launch_vectorcraft(&artcraft.vendor_root) {
                        bevy::log::warn!("{e}");
                    }
                }
                ui.small(if artcraft.vectorcraft_available { "vendor/vectorcraft present" } else { "not vendored — run sync" });
            });
    }


    // ── Top menu bar
    // ── Top menu bar ──


        // Status bar

    // Paint mode panel (PhotoCraft companion)
    if workspace.mode == AppMode::Paint {
        egui::Panel::right("paint_panel")
            .default_size(220.0)
            .show(&mut viewport_ui, |ui| {
                ui.heading("Paint");
                ui.small("PhotoCraft companion");
                ui.separator();

                // Live composite preview
                if *preview_rev != paint.revision || preview_tex.is_none() {
                    let (w, h) = paint.canvas_size();
                    let rgba = paint.composite_rgba();
                    let image =
                        egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
                    *preview_tex = Some(ui.ctx().load_texture(
                        "polyforge_paint_preview",
                        image,
                        egui::TextureOptions::NEAREST,
                    ));
                    *preview_rev = paint.revision;
                }
                if let Some(tex) = preview_tex.as_ref() {
                    let aspect =
                        paint.document.height as f32 / paint.document.width.max(1) as f32;
                    let width = ui.available_width();
                    let size = egui::vec2(width, (width * aspect).max(1.0));
                    ui.add(egui::Image::new(egui::load::SizedTexture::new(
                        tex.id(),
                        size,
                    )));
                }
                ui.separator();

                // Tools
                ui.horizontal_wrapped(|ui| {
                    for tool in PaintTool::all() {
                        if ui
                            .selectable_label(paint.tool == tool, tool.label())
                            .clicked()
                        {
                            paint.tool = tool;
                        }
                    }
                });

                ui.separator();
                ui.label("Brush");
                ui.add(egui::Slider::new(&mut paint.brush.size, 1.0..=256.0).text("Size"));
                ui.add(egui::Slider::new(&mut paint.brush.opacity, 0.0..=1.0).text("Opacity"));
                ui.add(egui::Slider::new(&mut paint.brush.flow, 0.0..=1.0).text("Flow"));
                ui.add(egui::Slider::new(&mut paint.brush.hardness, 0.0..=1.0).text("Hardness"));
                ui.add(egui::Slider::new(&mut paint.brush.spacing, 0.05..=1.0).text("Spacing"));

                // Colour
                ui.horizontal(|ui| {
                    ui.label("Colour");
                    let mut rgb = paint.brush.color.to_rgb_f32();
                    if ui.color_edit_button_rgb(&mut rgb).changed() {
                        paint.brush.color = Rgba::from_rgb_f32(rgb[0], rgb[1], rgb[2]);
                    }
                });

                // Blend mode
                egui::ComboBox::from_label("Blend")
                    .selected_text(paint.blend.label())
                    .show_ui(ui, |ui| {
                        for blend in [
                            PaintBlendMode::Normal,
                            PaintBlendMode::Multiply,
                            PaintBlendMode::Add,
                            PaintBlendMode::Screen,
                        ] {
                            ui.selectable_value(&mut paint.blend, blend, blend.label());
                        }
                    });

                ui.separator();
                ui.label(format!(
                    "Canvas {}x{}",
                    paint.document.width, paint.document.height
                ));
                ui.horizontal(|ui| {
                    if ui.button("512").clicked() {
                        paint.reset(512, 512);
                    }
                    if ui.button("1024").clicked() {
                        paint.reset(1024, 1024);
                    }
                });
                ui.horizontal(|ui| {
                    if ui.button("Clear layer").clicked() {
                        if let Some(layer) = paint.document.active_layer_mut() {
                            layer.pixels.fill(Rgba::TRANSPARENT);
                        }
                        paint.dirty = true;
                        paint.revision += 1;
                    }
                    if ui.button("Fill").clicked() {
                        paint.fill_active();
                    }
                });

                ui.separator();
                ui.label("Layers");
                let layer_count = paint.document.layers.len();
                for i in 0..layer_count {
                    ui.horizontal(|ui| {
                        let active = paint.document.active == i;
                        let name = paint.document.layers[i].name.clone();
                        if ui.selectable_label(active, name).clicked() {
                            paint.document.active = i;
                        }
                        let mut visible = paint.document.layers[i].visible;
                        if ui.checkbox(&mut visible, "eye").changed() {
                            paint.document.layers[i].visible = visible;
                            paint.dirty = true;
                            paint.revision += 1;
                        }
                    });
                }
                ui.horizontal(|ui| {
                    if ui.button("+").on_hover_text("Add layer").clicked() {
                        paint.add_layer();
                    }
                    if ui.button("-").on_hover_text("Delete layer").clicked() {
                        paint.remove_active_layer();
                    }
                    if ui.button("^").on_hover_text("Move up").clicked() {
                        paint.document.move_active_up();
                        paint.dirty = true;
                        paint.revision += 1;
                    }
                    if ui.button("v").on_hover_text("Move down").clicked() {
                        paint.document.move_active_down();
                        paint.dirty = true;
                        paint.revision += 1;
                    }
                });
                let active_layer = paint.document.active;
                if let Some(layer) = paint.document.layers.get_mut(active_layer) {
                    if ui
                        .add(egui::Slider::new(&mut layer.opacity, 0.0..=1.0).text("Layer alpha"))
                        .changed()
                    {
                        paint.dirty = true;
                        paint.revision += 1;
                    }
                }

                ui.separator();
                if ui.button("Save PNG").clicked() {
                    let path = std::path::Path::new("artifacts/polyforge_paint.png");
                    if let Some(dir) = path.parent() {
                        let _ = std::fs::create_dir_all(dir);
                    }
                    match paint.save_png(path) {
                        Ok(()) => bevy::log::info!("Paint saved -> {}", path.display()),
                        Err(e) => bevy::log::warn!("Paint save failed: {e}"),
                    }
                }
                ui.separator();
                if ui.button("Open PhotoCraft…").clicked() {
                    if let Err(e) = launch_photocraft(&artcraft.vendor_root) {
                        bevy::log::warn!("{e}");
                    }
                }
                ui.small(if artcraft.photocraft_available {
                    "vendor/photocraft present"
                } else {
                    "not vendored — run sync"
                });
            });
    }

    // Status bar — Blender / CadCraft / VectorCraft aware
    egui::Panel::bottom("status_bar").show(&mut viewport_ui, |ui| {
        ui.horizontal(|ui| {
            ui.strong(workspace.mode.label());
            ui.separator();
            match workspace.mode {
                AppMode::Poly => {
                    ui.label(format!("Sel: {:?}", editor.selection_mode));
                    ui.separator();
                    ui.label(format!("Tool: {}", i18n.tool_name(editor.active_tool)));
                    ui.separator();
                    let snap = match editor.snap_mode {
                        1 => "Vert",
                        2 => "Edge",
                        3 => "Face",
                        4 => "Grid",
                        _ => "Off",
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
                }
                AppMode::Cad => {
                    ui.label(&cmdline.message);
                    if let Some(e) = cad_ws.active_body {
                        if let Ok((_, body, _)) = cad_bodies.get(e) {
                            ui.separator();
                            ui.label(format!("{} · {} features", body.name, body.features.len()));
                        }
                    }
                    ui.separator();
                    ui.small("cmd: BOX CYL EXT SKETCH PAD SOLVE");
                }
                AppMode::Paint => {
                    ui.label("Paint · PhotoCraft");
                    ui.separator();
                    ui.small("Textures · raster · Open PhotoCraft from ribbon");
                }
                AppMode::Vector => {
                    ui.label(if vector_ws.auto_g1 { "Auto-G1 ON" } else { "Auto-G1 OFF" });
                    ui.separator();
                    ui.small("Pen · Pathfinder U/J/D/Y · SVG Ctrl+Shift+E/I · MMB handles");
                }
            }
        });
    });

    // CadCraft command line (above status when in CAD)
    if workspace.mode == AppMode::Cad {
        egui::Panel::bottom("cad_cmdline")
            .exact_size(36.0)
            .show(&mut viewport_ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Command:").strong());
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut cmdline.input)
                            .desired_width(320.0)
                            .hint_text("BOX / CYL / EXT / SKETCH / PAD / SOLVE  (Enter)"),
                    );
                    let submit = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))
                        || ui.button("Run").clicked();
                    if submit {
                        let cmd = cmdline.input.trim().to_uppercase();
                        if !cmd.is_empty() {
                            cmdline.history.push(cmd.clone());
                            cmdline.last_command = cmd.clone();
                            match cmd.as_str() {
                                "BOX" | "B" => {
                                    spawn_cad_box(
                                        &mut commands,
                                        &mut meshes,
                                        &mut std_materials,
                                        &mut cad_ws,
                                        "Box",
                                        1.0, 1.0, 1.0,
                                    );
                                    cmdline.message = "BOX created".into();
                                }
                                "CYL" | "CYLINDER" => {
                                    spawn_cad_box(
                                        &mut commands,
                                        &mut meshes,
                                        &mut std_materials,
                                        &mut cad_ws,
                                        "Cyl",
                                        0.01, 0.01, 0.01,
                                    );
                                    cmdline.message = "CYL placeholder body (use C hotkey for cylinder feature)".into();
                                }
                                "PAD" | "RECT" => {
                                    spawn_cad_pad_rect(
                                        &mut commands,
                                        &mut meshes,
                                        &mut std_materials,
                                        &mut cad_ws,
                                        "Pad",
                                        1.0, 0.8, 0.4,
                                    );
                                    cmdline.message = "PAD rectangle body".into();
                                }
                                "SKETCH" | "S" => {
                                    sketch_tool.active = true;
                                    cmdline.message = "Sketch tool ON — click points, Enter to pad".into();
                                }
                                "SOLVE" => {
                                    if let Some(e) = cad_ws.active_body {
                                        if let Ok((_, mut body, _)) = cad_bodies.get_mut(e) {
                                            let n = solve_body_sketches(&mut body);
                                            let r = solve_body_sketches_lsq(&mut body);
                                            cmdline.message = format!("SOLVE adj={n} LSQ residual={r:.5}");
                                        }
                                    } else {
                                        cmdline.message = "No active body".into();
                                    }
                                }
                                "EXT" | "EXTRUDE" => {
                                    cmdline.message = "EXT: use sketch + Enter pad, or task panel".into();
                                }
                                _ => {
                                    cmdline.message = format!("Unknown command: {cmd}");
                                }
                            }
                            cmdline.input.clear();
                        }
                    }
                    if ui.button("⋯").on_hover_text("Repeat last").clicked() {
                        cmdline.input = cmdline.last_command.clone();
                    }
                });
            });
    }


    // ═══════════════════════════════════════════════════════════════
    // TITLEBAR chrome (egui top strip standing in for OS titlebar row):
    //   [ mode: Poly | CAD | Paint | Vector ]  [ RIBBON … ]  …  [ ? ] [ ⚙ ]
    // Window min/max/close remain OS-native to the right of this panel.
    // ═══════════════════════════════════════════════════════════════
    // Titlebar tint per mode (Poly blue, CAD teal, Paint violet, Vector amber)
    let titlebar_bg = match workspace.mode {
        AppMode::Poly => egui::Color32::from_rgb(32, 48, 72),    // steel blue
        AppMode::Cad => egui::Color32::from_rgb(28, 56, 52),     // teal
        AppMode::Paint => egui::Color32::from_rgb(52, 36, 64),   // violet
        AppMode::Vector => egui::Color32::from_rgb(56, 48, 28),  // warm amber-brown
    };
    let mode_chip_selected = match workspace.mode {
        AppMode::Poly => egui::Color32::from_rgb(56, 120, 200),
        AppMode::Cad => egui::Color32::from_rgb(40, 160, 140),
        AppMode::Paint => egui::Color32::from_rgb(140, 90, 200),
        AppMode::Vector => egui::Color32::from_rgb(200, 150, 60),
    };
    egui::Panel::top("titlebar")
        .exact_size(36.0)
        .frame(egui::Frame::NONE.fill(titlebar_bg).inner_margin(egui::Margin::symmetric(8, 4)))
        .show(&mut viewport_ui, |ui| {
            ui.horizontal_centered(|ui| {
                // Mode selector — SVG-matched vector icons (assets/icons/mode_*.svg)
                ui.spacing_mut().item_spacing.x = 4.0;
                for mode in AppMode::all() {
                    let selected = workspace.mode == mode;
                    let accent = match mode {
                        AppMode::Poly => egui::Color32::from_rgb(56, 120, 200),
                        AppMode::Cad => egui::Color32::from_rgb(40, 160, 140),
                        AppMode::Paint => egui::Color32::from_rgb(140, 90, 200),
                        AppMode::Vector => egui::Color32::from_rgb(200, 150, 60),
                    };
                    // selected chip uses active mode accent; unselected keep own hue on icon
                    let chip_accent = if selected { mode_chip_selected } else { accent };
                    let resp = mode_icon_button(ui, mode, selected, chip_accent);
                    if resp.clicked() {
                        workspace.mode = mode;
                    }
                    resp.on_hover_text(format!("{} — assets/icons/mode_{}.svg", mode.label(), match mode {
                        AppMode::Poly => "poly",
                        AppMode::Cad => "cad",
                        AppMode::Paint => "paint",
                        AppMode::Vector => "vector",
                    }));
                }
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);

                // Compact File/Edit always available in titlebar (ribbon holds the rest)
                ui.menu_button("File", |ui| {
                    let _ = ui.button(i18n.t(Msg::MenuNewScene));
                    let _ = ui.button(i18n.t(Msg::MenuOpen));
                    let _ = ui.button(i18n.t(Msg::MenuSave));
                    ui.separator();
                    let _ = ui.button(i18n.t(Msg::MenuQuit));
                });
                ui.menu_button("Edit", |ui| {
                    let _ = ui.button(i18n.t(Msg::MenuUndo));
                    let _ = ui.button(i18n.t(Msg::MenuRedo));
                });
                ui.menu_button("View", |ui| {
                    let _ = ui.button(i18n.t(Msg::MenuResetCamera));
                    ui.checkbox(&mut settings.show_grid, "Show grid");
                });

                // Push ? and Settings to the trailing edge (before OS window controls)
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(egui::Button::new("⚙").frame(false))
                        .on_hover_text("Settings")
                        .clicked()
                    {
                        workspace.show_settings = !workspace.show_settings;
                        workspace.show_help = false;
                    }
                    if ui
                        .add(egui::Button::new("?").frame(false))
                        .on_hover_text("Help")
                        .clicked()
                    {
                        workspace.show_help = !workspace.show_help;
                        workspace.show_settings = false;
                    }
                    ui.label(
                        egui::RichText::new(workspace.mode.label())
                            .small()
                            .color(egui::Color32::from_rgb(160, 160, 170)),
                    );
                });
            });
        });

    // ── Always-on RIBBON (mode-specific tabs/tools) ──
    let ribbon_bg = match workspace.mode {
        AppMode::Poly => egui::Color32::from_rgb(40, 52, 68),
        AppMode::Cad => egui::Color32::from_rgb(36, 56, 52),
        AppMode::Paint => egui::Color32::from_rgb(52, 44, 60),
        AppMode::Vector => egui::Color32::from_rgb(56, 52, 40),
    };
    egui::Panel::top("ribbon")
        .exact_size(52.0)
        .frame(egui::Frame::NONE.fill(ribbon_bg).inner_margin(egui::Margin::symmetric(8, 6)))
        .show(&mut viewport_ui, |ui| {
            ui.horizontal(|ui| {
                match workspace.mode {
                    AppMode::Poly => {
                        ui.label(egui::RichText::new("Mesh").strong());
                        ui.separator();
                        ui.small("Select V/E/F · G/R/S transform");
                        ui.separator();
                        ui.small("E Extrude · I Inset · Ctrl+B Bevel · Ctrl+R Loop Cut · X Delete");
                        ui.separator();
                        ui.small("Add: Shift+A primitives");
                    }
                    AppMode::Cad => {
                        ui.label(egui::RichText::new("3D").strong());
                        ui.separator();
                        if ui.button("Box").on_hover_text("BOX").clicked() {
                            spawn_cad_box(
                                &mut commands,
                                &mut meshes,
                                &mut std_materials,
                                &mut cad_ws,
                                "Box",
                                1.0,
                                1.0,
                                1.0,
                            );
                        }
                        if ui.button("Pad").on_hover_text("PAD rect").clicked() {
                            spawn_cad_pad_rect(
                                &mut commands,
                                &mut meshes,
                                &mut std_materials,
                                &mut cad_ws,
                                "Pad",
                                1.0,
                                0.8,
                                0.4,
                            );
                        }
                        if ui.button("Sketch").on_hover_text("S").clicked() {
                            sketch_tool.active = !sketch_tool.active;
                        }
                        ui.separator();
                        ui.label("Modify");
                        if ui.button("Solve").clicked() {
                            if let Some(e) = cad_ws.active_body {
                                if let Ok((_, mut body, _)) = cad_bodies.get_mut(e) {
                                    let _ = solve_body_sketches(&mut body);
                                    let _ = solve_body_sketches_lsq(&mut body);
                                }
                            }
                        }
                        ui.separator();
                        ui.label("Planes");
                        ui.selectable_value(&mut sketch_tool.plane, SketchPlane::XY, "XY");
                        ui.selectable_value(&mut sketch_tool.plane, SketchPlane::XZ, "XZ");
                        ui.selectable_value(&mut sketch_tool.plane, SketchPlane::YZ, "YZ");
                    }
                    AppMode::Paint => {
                        ui.label(egui::RichText::new("Paint").strong());
                        ui.separator();
                        ui.small("Brush · Erase · Clone  (texture pipeline)");
                        ui.separator();
                        if ui.button("Open PhotoCraft…").clicked() {
                            if let Err(e) = launch_photocraft(&artcraft.vendor_root) {
                                bevy::log::warn!("{e}");
                            }
                        }
                        ui.small("Sync: bash scripts/sync_artcraft.sh");
                    }
                    AppMode::Vector => {
                        ui.label(egui::RichText::new("Draw").strong());
                        ui.separator();
                        if ui.button("Rect").clicked() {
                            let path = VectorPath::rect(
                                "Rect",
                                bevy::math::Vec2::new(-0.5, -0.3),
                                bevy::math::Vec2::new(0.5, 0.3),
                            )
                            .with_fill();
                            let e = commands.spawn((path, Name::new("VectorRect"))).id();
                            vector_ws.active_path = Some(e);
                        }
                        if ui.button("Pen / Bézier").clicked() {
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
                        ui.separator();
                        ui.label("Pathfinder");
                        ui.small("U · J · D · Y");
                        ui.separator();
                        ui.checkbox(&mut vector_ws.auto_g1, "Auto G1");
                        ui.separator();
                        if ui.button("Open VectorCraft…").clicked() {
                            if let Err(e) = launch_vectorcraft(&artcraft.vendor_root) {
                                bevy::log::warn!("{e}");
                            }
                        }
                    }
                }
            });
        });

    // ── Left toolbar (Blender tool shelf — Poly only) ──
    if workspace.mode == AppMode::Poly {
    egui::Panel::left("toolbar")
        .default_size(72.0)
        .show(&mut viewport_ui, |ui| {
            ui.heading("Tool Shelf");
            ui.small("Blender Edit Mode");
            ui.separator();

            tool_button(
                ui,
                &mut editor.active_tool,
                Tool::Select,
                i18n.t(Msg::ToolSelect),
                i18n.t(Msg::TipSelect),
            );
            tool_button(
                ui,
                &mut editor.active_tool,
                Tool::Move,
                i18n.t(Msg::ToolMove),
                i18n.t(Msg::TipMove),
            );
            tool_button(
                ui,
                &mut editor.active_tool,
                Tool::Rotate,
                i18n.t(Msg::ToolRotate),
                i18n.t(Msg::TipRotate),
            );
            tool_button(
                ui,
                &mut editor.active_tool,
                Tool::Scale,
                i18n.t(Msg::ToolScale),
                i18n.t(Msg::TipScale),
            );
            ui.separator();
            tool_button(
                ui,
                &mut editor.active_tool,
                Tool::Extrude,
                i18n.t(Msg::ToolExtrude),
                i18n.t(Msg::TipExtrude),
            );
            tool_button(
                ui,
                &mut editor.active_tool,
                Tool::Inset,
                i18n.t(Msg::ToolInset),
                i18n.t(Msg::TipInset),
            );
            tool_button(
                ui,
                &mut editor.active_tool,
                Tool::Bevel,
                i18n.t(Msg::ToolBevel),
                i18n.t(Msg::TipBevel),
            );
            tool_button(
                ui,
                &mut editor.active_tool,
                Tool::CreatePrimitive,
                i18n.t(Msg::ToolPrimitive),
                i18n.t(Msg::TipPrimitive),
            );

            match editor.active_tool {
                Tool::Move => gizmo.mode = GizmoMode::Translate,
                Tool::Rotate => gizmo.mode = GizmoMode::Rotate,
                Tool::Scale => gizmo.mode = GizmoMode::Scale,
                _ => {}
            }
        });

    // ── Right properties panel ──
    } // end Poly toolbar

    // ── Properties N-panel (Poly) ──
    if workspace.mode == AppMode::Poly {
    egui::Panel::right("properties")
        .default_size(240.0)
        .show(&mut viewport_ui, |ui| {
            ui.heading("Properties");
            ui.small("N-panel");
            ui.separator();

            ui.label(i18n.t(Msg::SelectionMode));
            ui.horizontal(|ui| {
                mode_button(
                    ui,
                    &mut editor.selection_mode,
                    SelectionMode::Vertex,
                    i18n.t(Msg::ModeVertex),
                );
                mode_button(
                    ui,
                    &mut editor.selection_mode,
                    SelectionMode::Edge,
                    i18n.t(Msg::ModeEdge),
                );
                mode_button(
                    ui,
                    &mut editor.selection_mode,
                    SelectionMode::Face,
                    i18n.t(Msg::ModeFace),
                );
                mode_button(
                    ui,
                    &mut editor.selection_mode,
                    SelectionMode::Cell,
                    i18n.t(Msg::ModeCell),
                );
                mode_button(
                    ui,
                    &mut editor.selection_mode,
                    SelectionMode::Object,
                    i18n.t(Msg::ModeObject),
                );
            });

            if let Ok((_mesh, sel)) = selection_q.single() {
                ui.label(i18n.selected_counts(
                    sel.vertices.len(),
                    sel.edges.len(),
                    sel.faces.len(),
                ));
            }

            ui.separator();
            ui.label(i18n.t(Msg::GizmoMode));
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(gizmo.mode == GizmoMode::Translate, i18n.t(Msg::ToolMove))
                    .clicked()
                {
                    gizmo.mode = GizmoMode::Translate;
                    editor.active_tool = Tool::Move;
                }
                if ui
                    .selectable_label(gizmo.mode == GizmoMode::Rotate, i18n.t(Msg::ToolRotate))
                    .clicked()
                {
                    gizmo.mode = GizmoMode::Rotate;
                    editor.active_tool = Tool::Rotate;
                }
                if ui
                    .selectable_label(gizmo.mode == GizmoMode::Scale, i18n.t(Msg::ToolScale))
                    .clicked()
                {
                    gizmo.mode = GizmoMode::Scale;
                    editor.active_tool = Tool::Scale;
                }
            });

            ui.separator();
            ui.checkbox(&mut editor.snap_enabled, i18n.t(Msg::Snap));
            if editor.snap_enabled {
                ui.add(
                    egui::Slider::new(&mut editor.snap_value, 0.01..=1.0)
                        .text(i18n.t(Msg::SnapValue)),
                );
            }

            ui.separator();
            ui.label(format!(
                "{}: {}",
                i18n.t(Msg::ToolLabel),
                i18n.tool_name(editor.active_tool)
            ));
            ui.label(format!("{}: {:?}", i18n.t(Msg::GizmoLabel), gizmo.mode));

            ui.separator();
            ui.label(i18n.t(Msg::Language));
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(
                        i18n.locale == Locale::English,
                        Locale::English.display_name(),
                    )
                    .clicked()
                {
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

    } // end Poly properties
}

fn editor_windows_system(
    mut contexts: EguiContexts,
    mut materials_table: ResMut<MaterialTable>,
    mut mesh_uv_q: Query<(&mut EditableMesh, &Selection), With<ActiveMesh>>,
    mut uv_state: ResMut<UvEditorState>,
    objects_q: Query<(Entity, Option<&ObjectName>), With<EditableObject>>,
    active_q: Query<Entity, With<ActiveMesh>>,
    registry: Res<CommandRegistry>,
    mut commands: Commands,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    // Materials
    egui::Window::new("Materials")
        .default_width(200.0)
        .show(ctx, |ui| {
            ui.heading("Slots");
            let MaterialTable { slots, active_slot } = &mut *materials_table;
            for (i, slot) in slots.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    let selected = *active_slot == i as u32;
                    if ui.selectable_label(selected, format!("{i}: {}", slot.name)).clicked() {
                        *active_slot = i as u32;
                    }
                });
                let mut col = [
                    slot.base_color.x,
                    slot.base_color.y,
                    slot.base_color.z,
                ];
                if ui.color_edit_button_rgb(&mut col).changed() {
                    slot.base_color.x = col[0];
                    slot.base_color.y = col[1];
                    slot.base_color.z = col[2];
                }
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

    // UV Editor window (interactive)
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
                if ui.button("Clear UV sel").clicked() {
                    uv_state.clear();
                }
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
            painter.rect_stroke(
                rect,
                0.0,
                egui::Stroke::new(1.0, egui::Color32::GRAY),
                egui::StrokeKind::Inside,
            );
            let to_screen = |uv: bevy::math::Vec2| {
                egui::pos2(
                    rect.left() + uv.x.clamp(0.0, 1.0) * rect.width(),
                    rect.bottom() - uv.y.clamp(0.0, 1.0) * rect.height(),
                )
            };
            let to_uv = |pos: egui::Pos2| {
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
                        let cur = to_uv(pos);
                        if let Some(last) = uv_state.last_cursor {
                            let delta = cur - last;
                            for &i in &uv_state.selected {
                                if let Some(uv) = mesh.uvs.get_mut(i as usize) {
                                    *uv = (*uv + delta).clamp(bevy::math::Vec2::ZERO, bevy::math::Vec2::ONE);
                                }
                            }
                        }
                        uv_state.last_cursor = Some(cur);
                        uv_state.dragging = true;
                    }
                } else {
                    uv_state.dragging = false;
                    uv_state.last_cursor = None;
                }
            }
        });
}

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
