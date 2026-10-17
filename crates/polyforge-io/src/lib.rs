//! # polyforge-io
//! Import and export of 3D assets and project files.

use bevy::prelude::*;
use polyforge_core::EditableMesh;
use thiserror::Error;

pub mod obj;
pub mod project;
pub mod gltf_export;
pub mod gltf_import;

pub use obj::*;
pub use project::*;
pub use gltf_export::*;
pub use gltf_import::*;

pub struct PolyForgeIoPlugin;
impl Plugin for PolyForgeIoPlugin {
    fn build(&self, _app: &mut App) {}
}

#[derive(Debug, Error)]
pub enum IoError {
    #[error("Unsupported format")]
    UnsupportedFormat,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Not implemented")]
    NotImplemented,
}

pub type IoResult<T> = Result<T, IoError>;

pub fn load_placeholder_cube() -> EditableMesh {
    EditableMesh::cube()
}
