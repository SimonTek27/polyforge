//! # polyforge-cad
//!
//! CadMode: parametric solid bodies, sketches, feature history, solvers, assembly.
//! Tessellates to [`polyforge_core::EditableMesh`] for the Bevy viewport.
//! Real B-rep kernel (OCCT) is staged behind the `cad_kernel` feature.

use bevy::prelude::*;
use polyforge_core::EditableMesh;

pub mod body;
pub mod feature;
pub mod sketch;
pub mod tessellate;
pub mod recompute;
pub mod fillet;
pub mod boolean;
pub mod params;
pub mod sketch_tool;
pub mod constraints;
pub mod solver;
pub mod lsq_solver;
pub mod assembly;
pub mod kernel;
pub mod step_export;

pub use feature::*;
pub use tessellate::*;
pub use recompute::*;
pub use fillet::*;
pub use boolean::*;
pub use params::*;
pub use sketch_tool::*;
pub use constraints::*;
pub use solver::*;
pub use lsq_solver::*;
pub use assembly::*;
pub use kernel::*;
pub use step_export::*;

pub struct PolyForgeCadPlugin;

impl Plugin for PolyForgeCadPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<CadBody>()
            .register_type::<CadFeatureId>()
            .init_resource::<CadWorkspace>()
            .init_resource::<SketchToolState>()
            .init_resource::<ActiveKernel>()
            .add_systems(
                Update,
                (
                    cad_recompute_system,
                    cad_sync_display_mesh_system,
                    sketch_tool_system,
                    apply_assembly_mates,
                ),
            );
    }
}

/// CadMode session state (active body, sketch edit).
#[derive(Resource, Debug, Clone, Default)]
pub struct CadWorkspace {
    pub active_body: Option<Entity>,
    pub editing_sketch: Option<CadFeatureId>,
    pub last_error: Option<String>,
    pub selected_curve: usize,
    pub selected_feature: usize,
}

/// Marker: entity is a CAD solid body (not a free mesh object).
#[derive(Component, Reflect, Debug, Clone, Default)]
#[reflect(Component)]
pub struct CadBody {
    pub name: String,
    pub features: Vec<CadFeature>,
    pub dirty: bool,
}

impl CadBody {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            features: Vec::new(),
            dirty: true,
        }
    }

    pub fn push_feature(&mut self, f: CadFeature) {
        self.features.push(f);
        self.dirty = true;
    }
}

/// Display mesh produced by tessellation (sibling of feature graph).
#[derive(Component, Debug, Clone, Default)]
pub struct CadDisplayMesh(pub EditableMesh);
