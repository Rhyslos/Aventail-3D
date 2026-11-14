//! The Paint page's side panels and shelf fit, however the window or the
//! right column is sized (a review of a report that the right side was
//! "a little bugged"): rows ran past the panel's edge, the Paint tab's
//! scroll bar was pushed out of sight, the chosen layer's settings were
//! squeezed to a row or two, and the shelf's buttons were drawn over its
//! filters. See `crate::fitting`.

use super::paint_graph_tests::moss_on_top;
use super::tests::{State, TestRenderer, harness_built_sized, info, with_a_cube};
use super::*;
use crate::panel::Panel;
use egui::{Rect, pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use paint::{Image, Layer, SeamBlend, TextureSet};

type H = Harness<'static, State>;

/// The window sizes tried: the reference pictures', a common laptop's, a
/// full HD screen's and a small window's.
const SIZES: [[f32; 2]; 4] = [
    [1280.0, 800.0],
    [1366.0, 768.0],
    [1920.0, 1080.0],
    [1024.0, 700.0],
];

/// Paint mode in a window of `size`, with the cube selected.
fn painting_at(size: [f32; 2], builder: egui_kittest::HarnessBuilder<State>) -> H {
    let mut harness = harness_built_sized(
        builder,
        with_a_cube(Workbench::new(info(), None)),
        TestRenderer::default(),
        size,
    );
    crate::theme::apply(&harness.ctx);
    harness.state_mut().workbench.mode = crate::palette::Mode::Paint;
    harness.run();
    let doc = harness.state_mut().workbench.document_mut();
    let cube = doc.scene().walk()[0].0;
    doc.set_selection([cube]);
    harness.run();
    harness
}

/// A harness that lets a frame or two more pass before the window rests:
/// a settings grid takes a frame to settle its columns when the tool or
/// the panel's width changes.
fn painting(size: [f32; 2]) -> H {
    painting_at(size, Harness::builder().with_max_steps(12))
}

/// The cube's material gets a busy stack, as a real project has: a fill,
/// a group of long-named layers holding a graph fill and a group of its
/// own, ten paint layers and a Seam blend layer on top, with the realism
/// channels on.
fn busy_layers(harness: &mut H) {
    let doc = harness.state_mut().workbench.document_mut();
    let mut texture = doc.scene().texture();
    texture.size = 64;
    doc.execute(scene::Command::SetTexture(texture)).unwrap();
    let graph = doc.scene().graphs().next_id();
    doc.execute(scene::Command::AddGraph(moss_on_top()))
        .unwrap();
    let image = || Image::new(64).unwrap();
    let mut set = TextureSet::new(64).unwrap();
    set.realism = true;
    set.layers.push(Layer::fill(
        "Base colour of the whole model",
        [90, 80, 70, 255],
    ));
    let mut moss = Layer::paint("Moss growing on the top faces", image());
    moss.graph = Some(paint::GraphFill::of(graph));
    let chips = Layer::group(
        "Scratches and chips",
        (0..3)
            .map(|i| Layer::paint(format!("Chip pass {i} with a long name"), image()))
            .collect(),
    );
    set.layers.push(Layer::group(
        "Weathering and grime from a very long winter",
        vec![
            Layer::paint("Mud splatter along the lower edge of the model", image()),
            moss,
            chips,
        ],
    ));
    for i in 0..10 {
        set.layers.push(Layer::paint(
            format!("Hand-painted detail layer number {i}"),
            image(),
        ));
    }
    let mut blend = Layer::paint("Seam blend", image());
    blend.seam_blend = Some(SeamBlend::default());
    set.layers.push(blend);
    doc.execute(scene::Command::SetTextureSet {
        material: "Default".into(),
        set: Some(set),
    })
    .unwrap();
    super::paint_graph_tests::wait(harness);
}

/// The graph fill inside the first group.
const GRAPH_FILL: [usize; 2] = [1, 1];

/// The Seam blend layer on top.
const SEAM_BLEND: [usize; 1] = [12];

fn choose(harness: &mut H, path: &[usize]) {
    harness
        .state_mut()
        .workbench
        .paint_layers
        .choose("Default", path.to_vec(), false);
    harness.run();
}

fn tool(harness: &mut H, label: &str) {
    harness.get_by_label(label).click();
    harness.run();
}

/// Opens every closed section of the Paint tab.
fn open_sections(harness: &mut H) {
    for title in [
        "Lays down",
        "Pen and smoothing",
        "Symmetry and trim",
        "Paint through and effects",
        "Keys for painting",
    ] {
        let closed = harness
            .query_by_label(title)
            .is_some_and(|n| n.accesskit_node().toggled() == Some(egui::accesskit::Toggled::False));
        if closed {
            harness.get_by_label(title).scroll_to_me();
            harness.run();
            harness.get_by_label(title).click();
            harness.run();
        }
    }
}

/// Makes the right column `px` wide.
fn right_column(harness: &mut H, px: f32) {
    let total = harness.ctx.content_rect().width();
    let root = egui_dock::NodeIndex::root();
    if let egui_dock::Node::Horizontal(split) =
        &mut harness.state_mut().workbench.dock.main_surface_mut()[root]
    {
        split.fraction = 1.0 - px / total;
    }
    harness.run();
    harness.run();
}

/// Brings `panel`'s tab to the front.
fn show(harness: &mut H, panel: Panel) {
    crate::layout::show_tab(&mut harness.state_mut().workbench.dock, panel);
    harness.run();
}

/// Where `panel`'s body is in the window.
fn body(harness: &H, panel: Panel) -> Rect {
    let dock = &harness.state().workbench.dock;
    let path = dock.find_tab(&panel).expect("the panel is in the layout");
    match &dock[path.surface][path.node] {
        egui_dock::Node::Leaf(leaf) => leaf.viewport,
        _ => panic!("{panel:?} isn't in a leaf"),
    }
}

/// A node's rect, if it has one.
fn rect_of(node: &egui_kittest::Node<'_>) -> Option<Rect> {
    let b = node.accesskit_node().bounding_box()?;
    Some(Rect::from_min_max(
        pos2(b.x0 as f32, b.y0 as f32),
        pos2(b.x1 as f32, b.y1 as f32),
    ))
}

/// A node's role and name, for messages.
fn name_of(node: &egui_kittest::Node<'_>) -> String {
    let a = node.accesskit_node();
    format!(
        "{:?} '{}' '{}'",
        a.role(),
        a.label().unwrap_or_default(),
        a.value().unwrap_or_default()
    )
}

/// The widgets starting inside the right column that run past its right
/// edge.
fn past_the_edge(harness: &H) -> Vec<String> {
    let column = body(harness, Panel::PaintLayers).union(body(harness, Panel::Paint));
    harness
        .root()
        .children_recursive()
        .filter_map(|n| Some((rect_of(&n)?, name_of(&n))))
        .filter(|(r, _)| r.left() >= column.left() && r.left() < column.right())
        .filter(|(r, _)| r.right() > column.right() + 0.5)
        .map(|(r, name)| format!("{name} at {r:?} (edge {})", column.right()))
        .collect()
}

/// Pairs of widgets drawn over each other inside `area`: leaves only (a
/// widget's words are drawn inside it, and floating scroll bars over
/// what they scroll), each wholly in `area`. Widgets scrolled out of
/// sight keep their places, so panels whose hidden parts reach into
/// `area` have to be put behind other tabs first.
fn overlapping(harness: &H, area: Rect) -> Vec<String> {
    use egui::accesskit::Role;
    let leaves: Vec<(Rect, String)> = harness
        .root()
        .children_recursive()
        .filter(|n| n.children().next().is_none())
        .filter(|n| {
            !matches!(
                n.accesskit_node().role(),
                Role::TextRun | Role::GenericContainer | Role::ScrollBar
            )
        })
        .filter_map(|n| Some((rect_of(&n)?, name_of(&n))))
        .filter(|(r, _)| area.contains_rect(*r) && r.area() > 0.0)
        .collect();
    let mut found = Vec::new();
    for (i, (a, an)) in leaves.iter().enumerate() {
        for (b, bn) in &leaves[i + 1..] {
            let both = a.intersect(*b);
            if both.width() > 1.0 && both.height() > 1.0 {
                found.push(format!("{an} {a:?} and {bn} {b:?}"));
            }
        }
    }
    found
}

/// Every paint tool, by its button's name.
const TOOLS: [&str; 10] = [
    "Paint brush (1)",
    "Eraser (2)",
    "Projection (3)",
    "Polygon fill (4)",
    "Smudge (5)",
    "Clone (6)",
    "Blur (7)",
    "Fill the layer (8)",
    "Blend seams (9)",
    "Pick a colour (P)",
];

/// Each tool chosen, its sections open, with a graph fill and then the
/// Seam blend layer chosen: nothing runs past the right column's edge
/// and nothing in the Paint tab is drawn over anything else.
fn check_every_tool(harness: &mut H, at: &str) {
    for layer in [&GRAPH_FILL[..], &SEAM_BLEND[..]] {
        choose(harness, layer);
        for name in TOOLS {
            tool(harness, name);
            open_sections(harness);
            // The shelf's brush cards scrolled out of sight reach into the
            // column: its few colours shown meanwhile.
            harness.state_mut().workbench.shelf_tab = crate::colour_shelf::ShelfTab::Colours;
            harness.run();
            let past = past_the_edge(harness);
            assert!(past.is_empty(), "{at}, {name}: past the edge: {past:#?}");
            // The same for the layers scrolled below the Paint layers
            // panel, over the Paint tab.
            show(harness, Panel::Objects);
            let over = overlapping(harness, body(harness, Panel::Paint));
            assert!(
                over.is_empty(),
                "{at}, {name}: drawn over each other: {over:#?}"
            );
            show(harness, Panel::PaintLayers);
            harness.state_mut().workbench.shelf_tab = crate::colour_shelf::ShelfTab::Brushes;
            harness.run();
        }
    }
}

#[test]
fn nothing_in_the_right_column_runs_past_its_edge_at_common_window_sizes() {
    for size in SIZES {
        let mut harness = painting(size);
        busy_layers(&mut harness);
        check_every_tool(&mut harness, &format!("{size:?}"));
    }
}

#[test]
fn nothing_runs_past_the_edge_of_a_narrow_right_column() {
    let mut harness = painting([1280.0, 800.0]);
    busy_layers(&mut harness);
    for px in [300.0, 250.0] {
        right_column(&mut harness, px);
        check_every_tool(&mut harness, &format!("{px} wide"));
    }
}

#[test]
fn the_paint_tabs_scroll_bar_stays_inside_the_panel() {
    // A row wider than the panel used to widen the tab's scrolling part,
    // taking its scroll bar past the window's edge.
    let mut harness = painting([1280.0, 800.0]);
    tool(&mut harness, "Clone (6)");
    open_sections(&mut harness);
    let panel = body(&harness, Panel::Paint);
    let bars: Vec<Rect> = harness
        .root()
        .children_recursive()
        .filter(|n| n.accesskit_node().role() == egui::accesskit::Role::ScrollBar)
        .filter_map(|n| rect_of(&n))
        .filter(|r| panel.intersects(*r))
        .collect();
    assert!(!bars.is_empty(), "the open sections make the tab scroll");
    for bar in bars {
        assert!(bar.right() <= panel.right() + 0.5, "{bar:?} in {panel:?}");
    }
}

/// Scrolls `label` into view and checks it then lies inside `panel`.
fn reachable(harness: &mut H, label: &str, panel: Panel) {
    harness.get_by_label(label).scroll_to_me();
    harness.run();
    let rect = harness.get_by_label(label).rect();
    let body = body(harness, panel);
    assert!(
        body.expand(0.5).contains_rect(rect),
        "{label} at {rect:?}, outside {body:?}"
    );
}

#[test]
fn the_last_of_the_paint_tab_can_be_scrolled_to() {
    for size in SIZES {
        let mut harness = painting(size);
        tool(&mut harness, "Paint brush (1)");
        open_sections(&mut harness);
        reachable(&mut harness, "Leave Paint mode", Panel::Paint);
        tool(&mut harness, "Blend seams (9)");
        reachable(
            &mut harness,
            "Mark one more seam, or unmark a marked one",
            Panel::Paint,
        );
    }
}

#[test]
fn the_chosen_layers_last_settings_can_be_scrolled_to_in_a_short_panel() {
    // The settings used to get what the list left, a row or two at these
    // sizes, with the rest out of reach.
    for size in [[1280.0, 800.0], [1366.0, 768.0], [1024.0, 700.0]] {
        let mut harness = painting(size);
        busy_layers(&mut harness);
        choose(&mut harness, &GRAPH_FILL);
        reachable(
            &mut harness,
            crate::material::tiling::TILING_LABEL,
            Panel::PaintLayers,
        );
        // With Tiling scrolled into view, the rows well above it are in
        // view too: the settings have the panel's height, not a sliver.
        let clip = harness.get_by_label("Clip to the layer below").rect();
        let panel = body(&harness, Panel::PaintLayers);
        assert!(panel.contains_rect(clip), "{size:?}: {clip:?} in {panel:?}");
        reachable(
            &mut harness,
            "Why does this look wrong?",
            Panel::PaintLayers,
        );
        choose(&mut harness, &SEAM_BLEND);
        reachable(
            &mut harness,
            crate::paint_layers::seam_rows::SHOWING_LABEL,
            Panel::PaintLayers,
        );
    }
}

#[test]
fn a_tall_panel_keeps_the_list_and_room_for_the_settings_apart() {
    // With room for both, the list and the settings scroll apart, the
    // settings with room for their first rows.
    let mut harness = painting([1920.0, 1080.0]);
    busy_layers(&mut harness);
    choose(&mut harness, &GRAPH_FILL);
    let panel = body(&harness, Panel::PaintLayers);
    let name = harness
        .query_all_by_value("Moss growing on the top faces")
        .next()
        .expect("the Name field")
        .rect();
    assert!(panel.contains_rect(name), "the Name row shows: {name:?}");
    // The top of the list is still in view beside them.
    let top = harness.get_by_label("Layer Seam blend").rect();
    assert!(panel.contains_rect(top), "{top:?}");
    // Scrolling the settings leaves the list where it is.
    reachable(
        &mut harness,
        "Why does this look wrong?",
        Panel::PaintLayers,
    );
    assert_eq!(harness.get_by_label("Layer Seam blend").rect(), top);
}

#[test]
fn the_shelf_buttons_are_not_drawn_over_its_tabs_and_filters() {
    // The reference picture showed Effects cut short and the + and Save
    // brushes… drawn over the filters.
    for size in SIZES {
        let harness = painting(size);
        let shelf = body(&harness, Panel::Brushes);
        let over = overlapping(&harness, shelf);
        assert!(over.is_empty(), "{size:?}: {over:#?}");
        for label in [
            "Saved",
            crate::brush_bar::KEEP_LABEL,
            "Save brushes…",
            "Load brushes…",
            "Search brushes",
        ] {
            let rect = harness.get_by_label(label).rect();
            assert!(
                shelf.expand(0.5).contains_rect(rect),
                "{size:?}: {label} at {rect:?} outside {shelf:?}"
            );
        }
    }
}

#[test]
fn the_shelf_search_narrows_before_its_buttons_move_down() {
    let harness = painting([1280.0, 800.0]);
    let saved = harness.get_by_label("Saved").rect();
    let search = harness.get_by_label("Search brushes").rect();
    assert!(
        (saved.center().y - search.center().y).abs() < 4.0,
        "on the filters' line: {saved:?} {search:?}"
    );
    // In a small window they go on a line of their own, still in view.
    let harness = painting([1024.0, 700.0]);
    let saved = harness.get_by_label("Saved").rect();
    let load = harness.get_by_label("Load brushes…").rect();
    assert!(load.top() >= saved.bottom(), "{saved:?} {load:?}");
}
