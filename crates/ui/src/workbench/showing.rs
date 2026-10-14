//! Hide, isolate and show everything
//! ([decision 0055](../../../../docs/decisions/0055-basics.md)).
//!
//! The keys, chosen by the project lead: **Shift + H** hides the
//! selection, **Ctrl + H** isolates it (hides everything else), **Alt + H**
//! shows everything again.
//!
//! - In the Model mode they work on objects, through the objects' Visible
//!   switch: saved with the project and undone like any change.
//! - In the Points, Edges and Polygons modes they work on the edited
//!   objects' polygons (the selected polygons, or those touching the
//!   selected points or edges). Hidden polygons are a view setting, like
//!   the camera: not drawn, not picked, not saved and not undone. Hiding
//!   unselects them, so no tool changes what can't be seen.

use super::Workbench;
use crate::components::kind_for;
use crate::objects::DocRequest;
use egui::{Key, KeyboardShortcut, Modifiers, Ui};
use scene::{Command, ElementKind, FaceId, ObjectId};
use std::collections::BTreeSet;

/// Hide the selection.
pub(crate) const HIDE_KEYS: KeyboardShortcut = KeyboardShortcut::new(Modifiers::SHIFT, Key::H);
/// Isolate the selection: hide everything else.
pub(crate) const ISOLATE_KEYS: KeyboardShortcut =
    KeyboardShortcut::new(Modifiers::COMMAND, Key::H);
/// Show everything again.
pub(crate) const SHOW_ALL_KEYS: KeyboardShortcut = KeyboardShortcut::new(Modifiers::ALT, Key::H);

/// What to hide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Hiding {
    /// The selection.
    Selection,
    /// Everything but the selection.
    Others,
}

impl Workbench {
    /// Shift + H, Ctrl + H and Alt + H. Not while typing. They come before
    /// the viewport's H (frame all), which would otherwise take Shift + H
    /// and Alt + H too.
    pub(super) fn showing_keys(&mut self, ui: &Ui, out: &mut Vec<DocRequest>) {
        if ui.ctx().memory(|m| m.focused().is_some()) || self.objects.is_renaming() {
            return;
        }
        // Ctrl + H first, as the others would match it logically.
        let (isolate, hide, show) = ui.input_mut(|i| {
            let isolate = i.consume_shortcut(&ISOLATE_KEYS);
            let show = i.consume_shortcut(&SHOW_ALL_KEYS);
            let hide = i.consume_shortcut(&HIDE_KEYS);
            (isolate, hide, show)
        });
        if isolate {
            self.hide(Hiding::Others, out);
        }
        if hide {
            self.hide(Hiding::Selection, out);
        }
        if show {
            self.show_everything(out);
        }
    }

    /// Hides the selection, or everything else.
    pub(crate) fn hide(&mut self, what: Hiding, out: &mut Vec<DocRequest>) {
        match kind_for(self.mode) {
            None => self.hide_objects(what, out),
            Some(kind) => self.hide_polygons(kind, what),
        }
    }

    fn hide_objects(&mut self, what: Hiding, out: &mut Vec<DocRequest>) {
        let selection = self.document.selection();
        if selection.is_empty() {
            self.note = Some(match what {
                Hiding::Selection => "Hide: select the objects to hide first.".into(),
                Hiding::Others => "Isolate: select the objects to keep first.".into(),
            });
            return;
        }
        let scene = self.document.scene();
        let ids: Vec<ObjectId> = match what {
            Hiding::Selection => selection.clone(),
            Hiding::Others => scene
                .walk()
                .into_iter()
                .map(|(id, _)| id)
                .filter(|id| {
                    // Not a selected object, not inside one, and not a
                    // group holding one (hiding that would hide it too).
                    !selection.iter().any(|s| {
                        s == id || scene.is_inside(*id, *s) || scene.is_inside(*s, *id)
                    })
                })
                .filter(|id| scene.object(*id).is_some_and(|o| o.visible))
                .collect(),
        };
        if ids.is_empty() {
            self.note = Some("Everything else is already hidden.".into());
            return;
        }
        let n = ids.len();
        out.push(DocRequest::Run(Command::SetVisible {
            ids,
            visible: false,
        }));
        if what == Hiding::Selection {
            self.document.clear_selection();
        }
        let things = if n == 1 { "object" } else { "objects" };
        self.note = Some(format!(
            "{} {n} {things}. Alt + H shows everything again.",
            if what == Hiding::Selection {
                "Hid"
            } else {
                "Isolated: hid"
            }
        ));
    }

