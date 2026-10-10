//! The panels the app window is made of.

use serde::{Deserialize, Serialize};

/// One dockable panel of the app window.
///
/// Each panel appears exactly once in the layout. Panels can be moved and
/// resized but not closed, so none can be lost by accident.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Panel {
    /// The 3D view of the scene.
    Viewport,
    /// Settings of the selected object or the active tool (Cinema 4D's
    /// Attribute Manager).
    Attributes,
    /// The tree of every object in the scene (Cinema 4D's Object Manager).
    Objects,
    /// Typed position, size and rotation of the selection.
    Coordinates,
}

impl Panel {
    /// Every panel, in reading order.
    pub const ALL: [Panel; 4] = [
        Self::Viewport,
        Self::Attributes,
        Self::Objects,
        Self::Coordinates,
    ];

    /// The name shown on the panel's tab.
    pub fn title(self) -> &'static str {
        match self {
            Self::Viewport => "Viewport",
            Self::Attributes => "Attributes",
            Self::Objects => "Objects",
            Self::Coordinates => "Coordinates",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_are_unique_and_match_the_design() {
        let titles: Vec<_> = Panel::ALL.iter().map(|p| p.title()).collect();
        assert_eq!(titles, ["Viewport", "Attributes", "Objects", "Coordinates"]);
    }

    #[test]
    fn panel_names_survive_saving() {
        for panel in Panel::ALL {
            let text = ron::to_string(&panel).unwrap();
            assert_eq!(ron::from_str::<Panel>(&text).unwrap(), panel);
        }
    }
}
