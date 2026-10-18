//! Transform gizmo system (translate / rotate / scale handles).

use bevy::prelude::*;
use bevy::color::palettes::css;

/// Which transform mode the gizmo is currently in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
pub enum GizmoMode {
    #[default]
    Translate,
    Rotate,
    Scale,
}

/// Which axis (or plane) is currently hovered / dragged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
pub enum GizmoAxis {
    #[default]
    None,
    X,
    Y,
    Z,
    XY,
    XZ,
    YZ,
    View, // free / screen-space
}

/// Runtime state of the interactive gizmo.
#[derive(Resource, Debug, Clone)]
pub struct GizmoState {
    pub mode: GizmoMode,
    pub active_axis: GizmoAxis,
    pub hovered_axis: GizmoAxis,
    pub is_dragging: bool,
    /// World-space origin of the gizmo (usually selection center).
    pub origin: Vec3,
    /// Mouse position when drag started (screen space).
    pub drag_start_screen: Vec2,
    /// Origin at the moment drag started.
    pub drag_start_origin: Vec3,
    /// Accumulated translation / rotation / scale during the current drag.
    pub drag_delta: Vec3,
    pub drag_rotation: Quat,
    pub drag_scale: Vec3,
    /// Visual size of the gizmo (world units, scaled by distance).
    pub size: f32,
}

impl Default for GizmoState {
    fn default() -> Self {
        Self {
            mode: GizmoMode::Translate,
            active_axis: GizmoAxis::None,
            hovered_axis: GizmoAxis::None,
            is_dragging: false,
            origin: Vec3::ZERO,
            drag_start_screen: Vec2::ZERO,
            drag_start_origin: Vec3::ZERO,
            drag_delta: Vec3::ZERO,
            drag_rotation: Quat::IDENTITY,
            drag_scale: Vec3::ONE,
            size: 1.0,
        }
    }
}

impl GizmoState {
    pub fn reset_drag(&mut self) {
        self.is_dragging = false;
        self.active_axis = GizmoAxis::None;
        self.drag_delta = Vec3::ZERO;
        self.drag_rotation = Quat::IDENTITY;
        self.drag_scale = Vec3::ONE;
    }
}

/// Colors for the three axes (classic RGB).
pub fn axis_color(axis: GizmoAxis, hovered: bool, active: bool) -> Color {
    let base = match axis {
        GizmoAxis::X | GizmoAxis::YZ => css::RED,
        GizmoAxis::Y | GizmoAxis::XZ => css::LIME,
        GizmoAxis::Z | GizmoAxis::XY => css::DODGER_BLUE,
        _ => css::WHITE,
    };

    if active {
        Color::srgb(1.0, 1.0, 0.2) // yellow when active
    } else if hovered {
        Color::srgb(1.0, 0.9, 0.4)
    } else {
        Color::Srgba(base)
    }
}

/// Draw the transform gizmo using Bevy's Gizmos API.
pub fn draw_gizmo(gizmos: &mut Gizmos, state: &GizmoState) {
    let o = state.origin;
    let s = state.size;

    match state.mode {
        GizmoMode::Translate => draw_translate_gizmo(gizmos, o, s, state),
        GizmoMode::Rotate => draw_rotate_gizmo(gizmos, o, s, state),
        GizmoMode::Scale => draw_scale_gizmo(gizmos, o, s, state),
    }
}

fn draw_translate_gizmo(gizmos: &mut Gizmos, origin: Vec3, size: f32, state: &GizmoState) {
    let axes = [
        (GizmoAxis::X, Vec3::X),
        (GizmoAxis::Y, Vec3::Y),
        (GizmoAxis::Z, Vec3::Z),
    ];

    for &(axis, dir) in &axes {
        let color = axis_color(axis, state.hovered_axis == axis, state.active_axis == axis);
        let end = origin + dir * size;

        // Axis line
        gizmos.line(origin, end, color);

        // Arrow head (simple cone approximated by a few lines)
        let tip = end;
        let side = size * 0.12;
        let back = end - dir * side * 1.8;
        let perp1 = dir.any_orthonormal_vector() * side;
        let perp2 = dir.cross(perp1).normalize() * side;

        gizmos.line(tip, back + perp1, color);
        gizmos.line(tip, back - perp1, color);
        gizmos.line(tip, back + perp2, color);
        gizmos.line(tip, back - perp2, color);
    }

    // Center cube
    let cube_size = size * 0.08;
    gizmos.primitive_3d(
        &Cuboid::from_size(Vec3::splat(cube_size * 2.0)),
        Isometry3d::from_translation(origin),
        Color::srgb(0.9, 0.9, 0.9),
    );
}

fn draw_rotate_gizmo(gizmos: &mut Gizmos, origin: Vec3, size: f32, state: &GizmoState) {
    let axes = [
        (GizmoAxis::X, Vec3::X),
        (GizmoAxis::Y, Vec3::Y),
        (GizmoAxis::Z, Vec3::Z),
    ];

    for &(axis, normal) in &axes {
        let color = axis_color(axis, state.hovered_axis == axis, state.active_axis == axis);
        // Circle lying in the plane perpendicular to the axis
        let rotation = Quat::from_rotation_arc(Vec3::Z, normal);
        gizmos.circle(Isometry3d::new(origin, rotation), size, color);
    }
}

fn draw_scale_gizmo(gizmos: &mut Gizmos, origin: Vec3, size: f32, state: &GizmoState) {
    let axes = [
        (GizmoAxis::X, Vec3::X),
        (GizmoAxis::Y, Vec3::Y),
        (GizmoAxis::Z, Vec3::Z),
    ];

    for &(axis, dir) in &axes {
        let color = axis_color(axis, state.hovered_axis == axis, state.active_axis == axis);
        let end = origin + dir * size;

        gizmos.line(origin, end, color);

        // Small cube at the tip
        let tip_size = size * 0.1;
        gizmos.primitive_3d(
            &Cuboid::from_size(Vec3::splat(tip_size)),
            Isometry3d::from_translation(end),
            color,
        );
    }

    // Uniform scale cube in the center
    let cube_size = size * 0.1;
    gizmos.primitive_3d(
        &Cuboid::from_size(Vec3::splat(cube_size * 2.0)),
        Isometry3d::from_translation(origin),
        Color::srgb(0.9, 0.9, 0.9),
    );
}
