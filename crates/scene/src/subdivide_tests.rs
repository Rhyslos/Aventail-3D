//! Tests for the Subdivide modifier and creases (decision 0143).

use crate::*;
use glam::Vec3;

fn create(doc: &mut Document, kind: PrimitiveKind) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
        .unwrap();
    doc.selection()[0]
}

fn subdivide(levels: u32, export_levels: u32, hard_edges_sharp: bool) -> ModifierKind {
    ModifierKind::Subdivide(SubdivideOptions {
        levels,
        export_levels,
        hard_edges_sharp,
    })
}

/// A cube (standing on the ground, 100 cm across) with a standard
/// Subdivide modifier.
fn smoothed_cube() -> (Document, ObjectId) {
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Cube);
    doc.execute(Command::AddModifier {
        ids: vec![id],
        kind: ModifierKind::Subdivide(SubdivideOptions::default()),
    })
    .unwrap();
    (doc, id)
}

/// Whether every point of `mesh` lies on the cube's surface.
fn stays_a_cube(mesh: &Mesh) -> bool {
    mesh.vertices().all(|(_, p)| {
        let q = p - Vec3::new(0.0, 0.0, 50.0);
        (q.abs().max_element() - 50.0).abs() < 1e-3
    })
}

#[test]
fn the_viewport_and_an_export_have_their_own_levels() {
    let (doc, id) = smoothed_cube();
    let scene = doc.scene();
    assert_eq!(scene.shown_shape(id).unwrap().face_count(), 24);
    assert_eq!(scene.exported_shape(id).unwrap().face_count(), 96);
    assert_eq!(scene.evaluated(id).unwrap().mesh.face_count(), 6);
    assert!(scene.modifier_problems(id).is_empty());
    assert_eq!(doc.undo_label(), Some("Add Subdivide"));
}

#[test]
fn a_cube_smoothed_by_angle_rounds_off() {
    // New shapes are smoothed by angle: their hard marks don't show, so
    // they don't keep the cube's edges sharp.
    let (doc, id) = smoothed_cube();
    assert!(matches!(
        doc.scene().object(id).unwrap().hardness,
        Hardness::ByAngle(_)
    ));
    assert!(!stays_a_cube(&doc.scene().exported_shape(id).unwrap()));
}

#[test]
fn marked_hard_edges_stay_sharp_unless_switched_off() {
    let (mut doc, id) = smoothed_cube();
    doc.execute(Command::SetHardness {
        ids: vec![id],
        hardness: Hardness::Marked,
    })
    .unwrap();
    // The smoothing changed, so the modifier is worked out again.
    assert!(stays_a_cube(&doc.scene().exported_shape(id).unwrap()));
    let mut off = doc.scene().modifiers(id)[0].clone();
    off.kind = subdivide(1, 2, false);
    doc.execute(Command::ChangeModifier {
        id,
        index: 0,
        modifier: off,
    })
    .unwrap();
    assert!(!stays_a_cube(&doc.scene().exported_shape(id).unwrap()));
}

#[test]
fn creases_are_recorded_edits_and_keep_edges_sharp() {
    let (mut doc, id) = smoothed_cube();
    let shape = doc.scene().evaluated(id).unwrap().mesh;
    let edges: Vec<EdgeKey> = shape.topology().edges().collect();
    doc.execute(Command::Record {
        id,
        operation: Operation::SetCreases {
            edges: edges.clone(),
            crease: 1.0,
            anchors: None,
        },
    })
    .unwrap();
    assert_eq!(doc.undo_label(), Some("Crease 12 edges"));
    let edited = doc.scene().evaluated(id).unwrap().mesh;
    assert_eq!(mesh::crease::creases(&edited).len(), 12);
    assert!(stays_a_cube(&doc.scene().exported_shape(id).unwrap()));
    doc.undo();
    assert!(!stays_a_cube(&doc.scene().exported_shape(id).unwrap()));
    doc.redo();
    // Uncreasing is an edit too.
    doc.execute(Command::Record {
        id,
        operation: Operation::SetCreases {
            edges,
            crease: 0.0,
            anchors: None,
        },
    })
    .unwrap();
    assert_eq!(doc.undo_label(), Some("Uncrease 12 edges"));
    assert!(mesh::crease::creases(&doc.scene().evaluated(id).unwrap().mesh).is_empty());
}

#[test]
fn a_crease_that_isnt_a_real_number_is_refused() {
    let (mut doc, id) = smoothed_cube();
    for crease in [f32::NAN, 2.0, -1.0] {
        let result = doc.execute(Command::Record {
            id,
            operation: Operation::SetCreases {
                edges: Vec::new(),
                crease,
                anchors: None,
            },
        });
        assert!(result.is_err(), "{crease}");
    }
}

