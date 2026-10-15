//! Tests for `.avtl` project files: everything saved comes back exactly,
//! damaged or hostile files are refused in plain words (never a crash),
//! and saving to disk never leaves a half-written project.

use super::*;
use crate::limits::Bounded;
use glam::Vec3;
use mesh::{CubeParams, PrimitiveKind, VertexId};
use scene::{Command, Object, ObjectKind, Operation, Place, Primitive, Transform};

/// A document using everything the current version of the format can
/// hold: all of [`rich_document_v15`], plus what version 16 adds (a
/// pivot moved and turned apart from its shape, a group's pivot moved
/// with an object inside, and the project in centimetres; the version
/// 16 sample holds exactly this).
fn rich_document() -> Document {
    let mut doc = rich_document_v15();
    let find = |doc: &Document, name: &str| {
        doc.scene()
            .walk()
            .into_iter()
            .map(|(id, _)| id)
            .find(|id| doc.scene().object(*id).unwrap().name == name)
    };
    let cube = find(&doc, "Cube").unwrap();
    let mut pivot = doc.scene().object(cube).unwrap().transform;
    pivot.position += Vec3::new(12.5, -3.0, 40.0);
    pivot.rotation.x += 0.75;
    doc.execute(Command::MovePivots(vec![(cube, pivot)]))
        .unwrap();
    // Every group with something inside has its pivot moved too.
    let groups: Vec<_> = doc
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .filter(|id| {
            matches!(doc.scene().object(*id).unwrap().kind, ObjectKind::Group)
                && !doc.scene().children(Some(*id)).is_empty()
        })
        .collect();
    for group in groups {
        let mut t = doc.scene().object(group).unwrap().transform;
        t.position.z -= 25.0;
        doc.execute(Command::MovePivots(vec![(group, t)])).unwrap();
    }
    doc.execute(Command::SetUnits(scene::Units::Centimetres))
        .unwrap();
    doc.set_selection([cube]);
    doc
}

/// A document using everything version 15 of the format can hold: all
/// of [`rich_document_v14`], plus the edits version 15 adds (a spiral, on
/// its own and mirrored; the version 15 sample holds exactly this).
fn rich_document_v15() -> Document {
    let mut doc = rich_document_v14();
    let cube = doc
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| doc.scene().object(*id).unwrap().name == "Cube")
        .unwrap();
    let shape = doc.scene().evaluated(cube).unwrap().mesh;
    let top = shape
        .faces()
        .map(|(f, _)| f)
        .max_by(|a, b| shape.face_normal(*a).z.total_cmp(&shape.face_normal(*b).z))
        .unwrap();
    let spiral = scene::Bend {
        angle: 900.0,
        direction: 30.0,
        radius: 40.0,
        segments: 6,
        rise: 7.5,
        mirrored: true,
    };
    let bent = Operation::ExtrudeBent {
        faces: vec![top],
        distance: 5.0,
        grouping: scene::Grouping::Together,
        bend: spiral,
        anchors: None,
    };
    for operation in [
        bent.clone(),
        Operation::Mirrored {
            op: Box::new(bent),
            mirror: mesh::symmetry::Mirror {
                axes: [true, false, true],
                centre: glam::Vec3::new(0.5, 0.0, -1.0),
                clip: false,
            },
        },
    ] {
        doc.execute(Command::Record {
            id: cube,
            operation,
        })
        .unwrap();
    }
    doc.set_selection([cube]);
    doc
}

/// A document using everything version 14 of the format can hold: all
/// of [`rich_document_v13`], plus the edits version 14 adds (the version
/// 14 sample holds exactly this).
fn rich_document_v14() -> Document {
    let mut doc = rich_document_v13();
    let cube = doc
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| doc.scene().object(*id).unwrap().name == "Cube")
        .unwrap();
    let shape = doc.scene().evaluated(cube).unwrap().mesh;
    let face = shape.faces().next().unwrap().0;
    doc.execute(Command::Record {
        id: cube,
        operation: Operation::Mirrored {
            op: Box::new(Operation::FlipPolygons {
                faces: vec![face],
                anchors: None,
            }),
            mirror: mesh::symmetry::Mirror {
                axes: [true, false, false],
                centre: glam::Vec3::new(1.5, 0.0, 0.0),
                clip: true,
            },
        },
    })
    .unwrap();
    doc.execute(Command::Record {
        id: cube,
        operation: Operation::WeldMiddle {
            mirror: mesh::symmetry::Mirror {
                axes: [false, true, false],
                centre: glam::Vec3::new(0.0, 2.0, 0.0),
                clip: false,
            },
            distance: 0.25,
        },
    })
    .unwrap();

    doc.set_selection([cube]);
    doc
}

/// A document using everything version 13 of the format can hold: all
/// of [`rich_document_v12`], plus the edits version 13 adds (the version
/// 13 sample holds exactly this).
fn rich_document_v13() -> Document {
    let mut doc = rich_document_v12();
    let cube = doc
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| doc.scene().object(*id).unwrap().name == "Cube")
        .unwrap();
    let shape = doc.scene().evaluated(cube).unwrap().mesh;
    let slides = mesh::edit::slides_for(
        &shape,
        &[shape.vertices().next().unwrap().0],
        glam::Vec3::new(0.0, 0.0, 20.0),
    );
    doc.execute(Command::Record {
        id: cube,
        operation: Operation::SlidePoints {
            slides,
            anchors: None,
        },
    })
    .unwrap();

    doc.set_selection([cube]);
    doc
}

/// A document using everything version 12 of the format can hold: all
/// of [`rich_document_v11`], plus the edits version 12 adds (the version
/// 12 sample holds exactly this).
fn rich_document_v12() -> Document {
    let mut doc = rich_document_v11();
    let cube = doc
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| doc.scene().object(*id).unwrap().name == "Cube")
        .unwrap();
    doc.execute(Command::Duplicate {
        ids: vec![cube],
        instances: true,
    })
    .unwrap();

    doc.set_selection([cube]);
    doc
}

/// A document using everything version 11 of the format can hold: all
/// of [`rich_document_v10`], plus the edits version 11 adds (the version
/// 11 sample holds exactly this).
fn rich_document_v11() -> Document {
    let mut doc = rich_document_v10();
    let cube = doc
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| doc.scene().object(*id).unwrap().name == "Cube")
        .unwrap();
    let shape = doc.scene().evaluated(cube).unwrap().mesh;
    let soft: Vec<mesh::EdgeKey> = shape.edges().iter().copied().take(3).collect();
    for operation in [
        Operation::SetHardEdges {
            edges: soft,
            hard: false,
            anchors: None,
        },
        Operation::HardEdgesByAngle { angle: 45.0 },
    ] {
        doc.execute(Command::Record {
            id: cube,
            operation,
        })
        .unwrap();
    }

    doc.set_selection([cube]);
    doc
}

/// A document using everything version 10 of the format can hold: all
/// of [`rich_document_v9`], plus the edits version 10 adds (the version
/// 10 sample holds exactly this).
fn rich_document_v10() -> Document {
    let mut doc = rich_document_v9();
    let cube = doc
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| doc.scene().object(*id).unwrap().name == "Cube")
        .unwrap();
    let shape = doc.scene().evaluated(cube).unwrap().mesh;
    let points: Vec<VertexId> = shape.vertices().take(2).map(|(v, _)| v).collect();
    let edges: Vec<mesh::EdgeKey> = shape.edges().iter().copied().take(2).collect();
    let faces: Vec<mesh::FaceId> = shape.faces().map(|(f, _)| f).take(2).collect();
    for operation in [
        Operation::DeleteElements {
            points: points.clone(),
            edges: edges.clone(),
            faces: faces.clone(),
            anchors: None,
        },
        Operation::FillHoles {
            points: points.clone(),
            anchors: None,
        },
        Operation::Dissolve {
            points: Vec::new(),
            edges,
            faces: Vec::new(),
            anchors: None,
        },
        Operation::FlipPolygons {
            faces,
            anchors: None,
        },
        Operation::FixFacing,
    ] {
        doc.execute(Command::Record {
            id: cube,
            operation,
        })
        .unwrap();
    }

    doc.set_selection([cube]);
    doc
}

/// A document using everything version 9 of the format can hold: all
/// of [`rich_document_v8`], plus the edits version 9 adds (the version
/// 9 sample holds exactly this).
fn rich_document_v9() -> Document {
    let mut doc = rich_document_v8();
    let cube = doc
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| doc.scene().object(*id).unwrap().name == "Cube")
        .unwrap();
    let shape = doc.scene().evaluated(cube).unwrap().mesh;
    let points: Vec<VertexId> = shape.vertices().take(3).map(|(v, _)| v).collect();
    let edges: Vec<mesh::EdgeKey> = shape.edges().iter().copied().take(2).collect();
    let faces: Vec<mesh::FaceId> = shape.faces().map(|(f, _)| f).take(2).collect();
    let places: Vec<Vec<VertexId>> = points.iter().map(|v| vec![*v]).collect();
    for operation in [
        Operation::Connect {
            points: points.clone(),
            edges: Vec::new(),
            cuts: 1,
            anchors: None,
        },
        Operation::Connect {
            points: Vec::new(),
            edges: edges.clone(),
            cuts: 3,
            anchors: None,
        },
        Operation::Bridge {
            edges,
            faces: Vec::new(),
            segments: 2,
            twist: -1,
            anchors: None,
        },
        Operation::Bridge {
            edges: Vec::new(),
            faces,
            segments: 1,
            twist: 0,
            anchors: None,
        },
        Operation::Weld {
            points: points.clone(),
            target: Some(points[0]),
            anchors: mesh::Anchors::record(&shape, mesh::ElementKind::Points, &places),
        },
    ] {
        doc.execute(Command::Record {
            id: cube,
            operation,
        })
        .unwrap();
    }

    doc.set_selection([cube]);
    doc
}

/// A document using everything version 8 of the format can hold: all of
/// [`rich_document_v7`], plus an extrude with a bend (the version 8 sample
/// holds exactly this).
fn rich_document_v8() -> Document {
    let mut doc = rich_document_v7();
    let cube = doc
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| doc.scene().object(*id).unwrap().name == "Cube")
        .unwrap();
    let shape = doc.scene().evaluated(cube).unwrap().mesh;
    let top = shape
        .faces()
        .map(|(f, _)| f)
        .max_by(|a, b| shape.face_normal(*a).z.total_cmp(&shape.face_normal(*b).z))
        .unwrap();
    doc.execute(Command::Record {
        id: cube,
        operation: Operation::ExtrudeBent {
            faces: vec![top],
            distance: 12.5,
            grouping: scene::Grouping::Separate,
            bend: scene::Bend {
                angle: 60.0,
                direction: 45.0,
                radius: 20.0,
                segments: 3,
                ..scene::Bend::default()
            },
            anchors: None,
        },
    })
    .unwrap();
    doc.set_selection([cube]);
    doc
}

