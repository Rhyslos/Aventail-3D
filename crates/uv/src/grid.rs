//! The numbered checker ([decision 0107](../../../docs/decisions/0107-numbered-checker.md)):
//! an image to lay on shapes by their UVs, made here rather than loaded,
//! so it follows the app's colours.
//!
//! The texture square is cut into [`CELLS`] × [`CELLS`] cells, each a
//! colour of a gradient from the top left to the bottom right, holding a
//! checker of lighter and darker squares and a label: a letter for its
//! column (A at the left) and a number for its row (1 at the top). The
//! labels make it easy to tell where on the texture a part of the model
//! is, and show at once when a piece lies turned or mirrored, because the
//! text does too. The letters are drawn with a small pixel font of the
//! app's own.

/// Cells across (and down) the texture square.
pub const CELLS: usize = 8;

/// Light and dark squares across each cell.
pub const SQUARES_PER_CELL: usize = 4;

/// The pixel font: each character 5 pixels wide and 7 tall, a row to a
/// byte (the lowest 5 bits, the leftmost pixel highest).
const GLYPHS: [(char, [u8; 7]); 18] = [
    ('A', [0x0e, 0x11, 0x11, 0x1f, 0x11, 0x11, 0x11]),
    ('B', [0x1e, 0x11, 0x11, 0x1e, 0x11, 0x11, 0x1e]),
    ('C', [0x0e, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0e]),
    ('D', [0x1e, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1e]),
    ('E', [0x1f, 0x10, 0x10, 0x1e, 0x10, 0x10, 0x1f]),
    ('F', [0x1f, 0x10, 0x10, 0x1e, 0x10, 0x10, 0x10]),
    ('G', [0x0e, 0x11, 0x10, 0x17, 0x11, 0x11, 0x0f]),
    ('H', [0x11, 0x11, 0x11, 0x1f, 0x11, 0x11, 0x11]),
    ('0', [0x0e, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0e]),
    ('1', [0x04, 0x0c, 0x04, 0x04, 0x04, 0x04, 0x0e]),
    ('2', [0x0e, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1f]),
    ('3', [0x1f, 0x02, 0x04, 0x02, 0x01, 0x11, 0x0e]),
    ('4', [0x02, 0x06, 0x0a, 0x12, 0x1f, 0x02, 0x02]),
    ('5', [0x1f, 0x10, 0x1e, 0x01, 0x01, 0x11, 0x0e]),
    ('6', [0x06, 0x08, 0x10, 0x1e, 0x11, 0x11, 0x0e]),
    ('7', [0x1f, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08]),
    ('8', [0x0e, 0x11, 0x11, 0x0e, 0x11, 0x11, 0x0e]),
    ('9', [0x0e, 0x11, 0x11, 0x0f, 0x01, 0x02, 0x0c]),
];

/// The label of the cell in column `x` and row `y` (from the top): "A1"
/// at the top left.
pub fn label(x: usize, y: usize) -> String {
    let column = char::from(b'A' + (x % 26) as u8);
    format!("{column}{}", y + 1)
}

fn glyph(c: char) -> Option<[u8; 7]> {
    GLYPHS.iter().find(|(g, _)| *g == c).map(|(_, rows)| *rows)
}

/// A colour as hue (0 to 1), saturation and value.
fn to_hsv([r, g, b]: [u8; 3]) -> [f32; 3] {
    let (r, g, b) = (
        f32::from(r) / 255.0,
        f32::from(g) / 255.0,
        f32::from(b) / 255.0,
    );
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d <= 0.0 {
        0.0
    } else if max == r {
        ((g - b) / d).rem_euclid(6.0) / 6.0
    } else if max == g {
        ((b - r) / d + 2.0) / 6.0
    } else {
        ((r - g) / d + 4.0) / 6.0
    };
    let s = if max <= 0.0 { 0.0 } else { d / max };
    [h, s, max]
}

