//! Transform and Repeat on every kind (decision 0270): heights exactly as
//! before (old graphs work out identically), colours and materials moved
//! the same way and seamless, normals turned with the picture, the same
//! pictures for the same seed on any number of cores, and the refusals
//! when nothing, or the wrong thing, is plugged in.

use super::pattern::cell_random;
use super::tests_each::{
    assert_in_range, assert_seamless, input, registry, size, with,
};
use crate::graph::{BoxId, Graph, GraphEdit, PlugRef};
use crate::image::{Grey, Size};
use crate::registry::Job;
use crate::settings::{SettingValue, Settings};
use crate::value::Value;
use crate::{Evaluator, ValueKind};
use glam::Vec2;

/// Runs `kind` with `inputs` (by key; others empty), giving its outputs,
/// empty ones `None`.
fn moved(
    kind: &str,
    s: &Settings,
    inputs: &[(&str, &Value)],
    threads: usize,
    seed: u64,
) -> Vec<Option<Value>> {
    let t = registry().get(kind).unwrap();
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
    let job = Job::new(info, size(32), seed, threads, s, given);
    t.run_outputs(&job).unwrap()
}

/// Settings to try on Transform: each turn and flip, and a move.
fn transforms() -> Vec<Settings> {
    use SettingValue::{Choice, Number, Switch};
    let mut out = vec![Settings::new()];
    for turn in 0..4 {
        for (fx, fy) in [(false, false), (true, false), (false, true), (true, true)] {
            out.push(with(&[
                ("turn", Choice(turn)),
                ("flip_across", Switch(fx)),
                ("flip_down", Switch(fy)),
            ]));
        }
    }
    // A move by whole pixels (at 32), so the seam check compares like
    // with like; moves between pixels blend, as for heights.
    out.push(with(&[("across", Number(0.25)), ("down", Number(-0.375))]));
    out.push(with(&[("across", Number(0.3)), ("down", Number(-0.6))]));
    out
}

/// Settings to try on Repeat.
fn repeats() -> Vec<Settings> {
    use SettingValue::{Number, Whole};
    vec![
        Settings::new(),
        with(&[("across", Whole(3)), ("down", Whole(4))]),
        with(&[("across", Whole(2)), ("down", Whole(2)), ("shift", Number(0.5))]),
        with(&[("vary", Number(1.0)), ("across", Whole(4)), ("down", Whole(1))]),
    ]
}

/// The old Transform (heights only), as released before decision 0270.
fn old_transform(job_settings: &Settings, input: &Grey) -> Grey {
    let info = registry().get("transform").unwrap().info();
    let job = Job::new(info, size(32), 0, 1, job_settings, Vec::new());
    let (across, down) = (job.number("across"), job.number("down"));
    let turn = job.choice("turn") % 4;
    let (flip_x, flip_y) = (job.switch("flip_across"), job.switch("flip_down"));
    let n = 32.0;
    job.fill_grey(|x, y| {
        let (mut u, mut v) = super::uv(x, y, n);
        u -= across;
        v -= down;
        let (cu, cv) = (u - 0.5, v - 0.5);
        let (cu, cv) = match turn {
            1 => (cv, -cu),
            2 => (-cu, -cv),
            3 => (-cv, cu),
            _ => (cu, cv),
        };
        let (mut u, mut v) = (cu + 0.5, cv + 0.5);
        if flip_x {
            u = 1.0 - u;
        }
        if flip_y {
            v = 1.0 - v;
        }
        input.sample(u, v)
    })
}

