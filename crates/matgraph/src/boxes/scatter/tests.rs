//! Scatter's checks: everything in range and seamless with every kind
//! plugged in and many settings, the same pictures for the same seed on
//! any number of cores, each kind of variation, the mask, sizes in
//! centimetres, settings at their ends, the refusals, and a pebbles
//! graph end to end. `scatter_speed` (ignored) times it by hand.

use crate::boxes::tests_each::{assert_in_range, assert_seamless, input, registry, size, with};
use crate::graph::{BoxId, Graph, GraphEdit, PlugRef};
use crate::image::{Image, Size};
use crate::registry::Job;
use crate::settings::{SettingValue, Settings};
use crate::value::Value;
use crate::{Evaluator, ValueKind};
use glam::Vec2;
use std::sync::Arc;

/// Scatter's outputs with `inputs` plugged in (by key; others empty),
/// at `n` pixels, seed `seed`, on `threads` cores, `cm` across.
fn scatter(
    s: &Settings,
    inputs: &[(&str, &Value)],
    n: u32,
    seed: u64,
    threads: usize,
    cm: f32,
) -> Vec<Option<Value>> {
    let t = registry().get("scatter").unwrap();
    let info = t.info();
    let given = info
        .inputs
        .iter()
        .map(|p| {
            inputs
                .iter()
                .find(|(k, _)| *k == p.key)
                .map(|(_, v)| (*v).clone())
        })
        .collect();
    let job = Job::new(info, size(n), seed, threads, s, given).with_real_size(cm);
    t.run_outputs(&job).unwrap()
}

/// The output `key`'s greys.
fn grey(out: &[Option<Value>], key: &str) -> Arc<Image<f32>> {
    let at = super::OUTPUTS.iter().position(|k| *k == key).unwrap();
    out[at].as_ref().unwrap().grey().unwrap().clone()
}

/// The share of pixels lighter than `above`.
fn share(image: &Image<f32>, above: f32) -> f32 {
    let lit = image.pixels().iter().filter(|v| **v > above).count();
    lit as f32 / image.pixels().len() as f32
}

/// Settings to try: each arrangement, kept apart, shifted rows, loose,
/// turned, every variation at its most.
fn many() -> Vec<Settings> {
    use SettingValue::{Choice, Number, Switch};
    vec![
        Settings::new(),
        with(&[("arrange", Choice(1)), ("shift", Number(0.5))]),
        with(&[
            ("arrange", Choice(1)),
            ("loose", Number(1.0)),
            ("spacing", Number(30.0)),
            ("size", Number(40.0)),
        ]),
        with(&[("apart", Switch(true)), ("spacing", Number(5.0))]),
        with(&[
            ("size", Number(150.0)),
            ("spacing", Number(60.0)),
            ("turn", Number(30.0)),
            ("turn_vary", Number(0.0)),
        ]),
        with(&[
            ("size_vary", Number(0.9)),
            ("height_vary", Number(1.0)),
            ("colour_vary", Number(1.0)),
        ]),
    ]
}

#[test]
fn every_kind_scattered_is_in_range_and_tiles_seamlessly() {
    let n = size(64);
    let (shape, colour, material) = (
        input(ValueKind::Shape, n),
        input(ValueKind::Colour, n),
        input(ValueKind::Material, n),
    );
    let mask = input(ValueKind::Height, n);
    for s in many() {
        for inputs in [
            vec![],
            vec![("shape", &shape)],
            vec![("colour", &colour)],
            vec![("material", &material), ("where", &mask)],
            vec![
                ("shape", &shape),
                ("colour", &colour),
                ("material", &material),
            ],
        ] {
            let out: Vec<Value> = scatter(&s, &inputs, 64, 3, 2, 100.0)
                .into_iter()
                .flatten()
                .collect();
            let what = format!("{s:?} {:?}", inputs.iter().map(|i| i.0).collect::<Vec<_>>());
            assert_in_range(&what, &out);
            assert_seamless(&what, &out);
        }
    }
}

