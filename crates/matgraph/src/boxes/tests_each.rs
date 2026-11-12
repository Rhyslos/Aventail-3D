//! Each standard box's checks: its outputs stay in range, it tiles
//! seamlessly, the same seed gives the same pictures (on any number of
//! cores), extreme settings don't break it, and it does what it says.
//! A stone-tiles graph runs end to end. Speed checks are ignored: run
//! them by hand with `cargo test -p matgraph --release -- --ignored
//! box_speed --nocapture`.

use crate::graph::{BoxId, Graph, GraphEdit, PlugRef};
use crate::image::{Grey, Image, Rgb, Size};
use crate::registry::{Job, Registry};
use crate::settings::{
    ColourGradient, GradientStop, SettingKind, SettingSpec, SettingValue, Settings,
};
use crate::value::{MaterialMaps, Value};
use crate::{Evaluator, ValueKind};
use glam::Vec2;
use std::sync::Arc;

pub(super) fn registry() -> &'static Registry {
    Registry::standard()
}

/// Settings with `key` set to `value`.
pub(super) fn with(pairs: &[(&str, SettingValue)]) -> Settings {
    let mut s = Settings::new();
    for (k, v) in pairs {
        s.set(*k, *v);
    }
    s
}

/// Runs box `kind` at `size` with seed `seed` on `threads` cores, with
/// `settings`, a real-world size of `cm`, and patterned, seamless inputs
/// of each kind it takes.
fn run_with(
    kind: &str,
    size: Size,
    seed: u64,
    threads: usize,
    settings: &Settings,
    cm: f32,
) -> Vec<Value> {
    run_inputs(kind, size, seed, threads, settings, cm, true)
}

/// As [`run_with`], with inputs that aren't needed left empty unless
/// `all`.
fn run_inputs(
    kind: &str,
    size: Size,
    seed: u64,
    threads: usize,
    settings: &Settings,
    cm: f32,
    all: bool,
) -> Vec<Value> {
    let info = registry().get(kind).unwrap().info();
    // A box needing one of its inputs (Transform, Repeat) gets its first.
    let one = registry().get(kind).unwrap().needs_an_input();
    let inputs = info
        .inputs
        .iter()
        .enumerate()
        .map(|(i, p)| (all || p.needed || (one && i == 0)).then(|| input(p.kind, size)))
        .collect();
    let job = Job::new(info, size, seed, threads, settings, inputs).with_real_size(cm);
    registry().get(kind).unwrap().run(&job).unwrap()
}

fn run(kind: &str, size: Size, settings: &Settings) -> Vec<Value> {
    run_with(kind, size, 7, 2, settings, 100.0)
}

pub(super) fn size(n: u32) -> Size {
    Size::new(n).unwrap()
}

/// A seamless input of `kind` with a pattern in it: noise for a height,
/// bricks for a shape, noise coloured for a colour, and a material made
/// from noise.
pub(super) fn input(kind: ValueKind, size: Size) -> Value {
    let alone = |id: &str| {
        let info = registry().get(id).unwrap().info();
        let job = Job::new(info, size, 3, 1, &Settings::new(), Vec::new());
        registry().get(id).unwrap().run(&job).unwrap().remove(0)
    };
    match kind {
        ValueKind::Shape => alone("bricks"),
        ValueKind::Height => alone("noise"),
        ValueKind::Colour => {
            let noise = alone("noise");
            let grey = noise.grey().unwrap();
            Value::Colour(Arc::new(grey.map(|v| [v, v * 0.5, 1.0 - v])))
        }
        ValueKind::Material => {
            let info = registry().get("material_from_height").unwrap().info();
            let job = Job::new(
                info,
                size,
                3,
                1,
                &Settings::new(),
                vec![Some(alone("noise")), None],
            );
            registry()
                .get("material_from_height")
                .unwrap()
                .run(&job)
                .unwrap()
                .remove(0)
        }
    }
}

/// Every map of a value, as greys (colours and normals channel by
/// channel), with whether each should be 0 to 1.
fn channels(value: &Value) -> Vec<Grey> {
    let rgb = |c: &Rgb| -> Vec<Grey> { (0..3).map(|i| c.map(|p| p[i])).collect() };
    match value {
        Value::Shape(g) | Value::Height(g) => vec![(**g).clone()],
        Value::Colour(c) => rgb(c),
        Value::Material(m) => {
            let mut all = rgb(&m.colour);
            for g in [
                &m.height,
                &m.roughness,
                &m.metal,
                &m.occlusion,
                &m.curvature,
            ] {
                all.push((**g).clone());
            }
            // The normal's channels go from -1 to 1: moved into 0 to 1.
            all.extend(rgb(&m.normal).into_iter().map(|g| g.map(|v| v * 0.5 + 0.5)));
            all
        }
    }
}

