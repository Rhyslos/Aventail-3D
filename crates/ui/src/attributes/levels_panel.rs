//! The Levels panel ([decision 0165](../../../../docs/decisions/0165-levels-panel.md)):
//! the selected object's levels of detail, a tab beside Attributes. Each
//! level has a row with its preview switch, its triangles against a budget
//! and the distance it is shown from, then how it is made: from the model
//! with its own changes to the modifiers and an automatic reduction, or by
//! hand, with *Edit this level*. Every change is a command; one drag of a
//! value is one undo step.

use super::curve_rows::Rows;
use super::{AttributesPanel, LABEL_WIDTH, label, section, stepper};
use crate::objects::DocRequest;
use crate::text::tr;
use crate::theme;
use crate::workbench::exporting::thousands;
use egui::{RichText, Ui, vec2};
use scene::{Command, Document, Lods, ObjectId};

/// What one level's rows need besides the level.
pub(super) struct LevelContext<'a> {
    pub(super) doc: &'a Document,
    /// The object with the levels.
    pub(super) id: ObjectId,
    /// Its place (0 for Level 1).
    pub(super) k: usize,
    /// Its triangles, once made.
    pub(super) triangles: Option<usize>,
    pub(super) units: crate::units::Units,
}

/// A level's name: the full shape for 0.
pub(crate) fn level_name(level: usize) -> String {
    if level == 0 {
        tr("Full shape").to_string()
    } else {
        crate::trf!("Level {}", level)
    }
}

/// The object whose levels the panel shows for `doc`'s selection: the
/// first selected shape, or, while a level made by hand is edited, its
/// object.
pub(crate) fn levels_object(doc: &Document) -> Option<ObjectId> {
    let scene = doc.scene();
    doc.selection().into_iter().find_map(|id| {
        if let Some((owner, _)) = scene.level_owner(id) {
            return Some(owner);
        }
        scene
            .object(id)
            .is_some_and(|o| o.kind.has_shape() && !matches!(o.kind, scene::ObjectKind::Lattice(_)))
            .then_some(id)
    })
}

impl AttributesPanel {
    /// The Levels panel's contents.
    pub(crate) fn levels_panel(
        &mut self,
        ui: &mut Ui,
        doc: &Document,
        units: crate::units::Units,
        out: &mut Vec<DocRequest>,
    ) {
        let Some(id) = levels_object(doc) else {
            ui.label(
                RichText::new(tr(
                    "Select a shape to see its levels of detail: simpler copies a game draws \
                     when it is far away.",
                ))
                .color(theme::TEXT_WEAK),
            );
            return;
        };
        let Some(object) = doc.scene().object(id) else {
            return;
        };
        ui.label(RichText::new(&object.name).strong());
        if let Some((_, k, _)) = crate::drawing::edited_level(doc) {
            ui.label(
                RichText::new(crate::trf!(
                    "You are editing {}'s own shape: the viewport shows it in the object's \
                     place.",
                    level_name(k + 1)
                ))
                .color(theme::accent(ui.ctx()).colors().light),
            );
        }
        let mut lods = object.lods.clone();
        let mut rows = Rows::default();
        egui::Grid::new(("levels_panel_top", id))
            .num_columns(2)
            .min_col_width(LABEL_WIDTH)
            .spacing(vec2(12.0, 6.0))
            .show(ui, |ui| {
                let name = tr("Make levels");
                label(
                    ui,
                    name,
                    tr(
                        "Make simpler copies of the shape for a game to draw when it is far away, \
                         where the detail can't be seen anyway. The shape itself doesn't change",
                    ),
                );
                rows.toggle(ui, &mut lods.on, name);
                ui.end_row();
                if lods.on {
                    count_row(ui, &mut lods, &mut rows);
                }
            });
        if !lods.on {
            self.finish_lods(id, &object.lods, lods, rows, out);
            return;
        }
        let made = doc.scene().lods_if_ready(id);
        let counts = made.as_ref().map(|m| m.triangles());
        self.full_row(ui, &mut lods, made.as_ref().map(|m| m.full), &mut rows);
        for k in 0..lods.levels.len() {
            ui.push_id(("level", id, k), |ui| {
                let cx = LevelContext {
                    doc,
                    id,
                    k,
                    triangles: counts.as_ref().and_then(|c| c.get(k).copied()),
                    units,
                };
                self.level_rows(ui, &cx, &mut lods, &mut rows, out);
            });
        }
        if let Some(why) = made.as_ref().and_then(|m| m.problem.as_ref()) {
            ui.label(RichText::new(why).color(theme::WARNING));
        }
        ui.add_space(6.0);
        ui.label(
            RichText::new(tr(
                "A game shows each level from its distance on. Export with Levels of detail \
                 ticked (Export window, Includes) to write them beside the object.",
            ))
            .small()
            .color(theme::TEXT_WEAK),
        );
        self.finish_lods(id, &object.lods, lods, rows, out);
    }

