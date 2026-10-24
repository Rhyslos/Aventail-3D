//! Tests for the Live bevel modifier and bevel weights in the scene
//! (decision 0149), and how it works with Weighted normals (decision
//! 0148), the order the game bevel will use.

use crate::*;
use glam::Vec3;
use mesh::custom_normals::has_custom_normals;
use mesh::live_bevel::LiveBevelProblem;

fn create(doc: &mut Document, kind: PrimitiveKind) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
        .unwrap();
    doc.selection()[0]
}

fn live_bevel() -> ModifierKind {
    ModifierKind::LiveBevel(LiveBevelOptions::default())
}

fn weighted() -> ModifierKind {
    ModifierKind::WeightedNormals(WeightedNormalsOptions::default())
}

fn add(doc: &mut Document, id: ObjectId, kind: ModifierKind) {
    doc.execute(Command::AddModifier {
        ids: vec![id],
        kind,
    })
    .unwrap();
}

/// A cube (standing on the ground, 100 cm across, smoothed by angle as
/// new shapes are) with a standard Live bevel.
fn bevelled_cube() -> (Document, ObjectId) {
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Cube);
    add(&mut doc, id, live_bevel());
    (doc, id)
}

/// The largest angle, in degrees, between a big polygon's corner normals
/// and its facing (the cube's six big faces), under `hardness`; and the
/// largest on the bevels (how round they shade).
fn leans(mesh: &Mesh, hardness: Hardness) -> (f32, f32) {
    let (_, normals) = mesh::corner_shading_with(mesh, hardness);
    let (starts, _) = mesh.face_layout();
    let mut big = 0.0f32;
    let mut bevel = 0.0f32;
    for (index, pair) in starts.windows(2).enumerate() {
        let face = FaceId::from_raw(index as u32);
        let facing = mesh.face_normal(face);
        let corners = mesh.face(face).unwrap();
        let p: Vec<Vec3> = corners.iter().map(|v| mesh.position(*v).unwrap()).collect();
        let wide = p.len() == 4 && p[0].distance(p[1]) > 50.0 && p[1].distance(p[2]) > 50.0;
        for normal in &normals[pair[0] as usize..pair[1] as usize] {
            let lean = normal.angle_between(facing).to_degrees();
            if wide {
                big = big.max(lean);
            } else {
                bevel = bevel.max(lean);
            }
        }
    }
    (big, bevel)
}

#[test]
fn a_cube_shows_and_exports_bevelled() {
    let (doc, id) = bevelled_cube();
    let scene = doc.scene();
    assert_eq!(doc.undo_label(), Some("Add Live bevel"));
    assert_eq!(scene.evaluated(id).unwrap().mesh.face_count(), 6);
    assert_eq!(scene.shown_shape(id).unwrap().face_count(), 54);
    assert_eq!(scene.shown_shape(id), scene.exported_shape(id));
    assert!(scene.modifier_problems(id).is_empty());
    assert_eq!(live_bevel().family(), ModifierFamily::Generate);
    assert!(!live_bevel().differs_by_purpose());
    assert!(!live_bevel().carries_custom_normals());
}

#[test]
fn the_bevel_follows_the_shape_as_it_is_modelled() {
    let (mut doc, id) = bevelled_cube();
    let before = doc.scene().shown_shape(id).unwrap();
    // Pull the cube's top up: the bevel follows, still 54 polygons.
    let shape = doc.scene().evaluated(id).unwrap().mesh;
    let top: Vec<VertexId> = shape
        .vertices()
        .filter(|(_, p)| p.z > 99.0)
        .map(|(v, _)| v)
        .collect();
    doc.execute(Command::Record {
        id,
        operation: Operation::MovePoints {
            points: top,
            offset: Vec3::new(0.0, 0.0, 50.0),
            anchors: None,
        },
    })
    .unwrap();
    let after = doc.scene().shown_shape(id).unwrap();
    assert_ne!(before, after);
    assert_eq!(after.face_count(), 54);
    let highest = after.vertices().map(|(_, p)| p.z).fold(0.0, f32::max);
    assert!((highest - 150.0).abs() < 1e-3);
    // Changing its settings works it out again.
    let mut wider = doc.scene().modifiers(id)[0].clone();
    wider.kind = ModifierKind::LiveBevel(LiveBevelOptions {
        segments: 1,
        ..LiveBevelOptions::default()
    });
    doc.execute(Command::ChangeModifier {
        id,
        index: 0,
        modifier: wider,
    })
    .unwrap();
    assert_eq!(
        doc.scene().shown_shape(id).unwrap().face_count(),
        6 + 12 + 8
    );
    assert_eq!(doc.undo_label(), Some("Change Live bevel"));
}

