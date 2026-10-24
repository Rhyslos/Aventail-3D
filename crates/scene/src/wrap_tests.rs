//! Tests for Bend along curve and Shrinkwrap (decision 0147): modifiers
//! reading another object through the stack's sources (decision 0142), so
//! the curve or target stays live, deleting it ends the modifier, copies
//! follow copies, and loops are refused.

use crate::*;
use glam::{Mat4, Vec3};

fn create(doc: &mut Document, kind: PrimitiveKind) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
        .unwrap();
    doc.selection()[0]
}

fn add_curve(doc: &mut Document, shape: CurveShape) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Curve(CurveObject::line(shape))))
        .unwrap();
    doc.selection()[0]
}

fn circle(doc: &mut Document, radius: f32) -> ObjectId {
    add_curve(doc, CurveShape::Ready(ReadyCurve::Circle { radius }))
}

fn at(x: f32, y: f32, z: f32) -> Transform {
    Transform {
        position: Vec3::new(x, y, z),
        ..Transform::default()
    }
}

fn bend(curve: ObjectId) -> ModifierKind {
    ModifierKind::BendAlongCurve(CurveBendSettings::of(curve))
}

fn wrap(target: ObjectId, mode: WrapMode) -> ModifierKind {
    let mut settings = ShrinkwrapSettings::of(target);
    settings.options.mode = mode;
    ModifierKind::Shrinkwrap(settings)
}

fn add(doc: &mut Document, id: ObjectId, kind: ModifierKind) -> Result<(), SceneError> {
    doc.execute(Command::AddModifier {
        ids: vec![id],
        kind,
    })
}

/// The object's shown shape's points, in the world.
fn world_points(doc: &Document, id: ObjectId) -> Vec<Vec3> {
    let scene = doc.scene();
    let matrix = scene.shape_matrix(id);
    scene
        .shown_shape(id)
        .unwrap()
        .vertices()
        .map(|(_, p)| matrix.transform_point3(p))
        .collect()
}

/// A long box bent round a circle of radius 200 on the ground.
fn box_round_a_circle() -> (Document, ObjectId, ObjectId) {
    let mut doc = Document::new();
    let curve = circle(&mut doc, 200.0);
    let cube = create(&mut doc, PrimitiveKind::Cube);
    let mut params = match &doc.scene().object(cube).unwrap().kind {
        ObjectKind::Primitive(Primitive::Cube(p)) => *p,
        other => panic!("{other:?}"),
    };
    params.size = Vec3::new(400.0, 20.0, 20.0);
    params.segments = [40, 1, 1];
    doc.execute(Command::SetPrimitive {
        ids: vec![cube],
        primitive: Primitive::Cube(params),
    })
    .unwrap();
    add(&mut doc, cube, bend(curve)).unwrap();
    (doc, cube, curve)
}

/// How far each of the shape's points is from the world's middle line
/// (Z), the least and the most.
fn reach(points: &[Vec3]) -> (f32, f32) {
    points
        .iter()
        .map(|p| p.truncate().length())
        .fold((f32::INFINITY, 0.0), |(lo, hi), r| (lo.min(r), hi.max(r)))
}

#[test]
fn a_shape_bent_along_a_curve_follows_it_and_undo_straightens_it() {
    let (mut doc, cube, _) = box_round_a_circle();
    assert_eq!(doc.undo_label(), Some("Add Bend along curve"));
    assert_eq!(doc.scene().modifier_problems(cube), vec![]);
    let (lo, hi) = reach(&world_points(&doc, cube));
    assert!(lo > 185.0 && hi < 215.0, "round the circle: {lo} {hi}");
    doc.undo();
    let (_, hi) = reach(&world_points(&doc, cube));
    assert!(hi > 199.0 && hi < 205.0, "straight again: {hi}");
}

#[test]
fn the_curve_stays_live() {
    let (mut doc, cube, curve) = box_round_a_circle();
    // A bigger circle: the shape follows.
    doc.execute(Command::SetReadyCurve {
        ids: vec![curve],
        ready: ReadyCurve::Circle { radius: 400.0 },
    })
    .unwrap();
    let (lo, hi) = reach(&world_points(&doc, cube));
    assert!(lo > 385.0 && hi < 415.0, "{lo} {hi}");
    // Moving the curve moves the shape with it.
    doc.execute(Command::SetTransforms(vec![(curve, at(0.0, 0.0, 300.0))]))
        .unwrap();
    assert!(world_points(&doc, cube).iter().all(|p| p.z >= 299.0));
    // Moving the shape doesn't: it stays on the curve.
    let before = world_points(&doc, cube);
    doc.execute(Command::SetTransforms(vec![(cube, at(1000.0, 0.0, 0.0))]))
        .unwrap();
    for (a, b) in before.iter().zip(world_points(&doc, cube)) {
        assert!(a.distance(b) < 1e-2, "{a} {b}");
    }
}