#[test]
fn only_what_is_plugged_in_comes_out_and_nothing_makes_pebbles() {
    let n = size(64);
    let alone = scatter(&Settings::new(), &[], 64, 1, 1, 100.0);
    assert!(alone[0].is_none() && alone[1].is_none());
    // Round pebbles: about one in each 10 cm square, each up to 8 cm
    // across, so they cover a share of the texture, highest in their
    // middles.
    let height = grey(&alone, "out");
    let covered = share(&grey(&alone, "copies"), 0.5);
    assert!((0.15..0.6).contains(&covered), "{covered}");
    assert!(height.pixels().iter().any(|v| *v > 0.9));
    let colour = input(ValueKind::Colour, n);
    let out = scatter(&Settings::new(), &[("colour", &colour)], 64, 1, 1, 100.0);
    assert!(out[0].is_none());
    assert_eq!(out[1].as_ref().unwrap().kind(), ValueKind::Colour);
    let material = input(ValueKind::Material, n);
    let out = scatter(
        &Settings::new(),
        &[("material", &material)],
        64,
        1,
        1,
        100.0,
    );
    assert!(out[0].is_some() && out[1].is_none());
}

#[test]
fn the_same_seed_gives_the_same_pictures_on_any_cores_and_another_seed_others() {
    let n = size(64);
    let material = input(ValueKind::Material, n);
    let inputs = [("material", &material)];
    for s in many() {
        let one = scatter(&s, &inputs, 64, 5, 1, 100.0);
        assert_eq!(one, scatter(&s, &inputs, 64, 5, 3, 100.0), "{s:?}");
        assert_ne!(one, scatter(&s, &inputs, 64, 6, 1, 100.0), "{s:?}");
    }
}

#[test]
fn each_kind_of_variation_varies_the_copies_and_none_keeps_them_alike() {
    use SettingValue::{Choice, Number};
    let n = size(128);
    let white = Value::Shape(Arc::new(Image::filled(n, 1.0)));
    let none = with(&[
        ("arrange", Choice(1)),
        ("spacing", Number(25.0)),
        ("size", Number(12.5)),
        ("size_vary", Number(0.0)),
        ("turn_vary", Number(0.0)),
        ("height_vary", Number(0.0)),
        ("colour_vary", Number(0.0)),
    ]);
    // Sixteen white squares, all alike: every copy's height is 1, and
    // the copies cover a quarter of the texture.
    let alike = scatter(&none, &[("shape", &white)], 128, 1, 1, 100.0);
    let height = grey(&alike, "out");
    let tops: Vec<f32> = (0..4)
        .flat_map(|r| (0..4).map(move |c| (c * 32 + 16, r * 32 + 16)))
        .map(|(x, y)| height.at(x, y))
        .collect();
    assert!(tops.iter().all(|t| (*t - 1.0).abs() < 1e-5), "{tops:?}");
    assert!((share(&height, 0.5) - 0.25).abs() < 0.02);
    // Height varies: copies of different heights.
    let mut s = none.clone();
    s.set("height_vary", Number(1.0));
    let varied = grey(&scatter(&s, &[("shape", &white)], 128, 1, 1, 100.0), "out");
    let tops: Vec<f32> = (0..4).map(|c| varied.at(c * 32 + 16, 16)).collect();
    assert!(
        tops.windows(2).any(|w| (w[0] - w[1]).abs() > 0.05),
        "{tops:?}"
    );
    // Size varies: smaller copies cover less.
    let mut s = none.clone();
    s.set("size_vary", Number(0.9));
    let smaller = grey(
        &scatter(&s, &[("shape", &white)], 128, 1, 1, 100.0),
        "copies",
    );
    assert!(share(&smaller, 0.5) < 0.2);
    // Turn: a copy turned an eighth reaches further across.
    let mut s = none.clone();
    s.set("turn", Number(45.0));
    let turned = grey(
        &scatter(&s, &[("shape", &white)], 128, 1, 1, 100.0),
        "copies",
    );
    assert!(turned.at(16 + 10, 16) > 0.5 && height.at(16 + 10, 16) < 0.5);
    // Colour varies: the copies' colours differ.
    let red = Value::Colour(Arc::new(Image::filled(n, [0.6, 0.3, 0.2])));
    let mut s = none.clone();
    s.set("colour_vary", Number(1.0));
    let out = scatter(&s, &[("colour", &red)], 128, 1, 1, 100.0);
    let c = out[1].as_ref().unwrap().colour().unwrap();
    let a = c.at(16, 16);
    assert!((0..4).any(|i| c.at(16 + i * 32, 48) != a), "{a:?}");
    let plain = scatter(&none, &[("colour", &red)], 128, 1, 1, 100.0);
    assert_eq!(
        plain[1].as_ref().unwrap().colour().unwrap().at(16, 16),
        [0.6, 0.3, 0.2]
    );
    // Random gives each copy its own grey.
    let random = grey(&alike, "random");
    let greys: Vec<f32> = (0..4).map(|c| random.at(c * 32 + 16, 16)).collect();
    assert!(greys.iter().all(|g| *g >= 0.1));
    assert!(greys.windows(2).any(|w| w[0] != w[1]), "{greys:?}");
}

