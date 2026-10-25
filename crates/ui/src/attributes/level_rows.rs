//! One level's rows in the Levels panel (decision 0165): its header, the
//! distance it is shown from and its budget, then how it is made. From the
//! model: its own changes to each modifier (on or off, its own settings)
//! and its automatic reduction. By hand: *Edit this level*, the note when
//! the model has changed since, and starting it again.

use super::curve_rows::Rows;
use super::levels_panel::{LevelContext, level_header, level_name};
use super::{AttributesPanel, LABEL_WIDTH, label, stepper};
use crate::objects::DocRequest;
use crate::text::tr;
use crate::theme;
use crate::workbench::exporting::thousands;
use egui::{RichText, Ui, vec2};
use scene::{Command, DecimateOptions, Lods, Modifier};

/// The name of the switch that edits a level made by hand.
pub(crate) const EDIT_LEVEL: &str = crate::text!("Edit this level");
/// The name of the button that makes a level by hand.
pub(crate) const MAKE_BY_HAND: &str = crate::text!("Make this level by hand");

impl AttributesPanel {
    /// Level `cx.k`'s rows; changes to the settings go into `lods`, and
    /// commands of their own into `out`.
    pub(super) fn level_rows(
        &mut self,
        ui: &mut Ui,
        cx: &LevelContext,
        lods: &mut Lods,
        rows: &mut Rows,
        out: &mut Vec<DocRequest>,
    ) {
        let k = cx.k;
        let scene = cx.doc.scene();
        let hand = scene.level_shape(cx.id, k).map(|(shape, _)| shape);
        super::section(ui, &level_name(k + 1));
        let budget = lods.levels[k].budget;
        level_header(ui, lods, k + 1, rows, |ui| {
            triangles_label(ui, cx.triangles, budget);
        });
        let level = &mut lods.levels[k];
        egui::Grid::new(("level_settings", cx.id, k))
            .num_columns(2)
            .min_col_width(LABEL_WIDTH)
            .spacing(vec2(12.0, 6.0))
            .show(ui, |ui| {
                let from = tr("Shown from");
                label(
                    ui,
                    from,
                    tr(
                        "How far from the camera a game starts showing this level instead of the \
                         one before. It goes into the exported file for the game to use",
                    ),
                );
                rows.take(stepper::length(
                    ui,
                    &mut level.distance,
                    (0.0, Lods::MAX_DISTANCE),
                    cx.units,
                    from,
                ));
                ui.end_row();
                let name = tr("Budget");
                label(
                    ui,
                    name,
                    tr(
                        "The triangles you aim for at this level (0 for none). Its count turns \
                         amber when it has more; nothing is changed to fit",
                    ),
                );
                rows.take(stepper::big_count(
                    ui,
                    &mut level.budget,
                    (0, Lods::MAX_BUDGET),
                    100,
                    name,
                ));
                ui.end_row();
                if hand.is_none() {
                    let reduce = tr("Reduce");
                    label(
                        ui,
                        reduce,
                        tr(
                            "Take triangles away automatically as a last step, keeping this share \
                             of the level's own triangles (after its changes to the modifiers), \
                             with Decimate's work",
                        ),
                    );
                    ui.horizontal(|ui| {
                        rows.toggle(ui, &mut level.reduce, reduce);
                        if level.reduce {
                            rows.take(stepper::percent(
                                ui,
                                &mut level.ratio,
                                (DecimateOptions::MIN_RATIO, 1.0),
                                &crate::trf!("{} reduced to", level_name(k + 1)),
                            ));
                        }
                    });
                    ui.end_row();
                }
            });
        match hand {
            None => {
                self.modifier_changes(ui, cx, lods, rows);
                let button = ui.button(tr(MAKE_BY_HAND)).on_hover_text(tr(
                    "Copy what this level shows into a shape of its own, to model by hand. It no \
                     longer follows the model; the viewport shows it while you edit it",
                ));
                if button.clicked() {
                    out.push(DocRequest::Run(Command::MakeLevelByHand {
                        id: cx.id,
                        level: k,
                    }));
                }
            }
            Some(shape) => hand_rows(ui, cx, shape, out),
        }
    }

