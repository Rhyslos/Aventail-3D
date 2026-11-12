//! Tests for graph fills' pixels.

use super::*;
use glam::Vec2;
use matgraph::{BoxId, GraphEdit, PlugRef, SettingValue};
use mesh::PrimitiveKind;
use paint::PaintSurface;

fn cube() -> PaintSurface {
    PaintSurface::of(&PrimitiveKind::Cube.standard().mesh())
}

fn source(surface: &PaintSurface, size: u32) -> Source<'_> {
    Source {
        surface,
        slots: &[0],
        size,
    }
}

fn registry() -> &'static Registry {
    Registry::standard()
}

/// The texel where the cube's surface nearest `at` lies.
fn texel(surface: &PaintSurface, at: Vec3, size: u32) -> (u32, u32) {
    let (i, w) = surface.closest(at, &[0]).unwrap();
    let t = &surface.triangles()[i];
    let uv = t.uvs[0] * w[0] + t.uvs[1] * w[1] + t.uvs[2] * w[2];
    let n = size as f32;
    ((uv.x * n) as u32, ((1.0 - uv.y) * n) as u32)
}

/// A graph: grey stone everywhere, moss (green) where the model faces up.
fn moss_on_top() -> Graph {
    let mut g = Graph::new("Moss on top");
    let kinds = [
        "colour_fill",
        "noise",
        "material_from_height",
        "colour_fill",
        "material_from_height",
        "model_facing_up",
        "levels",
        "mix_materials",
        "material_out",
    ];
    for (i, kind) in kinds.iter().enumerate() {
        g.apply(
            registry(),
            &GraphEdit::AddBox {
                kind: (*kind).into(),
                at: Vec2::new(i as f32 * 200.0, 0.0),
            },
        )
        .unwrap();
    }
    let p = |id: u64, plug: &str| PlugRef::new(BoxId(id), plug);
    for (from, to) in [
        (p(1, "colour"), p(3, "colour")),
        (p(2, "height"), p(3, "height")),
        (p(4, "colour"), p(5, "colour")),
        (p(2, "height"), p(5, "height")),
        (p(6, "up"), p(7, "in")),
        (p(3, "material"), p(8, "bottom")),
        (p(5, "material"), p(8, "top")),
        (p(7, "out"), p(8, "where")),
        (p(8, "material"), p(9, "material")),
    ] {
        g.apply(registry(), &GraphEdit::Connect { from, to })
            .unwrap();
    }
    for (node, key, value) in [
        (1, "colour", SettingValue::Colour([0.5, 0.5, 0.5])),
        (4, "colour", SettingValue::Colour([0.1, 0.6, 0.1])),
    ] {
        g.apply(
            registry(),
            &GraphEdit::SetSetting {
                node: BoxId(node),
                key: key.into(),
                value,
            },
        )
        .unwrap();
    }
    g
}

#[test]
fn the_models_maps_are_baked_only_where_asked_and_spread_past_the_edges() {
    let s = cube();
    let needed = BTreeSet::from([ModelMap::FacingUp, ModelMap::Up]);
    let bake = model_maps(
        &[source(&s, 64)],
        &needed,
        &BakeSettings::default(),
        &Progress::default(),
        9,
    )
    .unwrap();
    assert_eq!(bake.maps.key(), 9);
    assert!(bake.maps.get(ModelMap::Occlusion).is_none());
    let up = bake.maps.get(ModelMap::FacingUp).unwrap();
    let (x, y) = texel(&s, Vec3::new(0.0, 0.0, 100.0), 64);
    assert!(up.at(x.into(), y.into()) > 0.95, "the top faces up");
    let (x, y) = texel(&s, Vec3::new(0.0, -50.0, 50.0), 64);
    assert!(up.at(x.into(), y.into()) < 0.05, "the front doesn't");
    let height = bake.maps.get(ModelMap::Up).unwrap();
    let (x, y) = texel(&s, Vec3::new(0.0, 0.0, 100.0), 64);
    assert!(height.at(x.into(), y.into()) > 0.95, "the top is high up");
    // Every texel has a value in range, covered or not.
    assert!(up.pixels().iter().all(|v| (0.0..=1.0).contains(v)));
    assert!(bake.covered.iter().any(|c| *c) && bake.covered.iter().any(|c| !*c));
}

#[test]
fn moss_settles_on_top_of_the_model() {
    let s = cube();
    let g = moss_on_top();
    let needed = matgraph::model::maps_needed(registry(), &g);
    assert_eq!(needed, BTreeSet::from([ModelMap::FacingUp]));
    let bake = model_maps(
        &[source(&s, 64)],
        &needed,
        &BakeSettings::default(),
        &Progress::default(),
        1,
    )
    .unwrap();
    let made = graph_fill(&mut Evaluator::with_threads(2), registry(), &g, &bake, 2.0, 1.0).unwrap();
    let colour = |at: Vec3| {
        let (x, y) = texel(&s, at, 64);
        made.colour.pixel(x, y)
    };
    let top = colour(Vec3::new(0.0, 0.0, 100.0));
    let side = colour(Vec3::new(0.0, -50.0, 50.0));
    assert!(top[1] > top[0] + 40, "moss on top: {top:?}");
    assert!(
        (side[1] as i32 - side[0] as i32).abs() < 10,
        "stone on the side: {side:?}"
    );
    // Only the polygons are painted; the realism greys come too.
    let empty = bake.covered.iter().position(|c| !*c).unwrap() as u32;
    assert_eq!(made.colour.pixel(empty % 64, empty / 64)[3], 0);
    let (x, y) = texel(&s, Vec3::new(0.0, 0.0, 100.0), 64);
    assert_eq!(made.roughness.pixel(x, y)[3], 255);
    assert_eq!(made.height.pixel(x, y)[3], 255);
}

#[test]
fn a_graph_without_material_out_says_why() {
    let s = cube();
    let bake = model_maps(
        &[source(&s, 32)],
        &BTreeSet::new(),
        &BakeSettings::default(),
        &Progress::default(),
        1,
    )
    .unwrap();
    let why = graph_fill(
        &mut Evaluator::with_threads(1),
        registry(),
        &Graph::new("Empty"),
        &bake,
        1.0,
        1.0,
    )
    .unwrap_err();
    assert!(!why.reason.is_empty());
}

#[test]
fn a_size_a_graph_cant_use_and_a_cancel_give_nothing() {
    let s = cube();
    let none = BTreeSet::new();
    let settings = BakeSettings::default();
    assert!(model_maps(&[source(&s, 8)], &none, &settings, &Progress::default(), 1).is_none());
    let cancelled = Progress::default();
    cancelled.cancel.store(true, Relaxed);
    let needed = BTreeSet::from([ModelMap::Occlusion]);
    assert!(model_maps(&[source(&s, 32)], &needed, &settings, &cancelled, 1).is_none());
}

#[test]
fn unknown_texels_take_their_neighbours_value_a_few_rings_out() {
    let n = 8;
    let mut values = vec![f32::NAN; n * n];
    values[0] = 1.0;
    spread(&mut values, n, 2);
    assert_eq!(values[1], 1.0);
    assert_eq!(values[2], 1.0);
    assert!(values[3].is_nan());
    // Nothing known: nothing changes, and it stops.
    let mut empty = vec![f32::NAN; 4];
    spread(&mut empty, 2, 5);
    assert!(empty.iter().all(|v| v.is_nan()));
}
