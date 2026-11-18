//! A script command's settings (decision 0305): shown while its run is
//! still the last thing done, as a tool's Last edit is. Changing one asks
//! the workbench to run the command again with it, in place of the run
//! before (one undo step).

use super::{AttributesPanel, LABEL_WIDTH, section};
use crate::text::tr;
use crate::theme;
use crate::units::Units;
use egui::{RichText, Ui, vec2};
use script::{SettingKind, SettingValue};

impl AttributesPanel {
    /// The script command's section, if one is live.
    pub(super) fn script_section(&mut self, ui: &mut Ui, _units: Units) {
        let Some(view) = self.script.clone() else {
            return;
        };
        section(ui, &view.name);
        let mut values = view.values.clone();
        let mut changed = false;
        ui.push_id("script_settings", |ui| {
            egui::Grid::new("attributes_script")
                .num_columns(2)
                .min_col_width(LABEL_WIDTH)
                .spacing(vec2(12.0, 6.0))
                .show(ui, |ui| {
                    for setting in &view.settings {
                        let Some((_, value)) = values.iter_mut().find(|(n, _)| *n == setting.name)
                        else {
                            continue;
                        };
                        let label = ui.label(RichText::new(&setting.words).color(theme::TEXT_WEAK));
                        let mut help = crate::trf!("The script's setting {}", setting.name);
                        if view.count.as_deref() == Some(setting.name.as_str()) {
                            help.push_str(tr(" (Ctrl + wheel in the view changes it)"));
                        }
                        label.on_hover_text(help);
                        changed |= value_row(ui, &setting.words, &setting.kind, value);
                        ui.end_row();
                    }
                });
        });
        let hidden = view
            .settings
            .iter()
            .filter(|s| !crate::workbench::shown_kind(&s.kind))
            .count();
        let hint = if view.running {
            tr("Running with these settings…").to_string()
        } else if view.settings.is_empty() {
            tr("This command has no settings. Ctrl + Z takes its changes back.").to_string()
        } else {
            tr("Change a setting and the command runs again in place: still one step Ctrl + Z takes back.")
                .to_string()
        };
        ui.label(RichText::new(hint).small().color(theme::TEXT_WEAK));
        if hidden > 0 {
            ui.label(
                RichText::new(crate::trf!(
                    "{} of its settings can't be shown here, so they keep the values the script gives them.",
                    hidden
                ))
                .small()
                .color(theme::TEXT_WEAK),
            );
        }
        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);
        if changed {
            self.script_asked = Some(values);
        }
    }
}

/// One setting's control. Gives back whether it changed.
fn value_row(ui: &mut Ui, name: &str, kind: &SettingKind, value: &mut SettingValue) -> bool {
    match (kind, value) {
        (SettingKind::Bool, SettingValue::Bool(on)) => {
            let response = ui.checkbox(on, "");
            let shown = *on;
            response.widget_info(|| {
                egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, shown, name)
            });
            response.changed()
        }
        (SettingKind::Int, SettingValue::Int(n)) => {
            let mut whole = (*n).clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
            let changed = ui
                .add(egui::DragValue::new(&mut whole).speed(0.1))
                .changed();
            if changed {
                *n = i64::from(whole);
            }
            changed
        }
        (SettingKind::Float, SettingValue::Float(n)) => {
            let mut x = if n.is_finite() { *n } else { 0.0 };
            let changed = ui
                .add(egui::DragValue::new(&mut x).speed(0.01).max_decimals(4))
                .changed();
            if changed && x.is_finite() {
                *n = x;
            }
            changed
        }
        (SettingKind::Text, SettingValue::Text(text)) => ui
            .add(egui::TextEdit::singleline(text).desired_width(140.0))
            .changed(),
        (SettingKind::Choice(options), SettingValue::Choice(chosen)) => {
            let mut changed = false;
            egui::ComboBox::from_id_salt(("script setting", name))
                .selected_text(script::words_of(chosen))
                .show_ui(ui, |ui| {
                    for option in options {
                        if ui
                            .selectable_label(option == chosen, script::words_of(option))
                            .clicked()
                            && option != chosen
                        {
                            chosen.clone_from(option);
                            changed = true;
                        }
                    }
                });
            changed
        }
        (SettingKind::Vec3, SettingValue::Vec3(v)) => {
            let mut changed = false;
            ui.horizontal(|ui| {
                for n in v.iter_mut() {
                    changed |= ui
                        .add(egui::DragValue::new(n).speed(0.01).max_decimals(4))
                        .changed();
                }
            });
            changed
        }
        (SettingKind::Colour, SettingValue::Colour(c)) => {
            let mut rgba = [c[0] as f32, c[1] as f32, c[2] as f32, c[3] as f32];
            let changed = ui.color_edit_button_rgba_unmultiplied(&mut rgba).changed();
            if changed {
                *c = rgba.map(f64::from);
            }
            changed
        }
        _ => {
            ui.label(RichText::new(tr("(set by the script)")).weak());
            false
        }
    }
}
