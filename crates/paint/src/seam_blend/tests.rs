//! Tests for Seam blend layers (decision 0293).

use super::*;
use crate::seams::seams_of;
use crate::seams::tests::{side_by_side, two_pieces};
use glam::Vec2;

const RED: [u8; 4] = [220, 20, 20, 255];
const BLUE: [u8; 4] = [20, 20, 220, 255];
const SIZE: u32 = 128;

/// The texture with the left half red and the right half blue: each of
/// the test pieces painted its own colour.
fn halves() -> Image {
    let mut image = Image::new(SIZE).unwrap();
    for y in 0..SIZE {
        for x in 0..SIZE {
            image.set_pixel(x, y, if x < SIZE / 2 { RED } else { BLUE });
        }
    }
    image
}

/// `below` with `layer` laid over it.
fn shown(below: &Image, layer: &Image) -> Image {
    let mut out = below.clone();
    for y in 0..SIZE {
        for x in 0..SIZE {
            let mut p = below.pixel(x, y);
            crate::blend::over(&mut p, &layer.pixel(x, y), 1.0, crate::Blend::Normal);
            out.set_pixel(x, y, p);
        }
    }
    out
}

fn texel(uv: Vec2) -> (u32, u32) {
    let s = SIZE as f32;
    ((uv.x * s) as u32, ((1.0 - uv.y) * s) as u32)
}

/// Blends every seam of `surface` with `width`.
fn blended(surface: &PaintSurface, below: &Image, width: f32) -> Image {
    let seams = seams_of(surface, &[0]);
    let shape = BlendShape {
        id: 7,
        surface,
        slots: &[0],
        seams: &seams,
    };
    let chosen: Vec<SeamId> = seams
        .iter()
        .map(|s| SeamId {
            object: 7,
            points: s.points,
        })
        .collect();
    blend(below, &[shape], &chosen, width).unwrap()
}

fn purple(p: [u8; 4]) -> bool {
    p[0] > 70 && p[2] > 70 && p[0].abs_diff(p[2]) < 60
}

#[test]
fn both_sides_meet_in_the_middle_and_fade_out_over_the_width() {
    let surface = side_by_side();
    let below = halves();
    let layer = blended(&surface, &below, 8.0);
    let out = shown(&below, &layer);
    // On the seam's middle, a texel in from each side: both half way.
    let (lx, ly) = texel(Vec2::new(0.25, 0.25));
    let (rx, ry) = texel(Vec2::new(0.75, 0.25));
    assert!(purple(out.pixel(lx + 1, ly)), "{:?}", out.pixel(lx + 1, ly));
    assert!(purple(out.pixel(rx - 1, ry)), "{:?}", out.pixel(rx - 1, ry));
    // Far from the seam, untouched.
    let (fx, fy) = texel(Vec2::new(0.38, 0.12));
    assert_eq!(out.pixel(fx, fy), RED);
    assert_eq!(layer.pixel(fx, fy)[3], 0);
    // Fading: stronger nearer the seam.
    let near = layer.pixel(lx + 1, ly)[3];
    let farther = layer.pixel(lx + 5, ly)[3];
    assert!(near > farther, "{near} {farther}");
    // Outside the pieces nothing is written.
    assert!(layer.pixel(2, 2)[3] == 0 && layer.pixel(SIZE - 2, SIZE - 2)[3] == 0);
}

#[test]
fn a_mirrored_piece_still_meets_its_neighbour() {
    // The right piece flipped left to right: its seam edge runs the other
    // way and its inside lies on the other side.
    let surface = two_pieces(
        [
            Vec2::new(0.1, 0.1),
            Vec2::new(0.4, 0.1),
            Vec2::new(0.4, 0.4),
        ],
        [
            Vec2::new(0.9, 0.1),
            Vec2::new(0.6, 0.4),
            Vec2::new(0.9, 0.4),
        ],
    );
    let below = halves();
    let out = shown(&below, &blended(&surface, &below, 8.0));
    let (lx, ly) = texel(Vec2::new(0.25, 0.25));
    let (rx, ry) = texel(Vec2::new(0.75, 0.25));
    assert!(purple(out.pixel(lx + 1, ly)), "{:?}", out.pixel(lx + 1, ly));
    assert!(purple(out.pixel(rx + 1, ry)), "{:?}", out.pixel(rx + 1, ry));
}

