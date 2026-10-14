//! Tests for clean-up in the window: Delete and Backspace in the element
//! modes, Fill hole, Flip normals, Fix facing, Select flipped and Select
//! similar facing.

use super::modelling_tests::{H, cube, last_operation, note, press, shape};
use super::selection::SelectRequest;
use super::tests::fresh;
use super::*;
use egui_kittest::kittest::Queryable;
use scene::{Element, ElementSelection, Operation};

/// The fresh window's cube selected, in `mode_key`'s mode, with
/// `elements` selected.
fn ready(mode_key: Key, elements: impl Fn(&scene::Mesh) -> Vec<Element>) -> H {
    let mut harness = fresh();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    press(&mut harness, Modifiers::NONE, mode_key);
    let chosen = elements(&shape(&harness));
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements([(id, ElementSelection::of(chosen))], "Select", None);
    harness.run();
    harness
}

fn top(mesh: &scene::Mesh) -> Vec<Element> {
    mesh.faces()
        .map(|(f, _)| f)
        .filter(|f| mesh.face_normal(*f).z > 0.9)
        .map(Element::Polygon)
        .collect()
}

fn one_upright(mesh: &scene::Mesh) -> Vec<Element> {
    mesh.edges()
        .iter()
        .filter(|e| {
            let (a, b) = e.ends();
            (mesh.position(a).unwrap().z - mesh.position(b).unwrap().z).abs() > 1.0
        })
        .take(1)
        .map(|e| Element::Edge(*e))
        .collect()
}

fn report(harness: &H) -> mesh::MeshReport {
    mesh::MeshReport::new(&shape(harness))
}

fn run(harness: &mut H, command: crate::commands::CommandId) {
    let ctx = harness.ctx.clone();
    harness
        .state_mut()
        .workbench
        .run_command(command, &ctx, &mut Vec::new());
    harness.run();
}

#[test]
fn delete_in_the_polygons_mode_deletes_polygons_not_the_object() {
    let mut harness = ready(Key::Num3, top);
    let objects = harness.state().workbench.document().scene().walk().len();
    press(&mut harness, Modifiers::NONE, Key::Delete);
    assert_eq!(shape(&harness).face_count(), 5);
    assert_eq!(report(&harness).border_edges, 4);
    let doc = harness.state().workbench.document();
    assert_eq!(doc.scene().walk().len(), objects);
    assert_eq!(doc.undo_label(), Some("Delete 1 polygon"));
    assert!(matches!(
        last_operation(&harness),
        Some(Operation::DeleteElements { ref faces, .. }) if faces.len() == 1
    ));
}

/// Answers the question asked before dissolving across a sharp corner:
/// ticks "I know what I'm doing" and clicks Dissolve.
fn proceed(harness: &mut H) {
    assert!(
        harness
            .query_by_label(cleaning::DISSOLVE_QUESTION_TITLE)
            .is_some(),
        "the question is asked"
    );
    harness
        .get_by_label("I know what I'm doing, proceed")
        .click();
    harness.run();
    harness.get_by_label("Dissolve").click();
    harness.run();
}

#[test]
fn backspace_dissolves_and_a_refused_dissolve_records_nothing() {
    let mut harness = ready(Key::Num2, one_upright);
    press(&mut harness, Modifiers::NONE, Key::Backspace);
    // An upright edge of a cube is a sharp corner: it asks first.
    assert_eq!(shape(&harness).face_count(), 6);
    proceed(&mut harness);
    assert_eq!(shape(&harness).face_count(), 5);
    assert!(report(&harness).is_closed());
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Dissolve 1 edge")
    );
    // A border point can't be dissolved: nothing is recorded.
    let mut harness = ready(Key::Num3, top);
    press(&mut harness, Modifiers::NONE, Key::Delete);
    press(&mut harness, Modifiers::NONE, Key::Num1);
    let id = cube(&harness);
    let corner = shape(&harness)
        .vertices()
        .find(|(_, p)| p.z > 99.0)
        .map(|(v, _)| Element::Point(v))
        .unwrap();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements([(id, ElementSelection::of([corner]))], "Select", None);
    harness.run();
    let steps = harness.state().workbench.document().undo_steps();
    press(&mut harness, Modifiers::NONE, Key::Backspace);
    assert_eq!(harness.state().workbench.document().undo_steps(), steps);
    assert!(note(&harness).starts_with("Dissolve: nothing here can go"));
}

