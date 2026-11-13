//! Bypassing boxes (milestone 16, decision 0278): switching a box off for
//! a moment, so what goes in passes straight through, to see the graph
//! with and without it.
//!
//! No key was added (the project lead can choose one later): the small
//! switch on a box's title, **Bypass** in the box's right-click menu, and
//! the **Bypassed** tick in Attributes. A bypassed box is drawn dimmed with
//! a "Bypassed" mark, the wire drawn through it from the input it passes
//! on to each output. Each change is one undo step.

use super::MaterialEditor;
use super::canvas::Places;
use crate::text::tr;
use crate::theme;
use egui::{Context, Id, Order, Pos2, Rect, RichText, Ui};
use matgraph::{BoxId, BoxInfo, Graph, GraphEdit, PlugRef};
use scene::Document;

/// Bypass, in the box's right-click menu.
pub(crate) const BYPASS_LABEL: &str = crate::text!("Bypass");
/// The tick in Attributes.
pub(crate) const BYPASSED_LABEL: &str = crate::text!("Bypassed");
/// The right-click menu's entry for a bypassed box.
pub(crate) const SWITCH_ON_LABEL: &str = crate::text!("Switch back on");
/// Remove, in the box's right-click menu.
pub(crate) const REMOVE_LABEL: &str = crate::text!("Remove");

/// Box `id`'s kind, if the app knows it.
fn info_of<'r>(registry: &'r matgraph::Registry, graph: &Graph, id: BoxId) -> Option<&'r BoxInfo> {
    registry.get(&graph.get(id)?.kind).map(|t| t.info())
}

/// For each output of a bypassed box, the screen points of the input it
/// passes on and of the output, and what the output carries: the wire
/// drawn through the box.
pub(crate) fn through_lines(
    graph: &Graph,
    info: &BoxInfo,
    id: BoxId,
    places: &Places,
) -> Vec<(Pos2, Pos2, matgraph::ValueKind)> {
    let plugged: Vec<bool> = info
        .inputs
        .iter()
        .map(|i| {
            graph
                .wire_into(&PlugRef::new(id, i.key.to_string()))
                .is_some()
        })
        .collect();
    matgraph::eval::passed_through(&info.inputs, &info.outputs, &plugged)
        .into_iter()
        .zip(&info.outputs)
        .filter_map(|(from, out)| {
            let input = info.inputs.get(from?)?;
            let a = places.plug(&PlugRef::new(id, input.key.to_string()), false)?;
            let b = places.plug(&PlugRef::new(id, out.key.to_string()), true)?;
            Some((a.at, b.at, out.kind))
        })
        .collect()
}

impl MaterialEditor {
    /// Bypasses `ids` (`on`) or switches them back on, as one undo step,
    /// saying what happened in the status bar.
    pub(crate) fn set_bypass(
        &mut self,
        document: &mut Document,
        ids: &[BoxId],
        on: bool,
        note: &mut Option<String>,
    ) {
        let Some(graph) = self.current(document).cloned() else {
            return;
        };
        let edits: Vec<GraphEdit> = ids
            .iter()
            .filter(|id| graph.get(**id).is_some_and(|b| b.bypassed != on))
            .map(|id| GraphEdit::Bypass { node: *id, on })
            .collect();
        let Some(first) = edits.first() else {
            return;
        };
        let label = first.describe();
        match self.run(document, edits, &label, None, false) {
            Ok(()) => {
                let name = |id: &BoxId| {
                    info_of(self.registry(), &graph, *id)
                        .map_or_else(|| tr("The box").to_string(), |i| i.name.show())
                };
                *note = Some(match (ids, on) {
                    ([one], true) => crate::trf!(
                        "{} is bypassed: what goes in passes straight through. Click the \
                         switch on its title to switch it back on.",
                        name(one)
                    ),
                    ([one], false) => crate::trf!("{} is switched back on.", name(one)),
                    (many, true) => crate::trf!(
                        "{} boxes are bypassed: what goes in passes straight through.",
                        many.len()
                    ),
                    (many, false) => crate::trf!("{} boxes are switched back on.", many.len()),
                });
            }
            Err(why) => *note = Some(why.reason),
        }
    }

