//! Tests for version 61: version 60 with the project's kit grid (decision
//! 0160).

use super::tests::{assert_same, header};
use super::v60_tests::rich_document_v60;
use super::*;
use scene::{Command, KitGrid};

/// The sample's kit grid: not one of the menu's choices.
const ODD_STEP: f32 = 250.0;

/// Everything version 60 stores, with a kit grid of 2.5 m and Kit-sized
/// shapes on: the version 61 sample holds exactly this.
pub(super) fn rich_document_v61() -> Document {
    let mut doc = rich_document_v60();
    doc.execute(Command::SetKitGrid(KitGrid {
        step: ODD_STEP,
        sized_shapes: true,
    }))
    .unwrap();
    doc
}

#[test]
fn the_kit_grid_survives_a_round_trip() {
    let doc = rich_document_v61();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], FORMAT_VERSION.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_same(&doc, &back);
    assert_eq!(back.scene().kit_grid().step, ODD_STEP);
    assert!(back.scene().kit_grid().sized_shapes);
    // Kit-sized shapes on with the standard step is stored too.
    let mut doc = rich_document_v60();
    let sized = KitGrid {
        sized_shapes: true,
        ..KitGrid::default()
    };
    doc.execute(Command::SetKitGrid(sized)).unwrap();
    let back = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_eq!(back.scene().kit_grid(), sized);
}

#[test]
fn the_standard_kit_grid_is_not_stored() {
    let doc = rich_document_v60();
    assert_eq!(v43::ProjectV43::from_document(&doc).kit_grid, None);
    let back = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_eq!(back.scene().kit_grid(), KitGrid::default());
    assert_eq!(
        v43::ProjectV43::from_document(&rich_document_v61())
            .kit_grid
            .map(|k| k.step),
        Some(ODD_STEP)
    );
}

const SAMPLE_V61: &[u8] = include_bytes!("../../tests/data/sample-v61.avtl");

#[test]
fn a_file_saved_by_format_version_61_opens_with_its_kit_grid() {
    let doc = from_bytes(SAMPLE_V61).expect("the version 61 sample opens");
    assert_same(&rich_document_v61(), &doc);
    assert_eq!(doc.scene().kit_grid().step, ODD_STEP);
    assert!(doc.scene().kit_grid().sized_shapes);
}

const SAMPLE_V60: &[u8] = include_bytes!("../../tests/data/sample-v60.avtl");

#[test]
fn older_files_open_with_a_one_metre_kit_grid() {
    let doc = from_bytes(SAMPLE_V60).unwrap();
    assert_eq!(doc.scene().kit_grid(), KitGrid::default());
    assert_same(&rich_document_v60(), &doc);
    let again = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_same(&doc, &again);
    assert_eq!(again.scene().kit_grid(), KitGrid::default());
}

#[test]
fn a_kit_grid_out_of_limits_is_refused() {
    for step in [0.0, -100.0, 0.25, f32::NAN, f32::INFINITY, 1e9] {
        let mut model = v43::ProjectV43::from_document(&rich_document_v61());
        model.kit_grid = Some(v61::KitGridV61 {
            step,
            sized_shapes: true,
        });
        assert!(model.into_document().is_err(), "{step}");
    }
}

#[test]
fn an_older_app_is_told_it_needs_updating() {
    let error = super::tests::refused(&header(FORMAT_VERSION + 1, 0, 0));
    assert!(matches!(error, ProjectError::TooNew { .. }));
}

#[test]
fn random_version_61_bodies_never_panic() {
    let mut seed = 0x61be_4d70_7a9e_0160_u64;
    let good = v43::encode(&v43::ProjectV43::from_document(&rich_document_v61())).unwrap();
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
        let mut bytes = header(61, 0, body.len() as u64);
        bytes.extend_from_slice(&body);
        let _ = from_bytes(&bytes);
    }
}