#[test]
fn it_goes_before_smoothing_and_weighted_normals_when_added() {
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Cube);
    add(&mut doc, id, ModifierKind::Decimate(Default::default()));
    add(
        &mut doc,
        id,
        ModifierKind::Subdivide(SubdivideOptions::default()),
    );
    add(&mut doc, id, weighted());
    add(&mut doc, id, live_bevel());
    let kinds: Vec<&str> = doc
        .scene()
        .modifiers(id)
        .iter()
        .map(|m| m.kind.name())
        .collect();
    assert_eq!(
        kinds,
        ["Decimate", "Live bevel", "Subdivide", "Weighted normals"]
    );
    // The person can still put it anywhere.
    doc.execute(Command::MoveModifier { id, from: 1, to: 3 })
        .unwrap();
    assert_eq!(doc.scene().modifiers(id)[3].kind.name(), "Live bevel");
    // Other kinds still go at the end.
    add(&mut doc, id, ModifierKind::Remesh(Default::default()));
    assert_eq!(doc.scene().modifiers(id)[4].kind.name(), "Remesh");
}

#[test]
fn with_weighted_normals_last_a_bevelled_cube_shades_with_flat_big_faces() {
    for hardness in [Hardness::ByAngle(30.0), Hardness::Marked] {
        let mut doc = Document::new();
        let id = create(&mut doc, PrimitiveKind::Cube);
        doc.execute(Command::SetHardness {
            ids: vec![id],
            hardness,
        })
        .unwrap();
        // Soften the cube's marked edges, as the game bevel will.
        let shape = doc.scene().evaluated(id).unwrap().mesh;
        let edges: Vec<EdgeKey> = shape.topology().edges().collect();
        doc.execute(Command::Record {
            id,
            operation: Operation::SetHardEdges {
                edges,
                hard: false,
                anchors: None,
            },
        })
        .unwrap();
        // Weighted normals first: the bevel then goes before it.
        add(&mut doc, id, weighted());
        add(&mut doc, id, live_bevel());
        let scene = doc.scene();
        assert_eq!(scene.modifiers(id)[0].kind.name(), "Live bevel");
        assert!(scene.modifier_problems(id).is_empty());
        for shape in [scene.shown_shape(id), scene.exported_shape(id)] {
            let shape = shape.unwrap();
            assert!(has_custom_normals(&shape), "{hardness:?}");
            let (big, bevel) = leans(&shape, hardness);
            assert!(big < 0.05, "{hardness:?}: big faces lean {big}°");
            // The bevels shade round: their corners lean towards the faces
            // beside them.
            assert!(bevel > 10.0, "{hardness:?}: bevels lean {bevel}°");
        }
        assert_eq!(
            scene.final_weighted_normals(id, Purpose::Export),
            Some(WeightedNormalsOptions::default())
        );
    }
}

#[test]
fn without_weighted_normals_the_big_faces_lean() {
    // The usual shading (all soft) leans the big faces towards the bevels:
    // why Weighted normals is wanted.
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Cube);
    doc.execute(Command::SetHardness {
        ids: vec![id],
        hardness: Hardness::Marked,
    })
    .unwrap();
    let shape = doc.scene().evaluated(id).unwrap().mesh;
    let edges: Vec<EdgeKey> = shape.topology().edges().collect();
    doc.execute(Command::Record {
        id,
        operation: Operation::SetHardEdges {
            edges,
            hard: false,
            anchors: None,
        },
    })
    .unwrap();
    add(&mut doc, id, live_bevel());
    let (big, _) = leans(&doc.scene().shown_shape(id).unwrap(), Hardness::Marked);
    assert!(big > 1.0, "{big}");
}

#[test]
fn bevel_weights_are_recorded_edits_and_choose_the_marked_edges() {
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Cube);
    add(
        &mut doc,
        id,
        ModifierKind::LiveBevel(LiveBevelOptions {
            edges: BevelEdges::Marked,
            ..LiveBevelOptions::default()
        }),
    );
    // None marked: the shape goes on unbevelled, and says why.
    assert_eq!(doc.scene().shown_shape(id).unwrap().face_count(), 6);
    let problems = doc.scene().modifier_problems(id);
    assert_eq!(problems.len(), 1);
    assert_eq!(problems[0].1, LiveBevelProblem::NothingMarked.to_string());
    let shape = doc.scene().evaluated(id).unwrap().mesh;
    let edges: Vec<EdgeKey> = shape.topology().edges().take(1).collect();
    doc.execute(Command::Record {
        id,
        operation: Operation::SetBevelWeights {
            edges: edges.clone(),
            weight: 1.0,
            anchors: None,
        },
    })
    .unwrap();
    assert_eq!(doc.undo_label(), Some("Bevel weight 1 edge"));
    let edited = doc.scene().evaluated(id).unwrap().mesh;
    assert_eq!(mesh::bevel_weight::bevel_weights(&edited).len(), 1);
    // One edge, two rows.
    assert_eq!(doc.scene().shown_shape(id).unwrap().face_count(), 8);
    assert!(doc.scene().modifier_problems(id).is_empty());
    doc.undo();
    assert_eq!(doc.scene().shown_shape(id).unwrap().face_count(), 6);
    doc.redo();
    doc.execute(Command::Record {
        id,
        operation: Operation::SetBevelWeights {
            edges,
            weight: 0.0,
            anchors: None,
        },
    })
    .unwrap();
    assert_eq!(doc.undo_label(), Some("Clear bevel weight 1 edge"));
    // Weights that aren't 0 to 1 are refused.
    for weight in [f32::NAN, 1.5, -0.5] {
        let result = doc.execute(Command::Record {
            id,
            operation: Operation::SetBevelWeights {
                edges: Vec::new(),
                weight,
                anchors: None,
            },
        });
        assert!(result.is_err(), "{weight}");
    }
}

