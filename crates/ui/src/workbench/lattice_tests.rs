//! Tests for lattice cages in the window (decision 0144): Add lattice on
//! the modifiers palette, the Mesh menu and command search; the cage drawn
//! as lines and picked by them; its points found, selected and moved in
//! the Points mode; what Delete and the Edges mode say; and the
//! Attributes panel's Lattice and Modifiers sections.

use super::modelling_tests::{H, cube, note, on_screen, press};
use super::tests::{fresh, mesh_group};
use super::*;
use crate::transforming::{Axes, Target};
use egui::{Event, PointerButton};
use egui_kittest::kittest::{NodeT, Queryable};
use glam::Vec3;
use scene::{Element, ElementSelection, Interpolation, Lattice, ModifierKind, ObjectId, VertexId};
use tools::Change;

/// The fresh window's cube, selected, with a cage added round it by the
/// palette's button: the cube and the cage.
fn caged() -> (H, ObjectId, ObjectId) {
    let mut harness = fresh();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    harness.get_by_label("Add lattice").click();
    harness.run();
    let cage = harness.state().workbench.document().selection()[0];
    (harness, id, cage)
}

fn lattice(harness: &H, cage: ObjectId) -> Lattice {
    harness
        .state()
        .workbench
        .document()
        .scene()
        .object(cage)
        .and_then(|o| o.kind.lattice())
        .cloned()
        .unwrap()
}

/// Where the cage's point `i` is in the world.
fn world_point(harness: &H, cage: ObjectId, i: usize) -> Vec3 {
    let scene = harness.state().workbench.document().scene();
    scene
        .shape_matrix(cage)
        .transform_point3(lattice(harness, cage).points()[i])
}

fn click_at(harness: &mut H, at: egui::Pos2) {
    harness.event(Event::PointerMoved(at));
    harness.run();
    super::modelling_tests::button(harness, at, true, PointerButton::Primary, Modifiers::NONE);
    super::modelling_tests::button(harness, at, false, PointerButton::Primary, Modifiers::NONE);
    harness.run();
}

fn select_point(harness: &mut H, cage: ObjectId, i: u32) {
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements(
            [(
                cage,
                ElementSelection::of([Element::Point(VertexId::from_raw(i))]),
            )],
            "Select",
            None,
        );
    harness.run();
}

#[test]
fn add_lattice_puts_a_cage_round_the_cube_drawn_as_lines() {
    let (harness, id, cage) = caged();
    let scene = harness.state().workbench.document().scene();
    assert_ne!(cage, id, "the cage is selected");
    assert_eq!(lattice(&harness, cage).divisions(), [2, 2, 2]);
    assert!(matches!(
        scene.modifiers(id)[0].kind,
        ModifierKind::Lattice(o) if o.cage == cage
    ));
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Add lattice")
    );
    let overlay = harness.state().workbench.components.overlay();
    assert!(
        overlay
            .lines
            .iter()
            .filter(|(_, s)| *s == crate::OverlayStyle::CurveSelected)
            .count()
            >= 12
    );
    // The Attributes panel shows the cage's settings.
    assert!(harness.query_all_by_label("Divisions X").count() >= 1);
    assert!(harness.query_by_label("Reset points").is_some());
}

#[test]
fn clicking_a_cage_line_selects_the_cage() {
    let (mut harness, id, cage) = caged();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    // Halfway along the cage's top front edge, off the cube's surface
    // only by the line's width, but the line is in front.
    let l = lattice(&harness, cage);
    let (a, b) = (l.index(0, 0, 1), l.index(1, 0, 1));
    let middle = (world_point(&harness, cage, a) + world_point(&harness, cage, b)) * 0.5;
    let at = on_screen(&harness, middle + Vec3::new(0.0, -2.0, 2.0));
    click_at(&mut harness, at);
    let selection = harness.state().workbench.document().selection();
    assert_eq!(selection, vec![cage]);
    // The cube's surface is still the cube's: lines hidden behind it
    // aren't picked through it.
    let centre = harness
        .state()
        .workbench
        .document()
        .scene()
        .bounds_of([id])
        .unwrap()
        .centre();
    let at = on_screen(&harness, centre);
    click_at(&mut harness, at);
    assert_eq!(harness.state().workbench.document().selection(), vec![id]);
}

