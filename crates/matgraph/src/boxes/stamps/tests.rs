//! The Stamps box's checks: a plain refusal with nothing brought in,
//! see-through parts kept (the outline, the colour under), the same
//! pictures for the same seed on any number of cores, seamless with many
//! settings, picking among pictures by weight, the puff and the height
//! from lightness, a material pressed on, and a graph end to end.
//! `stamps_speed` (ignored) times it by hand.

use super::*;
use crate::boxes::tests_each::{assert_in_range, assert_seamless, input, registry, size, with};
use crate::graph::{BoxId, Graph, GraphEdit, PlugRef};
use crate::settings::{SettingValue, Settings};
use crate::stamp::tests::dot;
use crate::{Evaluator, ValueKind};
use glam::Vec2;

/// A square stamp `n` across, solid `colour` with a see-through hole in
/// the middle (a washer).
fn washer(n: u32, colour: [u8; 3]) -> Stamp {
    let mut rgba = Vec::new();
    let c = n as f32 / 2.0;
    for y in 0..n {
        for x in 0..n {
            let (dx, dy) = (x as f32 + 0.5 - c, y as f32 + 0.5 - c);
            let hole = (dx * dx + dy * dy).sqrt() < c * 0.4;
            let a = if hole { 0 } else { 255 };
            rgba.extend_from_slice(&[colour[0], colour[1], colour[2], a]);
        }
    }
    Stamp::new(n, n, rgba, Vec::new(), "washer.png").unwrap()
}

/// The box's outputs with `stamps` (none: nothing brought in) and
/// `inputs` plugged in by key.
fn stamps_run(
    stamps: Option<&Stamps>,
    s: &Settings,
    inputs: &[(&str, &Value)],
    n: u32,
    seed: u64,
    threads: usize,
) -> Result<Vec<Option<Value>>, Refusal> {
    let t = registry().get(STAMPS).unwrap();
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
    let job = Job::new(info, size(n), seed, threads, s, given)
        .with_real_size(100.0)
        .with_stamps(stamps.cloned().map(Arc::new));
    t.run_outputs(&job)
}

fn out(o: &[Option<Value>], key: &str) -> Value {
    let at = OUTPUTS.iter().position(|k| *k == key).unwrap();
    o[at].clone().unwrap()
}

fn red_and_blue() -> Stamps {
    Stamps::one(dot(24, 24, [220, 20, 20]))
        .unwrap()
        .with_added(dot(24, 24, [20, 20, 220]))
        .unwrap()
}

#[test]
fn with_no_stamps_the_box_says_how_to_bring_one_in() {
    let why = stamps_run(None, &Settings::new(), &[], 32, 1, 1).unwrap_err();
    assert!(why.reason.contains("Add a stamp"), "{}", why.reason);
    // Asked for every output, it stamps a white dot.
    let t = registry().get(STAMPS).unwrap();
    let settings = Settings::new();
    let job = Job::new(t.info(), size(32), 1, 1, &settings, vec![None; 3]);
    let all = t.run(&job).unwrap();
    assert_eq!(all.len(), 5);
    assert!(all[3].grey().unwrap().pixels().iter().any(|v| *v > 0.9));
}

#[test]
fn see_through_parts_are_kept() {
    // One big washer in the middle (a grid of one), green under it.
    use SettingValue::{Choice, Number};
    let s = with(&[
        ("arrange", Choice(1)),
        ("spacing", Number(100.0)),
        ("size", Number(80.0)),
        ("size_vary", Number(0.0)),
        ("turn_vary", Number(0.0)),
        ("colour_vary", Number(0.0)),
        ("height_vary", Number(0.0)),
    ]);
    let green = Value::Colour(Arc::new(Image::filled(size(64), [0.0, 1.0, 0.0])));
    let list = Stamps::one(washer(40, [255, 0, 0])).unwrap();
    let o = stamps_run(Some(&list), &s, &[("colour", &green)], 64, 3, 2).unwrap();
    let covered = out(&o, "copies");
    let colour = out(&o, "colour_out");
    let (c, k) = (covered.grey().unwrap(), colour.colour().unwrap());
    // The hole in the middle: nothing stamped, the green shows.
    assert!(c.at(32, 32) < 0.01, "{}", c.at(32, 32));
    assert!(k.at(32, 32)[1] > 0.99 && k.at(32, 32)[0] < 0.01);
    // The ring: red, covering.
    assert!(c.at(32, 12) > 0.99);
    assert!(k.at(32, 12)[0] > 0.99 && k.at(32, 12)[1] < 0.01);
    // Outside the stamp's square: green again.
    assert!(c.at(2, 2) < 0.01 && k.at(2, 2)[1] > 0.99);
    // The height follows the outline: none in the hole.
    let h = out(&o, "out");
    assert!(h.grey().unwrap().at(32, 32) < 0.01);
    assert!(h.grey().unwrap().at(32, 9) > 0.1);
}

