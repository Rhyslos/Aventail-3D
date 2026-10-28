//! Tests for `.avtl` project files: everything saved comes back exactly,
//! damaged or hostile files are refused in plain words (never a crash),
//! and saving to disk never leaves a half-written project.

use super::*;
use crate::limits::Bounded;
use glam::Vec3;
use mesh::{CubeParams, PrimitiveKind, VertexId};
use scene::{Command, Object, ObjectKind, Operation, Place, Primitive, Transform};

/// A document using everything the current version of the format can
/// hold: all of [`rich_document_v37`], plus two layout snapshots (one of
/// two objects) and a snapshot put back (version 38; the version 38 sample
/// holds exactly this).
pub(super) fn rich_document() -> Document {
    let mut doc = rich_document_v37();
    let named = |doc: &Document, name: &str| {
        doc.scene()
            .walk()
            .into_iter()
            .map(|(id, _)| id)
            .find(|id| doc.scene().object(*id).unwrap().name == name)
            .unwrap()
    };
    let (cube, sphere) = (named(&doc, "Cube"), named(&doc, "Sphere"));
    let both = scene::UvSnapshot::take(doc.scene(), &[cube, sphere], "Both, unwrapped").unwrap();
    let one = scene::UvSnapshot::take(doc.scene(), &[cube], "Cube only").unwrap();
    doc.execute(Command::SetSnapshots(vec![both.clone(), one]))
        .unwrap();
    for (id, operation) in both.restore(doc.scene()) {
        doc.execute(Command::Record { id, operation }).unwrap();
    }
    doc
}

/// A document using everything version 37 stores: all of
/// [`rich_document_v36`], plus a locked piece and a piece with settings of
/// its own (the version 37 sample holds exactly this).
fn rich_document_v37() -> Document {
    let mut doc = rich_document_v36();
    let cube = doc
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| doc.scene().object(*id).unwrap().name == "Cube")
        .unwrap();
    let mesh = doc.scene().evaluated(cube).unwrap().mesh;
    let faces: Vec<scene::FaceId> = mesh.faces().map(|(f, _)| f).collect();
    for operation in [
        Operation::SetUvLock {
            faces: faces[..1].to_vec(),
            lock: true,
            anchors: None,
        },
        Operation::SetPieceSettings {
            faces: faces[1..].to_vec(),
            settings: scene::PieceSettings {
                method: Some(scene::UnwrapMethod::SizeKeeping),
                strips: Some(false),
                density: Some(0.02),
            },
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

/// A document using everything version 36 stores: all of
/// [`rich_document_v35`], plus seam preferences, a stretch budget and an
/// unwrap with one (the version 36 sample holds exactly this).
fn rich_document_v36() -> Document {
    let mut doc = rich_document_v35();
    let sphere = doc
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| doc.scene().object(*id).unwrap().name == "Sphere")
        .unwrap();
    let mesh = doc.scene().evaluated(sphere).unwrap().mesh;
    let faces: Vec<scene::FaceId> = mesh.faces().map(|(f, _)| f).collect();
    let topology = mesh.topology();
    let inner: Vec<scene::EdgeKey> = topology
        .edges()
        .filter(|e| topology.edge_uses(*e).len() == 2)
        .collect();
    for operation in [
        Operation::SetSeamPreference {
            edges: inner[..1].to_vec(),
            preference: scene::SeamPreference::Never,
            anchors: None,
        },
        Operation::SetSeamPreference {
            edges: inner[1..2].to_vec(),
            preference: scene::SeamPreference::Prefer,
            anchors: None,
        },
        Operation::SetStretchBudget {
            faces: faces[..1].to_vec(),
            budget: Some(1.4),
            anchors: None,
        },
        Operation::Unwrap {
            faces,
            options: scene::UnwrapOptions {
                budget: Some(2.5),
                ..scene::UnwrapOptions::plain(true)
            },
            anchors: None,
        },
    ] {
        doc.execute(Command::Record {
            id: sphere,
            operation,
        })
        .unwrap();
    }
    doc
}

/// A document using everything version 35 stores: all of
/// [`rich_document_v34`], plus mending settings, an unwrap with every
/// choice changed and a relax of its own length (the version 35 sample
/// holds exactly this).
fn rich_document_v35() -> Document {
    let mut doc = rich_document_v34();
    doc.execute(Command::SetMending(scene::MendOptions {
        on: true,
        uneven: 4.5,
        size_off: 2.5,
    }))
    .unwrap();
    let plane = doc
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| doc.scene().object(*id).unwrap().name == "Plane")
        .unwrap();
    let mesh = doc.scene().evaluated(plane).unwrap().mesh;
    let faces: Vec<scene::FaceId> = mesh.faces().map(|(f, _)| f).collect();
    for operation in [
        Operation::Unwrap {
            faces: faces.clone(),
            options: scene::UnwrapOptions {
                strips: true,
                strip_angle: 35.0,
                method: scene::UnwrapMethod::AngleBased,
                only: None,
                hide: scene::HideSeams::Behind,
                budget: None,
            },
            anchors: None,
        },
        Operation::RelaxUvs {
            faces,
            steps: 150,
            anchors: None,
        },
    ] {
        doc.execute(Command::Record {
            id: plane,
            operation,
        })
        .unwrap();
    }
    doc
}

/// A document using everything version 34 stores: all of
/// [`rich_document_v33`], plus pinned UVs and an unwrap that keeps sizes
/// and unfolds across only (the version 34 sample holds exactly this).
fn rich_document_v34() -> Document {
    let mut doc = rich_document_v33();
    let plane = doc
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| doc.scene().object(*id).unwrap().name == "Plane")
        .unwrap();
    let mesh = doc.scene().evaluated(plane).unwrap().mesh;
    let points: Vec<VertexId> = mesh.vertices().map(|(v, _)| v).take(2).collect();
    let faces: Vec<scene::FaceId> = mesh.faces().map(|(f, _)| f).collect();
    let corners: Vec<Vec<VertexId>> = faces
        .iter()
        .map(|f| mesh.face(*f).unwrap().to_vec())
        .collect();
    let point_anchors = mesh::Anchors::record(
        &mesh,
        mesh::ElementKind::Points,
        &points.iter().map(|v| vec![*v]).collect::<Vec<_>>(),
    );
    for operation in [
        Operation::SetUvPins {
            faces: Vec::new(),
            points,
            pin: true,
            anchors: point_anchors,
        },
        Operation::Unwrap {
            faces: faces.clone(),
            options: scene::UnwrapOptions {
                method: scene::UnwrapMethod::SizeKeeping,
                only: Some(scene::UnfoldAxis::Across),
                ..scene::UnwrapOptions::plain(false)
            },
            anchors: mesh::Anchors::record(&mesh, mesh::ElementKind::Polygons, &corners),
        },
    ] {
        doc.execute(Command::Record {
            id: plane,
            operation,
        })
        .unwrap();
    }
    doc
}

/// A document using everything version 33 stores: all of
/// [`rich_document_v32`], plus a trim sheet and a piece laid on one of its
/// rows (the version 33 sample holds exactly this).
fn rich_document_v33() -> Document {
    let mut doc = rich_document_v32();
    let sheet = scene::TrimSheet::even(4);
    let row = sheet.rows[1].clone();
    doc.execute(Command::SetTrim(sheet)).unwrap();
    let plane = doc
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| doc.scene().object(*id).unwrap().name == "Plane")
        .unwrap();
    let mesh = doc.scene().evaluated(plane).unwrap().mesh;
    let faces: Vec<scene::FaceId> = mesh.faces().map(|(f, _)| f).take(1).collect();
    let corners: Vec<Vec<VertexId>> = faces
        .iter()
        .map(|f| mesh.face(*f).unwrap().to_vec())
        .collect();
    doc.execute(Command::Record {
        id: plane,
        operation: Operation::PlaceOnTrim {
            faces,
            bottom: row.bottom,
            top: row.top,
            anchors: mesh::Anchors::record(&mesh, mesh::ElementKind::Polygons, &corners),
        },
    })
    .unwrap();
    doc
}

/// A document using everything version 32 stores: all of
/// [`rich_document_v31`], plus packed islands (the version 32 sample holds
/// exactly this).
fn rich_document_v32() -> Document {
    let mut doc = rich_document_v31();
    let plane = doc
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| doc.scene().object(*id).unwrap().name == "Plane")
        .unwrap();
    let mesh = doc.scene().evaluated(plane).unwrap().mesh;
    let faces: Vec<scene::FaceId> = mesh.faces().map(|(f, _)| f).collect();
    doc.execute(Command::Record {
        id: plane,
        operation: Operation::PackUvs {
            islands: vec![(
                faces,
                glam::Affine2::from_scale_angle_translation(
                    glam::Vec2::splat(0.8),
                    std::f32::consts::FRAC_PI_2,
                    glam::Vec2::new(0.9, 0.05),
                ),
            )],
        },
    })
    .unwrap();
    doc
}

