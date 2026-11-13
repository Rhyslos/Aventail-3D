//! Milestone 25's weathering set (decision 0288): Dirt, Rust, Edge wear,
//! Dust and Water streaks. The checks every box gets (in range, seamless
//! with standard settings, seeds and cores, extreme settings, flat
//! inputs) are in `tests_each.rs`; here each box is checked for what it
//! says it does, on the stand-in, on a model's maps and on the material's
//! own bumps, and tiled with other settings. The speed check and the
//! pictures are ignored: `cargo test -p matgraph --release -- --ignored
//! weathering_speed --nocapture`, and `WEATHER_RENDERS=<folder> cargo test
//! -p matgraph --release -- --ignored weathering_renders`.

use crate::boxes::tests_each::{assert_in_range, assert_seamless, input, registry, size, with};
use crate::image::{Grey, Image, Rgb};
use crate::model::{ModelMap, ModelMaps, OnModel, maps_needed, uses_model};
use crate::registry::Job;
use crate::settings::{SettingValue, Settings};
use crate::value::{MaterialMaps, Value};
use crate::{BakeNote, Evaluator, Graph, GraphEdit, PlugRef, ValueKind, bake_notes};
use glam::Vec2;
use std::collections::BTreeSet;
use std::sync::Arc;

use SettingValue::{Choice, Colour, Number, Switch};

/// The new boxes' ids.
const NEW: [&str; 5] = ["dirt", "rust", "edge_wear", "dust", "water_streaks"];

/// A flat, plain material at `n` pixels: mid grey, half rough, metal by
/// `metal`.
fn flat(n: u32, metal: f32) -> Value {
    let s = size(n);
    let grey = |v| Arc::new(Image::filled(s, v));
    Value::Material(Arc::new(MaterialMaps {
        colour: Arc::new(Image::filled(s, [0.5, 0.5, 0.5])),
        height: grey(0.5),
        roughness: grey(0.3),
        metal: grey(metal),
        normal: Arc::new(Image::filled(s, [0.0, 0.0, 1.0])),
        occlusion: grey(1.0),
        curvature: grey(0.5),
        opacity: None,
        translucency: None,
    }))
}

fn job_inputs(kind: &str, inputs: &[(&str, Value)]) -> Vec<Option<Value>> {
    let info = registry().get(kind).unwrap().info();
    info.inputs
        .iter()
        .map(|p| {
            inputs
                .iter()
                .find(|(k, _)| *k == p.key)
                .map(|(_, v)| v.clone())
        })
        .collect()
}

/// Runs box `kind` at `n` pixels with `inputs` by key, on the model's
/// `maps` if any.
fn run_on(
    kind: &str,
    n: u32,
    s: &Settings,
    inputs: &[(&str, Value)],
    maps: Option<Arc<ModelMaps>>,
) -> (Arc<MaterialMaps>, Arc<Grey>) {
    let info = registry().get(kind).unwrap().info();
    let job = Job::new(info, size(n), 7, 2, s, job_inputs(kind, inputs))
        .with_real_size(100.0)
        .with_model(maps);
    let out = registry().get(kind).unwrap().run(&job).unwrap();
    assert_in_range(kind, &out);
    (
        out[0].material().unwrap().clone(),
        out[1].grey().unwrap().clone(),
    )
}

fn run(
    kind: &str,
    n: u32,
    s: &Settings,
    inputs: &[(&str, Value)],
) -> (Arc<MaterialMaps>, Arc<Grey>) {
    run_on(kind, n, s, inputs, None)
}

/// A model's maps at `n` pixels, each made by `f(map, u, v)`.
fn model(n: u32, f: impl Fn(ModelMap, f32, f32) -> f32) -> Arc<ModelMaps> {
    let mut maps = ModelMaps::new(size(n), 5);
    for map in ModelMap::ALL {
        let pixels = (0..n * n)
            .map(|i| {
                let (u, v) = ((i % n) as f32 + 0.5, (i / n) as f32 + 0.5);
                f(map, u / n as f32, v / n as f32)
            })
            .collect();
        assert!(maps.insert(map, Image::from_pixels(size(n), pixels).unwrap()));
    }
    Arc::new(maps)
}

