//! Tests for the Scatter modifier (decision 0146): a modifier reading
//! other shapes through the stack's sources (decision 0142), so deleting
//! a shape scattered, copies following copies, and loops refused.

use crate::*;
use glam::Vec3;

fn create(doc: &mut Document, kind: PrimitiveKind) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
        .unwrap();
    doc.selection()[0]
}

fn scatter(shapes: Vec<ObjectId>, place_on: PlaceOn) -> ModifierKind {
    let mut settings = ScatterSettings::of(shapes);
    settings.options.place_on = place_on;
    ModifierKind::Scatter(settings)
}

/// A plane (the surface) and a cube scattered on its four corners.
fn rocks_on_a_plane() -> (Document, ObjectId, ObjectId) {
    let mut doc = Document::new();
    let plane = create(&mut doc, PrimitiveKind::Plane);
    let cube = create(&mut doc, PrimitiveKind::Cube);
    doc.execute(Command::AddModifier {
        ids: vec![plane],
        kind: scatter(vec![cube], PlaceOn::Points),
    })
    .unwrap();
    (doc, plane, cube)
}

fn polygons(doc: &Document, id: ObjectId) -> usize {
    doc.scene().shown_shape(id).unwrap().face_count()
}

fn plane_polygons(doc: &Document, plane: ObjectId) -> usize {
    doc.scene().unmodified_shape(plane).unwrap().face_count()
}

fn corners(doc: &Document, plane: ObjectId) -> usize {
    doc.scene().unmodified_shape(plane).unwrap().vertex_count()
}

#[test]
fn the_surface_carries_the_copies_and_undo_takes_them_away() {
    let (mut doc, plane, cube) = rocks_on_a_plane();
    let cube_polygons = doc.scene().shown_shape(cube).unwrap().face_count();
    let expected = plane_polygons(&doc, plane) + corners(&doc, plane) * cube_polygons;
    assert_eq!(polygons(&doc, plane), expected);
    assert_eq!(doc.scene().modifier_problems(plane), vec![]);
    assert_eq!(doc.undo_label(), Some("Add Scatter"));
    doc.undo();
    assert_eq!(polygons(&doc, plane), plane_polygons(&doc, plane));
    // The cube is left alone, a shape of its own.
    assert_eq!(doc.scene().shown_shape(cube).unwrap().face_count(), 6);
}

#[test]
fn copies_follow_the_shape_scattered_hidden_or_not() {
    let (mut doc, plane, cube) = rocks_on_a_plane();
    let before = polygons(&doc, plane);
    // Hidden, it still scatters.
    doc.execute(Command::SetVisible {
        ids: vec![cube],
        visible: false,
    })
    .unwrap();
    assert_eq!(polygons(&doc, plane), before);
    // Editing it changes every copy: a cube with more segments.
    let mut params = match &doc.scene().object(cube).unwrap().kind {
        ObjectKind::Primitive(Primitive::Cube(p)) => *p,
        other => panic!("{other:?}"),
    };
    params.segments = [2, 1, 1];
    doc.execute(Command::SetPrimitive {
        ids: vec![cube],
        primitive: Primitive::Cube(params),
    })
    .unwrap();
    let corners = corners(&doc, plane);
    assert_eq!(polygons(&doc, plane), before + corners * 4);
}

#[test]
fn copies_stand_on_their_pivot_whatever_the_shape_s_place() {
    let (mut doc, plane, cube) = rocks_on_a_plane();
    let shape = |doc: &Document| doc.scene().shown_shape(plane).unwrap();
    let before = shape(&doc);
    // Moving the cube elsewhere doesn't move the copies.
    doc.execute(Command::SetTransforms(vec![(
        cube,
        Transform {
            position: Vec3::new(500.0, 300.0, 40.0),
            ..Transform::default()
        },
    )]))
    .unwrap();
    assert_eq!(
        shape(&doc).bounds().unwrap(),
        before.bounds().unwrap(),
        "the copies stay where they stand"
    );
    // Sizing it sizes them.
    doc.execute(Command::SetTransforms(vec![(
        cube,
        Transform {
            scale: Vec3::splat(2.0),
            ..Transform::default()
        },
    )]))
    .unwrap();
    let top = shape(&doc).bounds().unwrap().max.z;
    assert!(
        (top - 2.0 * before.bounds().unwrap().max.z).abs() < 1e-2,
        "{top}"
    );
}

