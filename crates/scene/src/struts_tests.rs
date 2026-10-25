//! Tests for the Struts modifier in the scene (decision 0159): adding it,
//! where a new one goes in the list, following the shape, baking and
//! heavy work.

use crate::*;
use glam::Vec3;

fn create(doc: &mut Document, kind: PrimitiveKind) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
        .unwrap();
    doc.selection()[0]
}

fn struts() -> ModifierKind {
    ModifierKind::Struts(StrutsOptions::default())
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
fn a_cube_shows_and_exports_as_a_frame() {
    let mut doc = Document::new();
    let cube = create(&mut doc, PrimitiveKind::Cube);
    add(&mut doc, cube, struts());
    assert_eq!(doc.undo_label(), Some("Add Struts"));
    let scene = doc.scene();
    let input = scene.unmodified_shape(cube).unwrap();
    let shown = scene.shown_shape(cube).unwrap();
    assert_eq!(scene.exported_shape(cube), Some(shown.clone()));
    assert!(scene.modifier_problems(cube).is_empty());
    let report = mesh::MeshReport::new(&shown);
    assert_eq!(report.border_edges, 0);
    assert_eq!(report.pieces, input.edges().len() + input.vertex_count());
    assert_eq!(struts().family(), ModifierFamily::Generate);
    assert!(!struts().differs_by_purpose());
    assert!(!struts().carries_custom_normals());
    assert!(struts().sources().is_empty());
    assert!(!struts().reads_pivot());
    doc.undo();
    assert!(doc.scene().modifiers(cube).is_empty());
}

#[test]
fn it_follows_shell_on_the_palette() {
    let kinds = ModifierKind::standard_kinds();
    let at = |name: &str| kinds.iter().position(|k| k.name() == name).unwrap();
    assert_eq!(at("Struts"), at("Shell") + 1);
}

#[test]
fn settings_are_clamped_when_added() {
    let mut doc = Document::new();
    let cube = create(&mut doc, PrimitiveKind::Cube);
    add(
        &mut doc,
        cube,
        ModifierKind::Struts(StrutsOptions {
            thickness: f32::NAN,
            sides: 0,
            ..StrutsOptions::default()
        }),
    );
    assert_eq!(
        doc.scene().modifiers(cube)[0].kind,
        ModifierKind::Struts(StrutsOptions {
            sides: StrutsOptions::MIN_SIDES,
            ..StrutsOptions::default()
        })
    );
}

#[test]
fn a_new_one_goes_before_bevels_smoothing_and_weighted_normals() {
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Cube);
    add(&mut doc, id, ModifierKind::LiveBevel(Default::default()));
    add(&mut doc, id, ModifierKind::Subdivide(Default::default()));
    add(
        &mut doc,
        id,
        ModifierKind::WeightedNormals(Default::default()),
    );
    add(&mut doc, id, struts());
    assert_eq!(
        names(&doc, id),
        ["Struts", "Live bevel", "Subdivide", "Weighted normals"]
    );
    // Weld by distance and Quadify go before it.
    add(&mut doc, id, ModifierKind::Quadify(Default::default()));
    add(
        &mut doc,
        id,
        ModifierKind::WeldByDistance(Default::default()),
    );
    assert_eq!(
        names(&doc, id)[..3],
        ["Weld by distance", "Quadify", "Struts"]
    );
    // Without any of those, it goes at the end.
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Cube);
    add(&mut doc, id, ModifierKind::Array(Default::default()));
    add(&mut doc, id, struts());
    assert_eq!(names(&doc, id), ["Array", "Struts"]);
}

#[test]
fn the_struts_follow_the_shape_as_it_is_modelled() {
    let mut doc = Document::new();
    let cube = create(&mut doc, PrimitiveKind::Cube);
    add(&mut doc, cube, struts());
    let before = doc.scene().shown_shape(cube).unwrap().bounds().unwrap();
    let shape = doc.scene().evaluated(cube).unwrap().mesh;
    let points: Vec<VertexId> = shape.vertices().map(|(v, _)| v).collect();
    doc.execute(Command::Record {
        id: cube,
        operation: Operation::MovePoints {
            points,
            offset: Vec3::new(0.0, 0.0, 30.0),
            anchors: None,
        },
    })
    .unwrap();
    let after = doc.scene().shown_shape(cube).unwrap().bounds().unwrap();
    assert!((after.min.z - before.min.z - 30.0).abs() < 1e-3);
    // Changing its settings works it out again.
    let mut round = doc.scene().modifiers(cube)[0].clone();
    round.kind = ModifierKind::Struts(StrutsOptions {
        sides: 8,
        keep_faces: true,
        ..StrutsOptions::default()
    });
    doc.execute(Command::ChangeModifier {
        id: cube,
        index: 0,
        modifier: round,
    })
    .unwrap();
    assert_eq!(doc.undo_label(), Some("Change Struts"));
    let shown = doc.scene().shown_shape(cube).unwrap();
    // 12 struts of 10 polygons, 8 joints of 3 bands of 8 and two ends,
    // and the cube's 6.
    assert_eq!(shown.face_count(), 12 * 10 + 8 * 26 + 6);
}

#[test]
fn bake_into_shape_keeps_the_frame() {
    let mut doc = Document::new();
    let cube = create(&mut doc, PrimitiveKind::Cube);
    add(&mut doc, cube, struts());
    let exported = doc.scene().exported_shape(cube).unwrap();
    doc.execute(Command::MakeEditable(vec![cube])).unwrap();
    let scene = doc.scene();
    assert!(scene.modifiers(cube).is_empty());
    assert_eq!(scene.object(cube).unwrap().kind, ObjectKind::Mesh(exported));
}

#[test]
fn heavy_struts_are_worked_out_on_another_thread() {
    let mut doc = Document::new();
    let cube = create(&mut doc, PrimitiveKind::Cube);
    let input = doc.scene().unmodified_shape(cube).unwrap();
    assert!(struts().work(&input, Purpose::Viewport) < MODIFIERS_IN_BACKGROUND_FROM);
    let big = mesh::Primitive::Plane(mesh::primitives::PlaneParams {
        width: 100.0,
        depth: 100.0,
        segments: [50, 50],
    })
    .mesh();
    assert!(struts().work(&big, Purpose::Viewport) >= MODIFIERS_IN_BACKGROUND_FROM);
}