fn mean(g: &Grey, xs: std::ops::Range<i64>, ys: std::ops::Range<i64>) -> f32 {
    let mut total = 0.0;
    let mut count = 0.0;
    for y in ys {
        for x in xs.clone() {
            total += g.at(x, y);
            count += 1.0;
        }
    }
    total / count
}

/// Only the model's shape counts (not the material's own).
fn model_only(more: &[(&str, SettingValue)]) -> Settings {
    let mut s = with(&[
        ("own", Number(0.0)),
        ("patchy", Number(0.0)),
        ("film", Number(0.0)),
    ]);
    for (k, v) in more {
        s.set(*k, v.clone());
    }
    s
}

#[test]
fn every_weathering_box_reads_the_models_maps_and_is_listed_together() {
    for kind in NEW {
        let t = registry().get(kind).unwrap();
        assert_eq!(t.info().group, crate::Group::Weather, "{kind}");
        assert!(!t.model_maps().is_empty(), "{kind}");
        assert!(crate::model::is_from_model(registry(), kind), "{kind}");
    }
    // Other boxes don't.
    for kind in ["noise", "fill_gaps", "edge_highlight"] {
        assert!(!crate::model::is_from_model(registry(), kind));
    }
    // The From the model boxes still do, through the same question.
    assert!(crate::model::is_from_model(registry(), "model_curvature"));
}

#[test]
fn with_nothing_to_weather_the_material_comes_back_as_it_was() {
    let m = flat(32, 0.0);
    let none = |kind: &str| {
        let mut s = with(&[("amount", Number(0.0)), ("film", Number(0.0))]);
        if kind == "rust" {
            s.set("flaking", Number(0.0));
        }
        if kind == "edge_wear" {
            s.set("depth", Number(0.0));
        }
        s
    };
    for kind in NEW {
        let (out, cover) = run(kind, 32, &none(kind), &[("material", m.clone())]);
        assert!(cover.pixels().iter().all(|v| *v == 0.0), "{kind}");
        assert_eq!(Value::Material(out), m, "{kind}");
    }
}

#[test]
fn dirt_gathers_in_the_stand_ins_creases_not_on_its_tops() {
    // The stand-in's bumps top out at the texture's corner, with creases
    // an eighth of the way across.
    let (out, cover) = run("dirt", 64, &model_only(&[]), &[("material", flat(64, 0.0))]);
    let crease = mean(&cover, 7..9, 7..9);
    let top = mean(&cover, 0..1, 0..1);
    assert!(crease > 0.9 && top < 0.05, "crease {crease} top {top}");
    // Darker and rougher there; the height untouched.
    assert!(out.colour.at(8, 8)[0] < 0.3);
    assert!(out.roughness.at(8, 8) > 0.8);
    assert_eq!(out.height.at(8, 8), 0.5);
}

#[test]
fn dirt_follows_the_models_own_crevices_in_the_painter() {
    // A crease down the middle of the model, open elsewhere.
    let maps = model(32, |map, u, _| match map {
        ModelMap::Curvature if (u - 0.5).abs() < 0.08 => 0.0,
        m => m
            .neutral()
            .max(if m == ModelMap::Occlusion { 1.0 } else { 0.0 }),
    });
    let s = model_only(&[]);
    let (_, cover) = run_on(
        "dirt",
        32,
        &s,
        &[("material", flat(32, 0.0))],
        Some(maps.clone()),
    );
    assert!(mean(&cover, 15..17, 0..32) > 0.6);
    assert!(mean(&cover, 2..6, 0..32) < 0.05);
    // At Follow the model 0, nothing (the material is flat).
    let s = model_only(&[("model", Number(0.0))]);
    let (_, cover) = run_on("dirt", 32, &s, &[("material", flat(32, 0.0))], Some(maps));
    assert!(cover.pixels().iter().all(|v| *v < 1e-4));
}

