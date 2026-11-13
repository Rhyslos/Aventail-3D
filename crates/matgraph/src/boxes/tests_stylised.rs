//! Milestone 23's stylised light boxes (decision 0286): Toon bands,
//! Painted highlight and Colour steps. The checks every box gets (in
//! range, seamless with standard settings, seeds and cores, extreme
//! settings, flat inputs) are in `tests_each.rs`; here each box is tiled
//! with other settings and checked for what it says it does: how many
//! bands and steps, the light's direction, palettes, a material's other
//! maps kept, and a stylised graph end to end. The speed check is
//! ignored: run it with `cargo test -p matgraph --release -- --ignored
//! stylised_speed --nocapture`.

use super::tests_each::{
    assert_in_range, assert_seamless, input, registry, size, test_palettes, with,
};
use crate::image::{Grey, Image, Rgb};
use crate::registry::Job;
use crate::settings::{SettingValue, Settings};
use crate::value::Value;
use crate::{Evaluator, Graph, GraphEdit, PlugRef, ValueKind};
use glam::Vec2;
use std::sync::Arc;

use SettingValue::{Choice, Colour, Number, Palette, Switch, Whole};

/// The new boxes' ids.
const NEW: [&str; 3] = ["toon_bands", "painted_highlight", "colour_steps"];

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

/// Runs box `kind` at `n` pixels with `inputs` by key (the others
/// empty), keeping the outputs it leaves empty; or why it can't run.
fn try_run(
    kind: &str,
    n: u32,
    s: &Settings,
    inputs: &[(&str, Value)],
) -> Result<Vec<Option<Value>>, crate::Refusal> {
    let info = registry().get(kind).unwrap().info();
    let job = Job::new(info, size(n), 7, 2, s, job_inputs(kind, inputs))
        .with_real_size(100.0)
        .with_palettes(test_palettes());
    registry().get(kind).unwrap().run_outputs(&job)
}

fn run(kind: &str, n: u32, s: &Settings, inputs: &[(&str, Value)]) -> Vec<Option<Value>> {
    try_run(kind, n, s, inputs).unwrap()
}

fn grey(v: &Option<Value>) -> &Grey {
    v.as_ref().unwrap().grey().unwrap()
}

fn colour(v: &Option<Value>) -> &Rgb {
    v.as_ref().unwrap().colour().unwrap()
}

/// A round bump in the middle of an `n`-pixel texture, `radius` pixels
/// across.
fn dome(n: u32, radius: f32) -> Value {
    let c = n as f32 / 2.0;
    let pixels = (0..n * n)
        .map(|i| {
            let (x, y) = ((i % n) as f32 + 0.5 - c, (i / n) as f32 + 0.5 - c);
            let d = (x * x + y * y).sqrt() / radius;
            (1.0 - d * d).max(0.0).sqrt()
        })
        .collect();
    Value::Height(Arc::new(Image::from_pixels(size(n), pixels).unwrap()))
}

/// A raised square, flat on top, from `lo` to `hi` pixels each way.
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
    Value::Height(Arc::new(Image::from_pixels(size(n), pixels).unwrap()))
}

fn plain_colour(n: u32, c: [f32; 3]) -> Value {
    Value::Colour(Arc::new(Image::filled(size(n), c)))
}

/// A colour going through every hue and lightness across and down.
fn rainbow(n: u32) -> Value {
    let pixels = (0..n * n)
        .map(|i| {
            let (u, v) = ((i % n) as f32 / n as f32, (i / n) as f32 / n as f32);
            let h = u * 6.0;
            let k = |o: f32| ((h + o) % 6.0 - 3.0).abs().clamp(1.0, 2.0) - 1.0;
            [k(0.0), k(4.0), k(2.0)].map(|c| c * v + (1.0 - v) * 0.5 * c)
        })
        .collect();
    Value::Colour(Arc::new(Image::from_pixels(size(n), pixels).unwrap()))
}

