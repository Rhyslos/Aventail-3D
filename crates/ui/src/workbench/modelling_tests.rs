//! Tests for Extrude and Inset in the window: the keys, dragging anywhere
//! in the view as one undo step, fine steps, cancelling, the Attributes
//! panel's Keep together switch and last edit, and extruding border edges.

use super::tests::{State, camera, fresh};
use super::*;
use crate::transforming::Tool;
use egui::{Event, PointerButton, Pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use scene::{Element, ElementSelection, FaceId, Grouping, ObjectId, Operation};

pub(super) type H = Harness<'static, State>;

pub(super) fn press(harness: &mut H, modifiers: Modifiers, key: Key) {
    harness.event(Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    });
    harness.event(Event::Key {
        key,
        physical_key: None,
        pressed: false,
        repeat: false,
        modifiers,
    });
    harness.run();
}

pub(super) fn cube(harness: &H) -> ObjectId {
    harness.state().workbench.document().scene().children(None)[0]
}

pub(super) fn shape(harness: &H) -> std::sync::Arc<scene::Mesh> {
    let doc = harness.state().workbench.document();
    doc.scene().evaluated(cube(harness)).unwrap().mesh
}

fn top_face(mesh: &scene::Mesh) -> FaceId {
    mesh.faces()
        .map(|(f, _)| f)
        .max_by(|a, b| mesh.face_normal(*a).z.total_cmp(&mesh.face_normal(*b).z))
        .unwrap()
}

/// The fresh window's cube selected, in the Polygons mode, its top
/// polygon selected, and `key`'s tool in use.
fn ready(key: Key) -> H {
    let mut harness = fresh();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    press(&mut harness, Modifiers::NONE, Key::Num3);
    let top = top_face(&shape(&harness));
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements(
            [(id, ElementSelection::of([Element::Polygon(top)]))],
            "Select",
            None,
        );
    harness.run();
    press(&mut harness, Modifiers::NONE, key);
    harness
}

pub(super) fn button(
    harness: &mut H,
    at: Pos2,
    pressed: bool,
    button: PointerButton,
    modifiers: Modifiers,
) {
    harness.event(Event::PointerButton {
        pos: at,
        button,
        pressed,
        modifiers,
    });
    harness.run();
}

/// A spot in the view away from the cube.
fn empty_spot(harness: &H) -> Pos2 {
    harness
        .get_by_label(crate::VIEWPORT_LABEL)
        .rect()
        .left_top()
        + egui::vec2(60.0, 80.0)
}

/// Presses at `from` and moves `dx` points right and `dx` up (up and
/// right push the cube's top out), in steps, without letting go.
fn drag_without_letting_go(harness: &mut H, from: Pos2, dx: f32) -> Pos2 {
    harness.event(Event::PointerMoved(from));
    harness.run();
    button(harness, from, true, PointerButton::Primary, Modifiers::NONE);
    let mut at = from;
    for _ in 0..5 {
        at += egui::vec2(dx / 5.0, -dx / 5.0);
        harness.event(Event::PointerMoved(at));
        harness.run();
    }
    at
}

fn drag(harness: &mut H, dx: f32) {
    let from = empty_spot(harness);
    let at = drag_without_letting_go(harness, from, dx);
    button(harness, at, false, PointerButton::Primary, Modifiers::NONE);
}

/// Where `world` appears on screen.
pub(super) fn on_screen(harness: &H, world: glam::Vec3) -> Pos2 {
    let rect = harness.get_by_label(crate::VIEWPORT_LABEL).rect();
    let size = glam::Vec2::new(rect.width(), rect.height());
    let at = camera(harness).project(world, size).expect("in view");
    rect.min + egui::vec2(at.x, at.y)
}

/// Drags from `from` by `by` points, in steps, and lets go.
fn drag_by(harness: &mut H, from: Pos2, by: egui::Vec2) {
    harness.event(Event::PointerMoved(from));
    harness.run();
    button(harness, from, true, PointerButton::Primary, Modifiers::NONE);
    let mut at = from;
    for _ in 0..5 {
        at += by / 5.0;
        harness.event(Event::PointerMoved(at));
        harness.run();
    }
    button(harness, at, false, PointerButton::Primary, Modifiers::NONE);
}