#[test]
fn dirt_follows_the_materials_own_gaps_too() {
    // Tiles made into a material: the gaps are low, enclosed creases.
    let n = 128;
    let tiles = input(ValueKind::Shape, size(n));
    let info = registry().get("material_from_height").unwrap().info();
    let job = Job::new(
        info,
        size(n),
        1,
        1,
        &Settings::new(),
        vec![Some(tiles.clone()), None],
    );
    let m = registry()
        .get("material_from_height")
        .unwrap()
        .run(&job)
        .unwrap()
        .remove(0);
    let s = model_only(&[("model", Number(0.0)), ("own", Number(1.0))]);
    let (_, cover) = run("dirt", n, &s, &[("material", m)]);
    let shape = tiles.grey().unwrap();
    let (mut gaps, mut tops) = ((0.0, 0.0), (0.0, 0.0));
    for (v, c) in shape.pixels().iter().zip(cover.pixels()) {
        if *v < 0.5 {
            gaps = (gaps.0 + c, gaps.1 + 1.0);
        } else {
            tops = (tops.0 + c, tops.1 + 1.0);
        }
    }
    let (gaps, tops) = (gaps.0 / gaps.1, tops.0 / tops.1);
    assert!(gaps > tops + 0.2, "gaps {gaps} tops {tops}");
}

#[test]
fn only_where_keeps_the_weathering_to_a_shape() {
    let n = 32;
    let half = Value::Shape(Arc::new(
        Image::from_pixels(
            size(n),
            (0..n * n)
                .map(|i| if i % n < n / 2 { 1.0 } else { 0.0 })
                .collect(),
        )
        .unwrap(),
    ));
    for kind in NEW {
        let s = with(&[("amount", Number(1.0))]);
        let (_, cover) = run(
            kind,
            n,
            &s,
            &[("material", flat(n, 1.0)), ("where", half.clone())],
        );
        assert!(mean(&cover, 20..32, 0..32) == 0.0, "{kind}");
    }
}

#[test]
fn rust_turns_metal_rough_and_raises_flakes() {
    let s = with(&[("amount", Number(0.7))]);
    let (out, cover) = run("rust", 64, &s, &[("material", flat(64, 1.0))]);
    let (mut covered, mut bare) = (Vec::new(), Vec::new());
    for (i, c) in cover.pixels().iter().enumerate() {
        if *c > 0.95 {
            covered.push(i);
        } else if *c == 0.0 {
            bare.push(i);
        }
    }
    assert!(!covered.is_empty() && !bare.is_empty());
    for i in covered {
        assert!(out.metal.pixels()[i] < 0.05);
        assert!(out.roughness.pixels()[i] > 0.85);
        assert!(out.height.pixels()[i] > 0.5);
        let [r, g, b] = out.colour.pixels()[i];
        assert!(r > g && g > b, "rust is orange-brown");
    }
    for i in bare {
        assert_eq!(out.metal.pixels()[i], 1.0);
    }
}

#[test]
fn rust_only_on_metal_leaves_painted_parts_alone() {
    let s = with(&[("amount", Number(1.0)), ("metal_only", Switch(true))]);
    let (_, cover) = run("rust", 32, &s, &[("material", flat(32, 0.0))]);
    assert!(cover.pixels().iter().all(|v| *v == 0.0));
    let (_, cover) = run("rust", 32, &s, &[("material", flat(32, 1.0))]);
    assert!(mean(&cover, 0..32, 0..32) > 0.5);
}

#[test]
fn rust_keeps_to_edges_more_with_starts_at_edges() {
    let edges = |at_edges: f32| {
        let s = model_only(&[("at_edges", Number(at_edges)), ("amount", Number(0.3))]);
        let (_, cover) = run("rust", 64, &s, &[("material", flat(64, 1.0))]);
        // The creases between the stand-in's bumps, against their tops.
        mean(&cover, 7..9, 0..64) - mean(&cover, 0..1, 0..64)
    };
    assert!(
        edges(1.0) > edges(0.0) + 0.2,
        "{} {}",
        edges(1.0),
        edges(0.0)
    );
}

