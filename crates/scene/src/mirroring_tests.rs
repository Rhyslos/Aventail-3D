//! Tests for edits made with symmetry on: made on both sides, together or
//! as a mirrored copy, and the result symmetrical.

use crate::*;
use glam::Vec3;
use mesh::symmetry::Mirror;
use mesh::{CubeParams, Mesh};

/// A cube cut in two each way, so it has a left and a right half.
fn cube() -> Mesh {
    Primitive::Cube(CubeParams {
        segments: [2, 2, 2],
        ..Default::default()
    })
    .mesh()
}

const ACROSS_X: Mirror = Mirror {
    axes: [true, false, false],
    centre: Vec3::ZERO,
    clip: false,
};

/// The shape after `op` on the cube, worked out through a document.
fn after(op: Operation) -> Mesh {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Primitive(Primitive::Cube(
        CubeParams {
            segments: [2, 2, 2],
            ..Default::default()
        },
    ))))
    .unwrap();
    let id = doc.selection()[0];
    doc.execute(Command::Record { id, operation: op }).unwrap();
    let shape = doc.scene().evaluated(id).unwrap();
    assert!(shape.warnings.is_empty(), "{:?}", shape.warnings);
    (*shape.mesh).clone()
}

fn mirrored(op: Operation) -> Operation {
    Operation::Mirrored {
        op: Box::new(op),
        mirror: ACROSS_X,
    }
}

/// Whether every point has a mirror across X.
fn symmetrical(mesh: &Mesh) -> bool {
    mesh.vertices().all(|(_, p)| {
        let m = Vec3::new(-p.x, p.y, p.z);
        mesh.vertices().any(|(_, q)| q.distance(m) < 1e-2)
    })
}

/// The cube's polygon facing `way` on the right (+X) half, or the one
/// facing +X nearest the front-top.
fn right_face(mesh: &Mesh, way: Vec3) -> FaceId {
    mesh.faces()
        .map(|(f, _)| f)
        .filter(|f| mesh.face_normal(*f).dot(way) > 0.9)
        .filter(|f| {
            mesh.face(*f)
                .unwrap()
                .iter()
                .all(|v| mesh.position(*v).unwrap().x >= -1e-3)
        })
        .max_by(|a, b| {
            let mid = |f: &FaceId| {
                let c = mesh.face(*f).unwrap();
                c.iter().map(|v| mesh.position(*v).unwrap()).sum::<Vec3>() / c.len() as f32
            };
            (mid(a).z - mid(a).y).total_cmp(&(mid(b).z - mid(b).y))
        })
        .unwrap()
}

#[test]
fn an_extrude_is_made_on_both_sides_together() {
    let base = cube();
    let face = right_face(&base, Vec3::X);
    let op = Operation::ExtrudePolygons {
        faces: vec![face],
        distance: 30.0,
        grouping: Grouping::Together,
        shape: crate::ExtrudeShape::EARLIER,
        anchors: None,
    };
    let one = after(op.clone());
    assert!(!symmetrical(&one));
    let both = after(mirrored(op));
    assert!(symmetrical(&both));
    assert_eq!(both.face_count(), base.face_count() + 8, "four walls each");
    let bounds = both.bounds().unwrap();
    assert!((bounds.min.x + 80.0).abs() < 1e-3 && (bounds.max.x - 80.0).abs() < 1e-3);
}

#[test]
fn a_selection_crossing_the_middle_moves_as_one_piece() {
    // The two top polygons at the front, either side of the middle: their
    // mirrors are each other, so the extrude is made once, as a slab.
    let base = cube();
    let tops: Vec<FaceId> = base
        .faces()
        .map(|(f, _)| f)
        .filter(|f| base.face_normal(*f).z > 0.9)
        .filter(|f| {
            base.face(*f)
                .unwrap()
                .iter()
                .all(|v| base.position(*v).unwrap().y <= 1e-3)
        })
        .collect();
    assert_eq!(tops.len(), 2);
    let op = Operation::ExtrudePolygons {
        faces: vec![tops[0]],
        distance: 20.0,
        grouping: Grouping::Together,
        shape: crate::ExtrudeShape::EARLIER,
        anchors: None,
    };
    let both = after(mirrored(op));
    let by_hand = after(Operation::ExtrudePolygons {
        faces: tops.clone(),
        distance: 20.0,
        grouping: Grouping::Together,
        shape: crate::ExtrudeShape::EARLIER,
        anchors: None,
    });
    assert_eq!(both.face_count(), by_hand.face_count());
    assert!(symmetrical(&both));
}

