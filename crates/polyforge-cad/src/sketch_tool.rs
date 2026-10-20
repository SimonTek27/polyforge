//! Interactive sketcher: click points on a work plane to build curves.

use bevy::prelude::*;
use polyforge_core::{AppMode, WorkspaceState};
use crate::{CadBody, CadFeature, CadFeatureId, CadWorkspace, SketchCurve, SketchPlane, next_feature_id};

#[derive(Resource, Debug, Clone)]
pub struct SketchToolState {
    pub active: bool,
    pub plane: SketchPlane,
    /// Points clicked so far (2D on plane).
    pub points: Vec<Vec2>,
    /// Auto-close polygon when returning near start.
    pub close_threshold: f32,
}

impl Default for SketchToolState {
    fn default() -> Self {
        Self {
            active: false,
            plane: SketchPlane::XY,
            points: Vec::new(),
            close_threshold: 0.08,
        }
    }
}

impl SketchToolState {
    pub fn clear(&mut self) {
        self.points.clear();
    }

    pub fn to_curves(&self) -> Vec<SketchCurve> {
        let mut curves = Vec::new();
        if self.points.len() >= 2 {
            for w in self.points.windows(2) {
                curves.push(SketchCurve::Line { a: w[0], b: w[1] });
            }
            if self.points.len() >= 3 {
                let a = *self.points.last().unwrap();
                let b = self.points[0];
                if a.distance(b) < self.close_threshold * 3.0 {
                    curves.push(SketchCurve::Line { a, b });
                }
            }
        }
        curves
    }
}

/// Map world hit on plane to 2D sketch coords.
pub fn world_to_sketch(plane: SketchPlane, world: Vec3) -> Vec2 {
    match plane {
        SketchPlane::XY => Vec2::new(world.x, world.y),
        SketchPlane::XZ => Vec2::new(world.x, world.z),
        SketchPlane::YZ => Vec2::new(world.y, world.z),
    }
}

pub fn sketch_to_world(plane: SketchPlane, p: Vec2) -> Vec3 {
    match plane {
        SketchPlane::XY => Vec3::new(p.x, p.y, 0.0),
        SketchPlane::XZ => Vec3::new(p.x, 0.0, p.y),
        SketchPlane::YZ => Vec3::new(0.0, p.x, p.y),
    }
}

/// Click-to-sketch when CadMode + sketch tool active.
pub fn sketch_tool_system(
    mut tool: ResMut<SketchToolState>,
    workspace: Res<WorkspaceState>,
    mut cad_ws: ResMut<CadWorkspace>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    camera_q: Query<(&Camera, &GlobalTransform)>,
    windows: Query<&Window>,
    mut bodies: Query<&mut CadBody>,
    mut gizmos: Gizmos,
) {
    if workspace.mode != AppMode::Cad {
        return;
    }

    // S toggles sketch tool (without Ctrl)
    if keys.just_pressed(KeyCode::KeyS)
        && !keys.pressed(KeyCode::ControlLeft)
        && !keys.pressed(KeyCode::ControlRight)
        && !keys.pressed(KeyCode::ShiftLeft)
        && !keys.pressed(KeyCode::AltLeft)
    {
        tool.active = !tool.active;
        if !tool.active {
            tool.clear();
        }
        info!("Sketch tool: {}", tool.active);
    }

    if !tool.active {
        return;
    }

    // Draw current polyline
    let col = Color::srgb(1.0, 0.85, 0.2);
    for w in tool.points.windows(2) {
        gizmos.line(
            sketch_to_world(tool.plane, w[0]),
            sketch_to_world(tool.plane, w[1]),
            col,
        );
    }
    for &p in &tool.points {
        let c = sketch_to_world(tool.plane, p);
        gizmos.sphere(c, 0.03, col);
    }
    // Draw plane grid hint
    let o = Vec3::ZERO;
    match tool.plane {
        SketchPlane::XY => {
            gizmos.line(o + Vec3::X, o - Vec3::X, Color::srgba(1.0, 1.0, 0.0, 0.3));
            gizmos.line(o + Vec3::Y, o - Vec3::Y, Color::srgba(1.0, 1.0, 0.0, 0.3));
        }
        SketchPlane::XZ => {
            gizmos.line(o + Vec3::X, o - Vec3::X, Color::srgba(1.0, 1.0, 0.0, 0.3));
            gizmos.line(o + Vec3::Z, o - Vec3::Z, Color::srgba(1.0, 1.0, 0.0, 0.3));
        }
        SketchPlane::YZ => {
            gizmos.line(o + Vec3::Y, o - Vec3::Y, Color::srgba(1.0, 1.0, 0.0, 0.3));
            gizmos.line(o + Vec3::Z, o - Vec3::Z, Color::srgba(1.0, 1.0, 0.0, 0.3));
        }
    }

    // Enter commits sketch to active body
    if keys.just_pressed(KeyCode::Enter) && tool.points.len() >= 2 {
        commit_sketch(&mut tool, &mut cad_ws, &mut bodies);
        return;
    }

    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    let Ok((camera, cam_tf)) = camera_q.single() else { return };
    let Ok(ray) = camera.viewport_to_world(cam_tf, cursor) else { return };

    let plane_n = match tool.plane {
        SketchPlane::XY => Vec3::Z,
        SketchPlane::XZ => Vec3::Y,
        SketchPlane::YZ => Vec3::X,
    };
    let denom = ray.direction.dot(plane_n);
    if denom.abs() < 1e-6 {
        return;
    }
    let t = -ray.origin.dot(plane_n) / denom;
    let hit = ray.origin + *ray.direction * t;
    let mut p = world_to_sketch(tool.plane, hit);

    // Auto horizontal/vertical snap to last point
    if let Some(&last) = tool.points.last() {
        let d = p - last;
        if d.x.abs() < 0.05 {
            p.x = last.x; // vertical line
        } else if d.y.abs() < 0.05 {
            p.y = last.y; // horizontal
        }
    }

    // Close if near start
    if tool.points.len() >= 3 {
        if p.distance(tool.points[0]) < tool.close_threshold {
            let first = tool.points[0];
            tool.points.push(first);
            commit_sketch(&mut tool, &mut cad_ws, &mut bodies);
            return;
        }
    }
    tool.points.push(p);
}

fn commit_sketch(
    tool: &mut SketchToolState,
    cad_ws: &mut CadWorkspace,
    bodies: &mut Query<&mut CadBody>,
) {
    let curves = tool.to_curves();
    if curves.is_empty() {
        return;
    }
    let sketch_id = next_feature_id();
    let sketch = CadFeature::Sketch {
        id: sketch_id,
        plane: tool.plane,
        curves,
        constraints: Vec::new(),
    };
    if let Some(e) = cad_ws.active_body {
        if let Ok(mut body) = bodies.get_mut(e) {
            body.push_feature(sketch);
            // Auto pad
            body.push_feature(CadFeature::Pad {
                id: next_feature_id(),
                sketch_id,
                distance: 0.4,
            });
            info!("Sketch committed + Pad 0.4 on '{}'", body.name);
        }
    } else {
        cad_ws.last_error = Some("No active CAD body — create a Box first".into());
        warn!("Sketch commit failed: no active body");
    }
    tool.clear();
    tool.active = false;
    let _ = CadFeatureId; // keep import used if needed
}
