//! The workbench's part in the Levels panel (decision 0165): bringing the
//! panel back beside Attributes when a shape with levels is selected, and
//! the viewport's banner while a level made by hand is edited, whose
//! button stops editing it.

use super::Workbench;
use crate::attributes::levels_panel::{level_name, levels_object};
use crate::objects::DocRequest;
use crate::panel::Panel;

/// The banner's button.
pub(crate) const STOP_EDITING_LEVEL: &str = crate::text!("Stop editing the level");

impl Workbench {
    /// Adds the Levels panel beside Attributes (not in front) when the
    /// shape whose levels it would show has levels of detail on and the
    /// layout has no Levels panel.
    pub(super) fn keep_levels_panel(&mut self) {
        if self.dock.find_tab(&Panel::Levels).is_some() {
            return;
        }
        let with_levels = levels_object(&self.document)
            .and_then(|id| self.document.scene().object(id))
            .is_some_and(|o| o.lods.on);
        if with_levels {
            crate::layout::add_beside_attributes(&mut self.dock, Panel::Levels);
        }
    }

    /// The viewport's banner while a level made by hand is being edited.
    pub(super) fn note_level_banner(&mut self) {
        let banner = crate::drawing::edited_level(&self.document).map(|(owner, k, _)| {
            let name = self
                .document
                .scene()
                .object(owner)
                .map_or("", |o| o.name.as_str());
            crate::trf!(
                "Editing {} of {}: its own shape, made by hand. The object is hidden meanwhile.",
                level_name(k + 1),
                name
            )
        });
        self.viewport.set_banner(banner);
    }

    /// Stops editing the level made by hand that is being edited: its
    /// object is selected again.
    pub(super) fn stop_editing_level(&mut self, requests: &mut Vec<DocRequest>) {
        if let Some((owner, _, _)) = crate::drawing::edited_level(&self.document) {
            requests.push(DocRequest::Select(vec![owner]));
        }
    }
}
