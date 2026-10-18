//! Application-level workspace modes (Poly / CAD / Vector).

use bevy::prelude::*;

/// Top-level workspace mode — selected on the left edge of the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
pub enum AppMode {
    /// Polygon mesh Edit Mode (Blender-parity Phase 1–2).
    #[default]
    Poly,
    /// Parametric solid / sketch mode (FreeCAD-parity, post Phase 2).
    Cad,
    /// 2D vector / path mode (future).
    Vector,
}

impl AppMode {
    pub fn label(self) -> &'static str {
        match self {
            AppMode::Poly => "PolyMode",
            AppMode::Cad => "CadMode",
            AppMode::Vector => "VectorMode",
        }
    }

    pub fn next(self) -> Self {
        match self {
            AppMode::Poly => AppMode::Cad,
            AppMode::Cad => AppMode::Vector,
            AppMode::Vector => AppMode::Poly,
        }
    }
}

/// Global workspace mode + chrome flags.
#[derive(Resource, Debug, Clone)]
pub struct WorkspaceState {
    pub mode: AppMode,
    pub show_settings: bool,
    pub show_help: bool,
}

impl Default for WorkspaceState {
    fn default() -> Self {
        Self {
            mode: AppMode::Poly,
            show_settings: false,
            show_help: false,
        }
    }
}

/// User preferences (Settings panel).
#[derive(Resource, Debug, Clone)]
pub struct AppSettings {
    pub units: Units,
    pub orbit_sensitivity: f32,
    pub show_grid: bool,
    pub auto_save: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
pub enum Units {
    #[default]
    Metric,
    Imperial,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            units: Units::Metric,
            orbit_sensitivity: 1.0,
            show_grid: true,
            auto_save: false,
        }
    }
}
