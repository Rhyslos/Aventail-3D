//! The material health check in the Material mode's preview (Phase 12,
//! milestone 27, [decision 0290](../../../../docs/decisions/0290-material-health-check.md)):
//! what may look wrong in a game, in plain words, each with buttons that
//! fix it.
//!
//! The check itself is `matgraph::health`; this part keeps its result
//! while the pictures stay the same, draws it, and carries out a fix:
//! graph edits as one undo step (tidied), looking at a box (selected and
//! framed), or the project's trim rows (one `scene::Command::SetTrim`).

use super::MaterialEditor;
use crate::dialog::help;
use crate::text::tr;
use crate::theme;
use egui::Ui;
use matgraph::health::{Action, PixelArt, Remedy, Warning, check};
use matgraph::{BoxId, Graph};
use scene::Document;

/// The panel's heading.
pub(crate) const HEALTH_CHECK: &str = crate::text!("Health check");

/// What a health check was worked out from: the pictures' number, the
/// graph and the registry (by where they are kept), and the other
/// pixel-art materials (name, pixels, real-world size's bits).
type MadeFrom = (u64, usize, usize, Vec<(String, u32, u32)>);

/// The health check worked out last, and what it was worked out from.
#[derive(Debug, Default)]
pub(crate) struct HealthCheck {
    /// What it was worked out from.
    made_from: Option<MadeFrom>,
    /// What it found.
    pub(crate) warnings: Vec<Warning>,
}

/// The project's pixel-art materials other than `shown`.
fn other_pixel_art(document: &Document, shown: Option<matgraph::GraphId>) -> Vec<PixelArt> {
    let graphs = document.scene().graphs();
    let registry = graphs.registry();
    graphs
        .iter()
        .filter(|(id, _)| Some(*id) != shown)
        .filter_map(|(_, g)| PixelArt::of(registry, g))
        .collect()
}

impl MaterialEditor {
    /// Works the health check out again when the pictures, the graph or
    /// the project's other pixel art changed; while a quick (small)
    /// picture is shown, keeps what the full-size one found.
    pub(crate) fn update_health(&mut self, document: &Document, graph: &std::sync::Arc<Graph>) {
        let full = self.pictures.full_size();
        let Some(done) = self.pictures.shown() else {
            return;
        };
        let others = other_pixel_art(document, self.graph);
        let key = (
            done.number(),
            std::sync::Arc::as_ptr(graph) as usize,
            std::sync::Arc::as_ptr(document.scene().graphs().registry()) as usize,
            others
                .iter()
                .map(|o| (o.name.clone(), o.pixels, o.real_size_cm.to_bits()))
                .collect::<Vec<_>>(),
        );
        if self.health.made_from.as_ref() == Some(&key) {
            return;
        }
        let quick = graph
            .first_of(matgraph::eval::MATERIAL_OUT)
            .and_then(|id| done.evaluation.picture(id))
            .is_some_and(|v| v.size() < full);
        if quick && self.health.made_from.is_some() {
            return;
        }
        let registry = self.registry_of(document).clone();
        self.health.warnings = check(&registry, graph, &done.evaluation, &others);
        self.health.made_from = Some(key);
    }

    /// The health check's rows: each warning and its fixes, or that none
    /// were found.
    pub(crate) fn health_rows(
        &mut self,
        ui: &mut Ui,
        document: &mut Document,
        graph: &std::sync::Arc<Graph>,
        note: &mut Option<String>,
    ) {
        ui.add_space(8.0);
        crate::dialog::heading(ui, tr(HEALTH_CHECK));
        if !self.inside.is_empty() {
            help(
                ui,
                tr(
                    "The health check looks at the whole material: go back out of your own box \
                     (Tab) to see it.",
                ),
            );
            return;
        }
        self.update_health(document, graph);
        if self.health.warnings.is_empty() {
            help(
                ui,
                tr(
                    "Nothing found: colours light well, detail holds up at a distance, the \
                     repeat doesn't show and the joins match.",
                ),
            );
            return;
        }
        let mut picked: Option<Remedy> = None;
        for (i, warning) in self.health.warnings.iter().enumerate() {
            ui.push_id(("health", i), |ui| {
                ui.label(
                    egui::RichText::new(format!("⚠ {}", warning.words))
                        .color(theme::WARNING)
                        .size(12.0),
                );
                ui.horizontal_wrapped(|ui| {
                    for remedy in &warning.remedies {
                        if ui.button(&remedy.words).clicked() {
                            picked = Some(remedy.clone());
                        }
                    }
                });
                ui.add_space(4.0);
            });
        }
        if let Some(remedy) = picked {
            self.carry_out_remedy(document, graph, &remedy, note);
        }
    }

    /// Carries out a health check fix.
    pub(crate) fn carry_out_remedy(
        &mut self,
        document: &mut Document,
        graph: &Graph,
        remedy: &Remedy,
        note: &mut Option<String>,
    ) {
        match &remedy.action {
            Action::LookAt(id) => self.look_at(graph, *id),
            Action::MakeTrimRows(id) => make_trim_rows(document, graph, *id, note),
            action => {
                let edits = action.edits(graph);
                let tidy = matches!(action, Action::InsertBox { .. });
                match self.run(document, edits, &remedy.words, None, tidy) {
                    Ok(()) => *note = Some(remedy.words.clone()),
                    Err(why) => *note = Some(why.reason),
                }
            }
        }
    }

    /// Selects box `id` and frames it on the canvas.
    fn look_at(&mut self, graph: &Graph, id: BoxId) {
        self.canvas.selected = [id].into();
        self.canvas.frame = None;
        if let (Some((lo, hi)), Some(rect)) = (
            super::look::box_rect(graph, self.registry(), id),
            self.canvas.rect.or(self.canvas.last_rect),
        ) {
            self.canvas.view.frame(rect, lo, hi);
        }
    }
}

/// Makes the project's trim rows match Trim sheet box `id`'s strips, one
/// undo step.
fn make_trim_rows(document: &mut Document, graph: &Graph, id: BoxId, note: &mut Option<String>) {
    let Some(b) = graph.get(id) else {
        return;
    };
    let rows = document.scene().graphs().registry().trim_rows().clone();
    let laid = super::trim_rows::strips_of(graph, b, id, &rows);
    match super::trim_rows::make_rows(document, &laid) {
        Ok(words) | Err(words) => *note = Some(words),
    }
}
