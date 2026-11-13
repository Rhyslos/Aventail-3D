//! Stamps (decision 0285), used through the real interface: Attributes
//! and a double-click asking for a picture, each picture handed back as
//! one undo step and added to the list, the box's plain refusal with
//! nothing brought in, Take off, a full box, a picture without
//! see-through parts, the box copied and pasted (in the same project and
//! another), and the stamps saved with the project.

use super::material_mode_tests::{H, box_at, click, graph, quick_page, undo};
use super::*;
use crate::material::look::COLUMN;
use crate::material::stamp_rows::{ADD_LABEL, TAKE_OFF_LABEL};
use egui::Modifiers;
use egui_kittest::kittest::Queryable;
use glam::Vec2;
use matgraph::boxes::stamps::{NO_STAMPS, STAMPS};
use matgraph::{BoxId, GraphEdit, GraphId, MAX_STAMPS, Stamp};

/// The Stamps box added to the page's graph.
const BOX: BoxId = BoxId(6);

/// The page with a Stamps box beside its boxes, selected, nothing
/// brought in yet.
fn with_stamps_box() -> H {
    let mut harness = quick_page();
    let edit = GraphEdit::AddBox {
        kind: STAMPS.into(),
        at: Vec2::new(4.0 * COLUMN, 0.0),
    };
    harness
        .state_mut()
        .workbench
        .document_mut()
        .execute(scene::Command::EditGraph {
            id: GraphId(1),
            edit,
        })
        .unwrap();
    harness.state_mut().workbench.material().canvas.selected = [BOX].into();
    harness.run();
    harness.run();
    harness
}

