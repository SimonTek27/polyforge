//! CadBody helpers.

use crate::{CadBody, CadFeature, next_feature_id};

impl CadBody {
    pub fn add_box(&mut self, width: f32, height: f32, depth: f32) {
        self.push_feature(CadFeature::Box {
            id: next_feature_id(),
            width,
            height,
            depth,
        });
    }

    pub fn add_cylinder(&mut self, radius: f32, height: f32) {
        self.push_feature(CadFeature::Cylinder {
            id: next_feature_id(),
            radius,
            height,
            segments: 32,
        });
    }

    pub fn add_sphere(&mut self, radius: f32) {
        self.push_feature(CadFeature::Sphere {
            id: next_feature_id(),
            radius,
            segments: 24,
        });
    }
}

impl CadBody {
    pub fn remove_feature_at(&mut self, index: usize) -> bool {
        if index < self.features.len() {
            self.features.remove(index);
            self.dirty = true;
            true
        } else {
            false
        }
    }

    pub fn clear_features(&mut self) {
        self.features.clear();
        self.dirty = true;
    }
}