/// The way world +X points on screen, `length` points long.
fn screen_x(harness: &H, length: f32) -> egui::Vec2 {
    let way = on_screen(harness, glam::Vec3::X * 100.0) - on_screen(harness, glam::Vec3::ZERO);
    way.normalized() * length
}

fn top_height(harness: &H) -> f32 {
    shape(harness).bounds().unwrap().max.z
}

/// How high the (first selected) top polygon is.
fn top_polygon_height(harness: &H) -> f32 {
    let mesh = shape(harness);
    let doc = harness.state().workbench.document();
    let face = doc
        .elements(cube(harness))
        .unwrap()
        .polygons()
        .next()
        .unwrap();
    let corners = mesh.face(face).unwrap();
    corners
        .iter()
        .map(|v| mesh.position(*v).unwrap().z)
        .sum::<f32>()
        / corners.len() as f32
}

/// Clicks the `n`th Keep together switch; returns how many there are.
fn click_keep_together(harness: &mut H, n: usize) -> usize {
    let switches: Vec<egui::Rect> = harness
        .get_all_by_label("Keep together")
        .filter(|node| node.accesskit_node().role() == egui::accesskit::Role::CheckBox)
        .map(|node| node.rect())
        .collect();
    let count = switches.len();
    let at = switches[n].center();
    harness.event(Event::PointerMoved(at));
    harness.run();
    button(harness, at, true, PointerButton::Primary, Modifiers::NONE);
    button(harness, at, false, PointerButton::Primary, Modifiers::NONE);
    count
}

pub(super) fn note(harness: &H) -> String {
    harness.state().workbench.note.clone().unwrap_or_default()
}

pub(super) fn last_operation(harness: &H) -> Option<Operation> {
    let doc = harness.state().workbench.document();
    let object = doc.scene().object(cube(harness))?;
    object
        .stack
        .layers()
        .last()?
        .entries
        .last()
        .map(|e| e.operation.clone())
}

#[test]
fn d_and_i_pick_extrude_and_inset() {
    let mut harness = ready(Key::D);
    assert_eq!(harness.state().workbench.tool(), Tool::Extrude);
    assert!(note(&harness).starts_with("Extrude: drag anywhere"));
    press(&mut harness, Modifiers::NONE, Key::I);
    assert_eq!(harness.state().workbench.tool(), Tool::Inset);
    // Ctrl + I still inverts the selection: it doesn't pick Inset.
    press(&mut harness, Modifiers::NONE, Key::Q);
    press(&mut harness, Modifiers::COMMAND, Key::I);
    assert_ne!(harness.state().workbench.tool(), Tool::Inset);
}

#[test]
fn extrude_and_inset_need_the_right_mode() {
    let mut harness = fresh();
    press(&mut harness, Modifiers::NONE, Key::D);
    assert_eq!(harness.state().workbench.tool(), Tool::Select);
    assert!(note(&harness).contains("press 3 for the Polygons mode"));
    press(&mut harness, Modifiers::NONE, Key::I);
    assert_eq!(harness.state().workbench.tool(), Tool::Select);
    assert!(note(&harness).starts_with("Inset works on polygons"));
}

#[test]
fn a_drag_anywhere_extrudes_as_one_undo_step() {
    let mut harness = ready(Key::D);
    let steps = harness.state().workbench.document().undo_steps();
    drag(&mut harness, 120.0);
    assert_eq!(shape(&harness).face_count(), 10);
    assert!(top_height(&harness) > 101.0, "pushed up");
    let doc = harness.state().workbench.document();
    assert_eq!(doc.undo_steps(), steps + 1, "one step for the whole drag");
    assert_eq!(doc.undo_label(), Some("Extrude 1 polygon"));
    assert!(note(&harness).starts_with("Extrude 1 polygon: "));
    // Dragging left pushes in.
    harness.state_mut().workbench.document_mut().undo();
    harness.run();
    drag(&mut harness, -120.0);
    assert!(top_polygon_height(&harness) < 99.9, "pushed in");
    // The polygon is still selected, so another drag carries on.
    drag(&mut harness, -40.0);
    assert_eq!(shape(&harness).face_count(), 14);
}