#[test]
fn the_mask_keeps_copies_where_it_is_white() {
    let n = size(64);
    let black = Value::Shape(Arc::new(Image::filled(n, 0.0)));
    let out = scatter(&Settings::new(), &[("where", &black)], 64, 1, 1, 100.0);
    assert_eq!(share(&grey(&out, "copies"), 0.0), 0.0);
    // The top half only (copies reach a little past it).
    let top: Vec<f32> = (0..64 * 64)
        .map(|i| if i / 64 < 32 { 1.0 } else { 0.0 })
        .collect();
    let top = Value::Shape(Arc::new(Image::from_pixels(n, top).unwrap()));
    let s = with(&[("spacing", SettingValue::Number(5.0))]);
    let out = scatter(&s, &[("where", &top)], 64, 1, 1, 100.0);
    let copies = grey(&out, "copies");
    let row = |y: i64| (0..64).map(|x| copies.at(x, y)).sum::<f32>();
    assert!(row(16) > 5.0, "{}", row(16));
    assert_eq!(row(48), 0.0);
}

#[test]
fn sizes_are_centimetres_on_the_real_world_size() {
    use SettingValue::{Choice, Number};
    let s = with(&[
        ("arrange", Choice(1)),
        ("size_vary", Number(0.0)),
        ("turn_vary", Number(0.0)),
    ]);
    let n = size(128);
    let white = Value::Shape(Arc::new(Image::filled(n, 1.0)));
    // 8 cm copies 10 cm apart: on 100 cm, 0.64 of the texture; on
    // 200 cm, as many copies again in each direction but the same share.
    for cm in [100.0, 200.0] {
        let out = scatter(&s, &[("shape", &white)], 128, 1, 1, cm);
        let covered = share(&grey(&out, "copies"), 0.5);
        assert!((covered - 0.64).abs() < 0.05, "{cm}: {covered}");
    }
    // The same 8 cm copies 20 cm apart cover a quarter as much.
    let mut apart = s.clone();
    apart.set("spacing", Number(20.0));
    let out = scatter(&apart, &[("shape", &white)], 128, 1, 1, 100.0);
    let covered = share(&grey(&out, "copies"), 0.5);
    assert!((covered - 0.16).abs() < 0.03, "{covered}");
}

