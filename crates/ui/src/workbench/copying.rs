//! Copy and paste objects, as copies or instances
//! ([decision 0055](../../../../docs/decisions/0055-basics.md)).
//!
//! The keys, chosen by the project lead: **Ctrl + C** copies the selected
//! objects and **Ctrl + V** pastes copies of them; **Alt + C** and
//! **Alt + V** do the same with instances. Duplicate and Instance in the
//! tool menu (`E`) and the Edit menu do both steps at once.
//!
//! What is copied is which objects: pasting copies them as they are then,
//! each placed just after its original. The system clipboard gets a short
//! note, as the window only hears about Ctrl + V when it holds some text.

use super::Workbench;
use crate::components::kind_for;
use crate::objects::DocRequest;
use egui::{Event, Key, KeyboardShortcut, Modifiers, Ui};
use scene::{Command, ObjectId};

/// What the system clipboard holds after copying objects. Pasting it in
/// the viewport pastes the objects; anything else copied since is
/// ignored there.
pub(crate) const CLIPBOARD_NOTE: &str = "[3D objects copied: paste them in the viewport]";

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
        if ui.ctx().memory(|m| m.focused().is_some()) || self.objects.is_renaming() {
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
                Event::Paste(text) if text == CLIPBOARD_NOTE => {
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
        let copied = objects && !self.copied.is_empty();
        let keys = |ui: &Ui, k: &KeyboardShortcut| ui.ctx().format_shortcut(k);
        let entries: [(&str, KeyboardShortcut, bool, &str); 3] = [
            (
                "Copy objects",
                COPY_KEYS,
                anything,
                "Copy the selected objects, to paste as copies (Ctrl + V) or instances (Alt + V)",
            ),
            (
                "Paste copies",
                PASTE_KEYS,
                copied,
                "Paste copies of the copied objects, each just after its original",
            ),
            (
                "Paste instances",
                PASTE_INSTANCES_KEYS,
                copied,
                "Paste instances of the copied objects: they show the original's shape and \
                 follow its changes",
            ),
        ];
        for (index, (name, k, enabled, help)) in entries.into_iter().enumerate() {
            let text = keys(ui, &k);
            if ui
                .add_enabled(enabled, egui::Button::new(name).shortcut_text(text))
                .on_hover_text(help)
                .on_disabled_hover_text("Works on objects in the Model mode: select some first")
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
                .on_disabled_hover_text("Works on objects in the Model mode: select some first")
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
                "Copy works on whole objects: go back to the Model mode (the top of the left \\
                 palette) first."
                    .into(),
            );
            return;
        }
        let selection = self.document.selection();
        if selection.is_empty() {
            self.note = Some("Copy: select the objects to copy first.".into());
            return;
        }
        let n = selection.len();
        self.copied = selection;
        ctx.copy_text(CLIPBOARD_NOTE.to_string());
        self.note = Some(format!(
            "Copied {n} object{}: Ctrl + V pastes copies, Alt + V instances.",
            if n == 1 { "" } else { "s" }
        ));
    }

    /// Pastes copies (or instances) of the copied objects that are still
    /// there.
    pub(crate) fn paste_objects(&mut self, instances: bool) {
        if kind_for(self.mode).is_some() {
            self.note = Some(
                "Paste works on whole objects: go back to the Model mode (the top of the left \\
                 palette) first."
                    .into(),
            );
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
                "Paste: copy some objects first (Ctrl + C, or Alt + C).".into()
            } else {
                "Paste: the copied objects have been deleted.".into()
            });
            return;
        }
        // Run here rather than queued, so the status bar keeps saying what
        // happened.
        if let Err(why) = self.document.execute(Command::Duplicate { ids, instances }) {
            self.note = Some(format!("Couldn't paste: {why}."));
            return;
        }
        self.note = Some(if instances {
            "Pasted instances: they follow their originals' shape.".into()
        } else {
            "Pasted copies, on top of their originals: move them away with W.".into()
        });
    }
}