#[test]
fn shift_makes_the_drag_ten_times_finer() {
    let mut harness = ready(Key::D);
    drag(&mut harness, 100.0);
    let coarse = top_height(&harness) - 100.0;
    harness.state_mut().workbench.document_mut().undo();
    harness.run();
    harness.event(Event::ModifiersChanged(Modifiers::SHIFT));
    harness.run();
    let from = empty_spot(&harness);
    let at = drag_without_letting_go(&mut harness, from, 100.0);
    button(
        &mut harness,
        at,
        false,
        PointerButton::Primary,
        Modifiers::SHIFT,
    );
    harness.event(Event::ModifiersChanged(Modifiers::NONE));
    harness.run();
    let fine = top_height(&harness) - 100.0;
    assert!(fine > 0.0 && fine < coarse * 0.2, "{fine} vs {coarse}");
}

#[test]
fn esc_or_a_right_click_cancels_and_letting_go_at_nothing_leaves_nothing() {
    for cancel in ["esc", "right"] {
        let mut harness = ready(Key::D);
        let steps = harness.state().workbench.document().undo_steps();
        let from = empty_spot(&harness);
        let at = drag_without_letting_go(&mut harness, from, 150.0);
        assert_eq!(shape(&harness).face_count(), 10, "shown while dragging");
        if cancel == "esc" {
            press(&mut harness, Modifiers::NONE, Key::Escape);
        } else {
            button(
                &mut harness,
                at,
                true,
                PointerButton::Secondary,
                Modifiers::NONE,
            );
            button(
                &mut harness,
                at,
                false,
                PointerButton::Secondary,
                Modifiers::NONE,
            );
        }
        button(
            &mut harness,
            at,
            false,
            PointerButton::Primary,
            Modifiers::NONE,
        );
        assert_eq!(shape(&harness).face_count(), 6, "{cancel}");
        assert_eq!(harness.state().workbench.document().undo_steps(), steps);
        assert!(note(&harness).starts_with("Cancelled"));
        // The selection and tool stay.
        assert_eq!(harness.state().workbench.tool(), Tool::Extrude);
    }
    // There and back again: nothing.
    let mut harness = ready(Key::D);
    let steps = harness.state().workbench.document().undo_steps();
    let from = empty_spot(&harness);
    let at = drag_without_letting_go(&mut harness, from, 100.0);
    harness.event(Event::PointerMoved(from));
    harness.run();
    button(
        &mut harness,
        from,
        false,
        PointerButton::Primary,
        Modifiers::NONE,
    );
    let _ = at;
    assert_eq!(shape(&harness).face_count(), 6);
    assert_eq!(harness.state().workbench.document().undo_steps(), steps);
    assert!(note(&harness).contains("nothing changed"));
}

#[test]
fn inset_follows_the_pull_towards_the_middle() {
    let mut harness = ready(Key::I);
    let middle = on_screen(&harness, glam::Vec3::new(0.0, 0.0, 100.0));
    // From the left, pulling right (towards the middle) insets.
    let left = middle - egui::vec2(150.0, 0.0);
    drag_by(&mut harness, left, egui::vec2(60.0, 0.0));
    let mesh = shape(&harness);
    assert_eq!(mesh.face_count(), 10);
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Inset 1 polygon")
    );
    // The cube's outline is unchanged: the inset stays on the top.
    assert!((top_height(&harness) - 100.0).abs() < 1e-3);
    harness.state_mut().workbench.document_mut().undo();
    harness.run();
    // From the right, it is pulling left that insets...
    let right = middle + egui::vec2(150.0, 0.0);
    drag_by(&mut harness, right, egui::vec2(-60.0, 0.0));
    assert_eq!(shape(&harness).face_count(), 10);
    harness.state_mut().workbench.document_mut().undo();
    harness.run();
    // ...and pulling away from the middle never grows the polygon.
    let steps = harness.state().workbench.document().undo_steps();
    drag_by(&mut harness, right, egui::vec2(60.0, 0.0));
    assert_eq!(harness.state().workbench.document().undo_steps(), steps);
}