/// Every pixel of every map is a real number from 0 to 1, and normals
/// are a unit long.
pub(super) fn assert_in_range(what: &str, values: &[Value]) {
    for value in values {
        for grey in channels(value) {
            for v in grey.pixels() {
                assert!(
                    v.is_finite() && (-1e-5..=1.0 + 1e-5).contains(v),
                    "{what}: {v}"
                );
            }
        }
        if let Value::Material(m) = value {
            for n in m.normal.pixels() {
                let len = glam::Vec3::from_array(*n).length();
                assert!((len - 1.0).abs() < 1e-3, "{what}: normal {len}");
            }
        }
    }
}

/// How far apart neighbouring pixels are across the wrap seam, against
/// the most they differ anywhere inside.
fn seam_and_inside(image: &Grey) -> (f32, f32) {
    let n = i64::from(image.size());
    let mut seam: f32 = 0.0;
    let mut inside: f32 = 0.0;
    for i in 0..n {
        seam = seam
            .max((image.at(n - 1, i) - image.at(0, i)).abs())
            .max((image.at(i, n - 1) - image.at(i, 0)).abs());
        for j in 0..n - 1 {
            inside = inside
                .max((image.at(j, i) - image.at(j + 1, i)).abs())
                .max((image.at(i, j) - image.at(i, j + 1)).abs());
        }
    }
    (seam, inside)
}

/// Every map of every value tiles: the jump across the left and right
/// edges, and the top and bottom, is no bigger than jumps inside.
pub(super) fn assert_seamless(what: &str, values: &[Value]) {
    for value in values {
        for grey in channels(value) {
            let (seam, inside) = seam_and_inside(&grey);
            assert!(seam <= inside + 1e-4, "{what}: seam {seam} inside {inside}");
        }
    }
}

/// Every box with an id other than Material out, which only passes on.
fn every_box() -> Vec<String> {
    registry()
        .iter()
        .map(|t| t.info().id.to_string())
        .filter(|id| id != "material_out")
        .collect()
}

#[test]
fn every_box_makes_what_it_says_in_range() {
    for kind in every_box() {
        let out = run(&kind, size(32), &Settings::new());
        let info = registry().get(&kind).unwrap().info();
        assert_eq!(out.len(), info.outputs.len(), "{kind}");
        for (value, plug) in out.iter().zip(&info.outputs) {
            assert_eq!(value.kind(), plug.kind, "{kind} {}", plug.key);
            assert_eq!(value.size(), 32, "{kind}");
        }
        assert_in_range(&kind, &out);
    }
}

#[test]
fn every_box_tiles_seamlessly_with_its_standard_settings() {
    for kind in every_box() {
        let out = run(&kind, size(64), &Settings::new());
        assert_seamless(&kind, &out);
    }
}

#[test]
fn the_patterns_tile_seamlessly_with_other_settings_too() {
    use SettingValue::{Choice, Number, Switch, Whole};
    let cases: Vec<(&str, Settings)> = vec![
        ("clouds", with(&[("look", Choice(1)), ("scale", Whole(5))])),
        (
            "clouds",
            with(&[("look", Choice(2)), ("detail", Whole(10))]),
        ),
        ("cells", with(&[("look", Choice(0)), ("scale", Whole(3))])),
        (
            "cells",
            with(&[("look", Choice(2)), ("jitter", Number(1.0))]),
        ),
        ("cells", with(&[("look", Choice(3)), ("scale", Whole(7))])),
        (
            "gradient",
            with(&[("direction", Choice(2)), ("repeats", Whole(3))]),
        ),
        (
            "gradient",
            with(&[("style", Choice(2)), ("repeats", Whole(2))]),
        ),
        // A ramp has a sharp step every repeat, at the edge as inside.
        (
            "gradient",
            with(&[("style", Choice(1)), ("repeats", Whole(2))]),
        ),
        (
            "bricks",
            with(&[("rows", Whole(5)), ("shift", Number(0.3))]),
        ),
        ("tiles", with(&[("down", Whole(6)), ("shift", Number(0.5))])),
        ("planks", with(&[("rows", Whole(5)), ("across", Whole(3))])),
        (
            "basic_shape",
            with(&[
                ("shape", Choice(3)),
                ("size", Number(1.0)),
                ("turn", Number(17.0)),
            ]),
        ),
        (
            "basic_shape",
            with(&[
                ("shape", Choice(1)),
                ("repeats", Whole(3)),
                ("size", Number(1.0)),
            ]),
        ),
        ("warp", with(&[("amount", Number(30.0))])),
        (
            "transform",
            with(&[("across", Number(0.37)), ("turn", Choice(1))]),
        ),
        (
            "transform",
            with(&[("flip_across", Switch(true)), ("down", Number(-0.2))]),
        ),
        (
            "repeat",
            with(&[
                ("across", Whole(3)),
                ("down", Whole(4)),
                ("shift", Number(0.5)),
            ]),
        ),
        (
            "edges",
            with(&[("side", Choice(0)), ("width", Number(8.0))]),
        ),
        (
            "bevel",
            with(&[("edge", Choice(2)), ("width", Number(10.0))]),
        ),
        ("blur", with(&[("radius", Number(20.0))])),
        ("mix_materials", with(&[("by", Choice(1))])),
    ];
    for (kind, settings) in cases {
        let out = run(kind, size(64), &settings);
        assert_seamless(kind, &out);
    }
}

