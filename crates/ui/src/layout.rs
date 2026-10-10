//! The arrangement of panels: the default, and checks on saved arrangements.

use crate::panel::Panel;
use egui_dock::{DockState, NodeIndex};

/// Share of the window width given to the viewport column (the right
/// column with Attributes and Objects gets the rest).
const MAIN_AREA_WIDTH: f32 = 0.75;
/// Share of the right column given to Attributes (Objects gets the rest).
const ATTRIBUTES_HEIGHT: f32 = 0.6;
/// Share of the viewport column given to the viewport (Coordinates gets the rest).
const VIEWPORT_HEIGHT: f32 = 0.87;

/// The standard arrangement from DESIGN.md §5.1, with the right column
/// swapped as the project lead prefers (Attributes on top):
///
/// ```text
/// ┌─────────────────────────────┬──────────────┐
/// │                             │  Attributes  │
/// │          Viewport           │              │
/// │                             ├──────────────┤
/// ├─────────────────────────────┤   Objects    │
/// │         Coordinates         │              │
/// └─────────────────────────────┴──────────────┘
/// ```
pub fn default_layout() -> DockState<Panel> {
    let mut dock = DockState::new(vec![Panel::Viewport]);
    let tree = dock.main_surface_mut();
    let [main_area, right_column] =
        tree.split_right(NodeIndex::root(), MAIN_AREA_WIDTH, vec![Panel::Attributes]);
    tree.split_below(right_column, ATTRIBUTES_HEIGHT, vec![Panel::Objects]);
    tree.split_below(main_area, VIEWPORT_HEIGHT, vec![Panel::Coordinates]);
    dock
}

/// True if every panel appears exactly once.
///
/// A saved layout could be damaged or come from a version with different
/// panels; such layouts are replaced by the default.
pub fn is_complete(dock: &DockState<Panel>) -> bool {
    let mut seen = Vec::new();
    for (_, panel) in dock.iter_all_tabs() {
        if seen.contains(panel) {
            return false;
        }
        seen.push(*panel);
    }
    Panel::ALL.iter().all(|p| seen.contains(p))
}

/// Uses a saved layout if it is complete, otherwise the default.
/// Returns a note for the status bar when the saved layout was not usable.
pub fn restore_or_default(saved: Option<DockState<Panel>>) -> (DockState<Panel>, Option<String>) {
    match saved {
        None => (default_layout(), None),
        Some(dock) if is_complete(&dock) => (dock, None),
        Some(_) => (
            default_layout(),
            Some("The saved panel layout was incomplete, so the standard layout is used.".into()),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Which panels share a leaf (tab group).
    fn leaves(dock: &DockState<Panel>) -> Vec<Vec<Panel>> {
        dock.main_surface()
            .iter()
            .filter_map(|node| node.tabs().map(|tabs| tabs.to_vec()))
            .collect()
    }

    #[test]
    fn default_layout_has_every_panel_once() {
        assert!(is_complete(&default_layout()));
    }

    #[test]
    fn default_layout_gives_each_panel_its_own_area() {
        let mut groups = leaves(&default_layout());
        groups.sort_by_key(|g| g.first().map(|p| p.title()));
        assert_eq!(
            groups,
            vec![
                vec![Panel::Attributes],
                vec![Panel::Coordinates],
                vec![Panel::Objects],
                vec![Panel::Viewport],
            ]
        );
    }

    #[test]
    fn default_layout_has_no_floating_windows() {
        assert_eq!(default_layout().iter_surfaces().count(), 1);
    }

    #[test]
    fn missing_panel_is_incomplete() {
        let dock = DockState::new(vec![Panel::Viewport, Panel::Objects]);
        assert!(!is_complete(&dock));
    }

    #[test]
    fn duplicate_panel_is_incomplete() {
        let mut dock = default_layout();
        dock.main_surface_mut()
            .split_right(NodeIndex::root(), 0.5, vec![Panel::Objects]);
        assert!(!is_complete(&dock));
    }

    #[test]
    fn restore_keeps_a_complete_saved_layout() {
        // A user's own arrangement: two groups instead of four, but every
        // panel still present exactly once.
        let mut saved = DockState::new(vec![Panel::Viewport, Panel::Coordinates]);
        saved.main_surface_mut().split_left(
            NodeIndex::root(),
            0.3,
            vec![Panel::Objects, Panel::Attributes],
        );
        let (dock, note) = restore_or_default(Some(saved));
        assert!(note.is_none());
        assert_eq!(leaves(&dock).len(), 2);
    }

    #[test]
    fn restore_replaces_a_broken_layout_with_a_note() {
        let broken = DockState::new(vec![Panel::Objects]);
        let (dock, note) = restore_or_default(Some(broken));
        assert!(is_complete(&dock));
        assert!(note.is_some_and(|n| n.contains("standard layout")));
    }

    #[test]
    fn restore_without_a_saved_layout_is_silent() {
        let (dock, note) = restore_or_default(None);
        assert!(is_complete(&dock));
        assert!(note.is_none());
    }

    #[test]
    fn layout_survives_saving_and_loading() {
        // eframe stores the layout as RON text between sessions.
        let original = default_layout();
        let text = ron::to_string(&original).unwrap();
        let loaded: DockState<Panel> = ron::from_str(&text).unwrap();
        assert!(is_complete(&loaded));
        assert_eq!(leaves(&loaded), leaves(&original));
    }

    #[test]
    fn damaged_saved_text_does_not_load() {
        assert!(ron::from_str::<DockState<Panel>>("(surfaces: [garbage").is_err());
    }
}
