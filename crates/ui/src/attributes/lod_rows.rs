//! The Levels of detail section of the Attributes panel
//! ([decision 0155](../../../../docs/decisions/0155-levels-of-detail.md)):
//! whether the object has levels, how many, each one's share of the
//! triangles and the distance it is shown from, with its triangles once
//! made, and which level the viewport shows.

use super::curve_rows::Rows;
use super::{AttributesPanel, FIELD_WIDTH, LABEL_WIDTH, label, section, stepper};
use crate::objects::DocRequest;
use crate::text::tr;
use crate::theme;
use crate::units::Units;
use crate::workbench::exporting::thousands as triangles;
use egui::{RichText, Ui, vec2};
use scene::{Command, DecimateOptions, Document, Lods, ObjectId};

/// The name of the choice of level shown.
const PREVIEW: &str = crate::text!("Preview");

/// A level's name: the full shape for 0.
fn level_name(level: usize) -> String {
    if level == 0 {
        tr("Full shape").to_string()
    } else {
        crate::trf!("Level {}", level)
    }
}

impl AttributesPanel {
    /// The Levels of detail section, for an object with a surface.
    pub(super) fn lods_section(
        &mut self,
        ui: &mut Ui,
        doc: &Document,
        id: ObjectId,
        units: Units,
        out: &mut Vec<DocRequest>,
    ) {
        let Some(object) = doc.scene().object(id) else {
            return;
        };
        section(ui, tr("Levels of detail"));
        let mut lods = object.lods.clone();
        let mut rows = Rows::default();
        let made = doc.scene().lods_if_ready(id);
        egui::Grid::new(("attributes_lods", id))
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
                if !lods.on {
                    return;
                }
                count_row(ui, &mut lods, &mut rows);
                let counts = made.as_ref().map(|m| m.triangles());
                for (k, level) in lods.levels.iter_mut().enumerate() {
                    level_rows(ui, k + 1, level, units, &mut rows);
                    label(
                        ui,
                        tr("Triangles"),
                        tr("How many triangles this level has, as a game counts them"),
                    );
                    let shown = counts
                        .as_ref()
                        .and_then(|c| c.get(k))
                        .map_or_else(|| "…".to_string(), |n| triangles(*n));
                    ui.label(RichText::new(shown).color(theme::TEXT_WEAK));
                    ui.end_row();
                }
                preview_row(ui, &mut lods, &mut rows);
            });
        if lods.on
            && let Some(made) = &made
        {
            match &made.problem {
                Some(why) => {
                    ui.label(RichText::new(why).color(theme::WARNING));
                }
                None => {
                    ui.label(
                        RichText::new(crate::trf!(
                            "The full shape has {} triangles.",
                            triangles(made.full)
                        ))
                        .small()
                        .color(theme::TEXT_WEAK),
                    );
                }
            }
        }
        if rows.began {
            self.gesture += 1;
        }
        if rows.changed && lods != object.lods {
            out.push(DocRequest::RunMerging(
                Command::SetLods {
                    ids: vec![id],
                    lods,
                },
                self.gesture,
            ));
        }
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

/// One level's share of the triangles and its distance.
fn level_rows(
    ui: &mut Ui,
    level: usize,
    settings: &mut scene::LodLevel,
    units: Units,
    rows: &mut Rows,
) {
    let name = level_name(level);
    ui.label(RichText::new(&name).strong()).on_hover_text(tr(
        "How much of the full shape this level keeps, as a share of its triangles. A level \
         with a larger share than the one before is made the same as that one",
    ));
    rows.take(stepper::percent(
        ui,
        &mut settings.ratio,
        (DecimateOptions::MIN_RATIO, 1.0),
        &name,
    ));
    ui.end_row();
    let from = tr("Shown from");
    label(
        ui,
        from,
        tr(
            "How far from the camera a game starts showing this level instead of the one before. \
             It goes into the exported file for the game to use",
        ),
    );
    rows.take(stepper::length(
        ui,
        &mut settings.distance,
        (0.0, Lods::MAX_DISTANCE),
        units,
        from,
    ));
    ui.end_row();
}

/// Which level the viewport shows.
fn preview_row(ui: &mut Ui, lods: &mut Lods, rows: &mut Rows) {
    label(
        ui,
        tr(PREVIEW),
        tr(
            "Which level the viewport shows in the shape's place, to check how it looks from \
             afar. Only the view changes: you still select and edit the full shape",
        ),
    );
    let current = lods.preview;
    let response = egui::ComboBox::from_id_salt(ui.id().with("lod_preview"))
        .selected_text(level_name(current))
        .width(FIELD_WIDTH)
        .show_ui(ui, |ui| {
            for level in 0..=lods.levels.len() {
                if ui
                    .selectable_label(level == current, level_name(level))
                    .clicked()
                    && level != current
                {
                    lods.preview = level;
                    rows.changed = true;
                    rows.began = true;
                }
            }
        })
        .response;
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, tr(PREVIEW)));
    ui.end_row();
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