#[test]
fn settings_at_their_ends_and_odd_sizes_never_break_it() {
    use SettingValue::Number;
    let n = size(16);
    let material = input(ValueKind::Material, n);
    for (spacing, sz) in [(0.5, 0.5), (0.5, 200.0), (200.0, 0.5), (200.0, 200.0)] {
        let s = with(&[("spacing", Number(spacing)), ("size", Number(sz))]);
        for cm in [1.0, 100.0, 10_000.0] {
            let out: Vec<Value> = scatter(&s, &[("material", &material)], 16, 1, 2, cm)
                .into_iter()
                .flatten()
                .collect();
            assert_in_range(&format!("{spacing} {sz} {cm}"), &out);
        }
    }
    // Copies smaller than a pixel leave almost nothing, but nothing
    // that isn't a number.
    let s = with(&[("size", Number(0.5))]);
    let out = scatter(&s, &[], 64, 1, 1, 10_000.0);
    assert!(share(&grey(&out, "copies"), 0.5) < 0.01);
}

#[test]
fn a_flat_material_stays_flat_and_its_bumps_turn_with_each_copy() {
    let n = size(64);
    let flat = crate::boxes::moving::plain(ValueKind::Material, n);
    let out = scatter(&Settings::new(), &[("material", &flat)], 64, 1, 1, 100.0);
    let m = out[0].as_ref().unwrap().material().unwrap().clone();
    assert!(m.normal.pixels().iter().all(|p| (p[2] - 1.0).abs() < 1e-5));
    // A slope facing right, on one copy turned a quarter clockwise, faces
    // down the texture (y is up the texture in a normal).
    use SettingValue::{Choice, Number};
    let slope = |dir: [f32; 3]| {
        let mut maps = flat.material().unwrap().as_ref().clone();
        maps.normal = Arc::new(Image::filled(n, dir));
        Value::Material(Arc::new(maps))
    };
    let s = with(&[
        ("arrange", Choice(1)),
        ("spacing", Number(200.0)),
        ("size", Number(50.0)),
        ("turn", Number(90.0)),
        ("turn_vary", Number(0.0)),
        ("height_vary", Number(0.0)),
        ("size_vary", Number(0.0)),
    ]);
    let tilted = slope(glam::Vec3::new(0.3, 0.0, 1.0).normalize().to_array());
    let out = scatter(&s, &[("material", &tilted)], 64, 1, 1, 100.0);
    let normal = out[0]
        .as_ref()
        .unwrap()
        .material()
        .unwrap()
        .normal
        .at(32, 32);
    assert!(normal[1] < -0.3 && normal[0].abs() < 1e-3, "{normal:?}");
}

