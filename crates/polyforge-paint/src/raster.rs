//! A simple RGBA8 raster image with blending helpers.

use crate::types::{BlendMode, Rgba};

/// Tightly packed, non-premultiplied RGBA8 image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RasterImage {
    pub width: u32,
    pub height: u32,
    /// `width * height * 4` bytes.
    pub pixels: Vec<u8>,
}

impl RasterImage {
    /// Transparent image of the given size.
    pub fn new(width: u32, height: u32) -> Self {
        let width = width.max(1);
        let height = height.max(1);
        Self {
            width,
            height,
            pixels: vec![0; (width * height * 4) as usize],
        }
    }

    /// Image of the given size filled with a single colour.
    pub fn filled(width: u32, height: u32, color: Rgba) -> Self {
        let width = width.max(1);
        let height = height.max(1);
        let mut img = Self::new(width, height);
        img.fill(color);
        img
    }

    pub fn from_rgba(width: u32, height: u32, pixels: Vec<u8>) -> Self {
        let expected = (width * height * 4) as usize;
        assert_eq!(
            pixels.len(),
            expected,
            "pixel buffer must be width*height*4"
        );
        Self { width, height, pixels }
    }

    pub fn len(&self) -> usize {
        (self.width * self.height) as usize
    }

    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    pub fn contains(&self, x: i64, y: i64) -> bool {
        x >= 0 && y >= 0 && x < self.width as i64 && y < self.height as i64
    }

    pub fn index(&self, x: u32, y: u32) -> usize {
        (y * self.width + x) as usize
    }

    pub fn pixel_at(&self, index: usize) -> Rgba {
        let o = index * 4;
        Rgba {
            r: self.pixels[o],
            g: self.pixels[o + 1],
            b: self.pixels[o + 2],
            a: self.pixels[o + 3],
        }
    }

    pub fn set_index(&mut self, index: usize, color: Rgba) {
        let o = index * 4;
        self.pixels[o] = color.r;
        self.pixels[o + 1] = color.g;
        self.pixels[o + 2] = color.b;
        self.pixels[o + 3] = color.a;
    }

    pub fn get(&self, x: u32, y: u32) -> Rgba {
        if x >= self.width || y >= self.height {
            return Rgba::TRANSPARENT;
        }
        self.pixel_at(self.index(x, y))
    }

    pub fn set(&mut self, x: u32, y: u32, color: Rgba) {
        if x >= self.width || y >= self.height {
            return;
        }
        let i = self.index(x, y);
        self.set_index(i, color);
    }

    /// Nearest sampling with out-of-bounds returning transparent.
    pub fn sample_nearest(&self, x: i64, y: i64) -> Rgba {
        if !self.contains(x, y) {
            return Rgba::TRANSPARENT;
        }
        self.get(x as u32, y as u32)
    }

    pub fn fill(&mut self, color: Rgba) {
        for chunk in self.pixels.chunks_exact_mut(4) {
            chunk.copy_from_slice(&color.to_array());
        }
    }

    pub fn clear(&mut self) {
        self.pixels.iter_mut().for_each(|b| *b = 0);
    }

    /// Composite `src` (with stroke `alpha` and `mode`) over `dst`.
    ///
    /// Straight alpha, "source over destination". `alpha` scales the source
    /// alpha (brush opacity / flow).
    pub fn blend(dst: Rgba, src: Rgba, alpha: f32, mode: BlendMode) -> Rgba {
        let alpha = alpha.clamp(0.0, 1.0);
        let sa = (src.a as f32 / 255.0) * alpha;
        if sa <= 0.0 {
            return dst;
        }
        let da = dst.a as f32 / 255.0;
        let out_a = sa + da * (1.0 - sa);
        if out_a <= 0.0 {
            return Rgba::TRANSPARENT;
        }

        let dr = dst.r as f32 / 255.0;
        let dg = dst.g as f32 / 255.0;
        let db = dst.b as f32 / 255.0;
        let sr = src.r as f32 / 255.0;
        let sg = src.g as f32 / 255.0;
        let sb = src.b as f32 / 255.0;

        let (br, bg, bb) = match mode {
            BlendMode::Normal => (sr, sg, sb),
            BlendMode::Multiply => (dr * sr, dg * sg, db * sb),
            BlendMode::Add => ((dr + sr).min(1.0), (dg + sg).min(1.0), (db + sb).min(1.0)),
            BlendMode::Screen => (
                1.0 - (1.0 - dr) * (1.0 - sr),
                1.0 - (1.0 - dg) * (1.0 - sg),
                1.0 - (1.0 - db) * (1.0 - sb),
            ),
        };

        let out = |b: f32, d: f32| ((b * sa + d * da * (1.0 - sa)) / out_a).clamp(0.0, 1.0);

        Rgba {
            r: (out(br, dr) * 255.0).round() as u8,
            g: (out(bg, dg) * 255.0).round() as u8,
            b: (out(bb, db) * 255.0).round() as u8,
            a: (out_a * 255.0).round() as u8,
        }
    }

    /// Convert to a Bevy `Image` (sRGB, 2D).
    pub fn to_bevy_image(&self) -> bevy::image::Image {
        use bevy::asset::RenderAssetUsages;
        use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

        bevy::image::Image::new(
            Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            self.pixels.clone(),
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        )
    }

    /// Encode as PNG (8-bit RGBA, stored/uncompressed deflate).
    pub fn to_png(&self) -> Vec<u8> {
        crate::png::encode_png(self.width, self.height, &self.pixels)
    }
}
