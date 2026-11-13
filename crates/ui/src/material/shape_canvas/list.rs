//! The pieces list beside the shape canvas (milestone 12, decision 0274):
//! the pieces from the top down, each group as a heading with its pieces
//! under it, linked copies marked; then the rows for what is selected
//! (colour, height, flipping, grouping, copying; edges, varied copies and
//! veins in `variety`), the chosen group's
//! (name, order), one chosen piece's, and the reference image's.

use super::report;
use crate::dialog::help;
use crate::material::MaterialEditor;
use crate::text::tr;
use crate::theme;
use egui::{Modifiers, Rect, RichText, Ui};
use scene::Document;
use shapes::{Drawing, GroupId, PieceId};
use std::sync::Arc;

/// How far a group's pieces are indented under it.
const INDENT: f32 = 14.0;

impl MaterialEditor {
    /// The pieces list beside the canvas, and the settings of what is
    /// selected.
    pub(super) fn pieces_list(
        &mut self,
        ui: &mut Ui,
        list: Rect,
        drawing: &Arc<Drawing>,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        ui.painter().rect_filled(list, 0.0, theme::PANEL);
        let mut ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(list.shrink(8.0))
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        egui::ScrollArea::vertical()
            .id_salt("shape pieces panel")
            .auto_shrink([false, false])
            .show(&mut ui, |ui| {
                crate::attributes::section(ui, tr("Pieces"));
                if drawing.is_empty() {
                    help(
                        ui,
                        tr("Nothing drawn yet. P draws with the pen; B adds a basic shape."),
                    );
                }
                let modifiers = ui.input(|i| i.modifiers);
                egui::ScrollArea::vertical()
                    .id_salt("shape pieces")
                    .max_height(list.height() * 0.35)
                    .show(ui, |ui| self.piece_lines(ui, drawing, modifiers));
                self.chosen_rows(ui, drawing, document, note);
                crate::attributes::section(ui, tr("Reference image"));
                self.reference_controls(ui, drawing, document, note);
                ui.add_space(8.0);
                help(
                    ui,
                    tr(
                        "Q select, W move, R rotate, S scale, T all; P pen, B basic shapes, M \
                         mirror line. Double-click a piece to edit its points. Ctrl + C / V \
                         copy and paste; Alt + V pastes linked copies. Alt + G groups, Shift + \
                         G ungroups. Tab goes back; Esc cancels.",
                    ),
                );
            });
    }

    /// One line per piece, top first, with each group's heading over its
    /// pieces.
    fn piece_lines(&mut self, ui: &mut Ui, drawing: &Drawing, modifiers: Modifiers) {
        let mut heading: Option<GroupId> = None;
        for piece in drawing.pieces().iter().rev() {
            if let Some(g) = piece.group
                && heading != Some(g)
                && let Some(group) = drawing.group(g)
            {
                heading = Some(g);
                let members = drawing.group_pieces(g);
                let chosen = self
                    .shape
                    .as_ref()
                    .is_some_and(|s| members.iter().all(|id| s.selected.contains(id)));
                let text = crate::trf!("▾ {} · group of {}", group.name, members.len());
                let r = ui.selectable_label(chosen, RichText::new(text).color(theme::TEXT_STRONG));
                if r.clicked() {
                    self.choose_many(&members, modifiers);
                }
            }
            if piece.group.is_none() {
                heading = None;
            }
            let chosen = self
                .shape
                .as_ref()
                .is_some_and(|s| s.selected.contains(&piece.id));
            let kind = tr(piece.kind.name());
            let text = if drawing.linked_count(piece.id) > 1 {
                crate::trf!("{} · {} · linked", piece.name, kind)
            } else {
                crate::trf!("{} · {}", piece.name, kind)
            };
            let indent = if piece.group.is_some() { INDENT } else { 0.0 };
            ui.horizontal(|ui| {
                ui.add_space(indent);
                if ui.selectable_label(chosen, text).clicked() {
                    self.choose_many(&[piece.id], modifiers);
                }
            });
        }
    }

    /// The app's click rule for pieces `ids` from the list: Ctrl adds,
    /// Shift takes out, otherwise only these.
    fn choose_many(&mut self, ids: &[PieceId], modifiers: Modifiers) {
        let Some(s) = self.shape.as_mut() else { return };
        if modifiers.command {
            s.selected.extend(ids.iter().copied());
        } else if modifiers.shift {
            for id in ids {
                s.selected.remove(id);
            }
        } else {
            s.selected = ids.iter().copied().collect();
        }
    }