#[test]
fn a_loop_cut_is_copied_unless_its_ring_crosses_the_middle() {
    let base = cube();
    let topology = base.topology();
    let edge_along = |axis: usize, x_min: f32| {
        topology
            .edges()
            .find(|e| {
                let (a, b) = e.ends();
                let (pa, pb) = (base.position(a).unwrap(), base.position(b).unwrap());
                let d = (pb - pa).abs();
                d[axis] > 1.0 && pa.x.min(pb.x) >= x_min - 1e-3
            })
            .unwrap()
    };
    // Along X on the right half: its ring goes round that half only; the
    // mirror's ring is the left half's. Two loops.
    let op = Operation::LoopCut {
        edge: edge_along(0, 0.0),
        cuts: 1,
        position: 0.3,
        anchors: None,
    };
    let one = after(op.clone());
    let both = after(mirrored(op));
    assert!(symmetrical(&both));
    assert_eq!(
        both.face_count() - base.face_count(),
        2 * (one.face_count() - base.face_count())
    );
    // Along Y: its ring crosses the middle and is its own mirror. One loop.
    let op = Operation::LoopCut {
        edge: edge_along(1, 0.0),
        cuts: 1,
        position: 0.5,
        anchors: None,
    };
    assert_eq!(after(mirrored(op.clone())), after(op));
}

#[test]
fn a_slit_weld_and_bent_extrude_are_mirrored_copies() {
    let base = cube();
    // A slit across the front's right half, seen from the front.
    let slit = Operation::Slit {
        line: SlitLine {
            points: vec![Vec3::new(20.0, -50.0, 120.0), Vec3::new(30.0, -50.0, -20.0)],
            eye: Eye::At(Vec3::new(20.0, -500.0, 50.0)),
            through: false,
        },
    };
    let both = after(mirrored(slit.clone()));
    assert!(symmetrical(&both));
    assert!(both.face_count() > after(slit).face_count());
    // Welding two right-hand points: their mirrors weld too.
    let near = |p: Vec3| {
        base.vertices()
            .min_by(|a, b| a.1.distance(p).total_cmp(&b.1.distance(p)))
            .unwrap()
            .0
    };
    let weld = Operation::Weld {
        points: vec![near(Vec3::new(50.0, -50.0, 100.0))],
        target: Some(near(Vec3::new(50.0, -50.0, 50.0))),
        anchors: None,
    };
    let both = after(mirrored(weld));
    assert!(symmetrical(&both));
    assert_eq!(both.vertex_count(), base.vertex_count() - 2);
    // A bend on the right side turns the mirrored way on the left.
    let bent = Operation::ExtrudeBent {
        faces: vec![right_face(&base, Vec3::X)],
        distance: 20.0,
        grouping: Grouping::Together,
        bend: Bend {
            angle: 60.0,
            direction: 70.0,
            radius: 20.0,
            segments: 3,
            ..Bend::default()
        },
        stretch: Vec3::ONE,
        anchors: None,
    };
    let both = after(mirrored(bent));
    assert!(symmetrical(&both));
    // A spiral on the right climbs the mirrored way on the left, so the
    // two are mirror images (one right-handed, one left-handed).
    let spiral = Operation::ExtrudeBent {
        faces: vec![right_face(&base, Vec3::X)],
        distance: 10.0,
        grouping: Grouping::Together,
        bend: Bend {
            angle: 720.0,
            direction: 70.0,
            radius: 40.0,
            segments: 6,
            rise: 10.0,
            mirrored: false,
        },
        stretch: Vec3::ONE,
        anchors: None,
    };
    let both = after(mirrored(spiral.clone()));
    assert!(symmetrical(&both));
    assert_eq!(
        both.vertex_count(),
        base.vertex_count() + 2 * 13 * 4,
        "twelve rows and one straight on, on each side"
    );
}

#[test]
fn a_bend_on_a_stretched_object_is_mirrored_as_seen() {
    // Stretched along X and sideways and up; the copy on the left turns
    // the mirrored way and keeps the same stretch.
    let base = cube();
    for stretch in [Vec3::new(1.5, 0.75, 0.75), Vec3::new(0.6, 1.2, 1.2)] {
        let bent = Operation::ExtrudeBent {
            faces: vec![right_face(&base, Vec3::X)],
            distance: 20.0,
            grouping: Grouping::Together,
            bend: Bend {
                angle: 80.0,
                direction: 40.0,
                radius: 25.0,
                segments: 4,
                ..Bend::default()
            },
            stretch,
            anchors: None,
        };
        let one = after(bent.clone());
        let both = after(mirrored(bent));
        assert!(symmetrical(&both), "{stretch}");
        assert_eq!(
            both.vertex_count() - base.vertex_count(),
            2 * (one.vertex_count() - base.vertex_count())
        );
    }
}

#[test]
fn delete_dissolve_flip_and_hard_edges_cover_both_sides() {
    let base = cube();
    let face = right_face(&base, Vec3::X);
    for op in [
        Operation::DeleteElements {
            points: Vec::new(),
            edges: Vec::new(),
            faces: vec![face],
            anchors: None,
        },
        Operation::FlipPolygons {
            faces: vec![face],
            anchors: None,
        },
        Operation::InsetPolygons {
            faces: vec![face],
            distance: 5.0,
            grouping: Grouping::Together,
            anchors: None,
        },
    ] {
        let both = after(mirrored(op.clone()));
        assert!(symmetrical(&both), "{}", op.describe());
        assert_ne!(both, after(op.clone()), "{}", op.describe());
    }
    let deleted = after(mirrored(Operation::DeleteElements {
        points: Vec::new(),
        edges: Vec::new(),
        faces: vec![face],
        anchors: None,
    }));
    assert_eq!(deleted.face_count(), base.face_count() - 2);
}

