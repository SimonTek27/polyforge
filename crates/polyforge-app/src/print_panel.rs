//! 3D print panel: printer selection, slice settings and G-code export.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use polyforge_core::{ActiveMesh, EditableMesh, WorkspaceState};
use polyforge_print::{
    generate_gcode, mesh_to_bed_triangles, BedAdhesionType, InfillPattern, PrintState,
    QualityProfile, SlicerEngine, SupportType,
};

#[derive(Resource)]
pub struct PrintUiState {
    /// Scene units -> millimetres (PolyForge models are unit-scale).
    pub mm_per_unit: f32,
    pub status: String,
    pub error: bool,
}

impl Default for PrintUiState {
    fn default() -> Self {
        Self {
            mm_per_unit: 100.0,
            status: String::new(),
            error: false,
        }
    }
}

fn format_duration(seconds: f64) -> String {
    let total = seconds.max(0.0) as u64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{h}h {m:02}m")
    } else if m > 0 {
        format!("{m}m {s:02}s")
    } else {
        format!("{s}s")
    }
}

fn output_path(settings: &polyforge_print::SliceSettings) -> std::path::PathBuf {
    let name = if settings.output_filename.is_empty() {
        "polyforge_print.gcode".to_string()
    } else {
        settings.output_filename.clone()
    };
    let name = if name.ends_with(".gcode") {
        name
    } else {
        format!("{name}.gcode")
    };
    std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join("artifacts")
        .join(name)
}

