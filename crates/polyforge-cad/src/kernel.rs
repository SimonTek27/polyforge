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

/// Placeholder for the OCCT backend (`--features cad_kernel`).
#[cfg(feature = "cad_kernel")]
pub struct OcctKernel;

#[cfg(feature = "cad_kernel")]
impl SolidKernel for OcctKernel {
    fn evaluate(&self, features: &[CadFeature]) -> EditableMesh {
        if !occt_ready() {
            return tessellate_features(features);
        }
        // TODO(Phase 34, step 3): map CadFeature to OCCT primitives/booleans,
        // triangulate with `polyforge_occt_sys::triangulate` and build an
        // EditableMesh. Until then the mesh backend is the fallback.
        tessellate_features(features)
    }
    fn name(&self) -> &'static str {
        "occt"
    }
}

#[cfg(feature = "cad_kernel")]
fn occt_ready() -> bool {
    use std::sync::OnceLock;
    static READY: OnceLock<bool> = OnceLock::new();
    *READY.get_or_init(|| {
        let ok = polyforge_occt_sys::init();
        if ok {
            info!(
                "OCCT kernel linked (version {})",
                polyforge_occt_sys::version().unwrap_or_else(|| "unknown".into())
            );
        } else {
            warn!("OCCT kernel failed to initialise; using mesh fallback");
        }
        ok
    })
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
