//! Tests for moving, rotating and scaling: what the gizmo works on, the
//! commands a change becomes, snapping, and drags as one undo step.

use super::*;
use crate::drawing::DrawingCache;
use mesh::{CubeParams, Primitive};
use scene::{ElementSelection, ObjectKind, PrimitiveKind};
use std::f32::consts::FRAC_PI_2;
use std::sync::Arc;

fn cube_document() -> (Document, ObjectId) {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Cube.standard(),
    )))
    .unwrap();
    let id = doc.selection()[0];
    (doc, id)
}

fn drawing(doc: &Document, mode: Mode) -> Arc<SceneDrawing> {
    let mut cache = DrawingCache::default();
    Arc::clone(
        &cache
            .update(doc, kind_for(mode), &crate::drawing::Hidden::default())
            .drawing,
    )
}

fn target(doc: &Document, mode: Mode, axes: Axes) -> Option<Target> {
    Target::of(doc, &drawing(doc, mode), mode, axes)
}

fn place(doc: &mut Document, id: ObjectId, transform: Transform) {
    doc.execute(Command::SetTransforms(vec![(id, transform)]))
        .unwrap();
}

fn transform_of(doc: &Document, id: ObjectId) -> Transform {
    doc.scene().object(id).unwrap().transform
}

// ------------------------------------------------------------ targets ---

#[test]
fn nothing_selected_means_no_gizmo() {
    let (mut doc, _) = cube_document();
    doc.clear_selection();
    assert!(target(&doc, Mode::Model, Axes::World).is_none());
    assert!(target(&doc, Mode::Points, Axes::World).is_none());
}

#[test]
fn objects_are_targeted_at_their_axes() {
    let (mut doc, cube) = cube_document();
    place(
        &mut doc,
        cube,
        Transform {
            position: Vec3::new(100.0, 0.0, 0.0),
            ..Transform::default()
        },
    );
    let t = target(&doc, Mode::Model, Axes::World).unwrap();
    assert!(t.pivot.abs_diff_eq(Vec3::new(100.0, 0.0, 0.0), 1e-4));
    assert_eq!(t.axes, [Vec3::X, Vec3::Y, Vec3::Z]);
    assert!(matches!(&t.targets, Targets::Objects(s) if s.len() == 1));
    assert_eq!(t.gizmo(GizmoTool::Move).origin, t.pivot);

    // Two objects: the gizmo sits between them.
    doc.execute(Command::Create(ObjectKind::Group)).unwrap();
    let group = doc.selection()[0];
    doc.set_selection([cube, group]);
    let t = target(&doc, Mode::Model, Axes::World).unwrap();
    assert!(t.pivot.abs_diff_eq(Vec3::new(50.0, 0.0, 0.0), 1e-4));
}

#[test]
fn objects_inside_a_selected_group_move_with_it() {
    let (mut doc, cube) = cube_document();
    doc.execute(Command::Group(vec![cube])).unwrap();
    let group = doc.selection()[0];
    doc.set_selection([group, cube]);
    let t = target(&doc, Mode::Model, Axes::World).unwrap();
    let Targets::Objects(starts) = &t.targets else {
        panic!()
    };
    assert_eq!(starts.len(), 1, "only the group");
    assert_eq!(starts[0].id, group);
}

#[test]
fn object_axes_follow_the_objects_turn() {
    let (mut doc, cube) = cube_document();
    place(
        &mut doc,
        cube,
        Transform {
            rotation: Vec3::new(FRAC_PI_2, 0.0, 0.0),
            ..Transform::default()
        },
    );
    let t = target(&doc, Mode::Model, Axes::Object).unwrap();
    assert!(t.axes[0].abs_diff_eq(Vec3::Y, 1e-4), "{:?}", t.axes);
    assert!(t.axes[2].abs_diff_eq(Vec3::Z, 1e-4));
}

