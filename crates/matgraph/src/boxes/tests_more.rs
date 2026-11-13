//! Milestone 10's boxes (decision 0272): Cracks, Chips, Scratches, Fill
//! the gaps, Brush strokes, Edge highlight, Slope blur and Directional
//! light. The checks every box gets (in range, seamless with standard
//! settings, seeds and cores, extreme settings, flat inputs) are in
//! `tests_each.rs`; here each box is tiled with other settings and
//! checked for what it says it does. The speed check is ignored: run it
//! with `cargo test -p matgraph --release -- --ignored more_box_speed
//! --nocapture`.

use super::tests_each::{assert_in_range, assert_seamless, input, registry, size, with};
use crate::image::{Grey, Image, Size};
use crate::registry::Job;
use crate::settings::{SettingValue, Settings};
use crate::value::Value;
use crate::{Evaluator, Graph, GraphEdit, PlugRef, ValueKind};
use glam::Vec2;
use std::sync::Arc;

use SettingValue::{Choice, Colour, Number, Switch};

/// The new boxes' ids.
const NEW: [&str; 8] = [
    "cracks",
    "chips",
    "scratches",
    "fill_gaps",
    "brush_strokes",
    "edge_highlight",
    "slope_blur",
    "directional_light",
];

/// Runs box `kind` with `inputs` by key (the others empty), every output
/// filled in.
fn run(kind: &str, n: u32, s: &Settings, inputs: &[(&str, Value)], cm: f32) -> Vec<Value> {
    let job_inputs = job_inputs(kind, inputs);
    let info = registry().get(kind).unwrap().info();
    let job = Job::new(info, size(n), 7, 2, s, job_inputs).with_real_size(cm);
    registry().get(kind).unwrap().run(&job).unwrap()
}