#[test]
fn the_same_seed_gives_the_same_pictures_on_any_number_of_cores() {
    let list = red_and_blue();
    let s = Settings::new();
    let a = stamps_run(Some(&list), &s, &[], 64, 9, 1).unwrap();
    let b = stamps_run(Some(&list), &s, &[], 64, 9, 3).unwrap();
    for key in ["colour_out", "out", "copies", "random"] {
        assert_eq!(out(&a, key), out(&b, key), "{key}");
    }
    let other = stamps_run(Some(&list), &s, &[], 64, 10, 1).unwrap();
    assert_ne!(out(&a, "copies"), out(&other, "copies"));
}

#[test]
fn stamps_tile_seamlessly_with_many_settings() {
    use SettingValue::{Choice, Number, Switch};
    let list = red_and_blue().with_added(washer(30, [200, 200, 0])).unwrap();
    let material = input(ValueKind::Material, size(64));
    let colour = input(ValueKind::Colour, size(64));
    let mask = input(ValueKind::Shape, size(64));
    for (i, s) in [
        Settings::new(),
        with(&[("size", Number(40.0)), ("apart", Switch(true))]),
        with(&[
            ("arrange", Choice(1)),
            ("spacing", Number(25.0)),
            ("shift", Number(0.5)),
            ("loose", Number(0.3)),
        ]),
        with(&[(HEIGHT_FROM, Choice(1)), (PUFF, Number(1.0))]),
        with(&[(PUFF, Number(0.0)), ("size", Number(0.5))]),
        // Copies as big as the texture, many deep.
        with(&[("size", Number(200.0)), ("spacing", Number(0.5))]),
    ]
    .into_iter()
    .enumerate()
    {
        // The seam check compares the steepest steps: each case its own
        // seed (with seed 5 the last case has a stamp's soft edge falling
        // exactly on the seam, as steep as any inside: looked at by eye).
        let o = stamps_run(
            Some(&list),
            &s,
            &[("material", &material), ("colour", &colour), ("where", &mask)],
            64,
            11 + i as u64,
            2,
        )
        .unwrap();
        let values: Vec<Value> = o.into_iter().map(Option::unwrap).collect();
        assert_in_range("stamps", &values);
        assert_seamless("stamps", &values);
    }
}

#[test]
fn copies_pick_among_the_pictures_by_weight() {
    use SettingValue::Number;
    let s = with(&[
        ("spacing", Number(4.0)),
        ("size", Number(3.0)),
        ("colour_vary", Number(0.0)),
        ("apart", SettingValue::Switch(true)),
    ]);
    let count = |list: &Stamps| {
        let o = stamps_run(Some(list), &s, &[], 256, 2, 2).unwrap();
        let c = out(&o, "colour_out");
        let (mut red, mut blue) = (0, 0);
        for p in c.colour().unwrap().pixels() {
            if p[0] > 0.6 && p[2] < 0.3 {
                red += 1;
            } else if p[2] > 0.6 && p[0] < 0.3 {
                blue += 1;
            }
        }
        (red as f32, blue as f32)
    };
    let (red, blue) = count(&red_and_blue());
    assert!(red > 0.0 && blue > 0.0);
    assert!((red / blue - 1.0).abs() < 0.35, "{red} {blue}");
    let (red, blue) = count(&red_and_blue().with_weight(1, 4));
    assert!(blue / red > 2.5, "{red} {blue}");
    // Only one picture: every copy is it.
    let (red, blue) = count(&Stamps::one(dot(24, 24, [220, 20, 20])).unwrap());
    assert!(red > 0.0 && blue == 0.0);
}