/// A document using everything version 7 of the format can hold: all of
/// [`rich_document_v6`], plus a slit (the version 7 sample holds exactly
/// this).
fn rich_document_v7() -> Document {
    let mut doc = rich_document_v6();
    let cube = doc
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| doc.scene().object(*id).unwrap().name == "Cube")
        .unwrap();
    doc.execute(Command::Record {
        id: cube,
        operation: Operation::Slit {
            line: scene::SlitLine {
                points: vec![
                    Vec3::new(-80.0, -50.0, 20.0),
                    Vec3::new(5.0, -50.0, 60.0),
                    Vec3::new(80.0, -50.0, 25.0),
                ],
                eye: scene::Eye::At(Vec3::new(30.0, -600.0, 200.0)),
                through: true,
            },
        },
    })
    .unwrap();
    doc.set_selection([cube]);
    doc
}

/// A document using everything version 6 of the format can hold: all of
/// [`rich_document_v5`], plus a loop cut (with anchors). The version 6
/// sample holds exactly this.
fn rich_document_v6() -> Document {
    let mut doc = rich_document_v5();
    let cube = doc
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| doc.scene().object(*id).unwrap().name == "Cube")
        .unwrap();
    let shape = doc.scene().evaluated(cube).unwrap().mesh;
    let edge = *shape
        .edges()
        .iter()
        .find(|e| mesh::loopcut::ring_polygons(&shape, **e) > 1)
        .unwrap();
    let (a, b) = edge.ends();
    let anchors = mesh::Anchors::record(&shape, mesh::ElementKind::Edges, &[vec![a, b]]);
    doc.execute(Command::Record {
        id: cube,
        operation: Operation::LoopCut {
            edge,
            cuts: 2,
            position: 0.375,
            anchors,
        },
    })
    .unwrap();
    doc.set_selection([cube]);
    doc
}

/// A document using everything version 5 of the format can hold: all of
/// [`rich_document_v4`], plus a move that keeps the structure it was made
/// on, a switched-off edit, a pinned edit, and a plain mesh with attribute
/// layers of every domain and kind. The version 5 sample holds exactly
/// this.
fn rich_document_v5() -> Document {
    let mut doc = rich_document_v4();
    let cube = *doc.scene().children(None).last().unwrap();
    let shape = doc.scene().evaluated(cube).unwrap().mesh;
    let points: Vec<VertexId> = shape.vertices().take(3).map(|(v, _)| v).collect();
    let elements: Vec<Vec<VertexId>> = points.iter().map(|v| vec![*v]).collect();
    let anchors = mesh::Anchors::record(&shape, mesh::ElementKind::Points, &elements);
    assert!(anchors.as_ref().is_some_and(|a| a.structure.is_some()));
    doc.execute(Command::Record {
        id: cube,
        operation: Operation::MovePoints {
            points,
            offset: Vec3::new(1.0, 2.0, 3.0),
            anchors,
        },
    })
    .unwrap();
    doc.execute(Command::SetEditEnabled {
        id: cube,
        edit: scene::EditId::from_raw(2),
        enabled: false,
    })
    .unwrap();
    doc.execute(Command::SetEditPinned {
        id: cube,
        edit: scene::EditId::from_raw(4),
        pinned: true,
    })
    .unwrap();
    // A plain mesh with a layer of every domain and kind.
    let mut plane = PrimitiveKind::Plane.standard().mesh();
    let (v, f) = (
        plane.vertices().next().unwrap().0,
        plane.faces().next().unwrap().0,
    );
    let corners = plane.face(f).unwrap().to_vec();
    let edge = mesh::EdgeKey::new(corners[0], corners[1]);
    use mesh::{AttributeElement as At, Domain, Value};
    let layers = [
        (
            "colour",
            Domain::Point,
            Value::Vec4(glam::Vec4::ONE),
            At::Point(v),
            Value::Vec4(glam::Vec4::new(1.0, 0.5, 0.25, 1.0)),
        ),
        (
            "weight",
            Domain::Point,
            Value::Float(0.0),
            At::Point(v),
            Value::Float(0.75),
        ),
        (
            "seam",
            Domain::Edge,
            Value::Bool(false),
            At::Edge(edge),
            Value::Bool(true),
        ),
        (
            "crease",
            Domain::Edge,
            Value::Float(0.0),
            At::Edge(edge),
            Value::Float(0.5),
        ),
        (
            "material",
            Domain::Face,
            Value::Int(0),
            At::Face(f),
            Value::Int(3),
        ),
        (
            "uv",
            Domain::Corner,
            Value::Vec2(glam::Vec2::ZERO),
            At::Corner(f, 2),
            Value::Vec2(glam::Vec2::new(0.25, 0.5)),
        ),
        (
            "normal",
            Domain::Corner,
            Value::Vec3(Vec3::Z),
            At::Corner(f, 1),
            Value::Vec3(Vec3::X),
        ),
    ];
    for (name, domain, default, element, value) in layers {
        plane.add_attribute(name, domain, default).unwrap();
        plane.set_attribute(name, element, value).unwrap();
    }
    plane.set_hard(corners[1], corners[2], true);
    doc.execute(Command::Create(ObjectKind::Mesh(std::sync::Arc::new(
        plane,
    ))))
    .unwrap();
    doc.set_selection([cube]);
    doc
}

/// Anchors as files before version 5 kept them: without the structure the
/// edit was made on.
fn older(anchors: Option<mesh::Anchors>) -> Option<mesh::Anchors> {
    anchors.map(|mut a| {
        a.structure = None;
        a
    })
}

/// A document using everything version 4 of the format can hold: all of
/// [`rich_document_v3`], plus a bevel of edges (with anchors) and of a
/// point. The version 4 sample holds exactly this.
fn rich_document_v4() -> Document {
    let mut doc = rich_document_v3();
    let cube = *doc.scene().children(None).last().unwrap();
    let shape = doc.scene().evaluated(cube).unwrap().mesh;
    let edge = *shape
        .edges()
        .iter()
        .find(|e| shape.topology().edge_uses(**e).len() == 2)
        .unwrap();
    let (a, b) = edge.ends();
    let anchors = older(mesh::Anchors::record(
        &shape,
        mesh::ElementKind::Edges,
        &[vec![a, b]],
    ));
    for operation in [
        Operation::BevelEdges {
            edges: vec![edge],
            shape: scene::BevelShape {
                width: 2.5,
                segments: 3,
                profile: 0.75,
            },
            anchors,
        },
        Operation::BevelPoints {
            points: vec![VertexId::from_raw(5)],
            width: 1.5,
            anchors: None,
        },
    ] {
        doc.execute(Command::Record {
            id: cube,
            operation,
        })
        .unwrap();
    }
    doc
}

/// A document using everything version 3 of the format can hold: all of
/// [`rich_document_v2`], plus an extruded and an inset polygon (with
/// anchors) and an extruded border edge. The version 3 sample holds
/// exactly this.
fn rich_document_v3() -> Document {
    let mut doc = rich_document_v2();
    let cube = *doc.scene().children(None).last().unwrap();
    let shape = doc.scene().evaluated(cube).unwrap().mesh;
    let face = shape.faces().next().unwrap().0;
    let corners = vec![shape.face(face).unwrap().to_vec()];
    let anchors = older(mesh::Anchors::record(
        &shape,
        mesh::ElementKind::Polygons,
        &corners,
    ));
    for operation in [
        Operation::InsetPolygons {
            faces: vec![face],
            distance: 7.5,
            grouping: scene::Grouping::Separate,
            anchors: anchors.clone(),
        },
        Operation::ExtrudePolygons {
            faces: vec![face],
            distance: -12.25,
            grouping: scene::Grouping::Together,
            anchors,
        },
        Operation::ExtrudeEdges {
            edges: vec![mesh::EdgeKey::new(
                VertexId::from_raw(0),
                VertexId::from_raw(1),
            )],
            distance: 3.0,
            grouping: scene::Grouping::Together,
            anchors: None,
        },
    ] {
        doc.execute(Command::Record {
            id: cube,
            operation,
        })
        .unwrap();
    }
    doc
}

/// A document using everything version 2 of the format can hold: all of
/// [`rich_document_v1`], plus points turned and scaled with a matrix,
/// with anchors to find them again. The version 2 sample holds exactly
/// this.
fn rich_document_v2() -> Document {
    let mut doc = rich_document_v1();
    let cube = *doc.scene().children(None).last().unwrap();
    // Where the two points are, so the edit can find them again.
    let shape = doc.scene().evaluated(cube).unwrap().mesh;
    let points = [VertexId::from_raw(1), VertexId::from_raw(2)];
    let anchors = older(mesh::Anchors::record(
        &shape,
        mesh::ElementKind::Edges,
        &[points.to_vec()],
    ));
    assert!(anchors.is_some());
    doc.execute(Command::Record {
        id: cube,
        operation: Operation::TransformPoints {
            points: points.to_vec(),
            matrix: glam::Mat4::from_scale_rotation_translation(
                Vec3::new(1.5, 1.0, 0.5),
                glam::Quat::from_rotation_z(0.25),
                Vec3::new(3.0, -2.0, 1.0),
            ),
            anchors,
        },
    })
    .unwrap();
    doc
}

