//! Tests for working a graph out on a model (milestone 5).

use super::*;
use crate::boxes::from_model::{CURVATURE, FACING_UP, POSITION, stand_in};
use crate::eval::Evaluator;
use crate::graph::{BoxId, GraphEdit, PlugRef};
use crate::image::Image;
use crate::settings::SettingValue;
use glam::Vec2;

fn registry() -> &'static Registry {
    Registry::standard()
}

fn size(n: u32) -> Size {
    Size::new(n).unwrap()
}

/// Adds boxes of `kinds` to a new graph, returning it and their ids.
fn graph(kinds: &[&str]) -> (Graph, Vec<BoxId>) {
    let mut g = Graph::new("On a model");
    let mut ids = Vec::new();
    for (i, kind) in kinds.iter().enumerate() {
        ids.push(BoxId(g.next_id()));
        g.apply(
            registry(),
            &GraphEdit::AddBox {
                kind: (*kind).into(),
                at: Vec2::new(i as f32 * 200.0, 0.0),
            },
        )
        .unwrap();
    }
    (g, ids)
}

fn connect(g: &mut Graph, from: (BoxId, &str), to: (BoxId, &str)) {
    g.apply(
        registry(),
        &GraphEdit::Connect {
            from: PlugRef::new(from.0, from.1),
            to: PlugRef::new(to.0, to.1),
        },
    )
    .unwrap();
}

