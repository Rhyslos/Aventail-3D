//! Tests for version 65: version 64 with the strokes painted on each paint
//! layer (decision 0179).

use super::tests::assert_same;
use super::v64_tests::rich_document_v64;
use super::*;
use paint::{BrushKind, DabRecord, StrokeRecord};
use std::sync::Arc;

fn a_stroke() -> StrokeRecord {
    StrokeRecord {
        kind: BrushKind::Erase,
        color: [1, 2, 3, 255],
        hardness: 0.25,
        flow: 0.75,
        through: true,
        dabs: vec![
            DabRecord {
                at: glam::Vec3::new(1.0, -50.0, 40.0),
                toward: glam::Vec3::NEG_Y,
                radius: 4.5,
                strength: 1.0,
            },
            DabRecord {
                at: glam::Vec3::new(3.0, -50.0, 41.0),
                toward: glam::Vec3::NEG_Y,
                radius: 4.5,
                strength: 1.0,
            },
        ],
        fills: Vec::new(),
    }
}

/// Everything version 64 stores, with a stroke on the texture set's
/// first layer. The version 65 sample holds exactly this.
pub(super) fn rich_document_v65() -> Document {
    let mut doc = rich_document_v64();
    let mut set = (**doc.scene().paint().set("Default").unwrap()).clone();
    set.layers[0].strokes.push(Arc::new(a_stroke()));
    doc.execute(scene::Command::SetTextureSet {
        material: "Default".into(),
        set: Some(set),
    })
    .unwrap();
    doc
}

#[test]
fn strokes_survive_a_round_trip() {
    let doc = rich_document_v65();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], FORMAT_VERSION.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_same(&doc, &back);
    let set = back.scene().paint().set("Default").unwrap();
    assert_eq!(*set.layers[0].strokes[0], a_stroke());
}

const SAMPLE_V65: &[u8] = include_bytes!("../../tests/data/sample-v65.avtl");

#[test]
fn a_file_saved_by_format_version_65_opens_with_its_strokes() {
    let doc = from_bytes(SAMPLE_V65).expect("the version 65 sample opens");
    assert_same(&rich_document_v65(), &doc);
}

#[test]
fn a_version_64_file_opens_without_strokes() {
    const SAMPLE_V64: &[u8] = include_bytes!("../../tests/data/sample-v64.avtl");
    let doc = from_bytes(SAMPLE_V64).unwrap();
    let set = doc.scene().paint().set("Default").unwrap();
    assert!(set.layers.iter().all(|l| l.strokes.is_empty()));
}

#[test]
fn damaged_strokes_are_refused() {
    let good = v43::ProjectV43::from_document(&rich_document_v65());
    let with = |change: &dyn Fn(&mut super::v65::StrokeV65)| {
        let mut model = good.clone();
        if let Some(p) = model.paint.as_mut() {
            change(&mut p.sets[0].layers[0].strokes[0]);
        }
        model.into_document()
    };
    assert!(with(&|s| s.kind = 9).is_err());
    assert!(with(&|s| s.flow = 2.0).is_err());
    assert!(with(&|s| s.dabs[0][6] = f32::NAN).is_err());
    assert!(with(&|s| s.dabs[0][6] = -1.0).is_err());
    assert!(with(&|_| ()).is_ok());
}
