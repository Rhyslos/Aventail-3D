//! The interface's part in updates ([decision 0169](../../../../docs/decisions/0169-installer-and-updates.md)):
//! when the app around the workbench has downloaded a newer build, a slim
//! bar under the top palette says so, with **Restart now**, **Later** and
//! **What's new**. Later puts the bar away; the app installs the update
//! when it closes. The workbench never downloads or installs anything: it
//! shows what it is told ([`Workbench::offer_update`],
//! [`Workbench::set_update_status`]) and returns
//! [`WorkbenchAction::InstallUpdate`].

use super::{Workbench, WorkbenchAction};
use crate::text::tr;
use crate::theme;
use egui::{RichText, Ui};

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

/// The accessible name of the bar's Restart now button.
pub(crate) const RESTART_NOW: &str = crate::text!("Restart now");
/// The accessible name of the bar's Later button.
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

    /// Whether the update bar is shown.
    pub(super) fn update_bar_shown(&self) -> bool {
        self.updating.offer.is_some() && !self.updating.later
    }

    /// The bar offering the update, under the top palette.
    pub(super) fn update_bar(&mut self, ui: &mut Ui, actions: &mut Vec<WorkbenchAction>) {
        let Some(offer) = self.updating.offer.clone() else {
            return;
        };
        ui.horizontal_centered(|ui| {
            ui.add_space(8.0);
            ui.label(
                RichText::new(crate::trf!(
                    "A new version is ready (build {}).",
                    offer.build
                ))
                .color(theme::TEXT_STRONG),
            );
            ui.add_space(8.0);
            if crate::dialog::main_button(ui, tr(RESTART_NOW))
                .on_hover_text(tr(
                    "Close the app, install the new version and open it again. Unsaved changes \
                     are asked about first.",
                ))
                .clicked()
            {
                actions.push(WorkbenchAction::InstallUpdate { now: true });
            }
            if ui
                .button(tr(LATER))
                .on_hover_text(tr("Keep working: it installs when you close the app"))
                .clicked()
            {
                self.updating.later = true;
                self.note = Some(crate::trf!(
                    "The new version (build {}) installs when you close the app.",
                    offer.build
                ));
                actions.push(WorkbenchAction::InstallUpdate { now: false });
            }
            ui.add_space(4.0);
            ui.hyperlink_to(tr("What's new"), &offer.notes_url);
        });
    }
}
