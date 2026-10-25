//! Tests for the Triangulate modifier in the scene (decision 0161):
//! adding it, where it goes in the list, keeping Weighted normals'
//! custom normals, baking and heavy work.

use crate::*;
use mesh::custom_normals::has_custom_normals;

fn create(doc: &mut Document, kind: PrimitiveKind) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
        .unwrap();
    doc.selection()[0]
}

fn triangulate() -> ModifierKind {
    ModifierKind::Triangulate(TriangulateOptions::default())
}

fn add(doc: &mut Document, id: ObjectId, kind: ModifierKind) {
    doc.execute(Command::AddModifier {
        ids: vec![id],
        kind,
    })
    .unwrap();
}

fn names(doc: &Document, id: ObjectId) -> Vec<&'static str> {
    doc.scene()
        .modifiers(id)
        .iter()
        .map(|m| m.kind.name())
        .collect()
}

#[test]
fn a_cube_shows_and_exports_as_triangles() {
    let mut doc = Document::new();
    let cube = create(&mut doc, PrimitiveKind::Cube);
    add(&mut doc, cube, triangulate());
    assert_eq!(doc.undo_label(), Some("Add Triangulate"));
    let scene = doc.scene();
    let shown = scene.shown_shape(cube).unwrap();
    assert_eq!(scene.exported_shape(cube), Some(shown.clone()));
    assert!(scene.modifier_problems(cube).is_empty());
    assert_eq!(shown.face_count(), 12);
    assert!(shown.faces().all(|(_, c)| c.len() == 3));
    let kind = triangulate();
    assert_eq!(kind.family(), ModifierFamily::Game);
    assert!(!kind.differs_by_purpose());
    assert!(kind.carries_custom_normals());
    assert!(kind.keeps_shading());
    assert!(kind.sources().is_empty());
    assert!(!kind.reads_pivot());
    doc.undo();
    assert!(doc.scene().modifiers(cube).is_empty());
}

#[test]
fn it_is_last_on_the_palette() {
    let kinds = ModifierKind::standard_kinds();
    assert_eq!(kinds.last().map(|k| k.name()), Some("Triangulate"));
    let at = |name: &str| kinds.iter().position(|k| k.name() == name).unwrap();
    assert_eq!(at("Triangulate"), at("Weighted normals") + 1);
}

#[test]
fn every_other_kind_goes_before_it() {
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Cube);
    add(&mut doc, id, triangulate());
    add(&mut doc, id, ModifierKind::Subdivide(Default::default()));
    add(
        &mut doc,
        id,
        ModifierKind::WeightedNormals(Default::default()),
    );
    add(&mut doc, id, ModifierKind::Quadify(Default::default()));
    assert_eq!(
        names(&doc, id),
        ["Quadify", "Subdivide", "Weighted normals", "Triangulate"]
    );
    for kind in ModifierKind::standard_kinds() {
        assert_eq!(
            kind.goes_before(&triangulate()),
            kind.name() != "Triangulate",
            "{}",
            kind.name()
        );
    }
    // A second Triangulate goes at the end, after the first.
    add(&mut doc, id, triangulate());
    assert_eq!(names(&doc, id)[3..], ["Triangulate", "Triangulate"]);
}

#[test]
fn weighted_normals_before_it_reach_the_export() {
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Cylinder);
    add(
        &mut doc,
        id,
        ModifierKind::WeightedNormals(Default::default()),
    );
    add(&mut doc, id, triangulate());
    let scene = doc.scene();
    assert_eq!(names(&doc, id), ["Weighted normals", "Triangulate"]);
    let exported = scene.exported_shape(id).unwrap();
    assert!(has_custom_normals(&exported));
    assert!(exported.faces().all(|(_, c)| c.len() == 3));
    assert!(scene.final_weighted_normals(id, Purpose::Export).is_some());
    assert_eq!(scene.normals_dropped_by(id, 0), None);
    // A Subdivide after them, before the Triangulate, still drops them,
    // and is named. (A new Subdivide goes before both: moved by hand.)
    add(&mut doc, id, ModifierKind::Subdivide(Default::default()));
    doc.execute(Command::MoveModifier { id, from: 0, to: 1 })
        .unwrap();
    assert_eq!(
        names(&doc, id),
        ["Weighted normals", "Subdivide", "Triangulate"]
    );
    assert_eq!(doc.scene().normals_dropped_by(id, 0), Some("Subdivide"));
    assert!(!has_custom_normals(
        &doc.scene().exported_shape(id).unwrap()
    ));
}

#[test]
fn an_all_triangle_shape_is_shown_as_it_is_with_a_reason() {
    let mut doc = Document::new();
    let cube = create(&mut doc, PrimitiveKind::Cube);
    add(&mut doc, cube, triangulate());
    add(&mut doc, cube, triangulate());
    let scene = doc.scene();
    let problems = scene.modifier_problems(cube);
    assert_eq!(problems.len(), 1);
    assert_eq!(problems[0].0, 1);
    assert!(problems[0].1.contains("triangle"), "{}", problems[0].1);
    assert_eq!(scene.shown_shape(cube).unwrap().face_count(), 12);
}

#[test]
fn bake_into_shape_keeps_the_triangles() {
    let mut doc = Document::new();
    let cube = create(&mut doc, PrimitiveKind::Cube);
    add(
        &mut doc,
        cube,
        ModifierKind::Triangulate(TriangulateOptions {
            method: TriangulateMethod::Fixed,
            quads_only: true,
        }),
    );
    let exported = doc.scene().exported_shape(cube).unwrap();
    doc.execute(Command::MakeEditable(vec![cube])).unwrap();
    let scene = doc.scene();
    assert!(scene.modifiers(cube).is_empty());
    assert_eq!(scene.object(cube).unwrap().kind, ObjectKind::Mesh(exported));
}

#[test]
fn heavy_shapes_are_triangulated_on_another_thread() {
    let mut doc = Document::new();
    let cube = create(&mut doc, PrimitiveKind::Cube);
    let input = doc.scene().unmodified_shape(cube).unwrap();
    assert!(triangulate().work(&input, Purpose::Viewport) < MODIFIERS_IN_BACKGROUND_FROM);
    let big = mesh::Primitive::Plane(mesh::primitives::PlaneParams {
        width: 100.0,
        depth: 100.0,
        segments: [160, 160],
    })
    .mesh();
    assert!(triangulate().work(&big, Purpose::Viewport) >= MODIFIERS_IN_BACKGROUND_FROM);
    // Nothing to cut is no work.
    let triangles = mesh::triangulate::triangulated(&big, &TriangulateOptions::default()).unwrap();
    assert_eq!(triangulate().work(&triangles, Purpose::Viewport), 0);
}