#[test]
fn bevel_weights_are_set_on_both_sides_with_symmetry() {
    let op = Operation::SetBevelWeights {
        edges: Vec::new(),
        weight: 1.0,
        anchors: None,
    };
    assert!(crate::mirroring::mirrors(&op));
}

#[test]
fn hard_edges_follow_the_objects_smoothing() {
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Cube);
    add(
        &mut doc,
        id,
        ModifierKind::LiveBevel(LiveBevelOptions {
            edges: BevelEdges::Hard,
            segments: 1,
            ..LiveBevelOptions::default()
        }),
    );
    // By angle (30°): the cube's 12 edges.
    assert_eq!(doc.scene().shown_shape(id).unwrap().face_count(), 26);
    // By its marks, with four softened: eight.
    doc.execute(Command::SetHardness {
        ids: vec![id],
        hardness: Hardness::Marked,
    })
    .unwrap();
    assert_eq!(doc.scene().shown_shape(id).unwrap().face_count(), 26);
    let shape = doc.scene().evaluated(id).unwrap().mesh;
    let top: Vec<EdgeKey> = shape
        .topology()
        .edges()
        .filter(|e| {
            let (a, b) = e.ends();
            shape.position(a).unwrap().z > 99.0 && shape.position(b).unwrap().z > 99.0
        })
        .collect();
    doc.execute(Command::Record {
        id,
        operation: Operation::SetHardEdges {
            edges: top,
            hard: false,
            anchors: None,
        },
    })
    .unwrap();
    let shown = doc.scene().shown_shape(id).unwrap();
    assert!(shown.face_count() < 26 && shown.face_count() > 6);
}

#[test]
fn bake_into_shape_makes_the_bevel_permanent() {
    let (mut doc, id) = bevelled_cube();
    let expected = doc.scene().exported_shape(id).unwrap();
    doc.execute(Command::MakeEditable(vec![id])).unwrap();
    let object = doc.scene().object(id).unwrap();
    assert!(object.modifiers.is_empty());
    let ObjectKind::Mesh(baked) = &object.kind else {
        panic!("a mesh")
    };
    assert_eq!(**baked, *expected);
    doc.undo();
    assert_eq!(doc.scene().modifiers(id).len(), 1);
}

#[test]
fn a_flat_sheet_is_left_as_it_is_with_a_reason() {
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Plane);
    add(&mut doc, id, live_bevel());
    let scene = doc.scene();
    assert_eq!(scene.shown_shape(id), scene.unmodified_shape(id));
    assert_eq!(
        scene.modifier_problems(id)[0].1,
        LiveBevelProblem::NothingSharp.to_string()
    );
}

#[test]
fn settings_are_brought_within_limits() {
    let mut doc = Document::new();
    let id = create(&mut doc, PrimitiveKind::Cube);
    add(
        &mut doc,
        id,
        ModifierKind::LiveBevel(LiveBevelOptions {
            width: f32::NAN,
            segments: 900,
            angle: -4.0,
            profile: 3.0,
            ..LiveBevelOptions::default()
        }),
    );
    let ModifierKind::LiveBevel(o) = doc.scene().modifiers(id)[0].kind else {
        panic!("a live bevel")
    };
    assert_eq!(o.width, 2.0);
    assert_eq!(o.segments, mesh::bevel::MAX_SEGMENTS);
    assert_eq!((o.angle, o.profile), (0.0, 1.0));
}

#[test]
fn heavy_shapes_are_worked_out_on_another_thread() {
    let light = mesh::Primitive::Cube(Default::default()).mesh();
    assert!(live_bevel().work(&light, Purpose::Viewport) < MODIFIERS_IN_BACKGROUND_FROM);
    // A box cut into many quads: only its box edges are sharp.
    let boxy = mesh::Primitive::Cube(mesh::CubeParams {
        size: Vec3::splat(100.0),
        segments: [20, 20, 20],
    })
    .mesh();
    assert!(live_bevel().work(&boxy, Purpose::Viewport) < MODIFIERS_IN_BACKGROUND_FROM);
    // A rough ball: thousands of sharp edges.
    let heavy = mesh::samples::heavy_ball(20_000, 50.0);
    let fine = ModifierKind::LiveBevel(LiveBevelOptions {
        angle: 1.0,
        ..LiveBevelOptions::default()
    });
    assert!(fine.work(&heavy, Purpose::Viewport) >= MODIFIERS_IN_BACKGROUND_FROM);
}