/// A document using everything version 31 stores: all of
/// [`rich_document_v30`], plus texture settings and a texel density set
/// (the version 31 sample holds exactly this).
fn rich_document_v31() -> Document {
    let mut doc = rich_document_v30();
    doc.execute(Command::SetTexture(scene::TextureSettings {
        size: 4096,
        density: 1024.0,
    }))
    .unwrap();
    let plane = doc
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| doc.scene().object(*id).unwrap().name == "Plane")
        .unwrap();
    let mesh = doc.scene().evaluated(plane).unwrap().mesh;
    let faces: Vec<scene::FaceId> = mesh.faces().map(|(f, _)| f).take(1).collect();
    let corners: Vec<Vec<VertexId>> = faces
        .iter()
        .map(|f| mesh.face(*f).unwrap().to_vec())
        .collect();
    doc.execute(Command::Record {
        id: plane,
        operation: Operation::SetTexelDensity {
            faces,
            density: 0.002,
            anchors: mesh::Anchors::record(&mesh, mesh::ElementKind::Polygons, &corners),
        },
    })
    .unwrap();
    doc
}

/// A document using everything version 30 stores: all of
/// [`rich_document_v29`], plus unwrapping, straightening and relaxing (the
/// version 30 sample holds exactly this).
fn rich_document_v30() -> Document {
    let mut doc = rich_document_v29();
    let plane = doc
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| doc.scene().object(*id).unwrap().name == "Plane")
        .unwrap();
    let mesh = doc.scene().evaluated(plane).unwrap().mesh;
    let faces: Vec<scene::FaceId> = mesh.faces().map(|(f, _)| f).collect();
    let corners: Vec<Vec<VertexId>> = faces
        .iter()
        .map(|f| mesh.face(*f).unwrap().to_vec())
        .collect();
    let anchors = mesh::Anchors::record(&mesh, mesh::ElementKind::Polygons, &corners);
    for operation in [
        Operation::Unwrap {
            faces: faces.clone(),
            options: scene::UnwrapOptions::plain(false),
            anchors: anchors.clone(),
        },
        Operation::RelaxUvs {
            faces: faces.clone(),
            steps: scene::RELAX_STEPS as u32,
            anchors: anchors.clone(),
        },
        Operation::StraightenUvs {
            faces: faces.clone(),
            anchors: anchors.clone(),
        },
    ] {
        doc.execute(Command::Record {
            id: plane,
            operation,
        })
        .unwrap();
    }
    doc
}

/// A document using everything version 29 stores: all of
/// [`rich_document_v28`], plus seams marked and cleared (the version 29
/// sample holds exactly this).
fn rich_document_v29() -> Document {
    let mut doc = rich_document_v28();
    let cube = doc
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| {
            let o = doc.scene().object(*id).unwrap();
            o.kind.has_shape()
                && doc
                    .scene()
                    .evaluated(*id)
                    .is_some_and(|e| e.mesh.topology().edges().count() > 4)
        })
        .unwrap();
    let mesh = doc.scene().evaluated(cube).unwrap().mesh;
    let inner: Vec<scene::EdgeKey> = mesh
        .topology()
        .edges()
        .filter(|e| mesh.topology().edge_uses(*e).len() == 2)
        .collect();
    let marked = inner
        .iter()
        .copied()
        .find(|e| !mesh::uvs::is_seam(&mesh, *e))
        .unwrap();
    let cleared = inner
        .iter()
        .copied()
        .find(|e| mesh::uvs::is_seam(&mesh, *e))
        .unwrap();
    for (edge, seam) in [(marked, true), (cleared, false)] {
        let (a, b) = edge.ends();
        doc.execute(Command::Record {
            id: cube,
            operation: Operation::SetSeams {
                edges: vec![edge],
                seam,
                anchors: mesh::Anchors::record(&mesh, mesh::ElementKind::Edges, &[vec![a, b]]),
            },
        })
        .unwrap();
    }
    doc
}

/// A document using everything version 28 stores: all of
/// [`rich_document_v26`], plus UVs moved by hand (the version 28 sample
/// holds exactly this).
fn rich_document_v28() -> Document {
    let mut doc = rich_document_v26();
    let plane = doc
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| doc.scene().object(*id).unwrap().name == "Plane")
        .unwrap();
    let mesh = doc.scene().evaluated(plane).unwrap().mesh;
    let faces: Vec<scene::FaceId> = mesh.faces().map(|(f, _)| f).collect();
    let corners: Vec<Vec<VertexId>> = faces
        .iter()
        .map(|f| mesh.face(*f).unwrap().to_vec())
        .collect();
    doc.execute(Command::Record {
        id: plane,
        operation: Operation::TransformUvs {
            faces,
            points: Vec::new(),
            transform: glam::Affine2::from_scale_angle_translation(
                glam::Vec2::splat(0.5),
                0.25,
                glam::Vec2::new(0.1, 0.2),
            ),
            anchors: mesh::Anchors::record(&mesh, mesh::ElementKind::Polygons, &corners),
        },
    })
    .unwrap();
    doc
}

/// A document using everything versions 26 and 27 store: all of
/// [`rich_document_v25`], plus an applied placement.
fn rich_document_v26() -> Document {
    let mut doc = rich_document_v25();
    let cylinder = doc
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| doc.scene().object(*id).unwrap().name == "Cylinder")
        .unwrap();
    doc.execute(scene::Command::SetTransforms(vec![(
        cylinder,
        scene::Transform {
            rotation: Vec3::new(0.3, 0.0, 0.0),
            scale: Vec3::new(1.5, 1.0, 0.75),
            ..doc.scene().object(cylinder).unwrap().transform
        },
    )]))
    .unwrap();
    doc.execute(scene::Command::ApplyTransforms {
        ids: vec![cylinder],
        parts: scene::ApplyParts::ROTATION_AND_SCALE,
    })
    .unwrap();
    doc
}

/// A document using everything version 25 stores.
fn rich_document_v25() -> Document {
    let mut doc = rich_document_v24();
    let cylinder = doc
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| doc.scene().object(*id).unwrap().name == "Cylinder")
        .unwrap();
    doc.execute(scene::Command::SetMirror {
        ids: vec![cylinder],
        mirror: Some(scene::LiveMirror {
            axes: [true, false, false],
            centre: Vec3::new(-20.0, 0.0, 0.0),
            weld_distance: 0.25,
            on_object: false,
        }),
    })
    .unwrap();
    doc
}

/// A document using everything version 24 stores.
fn rich_document_v24() -> Document {
    let mut doc = rich_document_v23();
    let cylinder = doc
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| doc.scene().object(*id).unwrap().name == "Cylinder")
        .unwrap();
    let shape = doc.scene().evaluated(cylinder).unwrap().mesh;
    let edge = shape.edges()[1];
    let groove = Operation::Groove {
        edges: vec![edge],
        shape: scene::GrooveShape {
            width: 2.0,
            depth: -1.25,
            sides: scene::GrooveSides::Straight,
        },
        stretch: Vec3::new(0.9, 0.9, 1.2),
        anchors: None,
    };
    let ridge = Operation::Groove {
        edges: vec![shape.edges()[3]],
        shape: scene::GrooveShape {
            width: 1.0,
            depth: 0.5,
            sides: scene::GrooveSides::Sloped,
        },
        stretch: Vec3::ONE,
        anchors: None,
    };
    for operation in [
        groove,
        Operation::Mirrored {
            op: Box::new(ridge),
            mirror: mesh::symmetry::Mirror {
                axes: [true, false, false],
                centre: Vec3::ZERO,
                clip: false,
            },
        },
    ] {
        doc.execute(Command::Record {
            id: cylinder,
            operation,
        })
        .unwrap();
    }
    doc
}

