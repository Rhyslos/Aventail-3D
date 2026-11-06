//! UV layout export ([decision 0244](../../../docs/decisions/0244-seeing-textures.md)):
//! the UV edges drawn as a picture to paint over in another program, as
//! a PNG at the texture's size (white lines on a clear background) or as
//! SVG lines.
//!
//! Lines are given in the texture's units (the square is 0 to 1, V up)
//! and drawn with V down, as images read.

use glam::Vec2;

/// The largest layout picture, in pixels across.
pub const MAX_LAYOUT_SIZE: u32 = 8192;

/// The layout as PNG bytes, `size` pixels square, each line `width`
/// pixels wide and antialiased.
pub fn layout_png(lines: &[[Vec2; 2]], size: u32, width: f32) -> Result<Vec<u8>, String> {
    let size = size.clamp(1, MAX_LAYOUT_SIZE);
    let mut alpha = vec![0f32; (size * size) as usize];
    let half = (width.max(0.5)) / 2.0;
    let s = size as f32;
    for [a, b] in lines {
        if !(a.is_finite() && b.is_finite()) {
            continue;
        }
        let p = Vec2::new(a.x * s, (1.0 - a.y) * s);
        let q = Vec2::new(b.x * s, (1.0 - b.y) * s);
        let lo = p.min(q) - Vec2::splat(half + 1.0);
        let hi = p.max(q) + Vec2::splat(half + 1.0);
        let x0 = lo.x.floor().max(0.0) as u32;
        let y0 = lo.y.floor().max(0.0) as u32;
        let x1 = (hi.x.ceil().max(0.0) as u32).min(size);
        let y1 = (hi.y.ceil().max(0.0) as u32).min(size);
        let d = q - p;
        let len2 = d.length_squared().max(1e-12);
        for y in y0..y1 {
            for x in x0..x1 {
                let c = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                let t = ((c - p).dot(d) / len2).clamp(0.0, 1.0);
                let dist = c.distance(p + d * t);
                let cover = (half + 0.5 - dist).clamp(0.0, 1.0);
                let k = (y * size + x) as usize;
                alpha[k] = alpha[k].max(cover);
            }
        }
    }
    let rgba: Vec<u8> = alpha
        .iter()
        .flat_map(|a| [255, 255, 255, (a * 255.0).round() as u8])
        .collect();
    let mut out = Vec::new();
    use image::ImageEncoder;
    image::codecs::png::PngEncoder::new(&mut out)
        .write_image(&rgba, size, size, image::ExtendedColorType::Rgba8)
        .map_err(|e| e.to_string())?;
    Ok(out)
}

/// The layout as an SVG drawing `size` units square (one per texture
/// pixel), black lines `width` wide on nothing, for vector programs.
pub fn layout_svg(lines: &[[Vec2; 2]], size: u32, width: f32) -> String {
    let size = size.clamp(1, MAX_LAYOUT_SIZE);
    let s = size as f32;
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{size}\" height=\"{size}\" \
         viewBox=\"0 0 {size} {size}\">\n<g stroke=\"black\" stroke-width=\"{}\" \
         stroke-linecap=\"round\" fill=\"none\">\n",
        width.max(0.1)
    );
    for [a, b] in lines {
        if !(a.is_finite() && b.is_finite()) {
            continue;
        }
        out.push_str(&format!(
            "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\"/>\n",
            a.x * s,
            (1.0 - a.y) * s,
            b.x * s,
            (1.0 - b.y) * s
        ));
    }
    out.push_str("</g>\n</svg>\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> Vec<[Vec2; 2]> {
        let c = [
            Vec2::new(0.25, 0.25),
            Vec2::new(0.75, 0.25),
            Vec2::new(0.75, 0.75),
            Vec2::new(0.25, 0.75),
        ];
        (0..4).map(|i| [c[i], c[(i + 1) % 4]]).collect()
    }

    #[test]
    fn the_png_has_lines_where_the_edges_are_and_nothing_elsewhere() {
        let bytes = layout_png(&square(), 64, 2.0).unwrap();
        let img = crate::image_file::read_image(&bytes).unwrap();
        assert_eq!((img.width, img.height), (64, 64));
        let alpha = |x: u32, y: u32| img.rgba[((y * 64 + x) * 4 + 3) as usize];
        // On the bottom edge (V 0.25 is 48 pixels down).
        assert!(alpha(32, 48) > 200);
        // In the middle and in a corner: clear.
        assert_eq!(alpha(32, 32), 0);
        assert_eq!(alpha(2, 2), 0);
    }

    #[test]
    fn the_svg_holds_one_line_per_edge_with_v_down() {
        let svg = layout_svg(&square(), 100, 1.0);
        assert_eq!(svg.matches("<line").count(), 4);
        assert!(svg.contains("y1=\"75.00\""), "{svg}");
        assert!(svg.starts_with("<svg"));
    }

    #[test]
    fn broken_lines_and_sizes_are_skipped_or_kept_in_range() {
        let lines = vec![[Vec2::NAN, Vec2::ONE], [Vec2::ZERO, Vec2::ONE]];
        assert_eq!(layout_svg(&lines, 0, 1.0).matches("<line").count(), 1);
        assert!(layout_png(&lines, 0, 1.0).is_ok());
        assert!(layout_png(&[], 16, 1.0).is_ok());
    }
}
