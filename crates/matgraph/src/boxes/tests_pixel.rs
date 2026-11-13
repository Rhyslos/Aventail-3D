//! Milestone 24's Pixelate box (decision 0287). The checks every box
//! gets (in range, seamless with standard settings, cores, extreme
//! settings, flat inputs) are in `tests_each.rs`; here it is checked for
//! what it says it does: an exact grid of squares, every map of a
//! material cut on the same squares, only a palette's colours, stray
//! pixels tidied, the outline, tiling square for square, the pixel grid
//! the export reads, and a pixel-art graph end to end. The speed check is
//! ignored: run it with `cargo test -p matgraph --release -- --ignored
//! pixelate_speed --nocapture`.

use super::pixelate::{PIXEL_COUNTS, cell_of};
use super::tests_each::{assert_in_range, assert_seamless, input, registry, size, with};
use crate::image::{Image, Rgb};
use crate::registry::Job;
use crate::settings::{SettingValue, Settings};
use crate::value::Value;
use crate::{BoxId, Evaluator, Graph, GraphEdit, PlugRef, ValueKind};
use glam::Vec2;
use std::sync::Arc;

use SettingValue::{Choice, Colour, Number, Palette, Switch};

const PIXELATE: &str = "pixelate";

/// The Pixels setting's choice for `n` squares across.
fn pixels(n: u32) -> SettingValue {
    Choice(PIXEL_COUNTS.iter().position(|c| *c == n).unwrap() as u32)
}

fn try_run(
    n: u32,
    s: &Settings,
    inputs: &[(&str, Value)],
) -> Result<Vec<Option<Value>>, crate::Refusal> {
    let info = registry().get(PIXELATE).unwrap().info();
    let given = info
        .inputs
        .iter()
        .map(|p| {
            inputs
                .iter()
                .find(|(k, _)| *k == p.key)
                .map(|(_, v)| v.clone())
        })
        .collect();
    let job = Job::new(info, size(n), 7, 2, s, given)
        .with_real_size(100.0)
        .with_palettes(super::tests_each::test_palettes());
    registry().get(PIXELATE).unwrap().run_outputs(&job)
}

fn run(n: u32, s: &Settings, inputs: &[(&str, Value)]) -> Vec<Option<Value>> {
    try_run(n, s, inputs).unwrap()
}

fn colour(v: &Option<Value>) -> &Rgb {
    v.as_ref().unwrap().colour().unwrap()
}

/// Whether every square of the grid `n` across `image` is one value.
fn squares_are_flat<P: Copy + PartialEq + std::fmt::Debug>(image: &Image<P>, n: u32) -> bool {
    let size = image.size();
    (0..size).all(|y| {
        (0..size).all(|x| {
            let (cx, cy) = (cell_of(x, size, n), cell_of(y, size, n));
            let (fx, fy) = (
                super::pixelate::cell_span(cx, size, n).0,
                super::pixelate::cell_span(cy, size, n).0,
            );
            image.at(i64::from(x), i64::from(y)) == image.at(i64::from(fx), i64::from(fy))
        })
    })
}

/// How many different values the squares hold.
fn distinct<P: Copy + PartialEq>(image: &Image<P>) -> Vec<P> {
    let mut seen: Vec<P> = Vec::new();
    for p in image.pixels() {
        if !seen.contains(p) {
            seen.push(*p);
        }
    }
    seen
}

#[test]
fn pixelate_is_a_finish_box_with_words_and_what_comes_next() {
    let info = registry().get(PIXELATE).unwrap().info();
    assert_eq!(info.group, crate::Group::Material);
    assert!(!info.about.show().is_empty());
    assert!(info.comes_next.iter().any(|c| c == "material_out"));
    assert!(registry().get(PIXELATE).unwrap().needs_an_input());
    assert!(registry().get(PIXELATE).unwrap().uses_palettes());
    // Every pixel count is a multiple of four (the dither repeats evenly)
    // between 16 and 256.
    assert!(
        PIXEL_COUNTS
            .iter()
            .all(|n| n % 4 == 0 && (16..=256).contains(n))
    );
}

