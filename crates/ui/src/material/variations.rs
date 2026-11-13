//! Variations and the dice (Phase 12, milestone 18,
//! [decision 0280](../../../../docs/decisions/0280-variations-and-the-dice.md)).
//!
//! A variation is a named look of the graph (Clean, Mossy, Ruined): the
//! settings shown in the painter, every seed and the moment baked
//! (`matgraph::Variation`). The canvas bar's **Variations** menu uses one
//! or saves the graph as a new one; Attributes (nothing selected) lists
//! them to use, rename, update and delete. **Roll the dice** on the bar
//! gives every seed a new number; a box's right-click menu rolls just
//! its seeds. Each is one undo step. Buttons only: no keys.
//!
//! A graph fill in the painter and a material made by a graph pick a
//! variation with [`picker`].

use super::MaterialEditor;
use crate::attributes::section;
use crate::dialog::help;
use crate::text::tr;
use crate::theme;
use egui::{RichText, Ui};
use matgraph::{BoxId, Graph, GraphEdit, VariationId};
use scene::Document;

/// The canvas bar's menu of variations.
pub(crate) const VARIATIONS_LABEL: &str = crate::text!("Variations");
/// The entry saving the graph as a new variation.
pub(crate) const SAVE_LABEL: &str = crate::text!("Save as a new variation");
/// The dice button on the canvas bar.
pub(crate) const DICE_LABEL: &str = crate::text!("Roll the dice");
/// A box's menu entry rolling its seeds.
pub(crate) const ROLL_BOX_LABEL: &str = crate::text!("Roll its seed");
/// Using a variation, in Attributes.
pub(crate) const USE_LABEL: &str = crate::text!("Use");
/// Updating a variation to the graph as it is, in Attributes.
pub(crate) const UPDATE_LABEL: &str = crate::text!("Update");
/// Deleting a variation, in Attributes.
pub(crate) const DELETE_LABEL: &str = crate::text!("Delete");
/// Shown beside the variation the graph looks as now.
pub(crate) const IN_USE_LABEL: &str = crate::text!("In use");
/// The variation picker's label, in the painter and Attributes.
pub(crate) const PICKER_LABEL: &str = crate::text!("Variation");
/// The picker's choice of no variation.
pub(crate) const AS_IT_IS_LABEL: &str = crate::text!("As the graph is");

/// Marks the variation rows' merge keys apart from Attributes' others.
const NAME_KEYS: u64 = 0x5641_0000;

impl MaterialEditor {
    /// The canvas bar's Variations menu: each variation (a click uses
    /// it), then saving the graph as a new one. Not inside your own box:
    /// variations are the graph's looks.
    pub(super) fn variations_menu(
        &mut self,
        ui: &mut Ui,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        if !self.inside.is_empty() {
            return;
        }
        let Some(graph) = self.current(document).cloned() else {
            return;
        };
        let registry = self.registry_of(document);
        let in_use = graph.variation_in_use(&registry);
        let title = match in_use.and_then(|id| graph.variation(id)) {
            Some(v) => crate::trf!("{}: {}", tr(VARIATIONS_LABEL), v.name),
            None => tr(VARIATIONS_LABEL).to_string(),
        };
        let mut picked = None;
        let mut save = false;
        crate::dialog::menu(ui, title, |ui| {
            if graph.variations().next().is_none() {
                ui.label(
                    RichText::new(tr(
                        "No variations yet. Save the graph's look as one, change it, and save \
                         another: then flip between them here.",
                    ))
                    .color(theme::TEXT_WEAK)
                    .small(),
                );
            }
            for (id, v) in graph.variations() {
                let chosen = Some(id) == in_use;
                if ui
                    .add(egui::Button::selectable(chosen, &v.name))
                    .on_hover_text(tr(
                        "Use this variation: its shown settings, seeds and moment are set back",
                    ))
                    .clicked()
                {
                    picked = Some(id);
                    ui.close();
                }
            }
            ui.separator();
            if ui
                .button(tr(SAVE_LABEL))
                .on_hover_text(tr(
                    "Keep the graph's look as it is now (the settings shown in the painter, \
                     every seed and the moment) under a name, to come back to",
                ))
                .clicked()
            {
                save = true;
                ui.close();
            }
            ui.label(
                RichText::new(tr(
                    "Rename, update and delete them in Attributes, with nothing selected.",
                ))
                .color(theme::TEXT_WEAK)
                .small(),
            );
        })
        .response
        .on_hover_text(tr(
            "Named looks of this graph (Clean, Mossy, Ruined…): flip between them, or save a \
             new one",
        ));
        if let Some(id) = picked {
            self.use_variation(document, id, note);
        }
        if save {
            self.save_variation(document, note);
        }
    }