    /// The level's changes to each of the object's modifiers: on or off at
    /// this level, and settings of its own.
    fn modifier_changes(
        &mut self,
        ui: &mut Ui,
        cx: &LevelContext,
        lods: &mut Lods,
        rows: &mut Rows,
    ) {
        let scene = cx.doc.scene();
        let modifiers: Vec<Modifier> = scene.modifiers(scene.modifier_owner(cx.id)).to_vec();
        if modifiers.is_empty() {
            return;
        }
        ui.label(
            RichText::new(tr("Modifiers at this level"))
                .color(theme::TEXT_WEAK)
                .small(),
        )
        .on_hover_text(tr(
            "Switch modifiers off, or give them simpler settings, for this level only. The model \
             and the other levels don't change, and the level still follows every edit of the \
             model",
        ));
        let level = &mut lods.levels[cx.k];
        let at_level = level.applied(&modifiers);
        for (m, here) in modifiers.iter().zip(at_level) {
            ui.push_id(("level_modifier", m.id.0), |ui| {
                let changed = level.override_for(m.id).is_some();
                ui.horizontal(|ui| {
                    let mut on = here.on;
                    let name = crate::trf!("{} at {}", m.kind.name(), level_name(cx.k + 1));
                    let switch = ui.checkbox(&mut on, m.kind.name());
                    switch.widget_info(|| {
                        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, on, &name)
                    });
                    if switch.changed() {
                        let settings = level.override_for(m.id).and_then(|o| o.settings.clone());
                        *level = level.with_override(&modifiers, m.id, on, settings);
                        rows.changed = true;
                        rows.began = true;
                    }
                    if changed {
                        ui.label(
                            RichText::new(tr("changed here"))
                                .small()
                                .color(theme::WARNING),
                        );
                        let back = ui.small_button(tr("Same as the model")).on_hover_text(tr(
                            "Take this level's change away: the modifier is as the model has it",
                        ));
                        if back.clicked() {
                            *level = level.with_override(&modifiers, m.id, m.on, None);
                            rows.changed = true;
                            rows.began = true;
                        }
                    }
                });
                if !here.on {
                    return;
                }
                egui::CollapsingHeader::new(RichText::new(tr("Settings here")).small())
                    .id_salt(("level_modifier_settings", cx.id, cx.k, m.id.0))
                    .show(ui, |ui| {
                        let mut kind = here.kind.clone();
                        let mut edited = Rows::default();
                        egui::Grid::new(("level_modifier_grid", cx.id, cx.k, m.id.0))
                            .num_columns(2)
                            .min_col_width(LABEL_WIDTH)
                            .spacing(vec2(12.0, 6.0))
                            .show(ui, |ui| {
                                super::modifier_rows::level_kind_rows(
                                    ui,
                                    &mut kind,
                                    cx.units,
                                    &mut edited,
                                );
                            });
                        rows.began |= edited.began;
                        if edited.changed && kind != here.kind {
                            *level = level.with_override(&modifiers, m.id, here.on, Some(kind));
                            rows.changed = true;
                        }
                    });
            });
        }
    }
}

/// A level's triangles, against its budget if it has one (amber when
/// over).
fn triangles_label(ui: &mut Ui, triangles: Option<usize>, budget: u32) {
    let shown = triangles.map_or_else(|| "…".to_string(), thousands);
    let over = budget > 0 && triangles.is_some_and(|t| t > budget as usize);
    let text = if budget > 0 {
        crate::trf!("{} of {} triangles", shown, thousands(budget as usize))
    } else {
        crate::trf!("{} triangles", shown)
    };
    let color = if over {
        theme::WARNING
    } else {
        theme::TEXT_WEAK
    };
    ui.label(RichText::new(text).color(color)).on_hover_text(tr(
        "How many triangles this level has as it is exported, as a game counts them, and the \
         budget set for it",
    ));
}

/// The rows of a level made by hand: Edit this level, whether the model
/// has changed since, and starting again.
fn hand_rows(ui: &mut Ui, cx: &LevelContext, shape: scene::ObjectId, out: &mut Vec<DocRequest>) {
    let scene = cx.doc.scene();
    let editing = cx.doc.is_selected(shape);
    ui.horizontal(|ui| {
        let edit = ui
            .add(egui::Button::selectable(editing, tr(EDIT_LEVEL)))
            .on_hover_text(tr(
                "Show this level's own shape in the viewport in the object's place, and edit it \
                 with every mode and tool. Click again (or select something else) to stop",
            ));
        edit.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::Button, true, editing, tr(EDIT_LEVEL))
        });
        if edit.clicked() {
            let select = if editing { cx.id } else { shape };
            out.push(DocRequest::Select(vec![select]));
        }
        let back = ui.button(tr("Make from the model")).on_hover_text(tr(
            "Delete this level's own shape: the level is made from the model again",
        ));
        if back.clicked() {
            out.push(DocRequest::Run(Command::Delete(vec![shape])));
        }
    });
    let name = scene.object(shape).map_or("", |o| o.name.as_str());
    ui.label(
        RichText::new(crate::trf!(
            "Made by hand: its shape is {}, inside the object in the Objects panel. It doesn't \
             follow changes to the model.",
            name
        ))
        .small()
        .color(theme::TEXT_WEAK),
    );
    if scene.level_drifted(cx.id, cx.k) == Some(true) {
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(tr("The model has changed since this level was made."))
                    .color(theme::WARNING),
            );
            if ui
                .button(tr("Start again from the model"))
                .on_hover_text(tr(
                    "Replace this level's shape with what it would show from the model now. Its \
                     edits go (undo brings them back)",
                ))
                .clicked()
            {
                out.push(DocRequest::Run(Command::RestartLevel {
                    id: cx.id,
                    level: cx.k,
                }));
            }
        });
    }
}