#[test]
fn edge_wear_shows_bare_metal_on_edges_and_sinks_a_little() {
    // A ridge down the middle of the model: an outer edge.
    let maps = model(32, |map, u, _| match map {
        ModelMap::Curvature if (u - 0.5).abs() < 0.1 => 1.0,
        m => m.neutral(),
    });
    let s = model_only(&[("breakup", Number(0.0)), ("amount", Number(0.5))]);
    let (out, cover) = run_on(
        "edge_wear",
        32,
        &s,
        &[("material", flat(32, 0.0))],
        Some(maps),
    );
    assert!(mean(&cover, 15..17, 0..32) > 0.95);
    assert!(mean(&cover, 2..8, 0..32) < 0.01);
    assert!(out.metal.at(16, 4) > 0.95);
    assert!(out.roughness.at(16, 4) < 0.4);
    assert!(out.height.at(16, 4) < 0.5);
    assert_eq!(out.colour.at(16, 4), [0.56, 0.56, 0.58]);
    assert_eq!(out.colour.at(4, 4), [0.5, 0.5, 0.5]);
}

#[test]
fn edge_wear_shows_the_material_plugged_into_under() {
    let wood = {
        let s = size(32);
        Value::Material(Arc::new(MaterialMaps {
            colour: Arc::new(Image::filled(s, [0.4, 0.25, 0.1])),
            roughness: Arc::new(Image::filled(s, 0.7)),
            metal: Arc::new(Image::filled(s, 0.0)),
            ..(**flat(32, 0.0).material().unwrap()).clone()
        }))
    };
    let s = model_only(&[("breakup", Number(0.0)), ("amount", Number(0.9))]);
    let (out, cover) = run(
        "edge_wear",
        32,
        &s,
        &[("material", flat(32, 1.0)), ("under", wood)],
    );
    // The stand-in's shoulders are its edges.
    let (i, c) = cover
        .pixels()
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .unwrap();
    assert!(*c > 0.99);
    let c = out.colour.pixels()[i];
    assert!((c[0] - 0.4).abs() + (c[1] - 0.25).abs() + (c[2] - 0.1).abs() < 1e-4);
    assert!((out.roughness.pixels()[i] - 0.7).abs() < 1e-5);
    assert_eq!(out.metal.pixels()[i], 0.0);
}

#[test]
fn dust_settles_on_what_faces_up_and_not_the_sides() {
    // Facing up rises from 0 on the left to 1 on the right.
    let maps = model(32, |map, u, _| match map {
        ModelMap::FacingUp => u,
        m => m.neutral(),
    });
    let s = model_only(&[]);
    let (out, cover) = run_on("dust", 32, &s, &[("material", flat(32, 1.0))], Some(maps));
    assert!(mean(&cover, 28..32, 0..32) > 0.75);
    assert!(mean(&cover, 0..6, 0..32) < 0.01);
    assert!(out.colour.at(30, 3)[0] > 0.6);
    assert!(out.metal.at(30, 3) < 0.3);
    // More Amount reaches further down the sides.
    let more = model_only(&[("amount", Number(0.9))]);
    let maps = model(32, |map, u, _| match map {
        ModelMap::FacingUp => u,
        m => m.neutral(),
    });
    let (_, wider) = run_on(
        "dust",
        32,
        &more,
        &[("material", flat(32, 1.0))],
        Some(maps),
    );
    assert!(mean(&wider, 8..16, 0..32) > mean(&cover, 8..16, 0..32) + 0.2);
}

/// One spot where a streak starts, at (`x`, `y`) of an `n` texture.
fn spot(n: u32, x: u32, y: u32) -> Value {
    Value::Shape(Arc::new(
        Image::from_pixels(
            size(n),
            (0..n * n)
                .map(|i| if i % n == x && i / n == y { 1.0 } else { 0.0 })
                .collect(),
        )
        .unwrap(),
    ))
}

