//! Core paint data types: colours, brush settings, tools and layers.

/// Straight (non-premultiplied) 8-bit RGBA colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Default for Rgba {
    fn default() -> Self {
        Self::TRANSPARENT
    }
}

impl Rgba {
    pub const TRANSPARENT: Rgba = Rgba { r: 0, g: 0, b: 0, a: 0 };
    pub const BLACK: Rgba = Rgba { r: 0, g: 0, b: 0, a: 255 };
    pub const WHITE: Rgba = Rgba { r: 255, g: 255, b: 255, a: 255 };

    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn opaque(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub fn to_array(self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }

    pub fn from_array([r, g, b, a]: [u8; 4]) -> Self {
        Self { r, g, b, a }
    }

    /// Linear-friendly helper: `0.0..=1.0` floats to 8-bit.
    pub fn from_rgb_f32(r: f32, g: f32, b: f32) -> Self {
        let c = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        Self { r: c(r), g: c(g), b: c(b), a: 255 }
    }

    pub fn to_rgb_f32(self) -> [f32; 3] {
        [
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
        ]
    }

    /// Neutral grey (used for layer thumbnails / fallbacks).
    pub fn grey(v: u8) -> Self {
        Self { r: v, g: v, b: v, a: 255 }
    }
}

/// Which painting operation the brush performs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaintTool {
    Brush,
    Eraser,
    Clone,
    Fill,
}

impl Default for PaintTool {
    fn default() -> Self {
        PaintTool::Brush
    }
}

impl PaintTool {
    pub fn label(self) -> &'static str {
        match self {
            PaintTool::Brush => "Brush",
            PaintTool::Eraser => "Erase",
            PaintTool::Clone => "Clone",
            PaintTool::Fill => "Fill",
        }
    }

    pub fn all() -> [PaintTool; 4] {
        [
            PaintTool::Brush,
            PaintTool::Eraser,
            PaintTool::Clone,
            PaintTool::Fill,
        ]
    }
}

/// Shape of an individual brush dab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PaintBrushPattern {
    /// Round dab (default).
    #[default]
    Circle,
    /// Axis-aligned square dab.
    Square,
    /// Rotated square (diamond) dab.
    Diamond,
}

impl PaintBrushPattern {
    pub fn label(self) -> &'static str {
        match self {
            PaintBrushPattern::Circle => "Circle",
            PaintBrushPattern::Square => "Square",
            PaintBrushPattern::Diamond => "Diamond",
        }
    }

    pub fn all() -> [PaintBrushPattern; 3] {
        [
            PaintBrushPattern::Circle,
            PaintBrushPattern::Square,
            PaintBrushPattern::Diamond,
        ]
    }
}

/// PBR texture channel being painted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PaintChannel {
    /// Albedo / base colour (sRGB).
    #[default]
    BaseColor,
    /// Roughness (linear, painted as grey).
    Roughness,
    /// Metalness (linear, painted as grey).
    Metalness,
    /// Tangent-space normal map (linear RGB, 128,128,255 = flat).
    Normal,
    /// Emissive colour (sRGB).
    Emissive,
}

impl PaintChannel {
    pub fn label(self) -> &'static str {
        match self {
            PaintChannel::BaseColor => "Base",
            PaintChannel::Roughness => "Rough",
            PaintChannel::Metalness => "Metal",
            PaintChannel::Normal => "Normal",
            PaintChannel::Emissive => "Emit",
        }
    }

    pub fn all() -> [PaintChannel; 5] {
        [
            PaintChannel::BaseColor,
            PaintChannel::Roughness,
            PaintChannel::Metalness,
            PaintChannel::Normal,
            PaintChannel::Emissive,
        ]
    }

    /// Scalar channels are painted with a greyscale value.
    pub fn is_scalar(self) -> bool {
        matches!(self, PaintChannel::Roughness | PaintChannel::Metalness)
    }

    /// Data channels must not be sRGB-decoded.
    pub fn is_data(self) -> bool {
        matches!(
            self,
            PaintChannel::Roughness | PaintChannel::Metalness | PaintChannel::Normal
        )
    }

    /// Sensible brush colour when switching to this channel.
    pub fn default_color(self) -> Option<Rgba> {
        match self {
            PaintChannel::Roughness | PaintChannel::Metalness => Some(Rgba::grey(128)),
            PaintChannel::Normal => Some(Rgba::opaque(128, 128, 255)),
            PaintChannel::BaseColor | PaintChannel::Emissive => None,
        }
    }
}

/// How a layer (or dab) combines with what is underneath.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BlendMode {
    #[default]
    Normal,
    Multiply,
    Add,
    Screen,
}

impl BlendMode {
    pub fn label(self) -> &'static str {
        match self {
            BlendMode::Normal => "Normal",
            BlendMode::Multiply => "Multiply",
            BlendMode::Add => "Add",
            BlendMode::Screen => "Screen",
        }
    }
}

