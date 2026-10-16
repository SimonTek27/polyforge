//! # polyforge-io
//!
//! Import and export of 3D assets.
//! Priority targets:
//! - glTF / GLB (modern standard)
//! - OBJ (simple interchange)
//! - Additional game and CAD formats in the future

use bevy::prelude::*;
use polyforge_core::EditableMesh;
use thiserror::Error;

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

/// Placeholder: load a simple cube for testing.
pub fn load_placeholder_cube() -> EditableMesh {
    EditableMesh::cube()
}