fn from_hsv([h, s, v]: [f32; 3]) -> [u8; 3] {
    let h6 = h.rem_euclid(1.0) * 6.0;
    let c = v * s;
    let x = c * (1.0 - (h6 % 2.0 - 1.0).abs());
    let (r, g, b) = match h6 as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    let byte = |f: f32| ((f + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    [byte(r), byte(g), byte(b)]
}

/// `t` of the way from `a` to `b` round the colour wheel (the shorter
/// way), so the colours between stay as vivid as the ends.
fn blend(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    let (x, y) = (to_hsv(a), to_hsv(b));
    let mut turn = y[0] - x[0];
    if turn > 0.5 {
        turn -= 1.0;
    } else if turn < -0.5 {
        turn += 1.0;
    }
    from_hsv([
        x[0] + turn * t,
        x[1] + (y[1] - x[1]) * t,
        x[2] + (y[2] - x[2]) * t,
    ])
}

fn mix(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    let t = t.clamp(0.0, 1.0);
    let c = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * t).round() as u8;
    [c(a[0], b[0]), c(a[1], b[1]), c(a[2], b[2])]
}

/// The numbered checker, `size` pixels square (at least [`CELLS`] × 8),
/// its cells running from colour `from` at the top left to `to` at the
/// bottom right (sRGB). Four bytes per pixel (red, green, blue, opaque),
/// row by row from the top of the texture square (V = 1) down.
pub fn numbered_checker(size: usize, from: [u8; 3], to: [u8; 3]) -> Vec<u8> {
    let size = size.max(CELLS * 8);
    let mut out = vec![255u8; size * size * 4];
    let cell = size as f32 / CELLS as f32;
    let square = cell / SQUARES_PER_CELL as f32;
    // The cell each pixel is in, and where in it.
    let which = |p: usize| ((p as f32 / cell) as usize).min(CELLS - 1);
    // Each cell's colour, worked out once.
    let bases: Vec<[u8; 3]> = (0..2 * CELLS - 1)
        .map(|k| blend(from, to, k as f32 / (2 * (CELLS - 1)) as f32))
        .collect();
    for y in 0..size {
        for x in 0..size {
            let (cx, cy) = (which(x), which(y));
            let base = bases[cx + cy];
            let sx = (x as f32 / square) as usize;
            let sy = (y as f32 / square) as usize;
            let mut c = if (sx + sy).is_multiple_of(2) {
                mix(base, [255, 255, 255], 0.22)
            } else {
                mix(base, [0, 0, 0], 0.18)
            };
            // A thin darker line between cells.
            let (ix, iy) = (x as f32 - cx as f32 * cell, y as f32 - cy as f32 * cell);
            if ix < 1.0 || iy < 1.0 || ix >= cell - 1.0 || iy >= cell - 1.0 {
                c = mix(base, [0, 0, 0], 0.55);
            }
            let at = (y * size + x) * 4;
            out[at..at + 3].copy_from_slice(&c);
        }
    }
    // Each cell's label on a dark badge in its middle.
    let scale = ((cell / 32.0).floor() as usize).max(1);
    for cy in 0..CELLS {
        for cx in 0..CELLS {
            let text = label(cx, cy);
            let chars: Vec<[u8; 7]> = text.chars().filter_map(glyph).collect();
            let width = (chars.len() * 6 - 1) * scale;
            let height = 7 * scale;
            let pad = 2 * scale;
            let mid_x = (cx as f32 * cell + cell / 2.0) as usize;
            let mid_y = (cy as f32 * cell + cell / 2.0) as usize;
            let left = mid_x.saturating_sub(width / 2);
            let top = mid_y.saturating_sub(height / 2);
            let base = bases[cx + cy];
            let badge = mix(base, [0, 0, 0], 0.65);
            let mut put = |x: usize, y: usize, c: [u8; 3]| {
                if x < size && y < size {
                    let at = (y * size + x) * 4;
                    out[at..at + 3].copy_from_slice(&c);
                }
            };
            for y in top.saturating_sub(pad)..top + height + pad {
                for x in left.saturating_sub(pad)..left + width + pad {
                    put(x, y, badge);
                }
            }
            for (k, rows) in chars.iter().enumerate() {
                for (row, bits) in rows.iter().enumerate() {
                    for col in 0..5 {
                        if bits & (0x10 >> col) == 0 {
                            continue;
                        }
                        let x0 = left + (k * 6 + col) * scale;
                        let y0 = top + row * scale;
                        for dy in 0..scale {
                            for dx in 0..scale {
                                put(x0 + dx, y0 + dy, [245, 245, 245]);
                            }
                        }
                    }
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixel(image: &[u8], size: usize, x: usize, y: usize) -> [u8; 3] {
        let at = (y * size + x) * 4;
        [image[at], image[at + 1], image[at + 2]]
    }

    #[test]
    fn cells_are_labelled_by_column_and_row() {
        assert_eq!(label(0, 0), "A1");
        assert_eq!(label(7, 7), "H8");
        assert_eq!(label(2, 4), "C5");
        for y in 0..CELLS {
            for x in 0..CELLS {
                assert!(label(x, y).chars().all(|c| glyph(c).is_some()));
            }
        }
    }

    #[test]
    fn the_image_runs_from_one_colour_to_the_other_with_labels() {
        let size = 256;
        let from = [200, 60, 60];
        let to = [40, 120, 200];
        let image = numbered_checker(size, from, to);
        assert_eq!(image.len(), size * size * 4);
        assert!(image.chunks(4).all(|p| p[3] == 255));
        // A light square near the top left corner is the first colour,
        // lightened; near the bottom right, the second.
        let near = |a: [u8; 3], b: [u8; 3]| a.iter().zip(&b).all(|(x, y)| x.abs_diff(*y) < 60);
        assert!(near(pixel(&image, size, 3, 3), mix(from, [255; 3], 0.22)));
        assert!(near(
            pixel(&image, size, size - 4, size - 4),
            mix(to, [255; 3], 0.22)
        ));
        // The middle of a cell has its badge and white letters.
        let cell = size / CELLS;
        let badge: Vec<[u8; 3]> = (0..cell)
            .map(|x| pixel(&image, size, x, cell / 2))
            .collect();
        assert!(badge.contains(&[245, 245, 245]), "the label's letters");
        // Neighbouring cells differ, so pieces can be told apart.
        assert_ne!(
            pixel(&image, size, cell + 3, cell * 3 + 3),
            pixel(&image, size, cell * 6 + 3, cell * 3 + 3)
        );
    }

    #[test]
    fn colours_blend_round_the_wheel() {
        for c in [
            [200, 60, 60],
            [40, 120, 200],
            [128, 128, 128],
            [0, 0, 0],
            [255, 255, 0],
        ] {
            let back = from_hsv(to_hsv(c));
            assert!(
                c.iter().zip(&back).all(|(a, b)| a.abs_diff(*b) <= 1),
                "{c:?} {back:?}"
            );
        }
        // Red to green passes yellow, not grey.
        let middle = blend([255, 0, 0], [0, 255, 0], 0.5);
        assert!(
            middle[0] > 200 && middle[1] > 200 && middle[2] < 30,
            "{middle:?}"
        );
        assert_eq!(blend([10, 20, 30], [200, 100, 50], 0.0), [10, 20, 30]);
    }

    #[test]
    fn tiny_sizes_are_made_big_enough() {
        let image = numbered_checker(1, [0; 3], [255; 3]);
        assert_eq!(image.len(), (CELLS * 8).pow(2) * 4);
    }
}

/// Writes the image's bytes to the file named by `PREVIEW`, to look at:
/// `PREVIEW=/tmp/grid.raw cargo test -p uv write_preview -- --ignored`.
#[cfg(test)]
#[test]
#[ignore = "writes a preview"]
fn write_preview() {
    let image = numbered_checker(1024, [0xc2, 0x43, 0x3d], [0x3d, 0xc2, 0x8f]);
    std::fs::write(std::env::var("PREVIEW").unwrap(), image).unwrap();
}
