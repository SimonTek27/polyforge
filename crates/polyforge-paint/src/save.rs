//! Paint project persistence: a small, dependency-free binary container.
//!
//! The file stores the full paint session — all five channel documents (layer
//! pixels, masks, blend/opacity/visibility), the active channel and the brush
//! settings — using raw RGBA bytes, so no image decoder is required.
//!
//! Layout (all integers little-endian):
//!
//! ```text
//! magic    "PFPNT"     5 bytes
//! version  u8          = 2
//! channel  u8          active PaintChannel
//! document x5          BaseColor, Roughness, Metalness, Normal, Emissive
//! brush                size/opacity/flow/hardness/spacing/colour/pattern,
//!                      tool, blend, clone offset, mask/tuning flags
//! ```
//!
//! document: `width u32, height u32, active u32, background RGBA4, layers u32`
//! layer:    `name_len u32 + UTF-8, flags u8, blend u8, opacity f32,
//!            pixels w*h*4, mask w*h*4 (only when the mask flag is set)`

use std::fmt;
use std::path::Path;

use crate::raster::RasterImage;
use crate::types::{
    BlendMode, BrushSettings, PaintBrushPattern, PaintChannel, PaintDocument, PaintLayer, PaintTool,
    Rgba,
};
use crate::PaintState;

const MAGIC: &[u8; 5] = b"PFPNT";
const VERSION: u8 = 2;

const FLAG_VISIBLE: u8 = 1 << 0;
const FLAG_HAS_MASK: u8 = 1 << 1;
const FLAG_MASK_ENABLED: u8 = 1 << 2;

/// Errors produced while decoding a paint project.
#[derive(Debug)]
pub enum PaintSaveError {
    /// The file does not start with the paint-project magic.
    BadMagic,
    /// The version byte is newer than this build understands.
    UnsupportedVersion(u8),
    /// The file ended before the expected data.
    Truncated,
    /// A layer name was not valid UTF-8.
    InvalidName,
    /// A layer stack was empty or the active index was out of range.
    InvalidLayers,
    /// A blend mode byte was unknown.
    UnknownBlend(u8),
    /// A tool byte was unknown.
    UnknownTool(u8),
    /// A brush-pattern byte was unknown.
    UnknownPattern(u8),
    /// A channel byte was unknown.
    UnknownChannel(u8),
    /// Underlying I/O failure.
    Io(std::io::Error),
}

impl fmt::Display for PaintSaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PaintSaveError::BadMagic => write!(f, "not a PolyForge paint project"),
            PaintSaveError::UnsupportedVersion(v) => write!(f, "unsupported version {v}"),
            PaintSaveError::Truncated => write!(f, "paint project is truncated"),
            PaintSaveError::InvalidName => write!(f, "layer name is not valid UTF-8"),
            PaintSaveError::InvalidLayers => write!(f, "invalid layer stack"),
            PaintSaveError::UnknownBlend(b) => write!(f, "unknown blend mode {b}"),
            PaintSaveError::UnknownTool(t) => write!(f, "unknown tool {t}"),
            PaintSaveError::UnknownPattern(p) => write!(f, "unknown brush pattern {p}"),
            PaintSaveError::UnknownChannel(c) => write!(f, "unknown channel {c}"),
            PaintSaveError::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for PaintSaveError {}

impl From<std::io::Error> for PaintSaveError {
    fn from(e: std::io::Error) -> Self {
        PaintSaveError::Io(e)
    }
}

fn blend_code(blend: BlendMode) -> u8 {
    match blend {
        BlendMode::Normal => 0,
        BlendMode::Multiply => 1,
        BlendMode::Add => 2,
        BlendMode::Screen => 3,
    }
}

fn blend_from(code: u8) -> Result<BlendMode, PaintSaveError> {
    Ok(match code {
        0 => BlendMode::Normal,
        1 => BlendMode::Multiply,
        2 => BlendMode::Add,
        3 => BlendMode::Screen,
        other => return Err(PaintSaveError::UnknownBlend(other)),
    })
}

fn tool_code(tool: PaintTool) -> u8 {
    match tool {
        PaintTool::Brush => 0,
        PaintTool::Eraser => 1,
        PaintTool::Clone => 2,
        PaintTool::Fill => 3,
    }
}