    /// The canvas bar's dice: every seed in the graph gets a new number.
    pub(super) fn dice_button(
        &mut self,
        ui: &mut Ui,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        if self.current(document).is_none() {
            return;
        }
        let clicked = dice(ui)
            .on_hover_text(tr(
                "Roll the dice: gives every seed in the graph a new number, for a fresh take on \
                 the same look (each box's seed and the graph's own). Undo takes it back",
            ))
            .clicked();
        if clicked {
            self.roll(document, None, note);
        }
    }

    /// Rolls every seed in the graph shown, or only box `only`'s, as one
    /// undo step.
    pub(crate) fn roll(
        &mut self,
        document: &mut Document,
        only: Option<BoxId>,
        note: &mut Option<String>,
    ) {
        let Some(graph) = self.current(document).cloned() else {
            return;
        };
        let registry = self.registry_of(document);
        self.rolls += 1;
        let edits = graph.rolled(&registry, graph.roll_from(self.rolls), only);
        let none = edits.is_empty();
        let label = match only {
            Some(_) => tr("Roll a box's seed"),
            None => tr("Roll the dice"),
        };
        *note = Some(match self.run(document, edits, label, None, false) {
            Err(why) => why.reason,
            Ok(()) if none => tr("That box has no seed to roll.").to_string(),
            Ok(()) if only.is_some() => {
                tr("Rolled the box's seed: a new take on its pattern. Undo takes it back.")
                    .to_string()
            }
            Ok(()) => tr(
                "Rolled the dice: every seed in the graph has a new number. Undo takes it back; \
                 Variations → Save as a new variation keeps it.",
            )
            .to_string(),
        });
    }

    /// Makes the graph look as variation `id` says, as one undo step.
    pub(crate) fn use_variation(
        &mut self,
        document: &mut Document,
        id: VariationId,
        note: &mut Option<String>,
    ) {
        let Some(graph) = self.current(document).cloned() else {
            return;
        };
        let Some(name) = graph.variation(id).map(|v| v.name.clone()) else {
            return;
        };
        let registry = self.registry_of(document);
        let edits = graph.variation_edits(&registry, id);
        if edits.is_empty() {
            *note = Some(crate::trf!("The graph already looks as {} says.", name));
            return;
        }
        let label = crate::trf!("Use the variation {}", name);
        *note = Some(match self.run(document, edits, &label, None, false) {
            Ok(()) => crate::trf!("Using the variation {}. Undo goes back.", name),
            Err(why) => why.reason,
        });
    }

    /// Saves the graph as it is as a new variation, named "Variation N".
    pub(crate) fn save_variation(&mut self, document: &mut Document, note: &mut Option<String>) {
        let Some(graph) = self.current(document).cloned() else {
            return;
        };
        let registry = self.registry_of(document);
        let name = graph.new_variation_name();
        let edit = GraphEdit::AddVariation(graph.variation_now(&registry, name.clone()));
        let label = edit.describe();
        *note = Some(match self.run(document, vec![edit], &label, None, false) {
            Ok(()) => crate::trf!(
                "Saved the graph's look as {}. Rename it in Attributes (click the empty canvas).",
                name
            ),
            Err(why) => why.reason,
        });
    }