#[test]
fn zero_levels_show_the_shape_as_it_is() {
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Cube);
    doc.execute(Command::AddModifier {
        ids: vec![id],
        kind: subdivide(0, 1, true),
    })
    .unwrap();
    let scene = doc.scene();
    assert_eq!(scene.shown_shape(id).unwrap().face_count(), 6);
    assert_eq!(scene.exported_shape(id).unwrap().face_count(), 24);
}

#[test]
fn levels_are_brought_within_limits() {
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Cube);
    doc.execute(Command::AddModifier {
        ids: vec![id],
        kind: subdivide(50, u32::MAX, true),
    })
    .unwrap();
    assert_eq!(
        doc.scene().modifiers(id)[0].kind,
        subdivide(
            SubdivideOptions::MAX_LEVELS,
            SubdivideOptions::MAX_LEVELS,
            true
        )
    );
}

#[test]
fn too_many_polygons_pass_the_shape_on_with_a_reason() {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Primitive(Primitive::Sphere(
        mesh::SphereParams {
            sides: 128,
            rings: 64,
            ..Default::default()
        },
    ))))
    .unwrap();
    let id = doc.selection()[0];
    doc.execute(Command::AddModifier {
        ids: vec![id],
        kind: subdivide(1, 5, true),
    })
    .unwrap();
    let scene = doc.scene();
    let input = scene.unmodified_shape(id).unwrap();
    assert!(mesh::subdivide::polygon_count(&input, 5) > SubdivideOptions::MAX_POLYGONS);
    let result = scene.final_result(id, Purpose::Export).unwrap();
    assert_eq!(result.mesh, input);
    assert_eq!(result.problems.len(), 1);
    assert!(result.problems[0].1.contains("Lower the levels"));
    // The viewport's one level is fine, but the panel still says why the
    // export's can't be made.
    assert!(scene.shown_shape(id).unwrap().face_count() > input.face_count());
    assert_eq!(scene.modifier_problems(id).len(), 1);
}

#[test]
fn bake_into_shape_uses_the_export_levels() {
    let (mut doc, id) = smoothed_cube();
    let expected = doc.scene().exported_shape(id).unwrap();
    doc.execute(Command::MakeEditable(vec![id])).unwrap();
    let object = doc.scene().object(id).unwrap();
    assert!(object.modifiers.is_empty());
    let ObjectKind::Mesh(baked) = &object.kind else {
        panic!("a mesh")
    };
    assert_eq!(baked.face_count(), 96);
    assert_eq!(**baked, *expected);
}

#[test]
fn heavy_subdivision_is_worked_out_on_another_thread() {
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Sphere);
    let sphere = doc.scene().unmodified_shape(id).unwrap();
    let light = subdivide(1, 2, true);
    assert!(light.work(&sphere, Purpose::Viewport) < MODIFIERS_IN_BACKGROUND_FROM);
    assert_eq!(subdivide(0, 2, true).work(&sphere, Purpose::Viewport), 0);
    // 10 000 polygons at level 2 make 160 000: off the interface thread.
    let polygons = sphere.face_count();
    let heavy = 10_000 / polygons.max(1) + 1;
    assert!(
        subdivide(2, 2, true).work(&sphere, Purpose::Viewport) * heavy
            >= MODIFIERS_IN_BACKGROUND_FROM
    );
}

#[test]
fn the_live_mirror_is_smoothed_across_without_a_crease() {
    // A cube open on its -X side, mirrored across it: one smooth shape.
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Cube);
    let shape = doc.scene().evaluated(id).unwrap().mesh;
    let left = shape
        .faces()
        .map(|(f, _)| f)
        .find(|f| shape.face_normal(*f).x < -0.9)
        .unwrap();
    doc.execute(Command::Record {
        id,
        operation: Operation::DeleteElements {
            points: Vec::new(),
            edges: Vec::new(),
            faces: vec![left],
            anchors: None,
        },
    })
    .unwrap();
    doc.execute(Command::SetMirror {
        ids: vec![id],
        mirror: Some(LiveMirror {
            centre: Vec3::new(-50.0, 0.0, 0.0),
            ..LiveMirror::on_object_x()
        }),
    })
    .unwrap();
    doc.execute(Command::AddModifier {
        ids: vec![id],
        kind: subdivide(2, 2, true),
    })
    .unwrap();
    let shown = doc.scene().shown_shape(id).unwrap();
    let report = mesh::MeshReport::new(&shown);
    assert_eq!(report.border_edges, 0, "{report:?}");
    // Rounded at the far ends, flat-free across the seam.
    let size = shown.bounds().unwrap().size();
    assert!(size.x < 200.0 - 1.0, "{size}");
}