fn tool_from(code: u8) -> Result<PaintTool, PaintSaveError> {
    Ok(match code {
        0 => PaintTool::Brush,
        1 => PaintTool::Eraser,
        2 => PaintTool::Clone,
        3 => PaintTool::Fill,
        other => return Err(PaintSaveError::UnknownTool(other)),
    })
}

fn pattern_code(pattern: PaintBrushPattern) -> u8 {
    match pattern {
        PaintBrushPattern::Circle => 0,
        PaintBrushPattern::Square => 1,
        PaintBrushPattern::Diamond => 2,
    }
}

fn pattern_from(code: u8) -> Result<PaintBrushPattern, PaintSaveError> {
    Ok(match code {
        0 => PaintBrushPattern::Circle,
        1 => PaintBrushPattern::Square,
        2 => PaintBrushPattern::Diamond,
        other => return Err(PaintSaveError::UnknownPattern(other)),
    })
}

fn channel_code(channel: PaintChannel) -> u8 {
    match channel {
        PaintChannel::BaseColor => 0,
        PaintChannel::Roughness => 1,
        PaintChannel::Metalness => 2,
        PaintChannel::Normal => 3,
        PaintChannel::Emissive => 4,
    }
}

fn channel_from(code: u8) -> Result<PaintChannel, PaintSaveError> {
    Ok(match code {
        0 => PaintChannel::BaseColor,
        1 => PaintChannel::Roughness,
        2 => PaintChannel::Metalness,
        3 => PaintChannel::Normal,
        4 => PaintChannel::Emissive,
        other => return Err(PaintSaveError::UnknownChannel(other)),
    })
}

/// Encode a full paint session into the `.pfp` container format.
pub fn encode(state: &PaintState) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.push(VERSION);
    out.push(channel_code(state.channel));

    for channel in PaintChannel::all() {
        write_document(&mut out, state.document_for(channel));
    }

    let brush = &state.brush;
    for value in [
        brush.size,
        brush.opacity,
        brush.flow,
        brush.hardness,
        brush.spacing,
    ] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out.extend_from_slice(&brush.color.to_array());
    out.push(pattern_code(brush.pattern));
    out.push(tool_code(state.tool));
    out.push(blend_code(state.blend));
    out.extend_from_slice(&state.clone_offset.0.to_le_bytes());
    out.extend_from_slice(&state.clone_offset.1.to_le_bytes());
    out.push(state.mask_paint as u8);
    out.push(state.symmetry_x as u8);
    out.push(state.symmetry_y as u8);
    out.extend_from_slice(&state.fill_tolerance.to_le_bytes());
    out
}

/// Decode a paint session produced by [`encode`].
pub fn decode(bytes: &[u8]) -> Result<PaintState, PaintSaveError> {
    let mut reader = Reader::new(bytes);
    if reader.take(5)? != MAGIC {
        return Err(PaintSaveError::BadMagic);
    }
    let version = reader.u8()?;
    if version != VERSION {
        return Err(PaintSaveError::UnsupportedVersion(version));
    }
    let channel = channel_from(reader.u8()?)?;

    let mut documents = Vec::with_capacity(5);
    for _ in 0..5 {
        documents.push(read_document(&mut reader)?);
    }

    let mut state = PaintState::default();
    state.document = documents.remove(0);
    state.roughness = documents.remove(0);
    state.metalness = documents.remove(0);
    state.normal = documents.remove(0);
    state.emissive = documents.remove(0);
    state.channel = channel;

    state.brush = BrushSettings {
        size: reader.f32()?,
        opacity: reader.f32()?,
        flow: reader.f32()?,
        hardness: reader.f32()?,
        spacing: reader.f32()?,
        color: Rgba::from_array(reader.array4()?),
        pattern: pattern_from(reader.u8()?)?,
    };
    state.tool = tool_from(reader.u8()?)?;
    state.blend = blend_from(reader.u8()?)?;
    state.clone_offset = (reader.f32()?, reader.f32()?);
    state.mask_paint = reader.u8()? != 0;
    state.symmetry_x = reader.u8()? != 0;
    state.symmetry_y = reader.u8()? != 0;
    state.fill_tolerance = reader.f32()?;

    state.dirty = true;
    state.revision = 1;
    Ok(state)
}

