//! Parametric feature definitions for CadBody history.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Serialize, Deserialize)]
pub struct CadFeatureId(pub u64);

#[derive(Debug, Clone, Reflect, Serialize, Deserialize)]
pub enum CadFeature {
    /// Box centered at origin (local).
    Box {
        id: CadFeatureId,
        width: f32,
        height: f32,
        depth: f32,
    },
    Cylinder {
        id: CadFeatureId,
        radius: f32,
        height: f32,
        segments: u32,
    },
    Sphere {
        id: CadFeatureId,
        radius: f32,
        segments: u32,
    },
    Cone {
        id: CadFeatureId,
        radius: f32,
        height: f32,
        segments: u32,
    },
    /// Pad: extrude a sketch along +Z by `distance`.
    Pad {
        id: CadFeatureId,
        sketch_id: CadFeatureId,
        distance: f32,
    },
    /// Pocket: cut by extruding sketch (mesh boolean approx for now).
    Pocket {
        id: CadFeatureId,
        sketch_id: CadFeatureId,
        distance: f32,
    },
    Sketch {
        id: CadFeatureId,
        plane: SketchPlane,
        curves: Vec<SketchCurve>,
        #[serde(default)]
        constraints: Vec<UserConstraint>,
    },
}

/// User-authored constraint stored on a sketch feature.
#[derive(Debug, Clone, Reflect, Serialize, Deserialize)]
pub struct UserConstraint {
    pub kind: UserConstraintKind,
    /// Primary curve index.
    pub a: usize,
    /// Secondary curve index (equal length / parallel).
    #[serde(default)]
    pub b: usize,
    /// Optional fixed length / distance.
    #[serde(default)]
    pub value: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect, Serialize, Deserialize)]
pub enum UserConstraintKind {
    Horizontal,
    Vertical,
    EqualLength,
    FixedLength,
    /// Make lines a and b parallel (Phase 30).
    Parallel,
    /// Snap end of curve a to start of curve b (shared endpoint).
    Coincident,
}


impl CadFeature {
    pub fn id(&self) -> CadFeatureId {
        match self {
            CadFeature::Box { id, .. }
            | CadFeature::Cylinder { id, .. }
            | CadFeature::Sphere { id, .. }
            | CadFeature::Cone { id, .. }
            | CadFeature::Pad { id, .. }
            | CadFeature::Pocket { id, .. }
            | CadFeature::Sketch { id, .. } => *id,
        }
    }

    pub fn label(&self) -> String {
        match self {
            CadFeature::Box { width, height, depth, .. } => {
                format!("Box {width:.2}×{height:.2}×{depth:.2}")
            }
            CadFeature::Cylinder { radius, height, .. } => {
                format!("Cylinder r={radius:.2} h={height:.2}")
            }
            CadFeature::Sphere { radius, .. } => format!("Sphere r={radius:.2}"),
            CadFeature::Cone { radius, height, .. } => {
                format!("Cone r={radius:.2} h={height:.2}")
            }
            CadFeature::Pad { distance, .. } => format!("Pad {distance:.2}"),
            CadFeature::Pocket { distance, .. } => format!("Pocket {distance:.2}"),
            CadFeature::Sketch { curves, constraints, .. } => format!("Sketch ({} curves, {} cns)", curves.len(), constraints.len()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect, Serialize, Deserialize, Default)]
pub enum SketchPlane {
    #[default]
    XY,
    XZ,
    YZ,
}

#[derive(Debug, Clone, Reflect, Serialize, Deserialize)]
pub enum SketchCurve {
    Line { a: Vec2, b: Vec2 },
    Circle { center: Vec2, radius: f32 },
    Rect { min: Vec2, max: Vec2 },
}
