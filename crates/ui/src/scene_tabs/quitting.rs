//! Quitting with unsaved changes in several tabs (decision 0266): a window
//! lists every tab with changes, then either asks about each in turn (the
//! usual "Save changes?" question, in its own tab), closes without saving
//! any, or cancels.

use super::{Quitting, SceneTabs};
use crate::text::tr;
use egui::{Key, Modifiers};

/// The quitting window's title.
pub(crate) const QUIT_TITLE: &str = crate::text!("Save changes before closing?");
/// Its button that asks about each tab in turn.
pub(crate) const GO_THROUGH: &str = crate::text!("Go through them");
/// Its button that closes without saving any.
pub(crate) const CLOSE_ALL: &str = crate::text!("Close without saving");

/// What was chosen in the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Answer {
    /// Ask about each tab in turn.
    GoThrough,
    /// Close, throwing every unsaved change away.
    CloseAll,
    /// Don't close.
    Cancel,
}

impl SceneTabs {
    /// The window listing the tabs with unsaved changes, while open.
    /// Enter goes through them; Esc cancels. Throwing every change away
    /// has no key, on purpose.
    pub(super) fn quitting_dialog(&self, ctx: &egui::Context) -> Option<Answer> {
        if self.quitting != Some(Quitting::Listing) {
            return None;
        }
        let names = self.unsaved_names();
        let mut answer = None;
        crate::dialog::Dialog::new(tr(QUIT_TITLE), 440.0)
            .id("quit_with_tabs")
            .modal()
            .show(ctx, |d| {
                d.header(false, |_| {});
                d.body(|ui| {
                    ui.label(crate::trf!(
                        "{} projects have changes that will be lost if you don't save them:",
                        names.len()
                    ));
                    for name in &names {
                        ui.label(crate::trf!("• {}", name));
                    }
                    crate::dialog::help(
                        ui,
                        tr("Enter: go through them, asking about each · Esc: cancel"),
                    );
                });
                d.footer(None, |ui| {
                    if crate::dialog::main_button(ui, tr(GO_THROUGH))
                        .on_hover_text(tr(
                            "Ask about each project in turn, in its own tab: save it, don't \
                             save it, or cancel",
                        ))
                        .clicked()
                    {
                        answer = Some(Answer::GoThrough);
                    }
                    if crate::dialog::plain_button(ui, tr("Cancel")).clicked() {
                        answer = Some(Answer::Cancel);
                    }
                    if crate::dialog::plain_button(ui, tr(CLOSE_ALL))
                        .on_hover_text(tr("Close the app and lose the changes in every project"))
                        .clicked()
                    {
                        answer = Some(Answer::CloseAll);
                    }
                });
            });
        let (enter, escape) = ctx.input_mut(|i| {
            (
                i.consume_key(Modifiers::NONE, Key::Enter),
                i.consume_key(Modifiers::NONE, Key::Escape),
            )
        });
        if enter {
            answer = Some(Answer::GoThrough);
        } else if escape {
            answer = Some(Answer::Cancel);
        }
        answer
    }
}
