//! Edges, varied copies and veins on the shape canvas (Phase 12 milestone
//! 13, decision 0275): the rows for the selected pieces in the pieces
//! list. Each change goes to every selected piece (and, through their
//! links, to their linked copies) as one undo step; a slider dragged is
//! one step.
//!
//! - **Edge** (shapes that add or cut out): Smooth, Jagged, Serrated,
//!   Wavy or Torn, with Amount, Tooth size (in centimetres, from the
//!   graph's real-world size) and a seed with its ↻ button.
//! - **Vary copies:** "Each copy differs", with Size ±, Angle ± and Colour
//!   shift, and ↻ New look (new seeds for the selected pieces).
//! - **Vein** (lines): Width (centimetres), Depth (groove to ridge) and
//!   Shade (darker to lighter).

use super::report;
use crate::dialog::help;
use crate::material::MaterialEditor;
use crate::text::tr;
use egui::{Response, Ui};
use scene::Document;
use shapes::{Drawing, Edge, EdgeKind, PieceId, PieceKind, Problem, Vary, Vein};
use std::sync::Arc;

impl MaterialEditor {
    /// The selected pieces' edge, varying and vein rows.
    pub(super) fn variety_rows(
        &mut self,
        ui: &mut Ui,
        drawing: &Arc<Drawing>,
        chosen: &[PieceId],
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        let cm = self.canvas_cm(document);
        let pieces: Vec<&shapes::Piece> =
            chosen.iter().filter_map(|id| drawing.piece(*id)).collect();
        if let Some(shape) = pieces.iter().find(|p| p.kind != PieceKind::Line) {
            crate::attributes::section(ui, tr("Edge"));
            self.edge_rows(ui, drawing, chosen, shape.edge, cm, document, note);
        }
        if let Some(first) = pieces.first() {
            crate::attributes::section(ui, tr("Vary copies"));
            self.vary_rows(ui, drawing, chosen, first.vary, document, note);
        }
        if let Some(line) = pieces.iter().find(|p| p.kind == PieceKind::Line) {
            crate::attributes::section(ui, tr("Vein"));
            self.vein_rows(ui, drawing, chosen, line.vein, cm, document, note);
        }
    }

    /// How many centimetres the canvas's texture square covers (the
    /// graph's real-world size).
    fn canvas_cm(&self, document: &Document) -> f32 {
        let registry = self.registry_of(document);
        self.current(document)
            .map_or(100.0, |g| matgraph::real_size_cm(&registry, g))
            .max(0.01)
    }

    #[allow(clippy::too_many_arguments)]
    fn edge_rows(
        &mut self,
        ui: &mut Ui,
        drawing: &Arc<Drawing>,
        chosen: &[PieceId],
        edge: Edge,
        cm: f32,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        let mut next = edge;
        ui.horizontal_wrapped(|ui| {
            for kind in EdgeKind::ALL {
                ui.selectable_value(&mut next.kind, kind, tr(kind.name()))
                    .on_hover_text(tr(kind.describe()));
            }
        });
        if next.kind != edge.kind {
            let label = tr("Change a shape's edge");
            self.change_chosen(document, drawing, chosen, label, None, note, |d, id| {
                shaped(d, id, |d| d.with_edge(id, next))
            });
            return;
        }
        if edge.kind == EdgeKind::Smooth {
            help(
                ui,
                tr("Pick an edge to roughen the outline. Linked copies share it."),
            );
            return;
        }
        let mut amount = edge.amount;
        let a = crate::dialog::row(ui, tr("Amount"), |ui| {
            ui.add(egui::Slider::new(&mut amount, shapes::edge::AMOUNT))
                .on_hover_text(tr("How deep the teeth are: 1 is as deep as they are wide"))
        });
        let mut size_cm = edge.size * cm;
        let range = shapes::edge::SIZE.start() * cm..=shapes::edge::SIZE.end() * cm;
        let s = crate::dialog::row(ui, tr("Tooth size"), |ui| {
            ui.add(
                egui::Slider::new(&mut size_cm, range)
                    .logarithmic(true)
                    .suffix(" cm"),
            )
            .on_hover_text(tr("How wide one tooth is along the outline"))
        });
        let mut seed = edge.seed;
        let mut rolled = false;
        let n = crate::dialog::row(ui, tr("Seed"), |ui| {
            let r = ui
                .add(egui::DragValue::new(&mut seed))
                .on_hover_text(tr("Another number gives other teeth of the same kind"));
            rolled = ui.button("↻").on_hover_text(tr("Roll new teeth")).clicked();
            r
        });
        if rolled {
            seed = shapes::drawing::next_seed(seed);
        }
        let changed = a.changed() || s.changed() || n.changed() || rolled;
        if changed {
            let key = self.slid_key(&[&a, &s, &n]);
            let next = Edge {
                amount,
                size: size_cm / cm,
                seed,
                ..edge
            };
            let label = tr("Change a shape's edge");
            self.change_chosen(document, drawing, chosen, label, key, note, |d, id| {
                shaped(d, id, |d| d.with_edge(id, next))
            });
        }
        self.slid_end(&[&a, &s, &n]);
    }

