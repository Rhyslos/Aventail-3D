//! Version 94 of the project file ([decision 0254](../../../../docs/decisions/0254-bake-settings-in-the-project.md)):
//! version 93's, with the Bake window's settings
//! ([`super::v43::ProjectV43::bake`], the text the interface wrote).
//! Files before version 94 hold none: the window opens with its standard
//! settings.

use super::v1::Problem;
use scene::Document;
use words::tr;

/// The Bake window's settings to store (empty: the standard ones).
pub(crate) fn bake_of(document: &Document) -> String {
    document.scene().bake_settings().to_string()
}

/// `document` with the stored Bake window settings. Longer than a
/// project keeps is damage. What the text says is the interface's to
/// check (it falls back to the standard settings).
pub(crate) fn with_bake(document: Document, stored: String) -> Result<Document, Problem> {
    if stored.len() > scene::MAX_BAKE_SETTINGS {
        return Err(tr("its bake settings are damaged").into());
    }
    Ok(document.with_bake_settings(stored))
}
