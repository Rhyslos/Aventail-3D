//! Tests for the Array modifier in the scene (decision 0153): copies in a
//! line, a grid or a ring, a ring turning round the pivot or a centre
//! object read through the stack's sources.

use crate::*;
use glam::Vec3;

fn create(doc: &mut Document, kind: PrimitiveKind) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
        .unwrap();
    doc.selection()[0]
}

fn at(x: f32, y: f32, z: f32) -> Transform {
    Transform {
        position: Vec3::new(x, y, z),
        ..Transform::default()
    }
}

fn array(options: ArrayOptions, centre: Option<ObjectId>) -> ModifierKind {
    ModifierKind::Array(ArraySettings { options, centre })
}

fn ring(count: u32, radius: f32) -> ArrayOptions {
    ArrayOptions {
        layout: ArrayLayout::Radial,
        count,
        radius,
        ..ArrayOptions::default()
    }
}

fn add(doc: &mut Document, id: ObjectId, kind: ModifierKind) -> Result<(), SceneError> {
    doc.execute(Command::AddModifier {
        ids: vec![id],
        kind,
    })
}

fn change(doc: &mut Document, id: ObjectId, kind: ModifierKind) {
    let mut modifier = doc.scene().modifiers(id)[0].clone();
    modifier.kind = kind;
    doc.execute(Command::ChangeModifier {
        id,
        index: 0,
        modifier,
    })
    .unwrap();
}

/// The box round the object's shown shape, in the world.
fn world_box(doc: &Document, id: ObjectId) -> Bounds {
    let scene = doc.scene();
    let matrix = scene.shape_matrix(id);
    Bounds::around(
        scene
            .shown_shape(id)
            .unwrap()
            .vertices()
            .map(|(_, p)| matrix.transform_point3(p)),
    )
    .unwrap()
}

fn close(a: Vec3, b: Vec3) -> bool {
    a.distance(b) < 1e-2
}

#[test]
fn a_line_of_copies_shows_and_exports() {
    let mut doc = Document::new();
    let cube = create(&mut doc, PrimitiveKind::Cube);
    add(&mut doc, cube, array(ArrayOptions::default(), None)).unwrap();
    assert_eq!(doc.undo_label(), Some("Add Array"));
    let scene = doc.scene();
    assert_eq!(scene.evaluated(cube).unwrap().mesh.face_count(), 6);
    let shown = scene.shown_shape(cube).unwrap();
    assert_eq!(shown.face_count(), 18);
    assert_eq!(scene.shown_shape(cube), scene.exported_shape(cube));
    assert!(scene.modifier_problems(cube).is_empty());
    let kind = array(ArrayOptions::default(), None);
    assert_eq!(kind.family(), ModifierFamily::Generate);
    assert!(!kind.differs_by_purpose());
    assert!(!kind.needs_sources());
    assert!(kind.sources().is_empty());
    assert!(!kind.reads_pivot());
    doc.undo();
    assert_eq!(doc.scene().shown_shape(cube).unwrap().face_count(), 6);
}

#[test]
fn it_follows_subdivide_on_the_palette() {
    let kinds = ModifierKind::standard_kinds();
    let at = |name: &str| kinds.iter().position(|k| k.name() == name).unwrap();
    assert_eq!(at("Array"), at("Subdivide") + 1);
}

#[test]
fn settings_are_clamped_when_added() {
    let mut doc = Document::new();
    let cube = create(&mut doc, PrimitiveKind::Cube);
    let wild = ArrayOptions {
        count: 0,
        angle: f32::NAN,
        merge_distance: f32::INFINITY,
        ..ArrayOptions::default()
    };
    add(&mut doc, cube, array(wild, None)).unwrap();
    let ModifierKind::Array(settings) = &doc.scene().modifiers(cube)[0].kind else {
        panic!("not an array");
    };
    assert_eq!(settings.options, wild.clamped());
    assert_eq!(settings.options.count, 1);
}

