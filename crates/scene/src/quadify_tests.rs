//! Tests for Quadify in the scene (decision 0150): the modifier, and the
//! one-off recorded edit.

use crate::*;
use mesh::decimate::{Decimator, Keep};
use mesh::quadify::QuadifyProblem;
use std::sync::Arc;

fn create(doc: &mut Document, kind: PrimitiveKind) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
        .unwrap();
    doc.selection()[0]
}

fn quadify() -> ModifierKind {
    ModifierKind::Quadify(QuadifyOptions::default())
}

fn add(doc: &mut Document, id: ObjectId, kind: ModifierKind) {
    doc.execute(Command::AddModifier {
        ids: vec![id],
        kind,
    })
    .unwrap();
}

/// `mesh` cut into triangles, keeping its UVs, materials and marks.
fn triangles_of(mesh: &Mesh) -> Mesh {
    Decimator::new(mesh, Keep::default(), Hardness::Marked)
        .unwrap()
        .mesh(false)
}

/// A plain mesh object: the standard `kind` cut into triangles.
fn triangulated(doc: &mut Document, kind: PrimitiveKind) -> ObjectId {
    let mesh = triangles_of(&kind.standard().mesh());
    doc.execute(Command::Create(ObjectKind::Mesh(Arc::new(mesh))))
        .unwrap();
    doc.selection()[0]
}

fn quads(mesh: &Mesh) -> usize {
    mesh.faces().filter(|(_, c)| c.len() == 4).count()
}

fn triangles(mesh: &Mesh) -> usize {
    mesh.faces().filter(|(_, c)| c.len() == 3).count()
}

#[test]
fn a_triangulated_cube_shows_and_exports_as_six_quads() {
    let mut doc = Document::new();
    let id = triangulated(&mut doc, PrimitiveKind::Cube);
    add(&mut doc, id, quadify());
    let scene = doc.scene();
    assert_eq!(doc.undo_label(), Some("Add Quadify"));
    assert_eq!(scene.evaluated(id).unwrap().mesh.face_count(), 12);
    let shown = scene.shown_shape(id).unwrap();
    assert_eq!((quads(&shown), shown.face_count()), (6, 6));
    assert_eq!(scene.shown_shape(id), scene.exported_shape(id));
    assert!(scene.modifier_problems(id).is_empty());
    assert_eq!(quadify().family(), ModifierFamily::CleanUp);
    assert!(!quadify().differs_by_purpose());
    assert!(!quadify().carries_custom_normals());
    assert!(quadify().sources().is_empty());
}

#[test]
fn a_shape_with_no_triangles_says_why() {
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Cube);
    add(&mut doc, id, quadify());
    let scene = doc.scene();
    assert_eq!(
        scene.modifier_problems(id),
        vec![(0, QuadifyProblem::NoTriangles.to_string())]
    );
    // The shape goes on unchanged.
    assert_eq!(
        *scene.shown_shape(id).unwrap(),
        *scene.unmodified_shape(id).unwrap()
    );
}

#[test]
fn quadify_tidies_up_after_decimate() {
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Sphere);
    add(
        &mut doc,
        id,
        ModifierKind::Decimate(DecimateOptions::default()),
    );
    let decimated = doc.scene().shown_shape(id).unwrap();
    assert_eq!(quads(&decimated), 0);
    add(&mut doc, id, quadify());
    let shown = doc.scene().shown_shape(id).unwrap();
    assert!(quads(&shown) > 0, "{}", shown.face_count());
    assert!(shown.face_count() < decimated.face_count());
    let report = mesh::MeshReport::new(&shown);
    assert!(report.is_closed(), "{report:?}");
}

#[test]
fn a_new_quadify_goes_before_bevels_and_smoothing() {
    let mut doc = Document::new();
    let id = triangulated(&mut doc, PrimitiveKind::Cube);
    add(
        &mut doc,
        id,
        ModifierKind::Decimate(DecimateOptions::default()),
    );
    add(
        &mut doc,
        id,
        ModifierKind::Subdivide(SubdivideOptions::default()),
    );
    add(
        &mut doc,
        id,
        ModifierKind::LiveBevel(LiveBevelOptions::default()),
    );
    add(&mut doc, id, quadify());
    let kinds: Vec<&'static str> = doc
        .scene()
        .modifiers(id)
        .iter()
        .map(|m| m.kind.name())
        .collect();
    assert_eq!(
        kinds,
        vec!["Decimate", "Quadify", "Live bevel", "Subdivide"]
    );
    // Smoothing a quadified cube gives quads only.
    let shown = doc.scene().shown_shape(id).unwrap();
    assert_eq!(triangles(&shown), 0);
}

#[test]
fn changing_settings_works_it_out_again_and_bake_makes_it_permanent() {
    let mut doc = Document::new();
    let id = triangulated(&mut doc, PrimitiveKind::Sphere);
    add(&mut doc, id, quadify());
    let standard = doc.scene().shown_shape(id).unwrap();
    let mut strict = doc.scene().modifiers(id)[0].clone();
    strict.kind = ModifierKind::Quadify(QuadifyOptions {
        max_distortion: 0.5,
        ..QuadifyOptions::default()
    });
    doc.execute(Command::ChangeModifier {
        id,
        index: 0,
        modifier: strict,
    })
    .unwrap();
    let fewer = doc.scene().shown_shape(id).unwrap();
    assert!(quads(&fewer) < quads(&standard));
    assert_eq!(doc.undo_label(), Some("Change Quadify"));
    doc.undo();
    assert_eq!(*doc.scene().shown_shape(id).unwrap(), *standard);
    // Bake into shape keeps what an export gets.
    let expected = doc.scene().exported_shape(id).unwrap();
    doc.execute(Command::MakeEditable(vec![id])).unwrap();
    let object = doc.scene().object(id).unwrap();
    assert!(object.modifiers.is_empty());
    let ObjectKind::Mesh(baked) = &object.kind else {
        panic!("a mesh")
    };
    assert_eq!(**baked, *expected);
}