#[test]
fn deleting_a_shape_takes_it_out_and_the_last_removes_the_modifier() {
    let (mut doc, plane, cube) = rocks_on_a_plane();
    let sphere = create(&mut doc, PrimitiveKind::Sphere);
    doc.execute(Command::AddModifier {
        ids: vec![plane],
        kind: scatter(vec![cube, sphere], PlaceOn::Centres),
    })
    .unwrap();
    doc.execute(Command::Delete(vec![sphere])).unwrap();
    let list = doc.scene().modifiers(plane);
    assert_eq!(list.len(), 2);
    let ModifierKind::Scatter(second) = &list[1].kind else {
        panic!("a scatter")
    };
    assert_eq!(second.shapes, vec![cube], "the sphere is taken out");
    // One undo step brings it back, in both.
    doc.undo();
    let ModifierKind::Scatter(second) = &doc.scene().modifiers(plane)[1].kind else {
        panic!("a scatter")
    };
    assert_eq!(second.shapes, vec![cube, sphere]);
    // Deleting the cube ends both modifiers (the second keeps the sphere).
    doc.execute(Command::Delete(vec![cube])).unwrap();
    let list = doc.scene().modifiers(plane);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].kind.sources(), vec![sphere]);
    doc.execute(Command::Delete(vec![sphere])).unwrap();
    assert!(doc.scene().modifiers(plane).is_empty());
    assert_eq!(polygons(&doc, plane), plane_polygons(&doc, plane));
}

#[test]
fn copies_follow_copied_shapes() {
    let (mut doc, plane, cube) = rocks_on_a_plane();
    // The surface alone: its copy scatters the same cube.
    doc.execute(Command::Duplicate {
        ids: vec![plane],
        instances: false,
    })
    .unwrap();
    let alone = doc.selection()[0];
    assert_eq!(doc.scene().modifiers(alone)[0].kind.sources(), vec![cube]);
    // The surface with its cube: the copy scatters the copied cube.
    doc.execute(Command::Duplicate {
        ids: vec![plane, cube],
        instances: false,
    })
    .unwrap();
    let copies = doc.selection();
    let plane_copy = copies
        .iter()
        .copied()
        .find(|id| !doc.scene().modifiers(*id).is_empty())
        .unwrap();
    let cube_copy = copies.iter().copied().find(|id| *id != plane_copy).unwrap();
    assert_eq!(
        doc.scene().modifiers(plane_copy)[0].kind.sources(),
        vec![cube_copy]
    );
}

#[test]
fn loops_are_refused() {
    let (mut doc, plane, cube) = rocks_on_a_plane();
    let add = |doc: &mut Document, id: ObjectId, shapes: Vec<ObjectId>| {
        doc.execute(Command::AddModifier {
            ids: vec![id],
            kind: scatter(shapes, PlaceOn::Random),
        })
    };
    let bad = Err(SceneError::Modifier(ModifierRefusal::BadSource));
    // Onto itself.
    assert_eq!(add(&mut doc, cube, vec![cube]), bad);
    // The plane scatters the cube; the cube can't scatter the plane.
    assert_eq!(add(&mut doc, cube, vec![plane]), bad);
    // Three round: a sphere scattered on the cube, the plane on the sphere.
    let sphere = create(&mut doc, PrimitiveKind::Sphere);
    add(&mut doc, cube, vec![sphere]).unwrap();
    assert_eq!(add(&mut doc, sphere, vec![plane]), bad);
    // An instance of the plane is made from it.
    doc.execute(Command::Duplicate {
        ids: vec![plane],
        instances: true,
    })
    .unwrap();
    let instance = doc.selection()[0];
    assert_eq!(add(&mut doc, plane, vec![instance]), bad);
    // No shapes, a group, or a cage.
    assert_eq!(
        add(&mut doc, plane, vec![]),
        Err(SceneError::Modifier(ModifierRefusal::NoSource))
    );
    doc.execute(Command::Create(ObjectKind::Group)).unwrap();
    let group = doc.selection()[0];
    assert_eq!(add(&mut doc, plane, vec![group]), bad);
    // A cutter of the plane can be scattered on it (it reads no shape
    // round in a circle).
    let cutter = create(&mut doc, PrimitiveKind::Cylinder);
    doc.execute(Command::AddBoolean {
        kept: plane,
        cutters: vec![cutter],
        kind: BooleanKind::Union,
    })
    .unwrap();
    add(&mut doc, plane, vec![cutter]).unwrap();
}

