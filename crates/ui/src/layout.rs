//! The arrangement of panels: the default, and checks on saved arrangements.

use crate::panel::Panel;
use egui_dock::{DockState, Node, NodeIndex, SurfaceIndex};

/// Share of the window width given to the viewport column (the right
/// column with Attributes and Objects gets the rest).
const MAIN_AREA_WIDTH: f32 = 0.75;
/// Share of the right column given to Attributes (Objects gets the rest).
const ATTRIBUTES_HEIGHT: f32 = 0.6;
/// Share of the viewport column given to the viewport (Coordinates gets the
/// rest) until the window's real size is known; see [`fit_coordinates`].
const VIEWPORT_HEIGHT: f32 = 0.85;

/// The height Coordinates needs to show all its rows, tab bar included, in
/// logical pixels. The standard layout gives it exactly this, so no empty
/// space is left under the last row.
pub const COORDINATES_HEIGHT: f32 = 140.0;

/// The smallest a panel can be dragged to, in logical pixels: a tab bar
/// and a little room.
pub const MIN_PANEL_SIZE: f32 = 60.0;

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
///
/// Edit layers is a tab beside Objects (decision 0091).
pub fn default_layout() -> DockState<Panel> {
    let mut dock = DockState::new(vec![Panel::Viewport]);
    let tree = dock.main_surface_mut();
    let [main_area, right_column] =
        tree.split_right(NodeIndex::root(), MAIN_AREA_WIDTH, vec![Panel::Attributes]);
    tree.split_below(
        right_column,
        ATTRIBUTES_HEIGHT,
        vec![Panel::Objects, Panel::EditLayers],
    );
    tree.split_below(main_area, VIEWPORT_HEIGHT, vec![Panel::Coordinates]);
    dock
}

/// Adds the panels a saved layout from an earlier version doesn't have
/// ([`Panel::ADDED_LATER`]) as tabs beside Objects (or beside the first
/// panel found), leaving the tab that was showing in front. Returns
/// whether any was added.
pub fn add_later_panels(dock: &mut DockState<Panel>) -> bool {
    let mut added = false;
    for panel in Panel::ADDED_LATER {
        if dock.find_tab(&panel).is_some() {
            continue;
        }
        let beside = dock
            .find_tab(&Panel::Objects)
            .or_else(|| dock.find_tab_from(|p| *p != Panel::Viewport))
            .or_else(|| dock.find_tab(&Panel::Viewport));
        let Some(beside) = beside else {
            continue;
        };
        if let Ok(leaf) = dock.leaf_mut(beside.node_path()) {
            let showing = leaf.active;
            leaf.append_tab(panel);
            leaf.active = showing;
            added = true;
        }
    }
    added
}

/// Brings `panel`'s tab to the front of its group.
pub fn show_tab(dock: &mut DockState<Panel>, panel: Panel) {
    if let Some(path) = dock.find_tab(&panel) {
        let _ = dock.set_active_tab(path);
    }
}

/// Hides the viewport's tab, because the viewport's own menu line takes
/// its place, as in the approved mockup. Run every frame, so it also
/// applies to saved layouts. If the user has stacked another panel with
/// the viewport, that group keeps its tabs, so no panel is ever hidden.
pub fn hide_viewport_tab(dock: &mut DockState<Panel>) {
    for (_, leaf) in dock.iter_leaves_mut() {
        leaf.tab_bar_hidden = leaf.tabs == [Panel::Viewport];
    }
}

/// Gives Coordinates exactly [`COORDINATES_HEIGHT`] when it sits directly
/// below another panel, as in the standard layout.
///
/// Splits are stored as shares of the space, so the right share depends on
/// the window's height; it is set once the panels have been drawn and their
/// sizes are known. Returns `true` if the split was adjusted, `false` if
/// Coordinates is elsewhere (the user's own arrangement is left alone) or
/// the sizes are not known yet.
pub fn fit_coordinates(dock: &mut DockState<Panel>) -> bool {
    let Some(path) = dock.find_tab(&Panel::Coordinates) else {
        return false;
    };
    let node = path.node_path();
    if node.surface != SurfaceIndex::main() || !node.node.is_right() {
        return false;
    }
    let Some(parent) = node.node.parent() else {
        return false;
    };
    let Node::Vertical(split) = &mut dock.main_surface_mut()[parent] else {
        return false;
    };
    let height = split.rect.height();
    if !height.is_finite() || height < COORDINATES_HEIGHT + MIN_PANEL_SIZE {
        return false;
    }
    split.fraction = 1.0 - COORDINATES_HEIGHT / height;
    true
}