/// A stamp read as a file would be (a 32-bit TGA with see-through,
/// written here): a `colour` dot on see-through, or (not `clear`) on a
/// solid white square.
fn stamp(colour: [u8; 3], clear: bool, name: &str) -> Stamp {
    let n = 20u16;
    let mut tga = vec![0u8, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    tga.extend_from_slice(&n.to_le_bytes());
    tga.extend_from_slice(&n.to_le_bytes());
    // 32 bits a pixel, the top row first, 8 bits of see-through.
    tga.extend_from_slice(&[32, 0x28]);
    for y in 0..n {
        for x in 0..n {
            let (dx, dy) = (f32::from(x) - 9.5, f32::from(y) - 9.5);
            let [r, g, b, a] = if (dx * dx + dy * dy).sqrt() < 8.0 {
                [colour[0], colour[1], colour[2], 255]
            } else if clear {
                [0, 0, 0, 0]
            } else {
                [255, 255, 255, 255]
            };
            tga.extend_from_slice(&[b, g, r, a]);
        }
    }
    fileio::image_file::read_stamp(&tga, name).unwrap()
}

fn note(harness: &H) -> String {
    harness.state().workbench.note.clone().unwrap_or_default()
}

fn asked(harness: &H) -> bool {
    harness
        .state()
        .actions
        .contains(&WorkbenchAction::ChooseStamp { path: None })
}

/// The stamps' names on the box.
fn names(harness: &H) -> Vec<String> {
    graph(harness)
        .get(BOX)
        .unwrap()
        .stamps
        .as_ref()
        .map(|s| {
            s.entries()
                .iter()
                .map(|e| e.stamp.name().to_string())
                .collect()
        })
        .unwrap_or_default()
}

/// Adds `stamp` through Attributes' Add a stamp….
fn add(harness: &mut H, stamp: Stamp) {
    harness.get_by_label(ADD_LABEL).click();
    harness.run();
    assert!(asked(harness), "Add a stamp… asks the app for a picture");
    harness.state_mut().workbench.add_box_stamp(stamp);
    harness.run();
    harness.run();
}

#[test]
fn attributes_adds_stamps_one_undo_step_each() {
    let mut harness = with_stamps_box();
    // Nothing brought in: the box says why it gives nothing.
    assert!(harness.get_all_by_label(NO_STAMPS).next().is_some());
    add(&mut harness, stamp([200, 40, 40], true, "rivet.png"));
    assert_eq!(names(&harness), ["rivet.png"]);
    assert!(note(&harness).contains("rivet.png"), "{}", note(&harness));
    add(&mut harness, stamp([40, 200, 40], true, "leaf.png"));
    assert_eq!(names(&harness), ["rivet.png", "leaf.png"]);
    assert_eq!(
        harness.state().workbench.document().undo_label(),
        Some("Change a Stamps box's stamps")
    );
    // The box works out now: its copies show.
    let registry = harness
        .state()
        .workbench
        .document()
        .scene()
        .graphs()
        .registry()
        .clone();
    let out = matgraph::Evaluator::with_threads(2).run(
        &registry,
        &graph(&harness),
        matgraph::Size::new(64).unwrap(),
        &[BOX],
    );
    assert!(out.outcome(BOX).unwrap().is_ok());
    undo(&mut harness);
    assert_eq!(names(&harness), ["rivet.png"]);
    undo(&mut harness);
    assert!(names(&harness).is_empty());
    // Handed a picture nobody asked for: it says where stamps go.
    harness
        .state_mut()
        .workbench
        .add_box_stamp(stamp([1, 2, 3], true, "x.png"));
    assert!(note(&harness).contains("Stamps box"), "{}", note(&harness));
    assert!(names(&harness).is_empty());
}

#[test]
fn take_off_removes_one_and_undo_puts_it_back() {
    let mut harness = with_stamps_box();
    add(&mut harness, stamp([200, 40, 40], true, "rivet.png"));
    add(&mut harness, stamp([40, 200, 40], true, "leaf.png"));
    harness.get_all_by_label(TAKE_OFF_LABEL).next().unwrap().click();
    harness.run();
    assert_eq!(names(&harness), ["leaf.png"]);
    // Taking the last off leaves the box holding none.
    harness.get_by_label(TAKE_OFF_LABEL).click();
    harness.run();
    assert!(names(&harness).is_empty());
    undo(&mut harness);
    undo(&mut harness);
    assert_eq!(names(&harness), ["rivet.png", "leaf.png"]);
}

#[test]
fn a_double_click_on_the_box_asks_for_a_stamp() {
    let mut harness = with_stamps_box();
    let (rect, _) = box_at(&mut harness, BOX);
    let spot = rect.left_top() + egui::vec2(30.0, 8.0);
    click(&mut harness, spot, Modifiers::NONE);
    click(&mut harness, spot, Modifiers::NONE);
    assert!(asked(&harness));
}

#[test]
fn a_full_box_says_so_and_a_solid_picture_is_explained() {
    let mut harness = with_stamps_box();
    add(&mut harness, stamp([9, 9, 9], false, "square.jpg"));
    assert!(
        note(&harness).contains("no see-through parts"),
        "{}",
        note(&harness)
    );
    for i in 1..MAX_STAMPS {
        harness.state_mut().workbench.material().ask_for_stamp(BOX);
        let s = stamp([i as u8, 9, 9], true, &format!("{i}.png"));
        harness.state_mut().workbench.add_box_stamp(s);
    }
    harness.run();
    assert_eq!(names(&harness).len(), MAX_STAMPS);
    // Add a stamp… is greyed out; handed one anyway, it says why not.
    harness.state_mut().workbench.material().ask_for_stamp(BOX);
    harness
        .state_mut()
        .workbench
        .add_box_stamp(stamp([1, 1, 1], true, "one too many.png"));
    let words = note(&harness);
    assert!(words.contains("one too many.png wasn't added"), "{words}");
    assert!(words.contains("16"), "{words}");
    assert_eq!(names(&harness).len(), MAX_STAMPS);
}

#[test]
fn copies_and_pastes_carry_the_stamps_even_into_another_project() {
    let mut harness = with_stamps_box();
    add(&mut harness, stamp([200, 40, 40], true, "rivet.png"));
    let held = graph(&harness).get(BOX).unwrap().stamps.clone();
    // In the same project: Ctrl + C, Ctrl + V.
    let bench = &mut harness.state_mut().workbench;
    let document = bench.document.clone();
    let mut note = None;
    bench.material().canvas.selected = [BOX].into();
    bench.material().copy(&document, false, &mut note);
    let mut document = bench.document.clone();
    bench
        .material()
        .paste(&mut document, Vec2::new(0.0, 900.0), false, &mut note);
    let g = document.scene().graphs().get(GraphId(1)).unwrap().clone();
    let pasted: Vec<_> = g
        .boxes()
        .filter(|(id, b)| b.kind == STAMPS && *id != BOX)
        .map(|(_, b)| b.stamps.clone())
        .collect();
    assert_eq!(pasted, [held.clone()]);
    // Another project (another tab): the stamps come along too.
    let clipboard = bench.material().clipboard.clone();
    let mut other = scene::Document::new();
    let gid = other.scene().graphs().next_id();
    other
        .execute(scene::Command::AddGraph(matgraph::Graph::new("Other")))
        .unwrap();
    let mut second = crate::material::MaterialEditor::default();
    second.project = 99;
    second.graph = Some(gid);
    second.clipboard = clipboard;
    second.paste(&mut other, Vec2::ZERO, false, &mut note);
    let g = other.scene().graphs().get(gid).unwrap();
    let (_, b) = g.boxes().next().expect("pasted");
    assert_eq!(b.kind, STAMPS);
    assert_eq!(b.stamps, held);
}

#[test]
fn stamps_are_saved_with_the_project() {
    let mut harness = with_stamps_box();
    add(&mut harness, stamp([200, 40, 40], true, "rivet.png"));
    let bytes = fileio::project::to_bytes(harness.state().workbench.document()).unwrap();
    let back = fileio::project::from_bytes(&bytes).unwrap();
    let kept = back
        .scene()
        .graphs()
        .get(GraphId(1))
        .unwrap()
        .get(BOX)
        .unwrap()
        .stamps
        .clone();
    assert_eq!(kept, graph(&harness).get(BOX).unwrap().stamps.clone());
}