#[test]
fn a_streak_runs_down_the_texture_below_where_it_starts() {
    let n = 64;
    let s = with(&[
        ("wobble", Number(0.0)),
        ("fade", Number(0.0)),
        ("length", Number(25.0)),
        ("strength", Number(1.0)),
    ]);
    let (out, cover) = run(
        "water_streaks",
        n,
        &s,
        &[("material", flat(n, 0.0)), ("start", spot(n, 20, 10))],
    );
    // 25 cm of a 100 cm texture is 16 pixels below it.
    for y in 10..24 {
        assert!(cover.at(20, y) > 0.9, "below at {y}: {}", cover.at(20, y));
    }
    assert!(cover.at(20, 30) < 0.01 && cover.at(20, 5) < 0.01);
    assert!(cover.at(30, 15) < 0.01);
    assert!(out.colour.at(20, 15)[0] < 0.3);
    // Fade makes it fainter toward its end.
    let s = with(&[
        ("wobble", Number(0.0)),
        ("fade", Number(1.0)),
        ("length", Number(25.0)),
    ]);
    let (_, faded) = run(
        "water_streaks",
        n,
        &s,
        &[("material", flat(n, 0.0)), ("start", spot(n, 20, 10))],
    );
    assert!(faded.at(20, 12) > faded.at(20, 22) + 0.3);
}

#[test]
fn a_streak_carries_the_colour_of_where_it_starts() {
    let n = 32;
    let mut colours = vec![[0.5, 0.5, 0.5]; (n * n) as usize];
    colours[(4 * n + 10) as usize] = [0.8, 0.3, 0.1];
    let m = Value::Material(Arc::new(MaterialMaps {
        colour: Arc::new(Rgb::from_pixels(size(n), colours).unwrap()),
        ..(**flat(n, 0.0).material().unwrap()).clone()
    }));
    let s = with(&[
        ("wobble", Number(0.0)),
        ("fade", Number(0.0)),
        ("carry", Number(1.0)),
        ("strength", Number(1.0)),
    ]);
    let (out, _) = run(
        "water_streaks",
        n,
        &s,
        &[("material", m), ("start", spot(n, 10, 4))],
    );
    let c = out.colour.at(10, 9);
    assert!(c[0] > 0.6 && c[2] < 0.25, "{c:?}");
}

#[test]
fn streaks_run_down_the_model_and_only_on_its_sides() {
    // The model's Up rises to the right: down is to the left. Its right
    // quarter faces up.
    let n = 64;
    let maps = model(n, |map, u, _| match map {
        ModelMap::Up => u,
        ModelMap::FacingUp => {
            if u > 0.75 {
                1.0
            } else {
                0.0
            }
        }
        m => m.neutral(),
    });
    let s = with(&[
        ("wobble", Number(0.0)),
        ("fade", Number(0.0)),
        ("length", Number(25.0)),
    ]);
    let starts = Value::Shape(Arc::new(
        Image::from_pixels(
            size(n),
            (0..n * n)
                .map(|i| {
                    if [40, 56].contains(&(i % n)) && i / n == 30 {
                        1.0
                    } else {
                        0.0
                    }
                })
                .collect(),
        )
        .unwrap(),
    ));
    let inputs = [("material", flat(n, 0.0)), ("start", starts)];
    let (_, cover) = run_on("water_streaks", n, &s, &inputs, Some(maps.clone()));
    assert!(cover.at(32, 30) > 0.9, "left of the start");
    assert!(cover.at(46, 30) < 0.01, "not to the right");
    assert!(cover.at(40, 36) < 0.01, "not down the texture");
    // The start on the part facing up makes no streak.
    assert!(cover.at(52, 30) < 0.01);
    // Runs: Down the texture ignores the model.
    let mut s = s.clone();
    s.set("runs", Choice(1));
    let (_, cover) = run_on("water_streaks", n, &s, &inputs, Some(maps));
    assert!(cover.at(40, 36) > 0.9 && cover.at(32, 30) < 0.01);
}

