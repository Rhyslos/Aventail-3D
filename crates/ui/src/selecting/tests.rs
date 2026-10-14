//! Tests for what selecting gestures and commands do to the document.

use super::*;
use crate::drawing::DrawingCache;
use glam::Vec3;
use mesh::{CubeParams, Primitive};
use scene::{Command, ObjectKind, PrimitiveKind};
use std::sync::Arc;

const SIZE: Vec2 = Vec2::new(1000.0, 600.0);

/// Looking at the origin from the front, right and above.
fn camera() -> Camera {
    Camera::looking_at(Vec3::new(300.0, -500.0, 250.0), Vec3::new(0.0, 0.0, 50.0))
}

fn pointer(world: Vec3) -> ViewPointer {
    let camera = camera();
    ViewPointer {
        camera,
        size: SIZE,
        at: camera.project(world, SIZE).unwrap(),
    }
}

fn nowhere() -> ViewPointer {
    ViewPointer {
        camera: camera(),
        size: SIZE,
        at: Vec2::new(3.0, 3.0),
    }
}

/// A document with a standard cube (selected), and its id.
fn cube_document() -> (Document, ObjectId) {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Cube.standard(),
    )))
    .unwrap();
    let id = doc.selection()[0];
    (doc, id)
}

/// Runs `work` with a selector for the document as it is drawn now.
fn with_selector<T>(
    doc: &mut Document,
    mode: Mode,
    visible_only: bool,
    work: impl FnOnce(&mut Selector<'_>) -> T,
) -> T {
    let mut cache = DrawingCache::default();
    let drawing: Arc<SceneDrawing> = Arc::clone(
        &cache
            .update(doc, kind_for(mode), &crate::drawing::Hidden::default())
            .drawing,
    );
    let scene = PickScene::new(
        drawing
            .items
            .iter()
            .map(|i| (i.mesh.as_ref(), i.transform, i.selected)),
    );
    let mut stroke = StrokeMemory::default();
    let mut selector = Selector {
        document: doc,
        drawing: &drawing,
        scene: &scene,
        mode,
        visible_only,
        stroke: &mut stroke,
    };
    work(&mut selector)
}

fn gesture(doc: &mut Document, mode: Mode, g: Gesture) -> Option<String> {
    with_selector(doc, mode, false, |s| s.gesture(g))
}

fn selected(doc: &Document, id: ObjectId, kind: ElementKind) -> usize {
    doc.elements(id).map_or(0, |s| s.count(kind))
}

/// A box around the whole cube.
fn around_cube() -> Gesture {
    Gesture::Region {
        camera: camera(),
        size: SIZE,
        regions: vec![Region::Rect {
            a: Vec2::ZERO,
            b: SIZE,
        }],
        how: Combine::Replace,
        stroke: None,
    }
}

const NEAR_CORNER: Vec3 = Vec3::new(50.0, -50.0, 100.0);

// ---------------------------------------------------------- the Model mode ---

#[test]
fn clicking_selects_objects_in_the_model_mode() {
    let (mut doc, cube) = cube_document();
    let click = |how| Gesture::Click {
        pointer: pointer(Vec3::new(0.0, -50.0, 50.0)),
        how,
    };
    gesture(
        &mut doc,
        Mode::Model,
        Gesture::Click {
            pointer: nowhere(),
            how: Combine::Replace,
        },
    );
    assert!(doc.selection().is_empty(), "a click on nothing unselects");
    gesture(&mut doc, Mode::Model, click(Combine::Replace));
    assert_eq!(doc.selection(), vec![cube]);
    gesture(&mut doc, Mode::Model, click(Combine::Toggle));
    assert!(
        doc.selection().is_empty(),
        "Ctrl + click takes it out again"
    );
    gesture(&mut doc, Mode::Model, click(Combine::Add));
    gesture(&mut doc, Mode::Model, click(Combine::Add));
    assert_eq!(doc.selection(), vec![cube], "adding twice keeps it");
    // Ctrl + click on nothing changes nothing.
    gesture(
        &mut doc,
        Mode::Model,
        Gesture::Click {
            pointer: nowhere(),
            how: Combine::Toggle,
        },
    );
    assert_eq!(doc.selection(), vec![cube]);
    assert_eq!(doc.undo_label(), Some("Select Cube"), "an undo step");
    doc.undo();
    assert!(doc.selection().is_empty(), "undo takes the selection back");
}

#[test]
fn boxes_select_add_and_remove_objects() {
    let (mut doc, cube) = cube_document();
    doc.clear_selection();
    gesture(&mut doc, Mode::Model, around_cube());
    assert_eq!(doc.selection(), vec![cube]);
    let remove = Gesture::Region {
        camera: camera(),
        size: SIZE,
        regions: vec![Region::Rect {
            a: Vec2::ZERO,
            b: SIZE,
        }],
        how: Combine::Remove,
        stroke: None,
    };
    gesture(&mut doc, Mode::Model, remove);
    assert!(doc.selection().is_empty());
}

#[test]
fn select_all_and_invert_work_on_objects() {
    let (mut doc, cube) = cube_document();
    doc.execute(Command::Create(ObjectKind::Group)).unwrap();
    let group = doc.selection()[0];
    with_selector(&mut doc, Mode::Model, false, |s| s.select_all());
    assert_eq!(doc.selection().len(), 2);
    doc.set_selection([cube]);
    with_selector(&mut doc, Mode::Model, false, |s| s.invert());
    assert_eq!(doc.selection(), vec![group]);
    let note = gesture(&mut doc, Mode::Model, Gesture::Grow(1));
    assert!(note.unwrap().contains("points, edges and polygons"));
}

// --------------------------------------------------------- element modes ---

#[test]
fn clicking_a_point_selects_it_as_an_undo_step() {
    let (mut doc, cube) = cube_document();
    let revision = doc.revision();
    let click = |how| Gesture::Click {
        pointer: pointer(NEAR_CORNER),
        how,
    };
    gesture(&mut doc, Mode::Points, click(Combine::Replace));
    assert_eq!(selected(&doc, cube, ElementKind::Points), 1);
    assert_eq!(doc.undo_label(), Some("Select (1 point selected)"));

    gesture(&mut doc, Mode::Points, click(Combine::Toggle));
    assert_eq!(selected(&doc, cube, ElementKind::Points), 0);
    doc.undo();
    assert_eq!(
        selected(&doc, cube, ElementKind::Points),
        1,
        "undo brings it back"
    );

    // A plain click on nothing unselects; Ctrl + click on nothing doesn't.
    gesture(
        &mut doc,
        Mode::Points,
        Gesture::Click {
            pointer: nowhere(),
            how: Combine::Toggle,
        },
    );
    assert_eq!(selected(&doc, cube, ElementKind::Points), 1);
    gesture(
        &mut doc,
        Mode::Points,
        Gesture::Click {
            pointer: nowhere(),
            how: Combine::Replace,
        },
    );
    assert_eq!(selected(&doc, cube, ElementKind::Points), 0);
    assert_eq!(doc.revision(), revision, "selecting isn't a change to save");
}

#[test]
fn each_mode_keeps_its_own_selection() {
    let (mut doc, cube) = cube_document();
    gesture(&mut doc, Mode::Points, around_cube());
    gesture(&mut doc, Mode::Polygons, around_cube());
    assert_eq!(selected(&doc, cube, ElementKind::Points), 8);
    assert_eq!(selected(&doc, cube, ElementKind::Polygons), 6);
    assert_eq!(selected(&doc, cube, ElementKind::Edges), 0);
    with_selector(&mut doc, Mode::Polygons, false, |s| s.clear_elements());
    assert_eq!(selected(&doc, cube, ElementKind::Polygons), 0);
    assert_eq!(
        selected(&doc, cube, ElementKind::Points),
        8,
        "Esc leaves other modes"
    );
}

#[test]
fn boxes_select_through_unless_only_visible_ones_are_wanted() {
    let (mut doc, cube) = cube_document();
    gesture(&mut doc, Mode::Points, around_cube());
    assert_eq!(selected(&doc, cube, ElementKind::Points), 8);
    with_selector(&mut doc, Mode::Points, true, |s| s.gesture(around_cube()));
    assert_eq!(selected(&doc, cube, ElementKind::Points), 7);
}

#[test]
fn shift_adds_and_ctrl_removes_with_a_box() {
    let (mut doc, cube) = cube_document();
    let near = Region::Circle {
        centre: pointer(NEAR_CORNER).at,
        radius: 4.0,
    };
    let region = |how| Gesture::Region {
        camera: camera(),
        size: SIZE,
        regions: vec![near.clone()],
        how,
        stroke: None,
    };
    gesture(&mut doc, Mode::Points, region(Combine::Replace));
    assert_eq!(selected(&doc, cube, ElementKind::Points), 1);
    gesture(&mut doc, Mode::Points, around_cube());
    gesture(&mut doc, Mode::Points, region(Combine::Remove));
    assert_eq!(selected(&doc, cube, ElementKind::Points), 7);
    gesture(&mut doc, Mode::Points, region(Combine::Add));
    assert_eq!(selected(&doc, cube, ElementKind::Points), 8);
}

#[test]
fn a_brush_stroke_is_one_undo_step() {
    let (mut doc, cube) = cube_document();
    let steps = doc.undo_steps();
    let corners = [
        Vec3::new(50.0, -50.0, 100.0),
        Vec3::new(-50.0, -50.0, 100.0),
        Vec3::new(50.0, -50.0, 0.0),
    ];
    for (i, corner) in corners.iter().enumerate() {
        let dab = Gesture::Region {
            camera: camera(),
            size: SIZE,
            regions: vec![Region::Circle {
                centre: pointer(*corner).at,
                radius: 5.0,
            }],
            how: if i == 0 {
                Combine::Replace
            } else {
                Combine::Add
            },
            stroke: Some(4),
        };
        gesture(&mut doc, Mode::Points, dab);
    }
    assert_eq!(selected(&doc, cube, ElementKind::Points), 3);
    assert_eq!(doc.undo_steps(), steps + 1);
    doc.undo();
    assert_eq!(selected(&doc, cube, ElementKind::Points), 0);
}

#[test]
fn objects_must_be_selected_before_their_elements() {
    let (mut doc, cube) = cube_document();
    doc.clear_selection();
    let note = gesture(&mut doc, Mode::Points, around_cube());
    assert!(note.unwrap().contains("Select an object first"));
    assert_eq!(selected(&doc, cube, ElementKind::Points), 0);
    let note = with_selector(&mut doc, Mode::Edges, false, |s| s.select_all());
    assert!(note.is_some());
}

#[test]
fn select_all_invert_grow_and_shrink_elements() {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Primitive(Primitive::Cube(
        CubeParams {
            segments: [2, 2, 2],
            ..CubeParams::default()
        },
    ))))
    .unwrap();
    let cube = doc.selection()[0];
    with_selector(&mut doc, Mode::Polygons, false, |s| s.select_all());
    assert_eq!(selected(&doc, cube, ElementKind::Polygons), 24);
    assert_eq!(doc.undo_label(), Some("Select all (24 polygons selected)"));
    with_selector(&mut doc, Mode::Polygons, false, |s| s.invert());
    assert_eq!(selected(&doc, cube, ElementKind::Polygons), 0);

    // One polygon, grown: its neighbours through shared points.
    gesture(
        &mut doc,
        Mode::Polygons,
        Gesture::Click {
            pointer: pointer(Vec3::new(25.0, -50.0, 75.0)),
            how: Combine::Replace,
        },
    );
    assert_eq!(selected(&doc, cube, ElementKind::Polygons), 1);
    gesture(&mut doc, Mode::Polygons, Gesture::Grow(1));
    let grown = selected(&doc, cube, ElementKind::Polygons);
    assert!(grown > 1, "{grown}");
    assert!(doc.undo_label().unwrap().starts_with("Grow"));
    gesture(&mut doc, Mode::Polygons, Gesture::Grow(-1));
    assert_eq!(selected(&doc, cube, ElementKind::Polygons), 1);
    gesture(&mut doc, Mode::Polygons, Gesture::Grow(0));
    assert_eq!(selected(&doc, cube, ElementKind::Polygons), 1);
}