#[test]
fn adding_a_picture_keeps_the_copies_where_they_were() {
    // The pick has its own random numbers: places never move.
    let s = Settings::new();
    let one = Stamps::one(dot(24, 24, [220, 20, 20])).unwrap();
    let a = stamps_run(Some(&one), &s, &[], 64, 4, 2).unwrap();
    let b = stamps_run(Some(&red_and_blue()), &s, &[], 64, 4, 2).unwrap();
    assert_eq!(out(&a, "copies"), out(&b, "copies"));
    assert_eq!(out(&a, "random"), out(&b, "random"));
}

#[test]
fn the_puff_rounds_the_height_up_from_the_edge() {
    use SettingValue::{Choice, Number};
    let one = |puff: f32, from: u32| {
        let s = with(&[
            ("arrange", Choice(1)),
            ("spacing", Number(100.0)),
            ("size", Number(90.0)),
            ("size_vary", Number(0.0)),
            ("height_vary", Number(0.0)),
            (PUFF, Number(puff)),
            (HEIGHT_FROM, Choice(from)),
        ]);
        let list = Stamps::one(dot(48, 48, [128, 128, 128])).unwrap();
        let o = stamps_run(Some(&list), &s, &[], 64, 1, 2).unwrap();
        out(&o, "out").grey().unwrap().clone()
    };
    let flat = one(0.0, 0);
    let dome = one(1.0, 0);
    // Flat: as high in near the edge as in the middle.
    assert!((flat.at(32, 32) - 1.0).abs() < 0.02);
    assert!((flat.at(32, 12) - 1.0).abs() < 0.05, "{}", flat.at(32, 12));
    // A dome: lower near the edge, full in the middle.
    assert!(dome.at(32, 32) > 0.9);
    assert!(dome.at(32, 12) < flat.at(32, 12) - 0.2, "{}", dome.at(32, 12));
    // From its lightness: a mid grey stamp is about half as high.
    let lit = one(0.0, 1);
    assert!((lit.at(32, 32) - 0.5).abs() < 0.05, "{}", lit.at(32, 32));
}

#[test]
fn a_material_gets_the_stamps_pressed_on() {
    use SettingValue::{Choice, Number};
    let s = with(&[
        ("arrange", Choice(1)),
        ("spacing", Number(100.0)),
        ("size", Number(60.0)),
        ("size_vary", Number(0.0)),
        ("colour_vary", Number(0.0)),
        ("height_vary", Number(0.0)),
    ]);
    let n = size(64);
    let flat = Value::Material(Arc::new(MaterialMaps {
        colour: Arc::new(Image::filled(n, [0.2, 0.2, 0.2])),
        height: Arc::new(Image::filled(n, 0.5)),
        roughness: Arc::new(Image::filled(n, 0.7)),
        metal: Arc::new(Image::filled(n, 1.0)),
        normal: Arc::new(Image::filled(n, [0.0, 0.0, 1.0])),
        occlusion: Arc::new(Image::filled(n, 1.0)),
        curvature: Arc::new(Image::filled(n, 0.5)),
        opacity: None,
        translucency: None,
    }));
    let list = Stamps::one(dot(32, 32, [250, 200, 0])).unwrap();
    let o = stamps_run(Some(&list), &s, &[("material", &flat)], 64, 1, 2).unwrap();
    let m = out(&o, "material_out");
    let m = m.material().unwrap();
    // In the middle: the stamp's colour, standing up from the material.
    assert!(m.colour.at(32, 32)[0] > 0.95);
    assert!(m.height.at(32, 32) > 0.9);
    // Outside: the material as it was.
    assert_eq!(m.colour.at(1, 1), [0.2, 0.2, 0.2]);
    assert!((m.height.at(1, 1) - 0.5).abs() < 1e-6);
    // The stamp's edge bends the normals; shine and metal kept.
    let tilted = m.normal.pixels().iter().filter(|p| p[2] < 0.99).count();
    assert!(tilted > 20, "{tilted}");
    assert!(m.roughness.pixels().iter().all(|v| (*v - 0.7).abs() < 1e-6));
    assert!(m.metal.pixels().iter().all(|v| (*v - 1.0).abs() < 1e-6));
    // Without a material plugged in, there is none out.
    let o = stamps_run(Some(&list), &s, &[], 64, 1, 2).unwrap();
    assert!(o[0].is_none());
}