fn distinct<T: PartialEq + Copy>(items: impl IntoIterator<Item = T>) -> Vec<T> {
    let mut seen: Vec<T> = Vec::new();
    for i in items {
        if !seen.contains(&i) {
            seen.push(i);
        }
    }
    seen
}

#[test]
fn the_new_boxes_are_colour_boxes_with_words_and_what_comes_next() {
    for id in NEW {
        let kind = registry().get(id).unwrap();
        let info = kind.info();
        assert_eq!(info.group, crate::Group::Colour, "{id}");
        assert!(!info.about.show().is_empty(), "{id}");
        assert!(kind.needs_an_input(), "{id}");
        assert!(!info.comes_next.is_empty(), "{id}");
        for next in &info.comes_next {
            assert!(registry().get(next).is_some(), "{id} → {next}");
        }
        // They hold no randomness: nothing for the dice to roll.
        assert!(!info.uses_seed(), "{id}");
    }
    // Toon bands and Painted highlight read sizes in centimetres.
    assert!(
        registry()
            .get("toon_bands")
            .unwrap()
            .info()
            .uses_real_size()
    );
    assert!(
        registry()
            .get("painted_highlight")
            .unwrap()
            .info()
            .uses_real_size()
    );
    // Toon bands and Colour steps can keep to a palette.
    assert!(registry().get("toon_bands").unwrap().uses_palettes());
    assert!(registry().get("colour_steps").unwrap().uses_palettes());
}

#[test]
fn toon_bands_give_exactly_the_bands_asked_for() {
    let n = 64;
    for bands in 2..=6 {
        let s = with(&[
            ("bands", Whole(bands)),
            ("softness", Number(0.0)),
            ("shadows", Switch(false)),
            ("up", Number(30.0)),
            ("depth", Number(40.0)),
        ]);
        let out = run("toon_bands", n, &s, &[("height", dome(n, 28.0))]);
        let levels = distinct(grey(&out[2]).pixels().iter().map(|v| v.to_bits()));
        assert_eq!(levels.len(), bands as usize, "{bands}");
        // The tones too: one flat colour per band.
        let tones = distinct(colour(&out[0]).pixels().iter().copied());
        assert_eq!(tones.len(), bands as usize, "{bands}");
    }
    // Soft lines add in-between tones, only near the lines.
    let soft = run(
        "toon_bands",
        n,
        &with(&[("softness", Number(1.0)), ("depth", Number(40.0))]),
        &[("height", dome(n, 28.0))],
    );
    assert!(distinct(grey(&soft[2]).pixels().iter().map(|v| v.to_bits())).len() > 3);
}

#[test]
fn toon_bands_light_the_side_facing_the_sun() {
    let n = 64;
    for (around, lit, shaded) in [
        (0.0, (46, 32), (18, 32)),
        (90.0, (32, 18), (32, 46)),
        (180.0, (18, 32), (46, 32)),
        (270.0, (32, 46), (32, 18)),
    ] {
        let s = with(&[
            ("around", Number(around)),
            ("up", Number(30.0)),
            ("depth", Number(40.0)),
            ("softness", Number(0.0)),
        ]);
        let out = run("toon_bands", n, &s, &[("height", dome(n, 24.0))]);
        let b = grey(&out[2]);
        assert_eq!(b.at(lit.0, lit.1), 1.0, "{around}");
        assert_eq!(b.at(shaded.0, shaded.1), 0.0, "{around}");
    }
}

#[test]
fn toon_bands_on_flat_ground_follow_the_painters_light() {
    // Flat ground faces straight up: its band is the painter's light at
    // its softest, cut into bands (crate::light, decision 0272).
    for (up, bands) in [(50.0, 3), (10.0, 2), (80.0, 5), (25.0, 4)] {
        let s = with(&[
            ("up", Number(up)),
            ("bands", Whole(bands)),
            ("softness", Number(0.0)),
        ]);
        let flat = Value::Height(Arc::new(Image::filled(size(16), 0.5)));
        let out = run("toon_bands", 16, &s, &[("height", flat)]);
        let sun = crate::light::sun(240.0, up);
        let want = super::toon_bands::banded(crate::light::lit(sun[2], 1.0), bands as u32, 0.0);
        assert!(grey(&out[2]).pixels().iter().all(|v| *v == want), "{up}");
    }
}

