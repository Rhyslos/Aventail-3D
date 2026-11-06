//! UV → UV sets ([decision 0241](../../../../docs/decisions/0241-uv-sets.md)):
//! listing the selected objects' UV sets, choosing the one every UV tool
//! works on, and adding, copying, renaming and deleting them. Each change
//! is recorded as `Operation::UvSet` on every selected object, one undo
//! step.

use super::Workbench;
use crate::text::tr;
use egui::Ui;
use mesh::uv_sets::{self, SetChange};
use scene::{ObjectId, Operation};

/// The submenu's name.
pub(crate) const UV_SETS_LABEL: &str = crate::text!("UV sets");

impl Workbench {
    /// The selected objects' sets (the first's, which the menu lists).
    fn uv_set_names(&self) -> Vec<String> {
        self.own_shapes()
            .first()
            .map(|(_, mesh)| uv_sets::names(mesh))
            .unwrap_or_default()
    }

    /// The submenu's entries.
    pub(super) fn uv_sets_entries(&mut self, ui: &mut Ui) {
        let names = self.uv_set_names();
        if names.is_empty() {
            ui.label(tr("Select an object with UVs to see its UV sets."));
            return;
        }
        ui.label(
            egui::RichText::new(tr("The set the UV tools work on (and paint uses):")).small(),
        );
        for (k, name) in names.iter().enumerate() {
            if ui
                .radio(k == 0, name.as_str())
                .on_hover_text(tr("Work on this set: the UV Editor shows it"))
                .clicked()
                && k != 0
            {
                self.run_uv_set(SetChange::MakeActive(name.clone()));
                ui.close();
            }
        }
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(tr("Name"));
            ui.add(egui::TextEdit::singleline(&mut self.uv_set_name).desired_width(120.0));
        });
        let name = self.uv_set_name.trim().to_owned();
        let usable = uv_sets::is_name(&name) && !names.contains(&name);
        let active = names[0].clone();
        if ui
            .add_enabled(usable, egui::Button::new(tr("Add a set with this name")))
            .on_hover_text(tr(
                "A new UV set, starting as a copy of the one worked on: for a second layout (a \
                 lightmap, a detail texture)",
            ))
            .clicked()
        {
            self.run_uv_set(SetChange::Add(name.clone()));
            ui.close();
        }
        if ui
            .add_enabled(usable, egui::Button::new(tr("Rename the set worked on")))
            .clicked()
        {
            self.run_uv_set(SetChange::Rename {
                from: active.clone(),
                to: name.clone(),
            });
            ui.close();
        }
        if ui
            .add_enabled(names.len() > 1, egui::Button::new(tr("Delete the set worked on")))
            .on_hover_text(tr("The next set takes its place"))
            .clicked()
        {
            self.run_uv_set(SetChange::Delete(active));
            ui.close();
        }
        if names.len() >= uv_sets::MAX_SETS {
            ui.label(egui::RichText::new(tr("That's as many sets as a shape can have.")).small());
        }
    }

    /// Makes `change` on every selected object with UVs, as one undo step.
    pub(crate) fn run_uv_set(&mut self, change: SetChange) {
        let ops: Vec<(ObjectId, Operation)> = self
            .own_shapes()
            .into_iter()
            .filter(|(_, m)| mesh::uvs::has_uvs(m))
            .map(|(id, _)| {
                (
                    id,
                    Operation::UvSet {
                        change: change.clone(),
                    },
                )
            })
            .collect();
        if ops.is_empty() || !change.is_real() {
            self.note = Some(crate::trf!(
                "{}: select objects with UVs, and give a name of letters, digits, spaces, - or _.",
                tr(UV_SETS_LABEL)
            ));
            return;
        }
        let label = match &change {
            SetChange::Add(n) => crate::trf!("Add the UV set {}", n),
            SetChange::Copy { to, .. } => crate::trf!("Copy a UV set as {}", to),
            SetChange::Rename { to, .. } => crate::trf!("Rename the UV set {}", to),
            SetChange::Delete(n) => crate::trf!("Delete the UV set {}", n),
            SetChange::MakeActive(n) => crate::trf!("Work on the UV set {}", n),
        };
        if self.record_each(&label, ops) {
            self.note = Some(crate::trf!("{}. Ctrl + Z takes it back.", label));
        }
    }
}