/// A document using everything version 1 of the format could hold: nested
/// groups, every kind of primitive with changed settings, a plain mesh,
/// recorded edits, a hidden object, a moved and turned object, and a
/// selection. The version 1 sample file holds exactly this.
fn rich_document_v1() -> Document {
    let mut doc = Document::new();
    for kind in PrimitiveKind::ALL {
        doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
            .unwrap();
    }
    let ids = doc.scene().children(None).to_vec();
    // Changed settings on the cube.
    let cube = *ids.last().unwrap();
    doc.execute(Command::SetPrimitive {
        ids: vec![cube],
        primitive: Primitive::Cube(CubeParams {
            size: Vec3::new(120.0, 80.0, 45.5),
            segments: [2, 3, 4],
        }),
    })
    .unwrap();
    // Recorded edits on the cube, one of them naming a missing point.
    doc.execute(Command::Record {
        id: cube,
        operation: Operation::MovePoints {
            points: vec![VertexId::from_raw(0), VertexId::from_raw(4000)],
            offset: Vec3::new(0.0, 0.0, 12.5),
            anchors: None,
        },
    })
    .unwrap();
    // The sphere becomes a plain mesh.
    let sphere = ids[3];
    doc.execute(Command::MakeEditable(vec![sphere])).unwrap();
    // Two groups, one inside the other, holding the plane.
    doc.execute(Command::Group(vec![ids[5]])).unwrap();
    let outer = doc.selection()[0];
    doc.execute(Command::Group(vec![outer])).unwrap();
    doc.execute(Command::Rename {
        id: outer,
        name: "Inner group – ÆØÅ".into(),
    })
    .unwrap();
    // A hidden object.
    doc.execute(Command::SetVisible {
        ids: vec![ids[0]],
        visible: false,
    })
    .unwrap();
    // An object placed, turned and scaled.
    let mut placed = Object::new(ObjectKind::Group);
    placed.name = "Placed".into();
    placed.transform = Transform {
        position: Vec3::new(10.0, -20.0, 30.0),
        rotation: Vec3::new(0.5, 0.25, -1.0),
        scale: Vec3::new(1.0, 2.0, 0.5),
    };
    doc.execute(Command::Insert {
        object: placed,
        place: Place {
            parent: None,
            index: 2,
        },
    })
    .unwrap();
    // A deleted object, so its id stays taken.
    doc.execute(Command::Delete(vec![ids[1]])).unwrap();
    doc.set_selection([cube, sphere]);
    doc
}

/// Checks two documents hold the same scene, shapes and selection.
fn assert_same(a: &Document, b: &Document) {
    assert_eq!(a.scene(), b.scene());
    assert_eq!(a.selection(), b.selection());
    assert_eq!(a.scene().next_object_id(), b.scene().next_object_id());
    for (id, _) in a.scene().walk() {
        assert_eq!(a.scene().evaluated(id), b.scene().evaluated(id), "{id}");
    }
}

// ----------------------------------------------------------- round trip ---

#[test]
fn everything_saved_comes_back_exactly() {
    let doc = rich_document();
    let bytes = to_bytes(&doc).unwrap();
    let opened = from_bytes(&bytes).unwrap();
    assert_same(&doc, &opened);
    assert_eq!(opened.undo_steps(), 0, "a fresh history");
}

#[test]
fn an_empty_scene_round_trips() {
    let doc = Document::new();
    let opened = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert!(opened.scene().is_empty());
}

#[test]
fn the_same_scene_saves_to_the_same_bytes() {
    let doc = rich_document();
    assert_eq!(to_bytes(&doc).unwrap(), to_bytes(&doc).unwrap());
    let copy = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_eq!(to_bytes(&copy).unwrap(), to_bytes(&doc).unwrap());
}

#[test]
fn the_header_says_what_the_file_is() {
    let bytes = to_bytes(&rich_document()).unwrap();
    assert_eq!(&bytes[..4], b"AVTL");
    assert_eq!(
        u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
        FORMAT_VERSION
    );
    assert_eq!(
        u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
        1,
        "zstd"
    );
    let length = u64::from_le_bytes(bytes[12..20].try_into().unwrap());
    assert_eq!(length as usize, bytes.len() - 20);
}

#[test]
fn compression_keeps_meshes_small() {
    let mut doc = Document::new();
    let sphere = mesh::SphereParams {
        sides: 128,
        rings: 64,
        ..Default::default()
    };
    doc.execute(Command::Create(ObjectKind::Primitive(Primitive::Sphere(
        sphere,
    ))))
    .unwrap();
    let id = doc.selection()[0];
    doc.execute(Command::MakeEditable(vec![id])).unwrap();
    let raw = postcard::to_stdvec(&v1::ProjectV1::from_document(&doc))
        .unwrap()
        .len();
    let saved = to_bytes(&doc).unwrap().len();
    // Positions are floating-point numbers, which pack less tightly than
    // text; a plain mesh still saves noticeably smaller.
    assert!(saved * 10 < raw * 7, "{saved} of {raw} bytes");
}

// ------------------------------------------------------ files refused ---

fn header(version: u32, flags: u32, length: u64) -> Vec<u8> {
    let mut bytes = b"AVTL".to_vec();
    bytes.extend_from_slice(&version.to_le_bytes());
    bytes.extend_from_slice(&flags.to_le_bytes());
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes
}

fn refused(bytes: &[u8]) -> ProjectError {
    match from_bytes(bytes) {
        Ok(_) => panic!("opened a broken file"),
        Err(e) => e,
    }
}

#[test]
fn other_files_are_not_projects() {
    for bytes in [
        &b""[..],
        b"AVT",
        b"PK\x03\x04 a zip file",
        b"avtl lower case",
    ] {
        assert!(
            matches!(refused(bytes), ProjectError::NotAProject),
            "{bytes:?}"
        );
    }
    assert_eq!(
        ProjectError::NotAProject.to_string(),
        "it isn't a project file"
    );
}

#[test]
fn newer_formats_are_refused_with_advice() {
    let error = refused(&header(FORMAT_VERSION + 1, 1, 0));
    assert!(matches!(error, ProjectError::TooNew { version } if version == FORMAT_VERSION + 1));
    let text = error.to_string();
    assert!(
        text.contains("newer version") && text.contains("Update"),
        "{text}"
    );
    // Flags this version doesn't know also mean a newer writer.
    assert!(matches!(
        refused(&header(1, 1 | 4, 0)),
        ProjectError::TooNew { .. }
    ));
}

#[test]
fn damaged_headers_are_explained() {
    assert!(matches!(
        refused(&header(0, 1, 0)),
        ProjectError::Damaged(_)
    ));
    let full = to_bytes(&Document::new()).unwrap();
    for cut in [5, 9, 13, 19] {
        let error = refused(&full[..cut]);
        assert!(error.to_string().contains("cut short"), "{cut}: {error}");
    }
    assert!(matches!(
        refused(&header(1, 1, u64::MAX)),
        ProjectError::TooLarge
    ));
}

#[test]
fn a_cut_short_or_padded_body_is_damaged() {
    let full = to_bytes(&rich_document()).unwrap();
    let error = refused(&full[..full.len() - 10]);
    assert!(error.to_string().contains("cut short"), "{error}");
    let mut padded = full.clone();
    padded.extend_from_slice(b"junk");
    assert!(matches!(refused(&padded), ProjectError::Damaged(_)));
}

#[test]
fn an_uncompressed_body_can_be_read() {
    let doc = rich_document();
    let body = postcard::to_stdvec(&v16::ProjectV16::from_document(&doc)).unwrap();
    let mut bytes = header(16, 0, body.len() as u64);
    bytes.extend_from_slice(&body);
    assert_same(&doc, &from_bytes(&bytes).unwrap());
    // Data after the scene is refused.
    let mut longer = body.clone();
    longer.push(0);
    let mut bytes = header(16, 0, longer.len() as u64);
    bytes.extend_from_slice(&longer);
    assert!(refused(&bytes).to_string().contains("unexpected data"));
}

/// The bytes of a version 1 file holding `model`, uncompressed.
fn file_of(model: &v1::ProjectV1) -> Vec<u8> {
    let body = postcard::to_stdvec(model).unwrap();
    let mut bytes = header(1, 0, body.len() as u64);
    bytes.extend_from_slice(&body);
    bytes
}

/// The bytes of a version 2 file holding `model`, uncompressed.
fn file_of_v2(model: &v2::ProjectV2) -> Vec<u8> {
    let body = postcard::to_stdvec(model).unwrap();
    let mut bytes = header(2, 0, body.len() as u64);
    bytes.extend_from_slice(&body);
    bytes
}

#[test]
fn turned_and_scaled_points_are_kept_and_checked() {
    let doc = rich_document_v2();
    let back = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_same(&doc, &back);
    let good = v2::ProjectV2::from_document(&doc);
    let cube = good.objects.iter().position(|o| o.name == "Cube").unwrap();
    // Read back exactly, as a version 2 file.
    assert_same(&doc, &from_bytes(&file_of_v2(&good)).unwrap());
    // A matrix that isn't numbers is refused, naming the object.
    let mut bad = good.clone();
    let ops = &mut bad.objects[cube].layers[0].operations;
    let last = ops.len() - 1;
    if let v2::OperationV2::TransformPoints { matrix, .. } = &mut ops[last] {
        matrix[4] = f32::NAN;
    }
    let text = refused(&file_of_v2(&bad)).to_string();
    assert!(
        text.contains("\"Cube\"") && text.contains("turn or scale"),
        "{text}"
    );
    // So is a recorded place that isn't numbers.
    let mut bad = good.clone();
    let ops = &mut bad.objects[cube].layers[0].operations;
    let v2::OperationV2::TransformPoints {
        anchors: Some(anchors),
        ..
    } = &mut ops[last]
    else {
        panic!("the rich document's turn records where its edge was")
    };
    anchors.points[0].1[2] = f32::INFINITY;
    let text = refused(&file_of_v2(&bad)).to_string();
    assert!(
        text.contains("\"Cube\"") && text.contains("recorded place"),
        "{text}"
    );
    // Version 1's model can't hold the new operation: it is only ever
    // used for reading.
    let old = v1::ProjectV1::from_document(&doc);
    assert_eq!(old.objects[cube].layers[0].operations.len(), 1);
}

#[test]
fn bad_values_inside_are_refused_naming_the_object() {
    let good = v1::ProjectV1::from_document(&rich_document());
    let cube = good.objects.iter().position(|o| o.name == "Cube").unwrap();

    let mut bad = good.clone();
    bad.objects[cube].transform.position[1] = f32::NAN;
    let text = refused(&file_of(&bad)).to_string();
    assert!(
        text.contains("\"Cube\"") && text.contains("position"),
        "{text}"
    );

    let mut bad = good.clone();
    let sphere = bad.objects.iter().position(|o| o.name == "Sphere").unwrap();
    if let v1::KindV1::Mesh(mesh) = &mut bad.objects[sphere].kind {
        mesh.faces[0] = Some(vec![0, 1, 99_999].into());
    }
    let text = refused(&file_of(&bad)).to_string();
    assert!(
        text.contains("\"Sphere\"") && text.contains("mesh"),
        "{text}"
    );

    let mut bad = good.clone();
    bad.objects[cube].layers[0].operations[0] = v1::OperationV1::MovePoints {
        points: vec![0].into(),
        offset: [0.0, f32::INFINITY, 0.0],
    };
    assert!(
        refused(&file_of(&bad))
            .to_string()
            .contains("recorded move")
    );

    let mut bad = good.clone();
    bad.objects[cube].parent = Some(424_242);
    assert!(matches!(refused(&file_of(&bad)), ProjectError::Damaged(_)));

    let mut bad = good.clone();
    bad.objects[cube].name = String::new();
    assert!(matches!(refused(&file_of(&bad)), ProjectError::Damaged(_)));
}

