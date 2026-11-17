//! The manual's script reference (`docs/manual/script-reference.md`) is
//! made from scripting's table of calls, with the names from the app's
//! branding (decision 0301): it fails when the page is out of date. Write
//! it again with `UPDATE_REFERENCE=1 cargo test -p app --test
//! script_reference`.

use app::branding::{APP_NAME, SCRIPT_NAME};

#[test]
fn script_reference_matches_the_table() {
    let names = script::Names {
        folder: SCRIPT_NAME.to_string(),
        made_by: APP_NAME.to_string(),
    };
    let page = script::reference_page(&names);
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/manual/script-reference.md");
    if std::env::var_os("UPDATE_REFERENCE").is_some() {
        std::fs::write(&path, &page).unwrap();
        return;
    }
    let kept = std::fs::read_to_string(&path)
        .unwrap_or_default()
        .replace("\r\n", "\n");
    assert!(
        kept == page,
        "docs/manual/script-reference.md is out of date: run \
         UPDATE_REFERENCE=1 cargo test -p app --test script_reference"
    );
}
