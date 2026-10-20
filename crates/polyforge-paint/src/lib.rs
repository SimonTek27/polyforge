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
pub use types::{
    BlendMode, BrushSettings, PaintChannel, PaintDocument, PaintLayer, PaintTool, Rgba,
};

use bevy::prelude::*;

/// Global Paint-mode state: per-channel documents, brush and active tool.
#[derive(Resource)]
pub struct PaintState {
    /// Base colour (albedo) document.
    pub document: PaintDocument,
    pub roughness: PaintDocument,
    pub metalness: PaintDocument,
    pub normal: PaintDocument,
    pub emissive: PaintDocument,
    /// Channel currently being painted and previewed.
    pub channel: PaintChannel,
    pub brush: BrushSettings,
    pub tool: PaintTool,
    pub blend: BlendMode,
    /// Offset (dx, dy) from destination to clone-source pixel.
    pub clone_offset: (f32, f32),
    /// Set whenever pixels change so the GPU texture can be re-uploaded.
    pub dirty: bool,
    /// Monotonic counter bumped on every mutation (layers, brush stamps, UI edits).
    pub revision: u64,
}

/// Default document for a channel at the given size.
///
/// Roughness defaults to mid-grey, metalness/emissive to black and the normal
/// map to flat `(128, 128, 255)`.
pub fn channel_document(channel: PaintChannel, width: u32, height: u32) -> PaintDocument {
    match channel {
        PaintChannel::BaseColor => PaintDocument::new(width, height),
        PaintChannel::Roughness => PaintDocument::data(width, height, Rgba::grey(128)),
        PaintChannel::Metalness => PaintDocument::data(width, height, Rgba::BLACK),
        PaintChannel::Normal => PaintDocument::data(width, height, Rgba::opaque(128, 128, 255)),
        PaintChannel::Emissive => PaintDocument::data(width, height, Rgba::BLACK),
    }
}

impl Default for PaintState {
    fn default() -> Self {
        Self {
            document: channel_document(PaintChannel::BaseColor, 1024, 1024),
            roughness: channel_document(PaintChannel::Roughness, 1024, 1024),
            metalness: channel_document(PaintChannel::Metalness, 1024, 1024),
            normal: channel_document(PaintChannel::Normal, 1024, 1024),
            emissive: channel_document(PaintChannel::Emissive, 1024, 1024),
            channel: PaintChannel::BaseColor,
            brush: BrushSettings::default(),
            tool: PaintTool::default(),
            blend: BlendMode::default(),
            clone_offset: (0.0, 0.0),
            dirty: false,
            revision: 0,
        }
    }
}

impl PaintState {
    pub fn canvas_size(&self) -> (u32, u32) {
        (self.document.width, self.document.height)
    }

    pub fn document_for(&self, channel: PaintChannel) -> &PaintDocument {
        match channel {
            PaintChannel::BaseColor => &self.document,
            PaintChannel::Roughness => &self.roughness,
            PaintChannel::Metalness => &self.metalness,
            PaintChannel::Normal => &self.normal,
            PaintChannel::Emissive => &self.emissive,
        }
    }

    fn document_mut(&mut self, channel: PaintChannel) -> &mut PaintDocument {
        match channel {
            PaintChannel::BaseColor => &mut self.document,
            PaintChannel::Roughness => &mut self.roughness,
            PaintChannel::Metalness => &mut self.metalness,
            PaintChannel::Normal => &mut self.normal,
            PaintChannel::Emissive => &mut self.emissive,
        }
    }

    pub fn active_document(&self) -> &PaintDocument {
        self.document_for(self.channel)
    }

    pub fn active_document_mut(&mut self) -> &mut PaintDocument {
        let channel = self.channel;
        self.document_mut(channel)
    }

    /// Resize/replace every channel canvas, keeping brush settings.
    pub fn reset(&mut self, width: u32, height: u32) {
        for channel in PaintChannel::all() {
            let document = channel_document(channel, width, height);
            *self.document_mut(channel) = document;
        }
        self.dirty = true;
        self.revision += 1;
    }