#[test]
fn settings_out_of_range_are_brought_within_limits() {
    let mut model = v1::ProjectV1::from_document(&Document::new());
    model.objects.push(v1::ObjectV1 {
        id: 0,
        parent: None,
        name: "Huge".into(),
        visible: true,
        transform: v1::TransformV1 {
            position: [0.0; 3],
            rotation: [0.0; 3],
            scale: [1.0; 3],
        },
        kind: v1::KindV1::Primitive(v1::PrimitiveV1::Sphere {
            radius: f32::NAN,
            sides: u32::MAX,
            rings: 0,
        }),
        layers: Vec::new().into(),
    });
    let doc = from_bytes(&file_of(&model)).unwrap();
    let id = doc.scene().children(None)[0];
    match &doc.scene().object(id).unwrap().kind {
        ObjectKind::Primitive(p) => assert_eq!(*p, p.clamped()),
        other => panic!("{other:?}"),
    }
}

// ----------------------------------------------------------------- fuzz ---

/// A small, repeatable random number source.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
}

#[test]
fn every_cut_of_a_file_is_refused_without_a_crash() {
    let full = to_bytes(&rich_document()).unwrap();
    for cut in 0..full.len() {
        assert!(from_bytes(&full[..cut]).is_err(), "cut at {cut}");
    }
}

#[test]
fn flipped_bytes_never_crash_the_reader() {
    let full = to_bytes(&rich_document()).unwrap();
    let raw = file_of(&v1::ProjectV1::from_document(&rich_document()));
    let mut rng = Lcg(7);
    for round in 0..3000 {
        let mut bytes = if round % 2 == 0 {
            full.clone()
        } else {
            raw.clone()
        };
        for _ in 0..(rng.next() % 4 + 1) {
            let at = (rng.next() as usize) % bytes.len();
            bytes[at] ^= 1 << (rng.next() % 8);
        }
        // Either it opens (the flip landed somewhere harmless) or it is
        // refused; it must never panic.
        let _ = from_bytes(&bytes);
    }
}

#[test]
fn random_bytes_after_a_valid_header_never_crash_the_reader() {
    let mut rng = Lcg(11);
    for _ in 0..2000 {
        let length = (rng.next() % 200) as usize;
        let body: Vec<u8> = (0..length).map(|_| rng.next() as u8).collect();
        for flags in [0, 1] {
            let mut bytes = header(1, flags, body.len() as u64);
            bytes.extend_from_slice(&body);
            // Refused, or by chance a valid (tiny) scene; never a crash.
            let _ = from_bytes(&bytes);
        }
    }
}

// ------------------------------------------------------------- on disk ---

/// A fresh folder for one test, removed when dropped.
struct Folder(PathBuf);

impl Folder {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("avtl-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn saving_and_opening_on_disk_keeps_a_backup_and_no_leftovers() {
    let folder = Folder::new("save");
    let path = folder.0.join("crate.avtl");
    let first = rich_document();
    save(&path, &first).unwrap();
    assert!(
        !backup_path(&path).exists(),
        "nothing to back up the first time"
    );
    assert_same(&first, &open(&path).unwrap());

    let mut second = first.clone();
    second.execute(Command::Create(ObjectKind::Group)).unwrap();
    save(&path, &second).unwrap();
    assert_same(&second, &open(&path).unwrap());
    assert_same(&first, &open(&backup_path(&path)).unwrap());
    let names: Vec<String> = std::fs::read_dir(&folder.0)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(!names.iter().any(|n| n.ends_with(".tmp")), "{names:?}");
    assert_eq!(names.len(), 2, "{names:?}");
}

#[test]
fn saving_where_it_cant_be_written_explains_and_leaves_nothing() {
    let folder = Folder::new("nowhere");
    let path = folder.0.join("missing folder").join("crate.avtl");
    let error = save(&path, &Document::new()).unwrap_err();
    assert!(matches!(
        error,
        ProjectError::Io {
            action: "write",
            ..
        }
    ));
    assert!(error.to_string().contains("crate.avtl"), "{error}");
    assert!(!path.exists());
}

#[test]
fn opening_a_missing_or_wrong_file_explains_why() {
    let folder = Folder::new("open");
    let missing = folder.0.join("nothing.avtl");
    let error = open(&missing).unwrap_err();
    assert!(matches!(error, ProjectError::Io { action: "read", .. }));
    let text_file = folder.0.join("notes.avtl");
    std::fs::write(&text_file, "just some text").unwrap();
    assert!(matches!(open(&text_file), Err(ProjectError::NotAProject)));
}

#[test]
fn a_failed_save_keeps_the_previous_file() {
    let folder = Folder::new("keep");
    let path = folder.0.join("crate.avtl");
    let first = rich_document();
    save(&path, &first).unwrap();
    // Block the backup's place with a folder that isn't empty, so saving
    // fails part way.
    std::fs::create_dir_all(folder.0.join("crate.avtl.bak").join("in the way")).unwrap();
    assert!(save(&path, &Document::new()).is_err());
    assert_same(&first, &open(&path).unwrap());
    let names: Vec<String> = std::fs::read_dir(&folder.0)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(!names.iter().any(|n| n.ends_with(".tmp")), "{names:?}");
}

// --------------------------------------------- files from older versions ---

/// A project saved by version 1 of the format, kept in the repository: it
/// must open in every future version of the app.
const SAMPLE_V1: &[u8] = include_bytes!("../../tests/data/sample-v1.avtl");

#[test]
fn a_file_saved_by_format_version_1_still_opens() {
    let doc = from_bytes(SAMPLE_V1).expect("the version 1 sample opens");
    let names: Vec<String> = doc
        .scene()
        .walk()
        .into_iter()
        .map(|(id, depth)| format!("{depth}:{}", doc.scene().object(id).unwrap().name))
        .collect();
    assert_eq!(
        names,
        [
            "0:Cone",
            "0:Placed",
            "0:Disc",
            "0:Sphere",
            "0:Cylinder",
            "0:Group 2",
            "1:Inner group – ÆØÅ",
            "2:Plane",
            "0:Cube",
        ]
    );
    let cube = *doc.scene().children(None).last().unwrap();
    assert_eq!(doc.scene().object(cube).unwrap().name, "Cube");
    assert_eq!(doc.scene().object(cube).unwrap().stack.operation_count(), 1);
    assert_eq!(doc.selection().len(), 2);
    assert_same(&rich_document_v1(), &doc);
}

/// A project saved by version 2 of the format, kept in the repository.
const SAMPLE_V2: &[u8] = include_bytes!("../../tests/data/sample-v2.avtl");

#[test]
fn a_file_saved_by_format_version_2_still_opens() {
    let doc = from_bytes(SAMPLE_V2).expect("the version 2 sample opens");
    let cube = *doc.scene().children(None).last().unwrap();
    assert_eq!(doc.scene().object(cube).unwrap().stack.operation_count(), 2);
    assert_same(&rich_document_v2(), &doc);
}

/// A project saved by version 3 of the format, kept in the repository.
const SAMPLE_V3: &[u8] = include_bytes!("../../tests/data/sample-v3.avtl");

#[test]
fn a_file_saved_by_format_version_3_still_opens() {
    let doc = from_bytes(SAMPLE_V3).expect("the version 3 sample opens");
    assert_same(&rich_document_v3(), &doc);
    let cube = *doc.scene().children(None).last().unwrap();
    assert_eq!(doc.scene().object(cube).unwrap().stack.operation_count(), 5);
}

/// The bytes of a version 3 file holding `model`, uncompressed.
fn file_of_v3(model: &v3::ProjectV3) -> Vec<u8> {
    let body = postcard::to_stdvec(model).unwrap();
    let mut bytes = header(3, 0, body.len() as u64);
    bytes.extend_from_slice(&body);
    bytes
}

#[test]
fn extrudes_and_insets_are_kept_and_checked() {
    let doc = rich_document_v3();
    assert_same(&doc, &from_bytes(&to_bytes(&doc).unwrap()).unwrap());
    let good = v3::ProjectV3::from_document(&doc);
    let cube = good.objects.iter().position(|o| o.name == "Cube").unwrap();
    assert_same(&doc, &from_bytes(&file_of_v3(&good)).unwrap());
    let ops = good.objects[cube].layers[0].operations.len();
    for (index, what) in [
        (ops - 3, "inset"),
        (ops - 2, "extrude"),
        (ops - 1, "extrude"),
    ] {
        let mut bad = good.clone();
        match &mut bad.objects[cube].layers[0].operations[index] {
            v3::OperationV3::InsetPolygons { distance, .. }
            | v3::OperationV3::ExtrudePolygons { distance, .. }
            | v3::OperationV3::ExtrudeEdges { distance, .. } => *distance = f32::NAN,
            v3::OperationV3::V2(_) => panic!("an extrude or inset"),
        }
        let text = refused(&file_of_v3(&bad)).to_string();
        assert!(text.contains("\"Cube\"") && text.contains(what), "{text}");
    }
    let mut bad = good.clone();
    if let v3::OperationV3::ExtrudeEdges { edges, .. } =
        &mut bad.objects[cube].layers[0].operations[ops - 1]
    {
        edges[0] = [4, 4];
    }
    let text = refused(&file_of_v3(&bad)).to_string();
    assert!(text.contains("to itself"), "{text}");
    // Version 2's model can't hold them: it is only used for reading.
    let old = v2::ProjectV2::from_document(&doc);
    assert_eq!(old.objects[cube].layers[0].operations.len(), ops - 3);
}

/// A project saved by version 4 of the format, kept in the repository.
const SAMPLE_V4: &[u8] = include_bytes!("../../tests/data/sample-v4.avtl");

#[test]
fn a_file_saved_by_format_version_4_still_opens() {
    let doc = from_bytes(SAMPLE_V4).expect("the version 4 sample opens");
    assert_same(&rich_document_v4(), &doc);
}

/// The bytes of a version 4 file holding `model`, uncompressed.
fn file_of_v4(model: &v4::ProjectV4) -> Vec<u8> {
    let body = postcard::to_stdvec(model).unwrap();
    let mut bytes = header(4, 0, body.len() as u64);
    bytes.extend_from_slice(&body);
    bytes
}

#[test]
fn bevels_are_kept_and_checked() {
    let doc = rich_document_v4();
    assert_same(&doc, &from_bytes(&to_bytes(&doc).unwrap()).unwrap());
    let good = v4::ProjectV4::from_document(&doc);
    let cube = good.objects.iter().position(|o| o.name == "Cube").unwrap();
    assert_same(&doc, &from_bytes(&file_of_v4(&good)).unwrap());
    let ops = good.objects[cube].layers[0].operations.len();
    // A width that isn't a number is refused, naming the object.
    for index in [ops - 2, ops - 1] {
        let mut bad = good.clone();
        match &mut bad.objects[cube].layers[0].operations[index] {
            v4::OperationV4::BevelEdges { width, .. }
            | v4::OperationV4::BevelPoints { width, .. } => *width = f32::NAN,
            v4::OperationV4::V3(_) => panic!("a bevel"),
        }
        let text = refused(&file_of_v4(&bad)).to_string();
        assert!(
            text.contains("\"Cube\"") && text.contains("width"),
            "{text}"
        );
    }
    // So is a profile that isn't a number, and an edge joining a point to
    // itself; rows out of range are brought within limits.
    let mut bad = good.clone();
    if let v4::OperationV4::BevelEdges { profile, .. } =
        &mut bad.objects[cube].layers[0].operations[ops - 2]
    {
        *profile = f32::INFINITY;
    }
    assert!(refused(&file_of_v4(&bad)).to_string().contains("profile"));
    let mut bad = good.clone();
    if let v4::OperationV4::BevelEdges { edges, .. } =
        &mut bad.objects[cube].layers[0].operations[ops - 2]
    {
        edges[0] = [3, 3];
    }
    assert!(refused(&file_of_v4(&bad)).to_string().contains("to itself"));
    let mut big = good.clone();
    if let v4::OperationV4::BevelEdges { segments, .. } =
        &mut big.objects[cube].layers[0].operations[ops - 2]
    {
        *segments = 100_000;
    }
    let opened = from_bytes(&file_of_v4(&big)).unwrap();
    let id = *opened.scene().children(None).last().unwrap();
    let ops_now = &opened.scene().object(id).unwrap().stack.layers()[0].entries;
    let Operation::BevelEdges { shape, .. } = &ops_now[ops - 2].operation else {
        panic!("a bevel");
    };
    assert_eq!(shape.segments, mesh::bevel::MAX_SEGMENTS);
    // Hollow profiles (below 0) are kept; beyond -100% is brought to it.
    for (stored, opened_as) in [(-0.5, -0.5), (-1.0, -1.0), (-7.0, -1.0), (3.0, 1.0)] {
        let mut file = good.clone();
        if let v4::OperationV4::BevelEdges { profile, .. } =
            &mut file.objects[cube].layers[0].operations[ops - 2]
        {
            *profile = stored;
        }
        let opened = from_bytes(&file_of_v4(&file)).unwrap();
        let id = *opened.scene().children(None).last().unwrap();
        let ops_now = &opened.scene().object(id).unwrap().stack.layers()[0].entries;
        let Operation::BevelEdges { shape, .. } = &ops_now[ops - 2].operation else {
            panic!("a bevel");
        };
        assert_eq!(shape.profile, opened_as, "stored {stored}");
    }
}

/// A project saved by version 5 of the format, kept in the repository.
const SAMPLE_V5: &[u8] = include_bytes!("../../tests/data/sample-v5.avtl");

#[test]
fn a_file_saved_by_format_version_5_still_opens() {
    let doc = from_bytes(SAMPLE_V5).expect("the version 5 sample opens");
    assert_same(&rich_document_v5(), &doc);
}

/// The bytes of a version 5 file holding `model`, uncompressed.
fn file_of_v5(model: &v5::ProjectV5) -> Vec<u8> {
    let body = postcard::to_stdvec(model).unwrap();
    let mut bytes = header(5, 0, body.len() as u64);
    bytes.extend_from_slice(&body);
    bytes
}

/// The bytes of a version 6 file holding `model`, uncompressed.
fn file_of_v6(model: &v6::ProjectV6) -> Vec<u8> {
    let body = postcard::to_stdvec(model).unwrap();
    let mut bytes = header(6, 0, body.len() as u64);
    bytes.extend_from_slice(&body);
    bytes
}

#[test]
fn edits_keep_their_ids_switches_and_structure() {
    let doc = rich_document();
    let opened = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    let cube = opened
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| opened.scene().object(*id).unwrap().name == "Cube")
        .unwrap();
    let entries = &opened.scene().object(cube).unwrap().stack.layers()[0].entries;
    let ids: Vec<u64> = entries.iter().map(|e| e.id.get()).collect();
    assert_eq!(ids, (1..=entries.len() as u64).collect::<Vec<_>>());
    assert!(!entries[1].enabled, "edit 2 is off");
    assert!(entries[3].pinned, "edit 4 is pinned");
    let last = entries
        .iter()
        .rev()
        .find_map(|e| e.operation.anchors())
        .unwrap();
    assert!(last.structure.is_some());
    // Edits from older files are numbered in order, on, and unpinned.
    let old = from_bytes(SAMPLE_V4).unwrap();
    let cube = *old.scene().children(None).last().unwrap();
    let entries = &old.scene().object(cube).unwrap().stack.layers()[0].entries;
    assert!(entries.iter().all(|e| e.enabled && !e.pinned));
    assert_eq!(entries[2].id.get(), 3);
}