#[test]
fn double_clicks_select_loops_and_rings() {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Primitive(Primitive::Cube(
        CubeParams {
            segments: [2, 2, 2],
            ..CubeParams::default()
        },
    ))))
    .unwrap();
    let cube = doc.selection()[0];
    // The edge across the front face's middle, from its centre to the
    // right: its loop goes round the cube.
    let on_edge = Vec3::new(25.0, -50.0, 50.0);
    let double = |ring, how| Gesture::DoubleClick {
        pointer: pointer(on_edge),
        ring,
        how,
    };
    gesture(&mut doc, Mode::Edges, double(false, Combine::Replace));
    assert_eq!(selected(&doc, cube, ElementKind::Edges), 8);
    assert!(doc.undo_label().unwrap().starts_with("Select loop"));
    gesture(&mut doc, Mode::Edges, double(true, Combine::Replace));
    let ring = selected(&doc, cube, ElementKind::Edges);
    assert!(ring >= 2, "{ring}");
    // Ctrl + double-click adds the loop to the ring.
    gesture(&mut doc, Mode::Edges, double(false, Combine::Add));
    assert!(selected(&doc, cube, ElementKind::Edges) > ring);

    // In the Points mode: the points along the loop.
    gesture(&mut doc, Mode::Points, double(false, Combine::Replace));
    assert_eq!(selected(&doc, cube, ElementKind::Points), 8);

    // In the Polygons mode: the band across the nearest edge.
    gesture(
        &mut doc,
        Mode::Polygons,
        Gesture::DoubleClick {
            pointer: pointer(Vec3::new(25.0, -50.0, 55.0)),
            ring: false,
            how: Combine::Replace,
        },
    );
    assert_eq!(selected(&doc, cube, ElementKind::Polygons), 8);

    // On nothing: nothing happens.
    let before = doc.undo_steps();
    gesture(
        &mut doc,
        Mode::Edges,
        Gesture::DoubleClick {
            pointer: nowhere(),
            ring: false,
            how: Combine::Replace,
        },
    );
    assert_eq!(doc.undo_steps(), before);
}

