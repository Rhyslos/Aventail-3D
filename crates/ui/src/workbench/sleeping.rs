//! A tab's workbench going to sleep and waking
//! ([decision 0267](../../../../docs/decisions/0267-sleeping-tabs.md)).
//!
//! Asleep, a workbench lets go of what it worked out and can work out
//! again (the scene drawing's summary, shapes after their cuts, modifiers
//! and levels of detail, the paint's mixes, the maps made for graph
//! fills, the Material mode's pictures) and stops its background work on
//! them. Everything else stays exactly as it was: the document with its
//! undo history and selection, the views and camera, the mode and tool,
//! the panel layouts, the file and its unsaved changes, the backups'
//! timer, the maps baked by hand (Bake maps, which take a while and
//! aren't saved). Waking is nothing more than drawing again: what was let
//! go of is worked out again from the document on the first frames.

use super::Workbench;
use crate::viewport::MemoryFreed;

/// Why a tab can't sleep yet: work the person started is under way, and
/// stopping it would lose it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StaysAwake {
    /// Saving or exporting.
    FileWork,
    /// The Paint layers panel's Bake maps.
    BakingMaps,
    /// File → Bake, writing maps to files.
    BakingToFiles,
    /// The Material mode's Export textures.
    ExportingTextures,
    /// Closing it is being asked about.
    Closing,
}

impl Workbench {
    /// Why this tab can't sleep now, if it can't: a save or export, a
    /// bake or a texture export under way, or its closing being asked
    /// about. It sleeps once that is done.
    pub(crate) fn stays_awake(&self) -> Option<StaysAwake> {
        if self.file_work().is_some() {
            Some(StaysAwake::FileWork)
        } else if self.bakes.busy() {
            Some(StaysAwake::BakingMaps)
        } else if self.baking_to_files() {
            Some(StaysAwake::BakingToFiles)
        } else if self.material.export_set.busy() {
            Some(StaysAwake::ExportingTextures)
        } else if self.closing_tab() {
            Some(StaysAwake::Closing)
        } else {
            None
        }
    }

    /// Puts the workbench to sleep: lets go of what it worked out and
    /// stops the background work on it (see the module's notes). Returns
    /// about how much memory that was. Nothing the person made or chose
    /// is touched.
    pub(crate) fn sleep(&mut self) -> MemoryFreed {
        let mut freed = MemoryFreed::default();
        freed.cpu += self.drawing.sleep();
        freed.cpu += self.graph_fills.sleep();
        self.seam_blends.cancel();
        // The Material mode's pictures are textures for egui, on the
        // graphics card.
        freed.gpu += self.material.pictures.sleep();
        freed.gpu += self.material.preview.sleep();
        freed.gpu += self.material.browser.pictures.sleep();
        self.sleep_bake_preview();
        self.asleep = true;
        freed
    }

    /// Wakes the workbench: the next frame draws as before, working out
    /// again what sleeping let go of.
    pub(crate) fn wake(&mut self) {
        self.asleep = false;
    }

    /// Whether the workbench is asleep.
    pub(crate) fn is_asleep(&self) -> bool {
        self.asleep
    }

    /// How many objects the scene drawing shows, and the paint's mixes
    /// (for tests).
    #[cfg(test)]
    pub(crate) fn drawn(&self) -> (usize, Vec<crate::PaintedTexture>) {
        self.drawing.shown()
    }

    /// Ends the save or export under way (for tests).
    #[cfg(test)]
    pub(crate) fn end_file_work(&mut self) {
        self.file_work_done();
    }

    /// Whether everything sleeping lets go of is gone (for tests).
    #[cfg(test)]
    pub(crate) fn holds_nothing_worked_out(&self) -> bool {
        self.drawing.is_empty()
            && !self.graph_fills.busy()
            && !self.seam_blends.busy()
            && !self.material.pictures.running()
            && !self.bake_preview_running()
    }
}