#[test]
fn every_count_gives_exactly_that_grid_of_flat_squares() {
    let colour_in = input(ValueKind::Colour, size(256));
    for n in PIXEL_COUNTS {
        let out = run(
            256,
            &with(&[("pixels", pixels(n))]),
            &[("colour", colour_in.clone())],
        );
        let c = colour(&out[1]);
        assert!(squares_are_flat(c, n), "{n}");
        // Neighbouring squares differ somewhere: the grid isn't coarser.
        let size = 256;
        let steps = (1..size)
            .filter(|x| {
                let a = c.at(i64::from(*x) - 1, 100);
                let b = c.at(i64::from(*x), 100);
                a != b
            })
            .count();
        assert!(steps as u32 <= n, "{n}: {steps} steps");
        assert!(steps as u32 >= n / 2, "{n}: {steps} steps");
        // Only the other outputs asked for come out.
        assert!(out[0].is_none() && out[2].is_none());
    }
}

#[test]
fn the_middle_takes_the_pixel_in_the_middle_and_the_average_the_mean() {
    // A ramp across: the average of a square is its middle's value.
    let n = 64u32;
    let ramp = Value::Height(Arc::new(
        Image::from_pixels(size(n), (0..n * n).map(|i| (i % n) as f32).collect()).unwrap(),
    ));
    let s = with(&[("pixels", pixels(16))]);
    let average = run(n, &s, &[("in", ramp.clone())]);
    let g = average[0].as_ref().unwrap().grey().unwrap();
    // Squares of four pixels: 0..4 averages to 1.5.
    assert_eq!(g.at(0, 0), 1.5);
    assert_eq!(g.at(5, 9), 5.5);
    let s = with(&[("pixels", pixels(16)), ("sample", Choice(1))]);
    let middle = run(n, &s, &[("in", ramp)]);
    let g = middle[0].as_ref().unwrap().grey().unwrap();
    assert_eq!(g.at(0, 0), 1.0);
    assert_eq!(g.at(7, 3), 5.0);
}

#[test]
fn a_material_is_cut_on_the_same_squares_map_by_map() {
    let m = input(ValueKind::Material, size(128));
    for (sample, n) in [(0, 32), (1, 24), (0, 48)] {
        let s = with(&[("pixels", pixels(n)), ("sample", Choice(sample))]);
        let out = run(128, &s, &[("material", m.clone())]);
        let maps = out[2].as_ref().unwrap().material().unwrap();
        assert!(squares_are_flat(&maps.colour, n));
        for g in [
            &maps.height,
            &maps.roughness,
            &maps.metal,
            &maps.occlusion,
            &maps.curvature,
        ] {
            assert!(squares_are_flat(g, n), "{n}");
        }
        assert!(squares_are_flat(&maps.normal, n));
        // Normals stay a unit long.
        for v in maps.normal.pixels() {
            let len = glam::Vec3::from_array(*v).length();
            assert!((len - 1.0).abs() < 1e-3);
        }
        assert_in_range("material", std::slice::from_ref(out[2].as_ref().unwrap()));
    }
}

#[test]
fn limit_to_a_palette_gives_only_its_colours_with_or_without_dither() {
    let earth: Vec<[f32; 3]> = [[230, 220, 200], [60, 50, 40], [150, 120, 90], [90, 110, 70]]
        .iter()
        .map(|c: &[u8; 3]| c.map(|v| f32::from(v) / 255.0))
        .collect();
    let colour_in = input(ValueKind::Colour, size(128));
    let m = input(ValueKind::Material, size(128));
    for dither in [0.0, 0.5, 1.0] {
        for tidy in [false, true] {
            for outline in [0, 1, 2] {
                let s = with(&[
                    ("limit", Switch(true)),
                    ("palette", Palette("Earth".into())),
                    ("dither", Number(dither)),
                    ("tidy", Switch(tidy)),
                    ("outline", Choice(outline)),
                    ("outline_colour", Colour([1.0, 0.0, 1.0])),
                ]);
                let out = run(
                    128,
                    &s,
                    &[("colour", colour_in.clone()), ("material", m.clone())],
                );
                for c in [
                    colour(&out[1]),
                    &out[2].as_ref().unwrap().material().unwrap().colour,
                ] {
                    for p in distinct(c) {
                        assert!(earth.contains(&p), "{p:?} isn't Earth's");
                    }
                    assert!(squares_are_flat(c, 32));
                }
            }
        }
    }
}

