//! Scanline flood fill with colour tolerance (Fill tool).

use crate::raster::RasterImage;
use crate::types::{BlendMode, Rgba};

/// Normalised colour distance used for tolerance comparisons.
///
/// Compares RGB and alpha as the largest per-channel difference (`0..=1`).
pub fn color_distance(a: Rgba, b: Rgba) -> f32 {
    let d = |x: u8, y: u8| (x as f32 - y as f32).abs() / 255.0;
    d(a.r, b.r)
        .max(d(a.g, b.g))
        .max(d(a.b, b.b))
        .max(d(a.a, b.a))
}

/// Fill the connected region around `(x, y)` whose pixels are within
/// `tolerance` (`0..=1`) of the seed colour.
///
/// Returns how many pixels were painted. Out-of-bounds seeds are ignored.
pub fn flood_fill(
    image: &mut RasterImage,
    x: u32,
    y: u32,
    color: Rgba,
    tolerance: f32,
    opacity: f32,
    blend: BlendMode,
) -> usize {
    if x >= image.width || y >= image.height {
        return 0;
    }
    let width = image.width as i64;
    let height = image.height as i64;
    let tolerance = tolerance.clamp(0.0, 1.0);
    let seed = image.get(x, y);

    let idx = |x: i64, y: i64| (y * width + x) as usize;
    let mut visited = vec![false; image.len()];
    let mut stack: Vec<(i64, i64)> = vec![(x as i64, y as i64)];
    let mut filled = 0usize;

    while let Some((sx, sy)) = stack.pop() {
        if sx < 0 || sy < 0 || sx >= width || sy >= height {
            continue;
        }
        if visited[idx(sx, sy)] || color_distance(image.get(sx as u32, sy as u32), seed) > tolerance
        {
            continue;
        }

        let mut left = sx;
        while left - 1 >= 0 {
            let i = idx(left - 1, sy);
            if visited[i] || color_distance(image.get((left - 1) as u32, sy as u32), seed) > tolerance
            {
                break;
            }
            left -= 1;
        }
        let mut right = sx;
        while right + 1 < width {
            let i = idx(right + 1, sy);
            if visited[i]
                || color_distance(image.get((right + 1) as u32, sy as u32), seed) > tolerance
            {
                break;
            }
            right += 1;
        }

        for px in left..=right {
            let i = idx(px, sy);
            visited[i] = true;
            let dst = image.pixel_at(i);
            image.set_index(i, RasterImage::blend(dst, color, opacity, blend));
            filled += 1;

            if sy - 1 >= 0 {
                let up = idx(px, sy - 1);
                if !visited[up]
                    && color_distance(image.get(px as u32, (sy - 1) as u32), seed) <= tolerance
                {
                    stack.push((px, sy - 1));
                }
            }
            if sy + 1 < height {
                let down = idx(px, sy + 1);
                if !visited[down]
                    && color_distance(image.get(px as u32, (sy + 1) as u32), seed) <= tolerance
                {
                    stack.push((px, sy + 1));
                }
            }
        }
    }

    filled
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image_with_barrier() -> RasterImage {
        let mut img = RasterImage::new(8, 8);
        for y in 0..8 {
            img.set(4, y, Rgba::opaque(0, 0, 0));
        }
        img
    }

    #[test]
    fn fill_stops_at_colour_barrier() {
        let mut img = image_with_barrier();
        let filled = flood_fill(
            &mut img,
            0,
            0,
            Rgba::opaque(255, 0, 0),
            0.0,
            1.0,
            BlendMode::Normal,
        );

        assert_eq!(filled, 4 * 8);
        assert_eq!(img.get(3, 3).r, 255);
        assert_eq!(img.get(5, 3).a, 0);
        assert_eq!(img.get(4, 3).a, 255);
    }

    #[test]
    fn tolerance_includes_similar_colours() {
        let gradient = || {
            let mut img = RasterImage::new(2, 1);
            img.set(0, 0, Rgba::opaque(100, 100, 100));
            img.set(1, 0, Rgba::opaque(110, 110, 110));
            img
        };

        let mut exact_image = gradient();
        let exact = flood_fill(
            &mut exact_image,
            0,
            0,
            Rgba::opaque(0, 0, 0),
            0.0,
            1.0,
            BlendMode::Normal,
        );
        assert_eq!(exact, 1);

        let mut loose_image = gradient();
        let loose = flood_fill(
            &mut loose_image,
            0,
            0,
            Rgba::opaque(0, 0, 0),
            0.05,
            1.0,
            BlendMode::Normal,
        );
        assert_eq!(loose, 2);
    }

    #[test]
    fn out_of_bounds_seed_fills_nothing() {
        let mut img = RasterImage::new(4, 4);
        assert_eq!(
            flood_fill(
                &mut img,
                9,
                9,
                Rgba::opaque(255, 255, 255),
                1.0,
                1.0,
                BlendMode::Normal
            ),
            0
        );
    }
}
