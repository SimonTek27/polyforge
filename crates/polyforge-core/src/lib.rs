//! # polyforge-core
//!
//! Core data structures for mesh representation, selection, and topology
//! operations used throughout PolyForge.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub mod mesh;
pub mod selection;
pub mod topology;
pub mod components;
pub mod gizmo;
pub mod picking;
pub mod marquee;

pub use mesh::*;
pub use selection::*;
pub use topology::*;
pub use components::*;
pub use gizmo::*;
pub use picking::*;
pub use marquee::*;

/// Plugin that registers core resources and systems.
pub struct PolyForgeCorePlugin;

impl Plugin for PolyForgeCorePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EditorState>()
            .init_resource::<GizmoState>()
            .init_resource::<MarqueeState>()
            .init_resource::<VertexDragState>()
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
    pub snap_enabled: bool,
    pub snap_value: f32,
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