#[test]
fn tools_go_round_and_keys_combine_as_chosen() {
    assert_eq!(SelectTool::default(), SelectTool::Box);
    let mut tool = SelectTool::Box;
    for expected in [SelectTool::Lasso, SelectTool::Brush, SelectTool::Box] {
        tool = tool.next();
        assert_eq!(tool, expected);
    }
    let ctrl = Modifiers::COMMAND;
    let shift = Modifiers::SHIFT;
    let alt = Modifiers::ALT;
    let none = Modifiers::NONE;
    // The standard: Ctrl adds (a click flips the one thing), Shift removes.
    let keys = SelectKeys::default();
    assert_eq!(keys, SelectKeys::CtrlAddsShiftRemoves);
    assert_eq!(keys.combine(none, true), Combine::Replace);
    assert_eq!(keys.combine(none, false), Combine::Replace);
    assert_eq!(keys.combine(ctrl, true), Combine::Toggle);
    assert_eq!(keys.combine(ctrl, false), Combine::Add);
    assert_eq!(keys.combine(shift, true), Combine::Remove);
    assert_eq!(keys.combine(shift, false), Combine::Remove);
    assert_eq!(
        keys.combine(ctrl | shift, false),
        Combine::Remove,
        "remove wins"
    );
    // The others.
    let other = SelectKeys::ShiftAddsCtrlRemoves;
    assert_eq!(other.combine(shift, false), Combine::Add);
    assert_eq!(other.combine(shift, true), Combine::Toggle);
    assert_eq!(other.combine(ctrl, false), Combine::Remove);
    let flips = SelectKeys::CtrlFlips;
    assert_eq!(flips.combine(ctrl, false), Combine::Toggle);
    assert_eq!(
        flips.combine(shift, false),
        Combine::Replace,
        "Shift unused"
    );
    let alt_removes = SelectKeys::CtrlAddsAltRemoves;
    assert_eq!(alt_removes.combine(alt, true), Combine::Remove);
    assert_eq!(alt_removes.combine(ctrl, false), Combine::Add);
    // Double-clicks: Alt asks for the ring, so it never removes there.
    assert_eq!(alt_removes.combine_loop(alt), Combine::Replace);
    assert_eq!(keys.combine_loop(ctrl | alt), Combine::Add);
    assert_eq!(keys.combine_loop(shift), Combine::Remove);
    for preset in SelectKeys::ALL {
        assert!(!preset.name().is_empty() && !preset.summary().is_empty());
        assert_eq!(preset.combine(none, true), Combine::Replace, "{preset:?}");
    }
    assert_eq!(count(1, ElementKind::Edges), "1 edge");
    assert_eq!(count(3, ElementKind::Edges), "3 edges");
}

