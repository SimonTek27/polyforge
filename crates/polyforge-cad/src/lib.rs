//! CadMode crate — parametric bodies, sketches, solver, assembly.
pub mod assembly;
pub mod body;
pub mod boolean;
pub mod constraints;
pub mod feature;
pub mod fillet;
pub mod kernel;
pub mod lsq_solver;
pub mod params;
pub mod recompute;
pub mod sketch;
pub mod sketch_tool;
pub mod solver;
pub mod step_export;
pub mod tessellate;

pub use assembly::*;
pub use body::*;
pub use boolean::*;
pub use constraints::*;
pub use feature::*;
pub use fillet::*;
pub use kernel::*;
pub use lsq_solver::*;
pub use params::*;
pub use recompute::*;
pub use sketch::*;
pub use sketch_tool::*;
pub use solver::*;
pub use step_export::*;
pub use tessellate::*;

use bevy::prelude::*;

#[derive(Resource, Debug, Clone, Default)]
pub struct CadWorkspace {
    pub active_body: Option<Entity>,
    pub editing_sketch: Option<CadFeatureId>,
    pub last_error: Option<String>,
    pub selected_curve: usize,
    pub selected_feature: usize,
}

pub struct PolyForgeCadPlugin;

impl Plugin for PolyForgeCadPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CadWorkspace>()
            .init_resource::<SketchToolState>()
            .init_resource::<ActiveKernel>()
            .register_type::<CadBody>()
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