#[test]
fn matching_colours_stay_as_they_are() {
    let surface = side_by_side();
    let mut below = Image::new(SIZE).unwrap();
    for y in 0..SIZE {
        for x in 0..SIZE {
            below.set_pixel(x, y, RED);
        }
    }
    let out = shown(&below, &blended(&surface, &below, 12.0));
    for y in 0..SIZE {
        for x in 0..SIZE {
            let p = out.pixel(x, y);
            assert!(
                p[0].abs_diff(RED[0]) <= 1 && p[2].abs_diff(RED[2]) <= 1,
                "{p:?}"
            );
        }
    }
}

#[test]
fn a_wider_blend_reaches_farther() {
    let surface = side_by_side();
    let below = halves();
    let reach = |width: f32| {
        let layer = blended(&surface, &below, width);
        let (lx, ly) = texel(Vec2::new(0.25, 0.25));
        (1..40)
            .take_while(|d| layer.pixel(lx + d, ly)[3] > 0)
            .count()
    };
    assert!(
        reach(16.0) > reach(4.0) + 4,
        "{} {}",
        reach(16.0),
        reach(4.0)
    );
}

#[test]
fn unpainted_sides_and_missing_seams_blend_nothing() {
    let surface = side_by_side();
    let empty = Image::new(SIZE).unwrap();
    assert_eq!(blended(&surface, &empty, 8.0).painted_tiles(), 0);
    // Only the left half painted: no colour to bring across.
    let mut left = Image::new(SIZE).unwrap();
    for y in 0..SIZE {
        for x in 0..SIZE / 2 {
            left.set_pixel(x, y, RED);
        }
    }
    assert_eq!(blended(&surface, &left, 8.0).painted_tiles(), 0);
    // A seam no longer on the model is counted, not blended.
    let seams = seams_of(&surface, &[0]);
    let shape = BlendShape {
        id: 7,
        surface: &surface,
        slots: &[0],
        seams: &seams,
    };
    let gone = [
        SeamId {
            object: 7,
            points: [5, 9],
        },
        SeamId {
            object: 8,
            points: [0, 2],
        },
    ];
    let (found, missing) = resolve(&[shape], &gone);
    assert!(found.is_empty());
    assert_eq!(missing, 2);
    assert_eq!(
        blend(&halves(), &[shape], &gone, 8.0)
            .unwrap()
            .painted_tiles(),
        0
    );
}

#[test]
fn the_tiles_used_cover_what_is_written() {
    let surface = side_by_side();
    let below = halves();
    let seams = seams_of(&surface, &[0]);
    let shape = BlendShape {
        id: 7,
        surface: &surface,
        slots: &[0],
        seams: &seams,
    };
    let chosen = [SeamId {
        object: 7,
        points: [0, 2],
    }];
    let tiles = tiles_used(&[shape], &chosen, 8.0, SIZE);
    assert!(!tiles.is_empty());
    let layer = blend(&below, &[shape], &chosen, 8.0).unwrap();
    for (index, tile) in layer.tiles().iter().enumerate() {
        if tile.is_some() {
            assert!(tiles.contains(&index), "tile {index}");
        }
    }
    // The layers below worked out on those tiles alone give the same.
    let only = crate::composite::flatten_tiles(
        &[crate::Layer::paint("Halves", below.clone())],
        SIZE,
        &tiles,
    )
    .unwrap();
    assert_eq!(blend(&only, &[shape], &chosen, 8.0).unwrap(), layer);
    assert!(tiles_used(&[shape], &[], 8.0, SIZE).is_empty());
}