#[test]
fn the_weathering_tiles_seamlessly_with_other_settings_too() {
    let cases: Vec<(&str, Settings)> = vec![
        (
            "dirt",
            with(&[
                ("amount", Number(0.8)),
                ("film", Number(1.0)),
                ("patch_size", Number(7.0)),
            ]),
        ),
        (
            "dirt",
            with(&[("model", Number(0.0)), ("own", Number(1.0))]),
        ),
        (
            "rust",
            with(&[
                ("amount", Number(0.9)),
                ("flaking", Number(1.0)),
                ("at_edges", Number(0.0)),
            ]),
        ),
        (
            "rust",
            with(&[("metal_only", Switch(true)), ("dark", Colour([0.0; 3]))]),
        ),
        (
            "edge_wear",
            with(&[
                ("amount", Number(0.8)),
                ("breakup", Number(1.0)),
                ("chip_size", Number(1.0)),
            ]),
        ),
        (
            "edge_wear",
            with(&[("depth", Number(1.0)), ("softness", Number(1.0))]),
        ),
        (
            "dust",
            with(&[("amount", Number(0.9)), ("corners", Number(1.0))]),
        ),
        (
            "water_streaks",
            with(&[
                ("amount", Number(1.0)),
                ("wobble", Number(1.0)),
                ("length", Number(150.0)),
            ]),
        ),
        (
            "water_streaks",
            with(&[
                ("carry", Number(1.0)),
                ("fade", Number(0.0)),
                ("width", Number(5.0)),
            ]),
        ),
    ];
    for (kind, s) in cases {
        let info = registry().get(kind).unwrap().info();
        let inputs: Vec<(&str, Value)> = vec![("material", input(ValueKind::Material, size(64)))];
        let job =
            Job::new(info, size(64), 3, 2, &s, job_inputs(kind, &inputs)).with_real_size(100.0);
        let out = registry().get(kind).unwrap().run(&job).unwrap();
        assert_seamless(kind, &out);
    }
}

/// Noise → Material from height → `kind` → Material out.
fn weathered_graph(kind: &str) -> Graph {
    let mut g = Graph::new("Weathered");
    let mut ids = Vec::new();
    for (i, k) in ["noise", "material_from_height", kind, "material_out"]
        .iter()
        .enumerate()
    {
        ids.push(crate::BoxId(g.next_id()));
        g.apply(
            registry(),
            &GraphEdit::AddBox {
                kind: (*k).into(),
                at: Vec2::new(i as f32 * 200.0, 0.0),
            },
        )
        .unwrap();
    }
    for (from, to) in [
        (
            PlugRef::new(ids[0], "height"),
            PlugRef::new(ids[1], "height"),
        ),
        (
            PlugRef::new(ids[1], "material"),
            PlugRef::new(ids[2], "material"),
        ),
        (
            PlugRef::new(ids[2], "material_out"),
            PlugRef::new(ids[3], "material"),
        ),
    ] {
        g.apply(registry(), &GraphEdit::Connect { from, to })
            .unwrap();
    }
    g
}

#[test]
fn a_weathered_graph_follows_the_model_in_the_painter() {
    let wanted = |kind: &str| -> BTreeSet<ModelMap> {
        registry()
            .get(kind)
            .unwrap()
            .model_maps()
            .iter()
            .copied()
            .collect()
    };
    for kind in NEW {
        let g = weathered_graph(kind);
        assert!(uses_model(registry(), &g), "{kind}");
        assert_eq!(maps_needed(registry(), &g), wanted(kind), "{kind}");
        // Its export on its own says it follows the model.
        let notes = bake_notes(registry(), &g, false);
        assert!(
            notes.iter().any(|n| matches!(n, BakeNote::FollowsModel(_))),
            "{kind}"
        );
        assert!(bake_notes(registry(), &g, true).is_empty(), "{kind}");
    }
    // On a model with a crease down its middle, the dirt is there, on the
    // model's whole texture square (not a repeat).
    let g = weathered_graph("dirt");
    let n = 64;
    let maps = model(n, |map, u, _| match map {
        ModelMap::Curvature if (u - 0.5).abs() < 0.05 => 0.0,
        m => m.neutral(),
    });
    let on = OnModel::new(maps, 4.0);
    let m = Evaluator::with_threads(2)
        .material_on_model(registry(), &g, &on)
        .unwrap();
    assert_eq!(m.size(), n);
    let dark = |x: i64| (0..n as i64).map(|y| m.colour.at(x, y)[0]).sum::<f32>() / n as f32;
    assert!(dark(32) < dark(8) - 0.1, "{} {}", dark(32), dark(8));
}

