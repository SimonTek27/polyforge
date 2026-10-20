//! Brush stamping: coverage accumulation, erase and clone.

use crate::raster::RasterImage;
use crate::types::{BlendMode, BrushSettings, PaintTool};

/// Live state for a single continuous stroke.
///
/// Coverage is accumulated per pixel so that overlapping dabs build up to
/// `brush.opacity` without exceeding it (standard stroke-opacity behaviour).
pub struct StrokeSession<'a> {
    width: u32,
    height: u32,
    coverage: Vec<f32>,
    brush: &'a BrushSettings,
    tool: PaintTool,
    blend: BlendMode,
    /// Optional clone source plus the (dx, dy) offset from destination pixel
    /// to the source pixel it samples.
    clone: Option<(&'a RasterImage, (f32, f32))>,
}

impl<'a> StrokeSession<'a> {
    pub fn new(
        canvas: &RasterImage,
        brush: &'a BrushSettings,
        tool: PaintTool,
        blend: BlendMode,
        clone: Option<(&'a RasterImage, (f32, f32))>,
    ) -> Self {
        Self {
            width: canvas.width,
            height: canvas.height,
            coverage: vec![0.0; canvas.len()],
            brush,
            tool,
            blend,
            clone,
        }
    }

    pub fn tool(&self) -> PaintTool {
        self.tool
    }

    /// Layer opacity contributed so far at a pixel (0..=1).
    pub fn coverage_at(&self, x: u32, y: u32) -> f32 {
        if x >= self.width || y >= self.height {
            return 0.0;
        }
        self.coverage[(y * self.width + x) as usize]
    }

    /// Stamp a single dab centred on `(cx, cy)` (pixel space).
    pub fn dab(&mut self, image: &mut RasterImage, cx: f32, cy: f32) {
        let radius = self.brush.radius();
        let x0 = (cx - radius).floor() as i64;
        let x1 = (cx + radius).ceil() as i64;
        let y0 = (cy - radius).floor() as i64;
        let y1 = (cy + radius).ceil() as i64;

        for y in y0..=y1 {
            if y < 0 || y >= self.height as i64 {
                continue;
            }
            for x in x0..=x1 {
                if x < 0 || x >= self.width as i64 {
                    continue;
                }
                let px = x as f32 + 0.5;
                let py = y as f32 + 0.5;
                let dist = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt() / radius;
                let cov = self.brush.falloff(dist);
                if cov <= 0.0 {
                    continue;
                }

                let idx = (y as u32 * self.width + x as u32) as usize;
                let delta = cov * self.brush.flow;
                let old = self.coverage[idx];
                let new = (old + delta).min(1.0);
                self.coverage[idx] = new;

                let alpha = (new - old) * self.brush.opacity;
                if alpha <= 0.0 {
                    continue;
                }
                self.paint_pixel(image, idx, alpha);
            }
        }
    }

    fn paint_pixel(&mut self, image: &mut RasterImage, idx: usize, alpha: f32) {
        match self.tool {
            PaintTool::Eraser => {
                let mut c = image.pixel_at(idx);
                let keep = 1.0 - alpha.clamp(0.0, 1.0);
                c.a = (c.a as f32 * keep).round() as u8;
                image.set_index(idx, c);
            }
            PaintTool::Clone => {
                let src = match self.clone {
                    Some((img, (dx, dy))) => {
                        let x = idx as u32 % self.width;
                        let y = idx as u32 / self.width;
                        img.sample_nearest(
                            (x as f32 + dx).round() as i64,
                            (y as f32 + dy).round() as i64,
                        )
                    }
                    None => self.brush.color,
                };
                let dst = image.pixel_at(idx);
                image.set_index(idx, RasterImage::blend(dst, src, alpha, self.blend));
            }
            PaintTool::Brush | PaintTool::Fill => {
                let dst = image.pixel_at(idx);
                image.set_index(
                    idx,
                    RasterImage::blend(dst, self.brush.color, alpha, self.blend),
                );
            }
        }
    }

    /// Stamp dabs along the segment `from -> to`, spaced by `brush.spacing`.
    ///
    /// Returns the last dab position so callers can chain segments.
    pub fn stroke_to(
        &mut self,
        image: &mut RasterImage,
        from: (f32, f32),
        to: (f32, f32),
    ) -> (f32, f32) {
        let dx = to.0 - from.0;
        let dy = to.1 - from.1;
        let dist = (dx * dx + dy * dy).sqrt();
        let step = (self.brush.size * self.brush.spacing).max(1.0);

        if dist <= f32::EPSILON {
            self.dab(image, to.0, to.1);
            return to;
        }

        let count = (dist / step).ceil() as u32;
        for i in 0..=count {
            let t = i as f32 / count as f32;
            self.dab(image, from.0 + dx * t, from.1 + dy * t);
        }
        to
    }
}

/// Flood the whole image with `color` at `opacity` (Fill tool).
pub fn fill_all(image: &mut RasterImage, color: crate::types::Rgba, opacity: f32, blend: BlendMode) {
    let n = image.len();
    for i in 0..n {
        let dst = image.pixel_at(i);
        image.set_index(i, RasterImage::blend(dst, color, opacity, blend));
    }
}
