//! Tests for version 113: version 111 with the scripts kept with the
//! project (decision 0307).

use super::v111_tests::rich_document_v111;
use super::*;
use scene::{Command, ProjectScript};

/// Everything version 111 stores, with two scripts kept with the
/// project, one LustScript and one Lua. The version 113 sample holds
/// exactly this.
pub(crate) fn rich_document_v113() -> Document {
    let mut doc = rich_document_v111();
    doc.execute(Command::SetScripts(vec![
        ProjectScript {
            name: "Bevel hard edges.lsc".into(),
            text: "-- name: Bevel hard edges\n-- api: 1\n\nend\n".into(),
        },
        ProjectScript {
            name: "Tidy names.lua".into(),
            text: "-- asks: read the project folder\nprint(\"é ✓\")\n".into(),
        },
    ]))
    .unwrap();
    doc
}

#[test]
fn scripts_survive_a_round_trip() {
    let doc = rich_document_v113();
    let bytes = to_bytes(&doc).unwrap();
    assert_eq!(bytes[4..8], FORMAT_VERSION.to_le_bytes());
    let back = from_bytes(&bytes).unwrap();
    assert_eq!(back.scene().scripts(), doc.scene().scripts());
    assert_eq!(
        back.scene().scripts()[1].text,
        doc.scene().scripts()[1].text
    );
}

#[test]
fn a_project_without_scripts_opens_without_any() {
    let doc = rich_document_v111();
    let back = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert!(back.scene().scripts().is_empty());
}

#[test]
fn damaged_script_lists_are_refused_with_a_reason() {
    let good = || v113::scripts_of(&rich_document_v113());
    let mut twice = good();
    let first = twice[0].clone();
    twice.push(first);
    let mut folder = good();
    folder[0].name = "../../outside.lua".into();
    let mut not_a_script = good();
    not_a_script[0].name = "notes.exe".into();
    let mut big = good();
    big[0].text = "x".repeat(scene::MAX_PROJECT_SCRIPT_BYTES + 1);
    let many: crate::limits::Bounded<_> = (0..=scene::MAX_PROJECT_SCRIPTS)
        .map(|i| v113::ScriptV113 {
            name: format!("{i}.lua"),
            text: String::new(),
        })
        .collect();
    for stored in [twice, folder, not_a_script, big, many] {
        let why = v113::with_scripts(Document::new(), stored).unwrap_err();
        assert!(why.to_string().contains("scripts"), "{why}");
    }
}

#[test]
fn flipped_bytes_in_a_file_with_scripts_never_crash_the_reader() {
    let full = to_bytes(&rich_document_v113()).unwrap();
    let mut state = 113u64;
    let mut next = move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        state >> 20
    };
    for _ in 0..200 {
        let mut bytes = full.clone();
        for _ in 0..(next() % 3 + 1) {
            let at = (next() as usize) % bytes.len();
            bytes[at] ^= 1 << (next() % 8);
        }
        if let Ok(doc) = from_bytes(&bytes) {
            assert!(scene::project_script::scripts_are_valid(
                doc.scene().scripts()
            ));
        }
    }
}

#[test]
fn hostile_script_sections_are_refused_not_kept() {
    // A file whose scripts were swapped for ones climbing out of a
    // folder, or for a script far bigger than a script may be, is
    // refused as a whole: nothing of it is half-kept.
    let doc = rich_document_v113();
    let mut model = v43::ProjectV43::from_document(&doc);
    model.scripts[1].name = "C:\\Windows\\evil.lua".into();
    assert!(model.into_document().is_err());
    let mut model = v43::ProjectV43::from_document(&doc);
    model.scripts[0].text = "-".repeat(scene::MAX_PROJECT_SCRIPT_BYTES + 1);
    assert!(model.into_document().is_err());
}

const SAMPLE_V113: &[u8] = include_bytes!("../../tests/data/sample-v113.avtl");

#[test]
fn a_file_saved_by_format_version_113_opens_with_its_scripts() {
    let doc = from_bytes(SAMPLE_V113).expect("the version 113 sample opens");
    let rich = rich_document_v113();
    assert_eq!(doc.scene().scripts(), rich.scene().scripts());
}

#[test]
fn a_version_111_file_opens_without_scripts() {
    const SAMPLE_V111: &[u8] = include_bytes!("../../tests/data/sample-v111.avtl");
    let doc = from_bytes(SAMPLE_V111).unwrap();
    assert!(doc.scene().scripts().is_empty());
}