#[test]
fn elements_are_targeted_through_their_points() {
    let (mut doc, cube) = cube_document();
    // No elements selected: nothing to move.
    assert!(target(&doc, Mode::Polygons, Axes::World).is_none());
    let mesh = PrimitiveKind::Cube.standard().mesh();
    let top = mesh
        .faces()
        .map(|(f, _)| f)
        .find(|f| mesh.face_normal(*f).z > 0.9)
        .unwrap();
    doc.select_elements(
        [(cube, ElementSelection::of([scene::Element::Polygon(top)]))],
        "Select",
        None,
    );
    let t = target(&doc, Mode::Polygons, Axes::World).unwrap();
    let Targets::Points(starts, ElementKind::Polygons) = &t.targets else {
        panic!()
    };
    assert_eq!(starts[0].points.len(), 4, "the top polygon's corners");
    assert!(
        t.pivot.abs_diff_eq(Vec3::new(0.0, 0.0, 100.0), 1e-3),
        "{}",
        t.pivot
    );
    assert!(
        t.bounds
            .size()
            .abs_diff_eq(Vec3::new(100.0, 100.0, 0.0), 1e-3)
    );
}

// ----------------------------------------------------------- commands ---

#[test]
fn moving_rotating_and_scaling_objects() {
    let (mut doc, cube) = cube_document();
    let t = target(&doc, Mode::Model, Axes::World).unwrap();
    let session = Session::new(t, 1);

    session
        .show(&mut doc, &Change::Translate(Vec3::new(10.0, 0.0, 5.0)))
        .unwrap();
    assert_eq!(transform_of(&doc, cube).position, Vec3::new(10.0, 0.0, 5.0));
    assert_eq!(doc.undo_label(), Some("Move Cube"));

    // The next frame replaces the move: still one step.
    let steps = doc.undo_steps();
    session
        .show(
            &mut doc,
            &Change::Rotate {
                axis: Vec3::Z,
                angle: FRAC_PI_2,
            },
        )
        .unwrap();
    assert_eq!(doc.undo_steps(), steps);
    assert_eq!(transform_of(&doc, cube).position, Vec3::ZERO);
    assert!((transform_of(&doc, cube).rotation.x - FRAC_PI_2).abs() < 1e-4);
    assert_eq!(doc.undo_label(), Some("Rotate Cube"));

    session
        .show(&mut doc, &Change::Scale(Vec3::new(2.0, 1.0, 1.0)))
        .unwrap();
    assert!(
        transform_of(&doc, cube)
            .scale
            .abs_diff_eq(Vec3::new(2.0, 1.0, 1.0), 1e-4)
    );

    // Cancelling puts everything back, leaving no step.
    session.cancel(&mut doc);
    assert_eq!(transform_of(&doc, cube), Transform::default());
    assert_eq!(doc.undo_steps(), steps - 1);
}

#[test]
fn objects_turn_around_the_gizmo_not_their_own_axis() {
    let (mut doc, cube) = cube_document();
    doc.execute(Command::Create(ObjectKind::Group)).unwrap();
    let group = doc.selection()[0];
    place(
        &mut doc,
        group,
        Transform {
            position: Vec3::new(200.0, 0.0, 0.0),
            ..Transform::default()
        },
    );
    doc.set_selection([cube, group]);
    let t = target(&doc, Mode::Model, Axes::World).unwrap();
    // The pivot is between them, at x = 100; a half turn swaps them.
    Session::new(t, 1)
        .show(
            &mut doc,
            &Change::Rotate {
                axis: Vec3::Z,
                angle: std::f32::consts::PI,
            },
        )
        .unwrap();
    assert!(
        transform_of(&doc, cube)
            .position
            .abs_diff_eq(Vec3::new(200.0, 0.0, 0.0), 1e-3)
    );
    assert!(
        transform_of(&doc, group)
            .position
            .abs_diff_eq(Vec3::ZERO, 1e-3)
    );
    assert_eq!(doc.undo_label(), Some("Rotate 2 objects"));
}

