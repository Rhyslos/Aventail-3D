//! Tests for high-to-low baking and the cage (decision 0251).

use super::baker::name_pairs;
use super::seams_tests::{add, shape, steps};
use super::*;
use crate::OverlayStyle;

fn rename(harness: &mut egui_kittest::Harness<'static, tests::State>, id: scene::ObjectId, name: &str) {
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::Rename {
            id,
            name: name.to_string(),
        })
        .unwrap();
}

#[test]
fn low_objects_pair_with_their_high_ones_and_floating_details() {
    let mut harness = tests::fresh();
    let low = add(&mut harness, scene::Primitive::Cube(Default::default()));
    let high = add(&mut harness, scene::Primitive::Sphere(Default::default()));
    let bolts = add(&mut harness, scene::Primitive::Cylinder(Default::default()));
    let other = add(&mut harness, scene::Primitive::Cube(Default::default()));
    rename(&mut harness, low, "Crate_low");
    rename(&mut harness, high, "crate_HIGH");
    rename(&mut harness, bolts, "crate_high_bolts");
    rename(&mut harness, other, "barrel_high");
    let w = &mut harness.state_mut().workbench;
    let pairs = name_pairs(w.document().scene(), &[low, other]);
    assert_eq!(pairs.len(), 1, "barrel_high isn't a low object");
    assert_eq!(pairs[0].0, low);
    let mut highs = pairs[0].1.clone();
    highs.sort();
    let mut want = vec![high, bolts];
    want.sort();
    assert_eq!(highs, want);
    w.document_mut().set_selection([low]);
    w.baker.by_name = true;
    let orders = w.bake_orders().unwrap();
    assert_eq!(orders.len(), 1);
    assert_eq!(orders[0].high_names.len(), 2);
    // Nothing named _low: says how.
    w.document_mut().set_selection([other]);
    assert!(w.bake_orders().err().unwrap().contains("_low"));
}

#[test]
fn a_cage_stroke_is_one_step_and_ctrl_pulls_in() {
    let mut harness = tests::fresh();
    let id = add(&mut harness, scene::Primitive::Cube(Default::default()));
    let m = shape(&harness);
    let v = m.vertices().next().unwrap().0;
    let before = steps(&harness);
    let w = &mut harness.state_mut().workbench;
    w.start_cage_brush();
    assert!(w.cage_brush_on());
    w.cage_dab(Some(1), vec![(id, v)]);
    w.cage_dab(Some(1), vec![(id, v)]);
    assert_eq!(steps(&harness), before + 1);
    let out = mesh::cage::push_of(&shape(&harness), v);
    assert!(out > 0.0);
    // A new stroke with Ctrl: back in by the same step.
    let w = &mut harness.state_mut().workbench;
    w.cage_brush.as_mut().unwrap().ctrl = true;
    w.cage_dab(Some(2), vec![(id, v)]);
    assert!(mesh::cage::push_of(&shape(&harness), v).abs() < 1e-4);
    harness.state_mut().workbench.stop_cage_brush();
    assert!(!harness.state().workbench.cage_brush_on());
}

#[test]
fn clearing_the_cage_is_one_step() {
    let mut harness = tests::fresh();
    let id = add(&mut harness, scene::Primitive::Cube(Default::default()));
    let v = shape(&harness).vertices().next().unwrap().0;
    let w = &mut harness.state_mut().workbench;
    w.start_cage_brush();
    w.cage_dab(None, vec![(id, v)]);
    w.clear_cage();
    assert!(mesh::cage::pushes(&shape(&harness)).is_empty());
    harness.state_mut().workbench.clear_cage();
    assert!(super::modelling_tests::note(&harness).contains("nothing painted"));
}

#[test]
fn the_cage_is_drawn_while_the_bake_window_shows_it() {
    let mut harness = tests::fresh();
    add(&mut harness, scene::Primitive::Cube(Default::default()));
    harness.state_mut().workbench.baker.open = true;
    harness.run();
    harness.run();
    let cage = |h: &egui_kittest::Harness<'static, tests::State>| {
        h.state()
            .workbench
            .components
            .overlay()
            .lines
            .iter()
            .filter(|(_, s)| *s == OverlayStyle::Cage)
            .count()
    };
    assert_eq!(cage(&harness), 12, "a cube's twelve edges");
    harness.state_mut().workbench.baker.show_cage = false;
    harness.run();
    harness.run();
    assert_eq!(cage(&harness), 0);
}

#[test]
fn the_cage_lines_sit_outside_the_shape() {
    let cube = scene::Primitive::Cube(Default::default()).mesh();
    let points: Vec<glam::Vec3> = cube.vertices().map(|(_, p)| p).collect();
    let centre = points.iter().copied().sum::<glam::Vec3>() / points.len() as f32;
    let corner = points[0].distance(centre);
    let lines = super::cage::cage_lines(&cube, 0.1);
    assert_eq!(lines.len(), 12);
    for [a, b] in lines {
        for p in [a, b] {
            // Every corner pushed out along its averaged facing.
            assert!(p.distance(centre) > corner * 1.01, "{p:?}");
        }
    }
}