    /// The graph's variations in its own Attributes: each to rename, use,
    /// update or delete, then saving a new one and the dice.
    pub(super) fn variation_rows(
        &mut self,
        ui: &mut Ui,
        graph: &Graph,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        section(ui, tr(VARIATIONS_LABEL));
        help(
            ui,
            tr(
                "Named looks of this graph: the settings shown in the painter, every seed and \
                 the moment baked. A paint layer or a material using the graph can pick one, \
                 so one graph gives several looks.",
            ),
        );
        let registry = self.registry_of(document);
        let in_use = graph.variation_in_use(&registry);
        enum Asked {
            Rename(VariationId, matgraph::Variation),
            Use(VariationId),
            Update(VariationId),
            Delete(VariationId),
        }
        let mut asked = None;
        for (id, v) in graph.variations() {
            ui.push_id(id.0, |ui| {
                ui.horizontal(|ui| {
                    let mut name = v.name.clone();
                    let field = ui
                        .add(egui::TextEdit::singleline(&mut name).desired_width(110.0))
                        .on_hover_text(tr("The variation's name: type to rename it"));
                    if field.changed() && !name.trim().is_empty() {
                        let mut renamed = v.clone();
                        renamed.name = name;
                        asked = Some(Asked::Rename(id, renamed));
                    }
                    let current = Some(id) == in_use;
                    if current {
                        ui.label(RichText::new(tr(IN_USE_LABEL)).color(theme::TEXT_WEAK));
                    } else if ui
                        .button(tr(USE_LABEL))
                        .on_hover_text(crate::trf!("Make the graph look as {} says", v.name))
                        .clicked()
                    {
                        asked = Some(Asked::Use(id));
                    }
                    if ui
                        .add_enabled(!current, egui::Button::new(tr(UPDATE_LABEL)))
                        .on_hover_text(crate::trf!(
                            "Keep the graph's look as it is now in {}",
                            v.name
                        ))
                        .on_disabled_hover_text(tr("It already holds the graph as it is"))
                        .clicked()
                    {
                        asked = Some(Asked::Update(id));
                    }
                    if ui
                        .button(tr(DELETE_LABEL))
                        .on_hover_text(tr(
                            "Delete this variation (undo brings it back). Layers that picked it \
                             show the graph as it is",
                        ))
                        .clicked()
                    {
                        asked = Some(Asked::Delete(id));
                    }
                });
            });
        }
        ui.horizontal_wrapped(|ui| {
            if ui
                .button(tr(SAVE_LABEL))
                .on_hover_text(tr(
                    "Keep the graph's look as it is now under a name, to come back to",
                ))
                .clicked()
            {
                self.save_variation(document, note);
            }
            if ui
                .button(tr(DICE_LABEL))
                .on_hover_text(tr("Give every seed in the graph a new number"))
                .clicked()
            {
                self.roll(document, None, note);
            }
        });
        let result = match asked {
            None => return,
            Some(Asked::Use(id)) => {
                self.use_variation(document, id, note);
                return;
            }
            Some(Asked::Rename(id, variation)) => {
                let key = self.field_key(NAME_KEYS ^ (id.0 << 8));
                let edit = GraphEdit::ChangeVariation { id, variation };
                let label = tr("Rename a variation");
                self.run(document, vec![edit], label, Some(key), false)
            }
            Some(Asked::Update(id)) => {
                let Some(name) = graph.variation(id).map(|v| v.name.clone()) else {
                    return;
                };
                let variation = graph.variation_now(&registry, name.clone());
                let edit = GraphEdit::ChangeVariation { id, variation };
                let label = crate::trf!("Update the variation {}", name);
                let done = self.run(document, vec![edit], &label, None, false);
                if done.is_ok() {
                    *note = Some(crate::trf!("{} now holds the graph's look as it is.", name));
                }
                done
            }
            Some(Asked::Delete(id)) => {
                let edit = GraphEdit::RemoveVariation(id);
                let label = edit.describe();
                self.run(document, vec![edit], &label, None, false)
            }
        };
        if let Err(why) = result {
            *note = Some(why.reason);
        }
    }
}

/// The canvas bar's dice button: a small die drawn in the button (the
/// bar is crowded, so no words; its accessible name and hover say what
/// it does).
fn dice(ui: &mut Ui) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(28.0, 22.0), egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), tr(DICE_LABEL))
    });
    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact(&response);
        let painter = ui.painter();
        painter.rect(
            rect,
            visuals.corner_radius,
            visuals.weak_bg_fill,
            visuals.bg_stroke,
            egui::StrokeKind::Inside,
        );
        let side = 13.0;
        let die = egui::Rect::from_center_size(rect.center(), egui::vec2(side, side));
        let ink = visuals.text_color();
        painter.rect_stroke(
            die,
            2.5,
            egui::Stroke::new(1.3, ink),
            egui::StrokeKind::Inside,
        );
        // Five pips, as on a die's five.
        let step = side * 0.27;
        for (x, y) in [
            (-1.0, -1.0),
            (1.0, -1.0),
            (0.0, 0.0),
            (-1.0, 1.0),
            (1.0, 1.0),
        ] {
            painter.circle_filled(die.center() + egui::vec2(x * step, y * step), 1.4, ink);
        }
    }
    response
}

/// Which variation of `graph` to show, as a drop-down (for a graph fill
/// layer and a material made by a graph): `chosen` now; the new choice
/// when one is picked (`None` for the graph as it is). Nothing is drawn
/// when the graph has no variations and none is chosen.
pub(crate) fn picker(
    ui: &mut Ui,
    graph: &Graph,
    chosen: Option<VariationId>,
    width: f32,
) -> Option<Option<VariationId>> {
    if graph.variations().next().is_none() && chosen.is_none() {
        return None;
    }
    let shown = match chosen {
        None => tr(AS_IT_IS_LABEL).to_string(),
        Some(id) => graph.variation(id).map_or_else(
            || tr("A deleted variation (shows the graph as it is)").to_string(),
            |v| v.name.clone(),
        ),
    };
    let mut picked = None;
    crate::dialog::drop_down(ui, tr(PICKER_LABEL), &shown, "", width, |ui| {
        if crate::dialog::drop_down_entry(ui, chosen.is_none(), tr(AS_IT_IS_LABEL), "").clicked() {
            picked = Some(None);
            ui.close();
        }
        for (id, v) in graph.variations() {
            if crate::dialog::drop_down_entry(ui, chosen == Some(id), &v.name, "").clicked() {
                picked = Some(Some(id));
                ui.close();
            }
        }
    })
    .on_hover_text(tr(
        "Which of the graph's variations this shows: one graph, several looks. As the graph \
         is follows the graph's own settings",
    ));
    picked.filter(|p| *p != chosen)
}
