//! Tests for Weld by distance in the scene (decision 0152): the modifier,
//! and the one-off recorded edit.

use crate::*;
use mesh::weld_distance::WeldProblem;
use std::sync::Arc;

/// A cube 100 wide whose six sides each have their own four points, as a
/// shape split along every edge comes from a game file.
fn split_cube() -> Mesh {
    let corners = [
        [0.0, 0.0, 0.0],
        [100.0, 0.0, 0.0],
        [100.0, 100.0, 0.0],
        [0.0, 100.0, 0.0],
        [0.0, 0.0, 100.0],
        [100.0, 0.0, 100.0],
        [100.0, 100.0, 100.0],
        [0.0, 100.0, 100.0],
    ];
    let sides: [[usize; 4]; 6] = [
        [0, 3, 2, 1],
        [4, 5, 6, 7],
        [0, 1, 5, 4],
        [1, 2, 6, 5],
        [2, 3, 7, 6],
        [3, 0, 4, 7],
    ];
    let mut mesh = Mesh::new();
    for side in sides {
        let ids: Vec<VertexId> = side
            .iter()
            .map(|c| mesh.add_vertex(glam::Vec3::from(corners[*c])).unwrap())
            .collect();
        mesh.add_face(&ids).unwrap();
    }
    mesh
}

fn create_mesh(doc: &mut Document, mesh: Mesh) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Mesh(Arc::new(mesh))))
        .unwrap();
    doc.selection()[0]
}

fn create(doc: &mut Document, kind: PrimitiveKind) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
        .unwrap();
    doc.selection()[0]
}

fn weld() -> ModifierKind {
    ModifierKind::WeldByDistance(WeldOptions::default())
}

fn add(doc: &mut Document, id: ObjectId, kind: ModifierKind) {
    doc.execute(Command::AddModifier {
        ids: vec![id],
        kind,
    })
    .unwrap();
}

fn closed(mesh: &Mesh) -> bool {
    mesh::MeshReport::new(mesh).is_closed()
}

#[test]
fn a_split_cube_shows_and_exports_closed() {
    let mut doc = Document::new();
    let id = create_mesh(&mut doc, split_cube());
    add(&mut doc, id, weld());
    let scene = doc.scene();
    assert_eq!(doc.undo_label(), Some("Add Weld by distance"));
    assert_eq!(scene.evaluated(id).unwrap().mesh.vertex_count(), 24);
    let shown = scene.shown_shape(id).unwrap();
    assert_eq!((shown.vertex_count(), shown.face_count()), (8, 6));
    assert!(closed(&shown));
    assert_eq!(scene.shown_shape(id), scene.exported_shape(id));
    assert!(scene.modifier_problems(id).is_empty());
    assert_eq!(weld().family(), ModifierFamily::CleanUp);
    assert!(!weld().differs_by_purpose());
    assert!(!weld().carries_custom_normals());
    assert!(weld().sources().is_empty());
}

#[test]
fn a_shape_with_nothing_close_says_why() {
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Cube);
    add(&mut doc, id, weld());
    let scene = doc.scene();
    assert_eq!(
        scene.modifier_problems(id),
        vec![(0, WeldProblem::NothingClose.to_string())]
    );
    assert_eq!(
        *scene.shown_shape(id).unwrap(),
        *scene.unmodified_shape(id).unwrap()
    );
    // Only at open borders, on a closed shape.
    let mut borders = doc.scene().modifiers(id)[0].clone();
    borders.kind = ModifierKind::WeldByDistance(WeldOptions {
        only_borders: true,
        ..WeldOptions::default()
    });
    doc.execute(Command::ChangeModifier {
        id,
        index: 0,
        modifier: borders,
    })
    .unwrap();
    assert_eq!(
        doc.scene().modifier_problems(id),
        vec![(0, WeldProblem::NoBorders.to_string())]
    );
}

#[test]
fn a_new_weld_goes_before_modifiers_that_read_how_polygons_join() {
    let mut doc = Document::new();
    let id = create_mesh(&mut doc, split_cube());
    add(
        &mut doc,
        id,
        ModifierKind::Remesh(RemeshOptions {
            voxel_size: 10.0,
            ..RemeshOptions::default()
        }),
    );
    add(
        &mut doc,
        id,
        ModifierKind::Subdivide(SubdivideOptions::default()),
    );
    add(&mut doc, id, weld());
    let kinds: Vec<&'static str> = doc
        .scene()
        .modifiers(id)
        .iter()
        .map(|m| m.kind.name())
        .collect();
    assert_eq!(kinds, vec!["Remesh", "Weld by distance", "Subdivide"]);
    assert!(weld().goes_before(&ModifierKind::Quadify(QuadifyOptions::default())));
    assert!(weld().goes_before(&ModifierKind::Decimate(DecimateOptions::default())));
    assert!(!weld().goes_before(&ModifierKind::Remesh(RemeshOptions::default())));
}