#[test]
fn settings_change_both_sides_and_the_name_says_so() {
    let base = cube();
    let op = mirrored(Operation::ExtrudePolygons {
        faces: vec![right_face(&base, Vec3::X)],
        distance: 30.0,
        grouping: Grouping::Together,
        shape: crate::ExtrudeShape::EARLIER,
        anchors: None,
    });
    assert_eq!(op.describe(), "Extrude 1 polygon, mirrored");
    assert_eq!(op.settings().unwrap().distance, Some(30.0));
    let longer = op.with(EditSettings {
        distance: Some(60.0),
        ..op.settings().unwrap()
    });
    assert!(matches!(longer, Operation::Mirrored { .. }));
    assert_eq!(longer.settings().unwrap().distance, Some(60.0));
    assert!(matches!(longer.inner(), Operation::ExtrudePolygons { .. }));
    let shape = after(longer);
    let bounds = shape.bounds().unwrap();
    assert!((bounds.max.x - 110.0).abs() < 1e-3 && (bounds.min.x + 110.0).abs() < 1e-3);
}

#[test]
fn a_shape_without_a_mirror_there_just_gets_the_edit() {
    // Lift a right-hand corner first, so the right side no longer mirrors
    // the left: the edit still happens on its own side.
    let base = cube();
    let face = right_face(&base, Vec3::X);
    let corner = base.face(face).unwrap()[0];
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Primitive(Primitive::Cube(
        CubeParams {
            segments: [2, 2, 2],
            ..Default::default()
        },
    ))))
    .unwrap();
    let id = doc.selection()[0];
    let opposite = Operation::FlipPolygons {
        faces: vec![face],
        anchors: None,
    };
    doc.execute(Command::Record {
        id,
        operation: Operation::MovePoints {
            points: vec![corner],
            offset: Vec3::new(13.0, 0.0, 0.0),
            anchors: None,
        },
    })
    .unwrap();
    doc.execute(Command::Record {
        id,
        operation: mirrored(opposite),
    })
    .unwrap();
    let shape = doc.scene().evaluated(id).unwrap();
    assert_eq!(shape.mesh.face_count(), base.face_count());
}

#[test]
fn whole_shape_edits_and_moves_are_not_mirrored() {
    use crate::mirroring::mirrors;
    assert!(!mirrors(&Operation::FixFacing));
    assert!(!mirrors(&Operation::HardEdgesByAngle { angle: 30.0 }));
    assert!(!mirrors(&Operation::FillHoles {
        points: Vec::new(),
        anchors: None
    }));
    assert!(mirrors(&Operation::FlipPolygons {
        faces: Vec::new(),
        anchors: None
    }));
}

#[test]
fn the_middle_is_welded_only_when_asked() {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Primitive(Primitive::Cube(
        CubeParams {
            segments: [2, 2, 2],
            ..Default::default()
        },
    ))))
    .unwrap();
    let id = doc.selection()[0];
    let base = cube();
    let middle = base
        .vertices()
        .find(|(_, p)| p.x.abs() < 1e-3 && p.z > 99.0 && p.y < -49.0)
        .unwrap()
        .0;
    // Nudge a middle point just off the middle.
    doc.execute(Command::Record {
        id,
        operation: Operation::MovePoints {
            points: vec![middle],
            offset: Vec3::new(0.05, 0.0, 0.0),
            anchors: None,
        },
    })
    .unwrap();
    let face = right_face(&base, Vec3::X);
    // A mirrored edit no longer welds anything by itself.
    doc.execute(Command::Record {
        id,
        operation: Operation::Mirrored {
            op: Box::new(Operation::FlipPolygons {
                faces: vec![face],
                anchors: None,
            }),
            mirror: ACROSS_X,
        },
    })
    .unwrap();
    let shape = doc.scene().evaluated(id).unwrap();
    assert!((shape.mesh.position(middle).unwrap().x - 0.05).abs() < 1e-5);
    // Weld the middle, when asked: back on the middle, as its own edit
    // that Ctrl + Z takes back.
    let steps = doc.undo_steps();
    doc.execute(Command::Record {
        id,
        operation: Operation::WeldMiddle {
            mirror: ACROSS_X,
            distance: 0.1,
        },
    })
    .unwrap();
    assert_eq!(doc.undo_steps(), steps + 1);
    let shape = doc.scene().evaluated(id).unwrap();
    assert_eq!(
        shape.mesh.position(middle).unwrap().x,
        0.0,
        "back on the middle"
    );
    assert_eq!(shape.mesh.vertex_count(), base.vertex_count());
    doc.undo();
    let shape = doc.scene().evaluated(id).unwrap();
    assert!((shape.mesh.position(middle).unwrap().x - 0.05).abs() < 1e-5);
}