#[test]
fn a_child_moves_in_its_parents_space() {
    let (mut doc, cube) = cube_document();
    doc.execute(Command::Group(vec![cube])).unwrap();
    let group = doc.selection()[0];
    place(
        &mut doc,
        group,
        Transform {
            rotation: Vec3::new(FRAC_PI_2, 0.0, 0.0),
            scale: Vec3::splat(2.0),
            ..Transform::default()
        },
    );
    doc.set_selection([cube]);
    let t = target(&doc, Mode::Model, Axes::World).unwrap();
    Session::new(t, 1)
        .show(&mut doc, &Change::Translate(Vec3::new(20.0, 0.0, 0.0)))
        .unwrap();
    // 20 cm along the world's X is 10 cm along the group's (turned,
    // doubled) -Y.
    let local = transform_of(&doc, cube).position;
    assert!(
        local.abs_diff_eq(Vec3::new(0.0, -10.0, 0.0), 1e-3),
        "{local}"
    );
    let world = doc.scene().world_matrix(cube).w_axis.truncate();
    assert!(world.abs_diff_eq(Vec3::new(20.0, 0.0, 0.0), 1e-3));
}

#[test]
fn points_move_and_turn_in_their_objects_space() {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Primitive(Primitive::Cube(
        CubeParams::default(),
    ))))
    .unwrap();
    let cube = doc.selection()[0];
    place(
        &mut doc,
        cube,
        Transform {
            position: Vec3::new(500.0, 0.0, 0.0),
            scale: Vec3::splat(2.0),
            ..Transform::default()
        },
    );
    let corner = VertexId::from_raw(0);
    doc.select_elements(
        [(cube, ElementSelection::of([scene::Element::Point(corner)]))],
        "Select",
        None,
    );
    let start = doc
        .scene()
        .evaluated(cube)
        .unwrap()
        .mesh
        .position(corner)
        .unwrap();
    let t = target(&doc, Mode::Points, Axes::World).unwrap();
    let session = Session::new(t, 1);
    session
        .show(&mut doc, &Change::Translate(Vec3::new(20.0, 0.0, 0.0)))
        .unwrap();
    assert_eq!(doc.undo_label(), Some("Move 1 point"));
    // 20 cm in the world is 10 cm in the doubled object.
    let moved = doc
        .scene()
        .evaluated(cube)
        .unwrap()
        .mesh
        .position(corner)
        .unwrap();
    assert!(
        moved.abs_diff_eq(start + Vec3::new(10.0, 0.0, 0.0), 1e-3),
        "{moved}"
    );
    // A turn of one point around itself leaves it where it is, recorded as
    // a matrix.
    session
        .show(
            &mut doc,
            &Change::Rotate {
                axis: Vec3::Z,
                angle: 1.0,
            },
        )
        .unwrap();
    let turned = doc
        .scene()
        .evaluated(cube)
        .unwrap()
        .mesh
        .position(corner)
        .unwrap();
    assert!(turned.abs_diff_eq(start, 1e-3), "{turned}");
    assert_eq!(doc.undo_label(), Some("Rotate 1 point"));
    assert_eq!(
        doc.scene().object(cube).unwrap().stack.operation_count(),
        1,
        "one recorded edit, however many frames"
    );
}

#[test]
fn a_change_that_changes_nothing_records_nothing() {
    let (mut doc, _) = cube_document();
    let steps = doc.undo_steps();
    let t = target(&doc, Mode::Model, Axes::World).unwrap();
    Session::new(t, 1)
        .show(&mut doc, &Change::Translate(Vec3::ZERO))
        .unwrap();
    assert_eq!(doc.undo_steps(), steps);
    assert!(is_nothing(&Change::Scale(Vec3::ONE)));
    assert!(!is_nothing(&Change::Rotate {
        axis: Vec3::Z,
        angle: 0.1
    }));
}

// ------------------------------------------------------------ snapping ---

#[test]
fn snapping_is_on_with_the_button_or_flipped_by_ctrl() {
    let mut s = SnapSettings::default();
    assert!(!s.active(false));
    assert!(s.active(true), "Ctrl flips it on");
    s.on = true;
    assert!(s.active(false));
    assert!(!s.active(true), "Ctrl flips it off");
    s.ctrl_flips = false;
    assert!(s.active(true), "button only: Ctrl changes nothing");
}

