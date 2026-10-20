//! Circle (brush) selection state.

use bevy::prelude::*;

#[derive(Resource, Debug, Clone)]
pub struct CircleSelectState {
    pub active: bool,
    pub radius_px: f32,
    pub cursor: Vec2,
}

impl Default for CircleSelectState {
    fn default() -> Self {
        Self {
            active: false,
            radius_px: 40.0,
            cursor: Vec2::ZERO,
        }
    }
}

/// Proportional editing (soft selection) state.
#[derive(Resource, Debug, Clone)]
pub struct ProportionalEdit {
    pub enabled: bool,
    pub radius: f32,
}

impl Default for ProportionalEdit {
    fn default() -> Self {
        Self {
            enabled: false,
            radius: 1.0,
        }
    }
}
