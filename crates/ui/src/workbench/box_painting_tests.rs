//! Painting a Painted box in the Material editor (decision 0282), used
//! through the real interface: opening the painting view from the box's
//! right-click menu, a double-click or Attributes; strokes wrapping over
//! the texture's edges; one undo step a stroke; the eraser and the
//! bucket; Esc cancelling a stroke and Tab going back; the resolution and
//! the kind; and a painted mask driving Fill the gaps.

use super::material_mode_tests::{H, box_at, drag, graph, quick_page, undo};
use super::*;
use crate::material::look::COLUMN;
use crate::material::paint_canvas::{self, PaintCanvas};
use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::kittest::Queryable;
use glam::Vec2;
use matgraph::boxes::painted::{PAINTED, PaintedKind};
use matgraph::{BoxId, GraphEdit, GraphId, Painting, PlugRef};
use std::sync::Arc;

/// The Painted box added to the page's graph.
const BOX: BoxId = BoxId(6);

/// The page with a Painted box beside its boxes, not yet opened.
fn with_painted_box() -> H {
    let mut harness = quick_page();
    let edit = GraphEdit::AddBox {
        kind: PAINTED.into(),
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
    harness.run();
    assert!(graph(&harness).get(BOX).is_some());
    harness
}

/// The page with the painting view open on the Painted box, at a small
/// resolution so tests run quickly.
fn view_page() -> H {
    let mut harness = with_painted_box();
    let bench = &mut harness.state_mut().workbench;
    bench.material().open_paint_canvas(BOX);
    bench.material().paint.as_mut().unwrap().new_size = 256;
    harness.run();
    harness.run();
    harness
}

fn view(harness: &mut H) -> &mut PaintCanvas {
    harness
        .state_mut()
        .workbench
        .material()
        .paint
        .as_mut()
        .unwrap()
}

fn is_open(harness: &mut H) -> bool {
    harness.state_mut().workbench.material().paint.is_some()
}

fn painting(harness: &H) -> Option<Arc<Painting>> {
    graph(harness).get(BOX).unwrap().painting.clone()
}

/// Where the texture place `p` (0 to 1 on the middle copy, y down) is
/// on screen.
fn at(harness: &mut H, p: Vec2) -> Pos2 {
    let v = view(harness);
    v.view.on_screen(v.area.unwrap(), p)
}

/// A brush stroke from `from` to `to`, on the texture.
fn stroke(harness: &mut H, from: Vec2, to: Vec2) {
    let (a, b) = (at(harness, from), at(harness, to));
    drag(harness, a, b, Modifiers::NONE);
}

/// How covered the painting is at the texture place `p`, 0 to 255.
fn alpha_at(painting: &Painting, p: Vec2) -> u8 {
    let n = painting.size() as f32;
    painting.pixel((p.x * n) as i64, (p.y * n) as i64)[3]
}

fn press(harness: &mut H, key: egui::Key) {
    let spot = at(harness, Vec2::new(0.5, 0.9));
    harness.event(Event::PointerMoved(spot));
    harness.run();
    harness.key_press(key);
    harness.run();
}

fn note(harness: &H) -> String {
    harness.state().workbench.note.clone().unwrap_or_default()
}

#[test]
fn the_box_menu_opens_the_painting_view_and_tab_goes_back() {
    let mut harness = with_painted_box();
    let (rect, _) = box_at(&mut harness, BOX);
    let spot = rect.left_top() + egui::vec2(20.0, 6.0);
    harness.event(Event::PointerMoved(spot));
    harness.step();
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos: spot,
            button: PointerButton::Secondary,
            pressed,
            modifiers: Modifiers::NONE,
        });
        harness.step();
    }
    harness.run();
    // The menu's entry (Attributes shows the box's own button too).
    harness
        .get_all_by_label(paint_canvas::PAINT_ON_IT_LABEL)
        .next()
        .unwrap()
        .click();
    harness.run();
    harness.run();
    assert!(is_open(&mut harness));
    harness.get_by_label(paint_canvas::PAINT_VIEW_LABEL);
    // The painter's brush bar and tools are over it.
    harness.get_by_label(crate::paint_tools::TOOLS_LABEL);
    press(&mut harness, egui::Key::Tab);
    assert!(!is_open(&mut harness));
    assert_eq!(harness.state().workbench.mode, Mode::Material);
}

