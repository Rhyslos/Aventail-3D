//! Tests for the standard boxes: the list is sound, every box runs, and
//! every pattern tiles seamlessly. Each box's own checks (output range,
//! seams, seeds, extreme settings, what it does) are in `tests_each.rs`.

use crate::graph::{BoxId, Graph, GraphEdit, PlugRef};
use crate::image::{Grey, Size};
use crate::registry::{Job, Registry};
use crate::settings::{SettingValue, Settings};
use crate::value::Value;
use crate::{Evaluator, ValueKind};
use glam::Vec2;
use std::collections::BTreeSet;

#[test]
fn the_standard_boxes_are_sound() {
    let registry = Registry::standard();
    let ids: Vec<String> = registry.iter().map(|t| t.info().id.to_string()).collect();
    assert_eq!(ids.len(), 38);
    // Milestone 1's ids are released: they never change.
    for id in [
        "noise",
        "tiles",
        "levels",
        "blend",
        "colour_fill",
        "material_from_height",
        "material_out",
    ] {
        assert!(registry.get(id).is_some(), "{id}");
    }
    assert_eq!(
        ids.iter().collect::<BTreeSet<_>>().len(),
        ids.len(),
        "ids differ"
    );
    for kind in registry.iter() {
        let info = kind.info();
        assert!(
            !info.name.show().is_empty() && info.about.show().len() > 40,
            "{}",
            info.id
        );
        for next in &info.comes_next {
            assert!(registry.get(next).is_some(), "{} names {next}", info.id);
        }
        let plugs: BTreeSet<_> = info
            .inputs
            .iter()
            .chain(&info.outputs)
            .map(|p| &p.key)
            .collect();
        assert_eq!(
            plugs.len(),
            info.inputs.len() + info.outputs.len(),
            "{}",
            info.id
        );
        let keys: BTreeSet<_> = info.settings.iter().map(|s| &s.key).collect();
        assert_eq!(keys.len(), info.settings.len(), "{}", info.id);
        for s in &info.settings {
            assert!(s.check(s.standard()).is_ok(), "{} {}", info.id, s.key);
            assert!(!s.about.show().is_empty());
        }
        // Every box but Material out has something usually after it, and
        // makes something.
        if !info.one_per_graph {
            assert!(!info.comes_next.is_empty(), "{}", info.id);
            assert!(!info.outputs.is_empty(), "{}", info.id);
        }
    }
    // Groups come in the experts' order.
    let groups: Vec<_> = registry.iter().map(|t| t.info().group).collect();
    assert!(groups.windows(2).all(|w| w[0] <= w[1]));
    let mut again = Registry::with_standard_boxes();
    let noise = again.get("noise").unwrap().clone();
    assert!(again.add(noise).is_err());
}

