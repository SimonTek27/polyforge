//! Screen-space area (marquee / box) selection state.

use bevy::prelude::*;

/// Active or idle marquee rectangle in screen (viewport) coordinates.
#[derive(Resource, Debug, Clone, Default)]
pub struct MarqueeState {
    pub active: bool,
    pub start: Vec2,
    pub current: Vec2,
    /// If true, Shift was held when the marquee started (additive selection).
    pub additive: bool,
}

impl MarqueeState {
    pub fn begin(&mut self, pos: Vec2, additive: bool) {
        self.active = true;
        self.start = pos;
        self.current = pos;
        self.additive = additive;
    }

    pub fn update(&mut self, pos: Vec2) {
        self.current = pos;
    }

    pub fn end(&mut self) -> Option<MarqueeRect> {
        if !self.active {
            return None;
        }
        self.active = false;
        let rect = MarqueeRect::from_corners(self.start, self.current);
        if rect.width() < 3.0 && rect.height() < 3.0 {
            return None;
        }
        Some(rect)
    }

    pub fn cancel(&mut self) {
        self.active = false;
    }

    pub fn rect(&self) -> Option<MarqueeRect> {
        if self.active {
            Some(MarqueeRect::from_corners(self.start, self.current))
        } else {
            None
        }
    }
}

/// Axis-aligned rectangle in screen space (origin top-left of viewport).
#[derive(Debug, Clone, Copy)]
pub struct MarqueeRect {
    pub min: Vec2,
    pub max: Vec2,
}

impl MarqueeRect {
    pub fn from_corners(a: Vec2, b: Vec2) -> Self {
        Self {
            min: Vec2::new(a.x.min(b.x), a.y.min(b.y)),
            max: Vec2::new(a.x.max(b.x), a.y.max(b.y)),
        }
    }

    pub fn width(&self) -> f32 {
        self.max.x - self.min.x
    }

    pub fn height(&self) -> f32 {
        self.max.y - self.min.y
    }

    pub fn contains(&self, p: Vec2) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }
}

/// State for free (camera-plane) vertex dragging.
#[derive(Resource, Debug, Clone, Default)]
pub struct VertexDragState {
    pub active: bool,
    pub start_screen: Vec2,
    pub last_screen: Vec2,
}