#[test]
fn editing_the_curve_s_points_reshapes_the_shape() {
    let mut doc = Document::new();
    let points = [Vec3::ZERO, Vec3::new(500.0, 0.0, 0.0)];
    let curve = add_curve(
        &mut doc,
        CurveShape::Points(Curve::corners(&points, false).unwrap()),
    );
    let cube = create(&mut doc, PrimitiveKind::Cube);
    add(&mut doc, cube, bend(curve)).unwrap();
    let flat = world_points(&doc, cube);
    assert!(flat.iter().all(|p| p.z < 200.0));
    // Lift the far end: the shape tilts up with it.
    doc.execute(Command::EditCurve {
        id: curve,
        edit: CurveEdit::Transform {
            parts: vec![CurvePart::point(CurvePointId::from_raw(0))],
            matrix: Mat4::from_translation(Vec3::new(0.0, 0.0, -500.0)),
        },
    })
    .unwrap();
    let tilted = world_points(&doc, cube);
    assert_ne!(flat, tilted);
}

#[test]
fn the_pivot_is_where_the_curve_runs_through_the_shape() {
    let mut doc = Document::new();
    let line = add_curve(
        &mut doc,
        CurveShape::Ready(ReadyCurve::Line { length: 400.0 }),
    );
    let cube = create(&mut doc, PrimitiveKind::Cube);
    add(&mut doc, cube, bend(line)).unwrap();
    // A cube's pivot is at its foot: it rides on the line.
    let lowest = |doc: &Document| {
        world_points(doc, cube)
            .iter()
            .map(|p| p.z)
            .fold(f32::INFINITY, f32::min)
    };
    assert!(lowest(&doc).abs() < 1e-2, "{}", lowest(&doc));
    // Its pivot in its middle: the line runs through it.
    let height = doc.scene().local_bounds(cube).unwrap().size().z;
    doc.execute(Command::MovePivots(vec![(
        cube,
        at(0.0, 0.0, height / 2.0),
    )]))
    .unwrap();
    assert!(
        (lowest(&doc) + height / 2.0).abs() < 1e-2,
        "{}",
        lowest(&doc)
    );
}

#[test]
fn deleting_the_curve_or_target_removes_the_modifier_in_one_step() {
    let (mut doc, cube, curve) = box_round_a_circle();
    doc.execute(Command::Delete(vec![curve])).unwrap();
    assert!(doc.scene().modifiers(cube).is_empty());
    doc.undo();
    assert_eq!(doc.scene().modifiers(cube).len(), 1);
    assert!(doc.scene().contains(curve));
    // The same for a target.
    let plane = create(&mut doc, PrimitiveKind::Plane);
    add(&mut doc, cube, wrap(plane, WrapMode::Nearest)).unwrap();
    doc.execute(Command::Delete(vec![plane])).unwrap();
    assert_eq!(doc.scene().modifiers(cube).len(), 1, "the bend stays");
    // Deleting the bent shape leaves the curve.
    doc.execute(Command::Delete(vec![cube])).unwrap();
    assert!(doc.scene().contains(curve));
}

#[test]
fn copies_follow_copied_curves_and_targets() {
    let (mut doc, cube, curve) = box_round_a_circle();
    doc.execute(Command::Duplicate {
        ids: vec![cube],
        instances: false,
    })
    .unwrap();
    let alone = doc.selection()[0];
    assert_eq!(doc.scene().modifiers(alone)[0].kind.sources(), vec![curve]);
    doc.execute(Command::Duplicate {
        ids: vec![cube, curve],
        instances: false,
    })
    .unwrap();
    let copies = doc.selection();
    let shape_copy = copies
        .iter()
        .copied()
        .find(|id| !doc.scene().modifiers(*id).is_empty())
        .unwrap();
    let curve_copy = copies.iter().copied().find(|id| *id != shape_copy).unwrap();
    assert_eq!(
        doc.scene().modifiers(shape_copy)[0].kind.sources(),
        vec![curve_copy]
    );
}