/// A document using everything version 23 of the format can hold: all
/// of [`rich_document_v22`], plus what version 23 adds (a bevel of edges
/// with its further settings, on its own and mirrored; the version 23
/// sample holds exactly this).
fn rich_document_v23() -> Document {
    let mut doc = rich_document_v22();
    let cube = doc
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| doc.scene().object(*id).unwrap().name == "Cube")
        .unwrap();
    let shape = doc.scene().evaluated(cube).unwrap().mesh;
    let edge = shape.edges()[0];
    let bevel = Operation::BevelEdges {
        edges: vec![edge],
        shape: scene::BevelShape {
            width: 1.5,
            segments: 2,
            profile: 0.25,
            options: scene::BevelOptions {
                skew: 0.35,
                flip_side: true,
                even: true,
                clamp: true,
                weld: false,
                weld_distance: 0.4,
                improved: false,
            },
        },
        stretch: Vec3::ONE,
        anchors: None,
    };
    for operation in [
        bevel.clone(),
        Operation::Mirrored {
            op: Box::new(bevel),
            mirror: mesh::symmetry::Mirror {
                axes: [false, false, true],
                centre: Vec3::new(0.0, 0.0, 1.0),
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
    doc
}

/// A document using everything version 22 of the format can hold: all
/// of [`rich_document_v21`], plus what version 22 adds (an extrude, an
/// inset and a bevel made on a stretched object, the extrude mirrored
/// too; the version 22 sample holds exactly this).
fn rich_document_v22() -> Document {
    let mut doc = rich_document_v21();
    let sphere = doc
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| doc.scene().object(*id).unwrap().name == "Sphere")
        .unwrap();
    let shape = doc.scene().evaluated(sphere).unwrap().mesh;
    let towards = |d: Vec3| {
        shape
            .faces()
            .map(|(f, _)| f)
            .max_by(|a, b| {
                shape
                    .face_normal(*a)
                    .dot(d)
                    .total_cmp(&shape.face_normal(*b).dot(d))
            })
            .unwrap()
    };
    let stretch = Vec3::new(1.4, 0.8, 0.8);
    let extrude = Operation::ExtrudePolygons {
        faces: vec![towards(Vec3::X)],
        distance: 6.0,
        grouping: scene::Grouping::Together,
        shape: scene::ExtrudeShape {
            end_size: 0.75,
            ..scene::ExtrudeShape::default()
        },
        stretch,
        anchors: None,
    };
    let inset = Operation::InsetPolygons {
        faces: vec![towards(Vec3::Y)],
        distance: 2.0,
        grouping: scene::Grouping::Separate,
        stretch,
        anchors: None,
    };
    let corner = shape.vertices().next().unwrap().0;
    let bevel = Operation::BevelPoints {
        points: vec![corner],
        width: 1.5,
        stretch,
        anchors: None,
    };
    for operation in [
        extrude.clone(),
        Operation::Mirrored {
            op: Box::new(extrude),
            mirror: mesh::symmetry::Mirror {
                axes: [true, false, false],
                centre: Vec3::ZERO,
                clip: false,
            },
        },
        inset,
        bevel,
    ] {
        doc.execute(Command::Record {
            id: sphere,
            operation,
        })
        .unwrap();
    }
    doc
}

/// A document using everything version 21 of the format can hold: all
/// of [`rich_document_v20`], plus what version 21 adds (a bend made on a
/// stretched object, on its own and mirrored; the version 21 sample holds
/// exactly this).
fn rich_document_v21() -> Document {
    let mut doc = rich_document_v20();
    let cylinder = doc
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| doc.scene().object(*id).unwrap().name == "Cylinder")
        .unwrap();
    let shape = doc.scene().evaluated(cylinder).unwrap().mesh;
    let top = shape
        .faces()
        .map(|(f, _)| f)
        .max_by(|a, b| shape.face_normal(*a).z.total_cmp(&shape.face_normal(*b).z))
        .unwrap();
    let bent = Operation::ExtrudeBent {
        faces: vec![top],
        distance: 15.0,
        grouping: scene::Grouping::Together,
        bend: scene::Bend {
            angle: 75.0,
            direction: 120.0,
            radius: 30.0,
            segments: 5,
            ..scene::Bend::default()
        },
        stretch: Vec3::new(0.75, 0.75, 1.5),
        anchors: None,
    };
    for operation in [
        bent.clone(),
        Operation::Mirrored {
            op: Box::new(bent),
            mirror: mesh::symmetry::Mirror {
                axes: [false, true, false],
                centre: Vec3::new(0.0, 2.0, 0.0),
                clip: true,
            },
        },
    ] {
        doc.execute(Command::Record {
            id: cylinder,
            operation,
        })
        .unwrap();
    }
    doc
}

/// A document using everything version 20 of the format can hold: all
/// of [`rich_document_v19`], plus what version 20 adds (each kind of
/// collision shape on a shape; the version 20 sample holds exactly this).
fn rich_document_v20() -> Document {
    let mut doc = rich_document_v19();
    let named = |doc: &Document, name: &str| {
        doc.scene()
            .walk()
            .into_iter()
            .map(|(id, _)| id)
            .find(|id| doc.scene().object(*id).unwrap().name == name)
            .unwrap()
    };
    for (name, collision) in [
        ("Cube", scene::Collision::Mesh),
        ("Sphere", scene::Collision::Convex),
        ("Cylinder", scene::Collision::Box),
    ] {
        let id = named(&doc, name);
        doc.execute(Command::SetCollision {
            ids: vec![id],
            collision,
        })
        .unwrap();
    }
    doc
}

/// A document using everything version 19 of the format can hold: all
/// of [`rich_document_v18`], plus what version 19 adds (a game project
/// folder, exporting on save; the version 19 sample holds exactly this).
fn rich_document_v19() -> Document {
    let mut doc = rich_document_v18();
    doc.execute(Command::SetExportTarget(Some(scene::ExportTarget {
        folder: "/home/someone/Games/Crate Run/models".into(),
        on_save: true,
    })))
    .unwrap();
    doc
}

