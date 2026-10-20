//! Mesh slicer: turns a triangle soup into per-layer 2D geometry.
//!
//! Port of ksengine's `SlicerEngine` to Rust. The pipeline is:
//! bounds -> build-volume check -> layer heights -> mesh/plane intersection
//! -> supports -> perimeters -> infill -> bed adhesion -> per-layer statistics.

use crate::geometry;
use crate::profile::PrinterProfile;
use crate::types::{
    BedAdhesionType, BoundingBox, InfillPattern, LayerInfo, LayerSlice, MeshTriangles, Polygon2D,
    Polygons2D, SliceInfo, SliceSettings, SupportType, Triangle, Vec2, Vec3,
};

/// Stateless slicer; settings and printer are passed per call.
#[derive(Debug, Default, Clone, Copy)]
pub struct SlicerEngine;

impl SlicerEngine {
    pub fn new() -> Self {
        Self
    }

    /// Slice a triangle mesh with the given settings and printer.
    pub fn slice(
        &self,
        triangles: &MeshTriangles,
        settings: &SliceSettings,
        printer: &PrinterProfile,
    ) -> SliceInfo {
        if triangles.is_empty() {
            return SliceInfo::failed("empty mesh");
        }
        if let Err(e) = settings.validate() {
            return SliceInfo::failed(format!("invalid settings: {e}"));
        }

        let bounds = triangles_bounds(triangles);

        if let Err(e) = self.check_build_volume(&bounds, printer) {
            return SliceInfo::failed(e);
        }

        let layer_heights = self.generate_layer_heights(&bounds, settings);
        if layer_heights.is_empty() {
            return SliceInfo::failed("model produced no layers (height below first layer)");
        }

        let total = layer_heights.len();
        let cross_sections: Vec<Polygons2D> = layer_heights
            .iter()
            .map(|&z| self.intersect_mesh_at_z(triangles, z))
            .collect();

        let supports = if settings.generate_support
            && settings.support_type != SupportType::None
        {
            self.generate_supports(&cross_sections, &bounds, settings)
        } else {
            vec![Vec::new(); total]
        };

        let mut layers: Vec<LayerSlice> = Vec::with_capacity(total);
        for (i, &z) in layer_heights.iter().enumerate() {
            let thickness = if i == 0 {
                settings.initial_layer_height
            } else {
                settings.layer_height
            };
            layers.push(self.process_layer(
                &cross_sections[i],
                &supports[i],
                z,
                thickness,
                settings,
                i,
                total,
            ));
        }

        // Aggregate statistics.
        let mut layer_infos = Vec::with_capacity(layers.len());
        let mut total_time = 0.0;
        let mut total_filament = 0.0;
        for layer in &layers {
            total_time += layer.print_time_estimate;
            total_filament += layer.filament_used;
            layer_infos.push(LayerInfo {
                z: layer.z,
                print_time: layer.print_time_estimate,
                filament_used: layer.filament_used,
                perimeter_count: layer.perimeters.len(),
                infill_polygons: layer.infill.len(),
                support_polygons: layer.support.len(),
            });
        }

        SliceInfo {
            printer_profile_id: printer.id.clone(),
            settings: settings.clone(),
            bounding_box: bounds,
            total_layers: layers.len(),
            print_time: total_time,
            filament_used: total_filament,
            layer_heights,
            layers,
            layer_infos,
            warnings: Vec::new(),
            errors: Vec::new(),
            success: true,
        }
    }

    /// Z centres of every layer, from the first-layer height upward.
    pub fn generate_layer_heights(
        &self,
        bounds: &BoundingBox,
        settings: &SliceSettings,
    ) -> Vec<f64> {
        let mut heights = Vec::new();
        if !bounds.is_valid() {
            return heights;
        }

        let min_z = bounds.min.z;
        let max_z = bounds.max.z;
        let first = if settings.initial_layer_height > 0.0 {
            settings.initial_layer_height
        } else {
            settings.layer_height
        };

        let first_center = min_z + first * 0.5;
        if first_center <= max_z {
            heights.push(first_center);
        }

        let mut current = min_z + first + settings.layer_height * 0.5;
        while current <= max_z + 1e-6 {
            heights.push(current);
            current += settings.layer_height;
        }

        heights
    }