#[test]
fn toon_bands_shade_with_their_colours_and_cast_shadows() {
    let n = 64;
    let white = plain_colour(n, [1.0; 3]);
    let s = |shadows: bool| {
        with(&[
            ("around", Number(0.0)),
            ("up", Number(20.0)),
            ("depth", Number(20.0)),
            ("softness", Number(0.0)),
            ("shadows", Switch(shadows)),
            ("shade_colour", Colour([0.2, 0.1, 0.5])),
            ("light_colour", Colour([1.0, 1.0, 1.0])),
        ])
    };
    let on = run(
        "toon_bands",
        n,
        &s(true),
        &[("height", block(n, 28, 36)), ("colour", white.clone())],
    );
    let off = run(
        "toon_bands",
        n,
        &s(false),
        &[("height", block(n, 28, 36)), ("colour", white)],
    );
    // Left of the block, away from the sun, falls into the shade colour.
    let near = |a: [f32; 3], b: [f32; 3]| (0..3).all(|i| (a[i] - b[i]).abs() < 1e-5);
    assert!(near(colour(&on[0]).at(24, 32), [0.2, 0.1, 0.5]));
    assert!(!near(colour(&off[0]).at(24, 32), [0.2, 0.1, 0.5]));
    // Right of it is untouched.
    assert_eq!(colour(&on[0]).at(44, 32), colour(&off[0]).at(44, 32));
}

#[test]
fn toon_bands_use_a_materials_own_normals_without_a_height() {
    let n = 64;
    let material = {
        let info = registry().get("material_from_height").unwrap().info();
        let s = with(&[("depth", Number(30.0))]);
        let job = Job::new(info, size(n), 3, 1, &s, vec![Some(dome(n, 24.0)), None]);
        registry()
            .get("material_from_height")
            .unwrap()
            .run(&job)
            .unwrap()
            .remove(0)
    };
    let s = with(&[
        ("around", Number(0.0)),
        ("up", Number(30.0)),
        ("shadows", Switch(false)),
    ]);
    let out = run("toon_bands", n, &s, &[("material", material.clone())]);
    let b = grey(&out[2]);
    assert!(
        b.at(46, 32) > b.at(18, 32),
        "{} {}",
        b.at(46, 32),
        b.at(18, 32)
    );
    let (m, before) = (
        out[1].as_ref().unwrap().material().unwrap(),
        material.material().unwrap(),
    );
    assert_eq!(m.normal, before.normal);
    assert_eq!(m.height, before.height);
    assert_eq!(m.roughness, before.roughness);
    assert_ne!(m.colour, before.colour);
}

#[test]
fn toon_bands_and_painted_highlight_need_a_height_or_a_material() {
    for kind in ["toon_bands", "painted_highlight"] {
        let why = try_run(
            kind,
            16,
            &Settings::new(),
            &[("colour", plain_colour(16, [0.5; 3]))],
        )
        .unwrap_err();
        assert!(why.reason.contains("Height or Material"), "{}", why.reason);
    }
}

