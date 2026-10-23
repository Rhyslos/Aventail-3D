//! The UV menu, in the menu bar and in the UV Editor: the seam tools,
//! turns and flips, laying out and the pieces, in groups (decision 0113).

use super::bars::commands_entries;
use super::{Workbench, piece_settings, seam_preferences, seams, selection, unwrapping};
use crate::objects::DocRequest;
use crate::text::tr;
use egui::Ui;

impl Workbench {
    /// The UV menu: the seam tools, turns and flips, and what is coming,
    /// from the command list (decision 0098).
    pub(super) fn uv_menu_entries(&mut self, ui: &mut Ui, requests: &mut Vec<DocRequest>) {
        ui.menu_button(tr(UV_SEAMS_MENU), |ui| self.uv_seams_entries(ui, requests))
            .response
            .on_hover_text(tr(
                "Mark, clear, suggest and select seams, and say where they may go",
            ));
        ui.menu_button(tr(UV_TURN_MENU), |ui| self.uv_turn_entries(ui, requests))
            .response
            .on_hover_text(tr("Quarter turns and mirrors of the selected UVs"));
        ui.menu_button(tr(UV_LAY_OUT_MENU), |ui| {
            self.uv_lay_out_entries(ui, requests)
        })
        .response
        .on_hover_text(tr(
            "Unwrap, relax, straighten, pack, texel density and trim rows",
        ));
        ui.menu_button(tr(UV_PIECES_MENU), |ui| {
            self.uv_pieces_entries(ui, requests)
        })
        .response
        .on_hover_text(tr(
            "Pin UVs, lock pieces, and give pieces settings and budgets of their own",
        ));
        ui.separator();
        ui.menu_button(tr("Texture"), |ui| {
            crate::uv_editor::texture_menu(ui, &mut self.document);
        });
        ui.menu_button(tr("Trim sheet"), |ui| {
            crate::uv_editor::trim_menu(ui, &mut self.document);
        });
        self.bake_entry(ui);
        ui.separator();
        ui.menu_button(tr(UV_SETTINGS_MENU), |ui| self.automation_menus(ui))
            .response
            .on_hover_text(tr("Every setting of the automatic UV tools"));
    }

    /// UV → Seams.
    fn uv_seams_entries(&mut self, ui: &mut Ui, requests: &mut Vec<DocRequest>) {
        use crate::commands::CommandId;
        if let Some(command) = commands_entries(ui, &[CommandId::MarkSeam, CommandId::ClearSeam]) {
            self.run_command(command, ui.ctx(), requests);
            ui.close();
        }
        for (label, help, run) in [
            (
                tr(seams::SEAMS_FROM_HARD_LABEL),
                tr(seams::SEAMS_FROM_HARD_HELP),
                Self::run_seams_from_hard_edges as fn(&mut Self),
            ),
            (
                tr(seams::SUGGEST_SEAMS_LABEL),
                tr(seams::SUGGEST_SEAMS_HELP),
                Self::run_suggest_seams,
            ),
            (
                tr(seams::SELECT_SEAMS_LABEL),
                tr(seams::SELECT_SEAMS_HELP),
                Self::run_select_seams,
            ),
        ] {
            if ui.button(label).on_hover_text(help).clicked() {
                run(self);
                ui.close();
            }
        }
        ui.separator();
        for (label, help, preference) in [
            (
                tr(seam_preferences::PREFER_LABEL),
                tr(seam_preferences::PREFER_HELP),
                scene::SeamPreference::Prefer,
            ),
            (
                tr(seam_preferences::NEVER_LABEL),
                tr(seam_preferences::NEVER_HELP),
                scene::SeamPreference::Never,
            ),
            (
                tr(seam_preferences::CLEAR_PREFERENCE_LABEL),
                tr(seam_preferences::CLEAR_PREFERENCE_HELP),
                scene::SeamPreference::Free,
            ),
        ] {
            if ui.button(label).on_hover_text(help).clicked() {
                self.run_seam_preference(preference);
                ui.close();
            }
        }
    }

    /// UV → Turn and flip.
    fn uv_turn_entries(&mut self, ui: &mut Ui, requests: &mut Vec<DocRequest>) {
        use crate::commands::CommandId;
        if let Some(command) = commands_entries(
            ui,
            &[
                CommandId::UvTurnLeft,
                CommandId::UvTurnRight,
                CommandId::UvFlipU,
                CommandId::UvFlipV,
            ],
        ) {
            self.run_command(command, ui.ctx(), requests);
            ui.close();
        }
    }