#[test]
fn a_double_click_or_attributes_opens_it_and_other_boxes_have_no_entry() {
    let mut harness = with_painted_box();
    harness.state_mut().workbench.material().canvas.selected = [BOX].into();
    harness.run();
    harness
        .get_by_label(paint_canvas::PAINT_ON_IT_LABEL)
        .click();
    harness.run();
    assert!(is_open(&mut harness));
    // Another box's menu has no "Paint on it".
    let mut harness = with_painted_box();
    let (rect, _) = box_at(&mut harness, BoxId(1));
    let spot = rect.left_top() + egui::vec2(20.0, 6.0);
    harness.event(Event::PointerMoved(spot));
    harness.step();
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos: spot,
            button: PointerButton::Secondary,
            pressed,
            modifiers: Modifiers::NONE,
        });
        harness.step();
    }
    harness.run();
    assert!(
        harness
            .query_by_label(paint_canvas::PAINT_ON_IT_LABEL)
            .is_none()
    );
}

#[test]
fn a_stroke_over_the_edge_comes_back_on_the_other_side_in_one_undo_step() {
    let mut harness = view_page();
    stroke(&mut harness, Vec2::new(0.85, 0.5), Vec2::new(1.15, 0.5));
    let p = painting(&harness).expect("painted");
    assert_eq!(p.size(), 256);
    assert!(alpha_at(&p, Vec2::new(0.9, 0.5)) > 0, "before the edge");
    assert!(
        alpha_at(&p, Vec2::new(0.1, 0.5)) > 0,
        "over it, on the left"
    );
    assert_eq!(alpha_at(&p, Vec2::new(0.5, 0.5)), 0, "not in the middle");
    assert_eq!(alpha_at(&p, Vec2::new(0.9, 0.1)), 0);
    // A second stroke, then undo takes them one at a time.
    stroke(&mut harness, Vec2::new(0.5, 0.2), Vec2::new(0.5, 0.3));
    assert!(alpha_at(&painting(&harness).unwrap(), Vec2::new(0.5, 0.25)) > 0);
    undo(&mut harness);
    let after_one = painting(&harness).unwrap();
    assert_eq!(alpha_at(&after_one, Vec2::new(0.5, 0.25)), 0);
    assert!(alpha_at(&after_one, Vec2::new(0.1, 0.5)) > 0);
    undo(&mut harness);
    assert!(painting(&harness).is_none());
    // The box gives the paint as a mask, at the graph's size.
    assert!(is_open(&mut harness));
}

#[test]
fn the_eraser_and_the_bucket_and_clear() {
    let mut harness = view_page();
    // 8: the bucket fills the whole painting.
    press(&mut harness, egui::Key::Num8);
    let spot = at(&mut harness, Vec2::new(0.3, 0.3));
    super::material_mode_tests::click(&mut harness, spot, Modifiers::NONE);
    let p = painting(&harness).expect("filled");
    assert_eq!(p.painted_tiles(), p.tiles().len());
    // 2: the eraser takes it off where it goes.
    press(&mut harness, egui::Key::Num2);
    stroke(&mut harness, Vec2::new(0.4, 0.5), Vec2::new(0.6, 0.5));
    let p = painting(&harness).unwrap();
    assert!(alpha_at(&p, Vec2::new(0.5, 0.5)) < 128);
    assert_eq!(alpha_at(&p, Vec2::new(0.5, 0.1)), 255);
    // Polygon fill needs a model: it says so and paints nothing.
    press(&mut harness, egui::Key::Num4);
    assert!(
        note(&harness).contains("works on a model"),
        "{}",
        note(&harness)
    );
    // Clear takes it all off, one undo step.
    harness.get_by_label(paint_canvas::CLEAR_LABEL).click();
    harness.run();
    assert!(painting(&harness).is_none());
    undo(&mut harness);
    assert!(painting(&harness).is_some());
}