fn set(g: &mut Graph, node: BoxId, key: &str, value: SettingValue) {
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

/// A model's maps at `n` pixels: facing up rising from left (0) to right
/// (1), curvature with a light edge down the middle, the rest flat.
fn maps(n: u32, key: u64) -> Arc<ModelMaps> {
    let s = size(n);
    let mut maps = ModelMaps::new(s, key);
    let across = |f: &dyn Fn(f32) -> f32| {
        let pixels = (0..n * n)
            .map(|i| f(((i % n) as f32 + 0.5) / n as f32))
            .collect();
        Image::from_pixels(s, pixels).unwrap()
    };
    assert!(maps.insert(ModelMap::FacingUp, across(&|u| u)));
    assert!(maps.insert(
        ModelMap::Curvature,
        across(&|u| if (u - 0.5).abs() < 0.1 { 1.0 } else { 0.5 })
    ));
    for map in [ModelMap::Up, ModelMap::Across, ModelMap::FrontToBack] {
        assert!(maps.insert(map, across(&|_| 0.25)));
    }
    Arc::new(maps)
}

fn on(n: u32, repeats: f32) -> OnModel {
    OnModel::new(maps(n, 7), repeats)
}

#[test]
fn maps_must_be_the_texture_size() {
    let mut maps = ModelMaps::new(size(32), 1);
    assert!(!maps.insert(ModelMap::Occlusion, Image::filled(size(16), 1.0)));
    assert!(maps.get(ModelMap::Occlusion).is_none());
    assert!(maps.insert(ModelMap::Occlusion, Image::filled(size(32), 1.0)));
    assert_eq!(maps.memory_size(), 32 * 32 * 4);
}

#[test]
fn the_repeat_size_follows_the_repeats_within_limits() {
    let at = |n, repeats| on(n, repeats).tile_size().get();
    assert_eq!(at(64, 1.0), 64);
    assert_eq!(at(64, 4.0), 16);
    // Past the smallest size, the smallest.
    assert_eq!(at(64, 400.0), Size::MIN);
    // A repeat bigger than the texture: at most twice the texture.
    assert_eq!(at(64, 0.1), 128);
    // Nonsense repeats count as one.
    assert_eq!(at(64, f32::NAN), 64);
    assert_eq!(at(64, -3.0), 64);
    assert_eq!(on(64, 0.0).repeats(), 1.0);
    assert_eq!(on(64, 2.0).square_cm(150.0), 300.0);
}

#[test]
fn a_value_repeated_shows_the_same_pixels_each_repeat() {
    let s = size(16);
    let pixels = (0..256).map(|i| (i % 16) as f32 / 15.0).collect();
    let value = Value::Height(Arc::new(Image::from_pixels(s, pixels).unwrap()));
    let twice = value.repeated(2.0, size(32), 2);
    let g = twice.grey().unwrap();
    assert_eq!(g.size(), 32);
    for y in 0..32 {
        for x in 0..16 {
            assert!((g.at(x, y) - g.at(x + 16, y)).abs() < 1e-5);
        }
    }
    // Once at the same size gives it back.
    let once = value.repeated(1.0, s, 1);
    for (a, b) in once
        .grey()
        .unwrap()
        .pixels()
        .iter()
        .zip(value.grey().unwrap().pixels())
    {
        assert!((a - b).abs() < 1e-5);
    }
    // A material keeps every map.
    let material = crate::value::MaterialMaps {
        colour: Arc::new(Image::filled(s, [0.2, 0.4, 0.6])),
        height: Arc::new(Image::filled(s, 0.5)),
        roughness: Arc::new(Image::filled(s, 0.7)),
        metal: Arc::new(Image::filled(s, 0.0)),
        normal: Arc::new(Image::filled(s, [0.5, 0.5, 1.0])),
        occlusion: Arc::new(Image::filled(s, 1.0)),
        curvature: Arc::new(Image::filled(s, 0.5)),
    };
    let laid = Value::Material(Arc::new(material)).repeated(3.0, size(64), 1);
    let m = laid.material().unwrap();
    assert_eq!(m.size(), 64);
    assert!((m.roughness.at(10, 10) - 0.7).abs() < 1e-5);
    assert!((m.colour.at(5, 50)[2] - 0.6).abs() < 1e-5);
}

#[test]
fn the_from_the_model_boxes_give_the_models_maps_in_the_painter() {
    let (mut g, ids) = graph(&[FACING_UP, "levels"]);
    connect(&mut g, (ids[0], "up"), (ids[1], "in"));
    let on = on(32, 4.0);
    let mut evaluator = Evaluator::with_threads(2);
    let e = evaluator.run_on_model(registry(), &g, &on, &[]);
    let up = e.picture(ids[0]).unwrap().grey().unwrap().clone();
    assert_eq!(up.size(), 32);
    assert_eq!(*up, **on.maps.get(ModelMap::FacingUp).unwrap());
    // What it feeds works on the model's square too.
    assert_eq!(e.picture(ids[1]).unwrap().size(), 32);
    // In the Material mode, the stand-in.
    let alone = evaluator.run(registry(), &g, size(32), &[]);
    let stand = alone.picture(ids[0]).unwrap().grey().unwrap().clone();
    assert!((stand.at(0, 0) - stand_in(ModelMap::FacingUp, 0.5 / 32.0, 0.5 / 32.0)).abs() < 1e-5);
    assert_ne!(*stand, *up);
}

#[test]
fn curvature_gives_edges_and_creases_apart() {
    let (g, ids) = graph(&[CURVATURE]);
    let on = on(32, 1.0);
    let e = Evaluator::with_threads(1).run_on_model(registry(), &g, &on, &[]);
    let Ok(result) = e.outcome(ids[0]).unwrap() else {
        panic!("curvature can't run")
    };
    let [Some(curvature), Some(edges), Some(creases)] = &result.outputs[..] else {
        panic!("three outputs")
    };
    assert_eq!(curvature.kind(), crate::ValueKind::Height);
    let (edges, creases) = (edges.grey().unwrap(), creases.grey().unwrap());
    // The light edge down the middle is an edge, not a crease; the flat
    // sides neither.
    assert!((edges.at(16, 3) - 1.0).abs() < 1e-5);
    assert_eq!(creases.at(16, 3), 0.0);
    assert_eq!(edges.at(2, 3), 0.0);
    assert_eq!(creases.at(2, 3), 0.0);
}

#[test]
fn repeats_meeting_the_model_are_laid_across_it() {
    // Noise (a repeat) blended with Facing up (the model): the noise is
    // made at the repeat's size, then laid four times across the square.
    let (mut g, ids) = graph(&["noise", FACING_UP, "blend"]);
    connect(&mut g, (ids[0], "height"), (ids[2], "bottom"));
    connect(&mut g, (ids[1], "up"), (ids[2], "top"));
    let on = on(64, 4.0);
    let e = Evaluator::with_threads(2).run_on_model(registry(), &g, &on, &[]);
    let noise = e.picture(ids[0]).unwrap();
    assert_eq!(noise.size(), 16);
    assert_eq!(e.picture(ids[2]).unwrap().size(), 64);
}

#[test]
fn a_graph_without_the_model_is_its_repeat_laid_across() {
    let (mut g, ids) = graph(&["noise", "material_from_height", "material_out"]);
    connect(&mut g, (ids[0], "height"), (ids[1], "height"));
    connect(&mut g, (ids[1], "material"), (ids[2], "material"));
    assert!(!uses_model(registry(), &g));
    assert!(maps_needed(registry(), &g).is_empty());
    let on = on(64, 2.0);
    let mut evaluator = Evaluator::with_threads(2);
    let laid = evaluator.material_on_model(registry(), &g, &on).unwrap();
    assert_eq!(laid.size(), 64);
    let repeat = evaluator.material(registry(), &g, size(32)).unwrap();
    let expected = Value::Material(repeat).repeated(2.0, size(64), 1);
    assert_eq!(*laid, **expected.material().unwrap());
    // And repeats twice: the left half matches the right.
    assert!((laid.height.at(3, 9) - laid.height.at(35, 9)).abs() < 1e-5);
}

#[test]
fn moss_in_the_creases_follows_the_model() {
    // Stone everywhere, moss where the model's curvature is a crease.
    let (mut g, ids) = graph(&[
        "colour_fill",
        "noise",
        "material_from_height",
        "colour_fill",
        "material_from_height",
        CURVATURE,
        "mix_materials",
        "material_out",
    ]);
    let [
        stone_colour,
        noise,
        stone,
        moss_colour,
        moss,
        curve,
        mix,
        out,
    ] = ids[..]
    else {
        unreachable!()
    };
    connect(&mut g, (stone_colour, "colour"), (stone, "colour"));
    connect(&mut g, (noise, "height"), (stone, "height"));
    connect(&mut g, (moss_colour, "colour"), (moss, "colour"));
    connect(&mut g, (noise, "height"), (moss, "height"));
    connect(&mut g, (stone, "material"), (mix, "bottom"));
    connect(&mut g, (moss, "material"), (mix, "top"));
    connect(&mut g, (curve, "creases"), (mix, "where"));
    connect(&mut g, (mix, "material"), (out, "material"));
    set(
        &mut g,
        stone_colour,
        "colour",
        SettingValue::Colour([0.5, 0.5, 0.5]),
    );
    set(
        &mut g,
        moss_colour,
        "colour",
        SettingValue::Colour([0.1, 0.6, 0.1]),
    );
    assert!(uses_model(registry(), &g));
    assert_eq!(
        maps_needed(registry(), &g),
        BTreeSet::from([ModelMap::Curvature])
    );
    // A model with a crease down the middle.
    let n = 32;
    let mut maps = ModelMaps::new(size(n), 3);
    let pixels = (0..n * n)
        .map(|i| {
            if (i % n).abs_diff(n / 2) < 3 {
                0.0
            } else {
                0.5
            }
        })
        .collect();
    maps.insert(
        ModelMap::Curvature,
        Image::from_pixels(size(n), pixels).unwrap(),
    );
    let on = OnModel::new(Arc::new(maps), 2.0);
    let m = Evaluator::with_threads(2)
        .material_on_model(registry(), &g, &on)
        .unwrap();
    let green = |x: i64| {
        let c = m.colour.at(x, 5);
        c[1] - c[0]
    };
    assert!(green(16) > 0.3, "moss in the crease: {}", green(16));
    assert!(green(2).abs() < 0.05, "stone on the flat: {}", green(2));
}

#[test]
fn only_what_the_model_feeds_is_made_again_for_another_model() {
    let (mut g, ids) = graph(&["noise", FACING_UP, "blend"]);
    connect(&mut g, (ids[0], "height"), (ids[2], "bottom"));
    connect(&mut g, (ids[1], "up"), (ids[2], "top"));
    let mut evaluator = Evaluator::with_threads(1);
    let first = on(32, 2.0);
    let e = evaluator.run_on_model(registry(), &g, &first, &[]);
    assert_eq!(e.made(), 3);
    let again = evaluator.run_on_model(registry(), &g, &first, &[]);
    assert_eq!((again.made(), again.reused()), (0, 3));
    // Another model (another key): the noise repeat is reused.
    let other = OnModel::new(maps(32, 8), 2.0);
    let e = evaluator.run_on_model(registry(), &g, &other, &[]);
    assert_eq!((e.made(), e.reused()), (2, 1));
    // Other repeats: the noise is made at another size, and what it
    // feeds again; the model's map is the same.
    let closer = OnModel::new(maps(32, 8), 1.0);
    let e = evaluator.run_on_model(registry(), &g, &closer, &[]);
    assert_eq!(e.picture(ids[0]).unwrap().size(), 32);
    assert_eq!((e.made(), e.reused()), (2, 1));
}

#[test]
fn position_gives_three_maps_and_the_needed_maps_say_so() {
    let (g, ids) = graph(&[POSITION]);
    assert_eq!(
        maps_needed(registry(), &g),
        BTreeSet::from([ModelMap::Up, ModelMap::Across, ModelMap::FrontToBack])
    );
    let e = Evaluator::with_threads(1).run_on_model(registry(), &g, &on(16, 1.0), &[]);
    let Ok(result) = e.outcome(ids[0]).unwrap() else {
        panic!("position can't run")
    };
    assert_eq!(result.outputs.len(), 3);
    for out in &result.outputs {
        assert!((out.as_ref().unwrap().grey().unwrap().at(3, 3) - 0.25).abs() < 1e-5);
    }
}

#[test]
fn a_missing_map_falls_back_to_the_stand_in() {
    // Occlusion wasn't made: the box still runs, on its stand-in.
    let (g, ids) = graph(&["model_occlusion"]);
    let e = Evaluator::with_threads(1).run_on_model(registry(), &g, &on(16, 1.0), &[]);
    let pic = e.picture(ids[0]).unwrap().grey().unwrap().clone();
    assert_eq!(pic.size(), 16);
    assert!(pic.pixels().iter().all(|v| (0.0..=1.0).contains(v)));
}

#[test]
fn without_material_out_there_is_nothing_to_lay() {
    let (g, _) = graph(&["noise"]);
    let why = Evaluator::with_threads(1)
        .material_on_model(registry(), &g, &on(16, 1.0))
        .unwrap_err();
    assert!(!why.reason.is_empty());
    assert!(!why.fixes.is_empty());
}

#[test]
fn every_stand_in_is_in_range_and_neutral_values_are_too() {
    for map in ModelMap::ALL {
        assert!((0.0..=1.0).contains(&map.neutral()));
        for i in 0..50 {
            for j in 0..50 {
                let v = stand_in(map, i as f32 / 50.0, j as f32 / 50.0);
                assert!((0.0..=1.0).contains(&v), "{map:?} {v}");
            }
        }
    }
}

#[test]
fn tiling_lays_the_repeat_more_often_but_keeps_the_models_size() {
    let tiled = on(64, 2.0).with_tiling(4.0);
    assert_eq!(tiled.repeats(), 8.0);
    assert_eq!(tiled.tile_size(), on(64, 8.0).tile_size());
    // The model is as big as it was: lengths on it stay real centimetres.
    assert_eq!(tiled.square_cm(150.0), 300.0);
    // Nonsense tiling counts as one.
    assert_eq!(on(64, 2.0).with_tiling(f32::NAN).repeats(), 2.0);
    assert_eq!(on(64, 2.0).with_tiling(0.0).repeats(), 2.0);
}

#[test]
fn a_tiled_material_is_the_repeat_laid_more_often() {
    let (mut g, ids) = graph(&["bricks", "material_from_height", "material_out"]);
    connect(&mut g, (ids[0], "shape"), (ids[1], "height"));
    connect(&mut g, (ids[1], "material"), (ids[2], "material"));
    let mut e = Evaluator::with_threads(2);
    let tiled = e
        .material_on_model(registry(), &g, &on(64, 1.0).with_tiling(2.0))
        .unwrap();
    let doubled = e.material_on_model(registry(), &g, &on(64, 2.0)).unwrap();
    assert_eq!(tiled, doubled);
    let plain = e.material_on_model(registry(), &g, &on(64, 1.0)).unwrap();
    assert_ne!(tiled, plain);
}

#[test]
fn what_follows_the_model_stays_put_when_tiled() {
    // Facing up straight into Material from height: nothing repeats.
    let (mut g, ids) = graph(&[FACING_UP, "material_from_height", "material_out"]);
    connect(&mut g, (ids[0], "up"), (ids[1], "height"));
    connect(&mut g, (ids[1], "material"), (ids[2], "material"));
    let mut e = Evaluator::with_threads(1);
    let plain = e.material_on_model(registry(), &g, &on(32, 1.0)).unwrap();
    let tiled = e
        .material_on_model(registry(), &g, &on(32, 1.0).with_tiling(4.0))
        .unwrap();
    assert_eq!(plain, tiled);
}
