//! The UV Editor's Snapshots menu ([decision 0114](../../../../docs/decisions/0114-layout-snapshots.md)):
//! named versions of the UV layout, kept in the project, to put back or
//! compare with.
//!
//! **Take a snapshot** keeps the UVs and seams of the objects shown in the
//! editor. Each snapshot can be renamed, put back (**Restore**: one undo
//! step, recorded on each object like any UV edit), compared with
//! (**Compare**: its layout drawn faintly under the current one), brought
//! up to date (**Update**) or removed.

use super::UvEditor;
use crate::theme;
use egui::{Pos2, Rect, Ui};
use scene::{Command, Document, ObjectId, UvSnapshot};

/// The name of the UV Editor's menu.
pub(crate) const SNAPSHOTS_MENU: &str = "Snapshots";

/// The button that takes a snapshot.
pub(crate) const TAKE_SNAPSHOT: &str = "Take a snapshot";

/// Marks renaming's undo merge keys, so a run of typing in a name is one
/// undo step.
const RENAME_KEY: u64 = 0x534e_4150_4e41_4d45;

/// Marks restores' undo merge keys: a restore of several objects is one
/// undo step.
const RESTORE_KEY: u64 = 0x534e_4150_0000_0000;

/// The objects a new snapshot is taken of: those the editor shows.
fn shown(doc: &Document) -> Vec<ObjectId> {
    super::shown_objects(doc)
}

/// Takes a snapshot of the objects shown, named `name` (or the next free
/// "Snapshot n"). Returns what happened, for the menu to say.
pub(crate) fn take(doc: &mut Document, name: Option<&str>) -> Result<String, String> {
    let mut list = doc.scene().snapshots().to_vec();
    if list.len() >= scene::MAX_SNAPSHOTS {
        return Err(format!(
            "A project keeps at most {} snapshots: remove one first.",
            scene::MAX_SNAPSHOTS
        ));
    }
    let wanted = name.map_or_else(|| format!("Snapshot {}", list.len() + 1), str::to_string);
    let name = scene::snapshot::free_name(&list, &wanted);
    let Some(snapshot) = UvSnapshot::take(doc.scene(), &shown(doc), &name) else {
        return Err("Select objects with UVs first: a snapshot keeps their layout.".into());
    };
    let objects = snapshot.objects.len();
    list.push(snapshot);
    doc.execute_labelled(
        Command::SetSnapshots(list),
        format!("Take the snapshot “{name}”"),
        None,
    )
    .map_err(|why| format!("Couldn't take it: {why}."))?;
    Ok(format!(
        "Took “{name}” of {objects} {}.",
        if objects == 1 { "object" } else { "objects" }
    ))
}

impl UvEditor {
    /// Puts snapshot `index` back on its objects, as one undo step.
    /// Returns whether anything was put back.
    pub(crate) fn restore_snapshot(&mut self, doc: &mut Document, index: usize) -> bool {
        let Some(snapshot) = doc.scene().snapshots().get(index).cloned() else {
            return false;
        };
        let ops = snapshot.restore(doc.scene());
        if ops.is_empty() {
            return false;
        }
        self.sessions += 1;
        let key = RESTORE_KEY | self.sessions;
        let label = format!("Restore the snapshot “{}”", snapshot.name);
        for (id, operation) in ops {
            if doc
                .execute_labelled(Command::Record { id, operation }, label.clone(), Some(key))
                .is_err()
            {
                return false;
            }
        }
        true
    }

    /// The snapshot compared with, if any (by name).
    pub(crate) fn compared(&self) -> Option<&str> {
        self.compare.as_deref()
    }

    /// Draws the compared snapshot's layout of the shown objects faintly,
    /// under the current UVs, and says which it is, top left.
    pub(super) fn paint_compared(&self, painter: &egui::Painter, rect: Rect, doc: &Document) {
        let Some(name) = &self.compare else {
            return;
        };
        let Some(snapshot) = doc.scene().snapshots().iter().find(|s| &s.name == name) else {
            return;
        };
        let mut lines = egui::Mesh::default();
        for id in shown(doc) {
            let Some(layout) = snapshot.layout(id) else {
                continue;
            };
            for (_, uvs) in &layout.faces {
                let corners: Vec<Pos2> = uvs.iter().map(|uv| self.to_screen(rect, *uv)).collect();
                for k in 0..corners.len() {
                    super::add_line(
                        &mut lines,
                        corners[k],
                        corners[(k + 1) % corners.len()],
                        1.0,
                        theme::UV_SNAPSHOT,
                    );
                }
            }
        }
        painter.add(egui::Shape::mesh(lines));
        painter.text(
            rect.left_top() + egui::vec2(10.0, 10.0),
            egui::Align2::LEFT_TOP,
            format!("Comparing with “{name}” (faint)"),
            egui::FontId::proportional(12.0),
            theme::UV_SNAPSHOT_TEXT,
        );
    }
}

