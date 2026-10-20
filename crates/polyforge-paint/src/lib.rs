//! # polyforge-paint
//!
//! Texture / raster paint pipeline backing PolyForge's **Paint mode**
//! (`AppMode::Paint`, PhotoCraft companion).
//!
//! The crate is intentionally self-contained: it depends only on Bevy (for the
//! plugin/`Image` bridge) and implements its own RGBA raster, brush engine,
//! layer stack and PNG writer. No external image crate is required.
//!
//! ```
//! use polyforge_paint::{PaintDocument, BrushSettings, PaintTool, BlendMode, StrokeSession};
//!
//! let mut doc = PaintDocument::empty(64, 64);
//! let brush = BrushSettings { size: 10.0, ..Default::default() };
//! let layer = doc.active_layer_mut().unwrap();
//! let mut stroke = StrokeSession::new(&layer.pixels, &brush, PaintTool::Brush, BlendMode::Normal, None);
//! stroke.stroke_to(&mut layer.pixels, (8.0, 8.0), (40.0, 40.0));
//! assert!(layer.pixels.pixels.iter().any(|b| *b != 0));
//! ```

pub mod brush;
pub mod png;
pub mod projection;
pub mod raster;
pub mod types;

pub use brush::{fill_all, StrokeSession};
pub use raster::RasterImage;
pub use types::{BlendMode, BrushSettings, PaintDocument, PaintLayer, PaintTool, Rgba};

use bevy::prelude::*;

/// Global Paint-mode state: the document, brush and active tool.
#[derive(Resource)]
pub struct PaintState {
    pub document: PaintDocument,
    pub brush: BrushSettings,
    pub tool: PaintTool,
    pub blend: BlendMode,
    /// Offset (dx, dy) from destination to clone-source pixel.
    pub clone_offset: (f32, f32),
    /// Set whenever pixels change so the GPU texture can be re-uploaded.
    pub dirty: bool,
}

impl Default for PaintState {
    fn default() -> Self {
        Self {
            document: PaintDocument::default(),
            brush: BrushSettings::default(),
            tool: PaintTool::default(),
            blend: BlendMode::default(),
            clone_offset: (0.0, 0.0),
            dirty: false,
        }
    }
}

impl PaintState {
    pub fn canvas_size(&self) -> (u32, u32) {
        (self.document.width, self.document.height)
    }

    /// Resize/replace the canvas, keeping settings.
    pub fn reset(&mut self, width: u32, height: u32) {
        self.document = PaintDocument::new(width, height);
        self.dirty = true;
    }

    /// Paint a single brush dab at pixel `(x, y)` on the active layer.
    ///
    /// When the active tool is [`PaintTool::Clone`] the layer directly below
    /// the active one is sampled using `clone_offset`.
    pub fn stamp(&mut self, x: f32, y: f32) {
        let PaintState {
            document,
            brush,
            tool,
            blend,
            clone_offset,
            dirty,
            ..
        } = self;

        if document.layers.is_empty() {
            return;
        }
        let active = document.active.min(document.layers.len() - 1);

        // Split the stack so the clone source (layer below) and the target
        // layer can be borrowed at the same time.
        let (below, above) = document.layers.split_at_mut(active);
        let clone = if *tool == PaintTool::Clone {
            below
                .last()
                .map(|l| (&l.pixels, *clone_offset))
        } else {
            None
        };
        let layer = &mut above[0];

        let mut session = StrokeSession::new(&layer.pixels, brush, *tool, *blend, clone);
        session.dab(&mut layer.pixels, x, y);
        *dirty = true;
    }

    /// Stamp a straight stroke between two points on the active layer.
    pub fn stroke(&mut self, from: (f32, f32), to: (f32, f32)) {
        let PaintState {
            document,
            brush,
            tool,
            blend,
            clone_offset,
            dirty,
            ..
        } = self;

        if document.layers.is_empty() {
            return;
        }
        let active = document.active.min(document.layers.len() - 1);
        let (below, above) = document.layers.split_at_mut(active);
        let clone = if *tool == PaintTool::Clone {
            below.last().map(|l| (&l.pixels, *clone_offset))
        } else {
            None
        };
        let layer = &mut above[0];

        let mut session = StrokeSession::new(&layer.pixels, brush, *tool, *blend, clone);
        session.stroke_to(&mut layer.pixels, from, to);
        *dirty = true;
    }

