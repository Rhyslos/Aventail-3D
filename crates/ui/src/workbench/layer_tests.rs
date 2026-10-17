//! Tests for edit layers in the window (decision 0091): the modifiers
//! palette's Edit layer button, the Mesh menu's entry and the Edit layers
//! panel.

use super::modelling_tests::{H, cube};
use super::tests::{fresh, main_menu, quick_harness};
use super::*;
use egui::{Event, PointerButton};
use egui_kittest::kittest::{NodeT, Queryable};
use scene::{Command, Operation, VertexId};

/// Each layer's name and how many edits it has, bottom first.
fn layers(harness: &H) -> Vec<(String, usize)> {
    let doc = harness.state().workbench.document();
    doc.scene()
        .object(cube(harness))
        .unwrap()
        .stack
        .layers()
        .iter()
        .map(|l| (l.name.clone(), l.entries.len()))
        .collect()
}

fn run(harness: &mut H, command: Command) {
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(command)
        .unwrap();
    harness.run();
}

/// Lifts the cube's first point.
fn lift(harness: &mut H) {
    let id = cube(harness);
    run(
        harness,
        Command::Record {
            id,
            operation: Operation::MovePoints {
                points: vec![VertexId::from_raw(0)],
                offset: glam::Vec3::Z * 10.0,
                anchors: None,
            },
        },
    );
}

/// The window with the cube selected, an edit in "Edits" and an empty
/// "Edits 2" on top, and the Edit layers panel in front. Frames are close
/// enough together for double-clicks.
fn with_two_layers() -> H {
    let mut harness = quick_harness();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    lift(&mut harness);
    run(&mut harness, Command::AddLayer(vec![id]));
    harness.get_by_label("Edit layers").click();
    harness.run();
    harness
}

/// A layer's row in the Edit layers panel (the Attributes panel has an
/// "Edits" heading too, higher up).
fn row<'h>(harness: &'h H, name: &'h str) -> egui_kittest::Node<'h> {
    harness
        .get_all_by_label(name)
        .max_by(|a, b| a.rect().min.y.total_cmp(&b.rect().min.y))
        .unwrap_or_else(|| panic!("no layer {name}"))
}

/// Where a layer's row is, scrolled into view first.
fn row_at(harness: &mut H, name: &str) -> egui::Pos2 {
    row(harness, name).scroll_to_me();
    harness.run();
    row(harness, name).rect().center()
}

fn click_at(harness: &mut H, at: egui::Pos2, button: PointerButton) {
    harness.event(Event::PointerMoved(at));
    harness.step();
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos: at,
            button,
            pressed,
            modifiers: Modifiers::NONE,
        });
        harness.step();
    }
    harness.run();
}

#[test]
fn the_edit_layer_button_needs_a_shape_selected() {
    let harness = fresh();
    let button = harness.get_by_label(crate::layers::ADD_LAYER_LABEL);
    assert!(button.accesskit_node().is_disabled());
}

#[test]
fn the_edit_layer_button_adds_a_layer_and_shows_the_panel() {
    let mut harness = fresh();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    let button = harness.get_by_label(crate::layers::ADD_LAYER_LABEL);
    // It sits on the right of the viewport, left of the panels.
    let viewport = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let objects = harness.get_by_label("Objects").rect();
    assert!(button.rect().min.x > viewport.center().x);
    assert!(button.rect().max.x <= objects.min.x + 1.0);
    button.click();
    harness.run();
    assert_eq!(layers(&harness), vec![("Edits".to_string(), 0)]);
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Add edit layer")
    );
    // The Edit layers panel came to the front and lists the layer.
    assert!(row(&harness, "Edits").rect().min.y > objects.min.y);
    assert!(harness.query_by_label_contains("No edits").is_some());
}

#[test]
fn the_mesh_menu_adds_a_layer_too() {
    let mut harness = fresh();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    main_menu(&harness, "Mesh").click();
    harness.run();
    harness
        .get_all_by_label(crate::layers::ADD_LAYER_LABEL)
        .max_by(|a, b| a.rect().min.y.total_cmp(&b.rect().min.y))
        .unwrap()
        .click();
    harness.run();
    assert_eq!(layers(&harness).len(), 1);
}

#[test]
fn the_panel_lists_layers_top_first_with_their_edits() {
    let harness = with_two_layers();
    let top = row(&harness, "Edits 2").rect();
    let bottom = row(&harness, "Edits").rect();
    assert!(top.min.y < bottom.min.y, "the top layer is listed first");
    assert!(harness.query_by_label("Move 1 point").is_some());
    assert!(harness.query_by_label_contains("new edits").is_some());
}