#[test]
fn toon_bands_keep_to_a_palette() {
    let n = 64;
    let swatches: Vec<[f32; 3]> = test_palettes()
        .iter()
        .next()
        .unwrap()
        .colours
        .iter()
        .map(|c| c.map(|v| f32::from(v) / 255.0))
        .collect();
    let s = with(&[
        ("colours", Choice(1)),
        ("bands", Whole(4)),
        ("depth", Number(30.0)),
    ]);
    // With a colour: shaded, then snapped.
    let out = run(
        "toon_bands",
        n,
        &s,
        &[("height", dome(n, 24.0)), ("colour", rainbow(n))],
    );
    assert!(
        colour(&out[0])
            .pixels()
            .iter()
            .all(|c| swatches.contains(c))
    );
    // Without: each band one swatch, dark to light.
    let alone = run("toon_bands", n, &s, &[("height", dome(n, 24.0))]);
    let used = distinct(colour(&alone[0]).pixels().iter().copied());
    assert!(used.iter().all(|c| swatches.contains(c)));
    assert!(used.len() >= 2);
    // A palette the project hasn't got is refused with ways round it.
    let gone = try_run(
        "toon_bands",
        16,
        &with(&[("colours", Choice(1)), ("palette", Palette("Stone".into()))]),
        &[("height", dome(16, 6.0))],
    )
    .unwrap_err();
    assert!(gone.reason.contains("Stone"));
    assert!(!gone.fixes.is_empty());
    // Shade and light colours never need one.
    let job = Job::new(
        registry().get("toon_bands").unwrap().info(),
        size(16),
        1,
        1,
        &Settings::new(),
        job_inputs("toon_bands", &[("height", dome(16, 6.0))]),
    );
    assert!(registry().get("toon_bands").unwrap().run(&job).is_ok());
}

#[test]
fn painted_highlight_lights_only_the_edges_facing_the_light() {
    let n = 64;
    let raised = block(n, 16, 48);
    let paint = plain_colour(n, [0.2; 3]);
    for (around, lit, dark) in [
        (90.0, (32, 17), (32, 46)),
        (0.0, (46, 32), (17, 32)),
        (180.0, (17, 32), (46, 32)),
        (270.0, (32, 46), (32, 17)),
    ] {
        let s = with(&[("around", Number(around)), ("softness", Number(0.0))]);
        let out = run(
            "painted_highlight",
            n,
            &s,
            &[("height", raised.clone()), ("colour", paint.clone())],
        );
        let h = grey(&out[2]);
        assert!(h.at(lit.0, lit.1) > 0.9, "{around}: {}", h.at(lit.0, lit.1));
        assert_eq!(h.at(dark.0, dark.1), 0.0, "{around}");
        // The middle of the flat top and the ground around: no stroke.
        assert_eq!(h.at(32, 32), 0.0, "{around}");
        assert_eq!(h.at(4, 4), 0.0, "{around}");
        // The colour is painted toward the highlight colour there.
        let c = colour(&out[0]);
        assert!(c.at(lit.0, lit.1)[0] > 0.6);
        assert_eq!(c.at(dark.0, dark.1), [0.2; 3]);
    }
    // Spread 1 reaches every edge.
    let all = run(
        "painted_highlight",
        n,
        &with(&[("spread", Number(1.0))]),
        &[("height", raised.clone())],
    );
    assert!(grey(&all[2]).at(32, 46) > 0.5);
    // Amount 0 leaves the colour.
    let none = run(
        "painted_highlight",
        n,
        &with(&[("amount", Number(0.0))]),
        &[("height", raised), ("colour", paint.clone())],
    );
    assert_eq!(colour(&none[0]), paint.colour().unwrap().as_ref());
}

#[test]
fn painted_highlight_wider_reaches_further_in() {
    let n = 128;
    let raised = block(n, 32, 96);
    let width = |cm: f32| {
        let out = run(
            "painted_highlight",
            n,
            &with(&[("width", Number(cm)), ("softness", Number(0.0))]),
            &[("height", raised.clone())],
        );
        (32..64).filter(|y| grey(&out[2]).at(64, *y) > 0.5).count()
    };
    assert!(width(4.0) > width(1.0), "{} {}", width(4.0), width(1.0));
}

