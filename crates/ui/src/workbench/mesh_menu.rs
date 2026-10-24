//! The Mesh menu: the modelling tools in groups, each a submenu one
//! level down (decision 0126).

use super::Workbench;
use super::bars::commands_entries;
use crate::commands::CommandId;
use crate::objects::{self, DocRequest};
use crate::text::tr;
use egui::Ui;

/// The Mesh menu's groups (decision 0126): what it holds was one long
/// list.
pub(crate) const MESH_BUILD_MENU: &str = crate::text!("Build");

pub(crate) const MESH_CUT_MENU: &str = crate::text!("Cut and join");

pub(crate) const MESH_REMOVE_MENU: &str = crate::text!("Remove");

pub(crate) const MESH_SHADING_MENU: &str = crate::text!("Shading");

pub(crate) const MESH_OBJECT_MENU: &str = crate::text!("Shape and object");

pub(crate) const MESH_BOOLEANS_MENU: &str = crate::text!("Booleans");

pub(crate) const MESH_MODIFIERS_MENU: &str = crate::text!("Modifiers");

pub(crate) const MESH_CURVE_MENU: &str = crate::text!("Curve");

impl Workbench {
    /// Menu entries for `commands`, greyed out with the reason when they
    /// can't be used now.
    fn ready_entries(
        &mut self,
        ui: &mut Ui,
        commands: &[CommandId],
        requests: &mut Vec<DocRequest>,
    ) {
        for command in commands {
            let info = command.info();
            let why = match self.readiness(*command) {
                crate::tool_menu::Ready::NotNow(why) => Some(why),
                _ => None,
            };
            let response = ui.add_enabled(why.is_none(), egui::Button::new(info.name));
            let response = match &why {
                Some(why) => response.on_disabled_hover_text(format!("{}\n{why}.", info.help)),
                None => response.on_hover_text(info.help),
            };
            if response.clicked() {
                self.run_command(*command, ui.ctx(), requests);
                ui.close();
            }
        }
    }

