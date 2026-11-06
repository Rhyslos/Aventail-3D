//! The bar above the canvas: which of the project's graphs is shown, a
//! new one, renaming and deleting it (decision 0258). Graphs are part of
//! the project, saved with it; each change is an undo step.

use super::MaterialEditor;
use crate::text::tr;
use crate::theme;
use egui::{RichText, Ui};
use matgraph::{GraphEdit, GraphId};
use scene::{Command, Document};

/// The graph list's accessible name.
pub(crate) const GRAPH_LIST_LABEL: &str = crate::text!("Material graph shown");
/// The button that renames the graph.
pub(crate) const RENAME_LABEL: &str = crate::text!("Rename");
/// The button that deletes the graph.
pub(crate) const DELETE_LABEL: &str = crate::text!("Delete graph");

impl MaterialEditor {
    /// The graphs bar.
    pub(super) fn graphs_bar(
        &mut self,
        ui: &mut Ui,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        let graphs: Vec<(GraphId, String)> = document
            .scene()
            .graphs()
            .iter()
            .map(|(id, g)| (id, g.name().to_string()))
            .collect();
        let shown = self
            .graph
            .and_then(|id| graphs.iter().find(|(g, _)| *g == id))
            .map(|(_, name)| name.clone());
        if let Some((name, first)) = &mut self.renaming_graph {
            let field = ui.add(egui::TextEdit::singleline(name).desired_width(200.0));
            if std::mem::take(first) {
                field.request_focus();
            }
            let (enter, escape) = ui.input(|i| {
                (
                    i.key_pressed(egui::Key::Enter),
                    i.key_pressed(egui::Key::Escape),
                )
            });
            let done = ui.button(tr("Done")).clicked();
            if escape {
                self.renaming_graph = None;
            } else if done || enter || field.lost_focus() {
                let name = self
                    .renaming_graph
                    .take()
                    .map(|(n, _)| n)
                    .unwrap_or_default();
                if Some(&name) != shown.as_ref() {
                    let edit = GraphEdit::Rename(name.trim().to_string());
                    let label = edit.describe();
                    if let Err(why) = self.run(document, vec![edit], &label, None, false) {
                        *note = Some(why.reason);
                    }
                }
            }
        } else if let Some(name) = &shown {
            let mut picked = None;
            crate::dialog::drop_down(ui, tr(GRAPH_LIST_LABEL), name, "", 200.0, |ui| {
                for (id, name) in &graphs {
                    if crate::dialog::drop_down_entry(ui, Some(*id) == self.graph, name, "")
                        .clicked()
                    {
                        picked = Some(*id);
                        ui.close();
                    }
                }
            })
            .on_hover_text(tr(
                "The material graph shown: every graph in the project is listed",
            ));
            if let Some(id) = picked {
                self.show_graph(Some(id));
            }
            if ui
                .button(tr(RENAME_LABEL))
                .on_hover_text(tr("Give the graph another name"))
                .clicked()
            {
                self.renaming_graph = Some((name.clone(), true));
            }
        }
        if ui
            .button(tr(super::NEW_GRAPH_LABEL))
            .on_hover_text(tr("Start another material graph in this project"))
            .clicked()
        {
            self.new_graph(document, note);
        }
        if let (Some(id), Some(name)) = (self.graph, &shown)
            && ui
                .button(tr(DELETE_LABEL))
                .on_hover_text(tr(
                    "Take the graph out of the project (undo brings it back)",
                ))
                .clicked()
        {
            match document.execute(Command::SetGraph { id, graph: None }) {
                Ok(()) => {
                    *note = Some(crate::trf!(
                        "Deleted the graph {}. Undo brings it back.",
                        name
                    ));
                    self.show_graph(None);
                }
                Err(why) => *note = Some(why.to_string()),
            }
        }
        if self.graph.is_some() && self.pictures.waiting() {
            ui.label(
                RichText::new(tr("Working out the pictures…"))
                    .color(theme::TEXT_WEAK)
                    .small(),
            );
        }
    }
}