#[test]
fn colour_steps_give_that_many_steps_per_channel() {
    let n = 64;
    for steps in [2, 3, 5, 8] {
        let s = with(&[("steps", Whole(steps)), ("mode", Choice(0))]);
        let out = run("colour_steps", n, &s, &[("colour", rainbow(n))]);
        let c = colour(&out[1]);
        for ch in 0..3 {
            let values = distinct(c.pixels().iter().map(|p| p[ch].to_bits()));
            assert!(values.len() <= steps as usize, "{steps} {ch}");
            assert!(values.len() >= 2, "{steps} {ch}");
        }
        // A height in terraces.
        let ramp = Value::Height(Arc::new(
            Image::from_pixels(
                size(n),
                (0..n * n)
                    .map(|i| (i % n) as f32 / (n - 1) as f32)
                    .collect(),
            )
            .unwrap(),
        ));
        let out = run("colour_steps", n, &s, &[("in", ramp)]);
        let levels = distinct(grey(&out[0]).pixels().iter().map(|v| v.to_bits()));
        assert_eq!(levels.len(), steps as usize, "{steps}");
    }
}

#[test]
fn colour_steps_in_lightness_keep_each_colours_hue() {
    let n = 64;
    let s = with(&[("steps", Whole(4))]);
    let out = run("colour_steps", n, &s, &[("colour", rainbow(n))]);
    let c = colour(&out[1]);
    // Every lightness is the middle of one of four bands: strong colours
    // too light to show are made less strong, never lighter or darker.
    let middles = [0.125, 0.375, 0.625, 0.875];
    let mut used = [false; 4];
    for p in c.pixels() {
        let l = super::palette_map::oklab(*p)[0];
        let band = middles.iter().position(|m| (l - m).abs() < 0.01);
        let Some(band) = band else {
            panic!("lightness {l} of {p:?} isn't a band's middle")
        };
        used[band] = true;
    }
    assert!(used.iter().filter(|u| **u).count() >= 3, "{used:?}");
    // A red stays reddish.
    let red = run(
        "colour_steps",
        16,
        &s,
        &[("colour", plain_colour(16, [0.8, 0.15, 0.1]))],
    );
    let r = colour(&red[1]).at(3, 3);
    assert!(r[0] > r[1] + 0.2 && r[0] > r[2] + 0.2, "{r:?}");
}

#[test]
fn colour_steps_snap_to_a_palette_and_dither_still_tiles() {
    let n = 64;
    let swatches: Vec<[f32; 3]> = test_palettes()
        .iter()
        .next()
        .unwrap()
        .colours
        .iter()
        .map(|c| c.map(|v| f32::from(v) / 255.0))
        .collect();
    let s = with(&[("snap", Switch(true)), ("dither", Number(1.0))]);
    let out = run("colour_steps", n, &s, &[("colour", rainbow(n))]);
    assert!(
        colour(&out[1])
            .pixels()
            .iter()
            .all(|c| swatches.contains(c))
    );
    let seamless_in = input(ValueKind::Colour, size(n));
    let out = run("colour_steps", n, &s, &[("colour", seamless_in)]);
    assert!(
        colour(&out[1])
            .pixels()
            .iter()
            .all(|c| swatches.contains(c))
    );
    assert_seamless(
        "colour steps, snapped and dithered",
        &[out[1].clone().unwrap()],
    );
    // Snap off: no palette needed.
    let job = Job::new(
        registry().get("colour_steps").unwrap().info(),
        size(16),
        1,
        1,
        &Settings::new(),
        job_inputs("colour_steps", &[("colour", rainbow(16))]),
    );
    assert!(registry().get("colour_steps").unwrap().run(&job).is_ok());
}

#[test]
fn a_material_keeps_its_other_maps_through_the_stylised_boxes() {
    let material = input(ValueKind::Material, size(32));
    for kind in NEW {
        let out = run(
            kind,
            32,
            &Settings::new(),
            &[("material", material.clone())],
        );
        let (a, b) = (
            out.iter().flatten().find_map(Value::material).unwrap(),
            material.material().unwrap(),
        );
        assert_eq!(a.height, b.height, "{kind}");
        assert_eq!(a.normal, b.normal, "{kind}");
        assert_eq!(a.roughness, b.roughness, "{kind}");
        assert_eq!(a.opacity, b.opacity, "{kind}");
        assert_ne!(a.colour, b.colour, "{kind}");
    }
}