#[test]
fn materials_are_matched_by_name() {
    let (mut doc, plane, cube) = rocks_on_a_plane();
    let named = |name: &str| Material {
        name: name.to_string(),
        color: [0.5, 0.5, 0.5],
    };
    doc.execute(Command::SetMaterials {
        id: plane,
        materials: vec![named("Ground"), named("Stone")],
    })
    .unwrap();
    doc.execute(Command::SetMaterials {
        id: cube,
        materials: vec![named("Stone")],
    })
    .unwrap();
    let shown = doc.scene().shown_shape(plane).unwrap();
    let stone = mesh::material::faces_with_slot(&shown, 1).len();
    assert_eq!(stone, corners(&doc, plane) * 6);
    // A material the surface hasn't: its first.
    doc.execute(Command::SetMaterials {
        id: cube,
        materials: vec![named("Moss")],
    })
    .unwrap();
    let shown = doc.scene().shown_shape(plane).unwrap();
    assert!(mesh::material::faces_with_slot(&shown, 1).is_empty());
}

#[test]
fn only_the_chosen_polygons_get_copies_and_lost_ones_say_why() {
    let mut doc = Document::new();
    let cube = create(&mut doc, PrimitiveKind::Cube);
    let rock = create(&mut doc, PrimitiveKind::Cube);
    let top = {
        let shape = doc.scene().unmodified_shape(cube).unwrap();
        shape
            .faces()
            .map(|(f, _)| f)
            .find(|f| shape.face_normal(*f).z > 0.9)
            .unwrap()
    };
    let mut settings = ScatterSettings::of(vec![rock]);
    settings.options.place_on = PlaceOn::Centres;
    settings.polygons = vec![top, top];
    doc.execute(Command::AddModifier {
        ids: vec![cube],
        kind: ModifierKind::Scatter(settings.clone()),
    })
    .unwrap();
    let ModifierKind::Scatter(kept) = &doc.scene().modifiers(cube)[0].kind else {
        panic!()
    };
    assert_eq!(kept.polygons, vec![top], "listed once");
    assert_eq!(polygons(&doc, cube), 12);
    settings.polygons = vec![FaceId::from_raw(4_000)];
    let mut changed = doc.scene().modifiers(cube)[0].clone();
    changed.kind = ModifierKind::Scatter(settings);
    doc.execute(Command::ChangeModifier {
        id: cube,
        index: 0,
        modifier: changed,
    })
    .unwrap();
    assert_eq!(polygons(&doc, cube), 6, "the input passes on");
    let problems = doc.scene().modifier_problems(cube);
    assert_eq!(problems.len(), 1);
    assert!(problems[0].1.contains("polygons"), "{problems:?}");
}

#[test]
fn bake_into_shape_keeps_the_copies_and_the_shapes() {
    let (mut doc, plane, cube) = rocks_on_a_plane();
    let exported = doc.scene().exported_shape(plane).unwrap();
    doc.execute(Command::MakeEditable(vec![plane])).unwrap();
    let scene = doc.scene();
    assert!(scene.modifiers(plane).is_empty());
    assert_eq!(
        scene.object(plane).unwrap().kind,
        ObjectKind::Mesh(exported)
    );
    assert!(scene.contains(cube));
    // Now the cube can be deleted without changing the plane.
    let before = polygons(&doc, plane);
    doc.execute(Command::Delete(vec![cube])).unwrap();
    assert_eq!(polygons(&doc, plane), before);
}

#[test]
fn heavy_scatters_are_worked_out_on_another_thread() {
    let (doc, plane, cube) = rocks_on_a_plane();
    let scene = doc.scene();
    let light = &scene.modifiers(plane)[0].kind;
    let sources = vec![SourceShape {
        id: cube,
        data: SourceData::Shape(scene.shown_shape(cube).unwrap()),
        matrix: glam::Mat4::IDENTITY,
        pivot: Vec3::ZERO,
        materials: vec![0],
    }];
    let input = scene.unmodified_shape(plane).unwrap();
    assert!(light.work_with(&input, Purpose::Viewport, &sources) < MODIFIERS_IN_BACKGROUND_FROM);
    let mut heavy = ScatterSettings::of(vec![cube]);
    heavy.options.count = 20_000;
    let heavy = ModifierKind::Scatter(heavy);
    assert!(heavy.work_with(&input, Purpose::Viewport, &sources) >= MODIFIERS_IN_BACKGROUND_FROM);
}
