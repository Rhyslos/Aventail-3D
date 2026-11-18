//! Version 113 of the project file ([decision 0307](../../../../docs/decisions/0307-keeping-sharing-and-permissions.md)):
//! version 112's, with the scripts kept with the
//! project ([`super::v43::ProjectV43::scripts`], stored as [`ScriptV113`]:
//! a file name and its text). Files before version 113 hold none; an
//! older app would open the project without them and lose them on
//! saving, so the version goes up.
//!
//! The scripts are only text here: nothing in them runs while a project
//! opens, and they don't run afterwards until the person opening it turns
//! them on.

use super::v1::Problem;
use crate::limits::Bounded;
use scene::{Document, ProjectScript};
use serde::{Deserialize, Serialize};
use words::tr;

/// A script as stored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ScriptV113 {
    /// Its file's name, with its ending.
    pub(crate) name: String,
    /// Its text.
    pub(crate) text: String,
}

/// The scripts to store.
pub(crate) fn scripts_of(document: &Document) -> Bounded<ScriptV113> {
    document
        .scene()
        .scripts()
        .iter()
        .map(|s| ScriptV113 {
            name: s.name.clone(),
            text: s.text.clone(),
        })
        .collect()
}

/// `document` with the stored scripts, checked: too many, a name that
/// isn't a script's (or names a folder), two of one name, or too much
/// text in one, is damage.
pub(crate) fn with_scripts(
    document: Document,
    stored: Bounded<ScriptV113>,
) -> Result<Document, Problem> {
    if stored.len() > scene::MAX_PROJECT_SCRIPTS {
        return Err(tr("its scripts are damaged").into());
    }
    let scripts: Vec<ProjectScript> = stored
        .into_iter()
        .map(|s| ProjectScript {
            name: s.name,
            text: s.text,
        })
        .collect();
    if !scene::project_script::scripts_are_valid(&scripts) {
        return Err(tr("its scripts are damaged").into());
    }
    Ok(document.with_scripts(scripts))
}
