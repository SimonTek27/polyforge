//! # polyforge-core
//!
//! Core data structures for mesh representation, selection, and topology
//! operations used throughout PolyForge.

/// Bevy major.minor this tree targets (see bevy-compat.toml).
pub const BEVY_TARGET_VERSION: &str = "0.20";

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub mod mesh;
pub mod selection;
pub mod topology;
pub mod components;
pub mod gizmo;
pub mod picking;
pub mod marquee;
pub mod i18n;
pub mod history;
pub mod circle_select;
pub mod lasso;
pub mod modifiers;
pub mod knife_tool;
pub mod plugin_api;
pub mod materials;
pub mod app_mode;
pub mod uv_edit;
pub mod hierarchy;

pub use mesh::*;
pub use selection::*;
pub use topology::*;
pub use components::*;
pub use gizmo::*;
pub use picking::*;
pub use marquee::*;
pub use i18n::*;
pub use history::*;
pub use circle_select::*;
pub use lasso::*;
pub use modifiers::*;
pub use knife_tool::*;
pub use plugin_api::*;
pub use materials::*;
pub use app_mode::*;
pub use uv_edit::*;
pub use hierarchy::*;

/// Plugin that registers core resources and systems.
pub struct PolyForgeCorePlugin;

impl Plugin for PolyForgeCorePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EditorState>()
            .init_resource::<GizmoState>()
            .init_resource::<MarqueeState>()
            .init_resource::<VertexDragState>()
            .init_resource::<I18n>()
            .init_resource::<History>()
            .init_resource::<CircleSelectState>()
            .init_resource::<ProportionalEdit>()
            .init_resource::<LassoState>()
            .init_resource::<KnifeToolState>()
            .init_resource::<SceneRoot>()
            .init_resource::<CommandRegistry>()
            .init_resource::<PendingPluginCommand>()
            .init_resource::<MacroRegistry>()
            .init_resource::<MaterialTable>()
            .init_resource::<WorkspaceState>()
            .init_resource::<AppSettings>()
            .init_resource::<UvEditorState>()
            .register_type::<EditableMesh>()
            .register_type::<Selection>()
            .register_type::<MeshId>();
    }
}

/// Global editor state shared across systems.
#[derive(Resource, Default, Debug, Clone)]
pub struct EditorState {
    pub active_tool: Tool,
    pub selection_mode: SelectionMode,
    /// Mode restored when leaving Object mode via Tab (Blender-like).
    pub edit_selection_mode: SelectionMode,
    pub snap_enabled: bool,
    pub snap_value: f32,
    /// Geometric snap: 0 none, 1 vertex, 2 edge, 3 face, 4 grid.
    pub snap_mode: u8,
}

/// Available modeling tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
pub enum Tool {
    #[default]
    Select,
    Move,
    Rotate,
    Scale,
    Extrude,
    Inset,
    Bevel,
    Bridge,
    Knife,
    CreatePrimitive,
}

/// Selection granularity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Reflect, Serialize, Deserialize)]
pub enum SelectionMode {
    #[default]
    Vertex,
    Edge,
    Face,
    /// Cell mode: selects faces as polygonal cells, with linked-region tools.
    Cell,
    Object,
}

impl SelectionMode {
    /// Cell mode operates on the same data as face mode.
    pub fn uses_faces(self) -> bool {
        matches!(self, SelectionMode::Face | SelectionMode::Cell)
    }
}
