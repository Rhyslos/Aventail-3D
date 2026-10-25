//! Levels made by hand while they are edited (decision 0165): a level's
//! own shape is drawn, picked and edited only while it is selected, and
//! its object isn't drawn meanwhile, so the viewport shows the level in
//! the object's place.

use scene::{Document, ObjectId, Scene};

/// The objects whose levels made by hand are being edited now.
#[derive(Debug, Clone, Default)]
pub(crate) struct EditingLevels {
    owners: Vec<ObjectId>,
}

impl EditingLevels {
    /// Whether object `id` (`selected` or not) is left out of the drawing:
    /// a level's shape not being edited, or an object one of whose levels
    /// is.
    pub(crate) fn hides(&self, scene: &Scene, id: ObjectId, selected: bool) -> bool {
        self.owners.contains(&id) || (!selected && scene.is_level_shape(id))
    }
}

/// Which objects' levels are being edited in `document`: those whose
/// level shapes are selected.
pub(crate) fn levels_being_edited(document: &Document) -> EditingLevels {
    let scene = document.scene();
    EditingLevels {
        owners: document
            .selection()
            .into_iter()
            .filter_map(|id| scene.level_owner(id).map(|(owner, _)| owner))
            .collect(),
    }
}

/// The level being edited, if one is: its object, its place (0 for Level
/// 1) and its shape (the first selected level shape).
pub(crate) fn edited_level(document: &Document) -> Option<(ObjectId, usize, ObjectId)> {
    let scene = document.scene();
    document.selection().into_iter().find_map(|id| {
        scene
            .level_owner(id)
            .map(|(owner, level)| (owner, level, id))
    })
}
