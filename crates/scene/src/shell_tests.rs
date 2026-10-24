//! Tests for the Shell modifier in the scene (decision 0154): adding it,
//! where a new one goes in the list, following the shape, baking,
//! custom normals and heavy work.

use crate::*;
use glam::Vec3;
use mesh::custom_normals::has_custom_normals;

fn create(doc: &mut Document, kind: PrimitiveKind) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
        .unwrap();
    doc.selection()[0]
}

fn shell() -> ModifierKind {
    ModifierKind::Shell(ShellOptions::default())
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
fn a_plane_shows_and_exports_as_a_closed_slab() {
    let mut doc = Document::new();
    let plane = create(&mut doc, PrimitiveKind::Plane);
    add(&mut doc, plane, shell());
    assert_eq!(doc.undo_label(), Some("Add Shell"));
    let scene = doc.scene();
    let input = scene.unmodified_shape(plane).unwrap();
    let shown = scene.shown_shape(plane).unwrap();
    assert_eq!(scene.exported_shape(plane), Some(shown.clone()));
    assert!(scene.modifier_problems(plane).is_empty());
    let report = mesh::MeshReport::new(&shown);
    assert_eq!(report.border_edges, 0);
    assert_eq!(report.non_manifold_edges, 0);
    let borders = input.topology().border_edges().len();
    assert_eq!(shown.face_count(), input.face_count() * 2 + borders);
    // 1 cm thick, inward: the plane stays where it was.
    let low = shown.vertices().map(|(_, p)| p.z).fold(f32::MAX, f32::min);
    let high = shown.vertices().map(|(_, p)| p.z).fold(f32::MIN, f32::max);
    assert!((high - low - 1.0).abs() < 1e-4);
    assert!((high - input.bounds().unwrap().max.z).abs() < 1e-4);
    assert_eq!(shell().family(), ModifierFamily::Generate);
    assert!(!shell().differs_by_purpose());
    assert!(!shell().carries_custom_normals());
    assert!(shell().sources().is_empty());
    doc.undo();
    assert!(doc.scene().modifiers(plane).is_empty());
}

#[test]
fn it_follows_array_on_the_palette() {
    let kinds = ModifierKind::standard_kinds();
    let at = |name: &str| kinds.iter().position(|k| k.name() == name).unwrap();
    assert_eq!(at("Shell"), at("Array") + 1);
}

#[test]
fn settings_are_clamped_when_added() {
    let mut doc = Document::new();
    let plane = create(&mut doc, PrimitiveKind::Plane);
    add(
        &mut doc,
        plane,
        ModifierKind::Shell(ShellOptions {
            thickness: f32::NAN,
            ..ShellOptions::default()
        }),
    );
    assert_eq!(doc.scene().modifiers(plane)[0].kind, shell());
}

#[test]
fn a_new_shell_goes_before_bevels_smoothing_and_weighted_normals() {
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Cylinder);
    add(&mut doc, id, ModifierKind::Decimate(Default::default()));
    add(&mut doc, id, ModifierKind::LiveBevel(Default::default()));
    add(&mut doc, id, ModifierKind::Subdivide(Default::default()));
    add(
        &mut doc,
        id,
        ModifierKind::WeightedNormals(Default::default()),
    );
    add(&mut doc, id, shell());
    assert_eq!(
        names(&doc, id),
        [
            "Decimate",
            "Shell",
            "Live bevel",
            "Subdivide",
            "Weighted normals"
        ]
    );
    // A Weld by distance goes before it: a crack would grow a rim.
    add(
        &mut doc,
        id,
        ModifierKind::WeldByDistance(Default::default()),
    );
    assert_eq!(names(&doc, id)[0], "Weld by distance");
    // Without any of those, it goes at the end.
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Cube);
    add(&mut doc, id, ModifierKind::Array(Default::default()));
    add(&mut doc, id, shell());
    assert_eq!(names(&doc, id), ["Array", "Shell"]);
}

#[test]
fn the_shell_follows_the_shape_as_it_is_modelled() {
    let mut doc = Document::new();
    let plane = create(&mut doc, PrimitiveKind::Plane);
    add(&mut doc, plane, shell());
    let shape = doc.scene().evaluated(plane).unwrap().mesh;
    let points: Vec<VertexId> = shape.vertices().map(|(v, _)| v).collect();
    doc.execute(Command::Record {
        id: plane,
        operation: Operation::MovePoints {
            points,
            offset: Vec3::new(0.0, 0.0, 30.0),
            anchors: None,
        },
    })
    .unwrap();
    let shown = doc.scene().shown_shape(plane).unwrap();
    let low = shown.vertices().map(|(_, p)| p.z).fold(f32::MAX, f32::min);
    assert!((low - 29.0).abs() < 1e-4, "{low}");
    // Changing its settings works it out again.
    let mut thicker = doc.scene().modifiers(plane)[0].clone();
    thicker.kind = ModifierKind::Shell(ShellOptions {
        thickness: 4.0,
        direction: ShellDirection::Outward,
        ..ShellOptions::default()
    });
    doc.execute(Command::ChangeModifier {
        id: plane,
        index: 0,
        modifier: thicker,
    })
    .unwrap();
    assert_eq!(doc.undo_label(), Some("Change Shell"));
    let shown = doc.scene().shown_shape(plane).unwrap();
    let high = shown.vertices().map(|(_, p)| p.z).fold(f32::MIN, f32::max);
    assert!((high - 34.0).abs() < 1e-4, "{high}");
}

#[test]
fn bake_into_shape_keeps_the_thickness() {
    let mut doc = Document::new();
    let plane = create(&mut doc, PrimitiveKind::Plane);
    add(&mut doc, plane, shell());
    let exported = doc.scene().exported_shape(plane).unwrap();
    doc.execute(Command::MakeEditable(vec![plane])).unwrap();
    let scene = doc.scene();
    assert!(scene.modifiers(plane).is_empty());
    assert_eq!(
        scene.object(plane).unwrap().kind,
        ObjectKind::Mesh(exported)
    );
}

#[test]
fn weighted_normals_after_it_still_reach_the_export() {
    let mut doc = Document::new();
    let cube = create(&mut doc, PrimitiveKind::Cube);
    add(
        &mut doc,
        cube,
        ModifierKind::WeightedNormals(Default::default()),
    );
    add(&mut doc, cube, shell());
    assert_eq!(names(&doc, cube), ["Shell", "Weighted normals"]);
    let shape = doc.scene().exported_shape(cube).unwrap();
    assert!(has_custom_normals(&shape));
    // Moved after it, the Shell drops them (decision 0148).
    doc.execute(Command::MoveModifier {
        id: cube,
        from: 0,
        to: 1,
    })
    .unwrap();
    let shape = doc.scene().exported_shape(cube).unwrap();
    assert!(!has_custom_normals(&shape));
}

#[test]
fn heavy_shells_are_worked_out_on_another_thread() {
    let mut doc = Document::new();
    let sphere = create(&mut doc, PrimitiveKind::Sphere);
    let input = doc.scene().unmodified_shape(sphere).unwrap();
    assert!(shell().work(&input, Purpose::Viewport) < MODIFIERS_IN_BACKGROUND_FROM);
    let big = mesh::Primitive::Plane(mesh::primitives::PlaneParams {
        width: 100.0,
        depth: 100.0,
        segments: [200, 200],
    })
    .mesh();
    assert!(shell().work(&big, Purpose::Viewport) >= MODIFIERS_IN_BACKGROUND_FROM);
}
