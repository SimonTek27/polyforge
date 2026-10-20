
//! Edit parametric feature dimensions (task panel).

use crate::feature::CadFeature;

/// Mutable view of dimensions for UI.
#[derive(Debug, Clone)]
pub struct FeatureParams {
    pub label: String,
    pub values: Vec<(String, f32)>,
}

pub fn feature_params(f: &CadFeature) -> FeatureParams {
    match f {
        CadFeature::Box { width, height, depth, .. } => FeatureParams {
            label: "Box".into(),
            values: vec![
                ("Width".into(), *width),
                ("Height".into(), *height),
                ("Depth".into(), *depth),
            ],
        },
        CadFeature::Cylinder { radius, height, .. } => FeatureParams {
            label: "Cylinder".into(),
            values: vec![("Radius".into(), *radius), ("Height".into(), *height)],
        },
        CadFeature::Sphere { radius, .. } => FeatureParams {
            label: "Sphere".into(),
            values: vec![("Radius".into(), *radius)],
        },
        CadFeature::Cone { radius, height, .. } => FeatureParams {
            label: "Cone".into(),
            values: vec![("Radius".into(), *radius), ("Height".into(), *height)],
        },
        CadFeature::Pad { distance, .. } => FeatureParams {
            label: "Pad".into(),
            values: vec![("Distance".into(), *distance)],
        },
        CadFeature::Pocket { distance, .. } => FeatureParams {
            label: "Pocket".into(),
            values: vec![("Distance".into(), *distance)],
        },
        CadFeature::Sketch { curves, constraints, .. } => FeatureParams {
            label: "Sketch".into(),
            values: vec![("Curves".into(), curves.len() as f32), ("Constraints".into(), constraints.len() as f32)],
        },
    }
}

/// Apply UI-edited values back onto feature (order matches feature_params).
pub fn apply_feature_params(f: &mut CadFeature, values: &[f32]) {
    match f {
        CadFeature::Box { width, height, depth, .. } => {
            if values.len() >= 3 {
                *width = values[0].max(0.01);
                *height = values[1].max(0.01);
                *depth = values[2].max(0.01);
            }
        }
        CadFeature::Cylinder { radius, height, .. }
        | CadFeature::Cone { radius, height, .. } => {
            if values.len() >= 2 {
                *radius = values[0].max(0.01);
                *height = values[1].max(0.01);
            }
        }
        CadFeature::Sphere { radius, .. } => {
            if let Some(v) = values.first() {
                *radius = v.max(0.01);
            }
        }
        CadFeature::Pad { distance, .. } | CadFeature::Pocket { distance, .. } => {
            if let Some(v) = values.first() {
                *distance = v.max(0.01);
            }
        }
        CadFeature::Sketch { .. } => {}
    }
}