    /// Intersect the mesh with the horizontal plane at `z`, returning closed
    /// 2D polygons.
    pub fn intersect_mesh_at_z(&self, triangles: &MeshTriangles, z: f64) -> Polygons2D {
        let mut segments: Vec<(Vec2, Vec2)> = Vec::with_capacity(triangles.len());
        for tri in triangles {
            if !tri.intersects_plane(z) {
                continue;
            }
            if let Some(seg) = intersect_triangle_plane(tri, z) {
                segments.push(seg);
            }
        }
        if segments.is_empty() {
            return Vec::new();
        }
        reconstruct_polygons(&segments)
    }

    /// Outer cross-section plus `wall_count - 1` inward-offset copies.
    pub fn generate_perimeters(
        &self,
        cross_section: &Polygons2D,
        settings: &SliceSettings,
    ) -> Polygons2D {
        let mut perimeters = cross_section.clone();
        let line_width = if settings.line_width > 0.0 {
            settings.line_width
        } else {
            settings.nozzle_diameter
        };
        let half = line_width * 0.5;

        for w in 1..settings.wall_count {
            let inner = geometry::offset_polygons(&perimeters, -half * w as f64);
            if inner.is_empty() {
                break;
            }
            perimeters.extend(inner);
        }

        perimeters
    }

    /// Infill lines for a layer, clipped to the given perimeters.
    pub fn generate_infill(
        &self,
        perimeters: &Polygons2D,
        settings: &SliceSettings,
        layer_index: usize,
        solid: bool,
    ) -> Polygons2D {
        if !solid && settings.infill_density <= 0.0 {
            return Vec::new();
        }

        let line_width = if settings.line_width > 0.0 {
            settings.line_width
        } else {
            settings.nozzle_diameter
        };
        let density = if solid { 1.0 } else { settings.infill_density };

        // density = line_width / spacing  =>  spacing = line_width / density
        let spacing = line_width / density.max(1e-3);

        let mut angle = settings.infill_pattern_angle.to_radians();
        if layer_index % 2 == 1 {
            angle += std::f64::consts::FRAC_PI_2;
        }

        geometry::generate_infill(settings.infill_pattern, perimeters, spacing, angle)
    }

    /// Brim / skirt around the first layer.
    pub fn generate_bed_adhesion(
        &self,
        first_layer_perimeters: &Polygons2D,
        settings: &SliceSettings,
    ) -> Polygons2D {
        let mut result = Vec::new();
        let line_width = if settings.line_width > 0.0 {
            settings.line_width
        } else {
            settings.nozzle_diameter
        };

        match settings.adhesion_type {
            BedAdhesionType::Brim => {
                if settings.brim_width > 0.0 {
                    let brim_lines = (settings.brim_width / line_width) as i32;
                    for i in 1..=brim_lines {
                        let brim = geometry::offset_polygons(
                            first_layer_perimeters,
                            line_width * i as f64,
                        );
                        result.extend(brim);
                    }
                }
            }
            BedAdhesionType::Skirt => {
                if settings.skirt_loops > 0 {
                    let distance = if settings.skirt_distance > 0.0 {
                        settings.skirt_distance
                    } else {
                        line_width * 2.0
                    };
                    let mut skirt = geometry::offset_polygons(first_layer_perimeters, distance);
                    for i in 1..settings.skirt_loops {
                        let mut more = geometry::offset_polygons(&skirt, line_width * i as f64);
                        skirt.append(&mut more);
                    }
                    result.extend(skirt);
                }
            }
            BedAdhesionType::Raft | BedAdhesionType::None => {}
        }

        result
    }

