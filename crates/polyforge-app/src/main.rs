//! PolyForge – Open-source 3D polygon modeling tool
//! Built with Bevy Engine · Licensed under GPL-3.0-or-later

use bevy::prelude::*;
use bevy::window::WindowPlugin;
use polyforge_core::{
    PolyForgeCorePlugin, EditableMesh, EditableObject, ActiveMesh, Selection, MeshId,
    eval_modifier_stack, ModifierStack, SceneRoot, Collection, InCollection,
    ObjectName, PendingPluginCommand, MacroRegistry,
};
use polyforge_ops::PolyForgeOpsPlugin;
use polyforge_uv::PolyForgeUvPlugin;
use polyforge_io::PolyForgeIoPlugin;
use polyforge_ui::PolyForgeUiPlugin;
use polyforge_cad::PolyForgeCadPlugin;
use polyforge_vector::{PolyForgeVectorPlugin, draw_vector_paths};
use polyforge_bridge::PolyForgeBridgePlugin;
use polyforge_paint::PolyForgePaintPlugin;
use polyforge_print::PolyForgePrintPlugin;

mod object_mode;
mod material_gpu;
mod cad_input;
mod vector_io;
mod scene_io;
mod camera;
mod gizmo_sys;
mod selection_sys;
mod scene_ops;
mod hotkeys;
mod print_panel;
mod paint_sys;

use object_mode::object_mode_transform_system;
use material_gpu::sync_materials_to_gpu;
use cad_input::cad_hotkey_system;
use vector_io::vector_io_system;
use scene_io::{scene_hotkeys, auto_save_system, AutoSaveTimer};
use camera::{OrbitCamera, orbit_camera_system};
use gizmo_sys::{update_gizmo_origin, gizmo_interaction_system, draw_gizmo_system};
use hotkeys::hotkey_system;
use print_panel::{print_panel_system, print_preview_system, PrintUiState};
use paint_sys::{paint_history_system, paint_stroke_system, paint_texture_system, PaintTextureBinding};
use scene_ops::{
    cycle_active_object_system, separate_object_system, join_active_objects_system,
    mirror_modifier_system, array_modifier_hotkeys, collection_visibility_system,
    parent_transform_system, interactive_knife_system,
    setup_plugin_commands, plugin_command_hotkeys, queue_plugin_hotkeys,
    queue_macro_hotkeys, execute_pending_plugin_command,
};
use selection_sys::{
    pointer_input_system, draw_selection_system, circle_select_system,
    proportional_and_circle_hotkeys,
};
use scene_ops::{
    draw_circle_brush_system, lasso_select_system, draw_lasso_system,
};

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "PolyForge".into(),
                    resolution: (1280u32, 720u32).into(),
                    ..default()
                }),
                ..default()
            }),
        )
        .add_plugins((
            PolyForgeCorePlugin,
            PolyForgeOpsPlugin,
            PolyForgeUvPlugin,
            PolyForgeIoPlugin,
            PolyForgeUiPlugin,
            PolyForgeCadPlugin,
            PolyForgeVectorPlugin,
            PolyForgeBridgePlugin,
            PolyForgePaintPlugin,
            PolyForgePrintPlugin,
        ))
        .init_resource::<MacroRunner>()
        .init_resource::<AutoSaveTimer>()
        .init_resource::<PrintUiState>()
        .init_resource::<PaintTextureBinding>()
        .add_systems(Startup, (setup_scene, setup_plugin_commands))
        .add_systems(
            bevy_egui::EguiPrimaryContextPass,
            print_panel_system,
        )
        .add_systems(
            Update,
            (
                orbit_camera_system,
                hotkey_system,
                proportional_and_circle_hotkeys,
                update_gizmo_origin,
                gizmo_interaction_system,
                pointer_input_system,
                circle_select_system,
                cycle_active_object_system,
                separate_object_system,
                join_active_objects_system,
                lasso_select_system,
                draw_lasso_system,
                mirror_modifier_system,
                interactive_knife_system,
                plugin_command_hotkeys,
            ),
        )
        .add_systems(
            Update,
            (
                array_modifier_hotkeys,
                collection_visibility_system,
                parent_transform_system,
                queue_plugin_hotkeys,
                execute_pending_plugin_command,
                queue_macro_hotkeys,
                run_macro_steps_system,
                draw_gizmo_system,
                draw_selection_system,
                draw_circle_brush_system,
                sync_mesh_to_gpu,
                object_mode_transform_system,
                sync_materials_to_gpu,
                cad_hotkey_system,
                draw_vector_paths,
                vector_io_system,
                scene_hotkeys,
                print_preview_system,
                auto_save_system,
            ),
        )
        .add_systems(
            Update,
            (
                paint_history_system,
                paint_stroke_system,
                paint_texture_system,
            )
                .chain(),
        )
        .run();
}

// ─────────────────────────────────────────────────────────────
// Scene setup
// ─────────────────────────────────────────────────────────────

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut scene_root: ResMut<SceneRoot>,
) {
    let root = commands
        .spawn((Collection {
            name: "Scene".into(),
            visible: true,
        },))
        .id();
    scene_root.root_collection = Some(root);
    let root_col = root;

    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.insert_resource(GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 120.0,
        ..default()
    });

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(3.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        OrbitCamera {
            distance: 6.0,
            yaw: 0.6,
            pitch: 0.4,
        },
    ));

    let editable = EditableMesh::cube();
    let bevy_mesh = editable.to_bevy_mesh();
    let mesh_handle = meshes.add(bevy_mesh);

    commands.spawn((
        Mesh3d(mesh_handle),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.35, 0.55, 0.85),
            metallic: 0.15,
            perceptual_roughness: 0.55,
            ..default()
        })),
        Transform::default(),
        EditableObject,
        ActiveMesh,
        ObjectName("Cube".into()),
        editable,
        Selection::default(),
        MeshId(1),
        InCollection(root_col),
        Name::new("Cube"),
    ));

    info!(
        "PolyForge ready · Bevy target {} · drag area-select, G move",
        polyforge_core::BEVY_TARGET_VERSION
    );
}

// ─────────────────────────────────────────────────────────────
// Camera
// ─────────────────────────────────────────────────────────────

fn sync_mesh_to_gpu(
    mut meshes: ResMut<Assets<Mesh>>,
    query: Query<(&EditableMesh, &Mesh3d, Option<&ModifierStack>)>,
) {
    for (editable, mesh3d, stack) in &query {
        if let Some(mut gpu_mesh) = meshes.get_mut(&mesh3d.0) {
            let evaluated = if let Some(st) = stack {
                if st.mirror.is_some() || st.array.is_some() || st.solidify.is_some() {
                    eval_modifier_stack(editable, st)
                } else {
                    editable.clone()
                }
            } else {
                editable.clone()
            };
            *gpu_mesh = evaluated.to_bevy_mesh();
        }
    }
}

#[derive(Resource, Default)]
struct MacroRunner {
    queue: Vec<String>,
}

fn run_macro_steps_system(
    mut runner: ResMut<MacroRunner>,
    keys: Res<ButtonInput<KeyCode>>,
    macros: Res<MacroRegistry>,
    mut pending: ResMut<PendingPluginCommand>,
) {
    if keys.just_pressed(KeyCode::F7) {
        if let Some(m) = macros.get("prep.export") {
            runner.queue = m.steps.clone();
        }
    }
    if pending.0.is_some() {
        return;
    }
    if let Some(next) = runner.queue.first().cloned() {
        runner.queue.drain(0..1);
        pending.0 = Some(next);
    }
}