#[test]
fn damaged_edits_and_mesh_layers_are_refused() {
    let good = v5::ProjectV5::from_document(&rich_document_v5());
    assert_same(
        &rich_document_v5(),
        &from_bytes(&file_of_v5(&good)).unwrap(),
    );
    let cube = good
        .project
        .objects
        .iter()
        .position(|o| o.name == "Cube")
        .unwrap();
    let damaged = |change: &dyn Fn(&mut v5::ProjectV5)| {
        let mut bad = good.clone();
        change(&mut bad);
        refused(&file_of_v5(&bad)).to_string()
    };
    // Two edits with one id.
    let text = damaged(&|p| p.project.objects[cube].layers[0].operations[1].id = 1);
    assert!(text.contains("same id"), "{text}");
    // Mesh layers: too many values, not numbers, hard edges, for a
    // missing object, for an object without a mesh, or listed twice.
    fn layer(p: &mut v5::ProjectV5) -> &mut v5::LayerV5 {
        &mut p.layers[0].layers[0]
    }
    assert!(
        damaged(&|p| layer(p).values = v5::ValuesV5::Vec4(vec![[0.0; 4]; 999].into()))
            .contains("damaged")
    );
    assert!(
        damaged(&|p| layer(p).default = v5::ValueV5::Vec4([f32::NAN; 4])).contains("real number")
    );
    assert!(damaged(&|p| layer(p).name = mesh::attributes::HARD.into()).contains("already exists"));
    assert!(damaged(&|p| p.layers[0].object = 9999).contains("isn't there"));
    assert!(damaged(&|p| p.layers[0].object = 0).contains("no mesh"));
    assert!(
        damaged(&|p| {
            let again = p.layers[0].clone();
            p.layers.push(again);
        })
        .contains("twice")
    );
}

/// A project saved by version 6 of the format, kept in the repository.
const SAMPLE_V6: &[u8] = include_bytes!("../../tests/data/sample-v6.avtl");

#[test]
fn a_file_saved_by_format_version_6_still_opens() {
    let doc = from_bytes(SAMPLE_V6).expect("the version 6 sample opens");
    assert_same(&rich_document_v6(), &doc);
}

#[test]
fn loop_cuts_are_kept_and_checked() {
    let doc = rich_document_v6();
    assert_same(&doc, &from_bytes(&to_bytes(&doc).unwrap()).unwrap());
    let good = v6::ProjectV6::from_document(&doc);
    assert_same(&doc, &from_bytes(&file_of_v6(&good)).unwrap());
    let cube = good
        .project
        .objects
        .iter()
        .position(|o| o.name == "Cube")
        .unwrap();
    let with = |change: &dyn Fn(&mut [u32; 2], &mut u32, &mut f32)| {
        let mut file = good.clone();
        let entry = file.project.objects[cube].layers[0]
            .operations
            .last_mut()
            .unwrap();
        let v6::OperationV6::LoopCut {
            edge,
            cuts,
            position,
            ..
        } = &mut entry.operation
        else {
            panic!("a loop cut");
        };
        change(edge, cuts, position);
        from_bytes(&file_of_v6(&file))
    };
    // A position that isn't a number, or an edge from a point to itself.
    let text = with(&|_, _, p| *p = f32::NAN).unwrap_err().to_string();
    assert!(text.contains("real number"), "{text}");
    let text = with(&|e, _, _| e[1] = e[0]).unwrap_err().to_string();
    assert!(text.contains("itself"), "{text}");
    // Settings out of range are brought back in.
    let opened = with(&|_, c, p| {
        *c = u32::MAX;
        *p = 7.0;
    })
    .unwrap();
    let id = opened
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| opened.scene().object(*id).unwrap().name == "Cube")
        .unwrap();
    let entries = &opened.scene().object(id).unwrap().stack.layers()[0].entries;
    let Operation::LoopCut { cuts, position, .. } = entries.last().unwrap().operation else {
        panic!("a loop cut");
    };
    assert_eq!(cuts, mesh::loopcut::MAX_CUTS);
    assert_eq!(position, 1.0);
}

/// The bytes of a version 7 file holding `model`, uncompressed.
fn file_of_v7(model: &v7::ProjectV7) -> Vec<u8> {
    let body = postcard::to_stdvec(model).unwrap();
    let mut bytes = header(7, 0, body.len() as u64);
    bytes.extend_from_slice(&body);
    bytes
}

/// A project saved by version 7 of the format, kept in the repository.
const SAMPLE_V7: &[u8] = include_bytes!("../../tests/data/sample-v7.avtl");

#[test]
fn a_file_saved_by_format_version_7_still_opens() {
    let doc = from_bytes(SAMPLE_V7).expect("the version 7 sample opens");
    assert_same(&rich_document_v7(), &doc);
}

