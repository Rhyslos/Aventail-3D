//! The workbench as one tab of several (decision 0266): what a tab keeps
//! for itself (its project, undo history, selection, view, mode, layout
//! and background work) and what all tabs share (the settings, the
//! libraries of brushes, own boxes, recipes and colours, the recent
//! projects). Each tab is a whole [`Workbench`]; only the tab in use is
//! shown, so the shared things are handed from one tab to the next when
//! the tab in use changes ([`Workbench::take_shared_from`]).

use super::{Workbench, WorkbenchAction};
use crate::project::Pending;
use crate::scene_tabs::TabStrip;
use crate::viewport::TabPlacement;
use scene::Document;

impl Workbench {
    /// A new tab's workbench: a new, empty project with this one's
    /// settings, libraries and panel layouts (each layout as it was last
    /// arranged), in the Standard layout and the Model mode.
    pub(crate) fn new_tab_like(&self) -> Workbench {
        let mut tab = Workbench::new(self.info.clone(), Some(self.layout().clone()))
            .with_uv_layout(Some(self.uv_layout().clone()))
            .with_paint_layout(Some(self.paint_layout().clone()))
            .with_material_layout(Some(self.material_layout().clone()));
        tab.take_shared_from(self);
        // The new project's units are the setting for new projects.
        tab.replace_document(tab.new_document());
        tab.tabbed = true;
        tab
    }

    /// Takes the shared things from `other`, the tab in use until now:
    /// every setting and library, so a change made in one tab is in every
    /// tab. The view options come along too, except whether edges are
    /// drawn, which each tab's mode ticks for itself (decision 0220).
    pub(crate) fn take_shared_from(&mut self, other: &Workbench) {
        let show_edges = self.viewport.options().show_edges;
        *self.viewport.options_mut() = other.viewport.options();
        self.viewport.options_mut().show_edges = show_edges;
        self.accent = other.accent;
        self.new_units = other.new_units;
        if self.undo_memory != other.undo_memory {
            self.set_undo_memory(other.undo_memory);
        }
        self.file.recent = other.file.recent.clone();
        self.autosaver.settings = other.autosaver.settings;
        self.autosave_folder = other.autosave_folder.clone();
        self.tool_menu.settings = other.tool_menu.settings;
        self.uv_settings = other.uv_settings;
        self.exporting.remembered = other.exporting.remembered;
        self.saved_brushes = other.saved_brushes.clone();
        self.material.library = other.material.library.clone();
        self.saved_recipes = other.saved_recipes.clone();
        self.used_colours = other.used_colours.clone();
        self.system_keyboard = other.system_keyboard;
        self.updating = other.updating.clone();
        // What was copied pastes in any tab (decision 0268).
        self.copied_objects = other.copied_objects.clone();
        self.material.clipboard = other.material.clipboard.clone();
    }

    /// Gives the project a new number in the run, so what was copied
    /// in the last one pastes as from another project.
    pub(super) fn new_project_number(&mut self) {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        self.project = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.material.project = self.project;
    }

    /// The workbench with a new project number.
    pub(super) fn with_new_project_number(mut self) -> Self {
        self.new_project_number();
        self
    }

    /// Whether the project is empty and untouched: never saved, nothing
    /// made and nothing to undo. New and Open use such a tab rather than
    /// starting another (decision 0266).
    pub fn is_untouched(&self) -> bool {
        self.file.path.is_none()
            && !self.has_unsaved_changes()
            && self.document.scene().children(None).is_empty()
            && self.document.undo_steps() == 0
            && self.document.redo_steps() == 0
    }

    /// What `next` becomes in a tab, if not what it does on its own: New
    /// and Open start a new tab (when that setting is on and this project
    /// isn't empty and untouched), and Quit goes to the tabs, which ask
    /// about every tab with changes.
    pub(super) fn tab_action(&self, next: &Pending) -> Option<WorkbenchAction> {
        if !self.tabbed {
            return None;
        }
        let new_tab = self.viewport.options().open_in_new_tab && !self.is_untouched();
        match next {
            Pending::New if new_tab => Some(WorkbenchAction::NewTab),
            Pending::Open(path) if new_tab => {
                Some(WorkbenchAction::OpenInNewTab { path: path.clone() })
            }
            Pending::Quit => Some(WorkbenchAction::Quit),
            _ => None,
        }
    }

    /// Makes this workbench a tab of [`crate::SceneTabs`], whose project
    /// is called `untitled` until it is first saved ("Untitled 2"; `None`
    /// for plain "Untitled").
    pub(crate) fn become_tab(&mut self, untitled: Option<String>) {
        self.tabbed = true;
        self.file.untitled = untitled;
    }

    /// The tab is to close (as the app is `quitting`, or on its own):
    /// returns `true` if it may close now; with unsaved changes it asks
    /// first ("Save changes to …?") and returns `false`, and a
    /// [`WorkbenchAction::CloseTab`] follows if the person agrees.
    pub(crate) fn ask_to_close_tab(&mut self, quitting: bool) -> bool {
        if self.has_unsaved_changes() {
            self.file.asking = Some(Pending::CloseTab { quitting });
            false
        } else {
            true
        }
    }

    /// Whether closing the tab is still under way: the question is asked,
    /// a save it waits for is going on, or the close is about to be handed
    /// over. `false` once it was cancelled (or the tab has closed).
    pub(crate) fn closing_tab(&self) -> bool {
        matches!(self.file.asking, Some(Pending::CloseTab { .. }))
            || matches!(self.file.after_save, Some(Pending::CloseTab { .. }))
            || self.queued.contains(&WorkbenchAction::CloseTab)
    }

    /// Being left for another tab: a backup copy of changes that no
    /// backup holds yet, so it doesn't wait for the tab to be used again.
    pub(crate) fn leave_tab(&mut self) -> Option<WorkbenchAction> {
        if !self.autosaver.leave(self.document.revision()) {
            return None;
        }
        let scene = self.document.scene().clone();
        Some(WorkbenchAction::Autosave {
            document: Box::new(Document::from_scene(scene, self.document.selection())),
            keep: self.autosaver.settings.copies as usize,
        })
    }

    /// The tabs to draw this frame (set by [`crate::SceneTabs`] before
    /// [`Workbench::show`]).
    pub(crate) fn set_tab_strip(&mut self, strip: TabStrip) {
        self.tab_strip = Some(strip);
    }

    /// The tabs drawn this frame, with what was clicked.
    pub(crate) fn take_tab_strip(&mut self) -> Option<TabStrip> {
        self.tab_strip.take()
    }

    /// The tabs on a row of their own above the menu bar, when Settings
    /// puts them there.
    pub(super) fn own_row_of_tabs(&mut self, ui: &mut egui::Ui) {
        let Some(strip) = self
            .tab_strip
            .as_mut()
            .filter(|s| s.placement == TabPlacement::OwnRow)
        else {
            return;
        };
        egui::Panel::top("scene_tabs")
            .frame(crate::theme::menu_bar_frame())
            .exact_size(crate::scene_tabs::OWN_ROW_HEIGHT)
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| crate::scene_tabs::strip::show(ui, strip));
            });
    }
}