#[test]
fn a_weathered_graph_with_no_model_tiles_end_to_end() {
    for kind in NEW {
        let g = weathered_graph(kind);
        let m = Evaluator::with_threads(2)
            .material(registry(), &g, size(64))
            .unwrap();
        assert_seamless(kind, &[Value::Material(m)]);
    }
}

#[test]
fn a_colour_wired_into_a_weathering_box_is_refused_in_plain_words() {
    let mut g = Graph::new("Colour");
    for (i, k) in ["colour_fill", "dirt"].iter().enumerate() {
        g.apply(
            registry(),
            &GraphEdit::AddBox {
                kind: (*k).into(),
                at: Vec2::new(i as f32 * 200.0, 0.0),
            },
        )
        .unwrap();
    }
    let (fill, dirt) = (
        g.first_of("colour_fill").unwrap(),
        g.first_of("dirt").unwrap(),
    );
    let why = g
        .check_connection(
            registry(),
            &PlugRef::new(fill, "colour"),
            &PlugRef::new(dirt, "material"),
        )
        .unwrap_err();
    // In plain words: Dirt's Material needs a whole material.
    assert!(
        why.reason.contains("Dirt") && why.reason.contains("Material"),
        "{}",
        why.reason
    );
}

#[test]
fn weathering_boxes_are_refused_inside_your_own_box() {
    let g = weathered_graph("rust");
    let rust = g.first_of("rust").unwrap();
    let pick = BTreeSet::from([rust]);
    let why = crate::own::Proposal::of(&g, registry(), &pick).unwrap_err();
    assert!(
        why.reason.contains("Rust") && why.reason.contains("model"),
        "{}",
        why.reason
    );
}

/// How long each box takes at 2048 pixels on two cores (run with
/// `--release`).
#[test]
#[ignore = "a speed check, run by hand"]
fn weathering_speed() {
    let n = 2048;
    let m = input(ValueKind::Material, size(n));
    for kind in NEW {
        let start = std::time::Instant::now();
        run(kind, n, &Settings::new(), &[("material", m.clone())]);
        println!("{kind}: {:.2} s", start.elapsed().as_secs_f32());
    }
}

/// Writes each box's material colour, and its weathering, at 512 pixels,
/// as PPM pictures into the folder `WEATHER_RENDERS` names.
#[test]
#[ignore = "pictures to look at, written by hand"]
fn weathering_renders() {
    let Ok(folder) = std::env::var("WEATHER_RENDERS") else {
        return;
    };
    let n = 512;
    let tiles = input(ValueKind::Shape, size(n));
    let info = registry().get("material_from_height").unwrap().info();
    let job = Job::new(
        info,
        size(n),
        1,
        2,
        &Settings::new(),
        vec![Some(tiles), None],
    );
    let tiled = registry()
        .get("material_from_height")
        .unwrap()
        .run(&job)
        .unwrap()
        .remove(0);
    let write = |name: &str, f: &dyn Fn(i64, i64) -> [f32; 3]| {
        let mut bytes = format!("P6 {n} {n} 255\n").into_bytes();
        for y in 0..i64::from(n) {
            for x in 0..i64::from(n) {
                bytes.extend(f(x, y).map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8));
            }
        }
        std::fs::write(format!("{folder}/{name}.ppm"), bytes).unwrap();
    };
    for (base, m) in [
        ("noise", input(ValueKind::Material, size(n))),
        ("tiles", tiled),
    ] {
        for kind in NEW {
            let (out, cover) = run(kind, n, &Settings::new(), &[("material", m.clone())]);
            write(&format!("{base}_{kind}"), &|x, y| out.colour.at(x, y));
            write(&format!("{base}_{kind}_cover"), &|x, y| [cover.at(x, y); 3]);
        }
    }
}
