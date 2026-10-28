//! Tests for version 82: version 81 with the live preview switch
//! (decision 0212).

use super::v81_tests::rich_document_v81;
use super::*;

/// Everything version 81 stores, exporting live into a game project
/// folder. The version 82 sample holds exactly this.
pub(super) fn rich_document_v82() -> Document {
    let mut doc = rich_document_v81();
    doc.execute(scene::Command::SetExportTarget(Some(scene::ExportTarget {
        folder: "/games/crates/models".into(),
        on_save: false,
        live: true,
    })))
    .unwrap();
    doc
}

#[test]
fn the_live_switch_survives_a_round_trip() {
    let doc = rich_document_v82();
    let back = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_eq!(back.scene().export_target(), doc.scene().export_target());
    assert!(back.scene().export_target().unwrap().live);
}

const SAMPLE_V82: &[u8] = include_bytes!("../../tests/data/sample-v82.avtl");

#[test]
fn a_file_saved_by_format_version_82_opens_live() {
    let doc = from_bytes(SAMPLE_V82).expect("the version 82 sample opens");
    assert_eq!(
        doc.scene().export_target(),
        rich_document_v82().scene().export_target()
    );
}
