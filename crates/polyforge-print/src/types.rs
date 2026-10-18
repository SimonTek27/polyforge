//! Core data types for the 3D printing pipeline.
//!
//! Geometry is stored in f64 to keep millimetre precision stable while
//! slicing (the viewport works in f32). This mirrors the roles of
//! `3DPrintTypes.h` in ksengine, adapted to Rust idioms.

use serde::{Deserialize, Serialize};

/// Double-precision 2D vector (millimetres).
pub use glam::DVec2 as Vec2;
/// Double-precision 3D vector (millimetres).
pub use glam::DVec3 as Vec3;

/// Axis-aligned bounding box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundingBox {
    pub min: Vec3,
    pub max: Vec3,
}

impl Default for BoundingBox {
    fn default() -> Self {
        Self {
            min: Vec3::splat(f64::MAX),
            max: Vec3::splat(f64::MIN),
        }
    }
}

impl BoundingBox {
    /// Empty (invalid) bounds that expand into a real box.
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn is_valid(&self) -> bool {
        self.min.cmple(self.max).all()
    }

    pub fn size(&self) -> Vec3 {
        if self.is_valid() {
            self.max - self.min
        } else {
            Vec3::ZERO
        }
    }

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    /// Grow the box to include `p`.
    pub fn expand_point(&mut self, p: Vec3) {
        self.min = self.min.min(p);
        self.max = self.max.max(p);
    }

    pub fn expand(&mut self, other: &BoundingBox) {
        if other.is_valid() {
            self.expand_point(other.min);
            self.expand_point(other.max);
        }
    }

    pub fn contains(&self, p: Vec3) -> bool {
        p.cmpge(self.min).all() && p.cmple(self.max).all()
    }
}

/// A single triangle of the source mesh.
#[derive(Debug, Clone, Copy)]
pub struct Triangle {
    pub v: [Vec3; 3],
    /// Geometric normal, derived on construction.
    pub normal: Vec3,
}

impl Triangle {
    pub fn new(a: Vec3, b: Vec3, c: Vec3) -> Self {
        let normal = (b - a).cross(c - a).normalize_or_zero();
        Self {
            v: [a, b, c],
            normal,
        }
    }

    pub fn min_z(&self) -> f64 {
        self.v[0].z.min(self.v[1].z).min(self.v[2].z)
    }

    pub fn max_z(&self) -> f64 {
        self.v[0].z.max(self.v[1].z).max(self.v[2].z)
    }

    /// Does the triangle straddle the horizontal plane at `z`?
    pub fn intersects_plane(&self, z: f64) -> bool {
        self.min_z() <= z && self.max_z() >= z
    }

    pub fn bounds(&self) -> BoundingBox {
        let mut bb = BoundingBox::empty();
        for p in self.v {
            bb.expand_point(p);
        }
        bb
    }
}

/// A triangle soup ready to be sliced.
pub type MeshTriangles = Vec<Triangle>;

/// Closed 2D polygon with optional holes (CCW outer, CW holes).
#[derive(Debug, Clone, Default)]
pub struct Polygon2D {
    pub vertices: Vec<Vec2>,
    pub holes: Vec<Vec<Vec2>>,
}

impl Polygon2D {
    pub fn new(vertices: Vec<Vec2>) -> Self {
        Self {
            vertices,
            holes: Vec::new(),
        }
    }

    pub fn is_valid(&self) -> bool {
        self.vertices.len() >= 3
    }

    pub fn bounds(&self) -> Option<BoundingBox> {
        if self.vertices.is_empty() {
            return None;
        }
        let mut bb = BoundingBox::empty();
        for v in &self.vertices {
            bb.expand_point(Vec3::new(v.x, v.y, 0.0));
        }
        Some(bb)
    }
}

pub type Polygons2D = Vec<Polygon2D>;