/// The old Repeat (heights only), as released before decision 0270.
fn old_repeat(job_settings: &Settings, input: &Grey, seed: u64) -> Grey {
    let info = registry().get("repeat").unwrap().info();
    let job = Job::new(info, size(32), seed, 1, job_settings, Vec::new());
    let across = job.whole("across").max(1);
    let down = job.whole("down").max(1);
    let shift = job.number("shift");
    let vary = job.number("vary").clamp(0.0, 1.0);
    let n = 32.0;
    job.fill_grey(|x, y| {
        let (u, v) = super::uv(x, y, n);
        let fy = v * down as f32;
        let row = fy.floor();
        let fx = u * across as f32 + if row as i64 % 2 == 1 { shift } else { 0.0 };
        let col = fx.floor();
        let value = input.sample(fx - col, fy - row);
        let dark = cell_random(
            seed,
            0,
            col as i64,
            row as i64,
            i64::from(across),
            i64::from(down),
        );
        value * (1.0 - vary * dark)
    })
}

#[test]
fn heights_move_exactly_as_before() {
    let height = input(ValueKind::Height, size(32));
    let grey = height.grey().unwrap();
    for s in transforms() {
        let out = moved("transform", &s, &[("in", &height)], 1, 0);
        let now = out[0].as_ref().unwrap().grey().unwrap();
        assert_eq!(**now, old_transform(&s, grey));
        assert!(out[1].is_none() && out[2].is_none());
    }
    for s in repeats() {
        let out = moved("repeat", &s, &[("in", &height)], 1, 11);
        let now = out[0].as_ref().unwrap().grey().unwrap();
        assert_eq!(**now, old_repeat(&s, grey, 11));
    }
}

/// A graph Noise → `kind` (wired the old way, `in` to `out`) → Levels,
/// worked out: the plugs and keys released before decision 0270 still
/// connect, and the picture is the old one.
#[test]
fn an_old_graph_still_connects_and_works_out_the_same() {
    for (kind, s) in [("transform", transforms()[5].clone()), ("repeat", repeats()[2].clone())] {
        let mut g = Graph::new("Old");
        for k in ["noise", kind, "levels"] {
            g.apply(
                registry(),
                &GraphEdit::AddBox {
                    kind: k.into(),
                    at: Vec2::ZERO,
                },
            )
            .unwrap();
        }
        let mut s = s.clone();
        if kind == "repeat" {
            s.set("seed", SettingValue::Whole(5));
        }
        for (key, value) in s.iter() {
            g.apply(
                registry(),
                &GraphEdit::SetSetting {
                    node: BoxId(2),
                    key: key.to_string(),
                    value,
                },
            )
            .unwrap();
        }
        for (from, to) in [((1, "height"), (2, "in")), ((2, "out"), (3, "in"))] {
            g.apply(
                registry(),
                &GraphEdit::Connect {
                    from: PlugRef::new(BoxId(from.0), from.1),
                    to: PlugRef::new(BoxId(to.0), to.1),
                },
            )
            .unwrap();
        }
        let e = Evaluator::with_threads(2).run(registry(), &g, size(32), &[]);
        assert_eq!(e.problems().count(), 0, "{kind}");
        let noise = e.picture(BoxId(1)).unwrap().grey().unwrap();
        // Repeat's seed is mixed with the graph's, as the evaluator does.
        let seed = crate::seed::box_seed(g.seed(), 5);
        let expected = match kind {
            "transform" => old_transform(&s, noise),
            _ => old_repeat(&s, noise, seed),
        };
        assert_eq!(**e.picture(BoxId(2)).unwrap().grey().unwrap(), expected, "{kind}");
    }
}

#[test]
fn a_colour_moves_as_its_channels_would_as_heights() {
    let colour = input(ValueKind::Colour, size(32));
    let channel = |c: &Value, i: usize| -> Value {
        Value::Height(std::sync::Arc::new(c.colour().unwrap().map(|p| p[i])))
    };
    for (kind, settings) in [("transform", transforms()), ("repeat", repeats())] {
        for s in settings {
            let out = moved(kind, &s, &[("colour", &colour)], 1, 5);
            assert!(out[0].is_none() && out[2].is_none());
            let c = out[1].clone().unwrap();
            assert_eq!(c.kind(), ValueKind::Colour);
            for i in 0..3 {
                let alone = moved(kind, &s, &[("in", &channel(&colour, i))], 1, 5);
                let h = alone[0].as_ref().unwrap().grey().unwrap();
                for (a, b) in channel(&c, i).grey().unwrap().pixels().iter().zip(h.pixels()) {
                    assert!((a - b).abs() < 1e-6, "{kind} channel {i}");
                }
            }
        }
    }
}