#[test]
fn the_points_mode_moves_cage_points_and_bends_the_cube() {
    let (mut harness, id, cage) = caged();
    press(&mut harness, Modifiers::NONE, Key::Num1);
    let l = lattice(&harness, cage);
    let corner = l.index(1, 0, 1) as u32;
    let at = on_screen(&harness, world_point(&harness, cage, corner as usize));
    click_at(&mut harness, at);
    let selected: Vec<VertexId> = harness
        .state()
        .workbench
        .document()
        .elements(cage)
        .unwrap()
        .points()
        .collect();
    assert_eq!(selected, vec![VertexId::from_raw(corner)]);
    assert!(
        harness
            .state()
            .workbench
            .components
            .overlay()
            .points
            .iter()
            .any(|(_, s)| *s == crate::OverlayStyle::Handle)
    );
    let workbench = &mut harness.state_mut().workbench;
    let summary = workbench
        .drawing
        .update(
            &workbench.document,
            Some(tools::ElementKind::Points),
            &workbench.hidden,
        )
        .clone();
    let target = Target::of(
        &workbench.document,
        &summary.drawing,
        Mode::Points,
        Axes::World,
    )
    .unwrap();
    let commands = target.commands(&Change::Translate(Vec3::new(0.0, 0.0, 40.0)));
    assert!(matches!(commands[0], scene::Command::EditLattice { .. }));
    for command in commands {
        workbench.document.execute(command).unwrap();
    }
    harness.run();
    let scene = harness.state().workbench.document().scene();
    let before = scene.unmodified_shape(id).unwrap().bounds().unwrap();
    let after = scene.shown_shape(id).unwrap().bounds().unwrap();
    assert!((after.size().z - before.size().z - 40.0).abs() < 1e-2);
    // The viewport draws the cube bent.
    let view = harness.state().renderer.drawn.last().unwrap().clone();
    let item = view.scene.items.iter().find(|i| i.id == id).unwrap();
    let drawn = item.drawn.bounds().unwrap();
    assert!((drawn.size().z - after.size().z).abs() < 1e-3);
}

#[test]
fn a_cages_points_stay_and_it_has_no_edges() {
    let (mut harness, _, cage) = caged();
    press(&mut harness, Modifiers::NONE, Key::Num1);
    select_point(&mut harness, cage, 0);
    press(&mut harness, Modifiers::NONE, Key::Delete);
    assert_eq!(lattice(&harness, cage).point_count(), 8);
    assert!(
        note(&harness).contains("can't be deleted"),
        "{}",
        note(&harness)
    );
    press(&mut harness, Modifiers::NONE, Key::Num2);
    assert!(note(&harness).contains("no edges"), "{}", note(&harness));
}

#[test]
fn the_attributes_panel_changes_the_cage() {
    let (mut harness, id, cage) = caged();
    harness.get_by_label("Linear").click();
    harness.run();
    assert_eq!(
        lattice(&harness, cage).interpolation(),
        Interpolation::Linear
    );
    assert!(
        harness
            .get_by_label("Reset points")
            .accesskit_node()
            .is_disabled(),
        "nothing to reset yet"
    );
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::EditLattice {
            id: cage,
            edit: scene::LatticeEdit::Transform {
                points: vec![0],
                matrix: glam::Mat4::from_translation(Vec3::X * -30.0),
            },
        })
        .unwrap();
    harness.run();
    harness.get_by_label("Reset points").click();
    harness.run();
    assert!(lattice(&harness, cage).is_at_rest());
    // The cube's Lattice names its cage and selects it.
    super::tests::open_modifier(&mut harness, id, 0);
    assert!(harness.query_all_by_label("Lattice").count() >= 1);
    harness.get_by_label("Select cage").click();
    harness.run();
    assert_eq!(harness.state().workbench.document().selection(), vec![cage]);
}

#[test]
fn the_mesh_menu_and_command_search_offer_it() {
    let mut harness = fresh();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    mesh_group(&mut harness, super::mesh_menu::MESH_MODIFIERS_MENU);
    assert!(harness.query_by_label("Deform").is_some(), "family name");
    harness
        .get_all_by_label("Add lattice")
        .find(|n| !n.accesskit_node().is_disabled())
        .expect("Add lattice can be chosen")
        .click();
    harness.run();
    assert_eq!(
        harness
            .state()
            .workbench
            .document()
            .scene()
            .modifiers(id)
            .len(),
        1
    );
    let entries = harness.state().workbench.search_entries(&harness.ctx);
    let entry = entries.iter().find(|e| e.name == "Add lattice").unwrap();
    assert_eq!(entry.place, "Mesh");
}