/// A snapping context with nothing to snap to but the grid.
fn grid_only<'a>(
    scene: &'a PickScene,
    drawing: &'a SceneDrawing,
    doc: &'a Document,
) -> Snapping<'a> {
    Snapping {
        settings: SnapSettings {
            points: false,
            edges: false,
            polygons: false,
            ..SnapSettings::default()
        },
        grid: GridDetail::HalfMetre,
        scene,
        drawing,
        document: doc,
        camera: Camera::looking_at(Vec3::new(300.0, -500.0, 250.0), Vec3::ZERO),
        size: Vec2::new(1000.0, 600.0),
        mouse: Vec2::ZERO,
        locked: [false; 3],
    }
}

#[test]
fn moves_snap_to_the_grid_lines_turns_and_scaling_to_steps() {
    let (mut doc, cube) = cube_document();
    place(
        &mut doc,
        cube,
        Transform {
            position: Vec3::new(10.0, 0.0, 0.0),
            ..Transform::default()
        },
    );
    let drawing = drawing(&doc, Mode::Model);
    let scene = PickScene::default();
    let t = target(&doc, Mode::Model, Axes::World).unwrap();
    let snapping = grid_only(&scene, &drawing, &doc);
    // From x = 10, a move of 30 lands on the 50 cm line: a move of 40.
    let Change::Translate(d) = t.snapped(
        Change::Translate(Vec3::new(30.0, 0.0, 0.0)),
        Handle::Axis(0),
        &snapping,
    ) else {
        panic!()
    };
    assert!(d.abs_diff_eq(Vec3::new(40.0, 0.0, 0.0), 1e-4), "{d}");
    let Change::Rotate { angle, .. } = t.snapped(
        Change::Rotate {
            axis: Vec3::Z,
            angle: 0.3,
        },
        Handle::Ring(2),
        &snapping,
    ) else {
        panic!()
    };
    assert!((angle - 15f32.to_radians()).abs() < 1e-5);
    let Change::Scale(f) = t.snapped(
        Change::Scale(Vec3::new(1.26, 1.0, 1.0)),
        Handle::ScaleAxis(0),
        &snapping,
    ) else {
        panic!()
    };
    assert!(f.abs_diff_eq(Vec3::new(1.3, 1.0, 1.0), 1e-5), "{f}");
    assert_eq!(grid_step(GridDetail::Metre), 100.0);
}

#[test]
fn moves_snap_to_a_point_along_the_dragged_arrow() {
    let (mut doc, cube) = cube_document();
    // A second cube, 300 cm along X, to snap to.
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Cube.standard(),
    )))
    .unwrap();
    let other = doc.selection()[0];
    place(
        &mut doc,
        other,
        Transform {
            position: Vec3::new(300.0, 0.0, 0.0),
            ..Transform::default()
        },
    );
    doc.set_selection([cube]);
    let drawing = drawing(&doc, Mode::Model);
    let scene = PickScene::new(
        drawing
            .items
            .iter()
            .map(|i| (i.mesh.as_ref(), i.transform, i.selected)),
    );
    let t = target(&doc, Mode::Model, Axes::World).unwrap();
    let camera = Camera::looking_at(Vec3::new(300.0, -800.0, 400.0), Vec3::new(150.0, 0.0, 50.0));
    let size = Vec2::new(1000.0, 600.0);
    // The other cube's near top corner, at (350, -50, 100).
    let corner = Vec3::new(350.0, -50.0, 100.0);
    let snapping = Snapping {
        settings: SnapSettings::default(),
        grid: GridDetail::Metre,
        scene: &scene,
        drawing: &drawing,
        document: &doc,
        camera,
        size,
        mouse: camera.project(corner, size).unwrap(),
        locked: [false; 3],
    };
    // Dragging the X arrow: only the X of the corner counts.
    let Change::Translate(d) = t.snapped(
        Change::Translate(Vec3::new(337.0, 0.0, 0.0)),
        Handle::Axis(0),
        &snapping,
    ) else {
        panic!()
    };
    assert!(d.abs_diff_eq(Vec3::new(350.0, 0.0, 0.0), 1e-2), "{d}");
    // The centre handle goes all the way to it.
    let Change::Translate(d) = t.snapped(Change::Translate(Vec3::ZERO), Handle::Centre, &snapping)
    else {
        panic!()
    };
    assert!(d.abs_diff_eq(corner, 1e-2), "{d}");
    // With Z locked, it stays at its height.
    let locked = Snapping {
        locked: [false, false, true],
        ..snapping
    };
    let Change::Translate(d) = t.snapped(Change::Translate(Vec3::ZERO), Handle::Centre, &locked)
    else {
        panic!()
    };
    assert!(d.z.abs() < 1e-3, "{d}");
}