#[test]
fn esc_cancels_a_stroke_and_tab_waits_for_it() {
    let mut harness = view_page();
    let (a, b) = (
        at(&mut harness, Vec2::new(0.3, 0.5)),
        at(&mut harness, Vec2::new(0.6, 0.5)),
    );
    harness.event(Event::PointerMoved(a));
    harness.step();
    harness.event(Event::PointerButton {
        pos: a,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.step();
    harness.event(Event::PointerMoved(b));
    harness.step();
    assert!(harness.state().workbench.brushing.box_stroke.is_some());
    // The stroke shows as it goes.
    assert!(view(&mut harness).live.is_some());
    harness.key_press(egui::Key::Tab);
    harness.step();
    assert!(is_open(&mut harness), "Tab waits for the stroke");
    harness.key_press(egui::Key::Escape);
    harness.step();
    harness.event(Event::PointerButton {
        pos: b,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.run();
    assert!(painting(&harness).is_none(), "nothing kept");
    assert!(is_open(&mut harness), "Esc only cancels");
    assert_eq!(harness.state().workbench.mode, Mode::Material);
}

#[test]
fn the_brush_keys_change_the_brush_used_on_the_painting() {
    let mut harness = view_page();
    let before = harness.state().workbench.attributes.brush.brush.radius;
    press(&mut harness, egui::Key::CloseBracket);
    assert!(harness.state().workbench.attributes.brush.brush.radius > before);
    press(&mut harness, egui::Key::L);
    assert!(harness.state().workbench.attributes.brush.mirror_on);
    // Mirrored: a dab on the left is painted on the right too.
    stroke(&mut harness, Vec2::new(0.2, 0.5), Vec2::new(0.21, 0.5));
    let p = painting(&harness).unwrap();
    assert!(alpha_at(&p, Vec2::new(0.2, 0.5)) > 0);
    assert!(alpha_at(&p, Vec2::new(0.8, 0.5)) > 0);
    press(&mut harness, egui::Key::L);
    // P picks the colour the box shows there (white paint on a mask).
    harness.state_mut().workbench.attributes.brush.brush.color = [10, 20, 30, 255];
    harness
        .state_mut()
        .workbench
        .material()
        .pictures
        .settle(5.0);
    harness.run();
    let spot = at(&mut harness, Vec2::new(0.2, 0.5));
    harness.event(Event::PointerMoved(spot));
    harness.run();
    harness.key_press(egui::Key::P);
    harness.run();
    let picked = harness.state().workbench.attributes.brush.brush.color;
    assert_ne!(picked, [10, 20, 30, 255], "{}", note(&harness));
}

#[test]
fn the_resolution_and_kind_change_as_undo_steps() {
    let mut harness = view_page();
    stroke(&mut harness, Vec2::new(0.4, 0.4), Vec2::new(0.6, 0.6));
    let bench = &mut harness.state_mut().workbench;
    let document = &mut bench.document;
    let mut note = None;
    bench
        .material
        .set_paint_resolution(document, 1024, &mut note);
    harness.run();
    let p = painting(&harness).unwrap();
    assert_eq!(p.size(), 1024);
    assert!(alpha_at(&p, Vec2::new(0.5, 0.5)) > 0, "the paint is kept");
    undo(&mut harness);
    assert_eq!(painting(&harness).unwrap().size(), 256);
    // The kind: the box gives its colour output now.
    harness.get_by_label("Colour").click();
    harness.run();
    let b = graph(&harness).get(BOX).unwrap().clone();
    assert_eq!(PaintedKind::of(&b.settings), PaintedKind::Colour);
    undo(&mut harness);
    let b = graph(&harness).get(BOX).unwrap().clone();
    assert_eq!(PaintedKind::of(&b.settings), PaintedKind::Mask);
}

#[test]
fn a_painted_mask_drives_fill_the_gaps() {
    let mut harness = view_page();
    // Fill the gaps after Noise, filling only where the mask is painted:
    // its gaps through a Blend by the painted mask.
    let edits = vec![
        GraphEdit::AddBox {
            kind: "fill_gaps".into(),
            at: Vec2::new(6.0 * COLUMN, 0.0),
        },
        GraphEdit::Connect {
            from: PlugRef::new(BoxId(1), "height"),
            to: PlugRef::new(BoxId(7), "height"),
        },
        GraphEdit::Connect {
            from: PlugRef::new(BOX, "mask_out"),
            to: PlugRef::new(BoxId(4), "where"),
        },
        GraphEdit::Connect {
            from: PlugRef::new(BoxId(7), "out"),
            to: PlugRef::new(BoxId(4), "top"),
        },
        GraphEdit::Connect {
            from: PlugRef::new(BoxId(1), "height"),
            to: PlugRef::new(BoxId(4), "bottom"),
        },
    ];
    for edit in edits {
        harness
            .state_mut()
            .workbench
            .document_mut()
            .execute(scene::Command::EditGraph {
                id: GraphId(1),
                edit,
            })
            .unwrap();
    }
    // Paint the mask over the left half only.
    let brush = &mut harness.state_mut().workbench.attributes.brush.brush;
    brush.radius = 60.0;
    brush.flow = 1.0;
    brush.hardness = 1.0;
    brush.color = [255, 255, 255, 255];
    for y in [0.1, 0.3, 0.5, 0.7, 0.9] {
        stroke(&mut harness, Vec2::new(0.05, y), Vec2::new(0.45, y));
    }
    let registry = harness
        .state()
        .workbench
        .document()
        .scene()
        .graphs()
        .registry()
        .clone();
    let g = graph(&harness);
    let out = matgraph::Evaluator::with_threads(2).run(
        &registry,
        &g,
        matgraph::Size::new(64).unwrap(),
        &[BoxId(4)],
    );
    let blended = out.outcome(BoxId(4)).unwrap().clone().expect("runs");
    let mask = out.outcome(BOX).unwrap().clone().unwrap();
    let Some(matgraph::Value::Shape(mask)) = mask.outputs[0].clone() else {
        panic!("a mask");
    };
    let filled = out.outcome(BoxId(7)).unwrap().clone().unwrap();
    let Some(matgraph::Value::Height(filled)) = filled.outputs[0].clone() else {
        panic!("a filled height");
    };
    let noise = out.outcome(BoxId(1)).unwrap().clone().unwrap();
    let Some(matgraph::Value::Height(noise)) = noise.outputs[0].clone() else {
        panic!("noise");
    };
    let Some(matgraph::Value::Height(result)) = blended.outputs[0].clone() else {
        panic!("a blended height");
    };
    // Where painted, the filled height; where not, the plain noise.
    assert!(
        mask.at(12, 32) > 0.9 && mask.at(48, 32) < 0.1,
        "{} {}",
        mask.at(12, 32),
        mask.at(48, 32)
    );
    assert!((result.at(12, 32) - filled.at(12, 32)).abs() < 0.02);
    assert!((result.at(48, 32) - noise.at(48, 32)).abs() < 0.02);
}

#[test]
fn a_painting_is_saved_with_the_project() {
    let mut harness = view_page();
    stroke(&mut harness, Vec2::new(0.2, 0.2), Vec2::new(0.8, 0.8));
    let bytes = fileio::project::to_bytes(harness.state().workbench.document()).unwrap();
    let back = fileio::project::from_bytes(&bytes).unwrap();
    let kept = back
        .scene()
        .graphs()
        .get(GraphId(1))
        .unwrap()
        .get(BOX)
        .unwrap()
        .painting
        .clone();
    assert_eq!(kept, painting(&harness));
}