#[test]
fn the_last_extrude_can_be_changed_in_the_attributes_panel() {
    let mut harness = ready(Key::D);
    drag(&mut harness, 100.0);
    assert!(
        harness
            .query_by_label_contains("Last edit: Extrude 1 polygon")
            .is_some()
    );
    assert!(harness.query_all_by_label("Distance").count() >= 1);
    // Switching Keep together off on the last edit changes the edit, as
    // an undo step. It is the only switch: the tool's own is hidden while
    // the last edit shows.
    let steps = harness.state().workbench.document().undo_steps();
    let count = click_keep_together(&mut harness, 0);
    assert_eq!(count, 1, "one switch");
    assert_eq!(
        last_operation(&harness).and_then(|op| op.grouping()),
        Some(Grouping::Separate)
    );
    let doc = harness.state().workbench.document();
    assert_eq!(doc.undo_steps(), steps + 1);
    assert_eq!(doc.undo_label(), Some("Change Extrude 1 polygon"));
    // ...and the next drag uses it too.
    assert_eq!(harness.state().workbench.grouping(), Grouping::Separate);
}

#[test]
fn the_tools_keep_together_switch_sets_the_next_drag() {
    let mut harness = ready(Key::D);
    assert_eq!(click_keep_together(&mut harness, 0), 1);
    assert_eq!(harness.state().workbench.grouping(), Grouping::Separate);
    drag(&mut harness, 80.0);
    assert_eq!(
        last_operation(&harness).and_then(|op| op.grouping()),
        Some(Grouping::Separate)
    );
}

#[test]
fn the_mesh_menu_and_tool_menu_pick_extrude() {
    let mut harness = ready(Key::Q);
    super::tests::main_menu(&harness, "Mesh").click();
    harness.run();
    harness.get_by_label_contains("Extrude").click();
    harness.run();
    assert_eq!(harness.state().workbench.tool(), Tool::Extrude);
}

#[test]
fn border_edges_extrude_and_the_selection_follows() {
    let mut harness = fresh();
    // A plane instead of the cube.
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::Create(scene::ObjectKind::Primitive(
            scene::Primitive::Plane(Default::default()),
        )))
        .unwrap();
    harness.run();
    let plane = *harness
        .state()
        .workbench
        .document()
        .selection()
        .first()
        .unwrap();
    press(&mut harness, Modifiers::NONE, Key::Num2);
    let mesh = harness
        .state()
        .workbench
        .document()
        .scene()
        .evaluated(plane)
        .unwrap()
        .mesh;
    // The edge along the plane's +X side, which grows towards +X.
    let edge = *mesh
        .edges()
        .iter()
        .find(|e| {
            let (a, b) = e.ends();
            mesh.position(a).unwrap().x > 49.0 && mesh.position(b).unwrap().x > 49.0
        })
        .unwrap();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements(
            [(plane, ElementSelection::of([Element::Edge(edge)]))],
            "Select",
            None,
        );
    harness.run();
    press(&mut harness, Modifiers::NONE, Key::D);
    let way = screen_x(&harness, 80.0);
    let from = empty_spot(&harness);
    drag_by(&mut harness, from, way);
    let faces = |h: &H| {
        h.state()
            .workbench
            .document()
            .scene()
            .evaluated(plane)
            .unwrap()
            .mesh
            .face_count()
    };
    assert_eq!(faces(&harness), 2);
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Extrude 1 edge")
    );
    let reach = |h: &H| {
        h.state()
            .workbench
            .document()
            .scene()
            .evaluated(plane)
            .unwrap()
            .mesh
            .bounds()
            .unwrap()
            .max
            .x
    };
    assert!(reach(&harness) > 51.0, "grew the way it was dragged");
    // The new outer edge is selected now, so the next drag carries on.
    let selected = harness
        .state()
        .workbench
        .document()
        .elements(plane)
        .unwrap()
        .clone();
    assert_eq!(selected.count(scene::ElementKind::Edges), 1);
    assert!(!selected.contains(Element::Edge(edge)));
    drag_by(&mut harness, from, way);
    assert_eq!(faces(&harness), 3);
    // One undo takes back the second strip and its selection together.
    harness.state_mut().workbench.document_mut().undo();
    assert_eq!(faces(&harness), 2);
}