#[test]
fn changes_read_naturally() {
    let (doc, _) = cube_document();
    let t = target(&doc, Mode::Model, Axes::World).unwrap();
    let units = crate::units::Units::Centimetres;
    assert_eq!(
        describe(&Change::Translate(Vec3::new(12.5, 0.0, 0.0)), &t, units),
        "Moving: X 12.5 cm, Y 0 cm, Z 0 cm"
    );
    assert_eq!(
        describe(
            &Change::Rotate {
                axis: Vec3::Z,
                angle: FRAC_PI_2
            },
            &t,
            units
        ),
        "Rotating: 90.0°"
    );
    assert_eq!(
        describe(&Change::Scale(Vec3::new(1.5, 1.0, 1.0)), &t, units),
        "Scaling: 150% × 100% × 100%"
    );
    assert_eq!(Tool::Move.gizmo(), Some(GizmoTool::Move));
    assert_eq!(Tool::Select.gizmo(), None);
    assert_eq!(Tool::All.name(), "Transform");
}

#[test]
fn a_moved_polygon_is_found_again_after_more_segments() {
    let (mut doc, cube) = cube_document();
    let mesh = PrimitiveKind::Cube.standard().mesh();
    let top = mesh
        .faces()
        .map(|(f, _)| f)
        .find(|f| mesh.face_normal(*f).z > 0.9)
        .unwrap();
    doc.select_elements(
        [(cube, ElementSelection::of([scene::Element::Polygon(top)]))],
        "Select",
        None,
    );
    let t = target(&doc, Mode::Polygons, Axes::World).unwrap();
    let Targets::Points(starts, _) = &t.targets else {
        panic!()
    };
    assert!(
        starts[0].anchors.is_some(),
        "the edit keeps where the polygon was"
    );
    Session::new(t, 1)
        .show(&mut doc, &Change::Translate(Vec3::Z * 40.0))
        .unwrap();
    doc.execute(Command::SetPrimitive {
        ids: vec![cube],
        primitive: Primitive::Cube(CubeParams {
            segments: [3, 3, 3],
            ..CubeParams::default()
        }),
    })
    .unwrap();
    let shape = doc.scene().evaluated(cube).unwrap();
    let lifted = shape
        .mesh
        .vertices()
        .filter(|(_, p)| (p.z - 140.0).abs() < 1e-3)
        .count();
    assert_eq!(lifted, 16, "the whole top, now 4 × 4 points");
    assert!(
        shape
            .warnings
            .iter()
            .all(|w| w.found_again && w.missing == 0)
    );
}

// ------------------------------------------------- pivots (Object mode) ---