#[test]
fn slits_are_kept_and_checked() {
    let doc = rich_document_v7();
    assert_same(&doc, &from_bytes(&to_bytes(&doc).unwrap()).unwrap());
    let good = v7::ProjectV7::from_document(&doc);
    assert_same(&doc, &from_bytes(&file_of_v7(&good)).unwrap());
    let cube = good
        .project
        .objects
        .iter()
        .position(|o| o.name == "Cube")
        .unwrap();
    let with = |change: &dyn Fn(&mut Bounded<[f32; 3]>, &mut v7::EyeV7)| {
        let mut file = good.clone();
        let entry = file.project.objects[cube].layers[0]
            .operations
            .last_mut()
            .unwrap();
        let v7::OperationV7::Slit { points, eye, .. } = &mut entry.operation else {
            panic!("a slit");
        };
        change(points, eye);
        from_bytes(&file_of_v7(&file))
    };
    let refused_with = |change: &dyn Fn(&mut Bounded<[f32; 3]>, &mut v7::EyeV7), what: &str| {
        let text = with(change).unwrap_err().to_string();
        assert!(text.contains(what), "{text}");
    };
    refused_with(&|p, _| p[1][2] = f32::NAN, "real numbers");
    refused_with(
        &|_, e| *e = v7::EyeV7::At([f32::INFINITY, 0.0, 0.0]),
        "view",
    );
    refused_with(&|_, e| *e = v7::EyeV7::Along([0.0; 3]), "view");
    refused_with(&|p, _| p.0.truncate(1), "1 points");
    refused_with(
        &|p, _| p.0 = vec![[0.0; 3]; mesh::slit::MAX_LINE_POINTS + 1],
        "points",
    );
    // A flat view's slit is kept too.
    assert!(with(&|_, e| *e = v7::EyeV7::Along([0.0, 1.0, 0.0])).is_ok());
}

/// The bytes of a version 8 file holding `model`, uncompressed.
fn file_of_v8(model: &v8::ProjectV8) -> Vec<u8> {
    let body = postcard::to_stdvec(model).unwrap();
    let mut bytes = header(8, 0, body.len() as u64);
    bytes.extend_from_slice(&body);
    bytes
}

/// A project saved by version 8 of the format, kept in the repository.
const SAMPLE_V8: &[u8] = include_bytes!("../../tests/data/sample-v8.avtl");

#[test]
fn a_file_saved_by_format_version_8_still_opens() {
    let doc = from_bytes(SAMPLE_V8).expect("the version 8 sample opens");
    assert_same(&rich_document_v8(), &doc);
}

#[test]
fn bent_extrudes_are_kept_and_checked() {
    let doc = rich_document_v8();
    assert_same(&doc, &from_bytes(&to_bytes(&doc).unwrap()).unwrap());
    let good = v8::ProjectV8::from_document(&doc);
    assert_same(&doc, &from_bytes(&file_of_v8(&good)).unwrap());
    let cube = good
        .project
        .objects
        .iter()
        .position(|o| o.name == "Cube")
        .unwrap();
    let with = |change: &dyn Fn(&mut f32, &mut v8::BendV8)| {
        let mut file = good.clone();
        let entry = file.project.objects[cube].layers[0]
            .operations
            .last_mut()
            .unwrap();
        let v8::OperationV8::ExtrudeBent { distance, bend, .. } = &mut entry.operation else {
            panic!("a bent extrude");
        };
        change(distance, bend);
        from_bytes(&file_of_v8(&file))
    };
    let text = with(&|_, b| b.angle = f32::NAN).unwrap_err().to_string();
    assert!(text.contains("real"), "{text}");
    let text = with(&|d, _| *d = f32::INFINITY).unwrap_err().to_string();
    assert!(text.contains("real"), "{text}");
    // Settings out of range are brought back in.
    let opened = with(&|_, b| {
        b.angle = 99_999.0;
        b.segments = 0;
    })
    .unwrap();
    let id = opened
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| opened.scene().object(*id).unwrap().name == "Cube")
        .unwrap();
    let entries = &opened.scene().object(id).unwrap().stack.layers()[0].entries;
    let Operation::ExtrudeBent { bend, .. } = entries.last().unwrap().operation else {
        panic!("a bent extrude");
    };
    assert_eq!(
        (bend.angle, bend.segments),
        (mesh::extrude::MAX_BEND_ANGLE, 1)
    );
}

/// The bytes of a version 9 file holding `model`, uncompressed.
fn file_of_v9(model: &v9::ProjectV9) -> Vec<u8> {
    let body = postcard::to_stdvec(model).unwrap();
    let mut bytes = header(9, 0, body.len() as u64);
    bytes.extend_from_slice(&body);
    bytes
}

/// A project saved by version 9 of the format, kept in the repository.
const SAMPLE_V9: &[u8] = include_bytes!("../../tests/data/sample-v9.avtl");

#[test]
fn a_file_saved_by_format_version_9_still_opens() {
    let doc = from_bytes(SAMPLE_V9).expect("the version 9 sample opens");
    assert_same(&rich_document_v9(), &doc);
    // And reads back the same through its own model.
    let model = v9::ProjectV9::from_document(&doc);
    assert_same(&doc, &from_bytes(&file_of_v9(&model)).unwrap());
}

/// The bytes of a version 10 file holding `model`, uncompressed.
fn file_of_v10(model: &v10::ProjectV10) -> Vec<u8> {
    let body = postcard::to_stdvec(model).unwrap();
    let mut bytes = header(10, 0, body.len() as u64);
    bytes.extend_from_slice(&body);
    bytes
}

/// A project saved by version 10 of the format, kept in the repository.
const SAMPLE_V10: &[u8] = include_bytes!("../../tests/data/sample-v10.avtl");

#[test]
fn a_file_saved_by_format_version_10_still_opens() {
    let doc = from_bytes(SAMPLE_V10).expect("the version 10 sample opens");
    assert_same(&rich_document_v10(), &doc);
    // And reads back the same through its own model.
    let model = v10::ProjectV10::from_document(&doc);
    assert_same(&doc, &from_bytes(&file_of_v10(&model)).unwrap());
}

/// The bytes of a version 11 file holding `model`, uncompressed.
fn file_of_v11(model: &v11::ProjectV11) -> Vec<u8> {
    let body = postcard::to_stdvec(model).unwrap();
    let mut bytes = header(11, 0, body.len() as u64);
    bytes.extend_from_slice(&body);
    bytes
}

/// A project saved by version 11 of the format, kept in the repository.
const SAMPLE_V11: &[u8] = include_bytes!("../../tests/data/sample-v11.avtl");

#[test]
fn a_file_saved_by_format_version_11_still_opens() {
    let doc = from_bytes(SAMPLE_V11).expect("the version 11 sample opens");
    assert_same(&rich_document_v11(), &doc);
    // And reads back the same through its own model.
    let model = v11::ProjectV11::from_document(&doc);
    assert_same(&doc, &from_bytes(&file_of_v11(&model)).unwrap());
}

/// The bytes of a version 12 file holding `model`, uncompressed.
fn file_of_v12(model: &v12::ProjectV12) -> Vec<u8> {
    let body = postcard::to_stdvec(model).unwrap();
    let mut bytes = header(12, 0, body.len() as u64);
    bytes.extend_from_slice(&body);
    bytes
}

/// A project saved by version 12 of the format, kept in the repository.
const SAMPLE_V12: &[u8] = include_bytes!("../../tests/data/sample-v12.avtl");

#[test]
fn a_file_saved_by_format_version_12_still_opens() {
    let doc = from_bytes(SAMPLE_V12).expect("the version 12 sample opens");
    assert_same(&rich_document_v12(), &doc);
    // And reads back the same through its own model.
    let model = v12::ProjectV12::from_document(&doc);
    assert_same(&doc, &from_bytes(&file_of_v12(&model)).unwrap());
}

/// The bytes of a version 13 file holding `model`, uncompressed.
fn file_of_v13(model: &v13::ProjectV13) -> Vec<u8> {
    let body = postcard::to_stdvec(model).unwrap();
    let mut bytes = header(13, 0, body.len() as u64);
    bytes.extend_from_slice(&body);
    bytes
}

/// A project saved by version 13 of the format, kept in the repository.
const SAMPLE_V13: &[u8] = include_bytes!("../../tests/data/sample-v13.avtl");

#[test]
fn a_file_saved_by_format_version_13_still_opens() {
    let doc = from_bytes(SAMPLE_V13).expect("the version 13 sample opens");
    assert_same(&rich_document_v13(), &doc);
    // And reads back the same through its own model.
    let model = v13::ProjectV13::from_document(&doc);
    assert_same(&doc, &from_bytes(&file_of_v13(&model)).unwrap());
}

/// The bytes of a version 14 file holding `model`, uncompressed.
fn file_of_v14(model: &v14::ProjectV14) -> Vec<u8> {
    let body = postcard::to_stdvec(model).unwrap();
    let mut bytes = header(14, 0, body.len() as u64);
    bytes.extend_from_slice(&body);
    bytes
}

/// A project saved by version 14 of the format, kept in the repository.
const SAMPLE_V14: &[u8] = include_bytes!("../../tests/data/sample-v14.avtl");

#[test]
fn a_file_saved_by_format_version_14_still_opens() {
    let doc = from_bytes(SAMPLE_V14).expect("the version 14 sample opens");
    assert_same(&rich_document_v14(), &doc);
    // And reads back the same through its own model.
    let model = v14::ProjectV14::from_document(&doc);
    assert_same(&doc, &from_bytes(&file_of_v14(&model)).unwrap());
}

/// The bytes of a version 15 file holding `model`, uncompressed.
fn file_of_v15(model: &v15::ProjectV15) -> Vec<u8> {
    let body = postcard::to_stdvec(model).unwrap();
    let mut bytes = header(15, 0, body.len() as u64);
    bytes.extend_from_slice(&body);
    bytes
}

/// A project saved by version 15 of the format, kept in the repository.
const SAMPLE_V15: &[u8] = include_bytes!("../../tests/data/sample-v15.avtl");

#[test]
fn a_file_saved_by_format_version_15_still_opens() {
    let doc = from_bytes(SAMPLE_V15).expect("the version 15 sample opens");
    assert_same(&rich_document_v15(), &doc);
    // Every shape sits on its pivot, in metres.
    assert_eq!(doc.scene().units(), scene::Units::Metres);
    for (id, _) in doc.scene().walk() {
        assert_eq!(doc.scene().object(id).unwrap().offset, Transform::default());
    }
    // And reads back the same through its own model.
    let model = v15::ProjectV15::from_document(&doc);
    assert_same(&doc, &from_bytes(&file_of_v15(&model)).unwrap());
}