    /// Confirm the bounding box fits the printer's build volume.
    pub fn check_build_volume(
        &self,
        bounds: &BoundingBox,
        printer: &PrinterProfile,
    ) -> Result<(), String> {
        if !bounds.is_valid() {
            return Ok(());
        }
        let size = bounds.size();
        if size.x > printer.build_volume_x {
            return Err(format!(
                "model width {:.1} mm exceeds printer max {:.1} mm",
                size.x, printer.build_volume_x
            ));
        }
        if size.y > printer.build_volume_y {
            return Err(format!(
                "model depth {:.1} mm exceeds printer max {:.1} mm",
                size.y, printer.build_volume_y
            ));
        }
        if size.z > printer.build_volume_z {
            return Err(format!(
                "model height {:.1} mm exceeds printer max {:.1} mm",
                size.z, printer.build_volume_z
            ));
        }
        Ok(())
    }

    /// Volume-based print time estimate (seconds) without full slicing.
    pub fn estimate_print_time_quick(
        &self,
        triangles: &MeshTriangles,
        settings: &SliceSettings,
    ) -> f64 {
        let volume = mesh_volume(triangles);
        let density = settings.infill_density.clamp(0.0, 1.0);
        let filament_volume = volume * density * 1.2;
        let area = std::f64::consts::PI * (settings.filament_diameter * 0.5).powi(2);
        let filament_length = if area > 0.0 {
            filament_volume / area
        } else {
            0.0
        };
        let speed = if settings.print_speed > 0.0 {
            settings.print_speed
        } else {
            50.0
        };
        filament_length / speed * 1.5
    }

    fn process_layer(
        &self,
        cross_section: &Polygons2D,
        support: &Polygons2D,
        z: f64,
        thickness: f64,
        settings: &SliceSettings,
        layer_index: usize,
        total_layers: usize,
    ) -> LayerSlice {
        let mut layer = LayerSlice {
            z,
            thickness,
            layer_index,
            is_first_layer: layer_index == 0,
            is_last_layer: layer_index == total_layers - 1,
            ..Default::default()
        };

        if cross_section.is_empty() && support.is_empty() {
            return layer;
        }

        layer.perimeters = self.generate_perimeters(cross_section, settings);

        if !cross_section.is_empty() {
            let bottom = settings.bottom_solid_layers as usize;
            let top = settings.top_solid_layers as usize;
            let is_bottom = layer_index < bottom;
            let is_top = layer_index + top >= total_layers;
            if is_bottom || is_top {
                let mut solid_settings = settings.clone();
                solid_settings.infill_pattern = InfillPattern::Lines;
                layer.infill = self.generate_infill(
                    &layer.perimeters,
                    &solid_settings,
                    layer_index,
                    true,
                );
            } else {
                layer.infill =
                    self.generate_infill(&layer.perimeters, settings, layer_index, false);
            }
        }

        if !support.is_empty() {
            layer.support = support.clone();
        }

        if layer.is_first_layer {
            layer.brim = self.generate_bed_adhesion(&layer.perimeters, settings);
        }

        self.estimate_layer(&mut layer, settings);
        layer
    }