#[test]
fn in_the_object_mode_the_gizmo_moves_and_turns_the_pivot_only() {
    let (mut doc, cube) = cube_document();
    let shape_before = doc.scene().shape_matrix(cube);
    let t = target(&doc, Mode::Object, Axes::World).unwrap();
    assert!(t.pivots);
    let change = Change::Translate(Vec3::new(0.0, 0.0, 100.0));
    assert_eq!(t.label(&change), "Move Cube's pivot");
    let commands = t.commands(&change);
    assert!(matches!(commands.as_slice(), [Command::MovePivots(_)]));
    for command in commands {
        doc.execute(command).unwrap();
    }
    assert!(
        transform_of(&doc, cube)
            .position
            .abs_diff_eq(Vec3::new(0.0, 0.0, 100.0), 1e-4)
    );
    assert!(
        doc.scene()
            .shape_matrix(cube)
            .abs_diff_eq(shape_before, 1e-3)
    );
    // Turning too.
    let t = target(&doc, Mode::Object, Axes::World).unwrap();
    let turn = Change::Rotate {
        axis: Vec3::Z,
        angle: FRAC_PI_2,
    };
    assert_eq!(t.label(&turn), "Rotate Cube's pivot");
    for command in t.commands(&turn) {
        doc.execute(command).unwrap();
    }
    assert!((transform_of(&doc, cube).rotation.x - FRAC_PI_2).abs() < 1e-4);
    assert!(
        doc.scene()
            .shape_matrix(cube)
            .abs_diff_eq(shape_before, 1e-3)
    );
    // A pivot has no size: scaling does nothing.
    let t = target(&doc, Mode::Object, Axes::World).unwrap();
    assert!(t.commands(&Change::Scale(Vec3::splat(2.0))).is_empty());
    // In the Model mode, the shape goes with its pivot as before.
    let t = target(&doc, Mode::Model, Axes::World).unwrap();
    assert!(!t.pivots);
    assert!(matches!(
        t.commands(&change).as_slice(),
        [Command::SetTransforms(_)]
    ));
}

#[test]
fn the_object_modes_gizmo_sits_on_the_pivot_not_the_shape() {
    let (mut doc, cube) = cube_document();
    doc.execute(Command::MovePivots(vec![(
        cube,
        Transform {
            position: Vec3::new(500.0, 0.0, 0.0),
            ..Transform::default()
        },
    )]))
    .unwrap();
    for mode in [Mode::Model, Mode::Object] {
        let t = target(&doc, mode, Axes::World).unwrap();
        assert!(t.pivot.abs_diff_eq(Vec3::new(500.0, 0.0, 0.0), 1e-4));
    }
    // Points are drawn, and moved, where the shape is.
    doc.select_elements(
        [(
            cube,
            ElementSelection::of(
                doc.scene()
                    .evaluated(cube)
                    .unwrap()
                    .mesh
                    .vertices()
                    .map(|(v, _)| scene::Element::Point(v)),
            ),
        )],
        "Select",
        None,
    );
    let t = target(&doc, Mode::Points, Axes::World).unwrap();
    assert!(t.bounds.centre().x.abs() < 1e-3, "{:?}", t.bounds);
}

#[test]
fn a_pivot_snaps_to_its_own_shapes_corner() {
    let (doc, _) = cube_document();
    let drawing = drawing(&doc, Mode::Object);
    let scene = PickScene::new(
        drawing
            .items
            .iter()
            .map(|i| (i.mesh.as_ref(), i.transform, i.selected)),
    );
    let camera = Camera::looking_at(Vec3::new(300.0, -800.0, 400.0), Vec3::ZERO);
    let size = Vec2::new(1000.0, 600.0);
    // The cube's own near top corner.
    let corner = Vec3::new(50.0, -50.0, 100.0);
    let snapping = Snapping {
        settings: SnapSettings::default(),
        grid: GridDetail::Metre,
        scene: &scene,
        drawing: &drawing,
        document: &doc,
        camera,
        size,
        mouse: camera.project(corner, size).unwrap(),
        locked: [false; 3],
    };
    let pivot = target(&doc, Mode::Object, Axes::World).unwrap();
    let Change::Translate(d) =
        pivot.snapped(Change::Translate(Vec3::ZERO), Handle::Centre, &snapping)
    else {
        panic!()
    };
    assert!(d.abs_diff_eq(corner, 1e-2), "{d}");
    // Moving the whole object, its own shape is never snapped to.
    let whole = target(&doc, Mode::Model, Axes::World).unwrap();
    let Change::Translate(d) =
        whole.snapped(Change::Translate(Vec3::ZERO), Handle::Centre, &snapping)
    else {
        panic!()
    };
    assert!(!d.abs_diff_eq(corner, 1e-2), "{d}");
}

// ------------------------------------------------- drags in a row (0126) ---