#[test]
fn a_spiral_keeps_its_rise_and_way_round() {
    let doc = from_bytes(&to_bytes(&rich_document_v15()).unwrap()).unwrap();
    let text = format!("{:?}", v15::ProjectV15::from_document(&doc));
    // Both spirals, on its own and mirrored, keep the bend's every number.
    assert_eq!(text.matches("angle: 900.0").count(), 2, "{text}");
    assert_eq!(text.matches("rise: 7.5, mirrored: true").count(), 2);
    assert_eq!(text.matches("MirroredBent").count(), 1);
}

/// The bytes of a version 16 file holding `model`, uncompressed.
fn file_of_v16(model: &v16::ProjectV16) -> Vec<u8> {
    let body = postcard::to_stdvec(model).unwrap();
    let mut bytes = header(16, 0, body.len() as u64);
    bytes.extend_from_slice(&body);
    bytes
}

/// A project saved by version 16 of the format, kept in the repository.
const SAMPLE_V16: &[u8] = include_bytes!("../../tests/data/sample-v16.avtl");

#[test]
fn a_file_saved_by_format_version_16_still_opens() {
    let doc = from_bytes(SAMPLE_V16).expect("the version 16 sample opens");
    assert_same(&rich_document(), &doc);
    let model = v16::ProjectV16::from_document(&doc);
    assert_same(&doc, &from_bytes(&file_of_v16(&model)).unwrap());
}

#[test]
fn moved_pivots_and_the_units_come_back() {
    let doc = rich_document();
    let opened = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_eq!(opened.scene().units(), scene::Units::Centimetres);
    let moved: Vec<_> = opened
        .scene()
        .walk()
        .into_iter()
        .filter(|(id, _)| opened.scene().object(*id).unwrap().offset != Transform::default())
        .collect();
    assert!(moved.len() >= 2, "{moved:?}");
    for (id, _) in opened.scene().walk() {
        assert!(
            opened
                .scene()
                .shape_matrix(id)
                .abs_diff_eq(doc.scene().shape_matrix(id), 1e-4)
        );
    }
    // Only moved pivots are stored.
    let model = v16::ProjectV16::from_document(&doc);
    assert_eq!(model.offsets.len(), moved.len());
}

#[test]
fn damaged_shape_places_are_refused() {
    let good = v16::ProjectV16::from_document(&rich_document());
    let first = good.offsets[0];
    let cases: Vec<(v16::ProjectV16, &str)> = vec![
        {
            let mut bad = good.clone();
            bad.offsets.push(first);
            (bad, "placed twice")
        },
        {
            let mut bad = good.clone();
            bad.offsets[0].object = 999_999;
            (bad, "isn't there")
        },
        {
            let mut bad = good.clone();
            bad.offsets[0].offset.position[2] = f32::NAN;
            (bad, "real number")
        },
        {
            let mut bad = good.clone();
            bad.offsets[0].offset.scale = [1.0, 0.0, 1.0];
            (bad, "squashed flat")
        },
    ];
    for (bad, words) in cases {
        let text = refused(&file_of_v16(&bad)).to_string();
        assert!(text.contains(words), "{words}: {text}");
    }
}

/// Writes the current version's sample. Run only by hand, when a new
/// format version is added; earlier samples are kept as they are:
/// `WRITE_SAMPLE=1 cargo test -p fileio write_the_sample`.
#[test]
fn write_the_sample() {
    if std::env::var_os("WRITE_SAMPLE").is_some() {
        let name = format!("tests/data/sample-v{FORMAT_VERSION}.avtl");
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, to_bytes(&rich_document()).unwrap()).unwrap();
    }
}

#[test]
fn the_backup_sits_beside_the_project() {
    assert_eq!(
        backup_path(Path::new("/work/crate.avtl")),
        PathBuf::from("/work/crate.avtl.bak")
    );
}

/// Part of the heavy-mesh check (DESIGN.md §4.8): saving and opening a
/// project holding a shape of millions of polygons. Run with
/// `tools/heavy/heavy-check.sh`.
#[test]
#[ignore = "heavy: run with tools/heavy/heavy-check.sh"]
fn heavy_project_check() {
    use std::time::Instant;
    let polygons: usize = std::env::var("HEAVY_POLYGONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3_000_000);
    let mut doc = Document::new();
    doc.execute(scene::Command::Create(ObjectKind::Mesh(
        std::sync::Arc::new(mesh::samples::heavy_ball(polygons, 100.0)),
    )))
    .unwrap();
    let start = Instant::now();
    let bytes = to_bytes(&doc).unwrap();
    let saved = start.elapsed();
    let start = Instant::now();
    let back = from_bytes(&bytes).unwrap();
    let opened = start.elapsed();
    assert_same(&doc, &back);
    let line = |what: &str, took: std::time::Duration, target: u128| {
        println!(
            "{:<58} {:>8.1}ms {:>10} {:>8}ms  {}",
            what,
            took.as_secs_f64() * 1000.0,
            "",
            target,
            if took.as_millis() <= target {
                "ok"
            } else {
                "SLOW"
            }
        );
        took.as_millis() <= target
    };
    println!();
    println!("Heavy-mesh check, project files: {polygons} polygons");
    let ok = line("Save the project (to memory, compressed)", saved, 5_000)
        & line("Open it again", opened, 5_000);
    println!("Project file size: about {} MB", bytes.len() / 1_000_000);
    assert!(ok);
}

#[test]
fn a_project_that_wouldnt_reopen_is_never_saved_over_the_old_one() {
    // A regression test from the whole-app review (L3): a mesh with a
    // point that isn't a number can exist in a session but can't be
    // opened. Saving it must fail and leave the previous save alone.
    let folder = Folder::new("unreadable");
    let path = folder.0.join("crate.avtl");
    let good = rich_document();
    save(&path, &good).unwrap();
    let mut bad_mesh = mesh::Mesh::new();
    let a = bad_mesh.add_vertex(Vec3::ZERO).unwrap();
    let b = bad_mesh.add_vertex(Vec3::X).unwrap();
    let c = bad_mesh.add_vertex(Vec3::NAN).unwrap();
    bad_mesh.add_face(&[a, b, c]).unwrap();
    let mut bad = good.clone();
    bad.execute(Command::Create(ObjectKind::Mesh(std::sync::Arc::new(
        bad_mesh,
    ))))
    .unwrap();
    let error = save(&path, &bad).unwrap_err();
    assert!(matches!(error, ProjectError::WouldNotReopen(_)), "{error}");
    assert!(error.to_string().contains("previous save is untouched"));
    // The old project still opens, unchanged; the bytes are kept aside
    // for a bug report; no temporary files are left.
    assert_same(&good, &open(&path).unwrap());
    assert!(with_suffix(&path, ".unreadable").exists());
    assert!(!backup_path(&path).exists(), "nothing was replaced");
    let names: Vec<String> = std::fs::read_dir(&folder.0)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(!names.iter().any(|n| n.ends_with(".tmp")), "{names:?}");
    // Bytes for an autosave copy are checked the same way.
    assert!(to_bytes(&bad).is_err());
}

#[cfg(unix)]
#[test]
fn saving_never_writes_through_planted_links() {
    // A regression test from the whole-app review (S3): links beside the
    // project, named like its temporary file or its backup, must not
    // redirect the save onto another file.
    let folder = Folder::new("links");
    let path = folder.0.join("crate.avtl");
    let victim = folder.0.join("victim.txt");
    std::fs::write(&victim, b"keep me").unwrap();
    std::os::unix::fs::symlink(&victim, with_suffix(&path, ".tmp")).unwrap();
    std::os::unix::fs::symlink(&victim, backup_path(&path)).unwrap();
    let doc = rich_document();
    save(&path, &doc).unwrap();
    save(&path, &doc).unwrap();
    assert_eq!(std::fs::read(&victim).unwrap(), b"keep me");
    assert_same(&doc, &open(&path).unwrap());
    // The backup is a real file now, not the link.
    assert!(
        !std::fs::symlink_metadata(backup_path(&path))
            .unwrap()
            .file_type()
            .is_symlink()
    );
}

// ------------------------------------------------ limits (decision 0045) ---

/// A shown object at the origin, with no edits.
fn plain_object(id: u64, kind: v1::KindV1) -> v1::ObjectV1 {
    v1::ObjectV1 {
        id,
        parent: None,
        name: format!("Object {id}"),
        visible: true,
        transform: v1::TransformV1 {
            position: [0.0; 3],
            rotation: [0.0; 3],
            scale: [1.0; 3],
        },
        kind,
        layers: Vec::new().into(),
    }
}

/// A file of `model` compressed, as the app saves them.
fn packed_file_of(model: &v1::ProjectV1) -> Vec<u8> {
    let body = zstd::bulk::compress(&postcard::to_stdvec(model).unwrap(), 3).unwrap();
    let mut bytes = header(1, FLAG_ZSTD, body.len() as u64);
    bytes.extend_from_slice(&body);
    bytes
}

#[test]
fn too_many_objects_edits_or_polygons_are_refused_with_the_limit() {
    use crate::limits::{MAX_EDITS, MAX_OBJECTS, MAX_POLYGONS};
    let mut model = v1::ProjectV1::from_document(&Document::new());
    model.objects = (0..=MAX_OBJECTS as u64)
        .map(|id| plain_object(id, v1::KindV1::Group))
        .collect();
    model.next_id = MAX_OBJECTS as u64 + 1;
    let text = refused(&packed_file_of(&model)).to_string();
    assert!(
        text.contains("objects") && text.contains(&MAX_OBJECTS.to_string()),
        "{text}"
    );

    let mut model = v1::ProjectV1::from_document(&Document::new());
    let cube = v1::KindV1::Primitive(v1::PrimitiveV1::Cube {
        size: [100.0; 3],
        segments: [1; 3],
    });
    let mut object = plain_object(0, cube);
    let mut layer = v1::LayerFile {
        name: "Edits".into(),
        enabled: true,
        operations: Vec::new().into(),
    };
    for _ in 0..=MAX_EDITS {
        layer.operations.push(v1::OperationV1::MovePoints {
            points: vec![0].into(),
            offset: [0.0, 0.0, 1.0],
        });
    }
    object.layers.push(layer);
    model.objects.push(object);
    model.next_id = 1;
    let text = refused(&packed_file_of(&model)).to_string();
    assert!(text.contains("edits"), "{text}");

    // Many of the densest cubes: estimated from their settings, refused
    // before any is built.
    let dense = v1::KindV1::Primitive(v1::PrimitiveV1::Cube {
        size: [100.0; 3],
        segments: [256; 3],
    });
    let count = MAX_POLYGONS / (6 * 256 * 256) + 1;
    let mut model = v1::ProjectV1::from_document(&Document::new());
    model.objects = (0..count as u64)
        .map(|id| plain_object(id, dense.clone()))
        .collect();
    model.next_id = count as u64;
    let start = std::time::Instant::now();
    let text = refused(&packed_file_of(&model)).to_string();
    assert!(text.contains("polygons"), "{text}");
    assert!(start.elapsed().as_secs() < 2, "refused at once");
}

#[test]
fn a_file_that_unpacks_to_too_much_is_refused() {
    use std::io::Write;
    // A few kilobytes that unpack to more than the limit (a "zip bomb"),
    // with the size written up front, and without.
    let size = crate::limits::MAX_UNPACKED as usize + (1 << 20);
    for say_size in [true, false] {
        let mut encoder = zstd::stream::Encoder::new(Vec::new(), 19).unwrap();
        if say_size {
            encoder.set_pledged_src_size(Some(size as u64)).unwrap();
        }
        let zeros = vec![0u8; 1 << 20];
        for _ in 0..size / zeros.len() {
            encoder.write_all(&zeros).unwrap();
        }
        let body = encoder.finish().unwrap();
        assert!(body.len() < 100_000, "a small file: {}", body.len());
        let mut bytes = header(5, FLAG_ZSTD, body.len() as u64);
        bytes.extend_from_slice(&body);
        assert!(
            matches!(refused(&bytes), ProjectError::TooLarge),
            "{say_size}"
        );
    }
}

#[test]
fn reading_stops_when_the_memory_budget_runs_out() {
    let bytes = to_bytes(&rich_document()).unwrap();
    let small = crate::limits::within(10_000, || from_bytes(&bytes));
    let text = small.err().unwrap().to_string();
    assert!(text.contains("limit"), "{text}");
    assert!(crate::limits::within(100 << 20, || from_bytes(&bytes)).is_ok());
}

/// A simple repeatable sequence of numbers, for the fuzz tests.
struct Noise(u64);

impl Noise {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 16
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n.max(1)
    }

    /// A number that is often ordinary and sometimes hostile.
    fn float(&mut self) -> f32 {
        match self.below(8) {
            0 => f32::NAN,
            1 => f32::INFINITY,
            2 => -1e30,
            3 => 0.0,
            _ => self.below(2000) as f32 - 1000.0,
        }
    }

    fn count(&mut self) -> u32 {
        match self.below(6) {
            0 => u32::MAX,
            1 => 0,
            _ => self.below(300) as u32,
        }
    }
}