#[test]
fn the_eye_switches_a_layer_off() {
    let mut harness = with_two_layers();
    harness.get_by_label("Switch off Edits").click();
    harness.run();
    let doc = harness.state().workbench.document();
    let stack = &doc.scene().object(cube(&harness)).unwrap().stack;
    assert!(!stack.layers()[0].enabled);
    assert_eq!(doc.undo_label(), Some("Switch layer off"));
}

#[test]
fn the_right_click_menu_deletes_a_layer() {
    let mut harness = with_two_layers();
    let at = row_at(&mut harness, "Edits");
    click_at(&mut harness, at, PointerButton::Secondary);
    harness.get_by_label("Delete layer").click();
    harness.run();
    assert_eq!(layers(&harness), vec![("Edits 2".to_string(), 0)]);
}

#[test]
fn the_layer_menu_moves_the_layer_clicked() {
    let mut harness = with_two_layers();
    let at = row_at(&mut harness, "Edits");
    click_at(&mut harness, at, PointerButton::Primary);
    harness.get_by_label(crate::layers::LAYER_MENU).click();
    harness.run();
    harness.get_by_label("Move up").click();
    harness.run();
    assert_eq!(
        layers(&harness),
        vec![("Edits 2".to_string(), 0), ("Edits".to_string(), 1)]
    );
}

#[test]
fn double_clicking_a_layer_renames_it() {
    let mut harness = with_two_layers();
    let at = row_at(&mut harness, "Edits 2");
    // Let a second pass after the window opened (egui would take clicks
    // so soon after its start for a triple-click).
    for _ in 0..60 {
        harness.step();
    }
    harness.event(Event::PointerMoved(at));
    harness.step();
    for _ in 0..2 {
        for pressed in [true, false] {
            harness.event(Event::PointerButton {
                pos: at,
                button: PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            });
            harness.step();
        }
    }
    harness.run();
    assert!(harness.state().workbench.layers_panel().is_renaming());
    harness.event(Event::Text("Details".into()));
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(layers(&harness)[1].0, "Details");
    assert!(!harness.state().workbench.layers_panel().is_renaming());
}

#[test]
fn the_panel_explains_when_it_has_nothing_to_show() {
    let mut harness = fresh();
    harness.get_by_label("Edit layers").click();
    harness.run();
    assert!(
        harness
            .query_by_label_contains("Select an object to see its edit layers")
            .is_some()
    );
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    assert!(
        harness
            .query_by_label_contains("No edit layers yet")
            .is_some()
    );
}

// ------------------------------------------------------------ edit list ---

/// The cube's first layer's first edit.
fn first_edit(harness: &H) -> scene::Entry {
    let doc = harness.state().workbench.document();
    doc.scene().object(cube(harness)).unwrap().stack.layers()[0].entries[0].clone()
}

/// Clicks the edit row named `name` (the lowest one, if there are more).
fn open_edit(harness: &mut H, name: &str) {
    let at = row_at(harness, name);
    click_at(harness, at, PointerButton::Primary);
}

#[test]
fn clicking_an_edit_opens_it_in_the_attributes_panel() {
    let mut harness = with_two_layers();
    open_edit(&mut harness, "Move 1 point");
    assert!(harness.query_by_label("Edit: Move 1 point").is_some());
    assert!(
        harness
            .query_by_label_contains("Edit 1 of 1 in Edits")
            .is_some()
    );
    harness
        .get_by_label(crate::attributes::CLOSE_EDIT_LABEL)
        .click();
    harness.run();
    assert!(harness.query_by_label("Edit: Move 1 point").is_none());
}

#[test]
fn an_opened_edit_can_be_switched_off_kept_and_deleted() {
    let mut harness = with_two_layers();
    open_edit(&mut harness, "Move 1 point");
    harness.get_by_label("Edit on").click();
    harness.run();
    assert!(!first_edit(&harness).enabled);
    harness.get_by_label("Edit on").click();
    harness.run();
    harness
        .get_by_label(crate::attributes::KEEP_EDITABLE_SWITCH)
        .click();
    harness.run();
    assert!(first_edit(&harness).pinned);
    assert!(harness.query_by_label(crate::layers::KEPT_MARK).is_some());
    // A kept edit is never frozen.
    assert!(
        harness
            .get_by_label(crate::attributes::FREEZE_LABEL)
            .accesskit_node()
            .is_disabled()
    );
    harness
        .get_by_label(crate::attributes::DELETE_EDIT_LABEL)
        .click();
    harness.run();
    assert_eq!(
        layers(&harness),
        vec![("Edits".to_string(), 0), ("Edits 2".to_string(), 0)]
    );
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Delete move 1 point")
    );
}

