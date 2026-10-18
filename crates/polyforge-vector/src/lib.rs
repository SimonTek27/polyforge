
//! # polyforge-vector
//! VectorMode scaffold (Phase 4): 2D path documents for future SVG-like tools.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub mod svg;
pub mod boolean2d;
pub mod edit;
pub mod continuity;
pub mod martinez;
pub use martinez::*;
pub use continuity::*;
pub use edit::*;
pub use boolean2d::*;
pub use svg::*;

pub struct PolyForgeVectorPlugin;

impl Plugin for PolyForgeVectorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VectorWorkspace>()
            .register_type::<VectorPath>()
            .add_systems(Update, cycle_active_path_system)
            .add_systems(Update, delete_active_path_system)
            .add_systems(Update, draw_active_path_nodes);
    }
}

#[derive(Resource, Debug, Clone)]
pub struct VectorWorkspace {
    pub active_path: Option<Entity>,
    pub stroke_width: f32,
    /// Phase 16: when true, dragging Bezier handles re-applies G1.
    pub auto_g1: bool,
}

impl Default for VectorWorkspace {
    fn default() -> Self {
        Self {
            active_path: None,
            stroke_width: 2.0,
            auto_g1: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
pub struct BezierSegment {
    pub p0: Vec2,
    pub p1: Vec2,
    pub p2: Vec2,
    pub p3: Vec2,
}

impl BezierSegment {
    pub fn sample(&self, steps: u32) -> Vec<Vec2> {
        let steps = steps.max(2);
        (0..=steps)
            .map(|i| {
                let t = i as f32 / steps as f32;
                let u = 1.0 - t;
                self.p0 * (u * u * u)
                    + self.p1 * (3.0 * u * u * t)
                    + self.p2 * (3.0 * u * t * t)
                    + self.p3 * (t * t * t)
            })
            .collect()
    }
}

#[derive(Component, Reflect, Debug, Clone, Serialize, Deserialize)]
#[reflect(Component)]
pub struct VectorPath {
    pub name: String,
    pub points: Vec<Vec2>,
    pub closed: bool,
    #[serde(default)]
    pub beziers: Vec<BezierSegment>,
    #[serde(default)]
    pub filled: bool,
    /// Phase 21: inner rings (holes), opposite winding optional.
    #[serde(default)]
    pub holes: Vec<Vec<Vec2>>,
}

impl Default for VectorPath {
    fn default() -> Self {
        Self {
            name: "Path".into(),
            points: Vec::new(),
            closed: false,
            beziers: Vec::new(),
            filled: false,
            holes: Vec::new(),
        }
    }
}

impl VectorPath {
    pub fn rect(name: &str, min: Vec2, max: Vec2) -> Self {
        Self {
            name: name.into(),
            points: vec![
                min,
                Vec2::new(max.x, min.y),
                max,
                Vec2::new(min.x, max.y),
            ],
            closed: true,
            beziers: Vec::new(),
            filled: false,
            holes: Vec::new(),
        }
    }

    pub fn with_fill(mut self) -> Self {
        self.filled = true;
        self
    }

    /// Flatten beziers into points for display/export.
    pub fn flattened(&self, steps: u32) -> Vec<Vec2> {
        if self.beziers.is_empty() {
            return self.points.clone();
        }
        let mut pts = Vec::new();
        for b in &self.beziers {
            let mut s = b.sample(steps);
            if !pts.is_empty() && !s.is_empty() {
                s.remove(0); // avoid duplicate joints
            }
            pts.extend(s);
        }
        pts
    }

    pub fn length(&self) -> f32 {
        if self.points.len() < 2 {
            return 0.0;
        }
        let mut len = 0.0;
        for w in self.points.windows(2) {
            len += w[0].distance(w[1]);
        }
        if self.closed && self.points.len() > 2 {
            len += self.points.last().unwrap().distance(self.points[0]);
        }
        len
    }
}

/// Draw vector paths as gizmos on XY plane (y = 0).
pub fn draw_vector_paths(
    mut gizmos: Gizmos,
    workspace: Res<polyforge_core::WorkspaceState>,
    q: Query<&VectorPath>,
) {
    use polyforge_core::AppMode;
    if workspace.mode != AppMode::Vector {
        return;
    }
    let col = Color::srgb(0.2, 0.85, 0.95);
    for path in &q {
        let pts = path.flattened(8);
        if pts.len() < 2 {
            continue;
        }
        for w in pts.windows(2) {
            let a = Vec3::new(w[0].x, 0.02, w[0].y);
            let b = Vec3::new(w[1].x, 0.02, w[1].y);
            gizmos.line(a, b, col);
        }
        if path.closed && pts.len() > 2 {
            let a = pts.last().unwrap();
            let b = pts[0];
            gizmos.line(
                Vec3::new(a.x, 0.02, a.y),
                Vec3::new(b.x, 0.02, b.y),
                col,
            );
        }
        // Holes
        let hole_col = Color::srgb(1.0, 0.35, 0.35);
        for hole in &path.holes {
            for w in hole.windows(2) {
                gizmos.line(
                    Vec3::new(w[0].x, 0.025, w[0].y),
                    Vec3::new(w[1].x, 0.025, w[1].y),
                    hole_col,
                );
            }
            if hole.len() > 2 {
                let a = hole.last().unwrap();
                let b = hole[0];
                gizmos.line(
                    Vec3::new(a.x, 0.025, a.y),
                    Vec3::new(b.x, 0.025, b.y),
                    hole_col,
                );
            }
        }
        // Fan fill (gizmo lines)
        if path.filled && pts.len() >= 3 {
            let fill = Color::srgba(0.2, 0.85, 0.95, 0.35);
            let c = pts[0];
            for w in pts[1..].windows(2) {
                gizmos.line(
                    Vec3::new(c.x, 0.01, c.y),
                    Vec3::new(w[0].x, 0.01, w[0].y),
                    fill,
                );
                gizmos.line(
                    Vec3::new(w[0].x, 0.01, w[0].y),
                    Vec3::new(w[1].x, 0.01, w[1].y),
                    fill,
                );
                gizmos.line(
                    Vec3::new(w[1].x, 0.01, w[1].y),
                    Vec3::new(c.x, 0.01, c.y),
                    fill,
                );
            }
        }
    }
}