#[test]
fn the_same_seed_gives_the_same_pictures_on_any_number_of_cores() {
    for kind in every_box() {
        let s = Settings::new();
        let one = run_with(&kind, size(64), 11, 1, &s, 100.0);
        let again = run_with(&kind, size(64), 11, 3, &s, 100.0);
        assert_eq!(one, again, "{kind}");
    }
}

#[test]
fn another_seed_gives_another_pattern() {
    for kind in every_box() {
        let info = registry().get(&kind).unwrap().info();
        if !info.uses_seed() {
            continue;
        }
        // Repeat's seed only picks how dark each copy is.
        let s = if kind == "repeat" {
            with(&[("vary", SettingValue::Number(1.0))])
        } else {
            Settings::new()
        };
        // With only what it needs (Warp's own noise, not By).
        let a = run_inputs(&kind, size(32), 1, 1, &s, 100.0, false);
        let b = run_inputs(&kind, size(32), 2, 1, &s, 100.0, false);
        assert_ne!(a, b, "{kind}");
    }
}

/// The smallest and largest value of a setting, where it has a range.
fn extremes(spec: &SettingSpec) -> Vec<SettingValue> {
    match &spec.kind {
        SettingKind::Number { min, max, .. } | SettingKind::Length { min, max, .. } => {
            vec![SettingValue::Number(*min), SettingValue::Number(*max)]
        }
        SettingKind::Whole { min, max, .. } => {
            vec![SettingValue::Whole(*min), SettingValue::Whole(*max)]
        }
        SettingKind::Seed => vec![SettingValue::Whole(i32::MIN), SettingValue::Whole(i32::MAX)],
        SettingKind::Choice { options, .. } => (0..options.len() as u32)
            .map(SettingValue::Choice)
            .collect(),
        SettingKind::Switch { .. } => vec![SettingValue::Switch(true)],
        SettingKind::Colour { .. } => {
            vec![
                SettingValue::Colour([0.0; 3]),
                SettingValue::Colour([1.0; 3]),
            ]
        }
        SettingKind::Gradient { .. } => {
            let stop = |at, colour| GradientStop { at, colour };
            vec![SettingValue::Gradient(
                ColourGradient::new(&[stop(0.5, [1.0, 0.0, 0.0]), stop(0.5, [0.0; 3])]).unwrap(),
            )]
        }
    }
}

#[test]
fn extreme_settings_and_sizes_never_break_a_box() {
    for kind in every_box() {
        let info = registry().get(&kind).unwrap().info();
        for spec in &info.settings {
            for value in extremes(spec) {
                let s = with(&[(&spec.key, value)]);
                // The smallest texture, and the smallest and largest
                // real-world sizes.
                for cm in [1.0, 10_000.0] {
                    let out = run_with(&kind, size(16), 5, 2, &s, cm);
                    assert_in_range(&format!("{kind} {} {value:?} {cm}", spec.key), &out);
                }
            }
        }
        // Every setting at its smallest at once, then at its largest.
        for end in [0, 1] {
            let mut s = Settings::new();
            for spec in &info.settings {
                let ends = extremes(spec);
                s.set(spec.key.to_string(), ends[end.min(ends.len() - 1)]);
            }
            let out = run_with(&kind, size(32), 5, 2, &s, 100.0);
            assert_in_range(&format!("{kind} all at end {end}"), &out);
        }
    }
}

#[test]
fn flat_and_empty_inputs_never_break_a_box() {
    for kind in every_box() {
        let info = registry().get(&kind).unwrap().info();
        for grey in [0.0, 1.0] {
            let sz = size(16);
            let inputs = info
                .inputs
                .iter()
                .map(|p| match p.kind {
                    k if k.is_grey() => Value::flat_grey(k, sz, grey),
                    ValueKind::Colour => {
                        Some(Value::Colour(Arc::new(Image::filled(sz, [grey; 3]))))
                    }
                    _ => Some(input(ValueKind::Material, sz)),
                })
                .collect();
            let job = Job::new(info, sz, 1, 1, &Settings::new(), inputs);
            let out = registry().get(&kind).unwrap().run(&job).unwrap();
            assert_in_range(&format!("{kind} flat {grey}"), &out);
        }
    }
}

fn grey_of(values: &[Value], at: usize) -> Arc<Grey> {
    values[at].grey().unwrap().clone()
}