    /// Overhang-driven support columns, one polygon set per layer.
    ///
    /// A cell is unsupported when the layer below has no material within the
    /// horizontal reach implied by `support_angle`. Unsupported cells are then
    /// propagated downward so every overhang is carried to the bed.
    pub fn generate_supports(
        &self,
        cross_sections: &[Polygons2D],
        bounds: &BoundingBox,
        settings: &SliceSettings,
    ) -> Vec<Polygons2D> {
        let n = cross_sections.len();
        if n == 0 || !bounds.is_valid() {
            return Vec::new();
        }

        let line_width = if settings.line_width > 0.0 {
            settings.line_width
        } else {
            settings.nozzle_diameter
        };
        let size = bounds.size();
        let cell = (line_width * 2.0)
            .max(size.x.max(size.y) / 256.0)
            .max(0.8);

        let min_x = bounds.min.x;
        let min_y = bounds.min.y;
        let cols = (((size.x) / cell).ceil() as usize + 1).max(1);
        let rows = (((size.y) / cell).ceil() as usize + 1).max(1);

        // Rasterise every cross-section into a boolean occupancy grid.
        let occupancy: Vec<Vec<bool>> = cross_sections
            .iter()
            .map(|polys| {
                let mut grid = vec![false; cols * rows];
                if polys.is_empty() {
                    return grid;
                }
                for r in 0..rows {
                    for c in 0..cols {
                        let p = Vec2::new(
                            min_x + (c as f64 + 0.5) * cell,
                            min_y + (r as f64 + 0.5) * cell,
                        );
                        grid[r * cols + c] = polys
                            .iter()
                            .any(|poly| geometry::point_in_polygon(p, &poly.vertices));
                    }
                }
                grid
            })
            .collect();

        let angle = settings.support_angle.clamp(5.0, 89.0).to_radians();
        let reach = (settings.layer_height / angle.tan()).max(cell);
        let radius = (reach / cell).ceil() as isize;

        let mut support: Vec<Vec<bool>> = vec![vec![false; cols * rows]; n];
        for i in (0..n).rev() {
            let mut layer = vec![false; cols * rows];
            for idx in 0..cols * rows {
                if occupancy[i][idx] {
                    continue;
                }
                let needed_above = if i + 1 < n { support[i + 1][idx] } else { false };
                let unsupported = i > 0
                    && !dilated_at(&occupancy[i - 1], idx, cols, rows, radius);
                if needed_above || unsupported {
                    layer[idx] = true;
                }
            }
            support[i] = layer;
        }

        support
            .iter()
            .map(|grid| grid_to_polygons(grid, cols, rows, min_x, min_y, cell))
            .collect()
    }

    fn estimate_layer(&self, layer: &mut LayerSlice, settings: &SliceSettings) {
        let perimeter_length: f64 = layer
            .perimeters
            .iter()
            .map(|p| path_length(&p.vertices, true))
            .sum();
        let infill_length: f64 = layer
            .infill
            .iter()
            .map(|p| path_length(&p.vertices, false))
            .sum();
        let support_length: f64 = layer
            .support
            .iter()
            .map(|p| path_length(&p.vertices, true))
            .sum();

        let print_speed = if layer.is_first_layer {
            settings.initial_layer_speed
        } else {
            settings.print_speed
        };
        let infill_speed = if settings.infill_speed > 0.0 {
            settings.infill_speed
        } else {
            print_speed
        };
        let support_speed = if settings.support_speed > 0.0 {
            settings.support_speed
        } else {
            print_speed
        };
        let travel_speed = if settings.travel_speed > 0.0 {
            settings.travel_speed
        } else {
            print_speed * 2.0
        };

        let line_width = if settings.line_width > 0.0 {
            settings.line_width
        } else {
            settings.nozzle_diameter
        };
        let filament_diameter = if settings.filament_diameter > 0.0 {
            settings.filament_diameter
        } else {
            1.75
        };
        let filament_area = line_width * layer.thickness;
        let filament_cross_section =
            std::f64::consts::PI * filament_diameter * filament_diameter / 4.0;

        let mut print_time = 0.0;
        if print_speed > 0.0 {
            print_time += perimeter_length / print_speed;
        }
        if infill_speed > 0.0 {
            print_time += infill_length / infill_speed;
        }
        if support_speed > 0.0 {
            print_time += support_length / support_speed;
        }
        let travel_estimate = (perimeter_length + infill_length + support_length) * 0.1;
        if travel_speed > 0.0 {
            print_time += travel_estimate / travel_speed;
        }

        layer.print_time_estimate = print_time;
        if filament_cross_section > 0.0 {
            layer.filament_used = (perimeter_length + infill_length + support_length)
                * filament_area
                / filament_cross_section;
        }
    }
}

/// Is any occupied cell within `radius` cells of `idx`?
fn dilated_at(grid: &[bool], idx: usize, cols: usize, rows: usize, radius: isize) -> bool {
    if radius < 0 {
        return false;
    }
    let c0 = (idx % cols) as isize;
    let r0 = (idx / cols) as isize;
    for dr in -radius..=radius {
        let r = r0 + dr;
        if r < 0 || r >= rows as isize {
            continue;
        }
        for dc in -radius..=radius {
            let c = c0 + dc;
            if c < 0 || c >= cols as isize {
                continue;
            }
            if grid[r as usize * cols + c as usize] {
                return true;
            }
        }
    }
    false
}