#[test]
fn extrude_follows_the_way_the_polygon_faces_on_screen() {
    // The cube's +X side: dragging the way +X points on screen pushes it
    // out, the other way pushes it in, whichever way that is on screen.
    let mut harness = ready(Key::Q);
    let id = cube(&harness);
    let mesh = shape(&harness);
    let side = mesh
        .faces()
        .map(|(f, _)| f)
        .max_by(|a, b| mesh.face_normal(*a).x.total_cmp(&mesh.face_normal(*b).x))
        .unwrap();
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements(
            [(id, ElementSelection::of([Element::Polygon(side)]))],
            "Select",
            None,
        );
    harness.run();
    press(&mut harness, Modifiers::NONE, Key::D);
    let reach = |h: &H| shape(h).bounds().unwrap().max.x;
    let from = empty_spot(&harness);
    let way = screen_x(&harness, 80.0);
    drag_by(&mut harness, from, way);
    assert!(reach(&harness) > 55.0, "out: {}", reach(&harness));
    harness.state_mut().workbench.document_mut().undo();
    harness.run();
    drag_by(&mut harness, from, -way);
    assert!(reach(&harness) < 50.01, "in: {}", reach(&harness));
    // Sideways to that way does almost nothing.
    harness.state_mut().workbench.document_mut().undo();
    harness.run();
    let across = egui::vec2(-way.y, way.x);
    drag_by(&mut harness, from, across);
    assert!((reach(&harness) - 50.0).abs() < 5.0, "{}", reach(&harness));
}

#[test]
fn dragging_on_another_polygon_selects_it_first() {
    // The top is selected; a drag that starts on the +X side extrudes
    // the side instead, without clicking it first.
    let mut harness = ready(Key::D);
    let id = cube(&harness);
    let side_centre = on_screen(&harness, glam::Vec3::new(50.0, 0.0, 50.0));
    // Rest the mouse there so the polygon under it is known.
    harness.event(Event::PointerMoved(side_centre));
    harness.run();
    harness.run();
    let way = screen_x(&harness, 80.0);
    drag_by(&mut harness, side_centre, way);
    let doc = harness.state().workbench.document();
    let selected: Vec<FaceId> = doc.elements(id).unwrap().polygons().collect();
    assert_eq!(selected.len(), 1);
    let mesh = shape(&harness);
    assert!(mesh.bounds().unwrap().max.x > 55.0, "the side came out");
    assert!(
        (mesh.bounds().unwrap().max.z - 100.0).abs() < 1e-3,
        "the top stayed"
    );
    // Undo takes back the extrude, then the selection.
    harness.state_mut().workbench.document_mut().undo();
    harness.state_mut().workbench.document_mut().undo();
    harness.run();
    let top = top_face(&shape(&harness));
    let doc = harness.state().workbench.document();
    assert!(doc.elements(id).unwrap().contains(Element::Polygon(top)));
}

// ---------------------------------------------------------------- bevel

/// The fresh window's cube selected, in `mode_key`'s mode, with `element`
/// selected and Bevel in use.
fn bevel_ready(mode_key: Key, element: impl Fn(&scene::Mesh) -> Element) -> H {
    let mut harness = fresh();
    let id = cube(&harness);
    harness
        .state_mut()
        .workbench
        .document_mut()
        .set_selection([id]);
    harness.run();
    press(&mut harness, Modifiers::NONE, mode_key);
    let chosen = element(&shape(&harness));
    harness
        .state_mut()
        .workbench
        .document_mut()
        .select_elements([(id, ElementSelection::of([chosen]))], "Select", None);
    harness.run();
    press(&mut harness, Modifiers::NONE, Key::V);
    harness
}

/// The cube's top edge facing the camera's right side (+X, top).
fn top_right_edge(mesh: &scene::Mesh) -> Element {
    let edge = *mesh
        .edges()
        .iter()
        .find(|e| {
            let (a, b) = e.ends();
            let (pa, pb) = (mesh.position(a).unwrap(), mesh.position(b).unwrap());
            pa.x > 49.0 && pb.x > 49.0 && pa.z > 99.0 && pb.z > 99.0
        })
        .unwrap();
    Element::Edge(edge)
}

/// The middle of the cube's top +X edge on screen.
fn edge_middle(harness: &H) -> Pos2 {
    on_screen(harness, glam::Vec3::new(50.0, 0.0, 100.0))
}