#[test]
fn bevel_rises_from_the_outline_to_the_top() {
    use SettingValue::{Choice, Number};
    // One square in a 64-pixel texture, 100 cm across: 2 cm is 1.28 pixels.
    let square = with(&[("shape", Choice(1)), ("size", Number(0.5))]);
    let shape = run("basic_shape", size(64), &square).remove(0);
    let bevel = |pairs: &[(&str, SettingValue)]| {
        let info = registry().get("bevel").unwrap().info();
        let job = Job::new(
            info,
            size(64),
            0,
            1,
            &with(pairs),
            vec![Some(shape.clone())],
        );
        grey_of(&registry().get("bevel").unwrap().run(&job).unwrap(), 0)
    };
    let sharp = bevel(&[
        ("width", Number(12.5)),
        ("edge", Choice(1)),
        ("roundness", Number(0.0)),
    ]);
    // 12.5 cm is 8 pixels: rising over the 8 pixels in from the edge.
    assert_eq!(sharp.at(2, 2), 0.0);
    let row: Vec<f32> = (16..32).map(|x| sharp.at(x, 32)).collect();
    assert!(row.windows(2).all(|w| w[0] <= w[1] + 1e-6), "{row:?}");
    assert!(sharp.at(17, 32) < 0.3);
    assert_eq!(sharp.at(32, 32), 1.0);
    // Round rises faster at first than a straight slope.
    let round = bevel(&[
        ("width", Number(12.5)),
        ("edge", Choice(0)),
        ("roundness", Number(0.0)),
    ]);
    assert!(round.at(18, 32) > sharp.at(18, 32));
    // Steps hold flat between steps.
    let steps = bevel(&[
        ("width", Number(12.5)),
        ("edge", Choice(2)),
        ("roundness", Number(0.0)),
    ]);
    let levels: std::collections::BTreeSet<u32> = (16..32)
        .map(|x| (steps.at(x, 32) * 1000.0) as u32)
        .collect();
    assert!(levels.len() <= 4, "{levels:?}");
    // Height scales the top.
    let low = bevel(&[("height", Number(0.5)), ("roundness", Number(0.0))]);
    assert!((low.at(32, 32) - 0.5).abs() < 1e-5);
}

#[test]
fn lengths_follow_the_real_world_size() {
    use SettingValue::Number;
    // A bevel 5 cm wide on a texture covering 50 cm reaches twice as far
    // (in pixels) as on one covering 100 cm.
    let square = with(&[("shape", SettingValue::Choice(1)), ("size", Number(0.5))]);
    let shape = run("basic_shape", size(64), &square).remove(0);
    let s = with(&[("width", Number(5.0)), ("roundness", Number(0.0))]);
    let at = |cm| {
        let info = registry().get("bevel").unwrap().info();
        let job = Job::new(info, size(64), 0, 1, &s, vec![Some(shape.clone())]).with_real_size(cm);
        grey_of(&registry().get("bevel").unwrap().run(&job).unwrap(), 0)
    };
    let (near, far) = (at(50.0), at(100.0));
    let rising = |g: &Grey| {
        (0..64)
            .filter(|x| (0.01..0.99).contains(&g.at(*x, 31)))
            .count()
    };
    assert!(
        rising(&near) > rising(&far),
        "{} {}",
        rising(&near),
        rising(&far)
    );
}

#[test]
fn changing_the_real_world_size_remakes_only_boxes_with_lengths() {
    let mut g = Graph::new("Sizes");
    let add = |g: &mut Graph, kind: &str| {
        let id = BoxId(g.next_id());
        g.apply(
            registry(),
            &GraphEdit::AddBox {
                kind: kind.into(),
                at: Vec2::ZERO,
            },
        )
        .unwrap();
        id
    };
    let (tiles, bevel, out) = (
        add(&mut g, "tiles"),
        add(&mut g, "bevel"),
        add(&mut g, "material_out"),
    );
    g.apply(
        registry(),
        &GraphEdit::Connect {
            from: PlugRef::new(tiles, "shape"),
            to: PlugRef::new(bevel, "shape"),
        },
    )
    .unwrap();
    g.apply(
        registry(),
        &GraphEdit::SetSetting {
            node: bevel,
            key: "width".into(),
            value: SettingValue::Number(10.0),
        },
    )
    .unwrap();
    let mut e = Evaluator::with_threads(1);
    // At 128 pixels the tiles' gaps are wider than a pixel.
    let before = e.run(registry(), &g, size(128), &[tiles, bevel]);
    let picture = before.picture(bevel).cloned();
    g.apply(
        registry(),
        &GraphEdit::SetSetting {
            node: out,
            key: "size_cm".into(),
            value: SettingValue::Number(400.0),
        },
    )
    .unwrap();
    assert_eq!(crate::real_size_cm(registry(), &g), 400.0);
    let after = e.run(registry(), &g, size(128), &[tiles, bevel]);
    assert_eq!(after.reused(), 1, "Tiles is found again");
    assert_eq!(after.made(), 1, "Bevel is made again");
    assert_ne!(after.picture(bevel).cloned(), picture);
    // A graph with no Material out covers the standard size.
    assert_eq!(
        crate::real_size_cm(registry(), &Graph::new("None")),
        crate::STANDARD_REAL_SIZE_CM
    );
}

