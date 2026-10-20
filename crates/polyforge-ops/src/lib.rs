//! # polyforge-ops
//! Modeling operations for PolyForge.

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
pub mod subdivide;
pub mod symmetrize;
pub mod bridge;
pub mod solidify;
pub mod dissolve;
pub mod separate;
pub mod rip;
pub mod knife;
pub mod spin;
pub mod screw;
pub mod snap;
pub mod rip_fill;

pub use extrude::*;
pub use primitive::*;
pub use transform::*;
pub use edit::*;
pub use loop_cut::*;
pub use bevel::*;
pub use merge::*;
pub use fill::*;
pub use subdivide::*;
pub use symmetrize::*;
pub use bridge::*;
pub use solidify::*;
pub use dissolve::*;
pub use separate::*;
pub use rip::*;
pub use knife::*;
pub use spin::*;
pub use screw::*;
pub use snap::*;
pub use rip_fill::*;

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

pub fn delete_faces(mesh: &mut EditableMesh, selection: &Selection) -> OpResult {
    delete_faces_op(mesh, selection)
}
