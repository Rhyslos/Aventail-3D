//! The workbench's part in the quad menu ([decision 0094](../../../../docs/decisions/0094-quad-menu.md)):
//! opening it on a right-click in the viewport (when switched on in
//! Settings), filling its four lists, and carrying out what was picked.

use super::Workbench;
use super::showing::{HIDE_KEYS, Hiding, ISOLATE_KEYS, SHOW_ALL_KEYS};
use crate::commands::CommandId;
use crate::objects::{DocRequest, REDO_SHORTCUT, UNDO_SHORTCUT};
use crate::palette::Mode;
use crate::quad_menu::{QuadAction, QuadItem, Quadrant};
use crate::text::tr;
use crate::tool_menu::Ready;

impl Workbench {
    /// Whether something is under way that a right-click belongs to (a
    /// drag it cancels, a click being waited for, a window): then it never
    /// opens the quad menu.
    pub(super) fn busy_for_quad(&self) -> bool {
        self.gizmo_session.is_some()
            || self.coordinates_session.is_some()
            || self.modelling_session.is_some()
            || self.loop_cut_session.is_some()
            || self.slit_session.is_some()
            || self.pair_session.is_some()
            || self.target_pick.is_some()
            || self.tool_menu.is_open()
            || self.window_open()
    }

    /// Notes, as a frame starts, whether the right button went down while
    /// something was under way: letting it go then (a right-click that
    /// cancels a drag) doesn't open the quad menu.
    pub(super) fn note_right_press(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.pointer.button_pressed(egui::PointerButton::Secondary)) {
            self.right_press_busy = self.busy_for_quad();
        }
    }

    /// Opens the quad menu where the viewport was right-clicked, if it is
    /// switched on and nothing else wanted the click.
    pub(super) fn open_quad_menu(&mut self) {
        let Some(at) = self.viewport.take_right_click() else {
            return;
        };
        if self.tool_menu.settings.quad_menu && !self.right_press_busy && !self.busy_for_quad() {
            self.quad_menu.open(at);
        }
    }

    /// Whether the quad menu is open.
    #[cfg(test)]
    pub(crate) fn is_quad_menu_open(&self) -> bool {
        self.quad_menu.is_open()
    }

    /// The four lists: Show (top left), Modes (top right), Edit (bottom
    /// left) and the current mode's tools (bottom right).
    pub(crate) fn quadrants(&self, ctx: &egui::Context) -> [Quadrant; 4] {
        let keys = |k: &egui::KeyboardShortcut| Some(ctx.format_shortcut(k));
        let nothing = self.document.selection().is_empty();
        let needs_selection = |mut item: QuadItem| {
            if nothing {
                item.unavailable = Some(tr("Nothing is selected").into());
            }
            item
        };
        let frame = CommandId::FrameSelected;
        let show = vec![
            QuadItem {
                shortcut: frame.info().shortcut.map(|k| ctx.format_shortcut(&k)),
                ..QuadItem::new(frame.name(), QuadAction::Command(frame))
            },
            needs_selection(QuadItem {
                shortcut: keys(&HIDE_KEYS),
                ..QuadItem::new(tr("Hide selected"), QuadAction::HideSelection)
            }),
            needs_selection(QuadItem {
                shortcut: keys(&ISOLATE_KEYS),
                ..QuadItem::new(tr("Isolate"), QuadAction::HideOthers)
            }),
            QuadItem {
                shortcut: keys(&SHOW_ALL_KEYS),
                ..QuadItem::new(tr("Show all"), QuadAction::ShowAll)
            },
        ];
        let modes = [
            Mode::Model,
            Mode::Object,
            Mode::Points,
            Mode::Edges,
            Mode::Polygons,
        ]
        .into_iter()
        .map(|mode| QuadItem {
            shortcut: mode.key().map(|k| k.name().to_string()),
            current: self.mode == mode,
            ..QuadItem::new(mode.name(), QuadAction::Mode(mode))
        })
        .collect();
        let doc = &self.document;
        let mut edit = vec![
            QuadItem {
                shortcut: keys(&UNDO_SHORTCUT),
                unavailable: doc
                    .undo_label()
                    .is_none()
                    .then(|| tr("Nothing to undo").into()),
                ..QuadItem::new(tr("Undo"), QuadAction::Undo)
            },
            QuadItem {
                shortcut: keys(&REDO_SHORTCUT),
                unavailable: doc
                    .redo_label()
                    .is_none()
                    .then(|| tr("Nothing to redo").into()),
                ..QuadItem::new(tr("Redo"), QuadAction::Redo)
            },
            QuadItem {
                unavailable: crate::layers::layer_targets(doc)
                    .is_empty()
                    .then(|| tr("Select an object with a shape first").into()),
                ..QuadItem::new(tr(crate::layers::ADD_LAYER_LABEL), QuadAction::AddEditLayer)
            },
        ];
        if matches!(self.mode, Mode::Model | Mode::Object) {
            edit.push(needs_selection(QuadItem::new(
                tr(super::aligning::SNAP_TO_LABEL),
                QuadAction::SnapTo,
            )));
            edit.push(needs_selection(QuadItem::new(
                tr(super::aligning::ALIGN_LABEL),
                QuadAction::Align,
            )));
        }
        // A group's commands are listed in its place (decision 0109).
        let tools = self
            .tool_menu_entries()
            .into_iter()
            .flat_map(|entry| {
                let members = entry.slot.command.members();
                if members.is_empty() {
                    vec![entry]
                } else {
                    members
                        .iter()
                        .map(|command| crate::tool_menu::Entry {
                            slot: crate::commands::Slot {
                                command: *command,
                                ..entry.slot
                            },
                            ready: self.readiness(*command),
                        })
                        .collect()
                }
            })
            .map(|entry| {
                let command = entry.slot.command;
                QuadItem {
                    shortcut: command.info().shortcut.map(|k| ctx.format_shortcut(&k)),
                    unavailable: match entry.ready {
                        Ready::Yes => None,
                        Ready::NotNow(why) => Some(why),
                        Ready::Later(when) => Some(when.to_string()),
                    },
                    ..QuadItem::new(command.name(), QuadAction::Command(command))
                }
            })
            .collect();
        [
            Quadrant {
                title: tr("Show"),
                items: show,
            },
            Quadrant {
                title: tr("Modes"),
                items: modes,
            },
            Quadrant {
                title: tr("Edit"),
                items: edit,
            },
            Quadrant {
                title: tr("Tools"),
                items: tools,
            },
        ]
    }

    /// Draws the quad menu, if open, and carries out what is picked.
    pub(super) fn quad_menu_show(&mut self, ctx: &egui::Context, requests: &mut Vec<DocRequest>) {
        if !self.quad_menu.is_open() {
            return;
        }
        let quadrants = self.quadrants(ctx);
        if let Some(action) = self.quad_menu.show(ctx, &quadrants) {
            self.quad_action(action, ctx, requests);
        }
    }

    /// Carries out a quad menu pick.
    pub(crate) fn quad_action(
        &mut self,
        action: QuadAction,
        ctx: &egui::Context,
        requests: &mut Vec<DocRequest>,
    ) {
        match action {
            QuadAction::Command(command) => self.run_command(command, ctx, requests),
            QuadAction::Mode(mode) if self.tool.has_settings() => self.press_mode_key(mode),
            QuadAction::Mode(mode) => self.mode = mode,
            QuadAction::Undo => requests.push(DocRequest::Undo),
            QuadAction::Redo => requests.push(DocRequest::Redo),
            QuadAction::HideSelection => self.hide(Hiding::Selection),
            QuadAction::HideOthers => self.hide(Hiding::Others),
            QuadAction::ShowAll => self.show_everything(),
            QuadAction::AddEditLayer => self.add_edit_layer(requests),
            QuadAction::SnapTo => {
                self.start_target_pick(super::aligning::TargetUse::SnapTo);
            }
            QuadAction::Align => self.start_target_pick(super::aligning::TargetUse::Align),
        }
    }
}