    /// Sends the levels changed in the panel, if they were, as one undo
    /// step per gesture.
    fn finish_lods(
        &mut self,
        id: ObjectId,
        before: &Lods,
        lods: Lods,
        rows: Rows,
        out: &mut Vec<DocRequest>,
    ) {
        if rows.began {
            self.gesture += 1;
        }
        if rows.changed && lods != *before {
            out.push(DocRequest::RunMerging(
                Command::SetLods {
                    ids: vec![id],
                    lods,
                },
                self.gesture,
            ));
        }
    }

    /// The full shape's row: its preview switch and triangles.
    fn full_row(&mut self, ui: &mut Ui, lods: &mut Lods, full: Option<usize>, rows: &mut Rows) {
        section(ui, &level_name(0));
        level_header(ui, lods, 0, rows, |ui| {
            let shown = full.map_or_else(|| "…".to_string(), thousands);
            ui.label(RichText::new(crate::trf!("{} triangles", shown)).color(theme::TEXT_WEAK))
                .on_hover_text(tr(
                    "How many triangles the full shape has as it is exported, as a game counts \
                     them",
                ));
        });
    }
}

/// How many levels.
fn count_row(ui: &mut Ui, lods: &mut Lods, rows: &mut Rows) {
    label(
        ui,
        tr("Levels"),
        tr("How many simpler copies: each one is shown from farther away than the one before"),
    );
    let mut count = lods.levels.len() as u32;
    let touched = stepper::count(ui, &mut count, (1, Lods::MAX_LEVELS as u32), tr("Levels"));
    if touched.changed {
        *lods = lods.with_count(count as usize);
    }
    rows.take(touched);
    ui.end_row();
}

/// A level's first line: its preview switch (highlighted while the
/// viewport shows it), then `rest`.
pub(super) fn level_header(
    ui: &mut Ui,
    lods: &mut Lods,
    level: usize,
    rows: &mut Rows,
    rest: impl FnOnce(&mut Ui),
) {
    let previewed = lods.preview == level;
    let accent = theme::accent(ui.ctx()).colors();
    let fill = if previewed {
        accent.soft
    } else {
        egui::Color32::TRANSPARENT
    };
    egui::Frame::new()
        .fill(fill)
        .corner_radius(3.0)
        .inner_margin(egui::Margin::symmetric(4, 2))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let name = crate::trf!("Preview {}", level_name(level));
                let button = ui
                    .add(egui::Button::selectable(previewed, tr("Preview")))
                    .on_hover_text(tr(
                        "Show this level in the viewport in the shape's place, to check how it \
                         looks from afar. Only the view changes",
                    ));
                button.widget_info(|| {
                    egui::WidgetInfo::selected(egui::WidgetType::Button, true, previewed, &name)
                });
                if button.clicked() && !previewed {
                    lods.preview = level;
                    rows.changed = true;
                    rows.began = true;
                }
                rest(ui);
            });
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_are_named_from_the_full_shape() {
        assert_eq!(level_name(0), "Full shape");
        assert_eq!(level_name(2), "Level 2");
    }
}