    /// The switch on box `id`'s title was clicked: bypass it, or switch it
    /// back on.
    pub(crate) fn toggle_bypass(
        &mut self,
        document: &mut Document,
        id: BoxId,
        note: &mut Option<String>,
    ) {
        let Some(on) = self
            .current(document)
            .and_then(|g| g.get(id))
            .map(|b| !b.bypassed)
        else {
            return;
        };
        self.set_bypass(document, &[id], on, note);
    }

    /// The **Bypassed** tick in Attributes, for box `id`.
    pub(crate) fn bypass_row(
        &mut self,
        ui: &mut Ui,
        graph: &Graph,
        id: BoxId,
        info: &BoxInfo,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        if info.outputs.is_empty() {
            return;
        }
        let Some(b) = graph.get(id) else { return };
        let mut on = b.bypassed;
        let about = tr(
            "Switches the box off for a moment: what goes into it passes straight through, so \
             you can compare the material with and without it. Its settings are kept.",
        );
        if ui
            .checkbox(&mut on, tr(BYPASSED_LABEL))
            .on_hover_text(about)
            .changed()
        {
            self.set_bypass(document, &[id], on, note);
        }
    }

    /// The right-click menu of a box, while open: Bypass (or Switch back
    /// on) and Remove, for the box and the others selected with it.
    pub(crate) fn box_menu(
        &mut self,
        ctx: &Context,
        rect: Rect,
        graph: &Graph,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        let Some((id, at)) = self.canvas.menu else {
            return;
        };
        let Some(b) = graph.get(id) else {
            self.canvas.menu = None;
            return;
        };
        let ids: Vec<BoxId> = if self.canvas.selected.contains(&id) {
            self.canvas.selected.iter().copied().collect()
        } else {
            vec![id]
        };
        let outputs = info_of(self.registry(), graph, id).is_some_and(|i| !i.outputs.is_empty());
        let mut picked = None;
        let shown = egui::Area::new(Id::new("material box menu"))
            .order(Order::Foreground)
            .fixed_pos(super::note::kept_inside(rect, at))
            .show(ctx, |ui| {
                egui::Frame::menu(ui.style()).show(ui, |ui| {
                    ui.set_min_width(140.0);
                    let words = if b.bypassed {
                        tr(SWITCH_ON_LABEL)
                    } else {
                        tr(BYPASS_LABEL)
                    };
                    if ui
                        .add_enabled(outputs || b.bypassed, egui::Button::new(words))
                        .clicked()
                    {
                        picked = Some(true);
                    }
                    if ui
                        .button(RichText::new(tr(REMOVE_LABEL)).color(theme::TEXT))
                        .clicked()
                    {
                        picked = Some(false);
                    }
                });
            });
        self.canvas.overlays.push(shown.response.rect);
        match picked {
            Some(true) => {
                self.canvas.menu = None;
                self.set_bypass(document, &ids, !b.bypassed, note);
            }
            Some(false) => {
                self.canvas.menu = None;
                self.canvas.selected = ids.into_iter().collect();
                self.remove_selected(document, false, note);
            }
            None => {}
        }
    }
}

/// Where a box's bypass switch sits on screen: on its title, left of
/// where the "can't run" badge goes.
pub(crate) fn switch_rect(box_rect: Rect) -> Rect {
    let badge = super::canvas::badge_rect(box_rect);
    let side = badge.width();
    Rect::from_center_size(
        egui::pos2(badge.left() - side * 0.45, badge.center().y),
        egui::vec2(side * 0.8, side * 0.5),
    )
}

/// Draws a box's bypass switch: a quiet outline with its knob to the
/// right while the box is on; filled with the accent, knob to the left,
/// while it is bypassed, so a bypassed box stands out.
pub(crate) fn draw_switch(
    painter: &egui::Painter,
    box_rect: Rect,
    bypassed: bool,
    accent: egui::Color32,
) {
    let r = switch_rect(box_rect);
    if r.width() < 6.0 {
        return;
    }
    let rounding = r.height() / 2.0;
    let knob = rounding * 0.7;
    if bypassed {
        painter.rect_filled(r, rounding, accent);
        painter.circle_filled(
            egui::pos2(r.left() + rounding, r.center().y),
            knob,
            theme::TEXT_STRONG,
        );
    } else {
        painter.rect_stroke(
            r,
            rounding,
            egui::Stroke::new(1.0, theme::TEXT_WEAK),
            egui::StrokeKind::Inside,
        );
        painter.circle_filled(
            egui::pos2(r.right() - rounding, r.center().y),
            knob,
            theme::TEXT_WEAK,
        );
    }
}
