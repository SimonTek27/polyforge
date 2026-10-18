
//! Interactive knife tool — multi-point stroke cut.

use bevy::prelude::*;

#[derive(Resource, Debug, Clone, Default)]
pub struct KnifeToolState {
    pub active: bool,
    /// Screen or world points along the stroke.
    pub world_points: Vec<Vec3>,
    pub point_a: Option<Vec3>,
    pub point_b: Option<Vec3>,
}

impl KnifeToolState {
    pub fn reset_points(&mut self) {
        self.point_a = None;
        self.point_b = None;
        self.world_points.clear();
    }

    pub fn push_world(&mut self, p: Vec3) {
        if self.world_points.last().map(|q| q.distance(p) > 1e-4).unwrap_or(true) {
            self.world_points.push(p);
        }
        if self.point_a.is_none() {
            self.point_a = Some(p);
        }
        self.point_b = Some(p);
    }
}
