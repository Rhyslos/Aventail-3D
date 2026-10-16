//! The panels the app window is made of.

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
}

impl Panel {
    /// Every panel that is always in a layout, in reading order.
    pub const ALL: [Panel; 5] = [
        Self::Viewport,
        Self::Attributes,
        Self::Objects,
        Self::Coordinates,
        Self::EditLayers,
    ];

    /// Panels added after the first release: a saved layout without them
    /// gets them added beside Objects rather than being reset.
    pub const ADDED_LATER: [Panel; 1] = [Self::EditLayers];

    /// Panels a layout may leave out: shown when wanted.
    pub const OPTIONAL: [Panel; 1] = [Self::UvEditor];

    /// The name shown on the panel's tab.
    pub fn title(self) -> &'static str {
        match self {
            Self::Viewport => "Viewport",
            Self::Attributes => "Attributes",
            Self::Objects => "Objects",
            Self::Coordinates => "Coordinates",
            Self::EditLayers => "Edit layers",
            Self::UvEditor => "UV Editor",
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
            [
                "Viewport",
                "Attributes",
                "Objects",
                "Coordinates",
                "Edit layers"
            ]
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