/// Adds `kinds` to a new graph, wires `wires` (box number and plug key)
/// and works it out at 64.
/// A wire: a box's output to a box's input, by box number and plug key.
type WireAt<'a> = ((u64, &'a str), (u64, &'a str));

fn worked_out(kinds: &[&str], wires: &[WireAt<'_>]) -> crate::Evaluation {
    let mut g = Graph::new("Scatter");
    for k in kinds {
        let add = GraphEdit::AddBox {
            kind: (*k).into(),
            at: Vec2::ZERO,
        };
        g.apply(registry(), &add).unwrap();
    }
    for (from, to) in wires {
        let connect = GraphEdit::Connect {
            from: PlugRef::new(BoxId(from.0), from.1),
            to: PlugRef::new(BoxId(to.0), to.1),
        };
        g.apply(registry(), &connect).unwrap();
    }
    Evaluator::with_threads(1).run(registry(), &g, Size::nearest(64), &[])
}

#[test]
fn an_empty_output_is_refused_with_why_and_scatter_alone_runs() {
    let e = worked_out(&["scatter"], &[]);
    assert_eq!(e.problems().count(), 0);
    assert_eq!(e.picture(BoxId(1)).unwrap().kind(), ValueKind::Height);
    let e = worked_out(
        &["scatter", "hue_saturation"],
        &[((1, "colour_out"), (2, "in"))],
    );
    let why = e.outcome(BoxId(2)).unwrap().as_ref().unwrap_err();
    assert!(why.reason.contains("gives nothing"), "{}", why.reason);
    // A material plugged in shows the material.
    let e = worked_out(
        &["noise", "material_from_height", "scatter"],
        &[
            ((1, "height"), (2, "height")),
            ((2, "material"), (3, "material")),
        ],
    );
    assert_eq!(e.picture(BoxId(3)).unwrap().kind(), ValueKind::Material);
}

#[test]
fn a_pebbles_graph_works_out_seamless_from_start_to_end() {
    // Pebbles (Scatter alone) made a material, mixed over a ground of
    // clouds where the copies are, into Material out at 100 cm.
    let mut g = Graph::new("Pebbles");
    let mut add = |kind: &str| {
        let id = BoxId(g.next_id());
        let edit = GraphEdit::AddBox {
            kind: kind.into(),
            at: Vec2::ZERO,
        };
        g.apply(registry(), &edit).unwrap();
        id
    };
    let pebbles = add("scatter");
    let stone = add("material_from_height");
    let clouds = add("clouds");
    let ground = add("material_from_height");
    let mix = add("mix_materials");
    let out = add("material_out");
    for (from, to) in [
        ((pebbles, "out"), (stone, "height")),
        ((clouds, "height"), (ground, "height")),
        ((ground, "material"), (mix, "bottom")),
        ((stone, "material"), (mix, "top")),
        ((pebbles, "copies"), (mix, "where")),
        ((mix, "material"), (out, "material")),
    ] {
        let connect = GraphEdit::Connect {
            from: PlugRef::new(from.0, from.1),
            to: PlugRef::new(to.0, to.1),
        };
        g.apply(registry(), &connect).unwrap();
    }
    let one = Evaluator::with_threads(1)
        .material(registry(), &g, size(128))
        .unwrap();
    let again = Evaluator::with_threads(2)
        .material(registry(), &g, size(128))
        .unwrap();
    assert_eq!(*one, *again);
    let value = Value::Material(one);
    assert_in_range("pebbles", std::slice::from_ref(&value));
    assert_seamless("pebbles", &[value]);
}

/// Times Scatter at 2048 pixels: run by hand with
/// `cargo test -p matgraph --release -- --ignored scatter_speed --nocapture`.
#[test]
#[ignore = "speed check: run by hand"]
fn scatter_speed() {
    use SettingValue::{Choice, Number, Switch};
    let threads = crate::parallel::cores();
    let n = size(2048);
    let material = input(ValueKind::Material, n);
    let shape = input(ValueKind::Shape, n);
    type Case<'a> = (&'a str, Settings, Vec<(&'a str, &'a Value)>);
    let cases: Vec<Case<'_>> = vec![
        ("pebbles", Settings::new(), vec![]),
        (
            "pebbles kept apart",
            with(&[("apart", Switch(true))]),
            vec![],
        ),
        ("a material", Settings::new(), vec![("material", &material)]),
        (
            "grass, 16 384 small copies",
            with(&[("spacing", Number(0.5)), ("size", Number(3.0))]),
            vec![("shape", &shape)],
        ),
        (
            "scales in a grid",
            with(&[
                ("arrange", Choice(1)),
                ("shift", Number(0.5)),
                ("spacing", Number(5.0)),
                ("size", Number(8.0)),
            ]),
            vec![("shape", &shape)],
        ),
        (
            "the deepest pile",
            with(&[("spacing", Number(0.5)), ("size", Number(200.0))]),
            vec![("material", &material)],
        ),
    ];
    for (what, s, inputs) in cases {
        let start = std::time::Instant::now();
        scatter(&s, &inputs, 2048, 1, threads, 100.0);
        let took = start.elapsed();
        #[allow(clippy::print_stdout)]
        {
            println!("{what:<28} {:>8.1} ms at 2048", took.as_secs_f64() * 1000.0);
        }
        // Generous: a slow, busy machine passes; a box gone wrong doesn't.
        assert!(took.as_secs() < 120, "{what} took {took:?}");
    }
}