    /// Paint a single brush dab at pixel `(x, y)` on the active channel.
    ///
    /// When the active tool is [`PaintTool::Clone`] the layer directly below
    /// the active one is sampled using `clone_offset`.
    pub fn stamp(&mut self, x: f32, y: f32) {
        let Self {
            document,
            roughness,
            metalness,
            normal,
            emissive,
            channel,
            brush,
            tool,
            blend,
            clone_offset,
            dirty,
            revision,
        } = self;

        let target = match *channel {
            PaintChannel::BaseColor => document,
            PaintChannel::Roughness => roughness,
            PaintChannel::Metalness => metalness,
            PaintChannel::Normal => normal,
            PaintChannel::Emissive => emissive,
        };

        if paint_dab(
            target,
            brush,
            *tool,
            *blend,
            *clone_offset,
            None,
            (x, y),
        ) {
            *dirty = true;
            *revision += 1;
        }
    }

    /// Stamp a straight stroke between two points on the active channel.
    pub fn stroke(&mut self, from: (f32, f32), to: (f32, f32)) {
        let Self {
            document,
            roughness,
            metalness,
            normal,
            emissive,
            channel,
            brush,
            tool,
            blend,
            clone_offset,
            dirty,
            revision,
        } = self;

        let target = match *channel {
            PaintChannel::BaseColor => document,
            PaintChannel::Roughness => roughness,
            PaintChannel::Metalness => metalness,
            PaintChannel::Normal => normal,
            PaintChannel::Emissive => emissive,
        };

        if paint_dab(
            target,
            brush,
            *tool,
            *blend,
            *clone_offset,
            Some(from),
            to,
        ) {
            *dirty = true;
            *revision += 1;
        }
    }

    /// Fill the whole active layer of the active channel with the brush colour.
    pub fn fill_active(&mut self) {
        let brush = self.brush.clone();
        let blend = self.blend;
        let channel = self.channel;
        {
            let document = self.document_mut(channel);
            let Some(layer) = document.active_layer_mut() else {
                return;
            };
            fill_all(
                &mut layer.pixels,
                brush.color,
                brush.opacity,
                blend,
            );
        }
        self.dirty = true;
        self.revision += 1;
    }

    pub fn add_layer(&mut self) {
        self.active_document_mut().add_layer();
        self.dirty = true;
        self.revision += 1;
    }

    pub fn remove_active_layer(&mut self) {
        if self.active_document_mut().remove_active_layer() {
            self.dirty = true;
            self.revision += 1;
        }
    }

    /// Composite one channel into an RGBA image.
    pub fn composite_channel(&self, channel: PaintChannel) -> RasterImage {
        self.document_for(channel).composite()
    }

    /// Flattened RGBA bytes of the base-colour document.
    pub fn composite_rgba(&self) -> Vec<u8> {
        self.document.composite().pixels
    }

    /// Build the glTF-style ORM map: `R` = AO (white), `G` = roughness,
    /// `B` = metalness.
    pub fn export_orm(&self) -> RasterImage {
        let roughness = self.roughness.composite();
        let metalness = self.metalness.composite();
        let mut out = RasterImage::new(roughness.width, roughness.height);
        let count = (roughness.width * roughness.height) as usize;
        for i in 0..count {
            let rough = roughness.pixel_at(i).r;
            let metal = metalness.pixel_at(i).r;
            out.set_index(i, Rgba::opaque(255, rough, metal));
        }
        out
    }

    /// Composite the base-colour document and encode it as PNG.
    pub fn export_png(&self) -> Vec<u8> {
        self.document.composite().to_png()
    }

    /// Composite the base-colour document and write it to `path`.
    pub fn save_png(&self, path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        std::fs::write(path, self.export_png())
    }

