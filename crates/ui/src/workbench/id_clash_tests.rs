//! Tests that no two widgets share an id. When two do, egui (in a debug
//! build) paints "First use of widget ID … / Second use of widget ID …"
//! over the window, and a click on one can reach the other: a tester saw
//! it beside the Levels of detail rows. These open every panel and window
//! with as much in them as possible and look for that warning in what was
//! drawn.

use super::modelling_tests::{H, cube};
use super::tests::fresh;
use egui::epaint::Shape;
use scene::{
    Command, Curve, CurveObject, CurveShape, CurveSolid, Lods, ModifierKind, ObjectId, ObjectKind,
    Sweep,
};

/// Opens one of the app's windows.
type OpenWindow = fn(&mut super::Workbench);

/// Every id clash warning egui painted in the last frame.
pub(super) fn id_clashes(harness: &H) -> Vec<String> {
    fn walk(shape: &Shape, out: &mut Vec<String>) {
        match shape {
            Shape::Vec(shapes) => shapes.iter().for_each(|s| walk(s, out)),
            Shape::Text(text) => {
                let said = text.galley.text();
                if said.contains("use of") && said.contains(" ID ") {
                    out.push(said.to_string());
                }
            }
            _ => {}
        }
    }
    let mut out = Vec::new();
    for clipped in &harness.output().shapes {
        walk(&clipped.shape, &mut out);
    }
    out
}

/// Runs a few frames (a grid is laid out twice on its first frame) and
/// fails on any clash, saying where.
fn no_clashes(harness: &mut H, where_: &str) {
    harness.run_steps(3);
    let found = id_clashes(harness);
    assert!(found.is_empty(), "{where_}: {found:?}");
}

/// The cube with levels of detail (the most levels), every kind of
/// modifier with standard settings that a cube can take, and a lattice.
fn rich_cube(harness: &mut H) -> ObjectId {
    let id = cube(harness);
    let doc = harness.state_mut().workbench.document_mut();
    let lods = Lods {
        on: true,
        ..Lods::default()
    }
    .with_count(Lods::MAX_LEVELS);
    doc.execute(Command::SetLods {
        ids: vec![id],
        lods,
    })
    .unwrap();
    for kind in ModifierKind::standard_kinds() {
        // Those needing another object (a cage, a curve, shapes to scatter)
        // are refused here; the cage comes next.
        let _ = doc.execute(Command::AddModifier {
            ids: vec![id],
            kind,
        });
    }
    let _ = doc.execute(Command::AddLattice {
        ids: vec![id],
        divisions: [3, 3, 3],
    });
    doc.set_selection([id]);
    settle(harness);
    id
}

/// Runs the window until nothing is worked out elsewhere.
fn settle(harness: &mut H) {
    harness.run_steps(1);
    for _ in 0..1500 {
        if !harness.state().workbench.drawing.busy() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
        harness.run_steps(1);
    }
    harness.run_steps(2);
}

#[test]
fn the_warning_is_found_when_two_widgets_share_an_id() {
    // The check itself: two buttons with one id.
    let mut harness = egui_kittest::Harness::new_ui(|ui| {
        ui.push_id("same", |ui| {
            let _ = ui.interact(
                egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(20.0, 20.0)),
                egui::Id::new("twice"),
                egui::Sense::click(),
            );
            let _ = ui.interact(
                egui::Rect::from_min_size(egui::pos2(0.0, 60.0), egui::vec2(20.0, 20.0)),
                egui::Id::new("twice"),
                egui::Sense::click(),
            );
        });
    });
    harness.run();
    let mut found = 0;
    for clipped in &harness.output().shapes {
        if let Shape::Text(text) = &clipped.shape
            && text.galley.text().contains("use of")
        {
            found += 1;
        }
    }
    assert!(found > 0, "egui warns about the clash in a test build");
}

#[test]
fn the_levels_of_detail_rows_have_their_own_ids() {
    let mut harness = fresh();
    rich_cube(&mut harness);
    no_clashes(&mut harness, "a cube with levels of detail and modifiers");
}

#[test]
fn no_panel_or_window_has_clashing_ids() {
    let mut harness = fresh();
    let id = rich_cube(&mut harness);
    // Many objects, a group and a curve with a sweep, for the Objects
    // panel and the curve's rows.
    {
        let doc = harness.state_mut().workbench.document_mut();
        for kind in [
            scene::PrimitiveKind::Sphere,
            scene::PrimitiveKind::Cylinder,
            scene::PrimitiveKind::Plane,
        ] {
            doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
                .unwrap();
        }
        doc.execute(Command::Create(ObjectKind::Group)).unwrap();
        let curve = Curve::corners(
            &[
                glam::Vec3::new(-100.0, 0.0, 0.0),
                glam::Vec3::new(0.0, 80.0, 0.0),
                glam::Vec3::new(100.0, 0.0, 0.0),
            ],
            false,
        )
        .unwrap();
        doc.execute(Command::Create(ObjectKind::Curve(CurveObject::line(
            CurveShape::Points(curve),
        ))))
        .unwrap();
        let curve = doc.selection()[0];
        doc.execute(Command::SetCurveSolid {
            ids: vec![curve],
            solid: CurveSolid::Sweep(Sweep::default()),
        })
        .unwrap();
        settle(&mut harness);
        no_clashes(&mut harness, "a swept curve selected");
    }
    for mode in crate::palette::Mode::SHOWN {
        let workbench = &mut harness.state_mut().workbench;
        workbench.document_mut().set_selection([id]);
        workbench.mode = mode;
        no_clashes(&mut harness, &format!("the {} mode", mode.name()));
    }
    harness.state_mut().workbench.mode = crate::palette::Mode::Model;
    let windows: [(&str, OpenWindow); 4] = [
        ("Export", |w| w.exporting.open = true),
        ("Settings", |w| w.open_settings()),
        ("Keys", |w| w.shortcuts.open = true),
        ("Check for baking", |w| w.baking.open = true),
    ];
    for (name, open) in windows {
        open(&mut harness.state_mut().workbench);
        no_clashes(&mut harness, name);
        harness.key_press(egui::Key::Escape);
        harness.run_steps(2);
    }
    harness
        .state_mut()
        .workbench
        .switch_layout(super::LayoutChoice::UvEdit);
    no_clashes(&mut harness, "the UV Edit layout");
}
