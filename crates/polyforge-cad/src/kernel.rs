//! Solid kernel abstraction (Phase 9).
//! Default: mesh-parametric backend. Optional `cad_kernel` feature reserved for OCCT.

use bevy::prelude::*;
use polyforge_core::EditableMesh;
use crate::feature::CadFeature;
use crate::tessellate::tessellate_features;

/// Backend that turns a feature stack into a display mesh.
pub trait SolidKernel: Send + Sync {
    fn evaluate(&self, features: &[CadFeature]) -> EditableMesh;
    fn name(&self) -> &'static str;
}

/// Mesh tessellation backend (always available).
pub struct MeshKernel;

impl SolidKernel for MeshKernel {
    fn evaluate(&self, features: &[CadFeature]) -> EditableMesh {
        tessellate_features(features)
    }
    fn name(&self) -> &'static str {
        "mesh-parametric"
    }
}

/// Placeholder for future OCCT backend (`--features cad_kernel`).
#[cfg(feature = "cad_kernel")]
pub struct OcctKernel;

#[cfg(feature = "cad_kernel")]
impl SolidKernel for OcctKernel {
    fn evaluate(&self, features: &[CadFeature]) -> EditableMesh {
        // TODO: call OCCT when linked
        tessellate_features(features)
    }
    fn name(&self) -> &'static str {
        "occt"
    }
}

#[derive(Resource)]
pub struct ActiveKernel {
    pub backend: String,
}

impl Default for ActiveKernel {
    fn default() -> Self {
        Self {
            backend: MeshKernel.name().into(),
        }
    }
}

pub fn evaluate_features(features: &[CadFeature]) -> EditableMesh {
    MeshKernel.evaluate(features)
}