#[test]
fn edge_finder_draws_lines_on_the_outline() {
    use SettingValue::{Choice, Number};
    let square = with(&[("shape", Choice(1)), ("size", Number(0.5))]);
    let shape = run("basic_shape", size(64), &square).remove(0);
    let edges = |side: u32| {
        let info = registry().get("edges").unwrap().info();
        let s = with(&[("side", Choice(side)), ("width", Number(6.25))]);
        let job = Job::new(info, size(64), 0, 1, &s, vec![Some(shape.clone())]);
        grey_of(&registry().get("edges").unwrap().run(&job).unwrap(), 0)
    };
    let (both, inside, outside) = (edges(0), edges(1), edges(2));
    // The square runs from pixel 16 to 47; 6.25 cm is 4 pixels.
    assert!(inside.at(16, 32) > 0.8 && inside.at(15, 32) == 0.0);
    assert!(outside.at(15, 32) > 0.8 && outside.at(16, 32) == 0.0);
    assert!(both.at(16, 32) > 0.8 && both.at(15, 32) > 0.8);
    assert_eq!(both.at(32, 32), 0.0);
    assert_eq!(both.at(2, 2), 0.0);
}

#[test]
fn basic_shapes_sit_in_the_middle_of_each_repeat() {
    use SettingValue::{Choice, Number, Whole};
    for shape in 0..4 {
        let out = grey_of(
            &run("basic_shape", size(64), &with(&[("shape", Choice(shape))])),
            0,
        );
        assert_eq!(out.at(32, 32), 1.0, "{shape}");
        assert_eq!(out.at(0, 0), 0.0, "{shape}");
    }
    let grid = grey_of(
        &run(
            "basic_shape",
            size(64),
            &with(&[("repeats", Whole(4)), ("size", Number(0.5))]),
        ),
        0,
    );
    assert_eq!(grid.at(8, 8), 1.0);
    assert_eq!(grid.at(8 + 16 * 3, 8 + 16), 1.0);
    assert_eq!(grid.at(0, 0), 0.0);
    let nothing = grey_of(
        &run("basic_shape", size(32), &with(&[("size", Number(0.0))])),
        0,
    );
    assert!(nothing.pixels().iter().all(|v| *v == 0.0));
}

#[test]
fn bricks_tiles_and_planks_have_gaps_and_a_grey_per_piece() {
    for kind in ["bricks", "tiles", "planks"] {
        let out = run(kind, size(64), &with(&[("gap", SettingValue::Number(0.3))]));
        let (shape, random) = (grey_of(&out, 0), grey_of(&out, 1));
        // Gaps are black in both; pieces are white in the shape.
        assert_eq!(shape.at(0, 0), 0.0, "{kind}");
        assert_eq!(random.at(0, 0), 0.0, "{kind}");
        assert!(shape.pixels().contains(&1.0), "{kind}");
        // Pieces differ from each other in Random.
        let greys: std::collections::BTreeSet<u32> = random
            .pixels()
            .iter()
            .zip(shape.pixels())
            .filter(|(_, s)| **s == 1.0)
            .map(|(r, _)| (r * 1000.0) as u32)
            .collect();
        assert!(greys.len() > 3, "{kind}: {greys:?}");
    }
    // Bricks: an odd number of rows is made even.
    let five = run(
        "bricks",
        size(64),
        &with(&[("rows", SettingValue::Whole(5))]),
    );
    let six = run(
        "bricks",
        size(64),
        &with(&[("rows", SettingValue::Whole(6))]),
    );
    assert_eq!(five[0], six[0]);
}

#[test]
fn transform_turns_and_moves_without_losing_anything() {
    use SettingValue::{Choice, Number, Switch};
    let original = input(ValueKind::Height, size(32));
    let once = |s: &Settings, v: &Value| {
        let info = registry().get("transform").unwrap().info();
        let job = Job::new(info, size(32), 0, 1, s, vec![Some(v.clone())]);
        registry()
            .get("transform")
            .unwrap()
            .run(&job)
            .unwrap()
            .remove(0)
    };
    let quarter = with(&[("turn", Choice(1))]);
    let mut turned = original.clone();
    for _ in 0..4 {
        turned = once(&quarter, &turned);
    }
    assert_eq!(turned, original);
    // A quarter turn clockwise: the top left goes to the top right.
    let q = once(&quarter, &original);
    let (o, q) = (original.grey().unwrap(), q.grey().unwrap());
    assert!((q.at(31, 0) - o.at(0, 0)).abs() < 1e-6);
    // Flipped twice is the same; moved a whole texture is the same.
    let flip = with(&[("flip_across", Switch(true)), ("flip_down", Switch(true))]);
    assert_eq!(once(&flip, &once(&flip, &original)), original);
    let moved = once(&with(&[("across", Number(1.0))]), &original);
    for (a, b) in moved.grey().unwrap().pixels().iter().zip(o.pixels()) {
        assert!((a - b).abs() < 1e-5);
    }
    // Moved by a quarter: what was at the left is a quarter along.
    let quarter_along = once(&with(&[("across", Number(0.25))]), &original);
    assert!((quarter_along.grey().unwrap().at(8, 5) - o.at(0, 5)).abs() < 1e-5);
}