#[test]
fn v_picks_bevel_where_it_works() {
    let mut harness = bevel_ready(Key::Num2, top_right_edge);
    assert_eq!(harness.state().workbench.tool(), Tool::Bevel);
    assert!(note(&harness).starts_with("Bevel: drag away"));
    // In the Polygons mode it says which modes it needs.
    press(&mut harness, Modifiers::NONE, Key::Q);
    press(&mut harness, Modifiers::NONE, Key::Num3);
    press(&mut harness, Modifiers::NONE, Key::V);
    assert_eq!(harness.state().workbench.tool(), Tool::Select);
    assert!(note(&harness).starts_with("Bevel works on edges or points"));
}

#[test]
fn dragging_away_from_the_edge_bevels_it() {
    let mut harness = bevel_ready(Key::Num2, top_right_edge);
    let steps = harness.state().workbench.document().undo_steps();
    // From the edge, straight up the screen: away from it.
    let from = edge_middle(&harness);
    drag_by(&mut harness, from, egui::vec2(0.0, -60.0));
    assert_eq!(shape(&harness).face_count(), 7);
    let doc = harness.state().workbench.document();
    assert_eq!(doc.undo_steps(), steps + 1);
    assert_eq!(doc.undo_label(), Some("Bevel 1 edge"));
    let Some(Operation::BevelEdges { shape: s, .. }) = last_operation(&harness) else {
        panic!("a bevel");
    };
    assert!(s.width > 1.0, "{}", s.width);
    assert_eq!((s.segments, s.profile), (1, 1.0));
}

#[test]
fn dragging_towards_the_edge_never_bevels() {
    let mut harness = bevel_ready(Key::Num2, top_right_edge);
    let steps = harness.state().workbench.document().undo_steps();
    let middle = edge_middle(&harness);
    let from = middle + egui::vec2(0.0, -120.0);
    drag_by(&mut harness, from, egui::vec2(0.0, 100.0));
    assert_eq!(harness.state().workbench.document().undo_steps(), steps);
    assert_eq!(shape(&harness).face_count(), 6);
}

#[test]
fn segments_and_profile_are_set_before_and_changed_after() {
    let mut harness = bevel_ready(Key::Num2, top_right_edge);
    // The tool's settings: Segments and Profile, no Keep together.
    assert!(harness.query_all_by_label("Segments").count() >= 1);
    assert!(harness.query_all_by_label("Profile").count() >= 1);
    assert!(harness.query_by_label("Keep together").is_none());
    harness.state_mut().workbench.attributes.bevel.segments = 3;
    harness.run();
    let from = edge_middle(&harness);
    drag_by(&mut harness, from, egui::vec2(0.0, -60.0));
    let Some(Operation::BevelEdges { shape: s, .. }) = last_operation(&harness) else {
        panic!("a bevel");
    };
    assert_eq!(s.segments, 3);
    assert_eq!(shape(&harness).face_count(), 6 + 3);
    // Afterwards, as the Last edit, with Width.
    assert!(
        harness
            .query_by_label_contains("Last edit: Bevel 1 edge")
            .is_some()
    );
    assert!(harness.query_all_by_label("Width").count() >= 1);
}

#[test]
fn a_hollow_profile_is_recorded_as_set() {
    let mut harness = bevel_ready(Key::Num2, top_right_edge);
    harness.state_mut().workbench.attributes.bevel.segments = 3;
    harness.state_mut().workbench.attributes.bevel.profile = -1.0;
    harness.run();
    let from = edge_middle(&harness);
    drag_by(&mut harness, from, egui::vec2(0.0, -60.0));
    let Some(Operation::BevelEdges { shape: s, .. }) = last_operation(&harness) else {
        panic!("a bevel");
    };
    assert_eq!((s.segments, s.profile), (3, -1.0));
    assert_eq!(shape(&harness).face_count(), 6 + 3);
}

#[test]
fn a_corner_point_bevels_too() {
    let mut harness = bevel_ready(Key::Num1, |mesh| {
        let corner = mesh
            .vertices()
            .find(|(_, p)| p.x > 49.0 && p.y < -49.0 && p.z > 99.0)
            .unwrap()
            .0;
        Element::Point(corner)
    });
    assert_eq!(harness.state().workbench.tool(), Tool::Bevel);
    let from = on_screen(&harness, glam::Vec3::new(50.0, -50.0, 100.0));
    drag_by(&mut harness, from, egui::vec2(-50.0, -50.0));
    assert_eq!(shape(&harness).face_count(), 7);
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Bevel 1 point")
    );
}
