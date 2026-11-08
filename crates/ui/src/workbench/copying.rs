//! Copy and paste objects, as copies or instances
//! ([decision 0055](../../../../docs/decisions/0055-basics.md)).
//!
//! The keys, chosen by the project lead: **Ctrl + C** copies the selected
//! objects and **Ctrl + V** pastes copies of them; **Alt + C** and
//! **Alt + V** do the same with instances. Duplicate and Instance in the
//! tool menu (`E`) and the Edit menu do both steps at once.
//!
//! In the same project, what is copied is which objects: pasting copies
//! them as they are then, each placed just after its original. Copying
//! also keeps a copy of the objects as they are, with everything they
//! need (`scene::Clipping`), shared by every tab: pasting in another tab
//! (or after a New or Open in this one) pastes that, fitted into the
//! project there, as one undo step
//! ([decision 0268](../../../../docs/decisions/0268-paste-between-tabs.md)).
//! The system clipboard gets a short note, as the window only hears about
//! Ctrl + V when it holds some text.

use super::Workbench;
use crate::components::kind_for;
use crate::objects::DocRequest;
use crate::text::tr;
use egui::{Event, Key, KeyboardShortcut, Modifiers, Ui};
use scene::{Clipping, Command, ObjectId};
use std::sync::Arc;

/// Objects copied, to paste in any tab (decision 0268).
#[derive(Debug, Clone)]
pub(crate) struct CopiedObjects {
    /// The project they were copied in (`Workbench::project`).
    pub(crate) from: u64,
    /// That project's name when they were copied, for messages.
    pub(crate) from_name: String,
    /// Their own copy of the objects and everything they need: it stays
    /// when that project changes, sleeps or closes.
    pub(crate) clipping: Arc<Clipping>,
}

/// What the system clipboard holds after copying objects. Pasting it in
/// the viewport pastes the objects; anything else copied since is
/// ignored there.
pub(crate) const CLIPBOARD_NOTE: &str =
    crate::text!("[3D objects copied: paste them in the viewport]");

/// Copy the selected objects, to paste as instances.
pub(crate) const COPY_INSTANCES_KEYS: KeyboardShortcut =
    KeyboardShortcut::new(Modifiers::ALT, Key::C);
/// Paste instances of the copied objects.
pub(crate) const PASTE_INSTANCES_KEYS: KeyboardShortcut =
    KeyboardShortcut::new(Modifiers::ALT, Key::V);
/// Copy (the system's own key).
pub(crate) const COPY_KEYS: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::C);
/// Paste (the system's own key).
pub(crate) const PASTE_KEYS: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::V);

impl Workbench {
    /// Ctrl + C, Ctrl + V, Alt + C and Alt + V. Not while typing (text
    /// fields copy and paste text).
    pub(super) fn copying_keys(&mut self, ui: &Ui) {
        if ui.ctx().memory(|m| m.focused().is_some()) || self.renaming() {
            return;
        }
        let (copy, paste, paste_instances) = ui.input_mut(|i| {
            let mut copy = i.consume_shortcut(&COPY_INSTANCES_KEYS);
            let paste_instances = i.consume_shortcut(&PASTE_INSTANCES_KEYS);
            let mut paste = false;
            // The window turns Ctrl + C and Ctrl + V into Copy and Paste;
            // tests and some systems send the keys themselves.
            copy |= i.consume_shortcut(&COPY_KEYS);
            paste |= i.consume_shortcut(&PASTE_KEYS);
            i.events.retain(|event| match event {
                Event::Copy => {
                    copy = true;
                    false
                }
                Event::Paste(text) if text == tr(CLIPBOARD_NOTE) => {
                    paste = true;
                    false
                }
                _ => true,
            });
            (copy, paste, paste_instances)
        });
        if copy {
            self.copy_objects(ui.ctx());
        }
        if paste || paste_instances {
            self.paste_objects(paste_instances);
        }
    }

    /// The Edit menu's Copy, Paste, Paste instances, Duplicate and
    /// Instance.
    pub(super) fn copy_entries(&mut self, ui: &mut Ui, out: &mut Vec<DocRequest>) {
        let objects = kind_for(self.mode).is_none();
        let anything = objects && !self.document.selection().is_empty();
        let copied = objects && (!self.copied.is_empty() || self.copied_objects.is_some());
        let keys = |ui: &Ui, k: &KeyboardShortcut| ui.ctx().format_shortcut(k);
        let entries: [(&str, KeyboardShortcut, bool, &str); 3] = [
            (
                tr("Copy objects"),
                COPY_KEYS,
                anything,
                tr(
                    "Copy the selected objects, to paste as copies (Ctrl + V) or instances (Alt + V)",
                ),
            ),
            (
                tr("Paste copies"),
                PASTE_KEYS,
                copied,
                tr("Paste copies of the copied objects, each just after its original"),
            ),
            (
                tr("Paste instances"),
                PASTE_INSTANCES_KEYS,
                copied,
                tr(
                    "Paste instances of the copied objects: they show the original's shape and \
                 follow its changes",
                ),
            ),
        ];
        for (index, (name, k, enabled, help)) in entries.into_iter().enumerate() {
            let text = keys(ui, &k);
            if ui
                .add_enabled(enabled, egui::Button::new(name).shortcut_text(text))
                .on_hover_text(help)
                .on_disabled_hover_text(tr("Works on objects in the Model mode: select some first"))
                .clicked()
            {
                match index {
                    0 => self.copy_objects(ui.ctx()),
                    1 => self.paste_objects(false),
                    _ => self.paste_objects(true),
                }
                ui.close();
            }
        }
        for command in [
            crate::commands::CommandId::Duplicate,
            crate::commands::CommandId::Instance,
        ] {
            let info = command.info();
            if ui
                .add_enabled(anything, egui::Button::new(info.name))
                .on_hover_text(info.help)
                .on_disabled_hover_text(tr("Works on objects in the Model mode: select some first"))
                .clicked()
            {
                self.run_command(command, ui.ctx(), out);
                ui.close();
            }
        }
    }

