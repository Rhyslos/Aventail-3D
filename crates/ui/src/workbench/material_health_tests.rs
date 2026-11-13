//! The material health check in the Material mode's preview (decision
//! 0290), used through the real interface: a healthy material says so; a
//! dark one is warned about and its fix puts a lighter Hue and saturation
//! box in, one undo step, after which the warning goes; Look at selects
//! the box; a trim sheet's fix makes the project's trim rows, one undo
//! step; pixel art unlike the project's other pixel art offers the
//! real-world size that matches. No keys or mouse actions of their own.

use super::graph_giving_tests::page;
use crate::material::health::HEALTH_CHECK;
use crate::material::trim_rows::MATCH_LABEL;
use egui_kittest::kittest::Queryable;
use glam::Vec2;
use matgraph::{BoxId, Graph, GraphEdit, GraphId, PlugRef, Registry, SettingValue};

type H = egui_kittest::Harness<'static, super::tests::State>;

const FILL: BoxId = BoxId(1);
const NOISE: BoxId = BoxId(2);
const MATERIAL: BoxId = BoxId(3);
const OUT: BoxId = BoxId(4);

/// A Colour fill of `colour` and Noise's height made a material, into
/// Material out.
fn painted(colour: [f32; 3]) -> Graph {
    let registry = Registry::standard();
    let mut g = Graph::new("Paint");
    for (kind, x, y) in [
        ("colour_fill", 0.0, 0.0),
        ("noise", 0.0, 300.0),
        ("material_from_height", 300.0, 0.0),
        ("material_out", 600.0, 0.0),
    ] {
        let edit = GraphEdit::AddBox {
            kind: kind.into(),
            at: Vec2::new(x, y),
        };
        g.apply(registry, &edit).unwrap();
    }
    let edit = GraphEdit::SetSetting {
        node: FILL,
        key: "colour".into(),
        value: SettingValue::Colour(colour),
    };
    g.apply(registry, &edit).unwrap();
    // Big, soft bumps: nothing to fade at a distance.
    let edit = GraphEdit::SetSetting {
        node: NOISE,
        key: "scale".into(),
        value: SettingValue::Whole(4),
    };
    g.apply(registry, &edit).unwrap();
    for (from, out, to, input) in [
        (FILL, "colour", MATERIAL, "colour"),
        (NOISE, "height", MATERIAL, "height"),
        (MATERIAL, "material", OUT, "material"),
    ] {
        let edit = GraphEdit::Connect {
            from: PlugRef::new(from, out),
            to: PlugRef::new(to, input),
        };
        g.apply(registry, &edit).unwrap();
    }
    g
}

fn graph(harness: &H, id: GraphId) -> std::sync::Arc<Graph> {
    harness
        .state()
        .workbench
        .document()
        .scene()
        .graphs()
        .get(id)
        .unwrap()
        .clone()
}

fn note(harness: &H) -> String {
    harness.state().workbench.note.clone().unwrap_or_default()
}

/// Runs frames until the pictures and the health check have caught up.
fn settle(harness: &mut H) {
    for _ in 0..4 {
        harness.run();
    }
}

#[test]
fn a_healthy_material_says_nothing_was_found() {
    let (mut harness, _) = page(painted([0.55, 0.42, 0.3]));
    settle(&mut harness);
    assert!(harness.query_by_label(HEALTH_CHECK).is_some());
    assert!(
        harness
            .query_by_label_contains("Nothing found: colours light well")
            .is_some()
    );
}

#[test]
fn a_dark_material_is_warned_about_and_fixed_in_one_step() {
    let (mut harness, id) = page(painted([0.02, 0.02, 0.02]));
    settle(&mut harness);
    assert!(
        harness
            .query_by_label_contains("darker than charcoal")
            .is_some()
    );
    let fix = "Put a Hue and saturation box before Material from height's colour, lighter";
    harness.get_by_label(fix).click();
    settle(&mut harness);
    let g = graph(&harness, id);
    assert_eq!(g.box_count(), 5);
    let hue = BoxId(5);
    assert_eq!(g.get(hue).unwrap().kind, "hue_saturation");
    assert_eq!(
        g.wire_into(&PlugRef::new(MATERIAL, "colour")).unwrap().from,
        PlugRef::new(hue, "out")
    );
    assert_eq!(
        g.wire_into(&PlugRef::new(hue, "in")).unwrap().from,
        PlugRef::new(FILL, "colour")
    );
    assert!(
        note(&harness).contains("Hue and saturation"),
        "{}",
        note(&harness)
    );
    // Lighter by a fifth: past charcoal, so the warning goes.
    assert!(
        harness
            .query_by_label_contains("darker than charcoal")
            .is_none()
    );
    // One undo step takes it all back.
    harness.state_mut().workbench.document_mut().undo();
    settle(&mut harness);
    assert_eq!(graph(&harness, id).box_count(), 4);
    assert!(
        harness
            .query_by_label_contains("darker than charcoal")
            .is_some()
    );
}