#[test]
fn dither_mixes_two_swatches_in_a_checked_pattern_of_squares() {
    // A colour halfway between two greys of a palette.
    let mid = Value::Colour(Arc::new(Image::filled(size(64), [0.5, 0.5, 0.5])));
    let palettes = crate::Palettes::new([crate::Palette {
        name: "Two".into(),
        colours: vec![[90, 90, 90], [165, 165, 165]],
    }]);
    let info = registry().get(PIXELATE).unwrap().info();
    let s = with(&[
        ("pixels", pixels(16)),
        ("limit", Switch(true)),
        ("dither", Number(1.0)),
    ]);
    let job = Job::new(info, size(64), 7, 2, &s, vec![None, Some(mid), None, None])
        .with_palettes(palettes);
    let out = registry().get(PIXELATE).unwrap().run_outputs(&job).unwrap();
    let c = colour(&out[1]);
    assert_eq!(distinct(c).len(), 2);
    assert!(squares_are_flat(c, 16));
    // The pattern repeats every four squares, so it tiles.
    for y in 0..16i64 {
        for x in 0..16i64 {
            assert_eq!(c.at(x * 4, y * 4), c.at((x + 4) * 4, y * 4));
        }
    }
}

#[test]
fn limit_to_a_palette_without_palettes_says_why() {
    let info = registry().get(PIXELATE).unwrap().info();
    let s = with(&[("limit", Switch(true))]);
    let colour_in = input(ValueKind::Colour, size(32));
    let job = Job::new(
        info,
        size(32),
        7,
        2,
        &s,
        vec![None, Some(colour_in), None, None],
    );
    let refusal = registry()
        .get(PIXELATE)
        .unwrap()
        .run_outputs(&job)
        .unwrap_err();
    assert!(!refusal.reason.is_empty());
}

#[test]
fn tidying_takes_lone_specks_away() {
    // One bright pixel's worth of square on a dark ground.
    let n = 64u32;
    let mut pixels_in = vec![[0.1, 0.1, 0.1]; (n * n) as usize];
    for y in 20..24 {
        for x in 20..24 {
            pixels_in[(y * n + x) as usize] = [0.9, 0.9, 0.9];
        }
    }
    let speck = Value::Colour(Arc::new(Image::from_pixels(size(n), pixels_in).unwrap()));
    let s = with(&[("pixels", pixels(16))]);
    let kept = run(n, &s, &[("colour", speck.clone())]);
    assert_eq!(distinct(colour(&kept[1])).len(), 2);
    let s = with(&[("pixels", pixels(16)), ("tidy", Switch(true))]);
    let tidy = run(n, &s, &[("colour", speck)]);
    let left = distinct(colour(&tidy[1]));
    assert_eq!(left.len(), 1);
    assert!(left[0].iter().all(|v| (v - 0.1).abs() < 1e-5));
}

/// A raised square from `lo` to `hi` pixels each way, `n` across.
fn block(n: u32, lo: u32, hi: u32) -> Value {
    let pixels = (0..n * n)
        .map(|i| {
            let (x, y) = (i % n, i / n);
            if (lo..hi).contains(&x) && (lo..hi).contains(&y) {
                1.0
            } else {
                0.0
            }
        })
        .collect();
    Value::Shape(Arc::new(Image::from_pixels(size(n), pixels).unwrap()))
}

#[test]
fn the_outline_is_one_square_wide_on_the_edge_or_outside() {
    let n = 64u32;
    let ground = Value::Colour(Arc::new(Image::filled(size(n), [0.6, 0.7, 0.5])));
    // A shape from square 4 to 12 of 16 (pixels 16 to 48).
    let shape = block(n, 16, 48);
    let line = [1.0, 0.0, 0.0];
    let count = |outline: u32| {
        let s = with(&[
            ("pixels", pixels(16)),
            ("outline", Choice(outline)),
            ("outline_colour", Colour(line)),
        ]);
        let out = run(
            n,
            &s,
            &[("colour", ground.clone()), ("shape", shape.clone())],
        );
        let c = colour(&out[1]).clone();
        let squares = (0..16)
            .flat_map(|y| (0..16).map(move |x| (x, y)))
            .filter(|(x, y)| c.at(x * 4, y * 4) == line)
            .collect::<Vec<_>>();
        (c, squares)
    };
    let (_, none) = count(0);
    assert!(none.is_empty());
    let (_, on) = count(1);
    // The 8 × 8 shape's ring: 28 squares, all inside it.
    assert_eq!(on.len(), 28);
    assert!(
        on.iter()
            .all(|(x, y)| (4..12).contains(x) && (4..12).contains(y))
    );
    let (_, outside) = count(2);
    assert_eq!(outside.len(), 32);
    assert!(
        outside
            .iter()
            .all(|(x, y)| !((4..12).contains(x) && (4..12).contains(y)))
    );
}