    /// UV → Lay out.
    fn uv_lay_out_entries(&mut self, ui: &mut Ui, requests: &mut Vec<DocRequest>) {
        use crate::commands::CommandId;
        if let Some(command) = commands_entries(ui, &[CommandId::Unwrap]) {
            self.run_command(command, ui.ctx(), requests);
            ui.close();
        }
        for (label, help, axis) in [
            (
                tr(unwrapping::UNFOLD_ACROSS_LABEL),
                tr(unwrapping::UNFOLD_ACROSS_HELP),
                scene::UnfoldAxis::Across,
            ),
            (
                tr(unwrapping::UNFOLD_UP_LABEL),
                tr(unwrapping::UNFOLD_UP_HELP),
                scene::UnfoldAxis::Up,
            ),
        ] {
            if ui.button(label).on_hover_text(help).clicked() {
                self.run_uv_tool(unwrapping::UvTool::Unfold(axis));
                ui.close();
            }
        }
        if let Some(command) = commands_entries(
            ui,
            &[CommandId::Relax, CommandId::Straighten, CommandId::Pack],
        ) {
            self.run_command(command, ui.ctx(), requests);
            ui.close();
        }
        ui.separator();
        for (label, help, to_aim) in [
            (
                tr(unwrapping::SET_DENSITY_LABEL),
                tr(unwrapping::SET_DENSITY_HELP),
                true,
            ),
            (
                tr(unwrapping::MATCH_DENSITY_LABEL),
                tr(unwrapping::MATCH_DENSITY_HELP),
                false,
            ),
        ] {
            if ui.button(label).on_hover_text(help).clicked() {
                self.run_texel_density(to_aim);
                ui.close();
            }
        }
        ui.menu_button(tr(unwrapping::PUT_ON_TRIM_LABEL), |ui| {
            self.put_on_trim_menu(ui);
        });
    }

    /// UV → Pieces.
    fn uv_pieces_entries(&mut self, ui: &mut Ui, requests: &mut Vec<DocRequest>) {
        use crate::commands::CommandId;
        // Also in the Select menu (decision 0118).
        if ui
            .add(
                egui::Button::new(tr(selection::CONNECTED_LABEL)).shortcut_text(
                    ui.ctx()
                        .format_shortcut(&crate::selecting::CONNECTED_SHORTCUT),
                ),
            )
            .on_hover_text(tr(
                "Select the whole pieces the selection is in: a little of a piece \
                 (a point, an edge or a polygon) brings the whole piece, parted at seams. \
                 Use it to pick one of two overlapping pieces and move it away.",
            ))
            .clicked()
        {
            self.select_requests
                .push(selection::SelectRequest::ConnectedInUvs);
            ui.close();
        }
        ui.separator();
        if let Some(command) = commands_entries(ui, &[CommandId::UvPin, CommandId::UvUnpin]) {
            self.run_command(command, ui.ctx(), requests);
            ui.close();
        }
        ui.separator();
        for (label, help, lock) in [
            (
                tr(piece_settings::LOCK_LABEL),
                tr(piece_settings::LOCK_HELP),
                true,
            ),
            (
                tr(piece_settings::UNLOCK_LABEL),
                tr(piece_settings::UNLOCK_HELP),
                false,
            ),
        ] {
            if ui.button(label).on_hover_text(help).clicked() {
                self.run_lock(lock);
                ui.close();
            }
        }
        ui.menu_button(tr(piece_settings::PIECE_SETTINGS_LABEL), |ui| {
            self.piece_settings_menu(ui);
        })
        .response
        .on_hover_text(tr("Give the selected pieces their own unwrap settings"));
        ui.menu_button(tr(seam_preferences::BUDGET_LABEL), |ui| {
            self.budget_menu(ui)
        })
        .response
        .on_hover_text(tr("Give the selected pieces a stretch budget of their own"));
    }
}

/// The UV Editor's own UV menu (issue #115).
pub(crate) const UV_EDITOR_UV_MENU: &str = crate::text!("UV");

/// The UV menu's groups (the project lead's choice, decision 0113).
pub(crate) const UV_SEAMS_MENU: &str = crate::text!("Seams");

pub(crate) const UV_TURN_MENU: &str = crate::text!("Turn and flip");

pub(crate) const UV_LAY_OUT_MENU: &str = crate::text!("Lay out");

pub(crate) const UV_PIECES_MENU: &str = crate::text!("Pieces");

pub(crate) const UV_SETTINGS_MENU: &str = crate::text!("Settings");
