//! # polyforge-ops
//!
//! Modeling operations: Extrude, Inset, Bevel, Loop Cut, Merge, Fill, etc.

use bevy::prelude::*;
use polyforge_core::{EditableMesh, Selection};

pub mod extrude;
pub mod primitive;
pub mod transform;
pub mod edit;
pub mod loop_cut;
pub mod bevel;
pub mod merge;
pub mod fill;

pub use extrude::*;
pub use primitive::*;
pub use transform::*;
pub use edit::*;
pub use loop_cut::*;
pub use bevel::*;
pub use merge::*;
pub use fill::*;

pub struct PolyForgeOpsPlugin;

impl Plugin for PolyForgeOpsPlugin {
    fn build(&self, _app: &mut App) {}
}

pub type OpResult = Result<(), OpError>;

#[derive(Debug, thiserror::Error)]
pub enum OpError {
    #[error("Empty selection")]
    EmptySelection,
    #[error("Invalid topology for this operation")]
    InvalidTopology,
    #[error("Not implemented yet")]
    NotImplemented,
}

/// Delete selected faces (legacy helper).
pub fn delete_faces(mesh: &mut EditableMesh, selection: &Selection) -> OpResult {
    delete_faces_op(mesh, selection)
}