/// True if no panel appears twice.
fn no_repeats(dock: &DockState<Panel>) -> bool {
    let mut seen = Vec::new();
    dock.iter_all_tabs().all(|(_, panel)| {
        let new = !seen.contains(panel);
        seen.push(*panel);
        new
    })
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
        Some(mut dock) if no_repeats(&dock) => {
            add_later_panels(&mut dock);
            if is_complete(&dock) {
                (dock, None)
            } else {
                (
                    default_layout(),
                    Some(
                        "The saved panel layout was incomplete, so the standard layout is used."
                            .into(),
                    ),
                )
            }
        }
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
                vec![Panel::Objects, Panel::EditLayers],
                vec![Panel::Viewport],
            ]
        );
    }

    /// The split above Coordinates in the standard layout.
    fn coordinates_split(dock: &mut DockState<Panel>) -> &mut egui_dock::SplitNode {
        let node = dock.find_tab(&Panel::Coordinates).unwrap().node_path().node;
        match &mut dock.main_surface_mut()[node.parent().unwrap()] {
            Node::Vertical(split) => split,
            _ => panic!("Coordinates should sit below the viewport"),
        }
    }

    #[test]
    fn coordinates_get_exactly_their_height_once_sizes_are_known() {
        let mut dock = default_layout();
        // Before the first frame, sizes are unknown: nothing changes.
        assert!(!fit_coordinates(&mut dock));
        coordinates_split(&mut dock).rect =
            egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1000.0, 800.0));
        assert!(fit_coordinates(&mut dock));
        let fraction = coordinates_split(&mut dock).fraction;
        assert!((800.0 * (1.0 - fraction) - COORDINATES_HEIGHT).abs() < 0.01);
    }

    #[test]
    fn a_tiny_window_keeps_the_viewport_usable() {
        let mut dock = default_layout();
        coordinates_split(&mut dock).rect =
            egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1000.0, 150.0));
        assert!(!fit_coordinates(&mut dock));
    }

    #[test]
    fn coordinates_moved_elsewhere_are_left_alone() {
        // The user stacked every panel as tabs in one group.
        let mut dock = DockState::new(Panel::ALL.to_vec());
        assert!(!fit_coordinates(&mut dock));
    }

    fn hidden_tabs(dock: &DockState<Panel>) -> Vec<Vec<Panel>> {
        dock.iter_leaves()
            .filter(|(_, leaf)| leaf.tab_bar_hidden)
            .map(|(_, leaf)| leaf.tabs.clone())
            .collect()
    }

    #[test]
    fn only_the_viewport_loses_its_tab() {
        let mut dock = default_layout();
        hide_viewport_tab(&mut dock);
        assert_eq!(hidden_tabs(&dock), vec![vec![Panel::Viewport]]);
    }

    #[test]
    fn a_viewport_sharing_its_group_keeps_its_tabs() {
        let mut dock = DockState::new(vec![Panel::Viewport, Panel::Objects]);
        hide_viewport_tab(&mut dock);
        assert!(hidden_tabs(&dock).is_empty());
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
            vec![Panel::Objects, Panel::Attributes, Panel::EditLayers],
        );
        let (dock, note) = restore_or_default(Some(saved));
        assert!(note.is_none());
        assert_eq!(leaves(&dock).len(), 2);
    }

    #[test]
    fn a_layout_saved_before_edit_layers_gets_it_beside_objects() {
        // The standard layout as saved by an earlier version.
        let mut saved = DockState::new(vec![Panel::Viewport]);
        let tree = saved.main_surface_mut();
        let [main_area, right] = tree.split_right(NodeIndex::root(), 0.75, vec![Panel::Attributes]);
        tree.split_below(right, 0.6, vec![Panel::Objects]);
        tree.split_below(main_area, 0.85, vec![Panel::Coordinates]);
        let (dock, note) = restore_or_default(Some(saved));
        assert!(note.is_none());
        assert!(is_complete(&dock));
        let (_, leaf) = dock
            .iter_leaves()
            .find(|(_, l)| l.tabs.contains(&Panel::Objects))
            .unwrap();
        assert_eq!(leaf.tabs, vec![Panel::Objects, Panel::EditLayers]);
        // Objects stays in front.
        assert_eq!(leaf.active.0, 0);
    }

    #[test]
    fn show_tab_brings_a_panel_to_the_front() {
        let mut dock = default_layout();
        show_tab(&mut dock, Panel::EditLayers);
        let (_, leaf) = dock
            .iter_leaves()
            .find(|(_, l)| l.tabs.contains(&Panel::EditLayers))
            .unwrap();
        assert_eq!(leaf.tabs[leaf.active.0], Panel::EditLayers);
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