#[test]
fn the_new_boxes_tile_seamlessly_with_other_settings_too() {
    let n = 64;
    let height = input(ValueKind::Height, size(n));
    let colour_in = input(ValueKind::Colour, size(n));
    let material = input(ValueKind::Material, size(n));
    let all: Vec<(&str, Value)> = vec![
        ("in", height.clone()),
        ("height", height.clone()),
        ("colour", colour_in.clone()),
        ("material", material.clone()),
    ];
    let cases: Vec<(&str, Settings)> = vec![
        (
            "toon_bands",
            with(&[("bands", Whole(5)), ("around", Number(33.0))]),
        ),
        (
            "toon_bands",
            with(&[("colours", Choice(1)), ("shadows", Switch(false))]),
        ),
        (
            "painted_highlight",
            with(&[("spread", Number(1.0)), ("width", Number(4.0))]),
        ),
        (
            "painted_highlight",
            with(&[("around", Number(200.0)), ("softness", Number(1.0))]),
        ),
        (
            "colour_steps",
            with(&[("mode", Choice(0)), ("steps", Whole(3))]),
        ),
        (
            "colour_steps",
            with(&[("snap", Switch(true)), ("dither", Number(0.5))]),
        ),
    ];
    for (kind, s) in cases {
        let out: Vec<Value> = run(kind, n, &s, &all).into_iter().flatten().collect();
        assert_in_range(kind, &out);
        assert_seamless(kind, &out);
    }
}

#[test]
fn the_same_settings_give_the_same_pictures_every_time() {
    let n = 64;
    let height = input(ValueKind::Height, size(n));
    let colour_in = input(ValueKind::Colour, size(n));
    for kind in NEW {
        let inputs = [
            ("in", height.clone()),
            ("height", height.clone()),
            ("colour", colour_in.clone()),
        ];
        let a = run(kind, n, &Settings::new(), &inputs);
        let b = run(kind, n, &Settings::new(), &inputs);
        assert_eq!(a, b, "{kind}");
    }
}

/// Tiles → Bevel → Gradient map → Toon bands → Painted highlight →
/// Colour steps → Material from height → Material out.
fn stylised_graph() -> Graph {
    let mut g = Graph::new("Stylised");
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
        "tiles",
        "bevel",
        "gradient_map",
        "toon_bands",
        "painted_highlight",
        "colour_steps",
        "material_from_height",
        "material_out",
    ]
    .iter()
    .map(|k| add(&mut g, k))
    .collect();
    let [tiles, bevel, map, toon, high, steps, mfh, out] = ids[..] else {
        unreachable!()
    };
    for (from, f, to, t) in [
        (tiles, "shape", bevel, "shape"),
        (bevel, "height", map, "in"),
        (map, "colour", toon, "colour"),
        (bevel, "height", toon, "height"),
        (toon, "colour_out", high, "colour"),
        (bevel, "height", high, "height"),
        (high, "colour_out", steps, "colour"),
        (bevel, "height", mfh, "height"),
        (steps, "colour_out", mfh, "colour"),
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
    g
}

#[test]
fn a_stylised_graph_makes_a_seamless_material() {
    let g = stylised_graph();
    let m = Evaluator::new()
        .material(registry(), &g, size(128))
        .unwrap();
    let value = Value::Material(m);
    assert_in_range("graph", std::slice::from_ref(&value));
    let check = crate::SeamCheck::of(&value);
    assert!(!check.shows(), "{}", check.describe());
}

#[test]
#[ignore = "speed check: run by hand"]
fn stylised_speed() {
    let threads = crate::parallel::cores();
    let sz = size(2048);
    for kind in NEW {
        let info = registry().get(kind).unwrap().info();
        let inputs = info
            .inputs
            .iter()
            .map(|p| Some(input(p.kind, sz)))
            .collect();
        let s = with(&[("snap", Switch(true))]);
        let job = Job::new(info, sz, 1, threads, &s, inputs).with_palettes(test_palettes());
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
