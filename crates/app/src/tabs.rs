//! What the app keeps for each scene tab (decision 0266), beside the
//! tab's workbench in `ui::SceneTabs`: the GPU renderer that draws its
//! views (meshes and textures on the GPU), its backup session and its
//! emergency copy. Made when a tab opens and let go when it closes, so a
//! closed tab's GPU memory and backup folder go with it.

use crate::backups::Backups;
use crate::emergency::{Emergency, Snapshots};
use crate::gpu::GpuViewport;
use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};
use ui::{NoRenderer, SceneTabs, TabId, ViewportRenderer, Workbench};
use words::tr;

/// One tab's share of the app.
pub struct TabParts {
    /// Draws the tab's views on the window's GPU.
    pub renderer: Box<dyn ViewportRenderer>,
    /// The tab's backup copies.
    pub backups: Backups,
    /// The tab's emergency copy, kept up to date.
    pub snapshots: Snapshots,
}

/// Every open tab's share, and how to make one for a new tab.
pub struct AllTabParts {
    parts: HashMap<TabId, TabParts>,
    /// The window's GPU, for new tabs' renderers (`None`: no GPU).
    gpu: Option<eframe::egui_wgpu::RenderState>,
    /// The emergency copy the panic hook holds; each tab's is its child.
    emergency: Emergency,
}

/// A renderer on the window's GPU, or one that says there is none.
pub fn renderer_for(gpu: Option<&eframe::egui_wgpu::RenderState>) -> Box<dyn ViewportRenderer> {
    match gpu {
        Some(state) => Box::new(GpuViewport::new(state)),
        None => Box::new(NoRenderer {
            reason: tr("the window has no GPU device").to_string(),
        }),
    }
}

impl AllTabParts {
    /// Starts with the first tab's `first` parts.
    pub fn new(
        first: (TabId, TabParts),
        gpu: Option<eframe::egui_wgpu::RenderState>,
        emergency: Emergency,
    ) -> Self {
        Self {
            parts: HashMap::from([first]),
            gpu,
            emergency,
        }
    }

    /// The tab `id`'s parts, if it has them.
    pub fn get_mut(&mut self, id: TabId) -> Option<&mut TabParts> {
        self.parts.get_mut(&id)
    }

    /// Makes parts for tabs that have just opened and lets go of closed
    /// tabs' (their backup copies are deleted: the work was saved or
    /// deliberately thrown away). Called every frame, before the tabs are
    /// shown.
    pub fn follow(&mut self, tabs: &mut SceneTabs) {
        for id in tabs.take_closed() {
            if let Some(mut gone) = self.parts.remove(&id) {
                gone.snapshots.emergency().forget();
                gone.backups.end();
            }
        }
        let folder = crate::backups::folder();
        for (id, workbench) in tabs.iter_mut() {
            if !self.parts.contains_key(&id) {
                let parts = self.make(folder.as_deref(), workbench);
                self.parts.insert(id, parts);
            }
        }
    }

    /// Puts the tab `id`'s renderer to sleep (decision 0267): what it
    /// held for drawing is let go of. Returns what was freed.
    pub fn sleep_renderer(&mut self, id: TabId) -> ui::MemoryFreed {
        self.parts
            .get_mut(&id)
            .map(|parts| parts.renderer.sleep())
            .unwrap_or_default()
    }

    /// A new tab's parts.
    fn make(&self, folder: Option<&Path>, workbench: &mut Workbench) -> TabParts {
        let backups = Backups::start_session(folder, workbench);
        let emergency = self.emergency.child();
        emergency.set_folder(backups.folder().map(Path::to_path_buf));
        TabParts {
            renderer: renderer_for(self.gpu.as_ref()),
            backups,
            snapshots: Snapshots::new(emergency),
        }
    }

    /// After a frame: every tab's finished backup reported and its
    /// project noted, and its emergency snapshot kept up to date. Returns
    /// when to look again for a snapshot that is due, if one is.
    pub fn update(&mut self, tabs: &mut SceneTabs) -> Option<Duration> {
        let now = Instant::now();
        let mut wait: Option<Duration> = None;
        for (id, workbench) in tabs.iter_mut() {
            let Some(parts) = self.parts.get_mut(&id) else {
                continue;
            };
            parts.backups.update(workbench);
            let unsaved = workbench.has_unsaved_changes();
            if let Some(due) = parts.snapshots.update(workbench.document(), unsaved, now) {
                wait = Some(wait.map_or(due, |w| w.min(due)));
            }
        }
        wait
    }

    /// A normal close: no tab's backup copies are needed any more.
    pub fn end(&mut self) {
        self.emergency.forget();
        for parts in self.parts.values_mut() {
            parts.snapshots.emergency().forget();
            parts.backups.end();
        }
    }
}