#[test]
fn fill_hole_closes_the_hole_and_says_when_there_is_none() {
    let mut harness = ready(Key::Num3, top);
    press(&mut harness, Modifiers::NONE, Key::Delete);
    // The deleted polygon's selection is gone: nothing selected fills
    // every hole.
    // J fills it.
    press(&mut harness, Modifiers::NONE, Key::J);
    assert!(report(&harness).is_closed());
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Fill every hole")
    );
    let steps = harness.state().workbench.document().undo_steps();
    run(&mut harness, crate::commands::CommandId::FillHole);
    assert_eq!(harness.state().workbench.document().undo_steps(), steps);
    assert!(note(&harness).contains("no holes"));
}

#[test]
fn flip_then_select_flipped_then_fix_facing() {
    let mut harness = ready(Key::Num3, top);
    run(&mut harness, crate::commands::CommandId::FlipNormals);
    assert_eq!(report(&harness).flipped_edges, 4);
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Flip 1 polygon")
    );
    // Select flipped finds that one polygon (from the Edges mode too: it
    // shows the Polygons mode).
    press(&mut harness, Modifiers::NONE, Key::Num2);
    harness
        .state_mut()
        .workbench
        .select_requests
        .push(SelectRequest::Flipped);
    harness.state_mut().workbench.mode = crate::palette::Mode::Polygons;
    harness.run();
    let id = cube(&harness);
    let selected = harness
        .state()
        .workbench
        .document()
        .elements(id)
        .unwrap()
        .polygons()
        .count();
    assert_eq!(selected, 1);
    assert!(note(&harness).starts_with("1 polygon facing the wrong way"));
    // Fix facing turns it back.
    harness.state_mut().workbench.run_fix_facing();
    harness.run();
    assert_eq!(report(&harness).flipped_edges, 0);
    assert!(matches!(
        last_operation(&harness),
        Some(Operation::FixFacing)
    ));
    // Nothing left to fix: nothing recorded.
    let steps = harness.state().workbench.document().undo_steps();
    harness.state_mut().workbench.run_fix_facing();
    assert_eq!(harness.state().workbench.document().undo_steps(), steps);
}

#[test]
fn select_similar_facing_keeps_the_selection_and_adds_alike_polygons() {
    let mut harness = fresh();
    let id = cube(&harness);
    let doc = harness.state_mut().workbench.document_mut();
    doc.set_selection([id]);
    doc.execute(scene::Command::SetPrimitive {
        ids: vec![id],
        primitive: scene::Primitive::Cube(mesh::CubeParams {
            segments: [2, 2, 1],
            ..Default::default()
        }),
    })
    .unwrap();
    harness.run();
    press(&mut harness, Modifiers::NONE, Key::Num3);
    // One of the top's four polygons.
    let first = top(&shape(&harness)).into_iter().take(1);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements([(id, ElementSelection::of(first))], "Select", None);
    harness
        .state_mut()
        .workbench
        .select_requests
        .push(SelectRequest::SimilarFacing);
    harness.run();
    let doc = harness.state().workbench.document();
    // The whole top, and none of the sides (all 90° away).
    assert_eq!(doc.elements(id).unwrap().polygons().count(), 4);
    assert_eq!(
        doc.undo_label(),
        Some("Select similar facing (4 polygons selected)")
    );
}

#[test]
fn the_clean_up_commands_say_where_they_work() {
    let mut harness = ready(Key::Num1, |_| Vec::new());
    run(&mut harness, crate::commands::CommandId::FlipNormals);
    assert!(note(&harness).starts_with("Flip normals works on polygons"));
    run(&mut harness, crate::commands::CommandId::FillHole);
    assert!(note(&harness).starts_with("Fill hole works in the Edges or Polygons mode"));
    // Nothing selected in the Points mode: Delete asks for a selection.
    press(&mut harness, Modifiers::NONE, Key::Delete);
    assert!(note(&harness).starts_with("Delete: select some points first"));
}

