//! The Settings window and the preferences the app keeps between
//! sessions: undo memory, units, the accent colour and the view options.

use super::Workbench;
use crate::settings::Values;
use crate::theme::Accent;
use crate::undo_memory::UndoMemory;
use crate::units::Units;
use crate::viewport::ViewOptions;
use scene::{Command, Document};

impl Workbench {
    /// The project's units: metres or centimetres.
    pub fn units(&self) -> Units {
        self.document.scene().units().into()
    }

    /// The units new projects start with, for saving between sessions.
    pub fn new_project_units(&self) -> Units {
        self.new_units
    }

    /// An empty document for a new project, in the units chosen for new
    /// projects.
    pub(super) fn new_document(&self) -> Document {
        Document::from_scene(scene::Scene::new().with_units(self.new_units.into()), [])
    }

    /// Makes a change of units made in the panels or Settings this frame
    /// (from `shown`, what the frame started with) an undoable step.
    pub(super) fn keep_units(&mut self, shown: Units) {
        if self.units == shown || self.units == self.units() {
            return;
        }
        let units = self.units;
        self.note = Some(
            match self.document.execute(Command::SetUnits(units.into())) {
                Ok(()) => crate::trf!(
                    "This project is now in {}: lengths show and are typed in {}. Ctrl + Z \
                     takes it back.",
                    units.name().to_lowercase(),
                    units.symbol()
                ),
                Err(why) => crate::trf!("Couldn't change the units: {}.", why),
            },
        );
    }

    /// Whether the Settings window is open.
    pub fn is_settings_open(&self) -> bool {
        self.settings.open
    }

    /// Opens the Settings window.
    pub fn open_settings(&mut self) {
        self.settings.open = true;
    }

    /// How much memory the undo history may use.
    pub fn undo_memory(&self) -> UndoMemory {
        self.undo_memory
    }

    /// The workbench with a remembered undo memory choice restored.
    #[must_use]
    pub fn with_undo_memory(mut self, memory: UndoMemory) -> Self {
        self.set_undo_memory(memory);
        self
    }

    fn set_undo_memory(&mut self, memory: UndoMemory) {
        self.undo_memory = memory;
        self.document.set_history_limit(memory.limit());
    }

    /// The workbench with the remembered units for new projects
    /// restored; the empty project it starts with gets them too.
    #[must_use]
    pub fn with_units(mut self, units: Units) -> Self {
        self.new_units = units;
        let untouched = self.document.scene().children(None).is_empty()
            && self.document.undo_steps() == 0
            && self.document.redo_steps() == 0;
        if untouched {
            self.replace_document(self.new_document());
        }
        self
    }

    /// The accent colour, for saving between sessions.
    pub fn accent(&self) -> Accent {
        self.accent
    }

    /// The workbench with a remembered accent colour restored.
    #[must_use]
    pub fn with_accent(mut self, accent: Accent) -> Self {
        self.accent = accent;
        self
    }

    /// Changes the accent colour (Window → Accent colour).
    pub fn set_accent(&mut self, accent: Accent) {
        self.accent = accent;
    }

    /// The workbench with remembered display choices restored.
    #[must_use]
    pub fn with_view_options(mut self, options: ViewOptions) -> Self {
        *self.viewport.options_mut() = options.migrated();
        self
    }

    /// Draws the Settings window, if open, and applies what changed.
    pub(super) fn settings_window(&mut self, ctx: &egui::Context) {
        let mut undo_memory = self.undo_memory;
        let history = (
            self.document.history_memory(),
            self.document.undo_steps() + self.document.redo_steps(),
        );
        let options = self.viewport.options_mut();
        let mut values = Values {
            accent: &mut self.accent,
            units: &mut self.units,
            new_units: &mut self.new_units,
            navigation: &mut options.navigation,
            grid: &mut options.grid_detail,
            select_visible_only: &mut options.select_visible_only,
            leave_element_modes_when_empty: &mut options.leave_element_modes_when_empty,
            select_keys: &mut options.select_keys,
            snap: &mut options.snap,
            undo_memory: &mut undo_memory,
            history,
            autosave: &mut self.autosaver.settings,
            autosave_folder: self.autosave_folder.as_deref(),
            tool_menu: &mut self.tool_menu.settings,
            dissolve_without_warning: &mut options.dissolve_without_warning,
            high_contrast: &mut options.high_contrast,
            check_for_updates: &mut options.check_for_updates,
            update_status: self.updating.status.as_deref(),
            skipped_update: &mut options.skipped_update,
            display: crate::settings::DisplayValues {
                hide_cavity: &mut options.hide_cavity,
                hide_hard_edges: &mut options.hide_hard_edges,
                show_seams: &mut options.show_seams,
                show_normals: &mut options.show_normals,
                show_collision: &mut options.show_collision,
                show_triangles: &mut options.show_triangles,
                flat_from_dice: &mut options.flat_from_dice,
            },
        };
        self.settings.show(ctx, &mut values);
        if undo_memory != self.undo_memory {
            self.set_undo_memory(undo_memory);
        }
    }
}