    /// Fill the whole active layer with the brush colour.
    pub fn fill_active(&mut self) {
        if let Some(layer) = self.document.active_layer_mut() {
            fill_all(&mut layer.pixels, self.brush.color, self.brush.opacity, self.blend);
            self.dirty = true;
        }
    }

    pub fn add_layer(&mut self) {
        self.document.add_layer();
        self.dirty = true;
    }

    pub fn remove_active_layer(&mut self) {
        if self.document.remove_active_layer() {
            self.dirty = true;
        }
    }

    /// Flattened RGBA bytes of the document (visible layers composited).
    pub fn composite_rgba(&self) -> Vec<u8> {
        self.document.composite().pixels
    }

    /// Composite the document and encode it as PNG.
    pub fn export_png(&self) -> Vec<u8> {
        self.document.composite().to_png()
    }

    /// Composite the document and write it to `path`.
    pub fn save_png(&self, path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        std::fs::write(path, self.export_png())
    }
}

/// Registers Paint-mode state. Systems/UI live in the host app.
pub struct PolyForgePaintPlugin;

impl Plugin for PolyForgePaintPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PaintState>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn red() -> Rgba {
        Rgba::opaque(255, 0, 0)
    }

    #[test]
    fn dab_paints_opaque_pixels() {
        let mut state = PaintState::default();
        state.brush.size = 16.0;
        state.brush.hardness = 1.0;
        state.brush.opacity = 1.0;
        state.brush.flow = 1.0;
        state.brush.color = red();
        state.document = PaintDocument::empty(32, 32);

        state.stamp(16.0, 16.0);

        let layer = state.document.active_layer().unwrap();
        let c = layer.pixels.get(16, 16);
        assert_eq!(c.r, 255);
        assert_eq!(c.a, 255);
        // A corner far from the brush must remain untouched.
        assert_eq!(layer.pixels.get(0, 0).a, 0);
    }

    #[test]
    fn eraser_reduces_alpha() {
        let mut state = PaintState::default();
        state.document = PaintDocument::empty(16, 16);
        state.brush.size = 16.0;
        state.brush.hardness = 1.0;
        state.brush.flow = 1.0;
        state.brush.opacity = 1.0;
        state.brush.color = red();
        state.stamp(8.0, 8.0);
        assert_eq!(state.document.active_layer().unwrap().pixels.get(8, 8).a, 255);

        state.tool = PaintTool::Eraser;
        state.stamp(8.0, 8.0);
        assert_eq!(state.document.active_layer().unwrap().pixels.get(8, 8).a, 0);
    }

    #[test]
    fn layer_stack_composites() {
        let mut doc = PaintDocument::empty(4, 4);
        {
            let base = doc.active_layer_mut().unwrap();
            base.pixels.fill(Rgba::opaque(0, 0, 255));
        }
        doc.add_layer();
        {
            let top = doc.active_layer_mut().unwrap();
            top.pixels.fill(Rgba::opaque(255, 0, 0));
            top.opacity = 0.0;
        }
        // Invisible/transparent top layer -> bottom shows through.
        let flat = doc.composite();
        assert_eq!(flat.get(1, 1).b, 255);

        doc.active_layer_mut().unwrap().opacity = 1.0;
        let flat = doc.composite();
        assert_eq!(flat.get(1, 1).r, 255);
        assert_eq!(flat.get(1, 1).b, 0);
    }

    #[test]
    fn uv_maps_to_flipped_pixel() {
        let (x, y) = projection::uv_to_pixel(0.25, 0.75, 100, 100);
        assert!((x - 25.0).abs() < 1e-4);
        assert!((y - 25.0).abs() < 1e-4);

        let (u, v) = projection::pixel_to_uv(25.0, 25.0, 100, 100);
        assert!((u - 0.25).abs() < 1e-4);
        assert!((v - 0.75).abs() < 1e-4);
    }

    #[test]
    fn png_export_has_signature() {
        let state = PaintState::default();
        let png = state.export_png();
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
    }
}