#[test]
fn every_kind_stays_in_range_and_seamless() {
    let height = input(ValueKind::Height, size(32));
    let colour = input(ValueKind::Colour, size(32));
    let material = input(ValueKind::Material, size(32));
    let all = [("in", &height), ("colour", &colour), ("material", &material)];
    for (kind, settings) in [("transform", transforms()), ("repeat", repeats())] {
        for (i, s) in settings.iter().enumerate() {
            let out: Vec<Value> = moved(kind, s, &all, 2, 3).into_iter().flatten().collect();
            assert_eq!(out.len(), 3);
            assert_in_range(kind, &out);
            // The last Transform moves between pixels, and the last Repeat
            // varies its copies, so its seam is a join between two copies
            // like any other: the largest step may fall there by chance.
            if i != settings.len() - 1 {
                assert_seamless(&format!("{kind} {i}"), &out);
            }
        }
    }
}

#[test]
fn the_same_seed_gives_the_same_pictures_on_any_number_of_cores() {
    let colour = input(ValueKind::Colour, size(32));
    let material = input(ValueKind::Material, size(32));
    let all = [("colour", &colour), ("material", &material)];
    for (kind, s) in [("transform", transforms()[7].clone()), ("repeat", repeats()[3].clone())] {
        let one = moved(kind, &s, &all, 1, 9);
        for threads in [2, 3, 8] {
            assert_eq!(moved(kind, &s, &all, threads, 9), one, "{kind} on {threads}");
        }
    }
    // Repeat's Vary darkens other copies with another seed.
    let s = &repeats()[3];
    assert_ne!(moved("repeat", s, &all, 1, 9), moved("repeat", s, &all, 1, 10));
}

/// A material made from a height and then moved has the normals of the
/// material made from the moved height: its bumps turn with it.
#[test]
fn a_moved_materials_bumps_turn_with_it() {
    let height = input(ValueKind::Height, size(32));
    let make = |h: &Value| -> Value {
        let t = registry().get("material_from_height").unwrap();
        let job = Job::new(t.info(), size(32), 3, 1, &Settings::new(), vec![Some(h.clone()), None]);
        t.run(&job).unwrap().remove(0)
    };
    let material = make(&height);
    for s in transforms().into_iter().take(17) {
        let moved_material = moved("transform", &s, &[("material", &material)], 1, 0)[2]
            .clone()
            .unwrap();
        let moved_height = moved("transform", &s, &[("in", &height)], 1, 0)[0]
            .clone()
            .unwrap();
        let expected = make(&moved_height);
        let (a, b) = (
            moved_material.material().unwrap(),
            expected.material().unwrap(),
        );
        for (p, q) in a.normal.pixels().iter().zip(b.normal.pixels()) {
            for i in 0..3 {
                assert!((p[i] - q[i]).abs() < 1e-4, "{s:?}: {p:?} {q:?}");
            }
        }
        assert_eq!(a.height, b.height);
    }
}

/// Copies made smaller have steeper bumps: the normals of a repeated
/// material lean further than the original's, as those of a material
/// made from the repeated height do.
#[test]
fn repeated_copies_have_steeper_bumps() {
    use SettingValue::Whole;
    let height = input(ValueKind::Height, size(32));
    let t = registry().get("material_from_height").unwrap();
    let job = Job::new(t.info(), size(32), 3, 1, &Settings::new(), vec![Some(height), None]);
    let material = t.run(&job).unwrap().remove(0);
    let lean = |v: &Value| -> f32 {
        let n = &v.material().unwrap().normal;
        n.pixels().iter().map(|p| 1.0 - p[2]).sum::<f32>() / n.pixels().len() as f32
    };
    let s = with(&[("across", Whole(4)), ("down", Whole(4))]);
    let copies = moved("repeat", &s, &[("material", &material)], 1, 0)[2]
        .clone()
        .unwrap();
    assert!(lean(&copies) > lean(&material) * 2.0);
    let one = with(&[("across", Whole(1)), ("down", Whole(1))]);
    let same = moved("repeat", &one, &[("material", &material)], 1, 0)[2]
        .clone()
        .unwrap();
    assert!((lean(&same) - lean(&material)).abs() < 1e-4);
}