#[test]
fn too_many_copies_say_why_and_show_the_shape() {
    let mut doc = Document::new();
    let cube = create(&mut doc, PrimitiveKind::Cube);
    let grid = ArrayOptions {
        layout: ArrayLayout::Grid,
        counts: [1000, 1000, 1],
        ..ArrayOptions::default()
    };
    add(&mut doc, cube, array(grid, None)).unwrap();
    let scene = doc.scene();
    let problems = scene.modifier_problems(cube);
    assert_eq!(problems.len(), 1);
    assert!(problems[0].1.contains("100 000"), "{problems:?}");
    assert_eq!(scene.shown_shape(cube).unwrap().face_count(), 6);
}

#[test]
fn a_ring_turns_round_the_pivot() {
    let mut doc = Document::new();
    let cube = create(&mut doc, PrimitiveKind::Cube);
    doc.execute(Command::SetTransforms(vec![(cube, at(1000.0, 0.0, 0.0))]))
        .unwrap();
    add(&mut doc, cube, array(ring(4, 200.0), None)).unwrap();
    assert!(doc.scene().modifiers(cube)[0].kind.reads_pivot());
    let b = world_box(&doc, cube);
    // Round the cube's pivot (in the middle of its foot), where it stands.
    assert!(close(b.min, Vec3::new(750.0, -250.0, 0.0)), "{b:?}");
    assert!(close(b.max, Vec3::new(1250.0, 250.0, 100.0)), "{b:?}");
    // Moving the pivot along X moves the ring's middle.
    doc.execute(Command::MovePivots(vec![(cube, at(1100.0, 0.0, 0.0))]))
        .unwrap();
    let b = world_box(&doc, cube);
    assert!(close(
        b.centre().truncate().extend(0.0),
        Vec3::new(1100.0, 0.0, 0.0)
    ));
}

#[test]
fn a_ring_turns_round_its_centre_object_which_stays_live() {
    let mut doc = Document::new();
    let hub = create(&mut doc, PrimitiveKind::Sphere);
    doc.execute(Command::SetTransforms(vec![(hub, at(0.0, 0.0, 0.0))]))
        .unwrap();
    let bolt = create(&mut doc, PrimitiveKind::Cube);
    doc.execute(Command::SetTransforms(vec![(bolt, at(300.0, 0.0, 0.0))]))
        .unwrap();
    add(&mut doc, bolt, array(ring(4, 0.0), Some(hub))).unwrap();
    let kind = &doc.scene().modifiers(bolt)[0].kind;
    assert_eq!(kind.sources(), vec![hub]);
    assert!(!kind.reads_pivot());
    let b = world_box(&doc, bolt);
    assert!(close(b.min, Vec3::new(-350.0, -350.0, 0.0)), "{b:?}");
    assert!(close(b.max, Vec3::new(350.0, 350.0, 100.0)), "{b:?}");
    // Move the hub: the ring turns round it there.
    doc.execute(Command::SetTransforms(vec![(hub, at(100.0, 0.0, 0.0))]))
        .unwrap();
    let b = world_box(&doc, bolt);
    assert!(close(b.min, Vec3::new(-150.0, -250.0, 0.0)), "{b:?}");
    assert!(close(b.max, Vec3::new(350.0, 250.0, 100.0)), "{b:?}");
}

#[test]
fn deleting_the_centre_object_puts_the_ring_on_the_pivot() {
    let mut doc = Document::new();
    let hub = create(&mut doc, PrimitiveKind::Sphere);
    let bolt = create(&mut doc, PrimitiveKind::Cube);
    add(&mut doc, bolt, array(ring(4, 50.0), Some(hub))).unwrap();
    doc.execute(Command::Delete(vec![hub])).unwrap();
    let modifiers = doc.scene().modifiers(bolt);
    assert_eq!(modifiers.len(), 1, "the array stays");
    assert!(modifiers[0].kind.sources().is_empty());
    assert!(modifiers[0].kind.reads_pivot());
    assert!(doc.scene().modifier_problems(bolt).is_empty());
    doc.undo();
    assert_eq!(doc.scene().modifiers(bolt)[0].kind.sources(), vec![hub]);
}