#[test]
fn repeat_once_gives_the_input_back_and_more_makes_copies() {
    use SettingValue::{Number, Whole};
    let original = input(ValueKind::Height, size(32));
    let o = original.grey().unwrap();
    let once = |s: &Settings| {
        let info = registry().get("repeat").unwrap().info();
        let job = Job::new(info, size(32), 0, 1, s, vec![Some(original.clone())]);
        grey_of(&registry().get("repeat").unwrap().run(&job).unwrap(), 0)
    };
    let one = once(&with(&[("across", Whole(1)), ("down", Whole(1))]));
    for (a, b) in one.pixels().iter().zip(o.pixels()) {
        assert!((a - b).abs() < 1e-5);
    }
    let two = once(&with(&[("across", Whole(2)), ("down", Whole(2))]));
    assert!((two.at(3, 4) - two.at(19, 20)).abs() < 1e-6);
    let varied = once(&with(&[("vary", Number(1.0))]));
    assert!(
        varied
            .pixels()
            .iter()
            .zip(two.pixels())
            .all(|(v, t)| v <= t)
    );
}

#[test]
fn blur_and_warp_at_nothing_give_the_input_back() {
    use SettingValue::Number;
    for (kind, key) in [("blur", "radius"), ("warp", "amount")] {
        let s = with(&[(key, Number(0.0))]);
        let out = run(kind, size(32), &s);
        let original = input(ValueKind::Height, size(32));
        for (a, b) in grey_of(&out, 0)
            .pixels()
            .iter()
            .zip(original.grey().unwrap().pixels())
        {
            assert!((a - b).abs() < 1e-5, "{kind}");
        }
    }
    // A blur keeps the average grey.
    let original = input(ValueKind::Height, size(32));
    let blurred = run("blur", size(32), &with(&[("radius", Number(10.0))]));
    let mean = |g: &Grey| g.pixels().iter().sum::<f32>() / g.pixels().len() as f32;
    assert!((mean(&grey_of(&blurred, 0)) - mean(original.grey().unwrap())).abs() < 1e-3);
}

#[test]
fn gradient_map_colours_greys_along_the_gradient() {
    let stop = |at, colour| GradientStop { at, colour };
    let red_to_blue =
        ColourGradient::new(&[stop(0.0, [1.0, 0.0, 0.0]), stop(1.0, [0.0, 0.0, 1.0])]).unwrap();
    let info = registry().get("gradient_map").unwrap().info();
    let s = with(&[("colours", SettingValue::Gradient(red_to_blue))]);
    for (grey, want) in [
        (0.0, [1.0, 0.0, 0.0]),
        (1.0, [0.0, 0.0, 1.0]),
        (0.5, [0.5, 0.0, 0.5]),
    ] {
        let flat = Value::flat_grey(ValueKind::Height, size(16), grey);
        let job = Job::new(info, size(16), 0, 1, &s, vec![flat]);
        let out = registry().get("gradient_map").unwrap().run(&job).unwrap();
        let got = out[0].colour().unwrap().at(3, 3);
        for i in 0..3 {
            assert!((got[i] - want[i]).abs() < 2e-3, "{grey}: {got:?}");
        }
    }
}

#[test]
fn hue_and_saturation_change_colours_only_where_asked() {
    use SettingValue::Number;
    let alone = |s: &Settings| run_inputs("hue_saturation", size(16), 0, 1, s, 100.0, false);
    let grey_out = alone(&with(&[("saturation", Number(0.0))]));
    for c in grey_out[0].colour().unwrap().pixels() {
        assert!((c[0] - c[1]).abs() < 1e-5 && (c[1] - c[2]).abs() < 1e-5);
    }
    let same = alone(&Settings::new());
    let original = input(ValueKind::Colour, size(16));
    for (a, b) in same[0]
        .colour()
        .unwrap()
        .pixels()
        .iter()
        .zip(original.colour().unwrap().pixels())
    {
        for i in 0..3 {
            assert!((a[i] - b[i]).abs() < 1e-4);
        }
    }
    let black = alone(&with(&[("lightness", Number(-1.0))]));
    let circle = run("basic_shape", size(16), &Settings::new()).remove(0);
    // Through a shape: outside it (black) keeps its colour.
    let info = registry().get("hue_saturation").unwrap().info();
    let job = Job::new(
        info,
        size(16),
        0,
        1,
        &with(&[("lightness", Number(-1.0))]),
        vec![Some(original.clone()), Some(circle)],
    );
    let masked = registry().get("hue_saturation").unwrap().run(&job).unwrap();
    assert_eq!(black[0].colour().unwrap().at(5, 5), [0.0; 3]);
    assert_eq!(
        masked[0].colour().unwrap().at(0, 0),
        original.colour().unwrap().at(0, 0)
    );
}