#[test]
fn a_dissolved_edge_is_no_longer_drawn_as_selected() {
    let mut harness = ready(Key::Num2, one_upright);
    let selected = |harness: &H| {
        harness
            .state()
            .workbench
            .components
            .overlay()
            .lines
            .iter()
            .filter(|(_, style)| *style == crate::OverlayStyle::Selected)
            .count()
    };
    assert_eq!(selected(&harness), 1);
    press(&mut harness, Modifiers::NONE, Key::Backspace);
    proceed(&mut harness);
    assert_eq!(shape(&harness).face_count(), 5);
    assert_eq!(selected(&harness), 0, "the gone edge isn't drawn");
}

/// A cube whose top is cut into four, in the Edges mode, with one of the
/// edges inside the top selected (flat: dissolving it asks nothing).
fn split_top() -> H {
    let mut harness = fresh();
    let id = cube(&harness);
    let doc = harness.state_mut().workbench.document_mut();
    doc.set_selection([id]);
    doc.execute(scene::Command::SetPrimitive {
        ids: vec![id],
        primitive: scene::Primitive::Cube(mesh::CubeParams {
            segments: [2, 2, 1],
            ..Default::default()
        }),
    })
    .unwrap();
    harness.run();
    press(&mut harness, Modifiers::NONE, Key::Num2);
    let mesh = shape(&harness);
    let inner = mesh
        .edges()
        .iter()
        .copied()
        .find(|e| {
            let (a, b) = e.ends();
            let (pa, pb) = (mesh.position(a).unwrap(), mesh.position(b).unwrap());
            pa.z > 99.0 && pb.z > 99.0 && pa.x.abs() < 1.0 && pb.x.abs() < 1.0
        })
        .unwrap();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements(
            [(id, ElementSelection::of([Element::Edge(inner)]))],
            "Select",
            None,
        );
    harness.run();
    harness
}

#[test]
fn a_flat_dissolve_asks_nothing() {
    let mut harness = split_top();
    let faces = shape(&harness).face_count();
    press(&mut harness, Modifiers::NONE, Key::Backspace);
    assert!(
        harness
            .query_by_label(cleaning::DISSOLVE_QUESTION_TITLE)
            .is_none()
    );
    assert_eq!(shape(&harness).face_count(), faces - 1);
}

#[test]
fn the_sharp_corner_question_can_be_cancelled_and_needs_the_tickbox() {
    let mut harness = ready(Key::Num2, one_upright);
    let steps = harness.state().workbench.document().undo_steps();
    press(&mut harness, Modifiers::NONE, Key::Backspace);
    // Dissolve is locked until the tickbox is ticked.
    harness.get_by_label("Dissolve").click();
    harness.run();
    assert_eq!(shape(&harness).face_count(), 6);
    assert!(
        harness
            .query_by_label(cleaning::DISSOLVE_QUESTION_TITLE)
            .is_some()
    );
    // While it is asked, the scene's keys do nothing.
    press(&mut harness, Modifiers::NONE, Key::Num1);
    assert_eq!(
        harness.state().workbench.mode(),
        crate::palette::Mode::Edges
    );
    harness.get_by_label("Cancel").click();
    harness.run();
    assert_eq!(shape(&harness).face_count(), 6);
    assert_eq!(harness.state().workbench.document().undo_steps(), steps);
    assert!(note(&harness).starts_with("Dissolve cancelled"));
    // Esc cancels too.
    press(&mut harness, Modifiers::NONE, Key::Backspace);
    press(&mut harness, Modifiers::NONE, Key::Escape);
    assert!(
        harness
            .query_by_label(cleaning::DISSOLVE_QUESTION_TITLE)
            .is_none()
    );
    assert_eq!(shape(&harness).face_count(), 6);
}

#[test]
fn dont_warn_me_again_is_remembered_and_can_be_switched_back() {
    let mut harness = ready(Key::Num2, one_upright);
    press(&mut harness, Modifiers::NONE, Key::Backspace);
    harness.get_by_label("Don't warn me again").click();
    harness.run();
    proceed(&mut harness);
    assert_eq!(shape(&harness).face_count(), 5);
    assert!(
        harness
            .state()
            .workbench
            .view_options()
            .dissolve_without_warning
    );
    // The next sharp dissolve goes straight ahead.
    let mut more = ready(Key::Num2, one_upright);
    more.state_mut()
        .workbench
        .viewport_mut()
        .options_mut()
        .dissolve_without_warning = true;
    press(&mut more, Modifiers::NONE, Key::Backspace);
    assert_eq!(shape(&more).face_count(), 5);
}