/// Every box run on its own at `size`, with flat inputs where it has any.
fn run_alone(kind: &str, size: Size, settings: &Settings) -> Vec<Value> {
    let registry = Registry::standard();
    let info = registry.get(kind).unwrap().info();
    let inputs = info
        .inputs
        .iter()
        .map(|p| match p.kind {
            ValueKind::Material => None,
            k if k.is_grey() => Value::flat_grey(k, size, 0.5),
            _ => None,
        })
        .collect();
    let job = Job::new(info, size, 7, 2, settings, inputs);
    registry.get(kind).unwrap().run(&job).unwrap()
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

#[test]
fn noise_repeats_exactly_at_the_texture_edge() {
    for (u, v) in [(0.1, 0.2), (0.73, 0.01), (0.999, 0.5)] {
        for scale in [1, 3, 8] {
            let a = super::noise::layered(u, v, scale, 4, 0.5, 11);
            let b = super::noise::layered(u + 1.0, v - 1.0, scale, 4, 0.5, 11);
            assert!((a - b).abs() < 1e-4, "{u} {v} {scale}: {a} {b}");
        }
    }
}

#[test]
fn every_maker_tiles_seamlessly() {
    let size = Size::new(128).unwrap();
    let mut bricks = Settings::new();
    bricks.set("shift", SettingValue::Number(0.5));
    bricks.set("across", SettingValue::Whole(3));
    for (kind, settings) in [
        ("noise", Settings::new()),
        ("tiles", Settings::new()),
        ("tiles", bricks),
    ] {
        let out = run_alone(kind, size, &settings);
        let grey = out[0].grey().unwrap();
        let (seam, inside) = seam_and_inside(grey);
        assert!(seam <= inside + 1e-5, "{kind}: seam {seam} inside {inside}");
    }
}

#[test]
fn a_made_material_tiles_seamlessly_too() {
    let registry = Registry::standard();
    let mut g = Graph::new("Seams");
    let add = |g: &mut Graph, kind: &str| {
        let id = BoxId(g.next_id());
        g.apply(
            registry,
            &GraphEdit::AddBox {
                kind: kind.into(),
                at: Vec2::ZERO,
            },
        )
        .unwrap();
        id
    };
    let (noise, mat, out) = (
        add(&mut g, "noise"),
        add(&mut g, "material_from_height"),
        add(&mut g, "material_out"),
    );
    for (from, to) in [
        ((noise, "height"), (mat, "height")),
        ((mat, "material"), (out, "material")),
    ] {
        g.apply(
            registry,
            &GraphEdit::Connect {
                from: PlugRef::new(from.0, from.1),
                to: PlugRef::new(to.0, to.1),
            },
        )
        .unwrap();
    }
    let maps = Evaluator::with_threads(2)
        .material(registry, &g, Size::new(64).unwrap())
        .unwrap();
    for grey in [&maps.occlusion, &maps.curvature, &maps.height] {
        let (seam, inside) = seam_and_inside(grey);
        assert!(seam <= inside + 1e-5, "seam {seam} inside {inside}");
    }
    let normal_z = maps.normal.map(|n| n[0]);
    let (seam, inside) = seam_and_inside(&normal_z);
    assert!(seam <= inside + 1e-5, "normal seam {seam} inside {inside}");
}

#[test]
fn tiles_are_white_inside_and_black_in_the_gaps() {
    let size = Size::new(64).unwrap();
    let mut wide = Settings::new();
    wide.set("gap", SettingValue::Number(0.2));
    let out = run_alone("tiles", size, &wide);
    let grey = out[0].grey().unwrap();
    // Four tiles of 16 pixels; gaps 3.2 pixels round each edge.
    assert_eq!(grey.at(8, 8), 1.0);
    assert_eq!(grey.at(0, 8), 0.0);
    assert_eq!(grey.at(16, 16), 0.0);
}

#[test]
fn levels_and_blend_do_what_they_say() {
    let size = Size::new(16).unwrap();
    let mut s = Settings::new();
    s.set("low", SettingValue::Number(0.4));
    s.set("high", SettingValue::Number(0.6));
    s.set("invert", SettingValue::Switch(true));
    let out = run_alone("levels", size, &s);
    assert!((out[0].grey().unwrap().at(0, 0) - 0.5).abs() < 1e-5);
    // Equal Darkest and Lightest: a sharp step, no division by nothing.
    s.set("high", SettingValue::Number(0.4));
    let out = run_alone("levels", size, &s);
    assert_eq!(out[0].grey().unwrap().at(0, 0), 0.0);
    for mode in super::BlendMode::ALL {
        let v = mode.apply(0.25, 0.5);
        assert!((0.0..=1.0).contains(&v));
    }
    assert_eq!(super::BlendMode::Subtract.apply(0.2, 0.5), 0.0);
}

#[test]
fn material_from_height_is_flat_on_a_flat_height() {
    let size = Size::new(32).unwrap();
    let registry = Registry::standard();
    let info = registry.get("material_from_height").unwrap().info();
    let job = Job::new(
        info,
        size,
        0,
        1,
        &Settings::new(),
        vec![Value::flat_grey(ValueKind::Height, size, 0.3), None],
    );
    let out = registry
        .get("material_from_height")
        .unwrap()
        .run(&job)
        .unwrap();
    let maps = out[0].material().unwrap();
    assert_eq!(maps.normal.at(5, 5), [0.0, 0.0, 1.0]);
    assert_eq!(maps.occlusion.at(5, 5), 1.0);
    assert_eq!(maps.curvature.at(5, 5), 0.5);
    assert_eq!(maps.colour.at(0, 0), [0.6; 3]);
    assert_eq!(maps.size(), 32);
}