#[test]
fn copies_follow_a_copied_centre() {
    let mut doc = Document::new();
    let hub = create(&mut doc, PrimitiveKind::Sphere);
    let bolt = create(&mut doc, PrimitiveKind::Cube);
    add(&mut doc, bolt, array(ring(4, 50.0), Some(hub))).unwrap();
    doc.execute(Command::Duplicate {
        ids: vec![bolt, hub],
        instances: false,
    })
    .unwrap();
    let copies = doc.selection();
    let bolt_copy = copies
        .iter()
        .copied()
        .find(|id| !doc.scene().modifiers(*id).is_empty())
        .unwrap();
    let hub_copy = copies.iter().copied().find(|id| *id != bolt_copy).unwrap();
    assert_eq!(
        doc.scene().modifiers(bolt_copy)[0].kind.sources(),
        vec![hub_copy]
    );
}

#[test]
fn any_object_but_itself_can_be_the_centre() {
    let mut doc = Document::new();
    let bolt = create(&mut doc, PrimitiveKind::Cube);
    doc.execute(Command::Create(ObjectKind::Group)).unwrap();
    let group = doc.selection()[0];
    doc.execute(Command::SetTransforms(vec![(group, at(0.0, 500.0, 0.0))]))
        .unwrap();
    add(&mut doc, bolt, array(ring(2, 0.0), Some(group))).unwrap();
    let b = world_box(&doc, bolt);
    assert!(close(b.max, Vec3::new(50.0, 1050.0, 100.0)), "{b:?}");
    assert_eq!(
        add(&mut doc, bolt, array(ring(2, 0.0), Some(bolt))),
        Err(SceneError::Modifier(ModifierRefusal::BadSource))
    );
    assert_eq!(
        add(&mut doc, bolt, array(ring(2, 0.0), Some(ObjectId(987_654)))),
        Err(SceneError::Modifier(ModifierRefusal::BadSource))
    );
}

#[test]
fn merging_joins_touching_copies() {
    let mut doc = Document::new();
    let plane = create(&mut doc, PrimitiveKind::Plane);
    let options = ArrayOptions {
        merge: true,
        ..ArrayOptions::default()
    };
    add(&mut doc, plane, array(options, None)).unwrap();
    let shown = doc.scene().shown_shape(plane).unwrap();
    assert_eq!(mesh::MeshReport::new(&shown).pieces, 1);
    change(
        &mut doc,
        plane,
        array(
            ArrayOptions {
                merge: false,
                ..options
            },
            None,
        ),
    );
    let shown = doc.scene().shown_shape(plane).unwrap();
    assert_eq!(mesh::MeshReport::new(&shown).pieces, 3);
    assert_eq!(doc.undo_label(), Some("Change Array"));
}

#[test]
fn bake_into_shape_keeps_the_copies() {
    let mut doc = Document::new();
    let cube = create(&mut doc, PrimitiveKind::Cube);
    add(&mut doc, cube, array(ArrayOptions::default(), None)).unwrap();
    let exported = doc.scene().exported_shape(cube).unwrap();
    doc.execute(Command::MakeEditable(vec![cube])).unwrap();
    let scene = doc.scene();
    assert!(scene.modifiers(cube).is_empty());
    assert_eq!(scene.object(cube).unwrap().kind, ObjectKind::Mesh(exported));
}

#[test]
fn heavy_arrays_are_worked_out_on_another_thread() {
    let mut doc = Document::new();
    let sphere = create(&mut doc, PrimitiveKind::Sphere);
    let input = doc.scene().unmodified_shape(sphere).unwrap();
    let light = array(ArrayOptions::default(), None);
    assert!(light.work(&input, Purpose::Viewport) < MODIFIERS_IN_BACKGROUND_FROM);
    let heavy = array(
        ArrayOptions {
            layout: ArrayLayout::Grid,
            counts: [20, 20, 1],
            ..ArrayOptions::default()
        },
        None,
    );
    assert!(heavy.work(&input, Purpose::Viewport) >= MODIFIERS_IN_BACKGROUND_FROM);
}