#[test]
fn settings_are_brought_within_limits() {
    let mut doc = Document::new();
    let id = triangulated(&mut doc, PrimitiveKind::Cube);
    add(
        &mut doc,
        id,
        ModifierKind::Quadify(QuadifyOptions {
            max_angle: f32::NAN,
            max_distortion: 1e9,
            ..QuadifyOptions::default()
        }),
    );
    let ModifierKind::Quadify(o) = doc.scene().modifiers(id)[0].kind else {
        panic!("quadify")
    };
    assert_eq!((o.max_angle, o.max_distortion), (40.0, 90.0));
}

#[test]
fn heavy_shapes_are_worked_out_on_another_thread() {
    let small = triangles_of(&PrimitiveKind::Sphere.standard().mesh());
    assert!(quadify().work(&small, Purpose::Viewport) < MODIFIERS_IN_BACKGROUND_FROM);
    let big = triangles_of(
        &mesh::Primitive::Sphere(mesh::primitives::SphereParams {
            radius: 50.0,
            sides: 128,
            rings: 64,
        })
        .mesh(),
    );
    assert!(quadify().work(&big, Purpose::Viewport) >= MODIFIERS_IN_BACKGROUND_FROM);
    // Quads only: nothing to do.
    let cube = PrimitiveKind::Cube.standard().mesh();
    assert_eq!(quadify().work(&cube, Purpose::Viewport), 0);
}

fn quadify_edit(faces: Vec<FaceId>, hardness: Hardness) -> Operation {
    Operation::Quadify {
        faces,
        options: QuadifyOptions::default(),
        hardness,
        anchors: None,
    }
}

#[test]
fn the_one_off_edit_joins_the_whole_shape_and_undo_takes_it_back() {
    let mut doc = Document::new();
    let id = triangulated(&mut doc, PrimitiveKind::Cube);
    doc.execute(Command::Record {
        id,
        operation: quadify_edit(Vec::new(), Hardness::Marked),
    })
    .unwrap();
    assert_eq!(doc.undo_label(), Some("Quadify"));
    let shape = doc.scene().evaluated(id).unwrap().mesh;
    assert_eq!((quads(&shape), shape.face_count()), (6, 6));
    doc.undo();
    assert_eq!(doc.scene().evaluated(id).unwrap().mesh.face_count(), 12);
}

#[test]
fn the_one_off_edit_on_chosen_polygons_joins_only_those() {
    let mut doc = Document::new();
    let id = triangulated(&mut doc, PrimitiveKind::Cube);
    let shape = doc.scene().evaluated(id).unwrap().mesh;
    let pairs = mesh::quadify::pairs(&shape, None, &QuadifyOptions::default(), Hardness::Marked);
    let chosen: Vec<FaceId> = pairs[..2].iter().flatten().copied().collect();
    let operation = quadify_edit(chosen, Hardness::Marked);
    assert_eq!(operation.describe(), "Quadify 4 polygons");
    doc.execute(Command::Record { id, operation }).unwrap();
    let shape = doc.scene().evaluated(id).unwrap().mesh;
    assert_eq!((quads(&shape), shape.face_count()), (2, 10));
}

#[test]
fn the_edit_s_settings_can_still_be_changed() {
    let operation = quadify_edit(Vec::new(), Hardness::Marked);
    let settings = operation.settings().unwrap();
    assert_eq!(settings.quadify, Some(QuadifyOptions::default()));
    let changed = operation.with(EditSettings {
        quadify: Some(QuadifyOptions {
            max_angle: 500.0,
            keep_seams: false,
            ..QuadifyOptions::default()
        }),
        ..settings
    });
    let Operation::Quadify { options, .. } = changed else {
        panic!("quadify")
    };
    assert_eq!(options.max_angle, 180.0);
    assert!(!options.keep_seams);
    assert!(!operation.mends_uvs());
    // Settings that aren't numbers are refused, as they couldn't be saved.
    let bad = Operation::Quadify {
        faces: Vec::new(),
        options: QuadifyOptions {
            max_angle: f32::NAN,
            ..QuadifyOptions::default()
        },
        hardness: Hardness::Marked,
        anchors: None,
    };
    assert!(!bad.numbers_are_real());
    assert!(!quadify_edit(Vec::new(), Hardness::ByAngle(f32::NAN)).numbers_are_real());
}

#[test]
fn the_edit_keeps_the_hard_edges_its_object_showed() {
    // A triangulated cube's diagonals are soft and its box edges hard:
    // by marks or by angle, six quads.
    for hardness in [Hardness::Marked, Hardness::ByAngle(30.0)] {
        let mut doc = Document::new();
        let id = triangulated(&mut doc, PrimitiveKind::Cube);
        doc.execute(Command::Record {
            id,
            operation: quadify_edit(Vec::new(), hardness),
        })
        .unwrap();
        assert_eq!(doc.scene().evaluated(id).unwrap().mesh.face_count(), 6);
    }
}

#[test]
fn with_symmetry_chosen_polygons_are_joined_on_both_sides() {
    let operation = quadify_edit(vec![FaceId::from_raw(0)], Hardness::Marked);
    assert!(crate::mirroring::mirrors(&operation));
    assert!(!crate::mirroring::mirrors(&quadify_edit(
        Vec::new(),
        Hardness::Marked
    )));
}
