//! UV <-> texel mapping helpers.
//!
//! PolyForge models are unit-scale with a Y-up scene; texture space is the
//! usual `u` to the right, `v` up convention, while images index `y` from the
//! top. These helpers keep that flip in one place so the paint systems can map
//! a ray hit's UV straight to a texel.

/// Map a normalised UV to pixel coordinates (fractional, top-left origin).
pub fn uv_to_pixel(u: f32, v: f32, width: u32, height: u32) -> (f32, f32) {
    (u * width as f32, (1.0 - v) * height as f32)
}

/// Map pixel coordinates back to normalised UV.
pub fn pixel_to_uv(x: f32, y: f32, width: u32, height: u32) -> (f32, f32) {
    (
        x / width.max(1) as f32,
        1.0 - y / height.max(1) as f32,
    )
}

/// Barycentric coordinates of `p` inside triangle `(a, b, c)`.
///
/// Returns `(wa, wb, wc)`; all three in `0..=1` means the point is inside.
pub fn barycentric(p: (f32, f32), a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> (f32, f32, f32) {
    let d = (b.1 - c.1) * (a.0 - c.0) + (c.0 - b.0) * (a.1 - c.1);
    if d.abs() < 1e-12 {
        return (-1.0, -1.0, -1.0);
    }
    let wa = ((b.1 - c.1) * (p.0 - c.0) + (c.0 - b.0) * (p.1 - c.1)) / d;
    let wb = ((c.1 - a.1) * (p.0 - c.0) + (a.0 - c.0) * (p.1 - c.1)) / d;
    let wc = 1.0 - wa - wb;
    (wa, wb, wc)
}

/// Basic UV bounding box for a set of triangle UVs, clamped to `0..=1`.
pub fn uv_bounds(uvs: &[(f32, f32)]) -> Option<(f32, f32, f32, f32)> {
    let first = uvs.first()?;
    let mut min = *first;
    let mut max = *first;
    for &(u, v) in uvs {
        min.0 = min.0.min(u);
        min.1 = min.1.min(v);
        max.0 = max.0.max(u);
        max.1 = max.1.max(v);
    }
    Some((
        min.0.clamp(0.0, 1.0),
        min.1.clamp(0.0, 1.0),
        max.0.clamp(0.0, 1.0),
        max.1.clamp(0.0, 1.0),
    ))
}
