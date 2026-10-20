//! Lasso (freehand polygon) selection.

use bevy::prelude::*;

#[derive(Resource, Debug, Clone, Default)]
pub struct LassoState {
    pub active: bool,
    pub points: Vec<Vec2>,
    pub closed: bool,
}

impl LassoState {
    pub fn clear(&mut self) {
        self.points.clear();
        self.closed = false;
    }

    /// Point-in-polygon (ray casting).
    pub fn contains(&self, p: Vec2) -> bool {
        if self.points.len() < 3 {
            return false;
        }
        let mut inside = false;
        let n = self.points.len();
        let mut j = n - 1;
        for i in 0..n {
            let pi = self.points[i];
            let pj = self.points[j];
            if ((pi.y > p.y) != (pj.y > p.y))
                && (p.x < (pj.x - pi.x) * (p.y - pi.y) / (pj.y - pi.y + 1e-12) + pi.x)
            {
                inside = !inside;
            }
            j = i;
        }
        inside
    }
}
