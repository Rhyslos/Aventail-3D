//! Tests for version 91: version 90 with unwrap recipes.

use super::v90_tests::rich_document_v90;
use super::*;
use scene::{Command, UnwrapRecipe};

/// Everything version 90 stores, with two unwrap recipes. The version 91
/// sample holds exactly this.
pub(super) fn rich_document_v91() -> Document {
    let mut doc = rich_document_v90();
    doc.execute(Command::SetRecipes(vec![
        UnwrapRecipe {
            name: "Kit walls".into(),
            settings: "(padding: 8, auto_steps: (lightmap: false))".into(),
        },
        UnwrapRecipe {
            name: "Props".into(),
            settings: "()".into(),
        },
    ]))
    .unwrap();
    doc
}

#[test]
fn recipes_survive_a_round_trip() {
    let doc = rich_document_v91();
    let back = from_bytes(&to_bytes(&doc).unwrap()).unwrap();
    assert_eq!(back.scene().recipes(), doc.scene().recipes());
    assert_eq!(back.scene().recipes().len(), 2);
}

#[test]
fn damaged_recipes_are_refused() {
    let twice = super::v91::RecipeV91 {
        name: "A".into(),
        settings: String::new(),
    };
    let stored: crate::limits::Bounded<_> = vec![twice.clone(), twice].into_iter().collect();
    assert!(super::v91::with_recipes(Document::new(), stored).is_err());
    let unnamed: crate::limits::Bounded<_> = vec![super::v91::RecipeV91 {
        name: "  ".into(),
        settings: String::new(),
    }]
    .into_iter()
    .collect();
    assert!(super::v91::with_recipes(Document::new(), unnamed).is_err());
}

const SAMPLE_V91: &[u8] = include_bytes!("../../tests/data/sample-v91.avtl");

#[test]
fn a_file_saved_by_format_version_91_opens_with_its_recipes() {
    let doc = from_bytes(SAMPLE_V91).expect("the version 91 sample opens");
    assert_eq!(doc.scene().recipes(), rich_document_v91().scene().recipes());
}

#[test]
fn a_version_90_file_holds_no_recipes() {
    const SAMPLE_V90: &[u8] = include_bytes!("../../tests/data/sample-v90.avtl");
    let doc = from_bytes(SAMPLE_V90).unwrap();
    assert!(doc.scene().recipes().is_empty());
}
