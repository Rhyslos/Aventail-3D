//! Tests for painting vertex colours as a recorded edit (decision 0205).

use crate::*;

fn cube(doc: &mut Document) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Cube.standard(),
    )))
    .unwrap();
    doc.selection()[0]
}

fn shown_colours(doc: &Document, id: ObjectId) -> Option<Vec<[u8; 4]>> {
    mesh::colours::colours(&doc.scene().evaluated(id).unwrap().mesh)
}

#[test]
fn painted_colours_replay_with_the_shape_and_undo() {
    let mut doc = Document::new();
    let id = cube(&mut doc);
    assert!(shown_colours(&doc, id).is_none());
    let shape = doc.scene().evaluated(id).unwrap().mesh;
    let (face, points) = shape.faces().next().map(|(f, l)| (f, l.to_vec())).unwrap();
    let corners: Vec<_> = points
        .iter()
        .map(|p| (face, *p, [0, 128, 0, 255]))
        .collect();
    doc.execute(Command::Record {
        id,
        operation: Operation::PaintColours { corners },
    })
    .unwrap();
    let painted = shown_colours(&doc, id).unwrap();
    assert_eq!(
        painted.iter().filter(|c| **c == [0, 128, 0, 255]).count(),
        points.len()
    );
    doc.undo().unwrap();
    assert!(shown_colours(&doc, id).is_none());
    doc.redo().unwrap();
    assert_eq!(shown_colours(&doc, id).unwrap(), painted);
}

#[test]
fn colours_survive_a_later_edit_that_keeps_the_polygons() {
    let mut doc = Document::new();
    let id = cube(&mut doc);
    let shape = doc.scene().evaluated(id).unwrap().mesh;
    let (face, points) = shape.faces().next().map(|(f, l)| (f, l.to_vec())).unwrap();
    doc.execute(Command::Record {
        id,
        operation: Operation::PaintColours {
            corners: vec![(face, points[0], [9, 9, 9, 255])],
        },
    })
    .unwrap();
    doc.execute(Command::Record {
        id,
        operation: Operation::MovePoints {
            points: points.clone(),
            offset: glam::Vec3::Z,
            anchors: None,
        },
    })
    .unwrap();
    let shape = doc.scene().evaluated(id).unwrap().mesh;
    assert_eq!(
        mesh::colours::colour_at(&shape, face, points[0]),
        Some([9, 9, 9, 255])
    );
}