/// Merge horizontal runs of occupied cells into rectangles.
fn grid_to_polygons(
    grid: &[bool],
    cols: usize,
    rows: usize,
    min_x: f64,
    min_y: f64,
    cell: f64,
) -> Polygons2D {
    let mut polys = Vec::new();
    for r in 0..rows {
        let mut c = 0;
        while c < cols {
            if !grid[r * cols + c] {
                c += 1;
                continue;
            }
            let start = c;
            while c < cols && grid[r * cols + c] {
                c += 1;
            }
            let x0 = min_x + start as f64 * cell;
            let x1 = min_x + c as f64 * cell;
            let y0 = min_y + r as f64 * cell;
            let y1 = y0 + cell;
            polys.push(Polygon2D::new(vec![
                Vec2::new(x0, y0),
                Vec2::new(x1, y0),
                Vec2::new(x1, y1),
                Vec2::new(x0, y1),
            ]));
        }
    }
    polys
}

fn path_length(verts: &[Vec2], closed: bool) -> f64 {
    if verts.len() < 2 {
        return 0.0;
    }
    let mut len = 0.0;
    for w in verts.windows(2) {
        len += (w[1] - w[0]).length();
    }
    if closed {
        len += (verts[0] - verts[verts.len() - 1]).length();
    }
    len
}

pub fn triangles_bounds(triangles: &MeshTriangles) -> BoundingBox {
    let mut bb = BoundingBox::empty();
    for tri in triangles {
        for v in tri.v {
            bb.expand_point(v);
        }
    }
    bb
}

/// Signed volume of a closed triangle mesh (tetrahedra about the origin).
pub fn mesh_volume(triangles: &MeshTriangles) -> f64 {
    let mut volume = 0.0;
    for tri in triangles {
        let [a, b, c] = tri.v;
        volume += a.dot(b.cross(c)) / 6.0;
    }
    volume.abs()
}

/// Intersect one triangle with plane `z`, yielding the crossing segment.
fn intersect_triangle_plane(tri: &Triangle, z: f64) -> Option<(Vec2, Vec2)> {
    let zv = [tri.v[0].z - z, tri.v[1].z - z, tri.v[2].z - z];
    let pos = zv.iter().filter(|v| **v > 0.0).count();
    let neg = zv.iter().filter(|v| **v < 0.0).count();
    if pos == 3 || neg == 3 {
        return None; // entirely on one side
    }

    let on_plane: Vec<usize> = (0..3).filter(|&i| zv[i].abs() < 1e-10).collect();

    if on_plane.len() == 3 {
        return None; // degenerate, coplanar
    }
    if on_plane.len() == 2 {
        let a = tri.v[on_plane[0]];
        let b = tri.v[on_plane[1]];
        return Some((Vec2::new(a.x, a.y), Vec2::new(b.x, b.y)));
    }
    if on_plane.len() == 1 {
        let i = on_plane[0];
        let other: Vec<usize> = (0..3).filter(|&k| k != i).collect();
        let a = tri.v[i];
        // Project along the crossing edge (the opposite vertex).
        let b = tri.v[other[0]];
        return Some((Vec2::new(a.x, a.y), Vec2::new(b.x, b.y)));
    }

    let mut hits: Vec<Vec2> = Vec::with_capacity(2);
    for i in 0..3usize {
        let j = (i + 1) % 3;
        let (zi, zj) = (zv[i], zv[j]);
        if (zi > 0.0 && zj < 0.0) || (zi < 0.0 && zj > 0.0) {
            let t = zi / (zi - zj);
            let p = tri.v[i] + (tri.v[j] - tri.v[i]) * t;
            hits.push(Vec2::new(p.x, p.y));
        }
    }

    if hits.len() == 2 {
        Some((hits[0], hits[1]))
    } else {
        None
    }
}