#[test]
fn the_outline_follows_a_heights_level_and_makes_see_through_parts_solid() {
    let n = 64u32;
    let m = input(ValueKind::Material, size(n));
    let mut maps = (**m.material().unwrap()).clone();
    let Value::Shape(mask) = block(n, 16, 48) else {
        unreachable!()
    };
    maps.opacity = Some(mask);
    let card = Value::Material(Arc::new(maps));
    let s = with(&[("pixels", pixels(16)), ("outline", Choice(2))]);
    let out = run(n, &s, &[("material", card)]);
    let made = out[2].as_ref().unwrap().material().unwrap();
    let opacity = made.opacity.as_ref().unwrap();
    // Outside the shape, the outline's squares are solid; further out
    // still see-through.
    assert_eq!(opacity.at(3 * 4, 8 * 4), 1.0);
    assert_eq!(opacity.at(0, 0), 0.0);
    assert_eq!(opacity.at(8 * 4, 8 * 4), 1.0);
    // A height's outline moves with the level.
    let dome = input(ValueKind::Height, size(n));
    let lines = |level: f32| {
        let s = with(&[
            ("pixels", pixels(16)),
            ("outline", Choice(1)),
            ("level", Number(level)),
            ("outline_colour", Colour([0.0, 0.0, 0.0])),
        ]);
        let ground = Value::Colour(Arc::new(Image::filled(size(n), [1.0; 3])));
        let out = run(n, &s, &[("in", dome.clone()), ("colour", ground)]);
        colour(&out[1])
            .pixels()
            .iter()
            .filter(|p| **p == [0.0; 3])
            .count()
    };
    assert_ne!(lines(0.3), lines(0.7));
}

#[test]
fn an_outline_from_a_colour_alone_is_refused_with_why() {
    let colour_in = input(ValueKind::Colour, size(32));
    let s = with(&[("outline", Choice(1))]);
    let refusal = try_run(32, &s, &[("colour", colour_in)]).unwrap_err();
    assert!(
        refusal.reason.contains("Outline from"),
        "{}",
        refusal.reason
    );
    // Nothing plugged in at all.
    assert!(try_run(32, &Settings::new(), &[]).is_err());
}

#[test]
fn pixelated_textures_tile_square_for_square() {
    let colour_in = input(ValueKind::Colour, size(128));
    let m = input(ValueKind::Material, size(128));
    let shape = input(ValueKind::Shape, size(128));
    for (n, sample, outline, tidy) in [(32, 0, 0, false), (24, 1, 1, true), (64, 0, 2, true)] {
        let s = with(&[
            ("pixels", pixels(n)),
            ("sample", Choice(sample)),
            ("outline", Choice(outline)),
            ("tidy", Switch(tidy)),
            ("limit", Switch(true)),
            ("dither", Number(0.6)),
        ]);
        let out = run(
            128,
            &s,
            &[
                ("colour", colour_in.clone()),
                ("material", m.clone()),
                ("shape", shape.clone()),
            ],
        );
        let values: Vec<Value> = out.into_iter().flatten().collect();
        assert_seamless(&format!("{n}"), &values);
        // The grid wraps: the last square and the first meet at the seam,
        // each a whole square wide.
        let c = values[0].colour().unwrap();
        let first = super::pixelate::cell_span(0, 128, n);
        let last = super::pixelate::cell_span(n - 1, 128, n);
        assert_eq!(first.0, 0);
        assert_eq!(last.1, 128);
        assert!(squares_are_flat(c, n));
    }
}

#[test]
fn the_same_pictures_come_out_twice_and_on_any_cores() {
    let m = input(ValueKind::Material, size(64));
    let s = with(&[("tidy", Switch(true)), ("outline", Choice(1))]);
    let info = registry().get(PIXELATE).unwrap().info();
    let go = |threads| {
        let job = Job::new(
            info,
            size(64),
            7,
            threads,
            &s,
            vec![None, None, Some(m.clone()), None],
        );
        registry().get(PIXELATE).unwrap().run(&job).unwrap()
    };
    assert_eq!(go(1), go(2));
    assert_eq!(go(2), go(2));
}