/// Brush shape / dynamics.
#[derive(Debug, Clone)]
pub struct BrushSettings {
    /// Brush diameter in pixels.
    pub size: f32,
    /// Maximum opacity a single stroke can reach (0..=1).
    pub opacity: f32,
    /// Per-dab flow (0..=1) — paints how fast opacity builds up.
    pub flow: f32,
    /// Edge falloff: 0 = soft, 1 = hard.
    pub hardness: f32,
    /// Primary paint colour.
    pub color: Rgba,
    /// Dab spacing as a fraction of `size` (0.05..=1.0).
    pub spacing: f32,
    /// Shape of each dab.
    pub pattern: PaintBrushPattern,
}

impl Default for BrushSettings {
    fn default() -> Self {
        Self {
            size: 48.0,
            opacity: 1.0,
            flow: 0.85,
            hardness: 0.6,
            color: Rgba::opaque(220, 90, 60),
            spacing: 0.2,
            pattern: PaintBrushPattern::Circle,
        }
    }
}

impl BrushSettings {
    /// Hardness/falloff applied to a normalised distance `d` (0 = centre).
    ///
    /// Returns coverage in `0..=1`.
    pub fn falloff(&self, d: f32) -> f32 {
        let d = d.clamp(0.0, 1.0);
        let hardness = self.hardness.clamp(0.0, 1.0);
        if d >= 1.0 {
            return 0.0;
        }
        // Soft inner radius; outside it fade linearly to the rim.
        let inner = (hardness * 0.92).min(0.98);
        if d <= inner {
            1.0
        } else {
            ((1.0 - d) / (1.0 - inner)).clamp(0.0, 1.0)
        }
    }

    pub fn radius(&self) -> f32 {
        (self.size * 0.5).max(0.5)
    }
}

/// A single raster layer in the document.
#[derive(Debug, Clone, PartialEq)]
pub struct PaintLayer {
    pub name: String,
    pub visible: bool,
    pub opacity: f32,
    pub blend: BlendMode,
    pub pixels: RasterImage,
    /// Optional greyscale mask (white = visible, black = hidden).
    pub mask: Option<RasterImage>,
    /// Whether `mask` is applied when compositing.
    pub mask_enabled: bool,
}

impl PaintLayer {
    pub fn new(name: impl Into<String>, width: u32, height: u32) -> Self {
        Self {
            name: name.into(),
            visible: true,
            opacity: 1.0,
            blend: BlendMode::Normal,
            pixels: RasterImage::new(width, height),
            mask: None,
            mask_enabled: true,
        }
    }

    pub fn has_mask(&self) -> bool {
        self.mask.is_some()
    }

    /// Create the mask on first use; existing masks are kept.
    pub fn ensure_mask(&mut self, width: u32, height: u32) -> &mut RasterImage {
        self.mask
            .get_or_insert_with(|| RasterImage::filled(width, height, Rgba::WHITE))
    }

    /// Drop the mask entirely (layer returns to unmasked).
    pub fn clear_mask(&mut self) {
        self.mask = None;
    }

    /// Fill an existing mask with one value (255 = keep everything).
    pub fn fill_mask(&mut self, value: u8) -> bool {
        match self.mask.as_mut() {
            Some(mask) => {
                mask.fill(Rgba::grey(value));
                true
            }
            None => false,
        }
    }

    /// Mask coverage at pixel index (`1.0` when no mask is applied).
    pub fn mask_coverage(&self, index: usize) -> f32 {
        match (self.mask_enabled, self.mask.as_ref()) {
            (true, Some(mask)) => mask.pixel_at(index).r as f32 / 255.0,
            _ => 1.0,
        }
    }
}

/// A stack of layers plus the canvas size.
#[derive(Debug, Clone, PartialEq)]
pub struct PaintDocument {
    pub width: u32,
    pub height: u32,
    pub layers: Vec<PaintLayer>,
    pub active: usize,
    /// Colour shown where every layer is transparent.
    pub background: Rgba,
}

impl Default for PaintDocument {
    fn default() -> Self {
        Self::new(1024, 1024)
    }
}

impl PaintDocument {
    pub fn new(width: u32, height: u32) -> Self {
        let width = width.max(1);
        let height = height.max(1);
        let mut base = PaintLayer::new("Background", width, height);
        base.pixels.fill(Rgba::WHITE);
        Self {
            width,
            height,
            layers: vec![base],
            active: 0,
            background: Rgba { r: 24, g: 24, b: 28, a: 255 },
        }
    }

    pub fn empty(width: u32, height: u32) -> Self {
        let width = width.max(1);
        let height = height.max(1);
        Self {
            width,
            height,
            layers: vec![PaintLayer::new("Layer 1", width, height)],
            active: 0,
            background: Rgba { r: 24, g: 24, b: 28, a: 255 },
        }
    }

    /// A data channel document: one empty layer over a constant background.
    pub fn data(width: u32, height: u32, background: Rgba) -> Self {
        let width = width.max(1);
        let height = height.max(1);
        Self {
            width,
            height,
            layers: vec![PaintLayer::new("Layer 1", width, height)],
            active: 0,
            background,
        }
    }