/// Stitch unordered crossing segments into closed polygons.
fn reconstruct_polygons(segments: &[(Vec2, Vec2)]) -> Polygons2D {
    let mut result = Vec::new();
    let mut used = vec![false; segments.len()];
    let tol = 1e-6;

    for i in 0..segments.len() {
        if used[i] {
            continue;
        }
        used[i] = true;
        let (a, b) = segments[i];
        let start = a;
        let mut current = b;
        let mut vertices = vec![a, b];
        let mut closed = false;

        loop {
            if (current - start).length() < tol {
                closed = true;
                break;
            }
            let mut advanced = false;
            for j in 0..segments.len() {
                if used[j] {
                    continue;
                }
                let (c, d) = segments[j];
                if (c - current).length() < tol {
                    vertices.push(d);
                    current = d;
                    used[j] = true;
                    advanced = true;
                    break;
                } else if (d - current).length() < tol {
                    vertices.push(c);
                    current = c;
                    used[j] = true;
                    advanced = true;
                    break;
                }
            }
            if !advanced {
                break;
            }
        }

        if closed
            && vertices.len() > 1
            && (vertices[0] - vertices[vertices.len() - 1]).length() < tol
        {
            vertices.pop();
        }

        if vertices.len() >= 3 {
            let ccw = geometry::ensure_ccw(vertices);
            result.push(Polygon2D::new(ccw));
        }
    }

    result
}