#[test]
fn smoothing_a_welded_cube_rounds_it_without_cracks() {
    let mut doc = Document::new();
    let id = create_mesh(&mut doc, split_cube());
    add(
        &mut doc,
        id,
        ModifierKind::Subdivide(SubdivideOptions::default()),
    );
    // Split, each side smooths on its own: six separate pieces.
    let cracked = doc.scene().shown_shape(id).unwrap();
    assert!(!closed(&cracked));
    add(&mut doc, id, weld());
    let smooth = doc.scene().shown_shape(id).unwrap();
    assert!(closed(&smooth));
}

#[test]
fn changing_the_distance_works_it_out_again_and_bake_makes_it_permanent() {
    let mut doc = Document::new();
    let id = create_mesh(&mut doc, split_cube());
    add(&mut doc, id, weld());
    let standard = doc.scene().shown_shape(id).unwrap();
    let mut huge = doc.scene().modifiers(id)[0].clone();
    huge.kind = ModifierKind::WeldByDistance(WeldOptions {
        distance: 1e9,
        ..WeldOptions::default()
    });
    doc.execute(Command::ChangeModifier {
        id,
        index: 0,
        modifier: huge,
    })
    .unwrap();
    assert_eq!(doc.undo_label(), Some("Change Weld by distance"));
    // Clamped to 1 m: the whole 1 m cube melts.
    let ModifierKind::WeldByDistance(o) = doc.scene().modifiers(id)[0].kind else {
        panic!("weld")
    };
    assert_eq!(o.distance, WeldOptions::MAX_DISTANCE);
    assert!(doc.scene().shown_shape(id).unwrap().face_count() < 6);
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
fn heavy_shapes_are_welded_on_another_thread() {
    let small = split_cube();
    assert!(weld().work(&small, Purpose::Viewport) < MODIFIERS_IN_BACKGROUND_FROM);
    let big = mesh::Primitive::Sphere(mesh::primitives::SphereParams {
        radius: 50.0,
        sides: 256,
        rings: 128,
    })
    .mesh();
    assert!(weld().work(&big, Purpose::Viewport) >= MODIFIERS_IN_BACKGROUND_FROM);
}

fn weld_edit(points: Vec<VertexId>) -> Operation {
    Operation::WeldByDistance {
        points,
        options: WeldOptions::default(),
        anchors: None,
    }
}

#[test]
fn the_one_off_edit_welds_the_whole_shape_and_undo_takes_it_back() {
    let mut doc = Document::new();
    let id = create_mesh(&mut doc, split_cube());
    doc.execute(Command::Record {
        id,
        operation: weld_edit(Vec::new()),
    })
    .unwrap();
    assert_eq!(doc.undo_label(), Some("Weld by distance"));
    let shape = doc.scene().evaluated(id).unwrap().mesh;
    assert_eq!(shape.vertex_count(), 8);
    assert!(closed(&shape));
    doc.undo();
    assert_eq!(doc.scene().evaluated(id).unwrap().mesh.vertex_count(), 24);
}

#[test]
fn the_one_off_edit_on_chosen_points_welds_only_those() {
    let mut doc = Document::new();
    let id = create_mesh(&mut doc, split_cube());
    let shape = doc.scene().evaluated(id).unwrap().mesh;
    // The three copies of the cube's first corner, and one other point.
    let origin: Vec<VertexId> = shape
        .vertices()
        .filter(|(_, p)| p.length() < 1e-3)
        .map(|(v, _)| v)
        .collect();
    assert_eq!(origin.len(), 3);
    let operation = weld_edit(origin);
    assert_eq!(operation.describe(), "Weld 3 points by distance");
    assert_eq!(
        weld_edit(vec![VertexId::from_raw(0)]).describe(),
        "Weld 1 point by distance"
    );
    doc.execute(Command::Record { id, operation }).unwrap();
    assert_eq!(doc.scene().evaluated(id).unwrap().mesh.vertex_count(), 22);
}

#[test]
fn the_edit_s_settings_can_still_be_changed() {
    let operation = weld_edit(Vec::new());
    let settings = operation.settings().unwrap();
    assert_eq!(settings.weld, Some(WeldOptions::default()));
    let changed = operation.with(EditSettings {
        weld: Some(WeldOptions {
            distance: -3.0,
            only_borders: true,
            ..WeldOptions::default()
        }),
        ..settings
    });
    let Operation::WeldByDistance { options, .. } = changed else {
        panic!("weld")
    };
    assert_eq!(options.distance, 0.0);
    assert!(options.only_borders);
    assert!(!operation.mends_uvs());
    // A distance that isn't a number is refused, as it couldn't be saved.
    let bad = Operation::WeldByDistance {
        points: Vec::new(),
        options: WeldOptions {
            distance: f32::NAN,
            ..WeldOptions::default()
        },
        anchors: None,
    };
    assert!(!bad.numbers_are_real());
    assert!(operation.numbers_are_real());
}

#[test]
fn with_symmetry_chosen_points_are_welded_on_both_sides() {
    let chosen = weld_edit(vec![VertexId::from_raw(0)]);
    assert!(crate::mirroring::mirrors(&chosen));
    assert!(!crate::mirroring::mirrors(&weld_edit(Vec::new())));
}