/// The UV Editor's Snapshots menu: take one; then each snapshot, oldest
/// first, with its name to change, Restore, Compare, Update and remove.
pub(crate) fn snapshots_menu(ui: &mut Ui, doc: &mut Document, editor: &mut UvEditor) {
    ui.set_max_width(420.0);
    ui.label(egui::RichText::new("Layout snapshots").strong());
    ui.label(egui::RichText::new("Named versions of the UV layout, kept with the project.").weak());
    ui.separator();
    let can_take = !shown(doc).is_empty();
    if ui
        .add_enabled(can_take, egui::Button::new(TAKE_SNAPSHOT))
        .on_hover_text(
            "Keep the UVs and seams of the objects shown in the UV Editor under a name, \
             to go back to or compare with later",
        )
        .on_disabled_hover_text("Select objects with UVs first")
        .clicked()
    {
        editor.note = Some(match take(doc, None) {
            Ok(said) | Err(said) => said,
        });
    }
    if let Some(note) = &editor.note {
        ui.label(egui::RichText::new(note).weak());
    }
    let list = doc.scene().snapshots().to_vec();
    if list.is_empty() {
        ui.label("No snapshots yet.");
        return;
    }
    ui.separator();
    let mut changed: Option<(Vec<UvSnapshot>, &str, Option<u64>)> = None;
    let mut restore = None;
    egui::Grid::new("uv_snapshots")
        .num_columns(5)
        .spacing([6.0, 4.0])
        .show(ui, |ui| {
            for (i, snapshot) in list.iter().enumerate() {
                let mut name = snapshot.name.clone();
                if ui
                    .add_sized([130.0, 20.0], egui::TextEdit::singleline(&mut name))
                    .on_hover_text("The snapshot's name. Each must differ.")
                    .changed()
                {
                    let mut renamed = list.clone();
                    renamed[i].name = name.clone();
                    if scene::snapshot::snapshots_are_valid(&renamed) {
                        if editor.compared() == Some(snapshot.name.as_str()) {
                            editor.compare = Some(name);
                        }
                        changed = Some((renamed, "Rename a snapshot", Some(RENAME_KEY | i as u64)));
                    }
                }
                let objects = snapshot.restore(doc.scene()).len();
                if ui
                    .add_enabled(objects > 0, egui::Button::new("Restore"))
                    .on_hover_text(format!(
                        "Put this layout back: the UVs and seams of {} {}, as one step to undo. \
                         Polygons made since keep theirs.",
                        objects,
                        if objects == 1 { "object" } else { "objects" }
                    ))
                    .on_disabled_hover_text("The objects it was taken of are gone")
                    .clicked()
                {
                    restore = Some(i);
                }
                let comparing = editor.compared() == Some(snapshot.name.as_str());
                if ui
                    .add(egui::Button::selectable(comparing, "Compare"))
                    .on_hover_text(
                        "Draw this layout faintly under the current one, to see what changed. \
                         Click again to stop.",
                    )
                    .clicked()
                {
                    editor.compare = (!comparing).then(|| snapshot.name.clone());
                }
                if ui
                    .button("Update")
                    .on_hover_text("Replace it with the current layout of the objects shown")
                    .clicked()
                {
                    if let Some(fresh) = UvSnapshot::take(doc.scene(), &shown(doc), &snapshot.name)
                    {
                        let mut updated = list.clone();
                        updated[i] = fresh;
                        changed = Some((updated, "Update a snapshot", None));
                    } else {
                        editor.note = Some("Select objects with UVs to update it from.".into());
                    }
                }
                if ui
                    .small_button("×")
                    .on_hover_text("Remove this snapshot (the UVs don't change)")
                    .clicked()
                {
                    let mut fewer = list.clone();
                    fewer.remove(i);
                    if comparing {
                        editor.compare = None;
                    }
                    changed = Some((fewer, "Remove a snapshot", None));
                }
                ui.end_row();
            }
        });
    if let Some((list, label, key)) = changed {
        let _ = doc.execute_labelled(Command::SetSnapshots(list), label, key);
    }
    if let Some(i) = restore {
        let name = list[i].name.clone();
        editor.note = Some(if editor.restore_snapshot(doc, i) {
            format!("Restored “{name}”. Ctrl + Z takes it back.")
        } else {
            format!("Couldn't restore “{name}”.")
        });
    }
}

#[cfg(test)]
mod tests;