/// Replace the Z-up expectation with the slicer's plane: `Vec3` is untouched.
#[allow(dead_code)]
fn _assert_vec3(_: Vec3) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube(size: f64) -> MeshTriangles {
        let s = size * 0.5;
        let v = [
            Vec3::new(-s, -s, -s),
            Vec3::new(s, -s, -s),
            Vec3::new(s, s, -s),
            Vec3::new(-s, s, -s),
            Vec3::new(-s, -s, s),
            Vec3::new(s, -s, s),
            Vec3::new(s, s, s),
            Vec3::new(-s, s, s),
        ];
        let faces = [
            [0, 1, 2, 3], // -Z
            [5, 4, 7, 6], // +Z handled below
            [4, 0, 3, 7],
            [1, 5, 6, 2],
            [3, 2, 6, 7],
            [4, 5, 1, 0],
        ];
        let mut tris = Vec::new();
        for f in faces {
            let (a, b, c, d) = (v[f[0]], v[f[1]], v[f[2]], v[f[3]]);
            tris.push(Triangle::new(a, b, c));
            tris.push(Triangle::new(a, c, d));
        }
        tris
    }

    #[test]
    fn layer_heights_for_cube() {
        let engine = SlicerEngine::new();
        let tris = cube(20.0);
        let bounds = triangles_bounds(&tris);
        let settings = SliceSettings::default();
        let heights = engine.generate_layer_heights(&bounds, &settings);
        // 20 mm tall, 0.2 mm layers -> ~100 layers.
        assert!(
            heights.len() >= 99 && heights.len() <= 101,
            "got {}",
            heights.len()
        );
    }

    #[test]
    fn cross_section_is_square() {
        let engine = SlicerEngine::new();
        let tris = cube(20.0);
        let polys = engine.intersect_mesh_at_z(&tris, 0.0);
        assert_eq!(polys.len(), 1, "expected a single square outline");
        let area = geometry::polygon_area(&polys[0].vertices).abs();
        assert!((area - 400.0).abs() < 1.0, "area was {area}");
    }

    #[test]
    fn full_slice_succeeds() {
        let engine = SlicerEngine::new();
        let tris = cube(20.0);
        let settings = SliceSettings::default();
        let printer = PrinterProfile::create_builtin("generic_marlin").unwrap();
        let info = engine.slice(&tris, &settings, &printer);
        assert!(info.success, "errors: {:?}", info.errors);
        assert!(info.total_layers > 90);
        assert!(info.filament_used > 0.0);
        assert!(info.print_time > 0.0);
    }

    #[test]
    fn build_volume_rejected() {
        let engine = SlicerEngine::new();
        let tris = cube(400.0);
        let printer = PrinterProfile::create_builtin("prusa_mini").unwrap();
        let info = engine.slice(&tris, &SliceSettings::default(), &printer);
        assert!(!info.success);
    }

    /// An upside-down T: a wide plate floating above a thin pillar.
    fn overhang_mesh() -> MeshTriangles {
        let mut tris = Vec::new();
        // Pillar 4 x 4 x 10 (z 0..10).
        let (px, py, pz) = (2.0, 2.0, 10.0);
        let v = [
            Vec3::new(-px, -py, 0.0),
            Vec3::new(px, -py, 0.0),
            Vec3::new(px, py, 0.0),
            Vec3::new(-px, py, 0.0),
            Vec3::new(-px, -py, pz),
            Vec3::new(px, -py, pz),
            Vec3::new(px, py, pz),
            Vec3::new(-px, py, pz),
        ];
        for f in [
            [0, 1, 2, 3],
            [5, 4, 7, 6],
            [4, 0, 3, 7],
            [1, 5, 6, 2],
            [3, 2, 6, 7],
            [4, 5, 1, 0],
        ] {
            tris.push(Triangle::new(v[f[0]], v[f[1]], v[f[2]]));
            tris.push(Triangle::new(v[f[0]], v[f[2]], v[f[3]]));
        }
        // Plate 20 x 20 x 2 (z 10..12) wider than the pillar.
        let (qx, qy) = (10.0, 10.0);
        let w = [
            Vec3::new(-qx, -qy, pz),
            Vec3::new(qx, -qy, pz),
            Vec3::new(qx, qy, pz),
            Vec3::new(-qx, qy, pz),
            Vec3::new(-qx, -qy, pz + 2.0),
            Vec3::new(qx, -qy, pz + 2.0),
            Vec3::new(qx, qy, pz + 2.0),
            Vec3::new(-qx, qy, pz + 2.0),
        ];
        for f in [
            [0, 1, 2, 3],
            [5, 4, 7, 6],
            [4, 0, 3, 7],
            [1, 5, 6, 2],
            [3, 2, 6, 7],
            [4, 5, 1, 0],
        ] {
            tris.push(Triangle::new(w[f[0]], w[f[1]], w[f[2]]));
            tris.push(Triangle::new(w[f[0]], w[f[2]], w[f[3]]));
        }
        tris
    }

    #[test]
    fn supports_are_generated_under_overhangs() {
        let engine = SlicerEngine::new();
        let printer = PrinterProfile::create_builtin("generic_marlin").unwrap();
        let mut settings = SliceSettings::default();
        settings.generate_support = true;
        settings.support_type = SupportType::Normal;

        let tris = overhang_mesh();
        let info = engine.slice(&tris, &settings, &printer);
        assert!(info.success, "errors: {:?}", info.errors);

        let support_layers = info
            .layers
            .iter()
            .filter(|l| !l.support.is_empty())
            .count();
        assert!(support_layers > 10, "support layers: {support_layers}");

        // No supports when disabled.
        settings.generate_support = false;
        settings.support_type = SupportType::None;
        let plain = engine.slice(&tris, &settings, &printer);
        assert!(plain.layers.iter().all(|l| l.support.is_empty()));
    }

    #[test]
    fn solid_shells_follow_layer_counts() {
        let engine = SlicerEngine::new();
        let printer = PrinterProfile::create_builtin("generic_marlin").unwrap();
        let mut settings = SliceSettings::default();
        settings.infill_density = 0.1;
        settings.bottom_solid_layers = 3;
        settings.top_solid_layers = 5;

        let info = engine.slice(&cube(20.0), &settings, &printer);
        assert!(info.success);
        let n = info.layers.len();
        // Solid layers are denser than sparse infill.
        let count_lines = |l: &LayerSlice| l.infill.len();
        let bottom: usize = (0..3).map(|i| count_lines(&info.layers[i])).sum();
        let top: usize = (n - 5..n).map(|i| count_lines(&info.layers[i])).sum();
        let middle: usize = (5..n / 2)
            .map(|i| count_lines(&info.layers[i]))
            .sum();
        assert!(bottom > middle, "bottom {bottom} vs middle {middle}");
        assert!(top > middle, "top {top} vs middle {middle}");
    }
}