#[test]
fn colour_to_shape_picks_light_colours_or_one_colour() {
    use SettingValue::{Choice, Colour, Number};
    let info = registry().get("colour_to_shape").unwrap().info();
    let on = |c: [f32; 3], s: &Settings| {
        let colour = Value::Colour(Arc::new(Image::filled(size(16), c)));
        let job = Job::new(info, size(16), 0, 1, s, vec![Some(colour)]);
        grey_of(
            &registry()
                .get("colour_to_shape")
                .unwrap()
                .run(&job)
                .unwrap(),
            0,
        )
        .at(0, 0)
    };
    let light = Settings::new();
    assert_eq!(on([1.0; 3], &light), 1.0);
    assert_eq!(on([0.0; 3], &light), 0.0);
    let green = with(&[
        ("by", Choice(1)),
        ("colour", Colour([0.0, 1.0, 0.0])),
        ("threshold", Number(0.2)),
    ]);
    assert_eq!(on([0.05, 0.95, 0.0], &green), 1.0);
    assert_eq!(on([1.0, 0.0, 0.0], &green), 0.0);
}

#[test]
fn colour_to_shape_is_offered_between_a_colour_and_a_shape_plug() {
    let mut g = Graph::new("Fix");
    for kind in ["colour_fill", "bevel"] {
        g.apply(
            registry(),
            &GraphEdit::AddBox {
                kind: kind.into(),
                at: Vec2::ZERO,
            },
        )
        .unwrap();
    }
    let refusal = g
        .check_connection(
            registry(),
            &PlugRef::new(BoxId(1), "colour"),
            &PlugRef::new(BoxId(2), "shape"),
        )
        .unwrap_err();
    assert!(
        refusal.reason.contains("inside or outside"),
        "{}",
        refusal.reason
    );
    assert!(refusal.fixes.contains(&crate::Fix::InsertBox {
        kind: "colour_to_shape".into()
    }));
}

#[test]
fn mix_materials_shows_top_where_asked() {
    use SettingValue::{Choice, Number};
    let sz = size(16);
    let maps = |grey: f32, colour: [f32; 3]| {
        let g = Arc::new(Image::filled(sz, grey));
        Value::Material(Arc::new(MaterialMaps {
            colour: Arc::new(Image::filled(sz, colour)),
            height: g.clone(),
            roughness: g.clone(),
            metal: g.clone(),
            normal: Arc::new(Image::filled(sz, [0.0, 0.0, 1.0])),
            occlusion: g.clone(),
            curvature: g,
        }))
    };
    let (low, high) = (maps(0.2, [1.0, 0.0, 0.0]), maps(0.8, [0.0, 0.0, 1.0]));
    let info = registry().get("mix_materials").unwrap().info();
    let mix = |s: &Settings, bottom: &Value, top: &Value| {
        let job = Job::new(
            info,
            sz,
            0,
            1,
            s,
            vec![Some(bottom.clone()), Some(top.clone()), None],
        );
        let out = registry().get("mix_materials").unwrap().run(&job).unwrap();
        out[0].material().unwrap().colour.at(0, 0)
    };
    let half = with(&[("amount", Number(0.5))]);
    assert_eq!(mix(&half, &low, &high), [0.5, 0.0, 0.5]);
    let by_height = with(&[("by", Choice(1))]);
    // Top shows where it stands higher, not where it is lower.
    assert_eq!(mix(&by_height, &low, &high), [0.0, 0.0, 1.0]);
    assert_eq!(mix(&by_height, &high, &low), [0.0, 0.0, 1.0]);
    let sunk = with(&[("by", Choice(1)), ("rise", Number(1.0))]);
    assert_eq!(mix(&sunk, &high, &low), [1.0, 0.0, 0.0]);
}

#[test]
fn cells_and_clouds_have_their_looks() {
    use SettingValue::Choice;
    // Flat cells hold one grey across each cell: few greys in all.
    let flat = grey_of(&run("cells", size(64), &with(&[("look", Choice(3))])), 0);
    let greys: std::collections::BTreeSet<u32> =
        flat.pixels().iter().map(|v| v.to_bits()).collect();
    assert!(greys.len() <= 36, "{}", greys.len());
    // Every look of Clouds differs from the others.
    let looks: Vec<_> = (0..3)
        .map(|l| run("clouds", size(32), &with(&[("look", Choice(l))])))
        .collect();
    assert_ne!(looks[0], looks[1]);
    assert_ne!(looks[1], looks[2]);
}

