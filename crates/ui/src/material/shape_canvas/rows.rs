//! Rows beside the shape canvas: the chosen piece's name, what it does,
//! mirroring, order and removing it, and the reference image's.

use super::report;
use crate::material::MaterialEditor;
use crate::text::tr;
use crate::theme;
use egui::{RichText, Ui};
use scene::Document;
use shapes::{Drawing, PieceKind};
use std::sync::Arc;

impl MaterialEditor {
    /// Bringing in, showing and removing the reference image.
    pub(super) fn reference_controls(
        &mut self,
        ui: &mut Ui,
        drawing: &Arc<Drawing>,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        let Some(r) = drawing.reference() else {
            if ui
                .button(tr("Reference image…"))
                .on_hover_text(tr(
                    "A photo or sketch to draw over; it never changes the shape",
                ))
                .clicked()
            {
                self.reference_asked = true;
            }
            return;
        };
        ui.label(RichText::new(&r.name).color(theme::TEXT_WEAK));
        let mut opacity = r.opacity;
        let slid = crate::dialog::row(ui, tr("Opacity"), |ui| {
            ui.add(egui::Slider::new(&mut opacity, 0.0..=1.0))
        });
        if slid.changed() {
            if slid.drag_started() || self.shape_field.is_none() {
                self.shape_field = Some(self.shape_key());
            }
            let key = self.shape_field;
            let changed = drawing.with_reference(Some(Arc::new(r.with_opacity(opacity))));
            let done = self.set_drawing(
                document,
                changed,
                tr("Change the reference image's opacity"),
                key,
            );
            report(done, note);
        }
        if slid.drag_stopped() {
            self.shape_field = None;
        }
        if ui
            .button(tr("Remove image"))
            .on_hover_text(tr("Take the reference image away"))
            .clicked()
        {
            let done = self.set_drawing(
                document,
                drawing.with_reference(None),
                tr("Remove the reference image"),
                None,
            );
            report(done, note);
        }
    }

    /// One piece's name, what it does, mirroring, order, editing points and
    /// removing it.
    pub(super) fn piece_rows(
        &mut self,
        ui: &mut Ui,
        drawing: &Arc<Drawing>,
        piece: shapes::Piece,
        document: &mut Document,
        note: &mut Option<String>,
    ) {
        let id = piece.id;
        let mut name = piece.name.clone();
        let typed = crate::dialog::row(ui, tr("Name"), |ui| ui.text_edit_singleline(&mut name));
        if typed.gained_focus() {
            self.shape_field = Some(self.shape_key());
        }
        if typed.changed() && !name.trim().is_empty() {
            let key = self.shape_field;
            match drawing.with_name(id, &name) {
                Ok(d) => {
                    let done = self.set_drawing(document, d, tr("Rename a piece"), key);
                    report(done, note);
                }
                Err(why) => *note = Some(tr(why.describe()).into()),
            }
        }
        let mut kind = piece.kind;
        crate::dialog::row(ui, tr("Does"), |ui| {
            for k in [PieceKind::Add, PieceKind::Cut, PieceKind::Line] {
                ui.selectable_value(&mut kind, k, tr(k.name()));
            }
        });
        if kind != piece.kind {
            match drawing.with_kind(id, kind) {
                Ok(d) => {
                    let done = self.set_drawing(document, d, tr("Change what a piece does"), None);
                    report(done, note);
                }
                Err(shapes::Problem::OpenPath) => {
                    // A line asked to fill: say why and offer to close it.
                    let closed = drawing
                        .with_path(id, piece.path.clone().closed())
                        .and_then(|d| d.with_kind(id, kind));
                    match closed {
                        Ok(d) => {
                            let done =
                                self.set_drawing(document, d, tr("Close a line and fill it"), None);
                            report(done, note);
                            *note = Some(tr(
                                "An open path can't be filled, so its last point was joined to its first.",
                            )
                            .into());
                        }
                        Err(why) => self.shape_note(tr(why.describe()), Vec::new(), note),
                    }
                }
                Err(why) => self.shape_note(tr(why.describe()), Vec::new(), note),
            }
        }
        let mut mirrored = piece.mirrored;
        let mirror_on = drawing.mirror().on;
        let hint = if mirror_on {
            tr("Drawn on both sides of the mirror line")
        } else {
            tr("Turn the mirror line on (M) to use this")
        };
        if ui
            .add_enabled(
                mirror_on,
                egui::Checkbox::new(&mut mirrored, tr("Mirrored")),
            )
            .on_hover_text(hint)
            .changed()
        {
            let label = tr("Change a piece's mirroring");
            let done = drawing
                .with_mirrored(id, mirrored)
                .map_err(|w| matgraph::Refusal::new(tr(w.describe())));
            if let Ok(d) = done {
                let done = self.set_drawing(document, d, label, None);
                report(done, note);
            }
        }
        ui.horizontal(|ui| {
            let mut changed = None;
            if ui
                .button(tr("Up"))
                .on_hover_text(tr("Draw it later, over the pieces above it"))
                .clicked()
            {
                changed = Some((drawing.with_order(id, 1), tr("Move a piece up")));
            }
            if ui
                .button(tr("Down"))
                .on_hover_text(tr("Draw it earlier, under the pieces below it"))
                .clicked()
            {
                changed = Some((drawing.with_order(id, -1), tr("Move a piece down")));
            }
            if ui.button(tr("Edit points")).clicked()
                && let Some(s) = self.shape.as_mut()
            {
                s.editing = Some(id);
                s.point = None;
            }
            if ui.button(tr("Remove")).clicked() {
                changed = Some((drawing.without(&[id]), tr("Remove pieces")));
            }
            if let Some((Ok(d), label)) = changed {
                let done = self.set_drawing(document, d, label, None);
                report(done, note);
            }
        });
    }
}