/// A file model with a few random, often nonsensical values: ids
/// pointing nowhere, corners naming missing points, numbers that aren't
/// real, huge settings, empty names, repeated edit ids.
fn random_model(noise: &mut Noise) -> v14::ProjectV14 {
    let mut model = v14::ProjectV14::from_document(&rich_document());
    // An instance pointing anywhere, or listed twice.
    match noise.below(6) {
        0 => model.instances.0[0].source = noise.below(40),
        1 => model.instances.0[0].object = noise.below(40),
        2 => {
            let again = model.instances.0[0];
            model.instances.0.push(again);
        }
        _ => {}
    }
    let count = model.project.objects.len() as u64;
    for _ in 0..1 + noise.below(3) {
        let at = noise.below(count) as usize;
        let object = &mut model.project.objects[at];
        match noise.below(10) {
            0 => object.id = noise.below(20),
            1 => object.parent = Some(noise.below(30)),
            2 => object.name = String::new(),
            3 => object.transform.position[noise.below(3) as usize] = noise.float(),
            4 => match &mut object.kind {
                v1::KindV1::Primitive(v1::PrimitiveV1::Cube { size, segments }) => {
                    size[0] = noise.float();
                    segments[noise.below(3) as usize] = noise.count();
                }
                v1::KindV1::Primitive(v1::PrimitiveV1::Sphere { sides, rings, .. }) => {
                    *sides = noise.count();
                    *rings = noise.count();
                }
                _ => {}
            },
            5 | 6 => {
                if let v1::KindV1::Mesh(mesh) = &mut object.kind {
                    match noise.below(3) {
                        0 => {
                            if let Some(Some(corners)) = mesh.faces.first_mut() {
                                corners[0] = noise.below(5000) as u32;
                            }
                        }
                        1 => {
                            if let Some(Some(p)) = mesh.points.first_mut() {
                                p[1] = noise.float();
                            }
                        }
                        _ => mesh
                            .hard
                            .push([noise.below(100) as u32, noise.below(100) as u32]),
                    }
                }
            }
            7 | 8 => {
                for layer in object.layers.iter_mut() {
                    for entry in layer.operations.iter_mut() {
                        match noise.below(4) {
                            0 => entry.id = noise.below(4),
                            1 => entry.enabled = !entry.enabled,
                            _ => match &mut entry.operation {
                                v14::OperationV14::V13(v13::OperationV13::V12(
                                    v12::OperationV12::V11(v11::OperationV11::V10(
                                        v10::OperationV10::V9(v9::OperationV9::V8(
                                            v8::OperationV8::V7(v7::OperationV7::V6(
                                                v6::OperationV6::V4(v4::OperationV4::BevelEdges {
                                                    segments,
                                                    width,
                                                    ..
                                                }),
                                            )),
                                        )),
                                    )),
                                )) => {
                                    *segments = noise.count();
                                    *width = noise.float();
                                }
                                v14::OperationV14::V13(v13::OperationV13::V12(
                                    v12::OperationV12::V11(v11::OperationV11::V10(
                                        v10::OperationV10::V9(v9::OperationV9::V8(
                                            v8::OperationV8::V7(v7::OperationV7::V6(
                                                v6::OperationV6::LoopCut {
                                                    edge,
                                                    cuts,
                                                    position,
                                                    ..
                                                },
                                            )),
                                        )),
                                    )),
                                )) => {
                                    edge[noise.below(2) as usize] = noise.below(40) as u32;
                                    *cuts = noise.count();
                                    *position = noise.float();
                                }
                                v14::OperationV14::V13(v13::OperationV13::V12(
                                    v12::OperationV12::V11(v11::OperationV11::V10(
                                        v10::OperationV10::V9(v9::OperationV9::V8(
                                            v8::OperationV8::V7(v7::OperationV7::Slit {
                                                points,
                                                eye,
                                                ..
                                            }),
                                        )),
                                    )),
                                )) => {
                                    if noise.below(2) == 0 {
                                        points.0.truncate(noise.below(3) as usize);
                                    } else if let Some(p) = points.first_mut() {
                                        p[noise.below(3) as usize] = noise.float();
                                    }
                                    if noise.below(4) == 0 {
                                        *eye = v7::EyeV7::Along([noise.float(), 0.0, 0.0]);
                                    }
                                }
                                v14::OperationV14::V13(v13::OperationV13::V12(
                                    v12::OperationV12::V11(v11::OperationV11::V10(
                                        v10::OperationV10::V9(v9::OperationV9::V8(
                                            v8::OperationV8::ExtrudeBent { distance, bend, .. },
                                        )),
                                    )),
                                )) => {
                                    *distance = noise.float();
                                    bend.angle = noise.float();
                                    bend.radius = noise.float();
                                    bend.segments = noise.count();
                                }
                                v14::OperationV14::V13(v13::OperationV13::V12(
                                    v12::OperationV12::V11(v11::OperationV11::V10(
                                        v10::OperationV10::V9(v9::OperationV9::Connect {
                                            cuts,
                                            edges,
                                            ..
                                        }),
                                    )),
                                )) => {
                                    *cuts = noise.count();
                                    if let Some(e) = edges.first_mut() {
                                        e[1] = e[0];
                                    }
                                }
                                v14::OperationV14::V13(v13::OperationV13::V12(
                                    v12::OperationV12::V11(v11::OperationV11::V10(
                                        v10::OperationV10::V9(v9::OperationV9::Bridge {
                                            segments,
                                            twist,
                                            ..
                                        }),
                                    )),
                                )) => {
                                    *segments = noise.count();
                                    *twist = noise.count() as i32;
                                }
                                v14::OperationV14::V13(v13::OperationV13::V12(
                                    v12::OperationV12::V11(v11::OperationV11::V10(
                                        v10::OperationV10::V9(v9::OperationV9::Weld {
                                            target, ..
                                        }),
                                    )),
                                )) => {
                                    *target = Some(noise.count());
                                }
                                v14::OperationV14::V13(v13::OperationV13::V12(
                                    v12::OperationV12::V11(v11::OperationV11::V10(
                                        v10::OperationV10::FillHoles { points, .. },
                                    )),
                                )) => {
                                    points.push(noise.count());
                                }
                                v14::OperationV14::V13(v13::OperationV13::V12(
                                    v12::OperationV12::V11(v11::OperationV11::HardEdgesByAngle {
                                        angle,
                                    }),
                                )) => {
                                    *angle = noise.float();
                                }
                                v14::OperationV14::V13(v13::OperationV13::SlidePoints {
                                    slides,
                                    ..
                                }) => {
                                    slides.push((noise.count(), noise.count(), noise.float()));
                                }
                                v14::OperationV14::Mirrored { axes, centre, .. } => {
                                    axes[0] = noise.below(2) == 0;
                                    centre[0] = noise.float();
                                }
                                v14::OperationV14::WeldMiddle {
                                    centre, distance, ..
                                } => {
                                    centre[1] = noise.float();
                                    *distance = noise.float();
                                }
                                _ => {}
                            },
                        }
                    }
                }
            }
            _ => {
                if let Some(m) = model.layers.first_mut() {
                    match noise.below(2) {
                        0 => m.object = noise.below(30),
                        _ => {
                            if let Some(layer) = m.layers.first_mut() {
                                layer.default = v5::ValueV5::Float(noise.float());
                            }
                        }
                    }
                }
            }
        }
    }
    model
}

#[test]
fn random_nonsense_inside_a_well_formed_file_is_refused_or_opened_never_a_crash() {
    let mut noise = Noise(2026);
    let mut opened = 0;
    for _ in 0..150 {
        let model = random_model(&mut noise);
        let bytes = file_of_v14(&model);
        let start = std::time::Instant::now();
        if let Ok(doc) = from_bytes(&bytes) {
            opened += 1;
            // Working out every shape (as drawing does) stays quick too.
            for (id, _) in doc.scene().walk() {
                let _ = doc.scene().evaluated(id);
            }
        }
        assert!(
            start.elapsed().as_secs() < 5,
            "a file took {:?} to read",
            start.elapsed()
        );
    }
    // Some damage is harmless and some isn't: both happen.
    assert!(opened > 0 && opened < 150, "{opened} opened");
}
