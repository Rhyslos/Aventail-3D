//! A graph fill's settings in the Paint layers panel ([decision 0261](../../../../docs/decisions/0261-graphs-in-the-painter.md)):
//! which graph it runs (with "Open in the graph"), how its making stands,
//! its Tiling (decision 0270), the variation it shows (decision 0280),
//! what of the model it follows, and the settings its graph shows in the
//! painter, each set the layer's own way.

use crate::dialog::help;
use crate::text::tr;
use crate::theme;
use egui::{RichText, Ui};
use matgraph::{GraphId, GraphStore};
use paint::GraphFill;

/// The button that opens the layer's graph in the Material mode.
pub(crate) const OPEN_GRAPH_LABEL: &str = crate::text!("Open in the graph");

/// How a graph fill's making stands, from the workbench.
pub(crate) struct Status<'a> {
    /// Whether it is being made now.
    pub(crate) making: bool,
    /// Why it couldn't be made, if it couldn't.
    pub(crate) problem: Option<&'a String>,
    /// For pixel art (decision 0287): how its big pixels land on the
    /// texture.
    pub(crate) pixels: Option<&'a String>,
}

/// What the rows did.
pub(crate) enum Did {
    /// One of the layer's own values changed.
    Changed,
    /// "Open in the graph" was clicked.
    Open,
}

/// The rows for graph fill `fill`; what was done, and for Open, the graph.
pub(crate) fn show(
    ui: &mut Ui,
    fill: &mut GraphFill,
    graphs: &GraphStore,
    status: Status<'_>,
    dragging: &mut bool,
) -> Option<(Did, Option<GraphId>)> {
    let registry = graphs.registry().clone();
    let registry = &*registry;
    crate::attributes::section(ui, tr("Graph fill"));
    let Some(graph) = graphs.get(fill.graph) else {
        help(
            ui,
            tr(
                "Its graph isn't in the project any more, so the layer keeps the pixels it was \
                 last made with. Undo brings the graph back; or add a new graph fill.",
            ),
        );
        return None;
    };
    let mut did = None;
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(graph.name()).strong());
        if ui
            .button(tr(OPEN_GRAPH_LABEL))
            .on_hover_text(tr(
                "Switch to the Material mode with this graph open, to change it. The layer \
                 follows when you come back",
            ))
            .clicked()
        {
            did = Some((Did::Open, Some(fill.graph)));
        }
    });
    if status.making {
        help(ui, tr("Making it on the model…"));
    } else if let Some(why) = status.problem {
        ui.add(egui::Label::new(RichText::new(why).color(crate::material::look::REFUSED)).wrap());
        help(
            ui,
            tr("Open the graph to fix it: the box that can't run is marked there, with why."),
        );
    }
    if let Some(pixels) = status.pixels {
        help(ui, pixels);
    }
    // How often it repeats on the model (decision 0270).
    let tiling = crate::fitting::side_row(ui, tr(crate::material::tiling::TILING_LABEL), |ui| {
        crate::material::tiling::control(ui, fill.tiling)
    });
    if let Some(tiling) = tiling {
        fill.tiling = tiling.tiling;
        did = Some((Did::Changed, None));
        if ui.input(|i| i.pointer.any_down()) {
            *dragging = true;
        }
    }
    // A trim sheet is laid once, so its strips stay on the rows (decision
    // 0289).
    if matgraph::is_trim_sheet(graph) {
        help(
            ui,
            tr(
                "A trim sheet is laid once across the texture, so its strips stay on the \
                 trim rows: Tiling doesn't change it.",
            ),
        );
    }
    // Which of the graph's looks it shows (decision 0280).
    if let Some(picked) =
        crate::fitting::side_row(ui, tr(crate::material::variations::PICKER_LABEL), |ui| {
            crate::material::variations::picker(ui, graph, fill.variation, 180.0)
        })
    {
        fill.variation = picked;
        did = Some((Did::Changed, None));
    }
    // The graph as the layer shows it, before its own values.
    let looked = fill.variation.map(|v| graph.with_variation(registry, v));
    let graph: &matgraph::Graph = looked.as_ref().unwrap_or(&**graph);
    // What of the model it follows.
    let follows: Vec<String> = graph
        .boxes()
        .filter(|(_, b)| matgraph::model::is_from_model(registry, &b.kind))
        .filter_map(|(_, b)| Some(registry.get(&b.kind)?.info().name.show()))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    if follows.is_empty() {
        help(
            ui,
            tr(
                "It doesn't use the model: the same material everywhere, laid on at its \
                 real-world size times its Tiling. A From the model box in the graph \
                 (Curvature, Occlusion…) makes it follow the model's edges and crevices.",
            ),
        );
    } else {
        help(
            ui,
            crate::trf!("{} {}", tr(FOLLOWS_LABEL), follows.join(", ")),
        );
    }
    // The settings the graph shows in the painter, the layer's own way.
    let shown = graph.shown(registry);
    if shown.is_empty() {
        help(
            ui,
            tr(
                "No settings shown here yet. In the Material mode, choose a box and tick \
                 Shown in the painter for the settings each layer should set its own way.",
            ),
        );
        return did;
    }
    let mut last_box = None;
    for item in shown {
        if last_box != Some(item.node) {
            last_box = Some(item.node);
            ui.label(
                RichText::new(item.info.name.show())
                    .small()
                    .color(theme::TEXT_WEAK),
            );
        }
        let own = fill.value(item.node, &item.spec.key);
        let now = own
            .clone()
            .map_or_else(|| item.value.clone(), |v| item.spec.nearest(v));
        ui.push_id((item.node, item.spec.key.as_ref()), |ui| {
            ui.horizontal_wrapped(|ui| {
                if let Some(value) = crate::material::box_rows::setting_row(
                    ui,
                    item.spec,
                    now.clone(),
                    registry.palettes(),
                ) && value != now
                {
                    fill.set_value(item.node, &item.spec.key, value);
                    did = Some((Did::Changed, None));
                    if ui.input(|i| i.pointer.any_down()) {
                        *dragging = true;
                    }
                }
                if own.is_some()
                    && ui
                        .small_button("↺")
                        .on_hover_text(tr("Back to the graph's own value"))
                        .clicked()
                {
                    fill.clear_value(item.node, &item.spec.key);
                    did = Some((Did::Changed, None));
                }
            });
        });
    }
    did
}

/// Before the list of what a graph fill follows of the model.
pub(crate) const FOLLOWS_LABEL: &str = crate::text!("Follows the model:");