/// Write `state` to `path` as a paint project.
pub fn save(state: &PaintState, path: impl AsRef<Path>) -> std::io::Result<()> {
    std::fs::write(path, encode(state))
}

/// Read a paint project from `path`.
pub fn load(path: impl AsRef<Path>) -> Result<PaintState, PaintSaveError> {
    let bytes = std::fs::read(path)?;
    decode(&bytes)
}

fn write_document(out: &mut Vec<u8>, document: &PaintDocument) {
    out.extend_from_slice(&document.width.to_le_bytes());
    out.extend_from_slice(&document.height.to_le_bytes());
    out.extend_from_slice(&(document.active as u32).to_le_bytes());
    out.extend_from_slice(&document.background.to_array());
    out.extend_from_slice(&(document.layers.len() as u32).to_le_bytes());

    for layer in &document.layers {
        out.extend_from_slice(&(layer.name.len() as u32).to_le_bytes());
        out.extend_from_slice(layer.name.as_bytes());
        let mut flags = 0u8;
        if layer.visible {
            flags |= FLAG_VISIBLE;
        }
        if layer.has_mask() {
            flags |= FLAG_HAS_MASK;
        }
        if layer.mask_enabled {
            flags |= FLAG_MASK_ENABLED;
        }
        out.push(flags);
        out.push(blend_code(layer.blend));
        out.extend_from_slice(&layer.opacity.to_le_bytes());
        out.extend_from_slice(&layer.pixels.pixels);
        if let Some(mask) = layer.mask.as_ref() {
            out.extend_from_slice(&mask.pixels);
        }
    }
}

