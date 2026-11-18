//! Tests for Remesh for sculpting's command and the shapes it merges
//! (decision 0312).

use crate::*;
use glam::Vec3;
use mesh::remesh::{Progress, SculptRemeshOptions, remesh_for_sculpting};
use std::sync::Arc;

/// A document with a standard sphere and a cube beside it overlapping it,
/// and their ids.
fn two_shapes() -> (Document, ObjectId, ObjectId) {
    let mut doc = Document::new();
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Sphere.standard(),
    )))
    .unwrap();
    let ball = doc.selection()[0];
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Cube.standard(),
    )))
    .unwrap();
    let cube = doc.selection()[0];
    let mut moved = doc.scene().object(cube).unwrap().transform;
    moved.position = Vec3::new(40.0, 0.0, 0.0);
    doc.execute(Command::SetTransforms(vec![(cube, moved)]))
        .unwrap();
    (doc, ball, cube)
}

/// What the interface sends: `joined` merged into `id` and remeshed.
fn remeshed(doc: &Document, id: ObjectId, joined: &[ObjectId]) -> Command {
    let (shape, materials) = doc.scene().joined_shape(id, joined).unwrap();
    let out = remesh_for_sculpting(
        &shape,
        &SculptRemeshOptions {
            voxel_size: 5.0,
            ..Default::default()
        },
        None,
        &Progress::default(),
    )
    .unwrap();
    Command::RemeshForSculpting {
        id,
        shape: Arc::new(out.mesh),
        materials,
        joined: joined.to_vec(),
    }
}

#[test]
fn the_shape_becomes_the_base_under_a_fresh_sculpt_layer() {
    let (mut doc, ball, _) = two_shapes();
    let before = doc.scene().object(ball).unwrap().clone();
    let command = remeshed(&doc, ball, &[]);
    let Command::RemeshForSculpting { shape, .. } = &command else {
        unreachable!()
    };
    let shape = Arc::clone(shape);
    assert_eq!(command.describe(doc.scene()), "Remesh for sculpting");
    doc.execute(command).unwrap();
    let object = doc.scene().object(ball).unwrap();
    assert!(matches!(&object.kind, ObjectKind::Mesh(m) if Arc::ptr_eq(m, &shape)));
    let layers = object.stack.layers();
    assert_eq!(layers.len(), 1);
    assert!(layers[0].is_sculpt());
    let sculpt = layers[0].sculpt().unwrap();
    assert!(sculpt.fits(&shape));
    // Nothing sculpted yet: the shape is the new base exactly.
    assert_eq!(*doc.scene().evaluated(ball).unwrap().mesh, *shape);
    assert_eq!(doc.selection(), vec![ball]);
    // Sculpting goes into the fresh layer.
    assert_eq!(doc.scene().sculpt_target(ball).unwrap().layer, Some(0));
    // One undo step brings the primitive and its stack back exactly.
    doc.undo().unwrap();
    assert_eq!(*doc.scene().object(ball).unwrap(), before);
    doc.redo().unwrap();
    assert!(doc.scene().object(ball).unwrap().stack.layers()[0].is_sculpt());
}

#[test]
fn selected_shapes_merge_into_one_and_the_others_go() {
    let (mut doc, ball, cube) = two_shapes();
    // Something inside the cube takes its place when it goes.
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Cylinder.standard(),
    )))
    .unwrap();
    let inside = doc.selection()[0];
    doc.execute(Command::Move {
        ids: vec![inside],
        parent: Some(cube),
        index: 0,
    })
    .unwrap();
    let ball_box = doc.scene().evaluated(ball).unwrap().mesh.bounds().unwrap();
    doc.execute(remeshed(&doc, ball, &[cube])).unwrap();
    assert!(!doc.scene().contains(cube));
    assert!(doc.scene().contains(inside));
    // Wider than the ball alone: the cube is part of it now.
    let merged = doc.scene().evaluated(ball).unwrap().mesh.bounds().unwrap();
    assert!(merged.max.x > ball_box.max.x + 20.0, "{merged:?}");
    let report = mesh::MeshReport::new(&doc.scene().evaluated(ball).unwrap().mesh);
    assert_eq!(report.pieces, 1);
    assert_eq!(report.border_edges, 0);
    doc.undo().unwrap();
    assert!(doc.scene().contains(cube));
    assert_eq!(doc.scene().parent(inside), Some(cube));
}

#[test]
fn joined_shapes_bring_their_materials_by_name() {
    let (mut doc, ball, cube) = two_shapes();
    let red = Material {
        name: "Red".into(),
        color: [1.0, 0.0, 0.0],
    };
    doc.execute(Command::SetMaterials {
        id: cube,
        materials: vec![red.clone()],
    })
    .unwrap();
    let (_, materials) = doc.scene().joined_shape(ball, &[cube]).unwrap();
    let materials = materials.unwrap();
    assert!(materials.iter().any(|m| m.name == "Red"));
    doc.execute(remeshed(&doc, ball, &[cube])).unwrap();
    assert!(doc.scene().object(ball).unwrap().materials.contains(&red));
    // Without materials anywhere, none are given.
    let (doc, ball, cube) = two_shapes();
    assert!(doc.scene().joined_shape(ball, &[cube]).unwrap().1.is_none());
}

#[test]
fn what_cant_be_sculpted_is_refused() {
    let (mut doc, ball, cube) = two_shapes();
    let shape = doc.scene().evaluated(ball).unwrap().mesh;
    let command = |id, joined: Vec<ObjectId>, shape: Arc<Mesh>| Command::RemeshForSculpting {
        id,
        shape,
        materials: None,
        joined,
    };
    // A group.
    doc.execute(Command::Group(vec![cube])).unwrap();
    let group = doc.scene().parent(cube).unwrap();
    assert_eq!(
        doc.execute(command(group, vec![], Arc::clone(&shape))),
        Err(SceneError::NoShape)
    );
    // An empty shape, itself among the joined, an unknown object.
    assert_eq!(
        doc.execute(command(ball, vec![], Arc::new(Mesh::new()))),
        Err(SceneError::NoShape)
    );
    assert_eq!(
        doc.execute(command(ball, vec![ball], Arc::clone(&shape))),
        Err(SceneError::NestedInItself)
    );
    doc.execute(Command::Delete(vec![cube])).unwrap();
    assert_eq!(
        doc.execute(command(cube, vec![], shape)),
        Err(SceneError::UnknownObject(cube))
    );
    assert!(doc.scene().joined_shape(group, &[]).is_none());
}