#[test]
fn odd_sources_and_loops_are_refused() {
    let mut doc = Document::new();
    let a = create(&mut doc, PrimitiveKind::Cube);
    let b = create(&mut doc, PrimitiveKind::Sphere);
    let curve = circle(&mut doc, 50.0);
    let bad = Err(SceneError::Modifier(ModifierRefusal::BadSource));
    // Bend along something that isn't a curve, or along nothing.
    assert_eq!(add(&mut doc, a, bend(b)), bad);
    assert_eq!(
        add(
            &mut doc,
            a,
            ModifierKind::BendAlongCurve(CurveBendSettings::default())
        ),
        bad
    );
    // Wrap onto itself, nothing, or round in a loop.
    assert_eq!(add(&mut doc, a, wrap(a, WrapMode::Nearest)), bad);
    assert_eq!(
        add(
            &mut doc,
            a,
            ModifierKind::Shrinkwrap(ShrinkwrapSettings::default())
        ),
        bad
    );
    add(&mut doc, a, wrap(b, WrapMode::Nearest)).unwrap();
    assert_eq!(add(&mut doc, b, wrap(a, WrapMode::Nearest)), bad);
    // A group has no shape to wrap onto.
    doc.execute(Command::Create(ObjectKind::Group)).unwrap();
    let group = doc.selection()[0];
    assert_eq!(add(&mut doc, a, wrap(group, WrapMode::Nearest)), bad);
    // A curve can be bent along by a curve's solid, and wrapped onto.
    add(&mut doc, b, bend(curve)).unwrap();
}

#[test]
fn shrinkwrap_lays_the_shape_on_the_target_which_stays_live() {
    let mut doc = Document::new();
    let plane = create(&mut doc, PrimitiveKind::Plane);
    let cube = create(&mut doc, PrimitiveKind::Cube);
    doc.execute(Command::SetTransforms(vec![(cube, at(0.0, 0.0, 300.0))]))
        .unwrap();
    add(&mut doc, cube, wrap(plane, WrapMode::Nearest)).unwrap();
    assert_eq!(doc.undo_label(), Some("Add Shrinkwrap"));
    assert!(world_points(&doc, cube).iter().all(|p| p.z.abs() < 1e-2));
    // The target moves: the shape follows; hidden, it still wraps.
    doc.execute(Command::SetTransforms(vec![(plane, at(0.0, 0.0, 50.0))]))
        .unwrap();
    doc.execute(Command::SetVisible {
        ids: vec![plane],
        visible: false,
    })
    .unwrap();
    assert!(
        world_points(&doc, cube)
            .iter()
            .all(|p| (p.z - 50.0).abs() < 1e-2)
    );
    // An offset lifts it off.
    let mut changed = doc.scene().modifiers(cube)[0].clone();
    let ModifierKind::Shrinkwrap(settings) = &mut changed.kind else {
        panic!()
    };
    settings.options.offset = 10.0;
    doc.execute(Command::ChangeModifier {
        id: cube,
        index: 0,
        modifier: changed,
    })
    .unwrap();
    assert!(
        world_points(&doc, cube)
            .iter()
            .all(|p| (p.z - 60.0).abs() < 1e-2)
    );
}

#[test]
fn only_the_chosen_points_move_and_lost_ones_say_why() {
    let mut doc = Document::new();
    let plane = create(&mut doc, PrimitiveKind::Plane);
    let cube = create(&mut doc, PrimitiveKind::Cube);
    doc.execute(Command::SetTransforms(vec![(cube, at(0.0, 0.0, 300.0))]))
        .unwrap();
    let shape = doc.scene().unmodified_shape(cube).unwrap();
    let top: Vec<VertexId> = shape
        .vertices()
        .filter(|(_, p)| p.z > 1.0)
        .map(|(id, _)| id)
        .collect();
    let mut settings = ShrinkwrapSettings::of(plane);
    settings.options.mode = WrapMode::AxisZ;
    settings.points = top.iter().chain(&top).copied().collect();
    add(&mut doc, cube, ModifierKind::Shrinkwrap(settings.clone())).unwrap();
    let ModifierKind::Shrinkwrap(kept) = &doc.scene().modifiers(cube)[0].kind else {
        panic!()
    };
    assert_eq!(kept.points.len(), top.len(), "listed once");
    let points = world_points(&doc, cube);
    assert_eq!(points.iter().filter(|p| p.z.abs() < 1e-2).count(), 4);
    assert_eq!(points.iter().filter(|p| p.z > 299.0).count(), 4);
    // Points that aren't there any more.
    settings.points = vec![VertexId::from_raw(4_000)];
    let mut changed = doc.scene().modifiers(cube)[0].clone();
    changed.kind = ModifierKind::Shrinkwrap(settings);
    doc.execute(Command::ChangeModifier {
        id: cube,
        index: 0,
        modifier: changed,
    })
    .unwrap();
    let problems = doc.scene().modifier_problems(cube);
    assert_eq!(problems.len(), 1);
    assert!(problems[0].1.contains("points"), "{problems:?}");
}