    /// The rows for what is selected: copying always; colour, height,
    /// flipping and grouping for any pieces; then the chosen group's or
    /// the one chosen piece's own rows.
    fn chosen_rows(
        &mut self,
        ui: &mut Ui,
        drawing: &Arc<Drawing>,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        let chosen = self.chosen_pieces(drawing);
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(!chosen.is_empty(), egui::Button::new(tr("Copy")))
                .on_hover_text(tr("Copy the selected pieces (Ctrl + C)"))
                .clicked()
            {
                self.copy_pieces(document, note);
            }
            let can_paste = self.pieces_clipboard.is_some();
            if ui
                .add_enabled(can_paste, egui::Button::new(tr("Paste")))
                .on_hover_text(tr("Paste copies with shapes of their own (Ctrl + V)"))
                .clicked()
            {
                self.paste_pieces(document, false, None, note);
            }
            if ui
                .add_enabled(can_paste, egui::Button::new(tr("Paste linked")))
                .on_hover_text(tr(
                    "Paste linked copies: change the shape of one and every one changes \
                     (Alt + V)",
                ))
                .clicked()
            {
                self.paste_pieces(document, true, None, note);
            }
        });
        let Some(first) = chosen.first().and_then(|id| drawing.piece(*id)).cloned() else {
            return;
        };
        ui.add_space(4.0);
        let title = if chosen.len() == 1 {
            tr("Chosen piece")
        } else {
            tr("Chosen pieces")
        };
        crate::attributes::section(ui, title);
        self.look_rows(ui, &first, document, note);
        ui.horizontal_wrapped(|ui| {
            if ui
                .button(tr("Flip ↔"))
                .on_hover_text(tr("Flip left for right, about their middle"))
                .clicked()
            {
                self.flip_chosen(document, true, note);
            }
            if ui
                .button(tr("Flip ↕"))
                .on_hover_text(tr("Flip upside down, about their middle"))
                .clicked()
            {
                self.flip_chosen(document, false, note);
            }
            if ui
                .add_enabled(chosen.len() > 1, egui::Button::new(tr("Group")))
                .on_hover_text(tr("Gather them into a group that moves as one (Alt + G)"))
                .clicked()
            {
                self.group_chosen(document, note);
            }
            let grouped = chosen
                .iter()
                .any(|id| drawing.piece(*id).is_some_and(|p| p.group.is_some()));
            if ui
                .add_enabled(grouped, egui::Button::new(tr("Ungroup")))
                .on_hover_text(tr("Take their groups apart (Shift + G)"))
                .clicked()
            {
                self.ungroup_chosen(document, note);
            }
        });
        self.variety_rows(ui, drawing, &chosen, document, note);
        if let Some(group) = self.chosen_group(drawing) {
            self.group_rows(ui, drawing, group, document, note);
        }
        if let [one] = chosen.as_slice() {
            ui.add_space(6.0);
            self.piece_rows(ui, drawing, first.clone(), document, note);
            let others = drawing.linked_count(*one) - 1;
            if others > 0 {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(crate::trf!("Linked with {} other pieces", others))
                            .color(theme::TEXT_WEAK),
                    );
                    if ui
                        .button(tr("Unlink"))
                        .on_hover_text(tr(
                            "Give this piece a shape of its own: changing it no longer changes \
                             the others",
                        ))
                        .clicked()
                    {
                        self.unlink_piece(document, *one, note);
                    }
                });
            }
        }
    }

    /// The selected pieces' colour and height (shown from the first).
    fn look_rows(
        &mut self,
        ui: &mut Ui,
        first: &shapes::Piece,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        let mut bytes = first.colour.map(crate::material::pictures::byte);
        let picked = crate::dialog::row(ui, tr("Colour"), |ui| {
            ui.color_edit_button_srgb(&mut bytes)
                .on_hover_text(tr("Its colour in the Shape box's Colours output"))
        });
        if picked.clicked() {
            let key = self.shape_key();
            if let Some(s) = self.shape.as_mut() {
                s.colour_key = Some(key);
            }
        }
        if picked.changed() {
            let key = self.shape.as_ref().and_then(|s| s.colour_key);
            let colour = bytes.map(|b| f32::from(b) / 255.0);
            self.colour_chosen(document, Some(colour), None, key, note);
        }
        let mut height = first.height;
        let slid = crate::dialog::row(ui, tr("Height"), |ui| {
            ui.add(egui::Slider::new(&mut height, 0.0..=1.0))
                .on_hover_text(tr(
                    "How high it stands in the Height output: 1 is the box's Height",
                ))
        });
        if slid.changed() {
            if slid.drag_started() || self.shape_field.is_none() {
                self.shape_field = Some(self.shape_key());
            }
            let key = self.shape_field;
            self.colour_chosen(document, None, Some(height), key, note);
        }
        if slid.drag_stopped() {
            self.shape_field = None;
        }
    }

    /// The chosen group's name and its place in the order.
    fn group_rows(
        &mut self,
        ui: &mut Ui,
        drawing: &Arc<Drawing>,
        group: GroupId,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        let Some(g) = drawing.group(group) else {
            return;
        };
        let mut name = g.name.clone();
        let typed = crate::dialog::row(ui, tr("Group"), |ui| ui.text_edit_singleline(&mut name));
        if typed.gained_focus() {
            self.shape_field = Some(self.shape_key());
        }
        if typed.changed() && !name.trim().is_empty() {
            let key = self.shape_field;
            match drawing.with_group_name(group, &name) {
                Ok(d) => report(
                    self.set_drawing(document, d, tr("Rename a group"), key),
                    note,
                ),
                Err(why) => *note = Some(tr(why.describe()).into()),
            }
        }
        ui.horizontal(|ui| {
            let mut changed = None;
            if ui
                .button(tr("Group up"))
                .on_hover_text(tr("Draw the whole group later, over what is above it"))
                .clicked()
            {
                changed = Some((drawing.with_group_order(group, 1), tr("Move a group up")));
            }
            if ui
                .button(tr("Group down"))
                .on_hover_text(tr("Draw the whole group earlier, under what is below it"))
                .clicked()
            {
                changed = Some((drawing.with_group_order(group, -1), tr("Move a group down")));
            }
            if let Some((Ok(d), label)) = changed {
                report(self.set_drawing(document, d, label, None), note);
            }
        });
    }
}
