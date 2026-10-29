//! The Paint page's right side like the mockup (decision 0227): the
//! Paint layers on top beside Objects, Attributes and the Paint tab under
//! them; layer rows with pictures, kind, blend mode and opacity; + Paint,
//! + Fill, + Group and ⋯ under the list.

use super::paint_layers_tests::{names, paint_front, painting_the_cube, press, set};
use crate::layout;
use crate::paint_layers::{MORE_LABEL, kind_text};
use crate::panel::Panel;
use egui_dock::{DockState, NodeIndex};
use egui_kittest::kittest::Queryable;
use paint::Layer;

/// Whether `top`'s tab sits in the column above `under`'s.
fn above(dock: &DockState<Panel>, top: Panel, under: Panel) -> bool {
    let (Some(a), Some(b)) = (dock.find_tab(&top), dock.find_tab(&under)) else {
        return false;
    };
    a.node.is_left() && b.node.is_right() && a.node.parent() == b.node.parent()
}

#[test]
fn the_paint_layout_puts_the_layers_on_top_and_the_paint_tab_under_them() {
    let dock = layout::paint_layout();
    assert!(above(&dock, Panel::PaintLayers, Panel::Paint));
    assert!(above(&dock, Panel::Objects, Panel::Attributes));
    // The layers and the Paint tab are in front of their neighbours.
    for front in [Panel::PaintLayers, Panel::Paint] {
        let path = dock.find_tab(&front).unwrap();
        let leaf = &dock.main_surface()[path.node];
        assert_eq!(leaf.tabs().unwrap()[path.tab.0], front);
        if let egui_dock::Node::Leaf(leaf) = leaf {
            assert_eq!(leaf.active, path.tab, "{front:?} is in front");
        }
    }
}

#[test]
fn a_paint_layout_saved_with_the_layers_underneath_gets_the_new_arrangement() {
    // The arrangement before decision 0227.
    let mut old = DockState::new(vec![Panel::Viewport]);
    let tree = old.main_surface_mut();
    let [main, right] = tree.split_right(
        NodeIndex::root(),
        0.75,
        vec![Panel::Attributes, Panel::Paint],
    );
    tree.split_below(right, 0.5, vec![Panel::PaintLayers, Panel::Objects]);
    let [view, _] = tree.split_below(main, 0.8, vec![Panel::Brushes, Panel::Coordinates]);
    tree.split_right(view, 0.5, vec![Panel::UvEditor]);
    let restored = layout::restore_paint_layout(Some(old));
    assert!(above(&restored, Panel::PaintLayers, Panel::Paint));
    // One saved in the new arrangement is kept as it is.
    let new = layout::paint_layout();
    let kept = layout::restore_paint_layout(Some(new.clone()));
    let tabs = |d: &DockState<Panel>| d.iter_all_tabs().map(|(_, p)| *p).collect::<Vec<_>>();
    assert_eq!(tabs(&kept), tabs(&new));
}

#[test]
fn the_list_has_paint_fill_and_group_under_it_and_the_rest_in_the_more_menu() {
    let mut harness = painting_the_cube();
    for shown in ["+ Paint", "+ Fill", "+ Group", MORE_LABEL] {
        assert!(
            harness.query_by_label(shown).is_some(),
            "{shown} under the list"
        );
    }
    for hidden in [
        "+ Gradient",
        "+ Light and shade",
        "Realism channels",
        "Load preset…",
    ] {
        assert!(harness.query_by_label(hidden).is_none(), "{hidden} is in ⋯");
    }
    press(&mut harness, MORE_LABEL);
    for listed in [
        "+ Gradient",
        "+ Edge highlight",
        "+ Material",
        "Realism channels",
        "Load preset…",
    ] {
        assert!(harness.query_by_label(listed).is_some(), "{listed} in ⋯");
    }
}

#[test]
fn the_more_menu_acts_on_the_chosen_layer() {
    let mut harness = painting_the_cube();
    paint_front(&mut harness);
    press(&mut harness, "+ Group");
    assert_eq!(names(&harness), ["Group", "Paint"]);
    press(&mut harness, "Delete");
    assert_eq!(names(&harness), ["Paint"]);
}

#[test]
fn each_layer_and_mask_gets_a_picture_kept_while_listed() {
    let mut harness = painting_the_cube();
    paint_front(&mut harness);
    assert_eq!(harness.state().workbench.paint_layers.pictures(), 1);
    press(&mut harness, "Add white mask");
    assert!(set(&harness).layers[0].mask.is_some());
    assert_eq!(
        harness.state().workbench.paint_layers.pictures(),
        2,
        "and its mask"
    );
    press(&mut harness, "+ Fill");
    // A fill shows its colour: no picture to make.
    assert_eq!(harness.state().workbench.paint_layers.pictures(), 2);
    press(&mut harness, "Delete");
    assert_eq!(names(&harness), ["Paint"]);
    // Taking the mask away lets its picture go.
    press(&mut harness, "Remove mask");
    assert!(set(&harness).layers[0].mask.is_none());
    assert_eq!(
        harness.state().workbench.paint_layers.pictures(),
        1,
        "let go"
    );
}

#[test]
fn a_row_says_what_kind_of_layer_it_is() {
    let image = paint::Image::new(64).unwrap();
    assert_eq!(kind_text(&Layer::paint("A", image.clone())), "Paint");
    assert_eq!(kind_text(&Layer::fill("B", [1, 2, 3, 255])), "Fill");
    assert_eq!(kind_text(&Layer::group("C", Vec::new())), "Group");
    let mut gradient = Layer::paint("D", image.clone());
    gradient.gradient = Some(paint::Gradient::default());
    assert_eq!(kind_text(&gradient), "Gradient");
    let mut masked = Layer::fill("E", [0, 0, 0, 255]);
    masked.mask = paint::Mask::new(64, true);
    masked.clipped = true;
    assert_eq!(kind_text(&masked), "Fill · mask · clipped");
    masked.mask.as_mut().unwrap().smart = Some(paint::SmartMask::of(paint::SmartKind::Edges));
    assert_eq!(kind_text(&masked), "Fill · smart mask · clipped");
}