    pub fn active_layer(&self) -> Option<&PaintLayer> {
        self.layers.get(self.active)
    }

    pub fn active_layer_mut(&mut self) -> Option<&mut PaintLayer> {
        self.layers.get_mut(self.active)
    }

    pub fn add_layer(&mut self) -> usize {
        let name = format!("Layer {}", self.layers.len() + 1);
        self.layers
            .push(PaintLayer::new(name, self.width, self.height));
        self.active = self.layers.len() - 1;
        self.active
    }

    /// Copy the active layer (pixels, mask, blend and opacity) directly above
    /// it and make the copy active.
    pub fn duplicate_active_layer(&mut self) -> bool {
        let Some(source) = self.layers.get(self.active) else {
            return false;
        };
        let mut copy = source.clone();
        copy.name = format!("{} copy", source.name);
        let at = self.active + 1;
        self.layers.insert(at, copy);
        self.active = at;
        true
    }

    /// Remove the active layer (never removes the last remaining layer).
    pub fn remove_active_layer(&mut self) -> bool {
        if self.layers.len() <= 1 {
            return false;
        }
        self.layers.remove(self.active);
        if self.active >= self.layers.len() {
            self.active = self.layers.len() - 1;
        }
        true
    }

    /// Move the active layer one step up (towards the top of the stack).
    pub fn move_active_up(&mut self) -> bool {
        if self.active + 1 >= self.layers.len() {
            return false;
        }
        self.layers.swap(self.active, self.active + 1);
        self.active += 1;
        true
    }

    /// Move the active layer one step down (towards the bottom).
    pub fn move_active_down(&mut self) -> bool {
        if self.active == 0 {
            return false;
        }
        self.layers.swap(self.active, self.active - 1);
        self.active -= 1;
        true
    }

    /// Flatten every visible layer, bottom to top, into a single image.
    pub fn composite(&self) -> RasterImage {
        let mut out = RasterImage::filled(self.width, self.height, self.background);
        for layer in &self.layers {
            if !layer.visible || layer.opacity <= 0.0 {
                continue;
            }
            composite_layer(&mut out, layer);
        }
        out
    }

    /// Merge the active layer into the one below it.
    ///
    /// The upper layer's opacity, blend mode and mask are baked into the lower
    /// layer's pixels, then the upper layer is removed. Returns `false` when
    /// the active layer is already the bottom layer.
    pub fn merge_active_down(&mut self) -> bool {
        if self.active == 0 || self.active >= self.layers.len() {
            return false;
        }
        let upper_index = self.active;
        let lower_index = upper_index - 1;
        {
            let (below, above) = self.layers.split_at_mut(upper_index);
            let lower = &mut below[lower_index];
            let upper = &above[0];
            if upper.visible && upper.opacity > 0.0 {
                let count = (lower.pixels.width * lower.pixels.height) as usize;
                let opacity = upper.opacity.clamp(0.0, 1.0);
                for i in 0..count {
                    let src = upper.pixels.pixel_at(i);
                    if src.a == 0 {
                        continue;
                    }
                    let coverage = opacity * upper.mask_coverage(i);
                    if coverage <= 0.0 {
                        continue;
                    }
                    let dst = lower.pixels.pixel_at(i);
                    let blended = RasterImage::blend(dst, src, coverage, upper.blend);
                    lower.pixels.set_index(i, blended);
                }
            }
        }
        self.layers.remove(upper_index);
        self.active = lower_index;
        true
    }

    /// Flatten all visible layers into a single layer (transparent background).
    ///
    /// Returns `false` when there is nothing to flatten.
    pub fn flatten(&mut self) -> bool {
        if self.layers.len() <= 1 {
            return false;
        }
        let mut out = RasterImage::new(self.width, self.height);
        for layer in &self.layers {
            if !layer.visible || layer.opacity <= 0.0 {
                continue;
            }
            composite_layer(&mut out, layer);
        }
        let mut merged = PaintLayer::new("Flattened", self.width, self.height);
        merged.pixels = out;
        self.layers = vec![merged];
        self.active = 0;
        true
    }
}

/// Blend one layer over `dst` honouring layer opacity and blend mode.
pub fn composite_layer(dst: &mut RasterImage, layer: &PaintLayer) {
    let op = layer.opacity.clamp(0.0, 1.0);
    if op <= 0.0 {
        return;
    }
    let n = (dst.width * dst.height) as usize;
    for i in 0..n {
        let src = layer.pixels.pixel_at(i);
        if src.a == 0 {
            continue;
        }
        let coverage = op * layer.mask_coverage(i);
        if coverage <= 0.0 {
            continue;
        }
        let dst_c = dst.pixel_at(i);
        let blended = RasterImage::blend(dst_c, src, coverage, layer.blend);
        dst.set_index(i, blended);
    }
}

use crate::raster::RasterImage;