pub fn print_panel_system(
    mut contexts: EguiContexts,
    mut workspace: ResMut<WorkspaceState>,
    mut print: ResMut<PrintState>,
    mut ui_state: ResMut<PrintUiState>,
    mesh_q: Query<&EditableMesh, With<ActiveMesh>>,
) {
    if !workspace.show_print {
        return;
    }
    let Ok(ctx) = contexts.ctx_mut() else { return; };

    let mut open = true;
    egui::Window::new("3D Print")
        .open(&mut open)
        .default_width(320.0)
        .resizable(true)
        .show(ctx, |ui| {
            let Some(printer) = print.registry.active().cloned() else {
                ui.label("No printer profiles available.");
                return;
            };

            // ── Printer ──
            ui.label("Printer");
            let profiles: Vec<(String, String)> = print
                .registry
                .profiles()
                .iter()
                .map(|p| (p.id.clone(), p.name.clone()))
                .collect();
            let active_name = printer.name.clone();
            let mut chosen: Option<String> = None;
            egui::ComboBox::from_id_salt("print_printer")
                .selected_text(active_name)
                .show_ui(ui, |ui| {
                    for (id, name) in &profiles {
                        if ui
                            .selectable_label(&printer.id == id, name)
                            .clicked()
                        {
                            chosen = Some(id.clone());
                        }
                    }
                });
            if let Some(id) = chosen {
                if print.registry.set_active(&id) {
                    if let Some(p) = print.registry.active() {
                        print.settings = p.default_settings.clone();
                    }
                    print.last_slice = None;
                    ui_state.status.clear();
                }
            }
            ui.small(format!(
                "Build volume: {:.0} × {:.0} × {:.0} mm · {}",
                printer.build_volume_x, printer.build_volume_y, printer.build_volume_z,
                printer.gcode_flavor_label()
            ));

            ui.separator();

            // ── Scale ──
            ui.label("Model scale");
            ui.add(
                egui::Slider::new(&mut ui_state.mm_per_unit, 1.0..=500.0)
                    .logarithmic(true)
                    .suffix(" mm/unit"),
            );
            ui.small("PolyForge models are unit-scale; 100 mm/unit makes a 1-unit cube 10 cm.");

            ui.separator();

            // ── Quality ──
            ui.label("Quality");
            ui.horizontal(|ui| {
                for q in [
                    QualityProfile::Draft,
                    QualityProfile::Standard,
                    QualityProfile::High,
                    QualityProfile::Ultra,
                ] {
                    let selected = print.settings.quality == q;
                    if ui.selectable_label(selected, format!("{q:?}")).clicked() {
                        print.settings = print.settings.clone().with_quality(q);
                    }
                }
            });
            ui.add(
                egui::Slider::new(&mut print.settings.layer_height, 0.05..=0.6)
                    .text("Layer height")
                    .suffix(" mm"),
            );
            ui.add(
                egui::Slider::new(&mut print.settings.wall_count, 1..=6).text("Walls"),
            );
            ui.add(
                egui::Slider::new(&mut print.settings.infill_density, 0.0..=1.0)
                    .text("Infill")
                    .custom_formatter(|v, _| format!("{:.0}%", v * 100.0)),
            );
            egui::ComboBox::from_id_salt("print_infill")
                .selected_text(format!("{:?}", print.settings.infill_pattern))
                .show_ui(ui, |ui| {
                    for p in [
                        InfillPattern::Grid,
                        InfillPattern::Lines,
                        InfillPattern::Triangles,
                        InfillPattern::Concentric,
                        InfillPattern::Gyroid,
                    ] {
                        ui.selectable_value(&mut print.settings.infill_pattern, p, format!("{p:?}"));
                    }
                });

            ui.separator();

            // ── Temperatures ──
            ui.horizontal(|ui| {
                ui.add(
                    egui::DragValue::new(&mut print.settings.nozzle_temp)
                        .range(0..=printer.max_hotend_temp)
                        .suffix(" °C")
                        .speed(1),
                );
                ui.label("nozzle");
                ui.add(
                    egui::DragValue::new(&mut print.settings.bed_temp)
                        .range(0..=printer.max_bed_temp)
                        .suffix(" °C")
                        .speed(1),
                );
                ui.label("bed");
            });

            ui.separator();

            // ── Bed adhesion ──
            egui::ComboBox::from_id_salt("print_adhesion")
                .selected_text(format!("{:?}", print.settings.adhesion_type))
                .show_ui(ui, |ui| {
                    for a in [
                        BedAdhesionType::None,
                        BedAdhesionType::Skirt,
                        BedAdhesionType::Brim,
                    ] {
                        ui.selectable_value(&mut print.settings.adhesion_type, a, format!("{a:?}"));
                    }
                });

            ui.separator();

            // ── Supports ──
            ui.label("Supports");
            ui.horizontal(|ui| {
                if ui
                    .checkbox(&mut print.settings.generate_support, "Generate")
                    .changed()
                {
                    print.settings.support_type = if print.settings.generate_support {
                        SupportType::Normal
                    } else {
                        SupportType::None
                    };
                }
                ui.add_enabled(
                    print.settings.generate_support,
                    egui::DragValue::new(&mut print.settings.support_angle)
                        .range(5.0..=89.0)
                        .suffix(" °")
                        .speed(0.5),
                );
                ui.label("overhang");
            });

            ui.separator();

            let Ok(mesh) = mesh_q.single() else {
                ui.colored_label(egui::Color32::YELLOW, "Select a mesh to print.");
                return;
            };

            ui.horizontal(|ui| {
                if ui.button("Slice").clicked() {
                    let tris = mesh_to_bed_triangles(mesh, ui_state.mm_per_unit, &printer);
                    let slice = SlicerEngine::new().slice(&tris, &print.settings, &printer);
                    ui_state.error = !slice.success;
                    ui_state.status = if slice.success {
                        format!(
                            "Sliced: {} layers · {} · {:.1} m filament",
                            slice.total_layers,
                            format_duration(slice.print_time),
                            slice.filament_used / 1000.0
                        )
                    } else {
                        format!("Slice failed: {}", slice.errors.join("; "))
                    };
                    print.last_slice = Some(slice);
                }

                if ui
                    .add_enabled(print.last_slice.is_some(), egui::Button::new("Export G-code"))
                    .clicked()
                {
                    if let Some(slice) = print.last_slice.as_ref() {
                        let path = output_path(&print.settings);
                        if let Some(dir) = path.parent() {
                            let _ = std::fs::create_dir_all(dir);
                        }
                        let gcode = generate_gcode(slice, &printer, &print.settings);
                        match std::fs::write(&path, gcode) {
                            Ok(()) => {
                                ui_state.error = false;
                                ui_state.status = format!("Wrote {}", path.display());
                                info!("G-code written to {}", path.display());
                            }
                            Err(e) => {
                                ui_state.error = true;
                                ui_state.status = format!("Write failed: {e}");
                            }
                        }
                    }
                }
            });

            if let Some(slice) = print.last_slice.as_ref() {
                let size = slice.bounding_box.size();
                ui.separator();
                ui.label(format!(
                    "{} layers · {} · {:.2} m filament",
                    slice.total_layers,
                    format_duration(slice.print_time),
                    slice.filament_used / 1000.0
                ));
                ui.small(format!(
                    "Bounds: {:.1} × {:.1} × {:.1} mm",
                    size.x, size.y, size.z
                ));
                for w in &slice.warnings {
                    ui.colored_label(egui::Color32::YELLOW, w);
                }
                for e in &slice.errors {
                    ui.colored_label(egui::Color32::LIGHT_RED, e);
                }
            }

            if !ui_state.status.is_empty() {
                let color = if ui_state.error {
                    egui::Color32::LIGHT_RED
                } else {
                    egui::Color32::LIGHT_GREEN
                };
                ui.colored_label(color, &ui_state.status);
            }
        });

    if !open {
        workspace.show_print = false;
    }
}