/// One horizontal slice of the model.
#[derive(Debug, Clone, Default)]
pub struct LayerSlice {
    /// Z position of the layer centre (mm).
    pub z: f64,
    /// Layer thickness (mm).
    pub thickness: f64,
    pub perimeters: Polygons2D,
    pub infill: Polygons2D,
    pub support: Polygons2D,
    pub brim: Polygons2D,
    pub layer_index: usize,
    pub is_first_layer: bool,
    pub is_last_layer: bool,
    /// Estimated print time for this layer (seconds).
    pub print_time_estimate: f64,
    /// Estimated filament consumed (mm of 1.75 mm stock).
    pub filament_used: f64,
}

pub type LayerSlices = Vec<LayerSlice>;

// ============================================================================
// Enumerations
// ============================================================================

/// G-code dialect understood by the target firmware.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum GCodeFlavor {
    #[default]
    Marlin,
    RepRap,
    Klipper,
    Bambu,
    Prusa,
    Smoothieware,
    Mach3,
}

impl GCodeFlavor {
    /// Klipper/RepRap emit relative extrusion (`M83`).
    pub fn uses_relative_extrusion(self) -> bool {
        matches!(self, GCodeFlavor::Klipper | GCodeFlavor::RepRap)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum InfillPattern {
    #[default]
    Grid,
    Lines,
    Triangles,
    Concentric,
    Gyroid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SupportType {
    #[default]
    None,
    Normal,
    Tree,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BedAdhesionType {
    #[default]
    Skirt,
    None,
    Brim,
    Raft,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum QualityProfile {
    Draft,
    #[default]
    Standard,
    High,
    Ultra,
    Custom,
}

impl QualityProfile {
    /// Reference layer height for the profile (mm).
    pub fn layer_height(self) -> f64 {
        match self {
            QualityProfile::Draft => 0.3,
            QualityProfile::Standard => 0.2,
            QualityProfile::High => 0.15,
            QualityProfile::Ultra => 0.1,
            QualityProfile::Custom => 0.2,
        }
    }
}

// ============================================================================
// Slice settings
// ============================================================================

/// Full slicing configuration. Defaults track a generic 0.4 mm / 1.75 mm PLA
/// machine (same baseline as ksengine's `SliceSettings`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SliceSettings {
    pub printer_profile_id: String,
    pub gcode_flavor: GCodeFlavor,

    // Quality
    pub quality: QualityProfile,
    pub layer_height: f64,
    pub initial_layer_height: f64,
    pub line_width: f64,

    // Walls
    pub wall_count: u32,
    pub top_solid_layers: u32,
    pub bottom_solid_layers: u32,

    // Infill
    pub infill_pattern: InfillPattern,
    /// 0.0 - 1.0
    pub infill_density: f64,
    pub infill_pattern_angle: f64,

    // Extrusion
    pub nozzle_diameter: f64,
    pub filament_diameter: f64,

    // Speeds (mm/s)
    pub print_speed: f64,
    pub outer_wall_speed: f64,
    pub inner_wall_speed: f64,
    pub infill_speed: f64,
    pub support_speed: f64,
    pub travel_speed: f64,
    pub initial_layer_speed: f64,

    // Temperatures (C)
    pub nozzle_temp: i32,
    pub initial_nozzle_temp: i32,
    pub bed_temp: i32,
    pub initial_bed_temp: i32,

    // Cooling (fan speed percentage, 0-100)
    pub cooling_enabled: bool,
    pub fan_speed: u8,
    pub initial_fan_speed: u8,
    pub fan_start_layer: f64,

    // Retraction
    pub retraction_enabled: bool,
    pub retraction_distance: f64,
    pub retraction_speed: f64,
    pub z_hop_enabled: bool,
    pub z_hop_height: f64,

    // Supports
    pub generate_support: bool,
    pub support_type: SupportType,
    pub support_angle: f64,

    // Bed adhesion
    pub adhesion_type: BedAdhesionType,
    pub skirt_loops: u32,
    pub skirt_distance: f64,
    pub brim_width: f64,

    // Output
    pub output_filename: String,
    pub include_comments: bool,
}

impl Default for SliceSettings {
    fn default() -> Self {
        Self {
            printer_profile_id: String::new(),
            gcode_flavor: GCodeFlavor::Marlin,

            quality: QualityProfile::Standard,
            layer_height: 0.2,
            initial_layer_height: 0.2,
            line_width: 0.4,

            wall_count: 2,
            top_solid_layers: 4,
            bottom_solid_layers: 4,

            infill_pattern: InfillPattern::Grid,
            infill_density: 0.15,
            infill_pattern_angle: 0.0,

            nozzle_diameter: 0.4,
            filament_diameter: 1.75,

            print_speed: 50.0,
            outer_wall_speed: 30.0,
            inner_wall_speed: 50.0,
            infill_speed: 60.0,
            support_speed: 40.0,
            travel_speed: 150.0,
            initial_layer_speed: 20.0,

            nozzle_temp: 210,
            initial_nozzle_temp: 215,
            bed_temp: 60,
            initial_bed_temp: 65,

            cooling_enabled: true,
            fan_speed: 100,
            initial_fan_speed: 0,
            fan_start_layer: 2.0,

            retraction_enabled: true,
            retraction_distance: 5.0,
            retraction_speed: 40.0,
            z_hop_enabled: false,
            z_hop_height: 0.2,

            generate_support: false,
            support_type: SupportType::None,
            support_angle: 45.0,

            adhesion_type: BedAdhesionType::Skirt,
            skirt_loops: 1,
            skirt_distance: 2.0,
            brim_width: 5.0,

            output_filename: String::new(),
            include_comments: true,
        }
    }
}

impl SliceSettings {
    /// A settings bundle tuned for a named material preset.
    pub fn for_material(material: &str) -> Self {
        let mut s = Self::default();
        match material.to_ascii_uppercase().as_str() {
            "PLA" => {
                s.nozzle_temp = 210;
                s.bed_temp = 60;
            }
            "PETG" => {
                s.nozzle_temp = 240;
                s.bed_temp = 80;
                s.print_speed = 40.0;
            }
            "ABS" | "ASA" => {
                s.nozzle_temp = 250;
                s.bed_temp = 100;
                s.cooling_enabled = false;
            }
            "TPU" => {
                s.nozzle_temp = 230;
                s.bed_temp = 45;
                s.print_speed = 25.0;
            }
            _ => {}
        }
        s
    }

    /// Apply a quality preset to the layer height.
    pub fn with_quality(mut self, q: QualityProfile) -> Self {
        if q != QualityProfile::Custom {
            self.layer_height = q.layer_height();
            self.initial_layer_height = q.layer_height();
            self.quality = q;
        }
        self
    }

    /// Validate obvious inconsistencies (returns a human readable error).
    pub fn validate(&self) -> Result<(), String> {
        if self.layer_height <= 0.0 {
            return Err("layer_height must be > 0".into());
        }
        if self.nozzle_diameter <= 0.0 {
            return Err("nozzle_diameter must be > 0".into());
        }
        if self.filament_diameter <= 0.0 {
            return Err("filament_diameter must be > 0".into());
        }
        if !(0.0..=1.0).contains(&self.infill_density) {
            return Err("infill_density must be within 0.0 - 1.0".into());
        }
        Ok(())
    }
}

// ============================================================================
// Slice result
// ============================================================================

/// Aggregated per-layer statistics.
#[derive(Debug, Clone, Default)]
pub struct LayerInfo {
    pub z: f64,
    pub print_time: f64,
    pub filament_used: f64,
    pub perimeter_count: usize,
    pub infill_polygons: usize,
    pub support_polygons: usize,
}

/// The complete output of a slicing run.
#[derive(Debug, Clone, Default)]
pub struct SliceInfo {
    pub printer_profile_id: String,
    pub settings: SliceSettings,
    pub bounding_box: BoundingBox,

    pub total_layers: usize,
    pub print_time: f64,
    pub filament_used: f64,

    pub layer_heights: Vec<f64>,
    pub layers: LayerSlices,
    pub layer_infos: Vec<LayerInfo>,

    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub success: bool,
}

impl SliceInfo {
    pub fn failed(msg: impl Into<String>) -> Self {
        Self {
            errors: vec![msg.into()],
            success: false,
            ..Default::default()
        }
    }
}