    /// The Mesh menu (decision 0126): the modelling tools in groups, each
    /// a submenu one level down, so the menu is short and every command is
    /// found where people look for it.
    pub(super) fn mesh_menu(&mut self, ui: &mut Ui, requests: &mut Vec<DocRequest>) {
        let groups: [(&str, &str, &[CommandId]); 3] = [
            (
                tr(MESH_BUILD_MENU),
                tr("Add to the shape: extrude, inset, bevel, groove, bridge, fill a hole"),
                &[
                    CommandId::Extrude,
                    CommandId::Inset,
                    CommandId::Bevel,
                    CommandId::Groove,
                    CommandId::Bridge,
                    CommandId::FillHole,
                ],
            ),
            (
                tr(MESH_CUT_MENU),
                tr("Cut new edges and join points: slit, loop cut, connect, weld"),
                &[
                    CommandId::Slit,
                    CommandId::LoopCut,
                    CommandId::Connect,
                    CommandId::Weld,
                ],
            ),
            (
                tr(MESH_REMOVE_MENU),
                tr("Take points, edges or polygons away"),
                &[CommandId::DeleteElements, CommandId::Dissolve],
            ),
        ];
        for (title, help, commands) in groups {
            crate::dialog::menu(ui, title, |ui| {
                if let Some(command) = commands_entries(ui, commands) {
                    self.run_command(command, ui.ctx(), requests);
                    ui.close();
                }
            })
            .response
            .on_hover_text(help);
        }
        crate::dialog::menu(ui, tr(MESH_CURVE_MENU), |ui| {
            use CommandId as C;
            self.ready_entries(
                ui,
                &[C::CurveSplit, C::CurveReverse, C::CurveOpenClose],
                requests,
            );
            ui.separator();
            ui.label(
                egui::RichText::new(tr("Handle type"))
                    .small()
                    .color(crate::theme::TEXT_WEAK),
            );
            self.ready_entries(ui, C::CurveHandles.members(), requests);
            ui.separator();
            self.ready_entries(ui, &[C::CurveRound, C::CurveEven], requests);
        })
        .response
        .on_hover_text(tr(
            "Edit curves: split, reverse, open or close, handle types, round corners, even \
             spacing. M and A on two end points join them.",
        ));
        crate::dialog::menu(ui, tr(MESH_BOOLEANS_MENU), |ui| {
            self.ready_entries(ui, &crate::booleans::COMMANDS, requests);
        })
        .response
        .on_hover_text(tr(
            "Union, subtract or intersect: select the shape to keep, then Ctrl + click the \
             shapes that cut it",
        ));
        crate::dialog::menu(ui, tr(MESH_MODIFIERS_MENU), |ui| {
            for (index, (family, commands)) in crate::modifiers::menu_groups().iter().enumerate() {
                if index > 0 {
                    ui.separator();
                }
                ui.label(
                    egui::RichText::new(*family)
                        .small()
                        .color(crate::theme::TEXT_WEAK),
                );
                self.ready_entries(ui, commands, requests);
            }
        })
        .response
        .on_hover_text(tr(
            "Add a modifier to the selected shapes: a change worked out live, which you can \
             switch off, reorder or remove in the Attributes panel",
        ));
        crate::dialog::menu(ui, tr(MESH_SHADING_MENU), |ui| {
            self.smoothing_entry(ui);
            if let Some(command) =
                commands_entries(ui, &[CommandId::HardEdges, CommandId::SoftEdges])
            {
                self.run_command(command, ui.ctx(), requests);
                ui.close();
            }
            if ui
                .button(tr("Hard edges by angle"))
                .on_hover_text(tr(
                    "In the selected objects, mark every edge whose polygons meet more \
                     sharply than an angle (30° to start with) hard, and the rest soft: \
                     boxy corners crisp, curves smooth. Change the angle under Last edit.",
                ))
                .clicked()
            {
                self.run_hard_by_angle();
                ui.close();
            }
            ui.separator();
            if let Some(command) = commands_entries(ui, &[CommandId::FlipNormals]) {
                self.run_command(command, ui.ctx(), requests);
                ui.close();
            }
            if ui
                .button(tr("Fix normals"))
                .on_hover_text(tr(
                    "Turn every polygon of the selected objects that faces the wrong \
                     way round: inside out compared with its neighbours, or facing \
                     into a closed shape. Select → Select flipped shows them first.",
                ))
                .clicked()
            {
                self.run_fix_facing();
                ui.close();
            }
        })
        .response
        .on_hover_text(tr(
            "How the shape is shaded: smoothing, hard and soft edges, normals",
        ));
        crate::dialog::menu(ui, tr(MESH_OBJECT_MENU), |ui| {
            objects::make_editable_entry(ui, &self.document, requests);
            if ui
                .button(tr("Split off"))
                .on_hover_text(tr(
                    "Move the selected polygons into a new object of their own, in the \
                     same place (Polygons mode)",
                ))
                .clicked()
            {
                self.run_split_off();
                ui.close();
            }
            let join = CommandId::Join;
            if ui
                .button(join.name())
                .on_hover_text(join.info().help)
                .clicked()
            {
                self.run_command(join, ui.ctx(), requests);
                ui.close();
            }
            objects::apply_entries(ui, &self.document, requests);
            let mirrored = self.selected_mirror().is_some();
            if ui
                .add_enabled(
                    mirrored,
                    egui::Button::new(tr(crate::palette::APPLY_MIRROR_LABEL)),
                )
                .on_hover_text(tr(
                    "Make the mirrored half part of the shape for real: symmetry goes off and \
                     both halves stay",
                ))
                .on_disabled_hover_text(tr("Select an object with symmetry on"))
                .clicked()
            {
                self.apply_mirror();
                ui.close();
            }
            let layer_targets = crate::layers::layer_targets(&self.document);
            if ui
                .add_enabled(
                    !layer_targets.is_empty(),
                    egui::Button::new(tr(crate::layers::ADD_LAYER_LABEL)),
                )
                .on_hover_text(tr(crate::layers::ADD_LAYER_HELP))
                .clicked()
            {
                self.add_edit_layer(requests);
                ui.close();
            }
        })
        .response
        .on_hover_text(tr(
            "Bake into shape, split off, join, apply transforms and edit layers",
        ));
    }
}