    fn hide_polygons(&mut self, kind: ElementKind, what: Hiding) {
        let mut hidden_now = 0;
        let mut selections = Vec::new();
        for (id, selected, mesh) in self.selected_parts() {
            let shown = self.hidden.shown(id, &mesh);
            // The polygons the selection names, among those shown.
            let chosen: BTreeSet<FaceId> = match kind {
                ElementKind::Polygons => selected.polygons().collect(),
                ElementKind::Edges => selected
                    .edges()
                    .flat_map(|e| shown.topology().edge_faces(e).collect::<Vec<_>>())
                    .collect(),
                ElementKind::Points => selected
                    .points()
                    .flat_map(|v| shown.topology().vertex_faces(v).to_vec())
                    .collect(),
            };
            let chosen: BTreeSet<FaceId> = chosen
                .into_iter()
                .filter(|f| shown.face(*f).is_some())
                .collect();
            if chosen.is_empty() {
                continue;
            }
            let hide: Vec<FaceId> = match what {
                Hiding::Selection => chosen.into_iter().collect(),
                Hiding::Others => shown
                    .faces()
                    .map(|(f, _)| f)
                    .filter(|f| !chosen.contains(f))
                    .collect(),
            };
            if hide.is_empty() {
                continue;
            }
            hidden_now += hide.len();
            self.hidden.faces.entry(id).or_default().extend(hide);
            // What can't be seen can't stay selected.
            let after = self.hidden.shown(id, &mesh);
            let valid = selected.valid_in(&after, after.topology());
            selections.push((id, valid));
        }
        if hidden_now == 0 {
            let what_for = match what {
                Hiding::Selection => "Hide: select the polygons (or their points or edges) to hide first.",
                Hiding::Others => "Isolate: select the polygons (or their points or edges) to keep first.",
            };
            self.note = Some(what_for.into());
            return;
        }
        self.hidden.revision += 1;
        let label = if what == Hiding::Selection { "Hide" } else { "Isolate" };
        self.document.select_elements(selections, label, None);
        let things = if hidden_now == 1 { "polygon" } else { "polygons" };
        self.note = Some(format!(
            "{} {hidden_now} {things}. Alt + H shows everything again.",
            if what == Hiding::Selection {
                "Hid"
            } else {
                "Isolated: hid"
            }
        ));
    }

    /// Alt + H: shows every hidden polygon, and in the Model mode every
    /// hidden object too.
    pub(crate) fn show_everything(&mut self, out: &mut Vec<DocRequest>) {
        let polygons: usize = self.hidden.faces.values().map(BTreeSet::len).sum();
        if polygons > 0 {
            self.hidden.faces.clear();
            self.hidden.revision += 1;
        }
        let mut objects = 0;
        if kind_for(self.mode).is_none() {
            let scene = self.document.scene();
            let ids: Vec<ObjectId> = scene
                .walk()
                .into_iter()
                .map(|(id, _)| id)
                .filter(|id| scene.object(*id).is_some_and(|o| !o.visible))
                .collect();
            objects = ids.len();
            if !ids.is_empty() {
                out.push(DocRequest::Run(Command::SetVisible { ids, visible: true }));
            }
        }
        self.note = Some(match (objects, polygons) {
            (0, 0) => "Nothing is hidden.".into(),
            (0, p) => format!("Showing {p} hidden polygon{} again.", plural(p)),
            (o, 0) => format!("Showing {o} hidden object{} again.", plural(o)),
            (o, p) => format!(
                "Showing {o} hidden object{} and {p} polygon{} again.",
                plural(o),
                plural(p)
            ),
        });
    }

    /// How many polygons are hidden, for the status bar.
    pub(super) fn hidden_text(&self) -> String {
        let n: usize = self.hidden.faces.values().map(BTreeSet::len).sum();
        if n == 0 {
            String::new()
        } else {
            format!(" {n} polygon{} hidden (Alt + H shows them).", plural(n))
        }
    }
}

fn plural(n: usize) -> &'static str {
    if n == 1 { "" } else { "s" }
}
