//! Application-level workspace modes (Poly / CAD / Paint / Vector).

use bevy::prelude::*;

/// Top-level workspace mode — selected on the titlebar (left of the ribbon).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
pub enum AppMode {
    /// Polygon mesh Edit Mode (Blender parity).
    #[default]
    Poly,
    /// Parametric solid / sketch mode (CadCraft parity).
    Cad,
    /// Raster / texture paint (PhotoCraft companion).
    Paint,
    /// 2D vector / path mode (VectorCraft parity).
    Vector,
}

impl AppMode {
    pub fn label(self) -> &'static str {
        match self {
            AppMode::Poly => "Poly · Blender",
            AppMode::Cad => "CAD · CadCraft",
            AppMode::Paint => "Paint · PhotoCraft",
            AppMode::Vector => "Vector · VectorCraft",
        }
    }

    pub fn short(self) -> &'static str {
        match self {
            AppMode::Poly => "Poly",
            AppMode::Cad => "CAD",
            AppMode::Paint => "Paint",
            AppMode::Vector => "Vector",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            AppMode::Poly => "🧊",
            AppMode::Cad => "📐",
            AppMode::Paint => "🎨",
            AppMode::Vector => "✎",
        }
    }

    pub fn next(self) -> Self {
        match self {
            AppMode::Poly => AppMode::Cad,
            AppMode::Cad => AppMode::Paint,
            AppMode::Paint => AppMode::Vector,
            AppMode::Vector => AppMode::Poly,
        }
    }

    pub fn all() -> [AppMode; 4] {
        [AppMode::Poly, AppMode::Cad, AppMode::Paint, AppMode::Vector]
    }
}

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

#[derive(Resource, Debug, Clone)]
pub struct CommandLineState {
    pub input: String,
    pub last_command: String,
    pub history: Vec<String>,
    pub message: String,
}

impl Default for CommandLineState {
    fn default() -> Self {
        Self {
            input: String::new(),
            last_command: String::new(),
            history: Vec::new(),
            message: "Ready — type a command (BOX, CYL, EXT, SKETCH, PAD…)".into(),
        }
    }
}
