//! The panels the app window is made of.

use crate::text::tr;
use serde::{Deserialize, Serialize};

/// One dockable panel of the app window.
///
/// Each panel appears once in a layout. Panels can be moved and resized
/// but not closed, so none can be lost by accident; the UV Editor is
/// shown and put away from the Window menu (decision 0096).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Panel {
    /// The 3D view of the scene.
    Viewport,
    /// Settings of the selected object or the active tool.
    Attributes,
    /// The tree of every object in the scene.
    Objects,
    /// Typed position, size and rotation of the selection.
    Coordinates,
    /// The selected object's edit layers and their edits (decision 0091).
    EditLayers,
    /// The selected objects' UVs, flat on the texture square (decision
    /// 0096). In the UV Edit layout; in others when the Window menu shows
    /// it.
    UvEditor,
    /// The automatic UV tools' settings (issue #115), beside Attributes in
    /// the UV Edit layout; elsewhere when the Window menu shows it.
    UvSettings,
    /// The selected object's levels of detail (decision 0165), a tab
    /// beside Attributes.
    Levels,
    /// The layers of a material's texture set (decision 0181), under
    /// Attributes in the Paint layout.
    PaintLayers,
    /// The brush bar (decision 0185), in Coordinates' place in the Paint
    /// layout.
    Brushes,
    /// The paint brush's settings (decision 0213), a tab beside
    /// Attributes in the Paint layout, so Attributes is only about the
    /// object.
    Paint,
}

impl Panel {
    /// Every panel that is always in a layout, in reading order.
    pub const ALL: [Panel; 4] = [
        Self::Viewport,
        Self::Attributes,
        Self::Objects,
        Self::Coordinates,
    ];

    /// Panels added after the first release: a saved layout without them
    /// gets them added beside Objects rather than being reset.
    pub const ADDED_LATER: [Panel; 0] = [];

    /// Panels a layout may leave out: shown when wanted. Edit layers
    /// stays out of the way until a layer is added (decision 0126).
    /// Levels is beside Attributes in the standard layout, and comes back
    /// there when wanted (decision 0165).
    /// Paint layers is in the Paint layout (decision 0181), and so is
    /// Paint (decision 0213).
    pub const OPTIONAL: [Panel; 7] = [
        Self::UvEditor,
        Self::EditLayers,
        Self::UvSettings,
        Self::Levels,
        Self::PaintLayers,
        Self::Brushes,
        Self::Paint,
    ];

    /// The name shown on the panel's tab.
    pub fn title(self) -> &'static str {
        match self {
            Self::Viewport => tr("Viewport"),
            Self::Attributes => tr("Attributes"),
            Self::Objects => tr("Objects"),
            Self::Coordinates => tr("Coordinates"),
            Self::EditLayers => tr("Edit layers"),
            Self::UvEditor => tr("UV Editor"),
            Self::UvSettings => tr("UV settings"),
            Self::Levels => tr("Levels"),
            Self::PaintLayers => tr("Paint layers"),
            Self::Brushes => tr("Brushes"),
            Self::Paint => tr("Paint"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_are_unique_and_match_the_design() {
        let titles: Vec<_> = Panel::ALL.iter().map(|p| p.title()).collect();
        assert_eq!(
            titles,
            ["Viewport", "Attributes", "Objects", "Coordinates",]
        );
    }

    #[test]
    fn panel_names_survive_saving() {
        for panel in Panel::ALL {
            let text = ron::to_string(&panel).unwrap();
            assert_eq!(ron::from_str::<Panel>(&text).unwrap(), panel);
        }
    }
}