    /// Write the full PBR texture set (base colour, ORM, normal, emissive).
    pub fn export_pbr(&self, dir: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        let dir = dir.as_ref();
        std::fs::create_dir_all(dir)?;
        std::fs::write(dir.join("basecolor.png"), self.document.composite().to_png())?;
        std::fs::write(dir.join("orm.png"), self.export_orm().to_png())?;
        std::fs::write(dir.join("normal.png"), self.normal.composite().to_png())?;
        std::fs::write(dir.join("emissive.png"), self.emissive.composite().to_png())?;
        Ok(())
    }
}

/// Paint into `document`, optionally as a segment from `from` to `to`.
fn paint_dab(
    document: &mut PaintDocument,
    brush: &BrushSettings,
    tool: PaintTool,
    blend: BlendMode,
    clone_offset: (f32, f32),
    from: Option<(f32, f32)>,
    to: (f32, f32),
) -> bool {
    if document.layers.is_empty() {
        return false;
    }
    let active = document.active.min(document.layers.len() - 1);

    // Split the stack so the clone source (layer below) and the target
    // layer can be borrowed at the same time.
    let (below, above) = document.layers.split_at_mut(active);
    let clone = if tool == PaintTool::Clone {
        below.last().map(|l| (&l.pixels, clone_offset))
    } else {
        None
    };
    let layer = &mut above[0];

    let mut session = StrokeSession::new(&layer.pixels, brush, tool, blend, clone);
    match from {
        Some(from) => {
            session.stroke_to(&mut layer.pixels, from, to);
        }
        None => session.dab(&mut layer.pixels, to.0, to.1),
    }
    true
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

    #[test]
    fn channel_defaults_are_neutral() {
        let state = PaintState::default();
        assert_eq!(state.roughness.composite().get(0, 0).r, 128);
        assert_eq!(state.metalness.composite().get(0, 0).r, 0);
        let n = state.normal.composite().get(0, 0);
        assert_eq!((n.r, n.g, n.b), (128, 128, 255));
        assert_eq!(state.emissive.composite().get(0, 0).r, 0);
    }

    #[test]
    fn painting_a_channel_leaves_base_color_untouched() {
        let mut state = PaintState::default();
        state.document = PaintDocument::empty(16, 16);
        state.roughness = PaintDocument::data(16, 16, Rgba::grey(128));
        state.channel = PaintChannel::Roughness;
        state.brush.size = 16.0;
        state.brush.hardness = 1.0;
        state.brush.opacity = 1.0;
        state.brush.flow = 1.0;
        state.brush.color = Rgba::grey(255);

        state.stamp(8.0, 8.0);

        let rough = state.roughness.active_layer().unwrap().pixels.get(8, 8);
        assert_eq!(rough.r, 255);
        let base = state.document.active_layer().unwrap().pixels.get(8, 8);
        assert_eq!(base.a, 0);
    }

    #[test]
    fn orm_packs_ao_roughness_metalness() {
        let mut state = PaintState::default();
        state.roughness = PaintDocument::data(2, 2, Rgba::grey(64));
        state.metalness = PaintDocument::data(2, 2, Rgba::grey(200));

        let orm = state.export_orm();
        let px = orm.get(0, 0);
        assert_eq!(px.r, 255);
        assert_eq!(px.g, 64);
        assert_eq!(px.b, 200);
        assert_eq!(px.a, 255);
    }

    #[test]
    fn export_pbr_writes_four_maps() {
        let mut state = PaintState::default();
        state.reset(4, 4);
        let dir = std::env::temp_dir().join("polyforge_paint_pbr_test");
        let _ = std::fs::remove_dir_all(&dir);

        state.export_pbr(&dir).expect("export");

        for name in ["basecolor.png", "orm.png", "normal.png", "emissive.png"] {
            let path = dir.join(name);
            assert!(path.is_file(), "{name} missing");
            let bytes = std::fs::read(&path).unwrap();
            assert_eq!(&bytes[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
        }
    }
}