fn corner_position(doc: &Document, cube: ObjectId, corner: VertexId) -> Vec3 {
    doc.scene()
        .evaluated(cube)
        .unwrap()
        .mesh
        .position(corner)
        .unwrap()
}

#[test]
fn drags_in_a_row_on_the_same_points_stay_one_edit() {
    let (mut doc, cube) = cube_document();
    let corner = VertexId::from_raw(0);
    doc.select_elements(
        [(cube, ElementSelection::of([scene::Element::Point(corner)]))],
        "Select",
        None,
    );
    let start = corner_position(&doc, cube, corner);
    let drag = |doc: &mut Document, key: u64, change: Change| {
        let t = target(doc, Mode::Points, Axes::World).unwrap();
        Session::new(t, key).show(doc, &change).unwrap();
    };
    drag(&mut doc, 1, Change::Translate(Vec3::new(10.0, 0.0, 0.0)));
    drag(&mut doc, 2, Change::Translate(Vec3::new(0.0, 5.0, 0.0)));
    let edits = |doc: &Document| doc.scene().object(cube).unwrap().stack.operation_count();
    assert_eq!(edits(&doc), 1, "two drags, one edit");
    assert!(
        corner_position(&doc, cube, corner).abs_diff_eq(start + Vec3::new(10.0, 5.0, 0.0), 1e-3)
    );
    // A turn after a move joins them as one matrix.
    drag(
        &mut doc,
        3,
        Change::Rotate {
            axis: Vec3::Z,
            angle: 0.5,
        },
    );
    assert_eq!(edits(&doc), 1);
    // Each drag is still its own undo step.
    doc.undo();
    assert_eq!(edits(&doc), 1);
    doc.undo();
    assert!(
        corner_position(&doc, cube, corner).abs_diff_eq(start + Vec3::new(10.0, 0.0, 0.0), 1e-3)
    );
    assert_eq!(edits(&doc), 1);
    doc.undo();
    assert_eq!(edits(&doc), 0);
}

#[test]
fn selecting_something_else_starts_a_new_edit() {
    let (mut doc, cube) = cube_document();
    let select = |doc: &mut Document, v: u32| {
        doc.select_elements(
            [(
                cube,
                ElementSelection::of([scene::Element::Point(VertexId::from_raw(v))]),
            )],
            "Select",
            None,
        );
    };
    let drag = |doc: &mut Document, key: u64| {
        let t = target(doc, Mode::Points, Axes::World).unwrap();
        Session::new(t, key)
            .show(doc, &Change::Translate(Vec3::X * 4.0))
            .unwrap();
    };
    select(&mut doc, 0);
    drag(&mut doc, 1);
    select(&mut doc, 1);
    drag(&mut doc, 2);
    assert_eq!(doc.scene().object(cube).unwrap().stack.operation_count(), 2);
    // The same points again, but after selecting something else in
    // between: a new edit.
    select(&mut doc, 2);
    select(&mut doc, 1);
    drag(&mut doc, 3);
    assert_eq!(doc.scene().object(cube).unwrap().stack.operation_count(), 3);
}

#[test]
fn combining_edits_keeps_the_points_and_anchors() {
    let points = vec![VertexId::from_raw(2), VertexId::from_raw(3)];
    let a = Operation::MovePoints {
        points: points.clone(),
        offset: Vec3::X,
        anchors: None,
    };
    let b = Operation::MovePoints {
        points: points.clone(),
        offset: Vec3::Y,
        anchors: None,
    };
    assert_eq!(
        combined(&a, b),
        Operation::MovePoints {
            points: points.clone(),
            offset: Vec3::new(1.0, 1.0, 0.0),
            anchors: None,
        }
    );
    let turn = Operation::TransformPoints {
        points: points.clone(),
        matrix: Mat4::from_rotation_z(1.0),
        anchors: None,
    };
    let Operation::TransformPoints { matrix, .. } = combined(&a, turn) else {
        panic!("a matrix");
    };
    let p = matrix.transform_point3(Vec3::ZERO);
    assert!(p.abs_diff_eq(Mat4::from_rotation_z(1.0).transform_point3(Vec3::X), 1e-5));
}