#[test]
fn a_target_without_a_surface_says_why() {
    let mut doc = Document::new();
    let curve = circle(&mut doc, 50.0);
    let cube = create(&mut doc, PrimitiveKind::Cube);
    // A curve that is only a line has no surface.
    add(&mut doc, cube, wrap(curve, WrapMode::Nearest)).unwrap();
    let problems = doc.scene().modifier_problems(cube);
    assert_eq!(problems.len(), 1, "{problems:?}");
    // Swept, it has one.
    doc.execute(Command::SetCurveSolid {
        ids: vec![curve],
        solid: CurveSolid::Sweep(Sweep::default()),
    })
    .unwrap();
    assert_eq!(doc.scene().modifier_problems(cube), vec![]);
}

#[test]
fn bake_into_shape_keeps_the_bent_shape_and_the_curve() {
    let (mut doc, cube, curve) = box_round_a_circle();
    let exported = doc.scene().exported_shape(cube).unwrap();
    doc.execute(Command::MakeEditable(vec![cube])).unwrap();
    let scene = doc.scene();
    assert!(scene.modifiers(cube).is_empty());
    assert_eq!(scene.object(cube).unwrap().kind, ObjectKind::Mesh(exported));
    assert!(scene.contains(curve));
}

#[test]
fn moving_the_pivot_reworks_only_the_kinds_that_read_it() {
    let (mut doc, cube, _) = box_round_a_circle();
    add(
        &mut doc,
        cube,
        ModifierKind::Subdivide(SubdivideOptions::default()),
    )
    .unwrap();
    doc.execute(Command::MoveModifier {
        id: cube,
        from: 1,
        to: 0,
    })
    .unwrap();
    let scene = doc.scene();
    scene.shown_shape(cube).unwrap();
    // Nothing changed: all kept.
    let input = scene.unmodified_shape(cube).unwrap();
    assert!(matches!(
        scene.stack_state(cube, Purpose::Viewport, input),
        Some(StackState::Ready(_))
    ));
    doc.execute(Command::MovePivots(vec![(cube, at(0.0, 0.0, 10.0))]))
        .unwrap();
    let scene = doc.scene();
    let input = scene.unmodified_shape(cube).unwrap();
    match scene.stack_state(cube, Purpose::Viewport, input) {
        Some(StackState::Needed(job)) => assert_eq!(job.steps_kept(), 1, "Subdivide is kept"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn heavy_work_is_done_on_another_thread() {
    let (doc, cube, _) = box_round_a_circle();
    let scene = doc.scene();
    let input = scene.unmodified_shape(cube).unwrap();
    let light = &scene.modifiers(cube)[0].kind;
    assert!(light.work_with(&input, Purpose::Viewport, &[]) < MODIFIERS_IN_BACKGROUND_FROM);
    let big = Primitive::Sphere(mesh::primitives::SphereParams {
        radius: 50.0,
        sides: 512,
        rings: 256,
    })
    .mesh();
    let target = vec![SourceShape {
        id: cube,
        data: SourceData::Shape(std::sync::Arc::new(big.clone())),
        matrix: Mat4::IDENTITY,
        pivot: Vec3::ZERO,
        materials: vec![0],
    }];
    let wrap = wrap(cube, WrapMode::Nearest);
    assert!(wrap.work_with(&input, Purpose::Viewport, &target) >= MODIFIERS_IN_BACKGROUND_FROM);
    assert!(light.work_with(&big, Purpose::Viewport, &[]) >= MODIFIERS_IN_BACKGROUND_FROM);
}