    /// Remembers the selected objects for pasting.
    pub(crate) fn copy_objects(&mut self, ctx: &egui::Context) {
        if kind_for(self.mode).is_some() {
            self.note = Some(
                tr(
                    "Copy works on whole objects: go back to the Model mode (the top of the left \
                 palette) first.",
                )
                .into(),
            );
            return;
        }
        let selection = self.document.selection();
        if selection.is_empty() {
            self.note = Some(tr("Copy: select the objects to copy first.").into());
            return;
        }
        let n = selection.len();
        self.copied_objects =
            Clipping::capture(self.document.scene(), &selection).map(|clipping| CopiedObjects {
                from: self.project,
                from_name: self.project_name(),
                clipping: Arc::new(clipping),
            });
        self.copied = selection;
        ctx.copy_text(tr(CLIPBOARD_NOTE).to_string());
        self.note = Some(crate::trf!(
            "Copied {} object{}: Ctrl + V pastes copies, Alt + V instances.",
            n,
            if n == 1 { "" } else { "s" }
        ));
    }

    /// Pastes copies (or instances) of the copied objects that are still
    /// there.
    pub(crate) fn paste_objects(&mut self, instances: bool) {
        if kind_for(self.mode).is_some() {
            self.note = Some(
                tr(
                    "Paste works on whole objects: go back to the Model mode (the top of the left \
                 palette) first.",
                )
                .into(),
            );
            return;
        }
        if let Some(other) = self
            .copied_objects
            .clone()
            .filter(|c| c.from != self.project)
        {
            self.paste_from_another_project(&other, instances);
            return;
        }
        let scene = self.document.scene();
        let ids: Vec<ObjectId> = self
            .copied
            .iter()
            .copied()
            .filter(|id| scene.contains(*id))
            .collect();
        if ids.is_empty() {
            self.note = Some(if self.copied.is_empty() {
                tr("Paste: copy some objects first (Ctrl + C, or Alt + C).").into()
            } else {
                tr("Paste: the copied objects have been deleted.").into()
            });
            return;
        }
        // Run here rather than queued, so the status bar keeps saying what
        // happened.
        if let Err(why) = self.document.execute(Command::Duplicate { ids, instances }) {
            self.note = Some(crate::trf!("Couldn't paste: {}.", why));
            return;
        }
        self.note = Some(if instances {
            tr("Pasted instances: they follow their originals' shape.").into()
        } else {
            tr("Pasted copies, on top of their originals: move them away with W.").into()
        });
    }

    /// Whether a graph fill is being made or waits to be.
    #[cfg(test)]
    pub(crate) fn graph_fills_busy(&self) -> bool {
        self.graph_fills.busy()
    }

    /// Whether objects copied (in any tab) are waiting to be pasted.
    #[cfg(test)]
    pub(crate) fn has_copied_objects(&self) -> bool {
        self.copied_objects.is_some()
    }

    /// Pastes objects copied in another project (or in the one this tab
    /// had before), with their materials' paint, graphs and own boxes,
    /// as one undo step. Instances can't stay linked to objects in
    /// another project, so `instances` pastes plain copies too, and says
    /// so.
    fn paste_from_another_project(&mut self, copied: &CopiedObjects, instances: bool) {
        let clipping = &copied.clipping;
        let plan = match clipping.plan(self.document.scene()) {
            Ok(plan) => plan,
            Err(why) => {
                self.note = Some(crate::trf!("Couldn't paste: {}.", why));
                return;
            }
        };
        if let Err(why) = self.document.execute(Command::Paste(Arc::clone(clipping))) {
            self.note = Some(crate::trf!("Couldn't paste: {}.", why));
            return;
        }
        self.note = Some(pasted_message(copied, &plan, instances));
    }
}

/// What the status bar says after pasting objects from another project.
fn pasted_message(copied: &CopiedObjects, plan: &scene::PastePlan, instances: bool) -> String {
    let clipping = &copied.clipping;
    let n = clipping.chosen_count();
    let mut words = Vec::new();
    if instances {
        words.push(
            tr(
                "Instances can't stay linked to objects in another project, so these are \
                 plain copies.",
            )
            .to_string(),
        );
    }
    words.push(crate::trf!(
        "Pasted {} object{} from {}.",
        n,
        if n == 1 { "" } else { "s" },
        copied.from_name
    ));
    let needed = clipping.needed_count();
    if needed > 0 {
        words.push(crate::trf!(
            "{} more came along because they need {} (cutters, cages or curves).",
            needed,
            if needed == 1 { "it" } else { "them" }
        ));
    }
    let brought = plan.sets_added() + plan.graphs_added() + plan.own.add.len();
    if brought > 0 {
        words.push(
            tr("Their paint, material graphs and boxes of your own came with them.").to_string(),
        );
    }
    let renamed = plan.renamed_materials();
    if !renamed.is_empty() {
        let list: Vec<String> = renamed
            .iter()
            .map(|(from, to)| crate::trf!("{} is {} here", from, to))
            .collect();
        words.push(crate::trf!(
            "This project already has other materials with their names, so: {}.",
            list.join(", ")
        ));
    }
    words.join(" ")
}
