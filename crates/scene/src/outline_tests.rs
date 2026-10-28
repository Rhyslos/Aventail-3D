//! Tests for objects' outlines (decision 0201).

use crate::*;

fn cube(doc: &mut Document) -> ObjectId {
    doc.execute(Command::Create(ObjectKind::Primitive(
        PrimitiveKind::Cube.standard(),
    )))
    .unwrap();
    doc.selection()[0]
}

fn outline_of(doc: &Document, id: ObjectId) -> Option<Outline> {
    doc.scene().object(id).unwrap().outline
}

#[test]
fn an_outline_is_set_changed_and_removed_in_undo_steps() {
    let mut doc = Document::new();
    let id = cube(&mut doc);
    assert_eq!(outline_of(&doc, id), None);
    let outline = Outline {
        width: 3.0,
        colour: [1, 2, 3],
    };
    doc.execute(Command::SetOutline {
        ids: vec![id],
        outline: Some(outline),
    })
    .unwrap();
    assert_eq!(outline_of(&doc, id), Some(outline));
    doc.execute(Command::SetOutline {
        ids: vec![id],
        outline: None,
    })
    .unwrap();
    assert_eq!(outline_of(&doc, id), None);
    doc.undo().unwrap();
    assert_eq!(outline_of(&doc, id), Some(outline));
    doc.undo().unwrap();
    assert_eq!(outline_of(&doc, id), None);
    doc.redo().unwrap();
    assert_eq!(outline_of(&doc, id), Some(outline));
}

#[test]
fn widths_are_kept_in_range_and_a_missing_number_refused() {
    let mut doc = Document::new();
    let id = cube(&mut doc);
    for (asked, kept) in [(0.0, 0.05), (1000.0, 50.0), (2.0, 2.0)] {
        doc.execute(Command::SetOutline {
            ids: vec![id],
            outline: Some(Outline {
                width: asked,
                ..Outline::default()
            }),
        })
        .unwrap();
        assert_eq!(outline_of(&doc, id).unwrap().width, kept);
    }
    let refused = doc.execute(Command::SetOutline {
        ids: vec![id],
        outline: Some(Outline {
            width: f32::NAN,
            ..Outline::default()
        }),
    });
    assert!(refused.is_err());
    assert_eq!(outline_of(&doc, id).unwrap().width, 2.0);
}

#[test]
fn opening_a_project_checks_the_outline() {
    let mut doc = Document::new();
    let id = cube(&mut doc);
    let bad = Outline {
        width: -1.0,
        ..Outline::default()
    };
    assert!(doc.clone().with_outline(id, bad).is_err());
    assert!(
        doc.clone()
            .with_outline(ObjectId::from_raw(999), Outline::default())
            .is_err()
    );
    let doc = doc.with_outline(id, Outline::default()).unwrap();
    assert_eq!(outline_of(&doc, id), Some(Outline::default()));
}