#[test]
fn a_flipping_brush_flips_each_thing_once_per_stroke() {
    let (mut doc, cube) = cube_document();
    let dab = |stroke| Gesture::Region {
        camera: camera(),
        size: SIZE,
        regions: vec![Region::Circle {
            centre: pointer(NEAR_CORNER).at,
            radius: 5.0,
        }],
        how: Combine::Toggle,
        stroke: Some(stroke),
    };
    let mut cache = DrawingCache::default();
    let drawing = Arc::clone(
        &cache
            .update(
                &doc,
                Some(tools::ElementKind::Polygons),
                &crate::drawing::Hidden::default(),
            )
            .drawing,
    );
    let scene = PickScene::new(
        drawing
            .items
            .iter()
            .map(|i| (i.mesh.as_ref(), i.transform, i.selected)),
    );
    let mut stroke = StrokeMemory::default();
    let mut selector = Selector {
        document: &mut doc,
        drawing: &drawing,
        scene: &scene,
        mode: Mode::Points,
        visible_only: false,
        stroke: &mut stroke,
    };
    // Going over the same corner three times in one stroke flips it once.
    for _ in 0..3 {
        selector.gesture(dab(1));
    }
    assert_eq!(selected(selector.document, cube, ElementKind::Points), 1);
    // A new stroke flips it back.
    selector.gesture(dab(2));
    assert_eq!(selected(selector.document, cube, ElementKind::Points), 0);
}

#[test]
fn quick_brush_strokes_over_shapes_merge_and_flip() {
    let (mut doc, cube) = cube_document();
    doc.clear_selection();
    let steps = doc.undo_steps();
    let over = |how, stroke| Gesture::Region {
        camera: camera(),
        size: SIZE,
        regions: vec![Region::Circle {
            centre: pointer(MIDDLE).at,
            radius: 5.0,
        }],
        how,
        stroke: Some(stroke),
    };
    gesture(&mut doc, Mode::Model, over(Combine::Replace, 9));
    gesture(&mut doc, Mode::Model, over(Combine::Add, 9));
    assert_eq!(doc.selection(), vec![cube]);
    assert_eq!(doc.undo_steps(), steps + 1, "one stroke, one step");
    gesture(&mut doc, Mode::Model, over(Combine::Toggle, 10));
    assert!(doc.selection().is_empty(), "flipped off");
}

const MIDDLE: Vec3 = Vec3::new(0.0, -50.0, 50.0);