/// A document using everything version 18 of the format can hold: all
/// of [`rich_document_v17`], plus what version 18 adds (materials on the
/// cube and on an instance, polygons given a slot, on their own and with
/// symmetry on; the version 18 sample holds exactly this).
fn rich_document_v18() -> Document {
    let mut doc = rich_document_v17();
    let find = |doc: &Document, name: &str| {
        doc.scene()
            .walk()
            .into_iter()
            .map(|(id, _)| id)
            .find(|id| doc.scene().object(*id).unwrap().name == name)
            .unwrap()
    };
    let cube = find(&doc, "Cube");
    let materials = vec![
        scene::Material::standard(),
        scene::Material {
            name: "Trim".into(),
            color: [0.9, 0.4, 0.1],
        },
        scene::Material {
            name: "Glass".into(),
            color: [0.2, 0.5, 1.0],
        },
    ];
    doc.execute(Command::SetMaterials {
        id: cube,
        materials: materials.clone(),
    })
    .unwrap();
    let instance = doc
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| {
            matches!(
                doc.scene().object(*id).unwrap().kind,
                ObjectKind::Instance(_)
            )
        })
        .unwrap();
    doc.execute(Command::SetMaterials {
        id: instance,
        materials: materials[1..].to_vec(),
    })
    .unwrap();
    let faces: Vec<mesh::FaceId> = doc
        .scene()
        .evaluated(cube)
        .unwrap()
        .mesh
        .faces()
        .map(|(f, _)| f)
        .take(3)
        .collect();
    for operation in [
        Operation::SetMaterial {
            faces: faces[..2].to_vec(),
            slot: 1,
            anchors: None,
        },
        Operation::Mirrored {
            op: Box::new(Operation::SetMaterial {
                faces: faces[2..].to_vec(),
                slot: 2,
                anchors: None,
            }),
            mirror: mesh::symmetry::Mirror {
                axes: [true, false, false],
                centre: glam::Vec3::ZERO,
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

/// A document using everything version 17 of the format can
/// hold: all of [`rich_document_v16`], plus what version 17 adds (shaped
/// extrudes: straight out with a smaller end and steps, along Z with a
/// flat end, and one made with symmetry on; the version 17 sample holds
/// exactly this).
fn rich_document_v17() -> Document {
    let mut doc = rich_document_v16();
    let cube = doc
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| doc.scene().object(*id).unwrap().name == "Cube")
        .unwrap();
    let shape = doc.scene().evaluated(cube).unwrap().mesh;
    let side = shape
        .faces()
        .map(|(f, _)| f)
        .max_by(|a, b| shape.face_normal(*a).x.total_cmp(&shape.face_normal(*b).x))
        .unwrap();
    let tapered = Operation::ExtrudePolygons {
        faces: vec![side],
        distance: 6.5,
        grouping: scene::Grouping::Together,
        shape: scene::ExtrudeShape {
            direction: scene::ExtrudeDirection::Straight,
            end_size: 0.75,
            steps: 3,
            flat_end: false,
        },
        anchors: None,
        stretch: glam::Vec3::ONE,
    };
    let level = Operation::ExtrudePolygons {
        faces: vec![side],
        distance: 2.0,
        grouping: scene::Grouping::Separate,
        shape: scene::ExtrudeShape {
            direction: scene::ExtrudeDirection::AlongZ,
            end_size: 1.25,
            steps: 1,
            flat_end: true,
        },
        anchors: None,
        stretch: glam::Vec3::ONE,
    };
    for operation in [
        tapered.clone(),
        level,
        Operation::Mirrored {
            op: Box::new(tapered),
            mirror: mesh::symmetry::Mirror {
                axes: [false, true, false],
                centre: glam::Vec3::new(0.0, 1.5, 0.0),
                clip: true,
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

/// A document using everything version 16 of the format can
/// hold: all of [`rich_document_v15`], plus what version 16 adds (a
/// pivot moved and turned apart from its shape, a group's pivot moved
/// with an object inside, and the project in centimetres; the version
/// 16 sample holds exactly this).
fn rich_document_v16() -> Document {
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
        stretch: glam::Vec3::ONE,
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
            stretch: glam::Vec3::ONE,
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
                options: scene::BevelOptions::EARLIER,
            },
            anchors,
            stretch: glam::Vec3::ONE,
        },
        Operation::BevelPoints {
            points: vec![VertexId::from_raw(5)],
            width: 1.5,
            anchors: None,
            stretch: glam::Vec3::ONE,
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
            stretch: glam::Vec3::ONE,
        },
        Operation::ExtrudePolygons {
            faces: vec![face],
            distance: -12.25,
            grouping: scene::Grouping::Together,
            shape: scene::ExtrudeShape::EARLIER,
            anchors,
            stretch: glam::Vec3::ONE,
        },
        Operation::ExtrudeEdges {
            edges: vec![mesh::EdgeKey::new(
                VertexId::from_raw(0),
                VertexId::from_raw(1),
            )],
            distance: 3.0,
            grouping: scene::Grouping::Together,
            anchors: None,
            stretch: glam::Vec3::ONE,
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
/// `doc` with object `id`'s plain mesh stripped of its UVs and seams, as
/// a primitive made editable before primitives had them (decision 0095).
fn without_uvs_on(doc: Document, id: scene::ObjectId) -> Document {
    let scene = doc.scene();
    let objects: Vec<_> = scene
        .walk()
        .into_iter()
        .map(|(o, _)| {
            let mut object = scene.object(o).unwrap().clone();
            if o == id
                && let ObjectKind::Mesh(m) = &object.kind
            {
                let mut m = (**m).clone();
                m.remove_attribute(mesh::attributes::UV);
                m.remove_attribute(mesh::attributes::SEAM);
                object.kind = ObjectKind::Mesh(std::sync::Arc::new(m));
            }
            (o, scene.parent(o), object)
        })
        .collect();
    let rebuilt = scene::Scene::from_objects(objects, scene.next_object_id()).unwrap();
    Document::from_scene(rebuilt, doc.selection())
}

fn rich_document_v1() -> Document {
    let mut doc = Document::new();
    // The kinds version 1 knows (the newer ones come with version 39).
    for kind in PrimitiveKind::ALL
        .into_iter()
        .filter(|k| k.in_first_files())
    {
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
    // As made before primitives had UVs, which the files up to now hold.
    let mut doc = without_uvs_on(doc, sphere);
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
///
/// Exactly, or else to 4 significant figures: a sample saved on one
/// computer is compared with a document worked out again on another,
/// whose sines and cosines may differ in the last bit (macOS on Arm and
/// Linux on x86 do), which a turned pivot carries into its numbers.
pub(super) fn assert_same(a: &Document, b: &Document) {
    // Stable names (decision 0090) are compared on their own: older files
    // have none, and a document's edits find their points by place alone
    // without them.
    let (a, b) = (&without_names(a), &without_names(b));
    if a.scene() != b.scene() {
        assert_eq!(
            rounded(&format!("{:?}", a.scene())),
            rounded(&format!("{:?}", b.scene()))
        );
    }
    assert_eq!(a.selection(), b.selection());
    assert_eq!(a.scene().next_object_id(), b.scene().next_object_id());
    for (id, _) in a.scene().walk() {
        let (x, y) = (a.scene().evaluated(id), b.scene().evaluated(id));
        if x != y {
            assert_eq!(
                rounded(&format!("{x:?}")),
                rounded(&format!("{y:?}")),
                "{id}"
            );
        }
    }
}

/// `doc` with its edits' stable names left out, as a file before version
/// 27 holds it.
fn without_names(doc: &Document) -> Document {
    let mut out = v26::ProjectV26::from_document(doc).into_document().unwrap();
    out.set_selection(doc.selection());
    out
}

/// `text` with every number that has a decimal point rounded to 4
/// significant figures (and -0 written as 0), for comparing values worked
/// out on different computers.
fn rounded(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let starts_number = chars[i].is_ascii_digit()
            || (chars[i] == '-' && chars.get(i + 1).is_some_and(char::is_ascii_digit));
        let after_word = i > 0 && (chars[i - 1].is_alphanumeric() || chars[i - 1] == '_');
        if !starts_number || after_word {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        let mut j = i + 1;
        while j < chars.len()
            && (chars[j].is_ascii_digit()
                || chars[j] == '.'
                || chars[j] == 'e'
                || (chars[j] == '-' && chars[j - 1] == 'e'))
        {
            j += 1;
        }
        let token: String = chars[i..j].iter().collect();
        match token.parse::<f64>() {
            Ok(x) if token.contains('.') || token.contains('e') => {
                let x = if x.abs() < 1e-4 { 0.0 } else { x };
                out.push_str(&format!("{x:.3e}"));
            }
            _ => out.push_str(&token),
        }
        i = j;
    }
    out
}

#[test]
fn rounding_for_comparisons_keeps_whole_numbers_and_rounds_the_rest() {
    assert_eq!(
        rounded("Vec3(-7.1011944, 10.0, 3) V2"),
        rounded("Vec3(-7.1011950, 10.000001, 3) V2")
    );
    assert_ne!(rounded("Vec3(1.5)"), rounded("Vec3(1.6)"));
    assert_eq!(rounded("(-0.0000001)"), rounded("(0.0)"));
    assert_eq!(rounded("FaceId(12)"), "FaceId(12)");
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
    // Version 1 can't hold UVs: compare the shape alone.
    let doc = without_uvs_on(doc, id);
    let raw = postcard::to_stdvec(&v1::ProjectV1::from_document(&doc))
        .unwrap()
        .len();
    let saved = to_bytes(&doc).unwrap().len();
    // Positions are floating-point numbers, which pack less tightly than
    // text; a plain mesh still saves noticeably smaller.
    assert!(saved * 10 < raw * 7, "{saved} of {raw} bytes");
}

// ------------------------------------------------------ files refused ---

pub(super) fn header(version: u32, flags: u32, length: u64) -> Vec<u8> {
    let mut bytes = b"AVTL".to_vec();
    bytes.extend_from_slice(&version.to_le_bytes());
    bytes.extend_from_slice(&flags.to_le_bytes());
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes
}

/// A file of the current version holding `model`, uncompressed.
pub(super) fn current_file(model: &v43::ProjectV43) -> Vec<u8> {
    let body = v43::encode(model).unwrap();
    let mut bytes = header(FORMAT_VERSION, 0, body.len() as u64);
    bytes.extend_from_slice(&body);
    bytes
}

pub(super) fn refused(bytes: &[u8]) -> ProjectError {
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
    let body = v43::encode(&v43::ProjectV43::from_document(&doc)).unwrap();
    let mut bytes = header(FORMAT_VERSION, 0, body.len() as u64);
    bytes.extend_from_slice(&body);
    assert_same(&doc, &from_bytes(&bytes).unwrap());
    // Data after the scene is refused.
    let mut longer = body.clone();
    longer.push(0);
    let mut bytes = header(FORMAT_VERSION, 0, longer.len() as u64);
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
    assert_same(&rich_document_v16(), &doc);
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
    let good = v16::ProjectV16::from_document(&rich_document_v16());
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

/// The bytes of a version 17 file holding `model`, uncompressed.
fn file_of_v17(model: &v17::ProjectV17) -> Vec<u8> {
    let body = postcard::to_stdvec(model).unwrap();
    let mut bytes = header(17, 0, body.len() as u64);
    bytes.extend_from_slice(&body);
    bytes
}

/// A project saved by version 17 of the format, kept in the repository.
const SAMPLE_V17: &[u8] = include_bytes!("../../tests/data/sample-v17.avtl");

#[test]
fn a_file_saved_by_format_version_17_still_opens() {
    let doc = from_bytes(SAMPLE_V17).expect("the version 17 sample opens");
    assert_same(&rich_document_v17(), &doc);
    let model = v17::ProjectV17::from_document(&doc);
    assert_same(&doc, &from_bytes(&file_of_v17(&model)).unwrap());
}

#[test]
fn shaped_extrudes_keep_their_shape_and_older_ones_the_first_way() {
    let doc = from_bytes(&to_bytes(&rich_document_v17()).unwrap()).unwrap();
    let text = format!("{:?}", v17::ProjectV17::from_document(&doc));
    assert_eq!(text.matches("Extrude(").count(), 2, "{text}");
    assert_eq!(text.matches("MirroredExtrude").count(), 1);
    assert!(text.contains("end_size: 0.75, steps: 3"));
    assert!(text.contains("AlongZ"));
    // Extrudes from older files open shaped the first way.
    let old = from_bytes(SAMPLE_V16).unwrap();
    let mut extrudes = 0;
    for (id, _) in old.scene().walk() {
        for layer in old.scene().object(id).unwrap().stack.layers() {
            for entry in &layer.entries {
                if let Operation::ExtrudePolygons { shape, .. } = entry.operation.inner() {
                    assert_eq!(*shape, scene::ExtrudeShape::EARLIER);
                    extrudes += 1;
                }
            }
        }
    }
    assert!(extrudes > 0);
}

/// The bytes of a version 18 file holding `model`, uncompressed.
fn file_of_v18(model: &v18::ProjectV18) -> Vec<u8> {
    let body = postcard::to_stdvec(model).unwrap();
    let mut bytes = header(18, 0, body.len() as u64);
    bytes.extend_from_slice(&body);
    bytes
}

/// A project saved by version 18 of the format, kept in the repository.
const SAMPLE_V18: &[u8] = include_bytes!("../../tests/data/sample-v18.avtl");

#[test]
fn a_file_saved_by_format_version_18_still_opens() {
    let doc = from_bytes(SAMPLE_V18).expect("the version 18 sample opens");
    assert_same(&rich_document_v18(), &doc);
    let model = v18::ProjectV18::from_document(&doc);
    assert_same(&doc, &from_bytes(&file_of_v18(&model)).unwrap());
}

#[test]
fn materials_and_slots_come_back_and_older_files_have_none() {
    let doc = from_bytes(&to_bytes(&rich_document()).unwrap()).unwrap();
    let model: v18::ProjectV18 = v18::ProjectV18::from_document(&doc);
    // Only the two objects with materials store a list.
    assert_eq!(model.materials.len(), 2);
    let text = format!("{model:?}");
    assert_eq!(text.matches("SetMaterial(").count(), 1, "{text}");
    assert_eq!(text.matches("MirroredSetMaterial").count(), 1);
    let old = from_bytes(SAMPLE_V17).unwrap();
    for (id, _) in old.scene().walk() {
        assert!(old.scene().object(id).unwrap().materials.is_empty());
    }
}

#[test]
fn damaged_materials_are_refused() {
    let good = v18::ProjectV18::from_document(&rich_document());
    let cases: Vec<(v18::ProjectV18, &str)> = vec![
        {
            let mut bad = good.clone();
            let first = bad.materials[0].clone();
            bad.materials.push(first);
            (bad, "stored twice")
        },
        {
            let mut bad = good.clone();
            bad.materials[0].object = 999_999;
            (bad, "isn't there")
        },
        {
            let mut bad = good.clone();
            bad.materials[0].list[0].name = " ".into();
            (bad, "no name")
        },
        {
            let mut bad = good.clone();
            bad.materials[0].list[0].color[1] = f32::NAN;
            (bad, "colour")
        },
        {
            let mut bad = good.clone();
            let first = bad.materials[0].list[0].clone();
            bad.materials[0].list = vec![first; mesh::material::MAX_SLOTS as usize + 1].into();
            (bad, "more than")
        },
    ];
    for (bad, words) in cases {
        let text = refused(&file_of_v18(&bad)).to_string();
        assert!(text.contains(words), "{words}: {text}");
    }
}

/// The bytes of a version 19 file holding `model`, uncompressed.
fn file_of_v19(model: &v19::ProjectV19) -> Vec<u8> {
    let body = postcard::to_stdvec(model).unwrap();
    let mut bytes = header(19, 0, body.len() as u64);
    bytes.extend_from_slice(&body);
    bytes
}

/// A project saved by version 19 of the format, kept in the repository.
const SAMPLE_V19: &[u8] = include_bytes!("../../tests/data/sample-v19.avtl");

#[test]
fn a_file_saved_by_format_version_19_still_opens() {
    let doc = from_bytes(SAMPLE_V19).expect("the version 19 sample opens");
    assert_same(&rich_document_v19(), &doc);
    let target = doc.scene().export_target().unwrap();
    assert_eq!(target.folder, "/home/someone/Games/Crate Run/models");
    assert!(target.on_save);
    let model = v19::ProjectV19::from_document(&doc);
    assert_same(&doc, &from_bytes(&file_of_v19(&model)).unwrap());
    // Older files have none.
    assert!(
        from_bytes(SAMPLE_V18)
            .unwrap()
            .scene()
            .export_target()
            .is_none()
    );
}

#[test]
fn damaged_game_project_folders_are_refused() {
    let good = v19::ProjectV19::from_document(&rich_document());
    for (folder, words) in [
        ("   ".to_string(), "folder"),
        ("a\nb".to_string(), "folder"),
        ("x".repeat(scene::units::MAX_FOLDER_LEN + 1), "folder"),
    ] {
        let mut bad = good.clone();
        bad.target.as_mut().unwrap().folder = folder;
        let text = refused(&file_of_v19(&bad)).to_string();
        assert!(text.contains(words), "{text}");
    }
}

/// The bytes of a version 20 file holding `model`, uncompressed.
fn file_of_v20(model: &v20::ProjectV20) -> Vec<u8> {
    let body = postcard::to_stdvec(model).unwrap();
    let mut bytes = header(20, 0, body.len() as u64);
    bytes.extend_from_slice(&body);
    bytes
}

/// A project saved by version 20 of the format, kept in the repository.
const SAMPLE_V20: &[u8] = include_bytes!("../../tests/data/sample-v20.avtl");

#[test]
fn a_file_saved_by_format_version_20_still_opens() {
    let doc = from_bytes(SAMPLE_V20).expect("the version 20 sample opens");
    assert_same(&rich_document_v20(), &doc);
    let model: v20::ProjectV20 = v20::ProjectV20::from_document(&doc);
    assert_eq!(model.collisions.len(), 3);
    assert_same(&doc, &from_bytes(&file_of_v20(&model)).unwrap());
    // Older files have none.
    let old = from_bytes(SAMPLE_V19).unwrap();
    assert!(
        old.scene()
            .walk()
            .iter()
            .all(|(id, _)| old.scene().object(*id).unwrap().collision == scene::Collision::None)
    );
}

#[test]
fn damaged_collisions_are_refused() {
    let good: v20::ProjectV20 = v20::ProjectV20::from_document(&rich_document_v20());
    let mut twice = good.clone();
    let first = twice.collisions[0];
    twice.collisions.push(first);
    assert!(refused(&file_of_v20(&twice)).to_string().contains("twice"));
    let mut missing = good.clone();
    missing.collisions[0].object = 999_999;
    assert!(
        refused(&file_of_v20(&missing))
            .to_string()
            .contains("isn't there")
    );
}

/// The bytes of a version 21 file holding `model`, uncompressed.
fn file_of_v21(model: &v21::ProjectV21) -> Vec<u8> {
    let body = postcard::to_stdvec(model).unwrap();
    let mut bytes = header(21, 0, body.len() as u64);
    bytes.extend_from_slice(&body);
    bytes
}

/// A project saved by version 21 of the format, kept in the repository.
const SAMPLE_V21: &[u8] = include_bytes!("../../tests/data/sample-v21.avtl");

#[test]
fn a_file_saved_by_format_version_21_still_opens() {
    let doc = from_bytes(SAMPLE_V21).expect("the version 21 sample opens");
    assert_same(&rich_document_v21(), &doc);
    let model = v21::ProjectV21::from_document(&doc);
    assert_same(&doc, &from_bytes(&file_of_v21(&model)).unwrap());
}

#[test]
fn stretched_bends_come_back_and_older_files_have_none() {
    let doc = from_bytes(&to_bytes(&rich_document_v21()).unwrap()).unwrap();
    let text = format!("{:?}", v21::ProjectV21::from_document(&doc));
    assert_eq!(text.matches("StretchedBent {").count(), 2, "{text}");
    assert_eq!(text.matches("MirroredStretchedBent").count(), 1);
    let stretches = |doc: &Document| -> Vec<Vec3> {
        doc.scene()
            .walk()
            .iter()
            .flat_map(|(id, _)| doc.scene().object(*id).unwrap().stack.layers().to_vec())
            .flat_map(|layer| layer.entries.clone())
            .filter_map(|entry| match entry.operation.inner() {
                Operation::ExtrudeBent { stretch, .. } => Some(*stretch),
                _ => None,
            })
            .collect()
    };
    let now = stretches(&doc);
    assert_eq!(
        now.iter().filter(|s| **s != Vec3::ONE).count(),
        2,
        "{now:?}"
    );
    let old = from_bytes(SAMPLE_V20).unwrap();
    let before = stretches(&old);
    assert!(!before.is_empty() && before.iter().all(|s| *s == Vec3::ONE));
}

#[test]
fn a_damaged_stretch_is_refused() {
    let mut model = v21::ProjectV21::from_document(&rich_document_v21());
    let text = format!("{model:?}");
    assert!(text.contains("StretchedBent"));
    let mut broke = false;
    for object in &mut model.project.project.places.project.project.objects.0 {
        for layer in &mut object.layers.0 {
            for entry in &mut layer.operations.0 {
                if let v21::OperationV21::StretchedBent { stretch, .. } = &mut entry.operation {
                    stretch[1] = -2.0;
                    broke = true;
                }
            }
        }
    }
    assert!(broke);
    assert!(
        refused(&file_of_v21(&model))
            .to_string()
            .contains("stretch")
    );
}

/// The bytes of a version 22 file holding `model`, uncompressed.
fn file_of_v22(model: &v22::ProjectV22) -> Vec<u8> {
    let body = postcard::to_stdvec(model).unwrap();
    let mut bytes = header(22, 0, body.len() as u64);
    bytes.extend_from_slice(&body);
    bytes
}

/// A project saved by version 22 of the format, kept in the repository.
const SAMPLE_V22: &[u8] = include_bytes!("../../tests/data/sample-v22.avtl");

#[test]
fn a_file_saved_by_format_version_22_still_opens() {
    let doc = from_bytes(SAMPLE_V22).expect("the version 22 sample opens");
    assert_same(&rich_document_v22(), &doc);
    let model = v22::ProjectV22::from_document(&doc);
    assert_same(&doc, &from_bytes(&file_of_v22(&model)).unwrap());
}

/// Every recorded edit's stretch, in order.
fn stretches(doc: &Document) -> Vec<Vec3> {
    doc.scene()
        .walk()
        .iter()
        .flat_map(|(id, _)| doc.scene().object(*id).unwrap().stack.layers().to_vec())
        .flat_map(|layer| layer.entries.clone())
        .filter(|entry| entry.operation.keeps_stretch())
        .map(|entry| entry.operation.stretch())
        .collect()
}

#[test]
fn stretched_edits_come_back_and_older_files_keep_only_their_bends() {
    let doc = from_bytes(&to_bytes(&rich_document_v22()).unwrap()).unwrap();
    let text = format!("{:?}", v22::ProjectV22::from_document(&doc));
    // The two bends of version 21 and the four new edits.
    assert_eq!(text.matches("Stretched {").count(), 6, "{text}");
    let now = stretches(&doc);
    assert_eq!(
        now.iter().filter(|s| **s != Vec3::ONE).count(),
        6,
        "{now:?}"
    );
    // A version 21 file's stretched bends keep their stretch.
    let v21 = from_bytes(SAMPLE_V21).unwrap();
    assert_eq!(
        stretches(&v21).iter().filter(|s| **s != Vec3::ONE).count(),
        2
    );
    let old = from_bytes(SAMPLE_V20).unwrap();
    assert!(stretches(&old).iter().all(|s| *s == Vec3::ONE));
}

#[test]
fn a_damaged_edit_stretch_is_refused() {
    let mut model = v22::ProjectV22::from_document(&rich_document_v22());
    let mut broke = false;
    for object in &mut model.project.project.places.project.project.objects.0 {
        for layer in &mut object.layers.0 {
            for entry in &mut layer.operations.0 {
                if let v22::OperationV22::Stretched { stretch, .. } = &mut entry.operation {
                    stretch[0] = f32::NAN;
                    broke = true;
                }
            }
        }
    }
    assert!(broke);
    assert!(
        refused(&file_of_v22(&model))
            .to_string()
            .contains("stretch")
    );
}

/// The bytes of a version 23 file holding `model`, uncompressed.
fn file_of_v23(model: &v23::ProjectV23) -> Vec<u8> {
    let body = postcard::to_stdvec(model).unwrap();
    let mut bytes = header(23, 0, body.len() as u64);
    bytes.extend_from_slice(&body);
    bytes
}

/// A project saved by version 23 of the format, kept in the repository.
const SAMPLE_V23: &[u8] = include_bytes!("../../tests/data/sample-v23.avtl");

#[test]
fn a_file_saved_by_format_version_23_still_opens() {
    let doc = from_bytes(SAMPLE_V23).expect("the version 23 sample opens");
    assert_same(&rich_document_v23(), &doc);
    let model = v23::ProjectV23::from_document(&doc);
    assert_same(&doc, &from_bytes(&file_of_v23(&model)).unwrap());
}

/// Every bevel of edges' settings, in order.
fn bevel_settings(doc: &Document) -> Vec<scene::BevelOptions> {
    doc.scene()
        .walk()
        .iter()
        .flat_map(|(id, _)| doc.scene().object(*id).unwrap().stack.layers().to_vec())
        .flat_map(|layer| layer.entries.clone())
        .filter_map(|entry| match entry.operation.inner() {
            Operation::BevelEdges { shape, .. } => Some(shape.options),
            _ => None,
        })
        .collect()
}

#[test]
fn bevel_settings_come_back_and_older_bevels_have_none() {
    let doc = from_bytes(&to_bytes(&rich_document_v23()).unwrap()).unwrap();
    let text = format!("{:?}", v23::ProjectV23::from_document(&doc));
    assert_eq!(text.matches("Bevel { op").count(), 2, "{text}");
    let now = bevel_settings(&doc);
    assert_eq!(
        now.iter()
            .filter(|o| **o != scene::BevelOptions::EARLIER)
            .count(),
        2,
        "{now:?}"
    );
    let old = from_bytes(SAMPLE_V22).unwrap();
    let before = bevel_settings(&old);
    assert!(!before.is_empty() && before.iter().all(|o| *o == scene::BevelOptions::EARLIER));
}

/// The bytes of a version 24 file holding `model`, uncompressed.
fn file_of_v24(model: &v24::ProjectV24) -> Vec<u8> {
    let body = postcard::to_stdvec(model).unwrap();
    let mut bytes = header(24, 0, body.len() as u64);
    bytes.extend_from_slice(&body);
    bytes
}

/// A project saved by version 24 of the format, kept in the repository.
const SAMPLE_V24: &[u8] = include_bytes!("../../tests/data/sample-v24.avtl");

#[test]
fn a_file_saved_by_format_version_24_still_opens() {
    let doc = from_bytes(SAMPLE_V24).expect("the version 24 sample opens");
    assert_same(&rich_document_v24(), &doc);
    let model = v24::ProjectV24::from_document(&doc);
    assert_same(&doc, &from_bytes(&file_of_v24(&model)).unwrap());
    let text = format!("{model:?}");
    assert_eq!(text.matches("Groove(").count(), 1, "{text}");
    assert_eq!(text.matches("MirroredGroove").count(), 1);
}

/// A project saved by version 25 of the format, kept in the repository.
const SAMPLE_V25: &[u8] = include_bytes!("../../tests/data/sample-v25.avtl");

#[test]
fn a_file_saved_by_format_version_25_still_opens_with_its_mirror() {
    let doc = from_bytes(SAMPLE_V25).expect("the version 25 sample opens");
    assert_same(&rich_document_v25(), &doc);
    let mirrored = doc
        .scene()
        .walk()
        .into_iter()
        .filter(|(id, _)| doc.scene().object(*id).unwrap().mirror.is_some())
        .count();
    assert_eq!(mirrored, 1);
    // Older files open with no mirrors.
    let old = from_bytes(SAMPLE_V24).unwrap();
    assert!(
        old.scene()
            .walk()
            .iter()
            .all(|(id, _)| old.scene().object(*id).unwrap().mirror.is_none())
    );
}

/// A project saved by version 26 of the format, kept in the repository.
const SAMPLE_V26: &[u8] = include_bytes!("../../tests/data/sample-v26.avtl");

#[test]
fn a_file_saved_by_format_version_26_still_opens_with_its_applied_placement() {
    let doc = from_bytes(SAMPLE_V26).expect("the version 26 sample opens");
    assert_same(&rich_document_v26(), &doc);
    let text = format!("{:?}", v26::ProjectV26::from_document(&doc));
    assert_eq!(text.matches("PlaceShape").count(), 1, "{text}");
}

/// Every recorded edit's anchored points' names, object by object.
fn all_names(doc: &Document) -> Vec<Option<Vec<u64>>> {
    doc.scene()
        .walk()
        .iter()
        .flat_map(|(id, _)| doc.scene().object(*id).unwrap().stack.layers().to_vec())
        .flat_map(|layer| layer.entries.clone())
        .map(|entry| entry.operation.anchors().and_then(|a| a.names.clone()))
        .collect()
}

/// A project saved by version 27 of the format, kept in the repository.
const SAMPLE_V27: &[u8] = include_bytes!("../../tests/data/sample-v27.avtl");

#[test]
fn stable_names_are_saved_and_come_back() {
    let doc = rich_document_v26();
    let names = all_names(&doc);
    assert!(names.iter().flatten().count() > 3, "{names:?}");
    let back = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_eq!(all_names(&back), names);
    // The same shapes, names and all.
    for (id, _) in doc.scene().walk() {
        assert_eq!(doc.scene().evaluated(id), back.scene().evaluated(id));
    }
    let sample = from_bytes(SAMPLE_V27).expect("the version 27 sample opens");
    assert_eq!(all_names(&sample), names);
    // Older files have none.
    assert!(
        all_names(&from_bytes(SAMPLE_V26).unwrap())
            .iter()
            .all(Option::is_none)
    );
}

#[test]
fn broken_names_are_refused() {
    let doc = rich_document_v26();
    let good = v27::ProjectV27::<v26::EntryV26>::from_document(&doc);
    let first = good.names.iter().next().cloned().unwrap();
    let cases = [
        // Twice.
        vec![first.clone(), first.clone()],
        // Too few for the edit's points.
        vec![v27::NamesV27 {
            names: first.names.iter().skip(1).copied().collect(),
            ..first.clone()
        }],
        // A zero.
        vec![v27::NamesV27 {
            names: first.names.iter().map(|_| 0).collect(),
            ..first.clone()
        }],
        // An edit that isn't there.
        vec![v27::NamesV27 {
            edit: 9999,
            ..first.clone()
        }],
        // An object that isn't there.
        vec![v27::NamesV27 {
            object: 9999,
            ..first.clone()
        }],
    ];
    for names in cases {
        let model = v27::ProjectV27 {
            names: names.into_iter().collect(),
            ..good.clone()
        };
        assert!(model.into_document().is_err());
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
        let doc = super::v66_tests::rich_document_v66();
        std::fs::write(path, to_bytes(&doc).unwrap()).unwrap();
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

#[test]
fn edit_layers_keep_their_names_order_and_switches() {
    // Decision 0091: layers made in the Edit layers panel need nothing new
    // in the file; their names, order and switches were always saved.
    let mut doc = rich_document();
    let id = doc
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| doc.scene().object(*id).is_some_and(|o| !o.stack.is_empty()))
        .expect("the rich document has edits");
    doc.execute(scene::Command::AddLayer(vec![id])).unwrap();
    doc.execute(scene::Command::AddLayer(vec![id])).unwrap();
    let top = doc.scene().object(id).unwrap().stack.layers().len() - 1;
    doc.execute(scene::Command::ChangeLayer {
        id,
        index: top,
        change: scene::LayerChange::Rename("Details".into()),
    })
    .unwrap();
    doc.execute(scene::Command::ChangeLayer {
        id,
        index: top,
        change: scene::LayerChange::SetEnabled(false),
    })
    .unwrap();
    doc.execute(scene::Command::ChangeLayer {
        id,
        index: top,
        change: scene::LayerChange::Move { up: false },
    })
    .unwrap();
    let back = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    let layers = |d: &Document| -> Vec<(String, bool, usize)> {
        d.scene()
            .object(id)
            .unwrap()
            .stack
            .layers()
            .iter()
            .map(|l| (l.name.clone(), l.enabled, l.entries.len()))
            .collect()
    };
    assert_eq!(layers(&back), layers(&doc));
    assert!(layers(&back).iter().any(|(n, on, _)| n == "Details" && !on));
    assert_eq!(
        doc.scene().evaluated(id).unwrap().mesh,
        back.scene().evaluated(id).unwrap().mesh
    );
}

#[test]
fn a_frozen_shape_and_the_edits_after_it_survive_saving() {
    // Decision 0092: freezing makes the base a plain mesh, which files
    // have always stored; later edits keep applying to it.
    let mut doc = rich_document();
    let found = doc.scene().walk().into_iter().find_map(|(id, _)| {
        let stack = &doc.scene().object(id)?.stack;
        let first = stack.layers().iter().find_map(|l| l.entries.first())?;
        (stack.operation_count() > 1 && stack.freeze_problem(first.id).is_none())
            .then_some((id, first.id))
    });
    let (id, edit) = found.expect("the rich document has an object with two edits");
    let before = doc.scene().evaluated(id).unwrap().mesh;
    doc.execute(scene::Command::Freeze { id, edit }).unwrap();
    let back = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    let after = back.scene().evaluated(id).unwrap().mesh;
    let points = |m: &scene::Mesh| m.vertices().map(|(_, p)| p).collect::<Vec<_>>();
    assert_eq!(points(&after), points(&before));
    assert!(matches!(
        back.scene().object(id).unwrap().kind,
        scene::ObjectKind::Mesh(_)
    ));
}

const SAMPLE_V28: &[u8] = include_bytes!("../../tests/data/sample-v28.avtl");

#[test]
fn a_file_saved_by_format_version_28_still_opens_with_its_moved_uvs() {
    let doc = from_bytes(SAMPLE_V28).expect("the version 28 sample opens");
    assert_same(&rich_document_v28(), &doc);
    let text = format!("{:?}", v28::ProjectV28::from_document(&doc));
    assert_eq!(text.matches("TransformUvs").count(), 1, "{text}");
    // The plane's UVs are moved: half size, turned and shifted.
    let plane = doc
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .find(|id| doc.scene().object(*id).unwrap().name == "Plane")
        .unwrap();
    let mesh = doc.scene().evaluated(plane).unwrap().mesh;
    let uvs = mesh::uvs::Uvs::of(&mesh).unwrap();
    let (face, _) = mesh.faces().next().unwrap();
    let corner = uvs.corner(&mesh, face, 0).unwrap();
    assert!(corner.x > 0.09 && corner.x < 0.65, "{corner}");
}

const SAMPLE_V29: &[u8] = include_bytes!("../../tests/data/sample-v29.avtl");

#[test]
fn a_file_saved_by_format_version_29_still_opens_with_its_seams() {
    let doc = from_bytes(SAMPLE_V29).expect("the version 29 sample opens");
    assert_same(&rich_document_v29(), &doc);
    let text = format!("{:?}", v29::ProjectV29::from_document(&doc));
    assert_eq!(text.matches("SetSeams").count(), 2, "{text}");
}

const SAMPLE_V30: &[u8] = include_bytes!("../../tests/data/sample-v30.avtl");

#[test]
fn a_file_saved_by_format_version_30_still_opens_unwrapped() {
    let doc = from_bytes(SAMPLE_V30).expect("the version 30 sample opens");
    assert_same(&rich_document_v30(), &doc);
    assert_eq!(doc.scene().texture(), scene::TextureSettings::default());
    let text = format!("{:?}", v30::ProjectV30::from_document(&doc));
    for kind in ["Unwrap", "StraightenUvs", "RelaxUvs"] {
        assert!(text.contains(kind), "{kind}");
    }
}

const SAMPLE_V32: &[u8] = include_bytes!("../../tests/data/sample-v32.avtl");

#[test]
fn a_file_saved_by_format_version_32_still_opens_packed() {
    let doc = from_bytes(SAMPLE_V32).expect("the version 32 sample opens");
    assert_same(&rich_document_v32(), &doc);
    assert!(doc.scene().trim().rows.is_empty());
    let text = format!(
        "{:?}",
        v31::ProjectV31::<v32::EntryV32>::from_document(&doc)
    );
    assert!(text.contains("PackUvs"));
}

const SAMPLE_V33: &[u8] = include_bytes!("../../tests/data/sample-v33.avtl");

#[test]
fn a_file_saved_by_format_version_33_opens_with_its_trim_sheet() {
    let doc = from_bytes(SAMPLE_V33).expect("the version 33 sample opens");
    assert_same(&rich_document_v33(), &doc);
    assert_eq!(doc.scene().trim(), &scene::TrimSheet::even(4));
    let text = format!(
        "{:?}",
        v33::ProjectV33::<v33::EntryV33>::from_document(&doc)
    );
    assert!(text.contains("PlaceOnTrim"));
}

// ------------------------------------- the development format (0126) ---

/// The rich document with development settings: the cube smoothed by
/// marked edges only, the other by 45°, and the cube's mirror on it.
pub(super) fn rich_document_v39() -> Document {
    let mut doc = rich_document();
    let ids: Vec<scene::ObjectId> = doc
        .scene()
        .walk()
        .into_iter()
        .map(|(id, _)| id)
        .filter(|id| doc.scene().object(*id).is_some_and(|o| o.kind.has_shape()))
        .collect();
    doc.execute(Command::SetHardness {
        ids: vec![ids[0]],
        hardness: scene::Hardness::Marked,
    })
    .unwrap();
    doc.execute(Command::SetHardness {
        ids: ids[1..].to_vec(),
        hardness: scene::Hardness::ByAngle(45.0),
    })
    .unwrap();
    doc.execute(Command::SetMirror {
        ids: vec![ids[0]],
        mirror: Some(scene::LiveMirror::on_object_x()),
    })
    .unwrap();
    doc
}

#[test]
fn the_development_format_keeps_smoothing_and_mirrors_on_their_objects() {
    let doc = rich_document_v39();
    let back = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_same(&doc, &back);
    for (id, _) in doc.scene().walk() {
        let (a, b) = (
            doc.scene().object(id).unwrap(),
            back.scene().object(id).unwrap(),
        );
        assert_eq!(a.hardness, b.hardness);
        assert_eq!(a.mirror, b.mirror);
    }
    assert!(
        back.scene()
            .walk()
            .iter()
            .any(|(id, _)| back.scene().object(*id).unwrap().hardness == scene::Hardness::Marked)
    );
}

#[test]
fn development_files_without_the_newer_settings_get_the_standard_ones() {
    // A development file written before a setting existed simply lacks it:
    // here, every object's settings are left out.
    let doc = rich_document();
    let mut model = v39::ProjectV39::from_document(&doc);
    model.objects = Default::default();
    let body = v39::encode(&model).unwrap();
    let mut bytes = header(39, 0, body.len() as u64);
    bytes.extend_from_slice(&body);
    let back = from_bytes(&bytes).unwrap();
    for (id, _) in back.scene().walk() {
        assert_eq!(
            back.scene().object(id).unwrap().hardness,
            scene::Hardness::ByAngle(scene::STANDARD_SMOOTHING_ANGLE)
        );
    }
}

#[test]
fn bad_development_settings_are_refused_in_plain_words() {
    let doc = rich_document();
    let file = |model: &v39::ProjectV39| {
        let body = v39::encode(model).unwrap();
        let mut bytes = header(39, 0, body.len() as u64);
        bytes.extend_from_slice(&body);
        bytes
    };
    let mut model = v39::ProjectV39::from_document(&doc);
    model.objects[0].smoothing = Some(f32::NAN);
    assert!(refused(&file(&model)).to_string().contains("smoothing"));
    let mut model = v39::ProjectV39::from_document(&doc);
    model.objects[0].smoothing = Some(400.0);
    assert!(refused(&file(&model)).to_string().contains("smoothing"));
    let mut model = v39::ProjectV39::from_document(&doc);
    model.objects[0].id = 999_999;
    assert!(refused(&file(&model)).to_string().contains("isn't there"));
    let mut model = v39::ProjectV39::from_document(&doc);
    let again = model.objects[0].clone();
    model.objects.push(again);
    assert!(refused(&file(&model)).to_string().contains("twice"));
}

#[test]
fn random_development_bodies_never_panic() {
    let mut seed = 0x2545_f491_4f6c_dd1du64;
    let doc = rich_document_v39();
    let good = v39::encode(&v39::ProjectV39::from_document(&doc)).unwrap();
    for round in 0..300 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let mut body = good.clone();
        let at = (seed as usize) % body.len();
        body[at] ^= (seed >> 32) as u8 | 1;
        if round % 3 == 0 {
            body.truncate(at);
        }
        let mut bytes = header(39, 0, body.len() as u64);
        bytes.extend_from_slice(&body);
        let _ = from_bytes(&bytes);
    }
}

const SAMPLE_V39: &[u8] = include_bytes!("../../tests/data/sample-v39.avtl");

#[test]
fn a_file_saved_by_format_version_39_opens_with_its_settings() {
    let doc = from_bytes(SAMPLE_V39).expect("the version 39 sample opens");
    assert_same(&rich_document_v39(), &doc);
}

const SAMPLE_V38: &[u8] = include_bytes!("../../tests/data/sample-v38.avtl");

#[test]
fn a_file_saved_by_format_version_38_opens_with_its_snapshots() {
    let doc = from_bytes(SAMPLE_V38).expect("the version 38 sample opens");
    assert_same(&rich_document(), &doc);
    let names: Vec<&str> = doc
        .scene()
        .snapshots()
        .iter()
        .map(|s| s.name.as_str())
        .collect();
    assert_eq!(names, ["Both, unwrapped", "Cube only"]);
    assert_eq!(doc.scene().snapshots()[0].objects.len(), 2);
    let text = format!(
        "{:?}",
        v38::ProjectV38::<v38::EntryV38>::from_document(&doc)
    );
    assert!(text.contains("RestoreUvs"));
}

#[test]
fn bad_snapshots_are_refused_in_plain_words() {
    let doc = rich_document();
    let mut model = v38::ProjectV38::<v38::EntryV38>::from_document(&doc);
    model.snapshots[1].name = model.snapshots[0].name.clone();
    let body = postcard::to_stdvec(&model).unwrap();
    let mut bytes = header(38, 0, body.len() as u64);
    bytes.extend_from_slice(&body);
    assert!(refused(&bytes).to_string().contains("snapshots"));
    let mut model = v38::ProjectV38::<v38::EntryV38>::from_document(&doc);
    model.snapshots[0].objects[0].layout.faces[0].uvs[0] = [f32::NAN, 0.0];
    let body = postcard::to_stdvec(&model).unwrap();
    let mut bytes = header(38, 0, body.len() as u64);
    bytes.extend_from_slice(&body);
    assert!(refused(&bytes).to_string().contains("real number"));
}

const SAMPLE_V37: &[u8] = include_bytes!("../../tests/data/sample-v37.avtl");

#[test]
fn a_file_saved_by_format_version_37_opens_with_its_locks_and_piece_settings() {
    let doc = from_bytes(SAMPLE_V37).expect("the version 37 sample opens");
    assert_same(&rich_document_v37(), &doc);
    assert!(doc.scene().snapshots().is_empty());
    let text = format!(
        "{:?}",
        v35::ProjectV35::<v37::EntryV37>::from_document(&doc)
    );
    assert!(text.contains("SetUvLock"));
    assert!(text.contains("SetPieceSettings"));
}

const SAMPLE_V36: &[u8] = include_bytes!("../../tests/data/sample-v36.avtl");

#[test]
fn a_file_saved_by_format_version_36_opens_with_its_preferences_and_budgets() {
    let doc = from_bytes(SAMPLE_V36).expect("the version 36 sample opens");
    assert_same(&rich_document_v36(), &doc);
    let text = format!(
        "{:?}",
        v35::ProjectV35::<v36::EntryV36>::from_document(&doc)
    );
    assert!(text.contains("SetSeamPreference"));
    assert!(text.contains("SetStretchBudget"));
    assert!(text.contains("budget: 2.5"));
}

const SAMPLE_V35: &[u8] = include_bytes!("../../tests/data/sample-v35.avtl");

#[test]
fn a_file_saved_by_format_version_35_opens_with_every_choice() {
    let doc = from_bytes(SAMPLE_V35).expect("the version 35 sample opens");
    assert_same(&rich_document_v35(), &doc);
    assert_eq!(doc.scene().mending().uneven, 4.5);
    let text = format!(
        "{:?}",
        v35::ProjectV35::<v35::EntryV35>::from_document(&doc)
    );
    assert!(text.contains("strip_angle: 35.0"));
    assert!(text.contains("steps: 150"));
}

const SAMPLE_V34: &[u8] = include_bytes!("../../tests/data/sample-v34.avtl");

#[test]
fn a_file_saved_by_format_version_34_opens_with_its_pins_and_unwrap_way() {
    let doc = from_bytes(SAMPLE_V34).expect("the version 34 sample opens");
    assert_same(&rich_document_v34(), &doc);
    assert_eq!(doc.scene().mending(), scene::MendOptions::default());
    let text = format!(
        "{:?}",
        v33::ProjectV33::<v34::EntryV34>::from_document(&doc)
    );
    assert!(text.contains("SetUvPins"));
    assert!(text.contains("SizeKeeping"));
}

const SAMPLE_V31: &[u8] = include_bytes!("../../tests/data/sample-v31.avtl");

#[test]
fn a_file_saved_by_format_version_31_still_opens_with_its_texture() {
    let doc = from_bytes(SAMPLE_V31).expect("the version 31 sample opens");
    assert_same(&rich_document_v31(), &doc);
    assert_eq!(
        doc.scene().texture(),
        scene::TextureSettings {
            size: 4096,
            density: 1024.0
        }
    );
    let text = format!(
        "{:?}",
        v31::ProjectV31::<v31::EntryV31>::from_document(&doc)
    );
    assert!(text.contains("SetTexelDensity"));
}

#[test]
fn newer_shapes_reopen_as_shapes_with_their_settings() {
    // Decision 0129: version 38's model holds a newer shape as its mesh,
    // and version 39 its settings, so it reopens as the same editable
    // shape, with edits recorded on it finding their points again.
    let mut doc = Document::new();
    let newer: Vec<PrimitiveKind> = PrimitiveKind::ALL
        .into_iter()
        .filter(|k| !k.in_first_files())
        .collect();
    assert_eq!(newer.len(), 11);
    for kind in &newer {
        doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
            .unwrap();
    }
    let ids = doc.scene().children(None).to_vec();
    let of_kind = |doc: &Document, kind: PrimitiveKind| {
        *ids.iter()
            .find(|id| {
                matches!(&doc.scene().object(**id).unwrap().kind,
                    ObjectKind::Primitive(p) if p.kind() == kind)
            })
            .unwrap()
    };
    let (ball, ring) = (
        of_kind(&doc, PrimitiveKind::Icosphere),
        of_kind(&doc, PrimitiveKind::Torus),
    );
    // Changed settings on one, and an edit recorded on another.
    doc.execute(Command::SetPrimitive {
        ids: vec![ball],
        primitive: Primitive::Icosphere(mesh::IcosphereParams {
            radius: 30.0,
            subdivisions: 3,
        }),
    })
    .unwrap();
    doc.execute(Command::Record {
        id: ring,
        operation: Operation::MovePoints {
            points: vec![VertexId::from_raw(0)],
            offset: Vec3::new(0.0, 0.0, 5.0),
            anchors: None,
        },
    })
    .unwrap();
    let back = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    for id in &ids {
        let (before, after) = (
            doc.scene().object(*id).unwrap(),
            back.scene().object(*id).unwrap(),
        );
        assert_eq!(before.kind, after.kind, "{}", before.name);
        assert_eq!(
            doc.scene().evaluated(*id).unwrap().mesh,
            back.scene().evaluated(*id).unwrap().mesh,
            "{}: the same shape, edits included",
            before.name
        );
    }
}

#[test]
fn a_newer_shapes_settings_on_a_group_are_refused() {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Group)).unwrap();
    let mut model = v39::ProjectV39::from_document(&doc);
    let mut object = model.objects.iter().next().unwrap().clone();
    object.shape = v39::ShapeV39::of(&PrimitiveKind::Torus.standard());
    model.objects = std::iter::once(object).collect();
    assert!(model.into_document().is_err());
}

/// Writes a project with every shape side by side, for looking at them in
/// the app: `SHAPES_OUT=path cargo test -p fileio every_shape_side_by_side -- --ignored`.
#[test]
#[ignore = "writes a file for looking at by hand"]
fn every_shape_side_by_side() {
    let Ok(out) = std::env::var("SHAPES_OUT") else {
        return;
    };
    let mut doc = Document::new();
    let mut placed = Vec::new();
    for (k, kind) in PrimitiveKind::ALL.into_iter().enumerate() {
        doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
            .unwrap();
        let id = doc.selection()[0];
        let (col, row) = ((k % 6) as f32, (k / 6) as f32);
        placed.push((
            id,
            Transform {
                position: Vec3::new(col * 200.0 - 500.0, row * 200.0 - 200.0, 0.0),
                ..Transform::default()
            },
        ));
    }
    doc.execute(Command::SetTransforms(placed)).unwrap();
    std::fs::write(out, to_bytes(&doc).unwrap()).unwrap();
}