#[test]
fn settings_are_kept_sorted_and_in_range() {
    let mut blend = SeamBlend::default();
    assert!(blend.is_valid());
    assert_eq!(blend.width, WIDTH);
    let id = |object: u64, a: u32, b: u32| SeamId {
        object,
        points: [a, b],
    };
    blend.mark([id(2, 1, 4), id(1, 3, 5)], false);
    blend.mark([id(1, 3, 5), id(1, 0, 1)], true);
    assert_eq!(blend.seams, vec![id(1, 0, 1), id(1, 3, 5), id(2, 1, 4)]);
    assert!(blend.marks_all(&[id(1, 0, 1), id(2, 1, 4)]));
    assert!(!blend.marks_all(&[id(1, 0, 1), id(9, 1, 4)]));
    assert!(!blend.marks_all(&[]));
    blend.unmark(&[id(1, 3, 5)]);
    assert_eq!(blend.seams.len(), 2);
    blend.mark([id(4, 0, 1)], false);
    assert_eq!(blend.seams, vec![id(4, 0, 1)]);
    assert!(blend.is_valid());
    for (width, usable) in [
        (0.0, MIN_WIDTH),
        (500.0, MAX_WIDTH),
        (f32::NAN, WIDTH),
        (3.0, 3.0),
    ] {
        assert_eq!(SeamBlend::usable_width(width), usable);
    }
    blend.width = 100.0;
    assert!(!blend.is_valid());
    blend.width = 4.0;
    blend.seams = vec![id(2, 0, 1), id(1, 0, 1)];
    assert!(!blend.is_valid(), "out of order");
}

#[test]
fn the_layers_under_a_layer_are_those_it_lies_on() {
    use crate::{Layer, TextureSet};
    let mut set = TextureSet::new(16).unwrap();
    let image = || Image::new(16).unwrap();
    set.layers = vec![
        Layer::paint("A", image()),
        Layer::group(
            "G",
            vec![Layer::paint("B", image()), Layer::paint("C", image())],
        ),
        Layer::paint("D", image()),
    ];
    let names = |layers: &[Layer]| -> Vec<String> {
        layers
            .iter()
            .map(|l| match l.children() {
                Some(c) => format!(
                    "{}({})",
                    l.name,
                    c.iter()
                        .map(|l| l.name.clone())
                        .collect::<Vec<_>>()
                        .join(",")
                ),
                None => l.name.clone(),
            })
            .collect()
    };
    assert_eq!(names(&set.below(&[2])), ["A", "G(B,C)"]);
    assert_eq!(names(&set.below(&[1, 1])), ["A", "G(B)"]);
    assert_eq!(names(&set.below(&[0])), Vec::<String>::new());
    assert!(set.below(&[9]).is_empty());
}

/// Fuzz-style: odd UVs (huge, tiny, not real, off the texture) never
/// panic and write nothing outside the texture.
#[test]
fn odd_uvs_never_crash_the_blend() {
    let below = halves();
    let mut state = 293u64;
    let mut next = move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (state >> 33) as f32 / (1u64 << 31) as f32
    };
    let odd = [f32::NAN, f32::INFINITY, -3.0, 5.0, 1e9, 1e-9, 0.0, 1.0];
    for round in 0..200 {
        let mut uv = || {
            let pick = next();
            if pick < 0.2 {
                odd[(next() * odd.len() as f32) as usize % odd.len()]
            } else {
                next() * 1.4 - 0.2
            }
        };
        let mut corners = [Vec2::ZERO; 6];
        for c in &mut corners {
            *c = Vec2::new(uv(), uv());
        }
        let surface = two_pieces(
            [corners[0], corners[1], corners[2]],
            [corners[3], corners[4], corners[5]],
        );
        let width = [1.0, 8.0, 64.0, -1.0, f32::NAN][round % 5];
        let layer = blended(&surface, &below, width);
        assert_eq!(layer.size(), SIZE);
    }
}
