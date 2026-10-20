//! Sketch creation helpers.

use crate::{CadBody, CadFeature, SketchCurve, SketchPlane, next_feature_id};

impl CadBody {
    /// Add a rectangular sketch on XY and pad it.
    pub fn add_pad_rect(&mut self, width: f32, depth: f32, height: f32) {
        let sketch_id = next_feature_id();
        let hw = width * 0.5;
        let hd = depth * 0.5;
        self.push_feature(CadFeature::Sketch {
            id: sketch_id,
            plane: SketchPlane::XY,
            curves: vec![SketchCurve::Rect {
                min: bevy::math::Vec2::new(-hw, -hd),
                max: bevy::math::Vec2::new(hw, hd),
            }],
            constraints: Vec::new(),
        });
        self.push_feature(CadFeature::Pad {
            id: next_feature_id(),
            sketch_id,
            distance: height,
        });
    }

    pub fn add_pad_circle(&mut self, radius: f32, height: f32) {
        let sketch_id = next_feature_id();
        self.push_feature(CadFeature::Sketch {
            id: sketch_id,
            plane: SketchPlane::XY,
            curves: vec![SketchCurve::Circle {
                center: bevy::math::Vec2::ZERO,
                radius,
            }],
            constraints: Vec::new(),
        });
        self.push_feature(CadFeature::Pad {
            id: next_feature_id(),
            sketch_id,
            distance: height,
        });
    }
}