fn read_document(reader: &mut Reader<'_>) -> Result<PaintDocument, PaintSaveError> {
    let width = reader.u32()?;
    let height = reader.u32()?;
    if width == 0 || height == 0 {
        return Err(PaintSaveError::InvalidLayers);
    }
    let active = reader.u32()? as usize;
    let background = Rgba::from_array(reader.array4()?);
    let layer_count = reader.u32()? as usize;
    if layer_count == 0 || active >= layer_count {
        return Err(PaintSaveError::InvalidLayers);
    }

    let mut layers = Vec::with_capacity(layer_count);
    for _ in 0..layer_count {
        let name_len = reader.u32()? as usize;
        let name = std::str::from_utf8(reader.take(name_len)?)
            .map_err(|_| PaintSaveError::InvalidName)?
            .to_string();
        let flags = reader.u8()?;
        let blend = blend_from(reader.u8()?)?;
        let opacity = reader.f32()?;
        let pixels = RasterImage::from_rgba(width, height, reader.take(reader.pixel_bytes(width, height))?.to_vec());
        let mask = if flags & FLAG_HAS_MASK != 0 {
            Some(RasterImage::from_rgba(
                width,
                height,
                reader.take(reader.pixel_bytes(width, height))?.to_vec(),
            ))
        } else {
            None
        };
        layers.push(PaintLayer {
            name,
            visible: flags & FLAG_VISIBLE != 0,
            opacity,
            blend,
            pixels,
            mask,
            mask_enabled: flags & FLAG_MASK_ENABLED != 0,
        });
    }

    Ok(PaintDocument {
        width,
        height,
        layers,
        active,
        background,
    })
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn pixel_bytes(&self, width: u32, height: u32) -> usize {
        (width as usize) * (height as usize) * 4
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], PaintSaveError> {
        let end = self.pos.checked_add(len).ok_or(PaintSaveError::Truncated)?;
        if end > self.bytes.len() {
            return Err(PaintSaveError::Truncated);
        }
        let slice = &self.bytes[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    fn u8(&mut self) -> Result<u8, PaintSaveError> {
        Ok(self.take(1)?[0])
    }

    fn array4(&mut self) -> Result<[u8; 4], PaintSaveError> {
        let slice = self.take(4)?;
        Ok([slice[0], slice[1], slice[2], slice[3]])
    }

    fn u32(&mut self) -> Result<u32, PaintSaveError> {
        let slice = self.take(4)?;
        Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
    }

    fn f32(&mut self) -> Result<f32, PaintSaveError> {
        let slice = self.take(4)?;
        Ok(f32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_state() -> PaintState {
        let mut state = PaintState::default();
        state.document = PaintDocument::empty(4, 3);
        state.roughness = PaintDocument::data(4, 3, Rgba::grey(200));

        let layer = state.document.active_layer_mut().unwrap();
        layer.name = "Base".into();
        layer.pixels.set(1, 1, Rgba::opaque(10, 20, 30));
        layer.opacity = 0.5;
        layer.blend = BlendMode::Screen;
        layer.visible = false;

        state.document.add_layer();
        state.ensure_active_mask();
        state.fill_active_mask(64);
        state.mask_paint = true;
        state.symmetry_x = true;
        state.channel = PaintChannel::Roughness;
        state.tool = PaintTool::Fill;
        state.blend = BlendMode::Multiply;
        state.clone_offset = (3.0, -2.0);
        state.fill_tolerance = 0.33;
        state.brush.size = 17.0;
        state.brush.color = Rgba::opaque(1, 2, 3);
        state.brush.pattern = PaintBrushPattern::Diamond;
        state
    }

    #[test]
    fn round_trip_preserves_documents_and_brush() {
        let state = sample_state();
        let decoded = decode(&encode(&state)).expect("decode");

        assert_eq!(decoded.document, state.document);
        assert_eq!(decoded.roughness, state.roughness);
        assert_eq!(decoded.metalness, state.metalness);
        assert_eq!(decoded.normal, state.normal);
        assert_eq!(decoded.emissive, state.emissive);
        assert_eq!(decoded.channel, state.channel);
        assert_eq!(decoded.tool, state.tool);
        assert_eq!(decoded.blend, state.blend);
        assert_eq!(decoded.clone_offset, state.clone_offset);
        assert!(decoded.mask_paint);
        assert!(decoded.symmetry_x);
        assert!(!decoded.symmetry_y);
        assert!((decoded.fill_tolerance - 0.33).abs() < 1e-6);
        assert_eq!(decoded.brush.size, 17.0);
        assert_eq!(decoded.brush.color, Rgba::opaque(1, 2, 3));
        assert_eq!(decoded.brush.pattern, PaintBrushPattern::Diamond);
        assert!(decoded.dirty);
    }

    #[test]
    fn round_trip_preserves_layers_masks_and_flags() {
        let state = sample_state();
        let decoded = decode(&encode(&state)).expect("decode");

        let layers = &decoded.document.layers;
        assert_eq!(layers.len(), 2);
        assert_eq!(layers[0].name, "Base");
        assert!(!layers[0].visible);
        assert_eq!(layers[0].blend, BlendMode::Screen);
        assert!((layers[0].opacity - 0.5).abs() < 1e-6);
        assert_eq!(layers[0].pixels.get(1, 1), Rgba::opaque(10, 20, 30));
        assert!(layers[0].mask.is_none());

        assert!(layers[1].has_mask());
        assert_eq!(layers[1].mask.as_ref().unwrap().get(0, 0).r, 64);
        assert!(layers[1].mask_enabled);
    }

    #[test]
    fn file_round_trip_through_disk() {
        let state = sample_state();
        let path = std::env::temp_dir().join("polyforge_paint_test.pfp");
        save(&state, &path).expect("save");
        let loaded = load(&path).expect("load");
        assert_eq!(loaded.document, state.document);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn bad_magic_is_rejected() {
        let mut bytes = encode(&sample_state());
        bytes[0] = b'X';
        assert!(matches!(decode(&bytes), Err(PaintSaveError::BadMagic)));
    }

    #[test]
    fn future_version_is_rejected() {
        let mut bytes = encode(&sample_state());
        bytes[5] = VERSION + 1;
        assert!(matches!(
            decode(&bytes),
            Err(PaintSaveError::UnsupportedVersion(_))
        ));
    }

    #[test]
    fn truncated_files_are_rejected() {
        let bytes = encode(&sample_state());
        for cut in [6, 20, bytes.len() - 1] {
            assert!(matches!(
                decode(&bytes[..cut]),
                Err(PaintSaveError::Truncated)
            ));
        }
    }

    #[test]
    fn empty_layer_stack_is_rejected() {
        let mut bytes = encode(&sample_state());
        let layer_count_at = 5 + 1 + 1 + 4 + 4 + 4 + 4;
        bytes[layer_count_at..layer_count_at + 4].copy_from_slice(&0u32.to_le_bytes());
        assert!(matches!(
            decode(&bytes),
            Err(PaintSaveError::InvalidLayers)
        ));
    }
}