/// As [`run`], keeping the outputs left empty.
fn run_some(kind: &str, n: u32, s: &Settings, inputs: &[(&str, Value)]) -> Vec<Option<Value>> {
    let job_inputs = job_inputs(kind, inputs);
    let info = registry().get(kind).unwrap().info();
    let job = Job::new(info, size(n), 7, 2, s, job_inputs).with_real_size(100.0);
    registry().get(kind).unwrap().run_outputs(&job).unwrap()
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

fn grey(v: &Value) -> &Grey {
    v.grey().unwrap()
}

fn mean(g: &Grey) -> f32 {
    g.pixels().iter().sum::<f32>() / g.pixels().len() as f32
}

fn flat(n: u32, v: f32) -> Value {
    Value::Height(Arc::new(Image::filled(size(n), v)))
}

/// A round bump in the middle of an `n`-pixel texture, `radius` pixels
/// across, as a height (a dome) or a shape (a disc).
fn dome(n: u32, radius: f32) -> Grey {
    let c = n as f32 / 2.0;
    let pixels = (0..n * n)
        .map(|i| {
            let (x, y) = ((i % n) as f32 + 0.5 - c, (i / n) as f32 + 0.5 - c);
            let d = (x * x + y * y).sqrt() / radius;
            (1.0 - d * d).max(0.0).sqrt()
        })
        .collect();
    Image::from_pixels(size(n), pixels).unwrap()
}

#[test]
fn the_new_boxes_are_in_their_groups_with_words_and_what_comes_next() {
    use crate::Group;
    let wanted = [
        ("cracks", Group::Detail),
        ("chips", Group::Detail),
        ("scratches", Group::Detail),
        ("fill_gaps", Group::Detail),
        ("brush_strokes", Group::Colour),
        ("edge_highlight", Group::Colour),
        ("directional_light", Group::Colour),
        ("slope_blur", Group::Shape),
    ];
    for (id, group) in wanted {
        let info = registry().get(id).unwrap().info();
        assert_eq!(info.group, group, "{id}");
        assert!(!info.about.show().is_empty(), "{id}");
        assert!(!info.comes_next.is_empty(), "{id}");
        for next in &info.comes_next {
            assert!(registry().get(next).is_some(), "{id} → {next}");
        }
    }
    // Sizes are centimetres, so each follows the real-world size.
    for id in NEW {
        assert!(registry().get(id).unwrap().info().uses_real_size(), "{id}");
    }
}

#[test]
fn the_new_boxes_tile_seamlessly_with_other_settings_too() {
    let height = input(ValueKind::Height, size(64));
    let shape = input(ValueKind::Shape, size(64));
    let colour = input(ValueKind::Colour, size(64));
    let material = input(ValueKind::Material, size(64));
    let all = |extra: &[(&'static str, Value)]| {
        let mut v = vec![
            ("in", height.clone()),
            ("height", height.clone()),
            ("colour", colour.clone()),
            ("material", material.clone()),
        ];
        v.extend(extra.iter().cloned());
        v
    };
    let cases: Vec<(&str, Settings, Vec<(&str, Value)>)> = vec![
        (
            "cracks",
            with(&[
                ("size", Number(13.0)),
                ("wiggle", Number(1.0)),
                ("amount", Number(1.0)),
            ]),
            all(&[("where", shape.clone())]),
        ),
        (
            "cracks",
            with(&[("wiggle", Number(0.0)), ("width", Number(3.0))]),
            vec![],
        ),
        (
            "chips",
            with(&[
                ("amount", Number(1.0)),
                ("reach", Number(6.0)),
                ("size", Number(9.0)),
            ]),
            vec![("shape", shape.clone()), ("height", height.clone())],
        ),
        (
            "scratches",
            with(&[
                ("direction", Number(73.0)),
                ("direction_vary", Number(180.0)),
                ("length", Number(60.0)),
                ("width", Number(1.0)),
            ]),
            all(&[("where", shape.clone())]),
        ),
        (
            "fill_gaps",
            with(&[
                ("level", Number(0.7)),
                ("patchy", Number(1.0)),
                ("patch_size", Number(7.0)),
            ]),
            all(&[]),
        ),
        (
            "brush_strokes",
            with(&[
                ("length", Number(30.0)),
                ("width", Number(9.0)),
                ("direction_vary", Number(180.0)),
                ("bristles", Number(1.0)),
            ]),
            all(&[]),
        ),
        (
            "edge_highlight",
            with(&[("width", Number(8.0)), ("hollows", Number(1.0))]),
            all(&[]),
        ),
        (
            "slope_blur",
            with(&[
                ("distance", Number(10.0)),
                ("mode", Choice(1)),
                ("direction", Choice(1)),
            ]),
            all(&[]),
        ),
        (
            "slope_blur",
            with(&[("mode", Choice(2))]),
            all(&[("by", input(ValueKind::Shape, size(64)))]),
        ),
        (
            "directional_light",
            with(&[
                ("around", Number(17.0)),
                ("up", Number(12.0)),
                ("depth", Number(10.0)),
            ]),
            all(&[]),
        ),
        (
            "directional_light",
            with(&[("softness", Number(0.0))]),
            vec![("height", height.clone())],
        ),
    ];
    for (kind, settings, inputs) in cases {
        let out = run(kind, 64, &settings, &inputs, 100.0);
        assert_in_range(kind, &out);
        assert_seamless(kind, &out);
    }
}

#[test]
fn cracks_cut_lines_into_the_height_and_amount_nothing_leaves_it() {
    let height = input(ValueKind::Height, size(128));
    let out = run(
        "cracks",
        128,
        &Settings::new(),
        &[("in", height.clone())],
        100.0,
    );
    let (carved, lines) = (grey(&out[0]), grey(&out[1]));
    let cracked = lines.pixels().iter().filter(|v| **v > 0.5).count();
    let all = lines.pixels().len();
    // Thin lines: some of the texture, nowhere near all.
    assert!(
        cracked > all / 200 && cracked < all / 4,
        "{cracked} of {all}"
    );
    for (c, h) in carved.pixels().iter().zip(grey(&height).pixels()) {
        assert!(*c <= *h + 1e-6);
    }
    let none = run(
        "cracks",
        128,
        &with(&[("amount", Number(0.0))]),
        &[("in", height.clone())],
        100.0,
    );
    assert_eq!(&none[0], &height);
    assert!(grey(&none[1]).pixels().iter().all(|v| *v == 0.0));
    // Nothing plugged in: carved into white.
    let alone = run("cracks", 128, &Settings::new(), &[], 100.0);
    assert!(grey(&alone[0]).pixels().iter().any(|v| *v == 1.0));
    // Nothing where Where is black.
    let nowhere = run(
        "cracks",
        64,
        &Settings::new(),
        &[("where", flat(64, 0.0))],
        100.0,
    );
    assert!(grey(&nowhere[1]).pixels().iter().all(|v| *v == 0.0));
}

#[test]
fn smaller_pieces_make_more_cracks() {
    let count = |cm: f32| {
        let out = run(
            "cracks",
            128,
            &with(&[("size", Number(cm)), ("amount", Number(1.0))]),
            &[],
            100.0,
        );
        grey(&out[1]).pixels().iter().filter(|v| **v > 0.5).count()
    };
    assert!(
        count(10.0) > count(40.0) + 50,
        "{} {}",
        count(10.0),
        count(40.0)
    );
}

#[test]
fn chips_bite_only_into_the_shape_and_only_at_its_edges() {
    let shape = input(ValueKind::Shape, size(128));
    let s = with(&[("amount", Number(0.8)), ("reach", Number(2.0))]);
    let out = run("chips", 128, &s, &[("shape", shape.clone())], 100.0);
    let (chipped, cut, chips) = (grey(&out[0]), grey(&out[1]), grey(&out[2]));
    let original = grey(&shape);
    assert!(chips.pixels().iter().any(|v| *v > 0.9), "some chips");
    for i in 0..original.pixels().len() {
        let (o, c, k) = (original.pixels()[i], chipped.pixels()[i], chips.pixels()[i]);
        assert!(c <= o + 1e-6);
        assert!(cut.pixels()[i] <= o + 1e-6);
        if o <= 0.5 {
            assert_eq!(k, 0.0, "no chips outside the shape");
        }
    }
    // Deep inside a brick (far from any edge) is never chipped.
    let inside = super::pattern::distance_inside(original, 50.0, 1);
    for i in 0..chips.pixels().len() {
        let (x, y) = ((i as u32 % 128) as i64, (i as u32 / 128) as i64);
        if inside.at(x, y) > 10.0 {
            assert_eq!(chips.pixels()[i], 0.0);
        }
    }
    let none = run(
        "chips",
        128,
        &with(&[("amount", Number(0.0))]),
        &[("shape", shape.clone())],
        100.0,
    );
    assert_eq!(grey(&none[0]).pixels(), original.pixels());
}

#[test]
fn scratches_run_the_way_direction_says() {
    let s = |turn: f32| {
        with(&[
            ("direction", Number(turn)),
            ("direction_vary", Number(0.0)),
            ("width", Number(0.6)),
            ("length", Number(40.0)),
        ])
    };
    // How alike each pixel is with the one beside it, across and down.
    let along = |g: &Grey| {
        let n = i64::from(g.size());
        let (mut across, mut down) = (0.0, 0.0);
        for y in 0..n {
            for x in 0..n {
                let v = g.at(x, y);
                across += v * g.at(x + 1, y);
                down += v * g.at(x, y + 1);
            }
        }
        (across, down)
    };
    let flat_way = run("scratches", 128, &s(0.0), &[], 100.0);
    let (a, d) = along(grey(&flat_way[1]));
    assert!(a > d * 1.5, "across {a} down {d}");
    let upright = run("scratches", 128, &s(90.0), &[], 100.0);
    let (a, d) = along(grey(&upright[1]));
    assert!(d > a * 1.5, "across {a} down {d}");
    // Depth nothing leaves the height.
    let height = input(ValueKind::Height, size(64));
    let none = run(
        "scratches",
        64,
        &with(&[("depth", Number(0.0))]),
        &[("in", height.clone())],
        100.0,
    );
    assert_eq!(&none[0], &height);
}

#[test]
fn fill_the_gaps_fills_low_places_up_to_the_level() {
    let height = input(ValueKind::Height, size(64));
    let s = with(&[
        ("level", Number(0.4)),
        ("patchy", Number(0.0)),
        ("softness", Number(0.0)),
    ]);
    let colour = Value::Colour(Arc::new(Image::filled(size(64), [1.0, 0.0, 0.0])));
    let out = run_some(
        "fill_gaps",
        64,
        &s,
        &[("height", height.clone()), ("colour", colour)],
    );
    let (filled, gaps) = (
        grey(out[0].as_ref().unwrap()),
        grey(out[1].as_ref().unwrap()),
    );
    let colours = out[2].as_ref().unwrap().colour().unwrap();
    assert!(out[3].is_none(), "no material in, none out");
    let h = grey(&height);
    for i in 0..h.pixels().len() {
        let (hv, fv, gv) = (h.pixels()[i], filled.pixels()[i], gaps.pixels()[i]);
        assert!(fv >= hv - 1e-6, "never lower");
        if hv < 0.35 {
            assert!(gv > 0.99 && (fv - 0.4).abs() < 1e-3, "{hv} {gv} {fv}");
            assert!(colours.pixels()[i][1] > 0.3, "the moss's green");
        }
        if hv > 0.45 {
            assert_eq!(gv, 0.0);
            assert_eq!(colours.pixels()[i], [1.0, 0.0, 0.0]);
        }
    }
    // Level nothing: no gaps at all.
    let none = run_some(
        "fill_gaps",
        64,
        &with(&[("level", Number(0.0))]),
        &[("height", height.clone())],
    );
    assert!(
        grey(none[1].as_ref().unwrap())
            .pixels()
            .iter()
            .all(|v| *v == 0.0)
    );
    assert_eq!(none[0].as_ref().unwrap(), &height);
    // A material's filling is flat and rough.
    let material = input(ValueKind::Material, size(64));
    let s = with(&[
        ("level", Number(1.0)),
        ("softness", Number(0.0)),
        ("patchy", Number(0.0)),
    ]);
    let out = run_some(
        "fill_gaps",
        64,
        &s,
        &[("height", flat(64, 0.2)), ("material", material)],
    );
    let m = out[3].as_ref().unwrap().material().unwrap();
    assert!(m.normal.pixels().iter().all(|n| n[2] > 0.999));
    assert!(m.roughness.pixels().iter().all(|r| (r - 0.9).abs() < 1e-4));
}

#[test]
fn brush_strokes_with_everything_off_give_the_colour_back_and_flatten_detail_on() {
    let colour = input(ValueKind::Colour, size(64));
    let off = with(&[
        ("keep_detail", Number(1.0)),
        ("colour_vary", Number(0.0)),
        ("bristles", Number(0.0)),
    ]);
    let out = run_some("brush_strokes", 64, &off, &[("colour", colour.clone())]);
    let (a, b) = (
        out[0].as_ref().unwrap().colour().unwrap(),
        colour.colour().unwrap(),
    );
    for (p, q) in a.pixels().iter().zip(b.pixels()) {
        for i in 0..3 {
            assert!((p[i] - q[i]).abs() < 1e-5);
        }
    }
    assert!(out[1].is_none());
    // Flat dabs: neighbouring pixels differ less than in the colour.
    let flat_dabs = with(&[("keep_detail", Number(0.0)), ("colour_vary", Number(0.0))]);
    let out = run_some(
        "brush_strokes",
        64,
        &flat_dabs,
        &[("colour", colour.clone())],
    );
    let busy = |c: &crate::image::Rgb| {
        let n = i64::from(c.size());
        let mut total = 0.0;
        for y in 0..n {
            for x in 0..n {
                total += (c.at(x, y)[0] - c.at(x + 1, y)[0]).abs();
            }
        }
        total
    };
    let painted = out[0].as_ref().unwrap().colour().unwrap();
    // The input noise is smooth, so dabs' edges add some steps back.
    assert!(
        busy(painted) < busy(b) * 1.5,
        "{} {}",
        busy(painted),
        busy(b)
    );
    // Its height is paint where strokes are.
    assert!(mean(grey(out[2].as_ref().unwrap())) > 0.3);
}

#[test]
fn edge_highlight_lightens_the_tops_of_edges() {
    // A bevelled square: its rim is an edge, the middle of the floor
    // around it is plain.
    let n = 64;
    let shape = {
        let pixels = (0..n * n)
            .map(|i| {
                let (x, y) = (i % n, i / n);
                if (16..48).contains(&x) && (16..48).contains(&y) {
                    1.0
                } else {
                    0.0
                }
            })
            .collect();
        Value::Height(Arc::new(Image::from_pixels(size(n), pixels).unwrap()))
    };
    let grey_colour = Value::Colour(Arc::new(Image::filled(size(n), [0.4; 3])));
    let s = with(&[("width", Number(6.0)), ("hollows", Number(1.0))]);
    let out = run_some(
        "edge_highlight",
        n,
        &s,
        &[("height", shape), ("colour", grey_colour.clone())],
    );
    let c = out[0].as_ref().unwrap().colour().unwrap();
    let edges = grey(out[2].as_ref().unwrap());
    // Just inside the rim: lighter. Just outside: darker. Far off: as it was.
    assert!(c.at(17, 32)[0] > 0.5, "{:?}", c.at(17, 32));
    assert!(c.at(14, 32)[0] < 0.4, "{:?}", c.at(14, 32));
    assert!((c.at(1, 1)[0] - 0.4).abs() < 1e-4);
    assert!(edges.at(17, 32) > 0.5);
    assert_eq!(edges.at(14, 32), 0.0);
    // Nothing asked: the colour as it was.
    let none = with(&[("amount", Number(0.0)), ("hollows", Number(0.0))]);
    let out = run_some(
        "edge_highlight",
        n,
        &none,
        &[
            ("height", input(ValueKind::Height, size(n))),
            ("colour", grey_colour.clone()),
        ],
    );
    assert_eq!(out[0].as_ref().unwrap(), &grey_colour);
}

#[test]
fn slope_blur_at_no_distance_gives_the_input_and_wears_below_builds() {
    let height = input(ValueKind::Height, size(64));
    let by = input(ValueKind::Shape, size(64));
    let still = run_some(
        "slope_blur",
        64,
        &with(&[("distance", Number(0.0))]),
        &[("in", height.clone()), ("by", by.clone())],
    );
    let (a, b) = (grey(still[0].as_ref().unwrap()), grey(&height));
    for (p, q) in a.pixels().iter().zip(b.pixels()) {
        assert!((p - q).abs() < 1e-5);
    }
    assert!(still[1].is_none() && still[2].is_none());
    let mode = |m: u32| {
        let out = run_some(
            "slope_blur",
            64,
            &with(&[("mode", Choice(m)), ("distance", Number(8.0))]),
            &[("in", height.clone())],
        );
        out[0].as_ref().unwrap().grey().unwrap().clone()
    };
    let (smear, wear, build) = (mode(0), mode(1), mode(2));
    for i in 0..smear.pixels().len() {
        assert!(wear.pixels()[i] <= smear.pixels()[i] + 1e-5);
        assert!(smear.pixels()[i] <= build.pixels()[i] + 1e-5);
    }
    assert!(mean(&wear) < mean(&build));
    // A colour and a material are smeared too.
    let out = run_some(
        "slope_blur",
        64,
        &Settings::new(),
        &[
            ("colour", input(ValueKind::Colour, size(64))),
            ("material", input(ValueKind::Material, size(64))),
        ],
    );
    assert!(out[0].is_none() && out[1].is_some() && out[2].is_some());
}

#[test]
fn slope_blur_with_nothing_to_blur_says_why() {
    let info = registry().get("slope_blur").unwrap().info();
    let inputs = info
        .inputs
        .iter()
        .map(|p| (p.key == "by").then(|| input(ValueKind::Height, size(16))))
        .collect();
    let job = Job::new(info, size(16), 1, 1, &Settings::new(), inputs);
    let why = registry()
        .get("slope_blur")
        .unwrap()
        .run_outputs(&job)
        .unwrap_err();
    assert!(why.reason.contains("nothing plugged in"), "{}", why.reason);
}

#[test]
fn slope_blur_in_a_graph_with_nothing_plugged_in_names_each_kind_once() {
    let mut g = Graph::new("Slope");
    let id = crate::BoxId(g.next_id());
    g.apply(
        registry(),
        &GraphEdit::AddBox {
            kind: "slope_blur".into(),
            at: Vec2::ZERO,
        },
    )
    .unwrap();
    let run = Evaluator::with_threads(1).run(registry(), &g, size(16), &[id]);
    let Some(Err(why)) = run.outcome(id) else {
        panic!("Slope blur ran with nothing plugged in")
    };
    assert_eq!(why.reason.matches("a height").count(), 1, "{}", why.reason);
}

#[test]
fn directional_light_lights_the_side_facing_the_sun() {
    let n = 64;
    let bump = Value::Height(Arc::new(dome(n, 20.0)));
    // The sun from the right, low.
    let s = with(&[
        ("around", Number(0.0)),
        ("up", Number(30.0)),
        ("depth", Number(20.0)),
    ]);
    let out = run_some("directional_light", n, &s, &[("height", bump.clone())]);
    let light = grey(out[2].as_ref().unwrap());
    assert!(
        light.at(44, 32) > light.at(20, 32) + 0.3,
        "{} {}",
        light.at(44, 32),
        light.at(20, 32)
    );
    // From the top of the texture: the top is lit.
    let s = with(&[
        ("around", Number(90.0)),
        ("up", Number(30.0)),
        ("depth", Number(20.0)),
    ]);
    let out = run_some("directional_light", n, &s, &[("height", bump.clone())]);
    let light = grey(out[2].as_ref().unwrap());
    assert!(light.at(32, 20) > light.at(32, 44) + 0.3);
    // No colour plugged in: the light and shade colours alone, warm
    // where lit and cool in the shade.
    let tone = out[0].as_ref().unwrap().colour().unwrap();
    let (lit_side, shade_side) = (tone.at(32, 20), tone.at(32, 48));
    assert!(lit_side[0] > lit_side[2] && shade_side[2] > shade_side[0]);
}

#[test]
fn directional_light_on_flat_ground_matches_the_painters_helper() {
    // Flat ground faces straight up: lit as the painter's light and shade
    // lights a surface facing up, with the same maths.
    for (up, softness) in [(50.0, 0.5), (10.0, 0.0), (80.0, 1.0)] {
        let s = with(&[
            ("up", Number(up)),
            ("softness", Number(softness)),
            ("shadows", Switch(false)),
        ]);
        let out = run_some("directional_light", 16, &s, &[("height", flat(16, 0.5))]);
        let light = grey(out[2].as_ref().unwrap());
        let sun = crate::light::sun(240.0, up);
        let want = crate::light::lit(sun[2], softness);
        assert!(
            light.pixels().iter().all(|v| (v - want).abs() < 1e-5),
            "{up}"
        );
    }
}

#[test]
fn directional_light_casts_shadows_away_from_the_sun() {
    let n = 64;
    // A tall block in the middle, the sun low from the right.
    let block = {
        let pixels = (0..n * n)
            .map(|i| {
                let (x, y) = (i % n, i / n);
                if (28..36).contains(&x) && (28..36).contains(&y) {
                    1.0
                } else {
                    0.0
                }
            })
            .collect();
        Value::Height(Arc::new(Image::from_pixels(size(n), pixels).unwrap()))
    };
    let s = |shadows: bool| {
        with(&[
            ("around", Number(0.0)),
            ("up", Number(20.0)),
            ("depth", Number(20.0)),
            ("shadows", Switch(shadows)),
        ])
    };
    let on = run_some(
        "directional_light",
        n,
        &s(true),
        &[("height", block.clone())],
    );
    let off = run_some(
        "directional_light",
        n,
        &s(false),
        &[("height", block.clone())],
    );
    let (on, off) = (
        grey(on[2].as_ref().unwrap()),
        grey(off[2].as_ref().unwrap()),
    );
    // Left of the block (away from the sun) is in shadow; right isn't.
    assert!(
        on.at(24, 32) < off.at(24, 32) - 0.3,
        "{} {}",
        on.at(24, 32),
        off.at(24, 32)
    );
    assert!((on.at(40, 32) - off.at(40, 32)).abs() < 1e-5);
}

#[test]
fn a_material_keeps_its_other_maps_through_the_colour_boxes() {
    let material = input(ValueKind::Material, size(32));
    let height = input(ValueKind::Height, size(32));
    for kind in ["edge_highlight", "directional_light", "brush_strokes"] {
        let mut inputs = vec![("material", material.clone())];
        if kind != "brush_strokes" {
            inputs.push(("height", height.clone()));
        }
        let out = run_some(kind, 32, &Settings::new(), &inputs);
        let (a, b) = (
            out[1].as_ref().unwrap().material().unwrap(),
            material.material().unwrap(),
        );
        assert_eq!(a.height, b.height, "{kind}");
        assert_eq!(a.normal, b.normal, "{kind}");
        assert_eq!(a.roughness, b.roughness, "{kind}");
        assert_ne!(a.colour, b.colour, "{kind}");
    }
}

/// Bricks → Chips → Cracks → Scratches → Fill the gaps → Slope blur →
/// Material from height; its colour by Gradient map, Brush strokes,
/// Edge highlight and Directional light.
fn all_new_graph() -> (Graph, Vec<crate::BoxId>) {
    let mut g = Graph::new("All new");
    let r = registry();
    let add = |g: &mut Graph, kind: &str| {
        let id = crate::BoxId(g.next_id());
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
        "bricks",
        "chips",
        "cracks",
        "scratches",
        "fill_gaps",
        "slope_blur",
        "gradient_map",
        "brush_strokes",
        "edge_highlight",
        "directional_light",
        "material_from_height",
        "material_out",
    ]
    .iter()
    .map(|k| add(&mut g, k))
    .collect();
    let [
        bricks,
        chips,
        cracks,
        scratches,
        gaps,
        slope,
        map,
        strokes,
        edges,
        light,
        mfh,
        out,
    ] = ids[..]
    else {
        unreachable!()
    };
    for (from, f, to, t) in [
        (bricks, "shape", chips, "shape"),
        (chips, "out", cracks, "in"),
        (cracks, "out", scratches, "in"),
        (scratches, "out", gaps, "height"),
        (gaps, "out", slope, "in"),
        (slope, "out", map, "in"),
        (map, "colour", strokes, "colour"),
        (strokes, "colour_out", edges, "colour"),
        (slope, "out", edges, "height"),
        (edges, "colour_out", light, "colour"),
        (slope, "out", light, "height"),
        (slope, "out", mfh, "height"),
        (light, "colour_out", mfh, "colour"),
        (mfh, "material", out, "material"),
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
    (g, ids)
}

#[test]
fn every_new_box_in_a_graph_makes_a_seamless_material() {
    let (g, _) = all_new_graph();
    let r = registry();
    let m = Evaluator::new().material(r, &g, size(64)).unwrap();
    let value = Value::Material(m);
    assert_in_range("graph", std::slice::from_ref(&value));
    // The app's own seam check (by averages): a crack crossing the join
    // can make the single biggest step sit on it.
    let check = crate::SeamCheck::of(&value);
    assert!(!check.shows(), "{}", check.describe());
}

#[test]
fn the_filling_colour_lands_in_the_gaps() {
    let s = with(&[
        ("fill_colour", Colour([0.0, 0.0, 1.0])),
        ("level", Number(1.0)),
        ("softness", Number(0.0)),
        ("colour_vary", Number(0.0)),
    ]);
    let out = run_some(
        "fill_gaps",
        16,
        &s,
        &[
            ("height", flat(16, 0.0)),
            (
                "colour",
                Value::Colour(Arc::new(Image::filled(size(16), [1.0; 3]))),
            ),
        ],
    );
    let c = out[2].as_ref().unwrap().colour().unwrap();
    assert!(c.pixels().iter().all(|p| p[2] > 0.99 && p[0] < 0.01));
}

#[test]
#[ignore = "speed check: run by hand"]
fn more_box_speed() {
    let threads = crate::parallel::cores();
    let sz: Size = size(2048);
    for kind in NEW {
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
            println!("{kind:<18} {:>8.1} ms at 2048", took.as_secs_f64() * 1000.0);
        }
        assert!(took.as_secs() < 60, "{kind} took {took:?}");
    }
}