/// The stone-tiles graph a starter graph will be built on: Tiles, Warp,
/// Bevel, Noise blended on top, a Gradient map for colour, Material from
/// height, and Mix materials laying moss in the gaps by height, into
/// Material out.
fn stone_tiles() -> (Graph, BoxId) {
    let mut g = Graph::new("Stone tiles");
    let mut add = |kind: &str| {
        let id = BoxId(g.next_id());
        g.apply(
            registry(),
            &GraphEdit::AddBox {
                kind: kind.into(),
                at: Vec2::ZERO,
            },
        )
        .unwrap();
        id
    };
    let tiles = add("tiles");
    let warp = add("warp");
    let bevel = add("bevel");
    let noise = add("noise");
    let blend = add("blend");
    let colours = add("gradient_map");
    let stone = add("material_from_height");
    let moss_colour = add("colour_fill");
    let clouds = add("clouds");
    let moss = add("material_from_height");
    let mix = add("mix_materials");
    let out = add("material_out");
    for (from, to) in [
        ((tiles, "shape"), (warp, "in")),
        ((warp, "out"), (bevel, "shape")),
        ((bevel, "height"), (blend, "bottom")),
        ((noise, "height"), (blend, "top")),
        ((tiles, "shape"), (blend, "where")),
        ((blend, "out"), (colours, "in")),
        ((blend, "out"), (stone, "height")),
        ((colours, "colour"), (stone, "colour")),
        ((clouds, "height"), (moss, "height")),
        ((moss_colour, "colour"), (moss, "colour")),
        ((stone, "material"), (mix, "bottom")),
        ((moss, "material"), (mix, "top")),
        ((mix, "material"), (out, "material")),
    ] {
        g.apply(
            registry(),
            &GraphEdit::Connect {
                from: PlugRef::new(from.0, from.1),
                to: PlugRef::new(to.0, to.1),
            },
        )
        .unwrap();
    }
    for (node, key, value) in [
        (blend, "mode", SettingValue::Choice(1)),
        (blend, "amount", SettingValue::Number(0.25)),
        (
            moss_colour,
            "colour",
            SettingValue::Colour([0.25, 0.4, 0.15]),
        ),
        (mix, "by", SettingValue::Choice(1)),
        (mix, "rise", SettingValue::Number(-0.35)),
        (out, "size_cm", SettingValue::Number(200.0)),
    ] {
        g.apply(
            registry(),
            &GraphEdit::SetSetting {
                node,
                key: key.into(),
                value,
            },
        )
        .unwrap();
    }
    (g, out)
}

#[test]
fn a_stone_tiles_graph_makes_a_seamless_material_end_to_end() {
    let (g, _) = stone_tiles();
    let one = Evaluator::with_threads(1)
        .material(registry(), &g, size(128))
        .unwrap();
    let again = Evaluator::with_threads(3)
        .material(registry(), &g, size(128))
        .unwrap();
    assert_eq!(*one, *again);
    let value = Value::Material(one.clone());
    assert_in_range("stone tiles", std::slice::from_ref(&value));
    assert_seamless("stone tiles", &[value]);
    // Moss shows in the low gaps, stone on the tiles.
    let colour = |x, y| one.colour.at(x, y);
    let gap = colour(0, 0);
    assert!(gap[1] > gap[0] && gap[1] > gap[2], "{gap:?}");
    // Another graph seed makes other stones.
    let mut other = g.clone();
    other.apply(registry(), &GraphEdit::SetSeed(99)).unwrap();
    let reseeded = Evaluator::with_threads(1)
        .material(registry(), &other, size(128))
        .unwrap();
    assert_ne!(*one, *reseeded);
}

/// Times the heavier boxes at 2048 pixels: run by hand with
/// `cargo test -p matgraph --release -- --ignored box_speed --nocapture`.
#[test]
#[ignore = "speed check: run by hand"]
fn box_speed() {
    let threads = crate::parallel::cores();
    let sz = size(2048);
    for kind in [
        "noise",
        "clouds",
        "cells",
        "bricks",
        "planks",
        "basic_shape",
        "blur",
        "warp",
        "edges",
        "bevel",
        "repeat",
        "gradient_map",
        "hue_saturation",
        "mix_materials",
    ] {
        let info = registry().get(kind).unwrap().info();
        let inputs = info
            .inputs
            .iter()
            .map(|p| Some(input(p.kind, sz)))
            .collect();
        let job = Job::new(info, sz, 1, threads, &Settings::new(), inputs);
        let start = std::time::Instant::now();
        registry().get(kind).unwrap().run(&job).unwrap();
        let took = start.elapsed();
        #[allow(clippy::print_stdout)]
        {
            println!("{kind:<16} {:>8.1} ms at 2048", took.as_secs_f64() * 1000.0);
        }
        // Generous: a slow, busy machine passes; a box gone wrong doesn't.
        assert!(took.as_secs() < 60, "{kind} took {took:?}");
    }
    let (g, _) = stone_tiles();
    let start = std::time::Instant::now();
    Evaluator::new().material(registry(), &g, sz).unwrap();
    #[allow(clippy::print_stdout)]
    {
        println!(
            "stone tiles graph {:>8.1} ms at 2048",
            start.elapsed().as_secs_f64() * 1000.0
        );
    }
}