#[test]
fn freeze_up_to_here_bakes_the_edits_below_into_the_shape() {
    let mut harness = with_two_layers();
    let before = super::modelling_tests::shape(&harness);
    open_edit(&mut harness, "Move 1 point");
    harness
        .get_by_label(crate::attributes::FREEZE_LABEL)
        .click();
    harness.run();
    let doc = harness.state().workbench.document();
    assert!(matches!(
        doc.scene().object(cube(&harness)).unwrap().kind,
        scene::ObjectKind::Mesh(_)
    ));
    assert_eq!(doc.undo_label(), Some("Freeze 1 edit"));
    let after = super::modelling_tests::shape(&harness);
    let positions = |m: &scene::Mesh| m.vertices().map(|(_, p)| p).collect::<Vec<_>>();
    assert_eq!(positions(&after), positions(&before));
}

#[test]
fn a_new_edit_closes_the_opened_one() {
    let mut harness = with_two_layers();
    open_edit(&mut harness, "Move 1 point");
    assert!(harness.query_by_label("Edit: Move 1 point").is_some());
    lift(&mut harness);
    assert!(harness.query_by_label("Edit: Move 1 point").is_none());
}

#[test]
fn an_edits_eye_switches_it_off() {
    let mut harness = with_two_layers();
    row_at(&mut harness, "Move 1 point");
    harness.get_by_label("Switch off Move 1 point").click();
    harness.run();
    assert!(!first_edit(&harness).enabled);
    assert!(
        harness
            .query_by_label_contains("Switch on Move 1 point")
            .is_some()
    );
}

#[test]
fn an_edit_that_lost_its_points_says_so() {
    let mut harness = with_two_layers();
    // An edit on a cube with more segments; with fewer, its point is gone.
    let id = cube(&harness);
    let segments = |n: u32| Command::SetPrimitive {
        ids: vec![id],
        primitive: scene::Primitive::Cube(mesh::CubeParams {
            segments: [n, n, n],
            ..Default::default()
        }),
    };
    run(&mut harness, segments(3));
    run(
        &mut harness,
        Command::Record {
            id,
            operation: Operation::MovePoints {
                points: vec![VertexId::from_raw(40)],
                offset: glam::Vec3::Z,
                anchors: None,
            },
        },
    );
    run(&mut harness, segments(1));
    harness.get_by_label("Edit layers").click();
    harness.run();
    assert!(
        harness
            .query_by_label(crate::layers::MISSING_MARK)
            .is_some()
    );
}

#[test]
fn changing_an_earlier_edit_keeps_the_later_ones() {
    let mut harness = quick_harness();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    let mesh = super::modelling_tests::shape(&harness);
    let top = mesh
        .faces()
        .map(|(f, _)| f)
        .max_by(|a, b| mesh.face_normal(*a).z.total_cmp(&mesh.face_normal(*b).z))
        .unwrap();
    let corners = vec![mesh.face(top).unwrap().to_vec()];
    run(
        &mut harness,
        Command::Record {
            id,
            operation: Operation::ExtrudePolygons {
                faces: vec![top],
                distance: 40.0,
                grouping: scene::Grouping::Together,
                shape: scene::ExtrudeShape::EARLIER,
                anchors: mesh::Anchors::record(&mesh, scene::ElementKind::Polygons, &corners),
                stretch: glam::Vec3::ONE,
            },
        },
    );
    run(&mut harness, Command::AddLayer(vec![id]));
    lift(&mut harness);
    harness.get_by_label("Edit layers").click();
    harness.run();
    open_edit(&mut harness, "Extrude 1 polygon");
    let height = |h: &H| {
        super::modelling_tests::shape(h)
            .bounds()
            .map_or(0.0, |b| b.max.z)
    };
    let before = height(&harness);
    let at = harness.get_by_label("Increase Distance").rect().center();
    click_at(&mut harness, at, PointerButton::Primary);
    assert!(height(&harness) > before, "the old extrude grew");
    assert_eq!(layers(&harness)[1].1, 1, "the later edit is still there");
    // It stays open for more changes.
    assert!(harness.query_by_label("Edit: Extrude 1 polygon").is_some());
}
