//! The interface's part in updates ([decision 0169](../../../../docs/decisions/0169-installer-and-updates.md),
//! [0172](../../../../docs/decisions/0172-update-popup-and-splash-check.md)):
//! when the app around the workbench has downloaded a newer build, a
//! window in the middle says so, with **Restart now**, **Later** and
//! **What's new**. Later (or Esc) puts it away; the app installs the
//! update when it closes. The workbench never downloads or installs anything: it
//! shows what it is told ([`Workbench::offer_update`],
//! [`Workbench::set_update_status`]) and returns
//! [`WorkbenchAction::InstallUpdate`].

use super::{Workbench, WorkbenchAction};
use crate::text::tr;
use crate::theme;
use egui::RichText;

/// A newer build, downloaded and ready to install.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateOffer {
    /// Its build number.
    pub build: u32,
    /// The page listing what changed in it.
    pub notes_url: String,
}

/// What the workbench knows about updates.
#[derive(Debug, Clone, Default)]
pub(super) struct Updating {
    /// The build ready to install, if any.
    offer: Option<UpdateOffer>,
    /// Later was clicked for the offer: the bar stays away.
    later: bool,
    /// What the last check found, for Settings.
    pub(super) status: Option<String>,
}

/// The update window's title.
pub(crate) const UPDATE_TITLE: &str = crate::text!("A new version is ready");
/// The accessible name of the window's Restart now button.
pub(crate) const RESTART_NOW: &str = crate::text!("Restart now");
/// The accessible name of the window's Later button.
pub(crate) const LATER: &str = crate::text!("Later");

impl Workbench {
    /// A newer build is downloaded and ready: the bar offers it (again,
    /// unless Later was clicked for this same build).
    pub fn offer_update(&mut self, offer: UpdateOffer) {
        if self.updating.offer.as_ref() != Some(&offer) {
            self.updating.later = false;
        }
        self.updating.offer = Some(offer);
    }

    /// What the last check for updates found, in a few words (shown in
    /// Settings → Interface → Updates).
    pub fn set_update_status(&mut self, status: String) {
        self.updating.status = Some(status);
    }

    /// Whether the person wants the app to look for new versions
    /// (Settings → Interface → Updates; on as standard).
    pub fn checks_for_updates(&self) -> bool {
        self.viewport.options().check_for_updates
    }

    /// Whether the update window is shown.
    pub(super) fn update_window_shown(&self) -> bool {
        self.updating.offer.is_some() && !self.updating.later
    }

    /// The window offering the update.
    pub(super) fn update_window(
        &mut self,
        ctx: &egui::Context,
        actions: &mut Vec<WorkbenchAction>,
    ) {
        if !self.update_window_shown() {
            return;
        }
        let Some(offer) = self.updating.offer.clone() else {
            return;
        };
        let mut answer = None;
        crate::dialog::Dialog::new(tr(UPDATE_TITLE), 420.0)
            .id("update_offer")
            .modal()
            .show(ctx, |d| {
                d.header(false, |_| {});
                d.body(|ui| {
                    ui.label(
                        RichText::new(crate::trf!(
                            "Build {} is downloaded and ready to install.",
                            offer.build
                        ))
                        .color(theme::TEXT_STRONG),
                    );
                    ui.add_space(4.0);
                    ui.label(tr(
                        "Restart now to install it: the app closes (asking about unsaved work \
                         first), updates and opens again in a few seconds. Or keep working: it \
                         installs when you close the app.",
                    ));
                    ui.add_space(4.0);
                    ui.hyperlink_to(tr("What's new in this build"), &offer.notes_url);
                    crate::dialog::help(ui, tr("Esc: later"));
                });
                d.footer(None, |ui| {
                    if crate::dialog::main_button(ui, tr(RESTART_NOW)).clicked() {
                        answer = Some(true);
                    }
                    if crate::dialog::plain_button(ui, tr(LATER)).clicked() {
                        answer = Some(false);
                    }
                });
            });
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
            answer = Some(false);
        }
        match answer {
            Some(true) => actions.push(WorkbenchAction::InstallUpdate { now: true }),
            Some(false) => {
                self.updating.later = true;
                self.note = Some(crate::trf!(
                    "The new version (build {}) installs when you close the app.",
                    offer.build
                ));
                actions.push(WorkbenchAction::InstallUpdate { now: false });
            }
            None => {}
        }
    }
}