#[test]
fn look_at_selects_the_box_making_the_colour() {
    let (mut harness, _) = page(painted([0.99, 0.99, 0.99]));
    settle(&mut harness);
    assert!(harness.query_by_label_contains("fresh snow").is_some());
    harness
        .get_by_label("Look at Colour fill, which makes the colour")
        .click();
    harness.run();
    let selected = &harness.state_mut().workbench.material().canvas.selected;
    assert_eq!(selected.iter().copied().collect::<Vec<_>>(), vec![FILL]);
}

/// [`painted`] into a Trim sheet's Strip 1, laid on the project's rows.
fn sheet() -> Graph {
    let registry = Registry::standard();
    let mut g = painted([0.55, 0.42, 0.3]);
    let edit = GraphEdit::AddBox {
        kind: "trim_sheet".into(),
        at: Vec2::new(450.0, 0.0),
    };
    g.apply(registry, &edit).unwrap();
    for (from, out, to, input) in [
        (MATERIAL, "material", BoxId(5), "strip1"),
        (BoxId(5), "material_out", OUT, "material"),
    ] {
        let edit = GraphEdit::Connect {
            from: PlugRef::new(from, out),
            to: PlugRef::new(to, input),
        };
        g.apply(registry, &edit).unwrap();
    }
    g
}

#[test]
fn a_trim_sheets_fix_makes_the_projects_trim_rows() {
    let (mut harness, _) = page(sheet());
    settle(&mut harness);
    assert!(
        harness
            .query_by_label_contains("the project has none yet")
            .is_some()
    );
    harness.get_by_label(MATCH_LABEL).click();
    settle(&mut harness);
    let rows = harness
        .state()
        .workbench
        .document()
        .scene()
        .trim()
        .rows
        .clone();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "Strip 1");
    assert!(note(&harness).starts_with("The project's trim rows now match"));
    assert!(
        harness
            .query_by_label_contains("the project has none yet")
            .is_none()
    );
    harness.state_mut().workbench.document_mut().undo();
    settle(&mut harness);
    assert!(
        harness
            .state()
            .workbench
            .document()
            .scene()
            .trim()
            .rows
            .is_empty()
    );
}

#[test]
fn pixel_art_unlike_the_projects_other_pixel_art_offers_a_matching_size() {
    let registry = Registry::standard();
    let pixel = |name: &str, size_cm: f32| {
        let mut g = painted([0.55, 0.42, 0.3]);
        g.apply(registry, &GraphEdit::Rename(name.into())).unwrap();
        let edit = GraphEdit::AddBox {
            kind: matgraph::PIXELATE.into(),
            at: Vec2::new(450.0, 0.0),
        };
        g.apply(registry, &edit).unwrap();
        for (from, out, to, input) in [
            (MATERIAL, "material", BoxId(5), "material"),
            (BoxId(5), "material_out", OUT, "material"),
        ] {
            let edit = GraphEdit::Connect {
                from: PlugRef::new(from, out),
                to: PlugRef::new(to, input),
            };
            g.apply(registry, &edit).unwrap();
        }
        let edit = GraphEdit::SetSetting {
            node: OUT,
            key: matgraph::eval::REAL_SIZE.into(),
            value: SettingValue::Number(size_cm),
        };
        g.apply(registry, &edit).unwrap();
        g
    };
    // This one: 32 pixels over 200 cm, 16 a metre. Another graph in the
    // project: 32 over 100 cm, 32 a metre.
    let (mut harness, id) = page(pixel("Floor", 200.0));
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::AddGraph(pixel("Wall", 100.0)))
        .unwrap();
    settle(&mut harness);
    assert!(harness.query_by_label_contains("but Wall has 32").is_some());
    harness
        .get_by_label("Make one repeat cover 100 cm (32 pixels a metre)")
        .click();
    settle(&mut harness);
    assert_eq!(
        matgraph::real_size_cm(registry, &graph(&harness, id)),
        100.0
    );
    assert!(harness.query_by_label_contains("but Wall has").is_none());
}