/// A pixel-art graph: Tiles → Bevel → Gradient map, Material from height
/// → Pixelate (Earth palette, outline) → Material out.
fn pixel_graph() -> (Graph, BoxId) {
    let r = registry();
    let mut g = Graph::new("Pixel stone");
    let add = |g: &mut Graph, kind: &str| {
        let id = BoxId(g.next_id());
        g.apply(
            r,
            &GraphEdit::AddBox {
                kind: kind.into(),
                at: Vec2::ZERO,
            },
        )
        .unwrap();
        id
    };
    let ids: Vec<_> = [
        "tiles",
        "bevel",
        "gradient_map",
        "material_from_height",
        PIXELATE,
        "material_out",
    ]
    .iter()
    .map(|k| add(&mut g, k))
    .collect();
    let [tiles, bevel, map, mfh, pix, out] = ids[..] else {
        unreachable!()
    };
    for (from, f, to, t) in [
        (tiles, "shape", bevel, "shape"),
        (bevel, "height", map, "in"),
        (bevel, "height", mfh, "height"),
        (map, "colour", mfh, "colour"),
        (mfh, "material", pix, "material"),
        (pix, "material_out", out, "material"),
    ] {
        g.apply(
            r,
            &GraphEdit::Connect {
                from: PlugRef::new(from, f),
                to: PlugRef::new(to, t),
            },
        )
        .unwrap_or_else(|e| panic!("{f} → {t}: {}", e.reason));
    }
    (g, pix)
}

#[test]
fn a_pixelate_box_just_before_material_out_sets_the_pixel_grid() {
    let r = registry();
    let (mut g, pix) = pixel_graph();
    assert_eq!(crate::pixel_grid(r, &g), Some(32));
    g.apply(
        r,
        &GraphEdit::SetSetting {
            node: pix,
            key: "pixels".into(),
            value: pixels(48),
        },
    )
    .unwrap();
    assert_eq!(crate::pixel_grid(r, &g), Some(48));
    // Bypassed: no grid.
    g.apply(
        r,
        &GraphEdit::Bypass {
            node: pix,
            on: true,
        },
    )
    .unwrap();
    assert_eq!(crate::pixel_grid(r, &g), None);
    g.apply(
        r,
        &GraphEdit::Bypass {
            node: pix,
            on: false,
        },
    )
    .unwrap();
    // Pixelate's height output into nothing, or another box between it
    // and Material out: no grid.
    let (plain, _) = super::tests_each::stone_tiles();
    assert_eq!(crate::pixel_grid(r, &plain), None);
    assert_eq!(crate::pixel_grid(r, &Graph::new("Empty")), None);
}

#[test]
fn a_pixel_art_graph_makes_a_seamless_material_of_whole_squares() {
    let (g, _) = pixel_graph();
    let m = Evaluator::new()
        .material(registry(), &g, size(128))
        .unwrap();
    assert!(squares_are_flat(&m.colour, 32));
    assert!(squares_are_flat(&m.normal, 32));
    let value = Value::Material(m);
    assert_in_range("graph", std::slice::from_ref(&value));
    let check = crate::SeamCheck::of(&value);
    assert!(!check.shows(), "{}", check.describe());
}

#[test]
#[ignore = "speed check: run by hand"]
fn pixelate_speed() {
    let threads = crate::parallel::cores();
    let sz = size(2048);
    let info = registry().get(PIXELATE).unwrap().info();
    let inputs = info
        .inputs
        .iter()
        .map(|p| Some(input(p.kind, sz)))
        .collect();
    let s = with(&[
        ("limit", Switch(true)),
        ("tidy", Switch(true)),
        ("outline", Choice(1)),
        ("pixels", pixels(256)),
    ]);
    let job = Job::new(info, sz, 1, threads, &s, inputs)
        .with_palettes(super::tests_each::test_palettes());
    let start = std::time::Instant::now();
    registry().get(PIXELATE).unwrap().run(&job).unwrap();
    let took = start.elapsed();
    #[allow(clippy::print_stdout)]
    {
        println!("pixelate {:>8.1} ms at 2048", took.as_secs_f64() * 1000.0);
    }
    assert!(took.as_secs() < 60, "took {took:?}");
}
