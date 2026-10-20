//! # polyforge-print
//!
//! 3D printing subsystem for PolyForge: mesh slicing, printer profiles and
//! G-code generation. Ported from ksengine's editor-only 3D print tooling
//! (`SlicerEngine`, `GCodeGenerator`, `PrinterProfile`).
//!
//! ```
//! use polyforge_print::{generate_gcode, PrinterProfile, SlicerEngine, Triangle, Vec3};
//!
//! // A 20 mm cube.
//! let s = 10.0;
//! let v = [
//!     Vec3::new(-s, -s, -s), Vec3::new(s, -s, -s), Vec3::new(s, s, -s), Vec3::new(-s, s, -s),
//!     Vec3::new(-s, -s, s), Vec3::new(s, -s, s), Vec3::new(s, s, s), Vec3::new(-s, s, s),
//! ];
//! let mut triangles = Vec::new();
//! for f in [[0, 1, 2, 3], [5, 4, 7, 6], [4, 0, 3, 7], [1, 5, 6, 2], [3, 2, 6, 7], [4, 5, 1, 0]] {
//!     triangles.push(Triangle::new(v[f[0]], v[f[1]], v[f[2]]));
//!     triangles.push(Triangle::new(v[f[0]], v[f[2]], v[f[3]]));
//! }
//!
//! let printer = PrinterProfile::create_builtin("generic_marlin").unwrap();
//! let settings = printer.default_settings.clone();
//! let slice = SlicerEngine::new().slice(&triangles, &settings, &printer);
//! assert!(slice.success);
//! let gcode = generate_gcode(&slice, &printer, &settings);
//! assert!(gcode.contains("G28"));
//! ```

#[cfg(feature = "bevy")]
use bevy::prelude::{App, Plugin, Resource};
use thiserror::Error;

pub mod gcode;
pub mod geometry;
pub mod profile;
pub mod slicer;
pub mod types;

#[cfg(feature = "mesh")]
pub mod mesh_source;

pub use gcode::*;
pub use geometry::*;
pub use profile::*;
pub use slicer::*;
pub use types::*;

#[cfg(feature = "mesh")]
pub use mesh_source::*;

#[derive(Debug, Error)]
pub enum PrintError {
    #[error("empty mesh")]
    EmptyMesh,
    #[error("invalid settings: {0}")]
    InvalidSettings(String),
    #[error("model does not fit build volume: {0}")]
    BuildVolume(String),
    #[error("slicing failed: {0}")]
    SliceFailed(String),
    #[error("printer not found: {0}")]
    PrinterNotFound(String),
}

pub type PrintResult<T> = Result<T, PrintError>;

/// Resource holding the available printers and the active selection.
#[cfg(feature = "bevy")]
#[derive(Resource)]
pub struct PrintState {
    pub registry: PrinterRegistry,
    pub settings: SliceSettings,
    pub last_slice: Option<SliceInfo>,
}

#[cfg(feature = "bevy")]
impl Default for PrintState {
    fn default() -> Self {
        let registry = PrinterRegistry::with_builtins();
        let settings = registry
            .active()
            .map(|p| p.default_settings.clone())
            .unwrap_or_default();
        Self {
            registry,
            settings,
            last_slice: None,
        }
    }
}

/// Registers print resources with the Bevy app.
#[cfg(feature = "bevy")]
pub struct PolyForgePrintPlugin;

#[cfg(feature = "bevy")]
impl Plugin for PolyForgePrintPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PrintState>();
    }
}