#[test]
fn shrinking_keeps_the_average_and_the_inside_is_measured() {
    let washer = washer(64, [255, 255, 255]);
    let big = shrunk(&washer, 64, 64);
    let small = shrunk(&washer, 8, 8);
    let mean = |v: &[[f32; 4]]| v.iter().map(|p| p[3]).sum::<f32>() / v.len() as f32;
    assert!((mean(&big) - mean(&small)).abs() < 0.03);
    let d = distances(&big, 64, 64);
    // The picture's edge and the hole's edge are near; the ring's middle
    // is farthest in.
    assert!(d[0] < 1.0);
    assert!(d[32 * 64 + 32] == 0.0);
    let ring = d[32 * 64 + 4];
    assert!(ring > 2.0, "{ring}");
}

#[test]
fn a_stamps_graph_runs_end_to_end_and_its_stamps_change_the_result() {
    let registry = registry();
    let mut g = Graph::new("Rivets");
    let add = |g: &mut Graph, kind: &str, x: f32| {
        let id = BoxId(g.next_id());
        g.apply(
            registry,
            &GraphEdit::AddBox {
                kind: kind.into(),
                at: Vec2::new(x, 0.0),
            },
        )
        .unwrap();
        id
    };
    let stamps = add(&mut g, STAMPS, 0.0);
    let made = add(&mut g, "material_from_height", 300.0);
    let out_box = add(&mut g, "material_out", 600.0);
    let wire = |g: &mut Graph, from: BoxId, fp: &str, to: BoxId, tp: &str| {
        g.apply(
            registry,
            &GraphEdit::Connect {
                from: PlugRef::new(from, fp),
                to: PlugRef::new(to, tp),
            },
        )
        .unwrap();
    };
    wire(&mut g, stamps, "out", made, "height");
    wire(&mut g, stamps, "colour_out", made, "colour");
    wire(&mut g, made, "material", out_box, "material");
    let mut e = Evaluator::with_threads(2);
    // Nothing brought in: the box says why.
    let first = e.run(registry, &g, size(32), &[stamps]);
    assert!(first.outcome(stamps).unwrap().is_err());
    let set = |g: &mut Graph, list: Stamps| {
        g.apply(
            registry,
            &GraphEdit::SetStamps {
                node: stamps,
                stamps: Some(Arc::new(list)),
            },
        )
        .unwrap();
    };
    set(&mut g, red_and_blue());
    let a = e.material(registry, &g, size(64)).unwrap();
    assert_eq!(e.run(registry, &g, size(64), &[stamps]).made(), 0);
    set(&mut g, red_and_blue().with_weight(0, 7));
    let b = e.material(registry, &g, size(64)).unwrap();
    assert_ne!(a.colour, b.colour);
    // Only a Stamps box holds stamps; copies of it keep them.
    let noise = add(&mut g, "noise", 0.0);
    let refused = g.apply(
        registry,
        &GraphEdit::SetStamps {
            node: noise,
            stamps: Some(Arc::new(red_and_blue())),
        },
    );
    assert!(refused.is_err());
    let copies = g.get(stamps).unwrap().copy_edits(BoxId(99));
    assert!(
        copies
            .iter()
            .any(|e| matches!(e, GraphEdit::SetStamps { stamps: Some(_), .. }))
    );
}

/// How long the box takes at 2048 pixels (run by hand: `cargo test -p
/// matgraph --release -- --ignored stamps_speed --nocapture`).
#[test]
#[ignore = "a timing, run by hand"]
fn stamps_speed() {
    use SettingValue::Number;
    let list = red_and_blue().with_added(washer(512, [9, 9, 9])).unwrap();
    let material = input(ValueKind::Material, size(2048));
    for (what, s) in [
        ("standard", Settings::new()),
        ("many small", with(&[("spacing", Number(1.0)), ("size", Number(1.5))])),
        ("big piles", with(&[("spacing", Number(5.0)), ("size", Number(40.0))])),
    ] {
        let start = std::time::Instant::now();
        stamps_run(Some(&list), &s, &[("material", &material)], 2048, 1, 2).unwrap();
        println!("{what}: {:?}", start.elapsed());
    }
}