    fn vary_rows(
        &mut self,
        ui: &mut Ui,
        drawing: &Arc<Drawing>,
        chosen: &[PieceId],
        vary: Vary,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        let mut on = vary.on;
        if ui
            .checkbox(&mut on, tr("Each copy differs"))
            .on_hover_text(tr(
                "Every copy gets its own size, angle, colour and teeth, so no two match, even \
                 linked ones",
            ))
            .changed()
        {
            let next = Vary { on, ..vary };
            let label = tr("Change how copies vary");
            self.change_chosen(document, drawing, chosen, label, None, note, |d, id| {
                d.with_vary(id, next)
            });
            return;
        }
        if !vary.on {
            return;
        }
        let mut size = vary.size * 100.0;
        let a = crate::dialog::row(ui, tr("Size ±"), |ui| {
            ui.add(egui::Slider::new(&mut size, 0.0..=50.0).suffix(" %"))
                .on_hover_text(tr("How much bigger or smaller a copy may be"))
        });
        let mut angle = vary.angle.to_degrees();
        let b = crate::dialog::row(ui, tr("Angle ±"), |ui| {
            ui.add(egui::Slider::new(&mut angle, 0.0..=180.0).suffix("°"))
                .on_hover_text(tr("How far a copy may be turned either way"))
        });
        let mut colour = vary.colour * 100.0;
        let c = crate::dialog::row(ui, tr("Colour shift"), |ui| {
            ui.add(egui::Slider::new(&mut colour, 0.0..=100.0).suffix(" %"))
                .on_hover_text(tr("How much a copy may be lighter, darker or tinted"))
        });
        if a.changed() || b.changed() || c.changed() {
            let key = self.slid_key(&[&a, &b, &c]);
            let next = Vary {
                on: true,
                size: size / 100.0,
                angle: angle.to_radians(),
                colour: colour / 100.0,
            };
            let label = tr("Change how copies vary");
            self.change_chosen(document, drawing, chosen, label, key, note, |d, id| {
                d.with_vary(id, next)
            });
        }
        self.slid_end(&[&a, &b, &c]);
        if ui
            .button(tr("↻ New look"))
            .on_hover_text(tr(
                "Give the selected pieces new seeds: each comes out another way",
            ))
            .clicked()
        {
            let done = drawing.with_new_seeds(chosen);
            match done {
                Ok(d) => report(
                    self.set_drawing(document, d, tr("Give pieces a new look"), None),
                    note,
                ),
                Err(why) => self.shape_note(tr(why.describe()), Vec::new(), note),
            }
        }
        help(
            ui,
            tr("A group varies as one, by its bottom piece, so a leaf keeps its veins."),
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn vein_rows(
        &mut self,
        ui: &mut Ui,
        drawing: &Arc<Drawing>,
        chosen: &[PieceId],
        vein: Vein,
        cm: f32,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        let mut width = vein.width * cm;
        let range =
            shapes::drawing::VEIN_WIDTH.start() * cm..=shapes::drawing::VEIN_WIDTH.end() * cm;
        let a = crate::dialog::row(ui, tr("Width"), |ui| {
            ui.add(
                egui::Slider::new(&mut width, range)
                    .logarithmic(true)
                    .suffix(" cm"),
            )
            .on_hover_text(tr("How wide the vein is"))
        });
        let mut depth = vein.depth;
        let b = crate::dialog::row(ui, tr("Depth"), |ui| {
            ui.add(egui::Slider::new(&mut depth, shapes::drawing::VEIN_DEPTH))
                .on_hover_text(tr(
                    "Below 0 a groove, above 0 a ridge, in the Height (inside the shape)",
                ))
        });
        let mut shade = vein.shade;
        let c = crate::dialog::row(ui, tr("Shade"), |ui| {
            ui.add(egui::Slider::new(&mut shade, shapes::drawing::VEIN_SHADE))
                .on_hover_text(tr("Below 0 darker, above 0 lighter, in the Colours"))
        });
        if a.changed() || b.changed() || c.changed() {
            let key = self.slid_key(&[&a, &b, &c]);
            let next = Vein {
                width: width / cm,
                depth,
                shade,
            };
            let label = tr("Change a vein");
            self.change_chosen(document, drawing, chosen, label, key, note, |d, id| {
                lined(d, id, |d| d.with_vein(id, next))
            });
        }
        self.slid_end(&[&a, &b, &c]);
        help(
            ui,
            tr("Lines are veins: a groove or ridge in the Height and a line in the Colours."),
        );
    }

    /// Changes each of `chosen` by `change`, as one undo step labelled
    /// `label` (merged under `key`), or says why not.
    #[allow(clippy::too_many_arguments)]
    fn change_chosen(
        &mut self,
        document: &mut Document,
        drawing: &Drawing,
        chosen: &[PieceId],
        label: &str,
        key: Option<u64>,
        note: &mut Option<String>,
        change: impl Fn(&Drawing, PieceId) -> Result<Drawing, Problem>,
    ) {
        let mut d = drawing.clone();
        for id in chosen {
            match change(&d, *id) {
                Ok(next) => d = next,
                Err(why) => return self.shape_note(tr(why.describe()), Vec::new(), note),
            }
        }
        report(self.set_drawing(document, d, label, key), note);
    }

    /// The merge key for sliders `slid`: kept while one is dragged.
    fn slid_key(&mut self, slid: &[&Response]) -> Option<u64> {
        if slid.iter().any(|r| r.drag_started()) || self.shape_field.is_none() {
            self.shape_field = Some(self.shape_key());
        }
        self.shape_field
    }

    /// Lets the merge key go when a drag ends.
    fn slid_end(&mut self, slid: &[&Response]) {
        if slid.iter().any(|r| r.drag_stopped()) {
            self.shape_field = None;
        }
    }
}

/// `change` for a piece that adds or cuts out; lines are left as they are.
fn shaped(
    d: &Drawing,
    id: PieceId,
    change: impl FnOnce(&Drawing) -> Result<Drawing, Problem>,
) -> Result<Drawing, Problem> {
    match d.piece(id) {
        Some(p) if p.kind != PieceKind::Line => change(d),
        _ => Ok(d.clone()),
    }
}

/// `change` for a line; other pieces are left as they are.
fn lined(
    d: &Drawing,
    id: PieceId,
    change: impl FnOnce(&Drawing) -> Result<Drawing, Problem>,
) -> Result<Drawing, Problem> {
    match d.piece(id) {
        Some(p) if p.kind == PieceKind::Line => change(d),
        _ => Ok(d.clone()),
    }
}