#[test]
fn vary_darkens_a_copys_colour_and_lowers_its_height_only() {
    use SettingValue::Number;
    let material = input(ValueKind::Material, size(32));
    let plain = moved("repeat", &Settings::new(), &[("material", &material)], 1, 4)[2]
        .clone()
        .unwrap();
    let varied = moved("repeat", &with(&[("vary", Number(1.0))]), &[("material", &material)], 1, 4)[2]
        .clone()
        .unwrap();
    let (p, v) = (plain.material().unwrap(), varied.material().unwrap());
    assert!(v.colour.pixels().iter().zip(p.colour.pixels()).all(|(a, b)| a[0] <= b[0] + 1e-6));
    assert!(v.height.pixels().iter().zip(p.height.pixels()).all(|(a, b)| *a <= b + 1e-6));
    assert_ne!(v.colour, p.colour);
    assert_eq!(v.roughness, p.roughness);
    assert_eq!(v.metal, p.metal);
}

/// Adds `kinds` to a new graph, wires `wires` and works it out at 32.
fn worked_out(kinds: &[&str], wires: &[((u64, &str), (u64, &str))]) -> (Graph, crate::Evaluation) {
    let mut g = Graph::new("Moving");
    for k in kinds {
        g.apply(
            registry(),
            &GraphEdit::AddBox {
                kind: (*k).into(),
                at: Vec2::ZERO,
            },
        )
        .unwrap();
    }
    for (from, to) in wires {
        g.apply(
            registry(),
            &GraphEdit::Connect {
                from: PlugRef::new(BoxId(from.0), from.1),
                to: PlugRef::new(BoxId(to.0), to.1),
            },
        )
        .unwrap();
    }
    let e = Evaluator::with_threads(1).run(registry(), &g, Size::nearest(32), &[]);
    (g, e)
}

#[test]
fn a_box_with_only_a_colour_shows_the_colour() {
    let (_, e) = worked_out(
        &["colour_fill", "transform"],
        &[((1, "colour"), (2, "colour"))],
    );
    assert_eq!(e.problems().count(), 0);
    assert_eq!(e.picture(BoxId(2)).unwrap().kind(), ValueKind::Colour);
}

#[test]
fn nothing_plugged_in_or_an_empty_output_is_refused_with_why() {
    let (_, e) = worked_out(&["repeat", "noise"], &[]);
    let why = e.outcome(BoxId(1)).unwrap().as_ref().unwrap_err();
    assert!(why.reason.contains("nothing plugged in"), "{}", why.reason);
    assert!(!why.fixes.is_empty(), "offers boxes to plug in");
    // Wired to the colour output while only a height is plugged in.
    let (_, e) = worked_out(
        &["noise", "transform", "hue_saturation"],
        &[((1, "height"), (2, "in")), ((2, "colour_out"), (3, "in"))],
    );
    assert!(e.outcome(BoxId(2)).unwrap().is_ok());
    let why = e.outcome(BoxId(3)).unwrap().as_ref().unwrap_err();
    assert!(why.reason.contains("gives nothing"), "{}", why.reason);
    assert_eq!(why.fixes, vec![crate::Fix::LookAt(BoxId(2))]);
}

#[test]
fn transform_and_repeat_are_never_offered_as_makers() {
    for kind in ValueKind::ALL {
        for fix in crate::graph::connect::makers_of(registry(), kind) {
            let crate::Fix::AddBox { kind: id } = fix else {
                continue;
            };
            assert!(id != "transform" && id != "repeat", "{id}");
        }
    }
}
