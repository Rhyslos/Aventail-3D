//! File → Bake… ([decision 0250](../../../../docs/decisions/0250-the-baker.md)):
//! the maps a game asset is textured with (normal, occlusion, curvature,
//! thickness, colour ID, position and height), baked onto the selected
//! object's UVs, from a high-detail object of your choosing or from the
//! object itself, and written as PNG files into a folder you choose.
//!
//! The bake runs on another thread with a progress bar and Cancel; nothing
//! in the scene changes.

use super::{Workbench, WorkbenchAction};
use crate::text::tr;
use bake::maps::{CAGE_RANGE, High, MapKind, MapSettings};
use egui::Ui;
use scene::ObjectId;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::Ordering::Relaxed;

/// The window's title, and the File menu's entry.
pub(crate) const BAKER_TITLE: &str = crate::text!("Bake maps");
pub(crate) const BAKER_ENTRY: &str = crate::text!("Bake…");
/// The window's button.
pub(crate) const BAKE_BUTTON: &str = crate::text!("Bake into a folder…");
/// Bake each `_low` object from its `_high` ones.
pub(crate) const BY_NAME_LABEL: &str = crate::text!("Matched by name (_low and _high)");
/// The live preview's switch, its problem view, and all well.
pub(crate) const PREVIEW_LABEL: &str = crate::text!("Preview on the model");
pub(crate) const PROBLEMS_LABEL: &str = crate::text!("Problems");
pub(crate) const NO_PROBLEMS: &str =
    crate::text!("No problems: every texel found the high-detail model, square on.");
/// The cage brush's button.
pub(crate) const PAINT_CAGE_LABEL: &str = crate::text!("Paint the cage");

/// The selected objects named `…_low`, each with the objects whose names
/// start with the same and go on with `_high` (any case), in the scene's
/// order.
pub(crate) fn name_pairs(scene: &scene::Scene, selected: &[ObjectId]) -> Vec<(ObjectId, Vec<ObjectId>)> {
    let all: Vec<(ObjectId, String)> = scene
        .walk()
        .into_iter()
        .filter_map(|(id, _)| {
            let o = scene.object(id)?;
            o.kind.has_shape().then(|| (id, o.name.to_lowercase()))
        })
        .collect();
    selected
        .iter()
        .filter_map(|low| {
            let name = scene.object(*low)?.name.to_lowercase();
            let base = name.strip_suffix("_low")?;
            let wanted = format!("{base}_high");
            let highs: Vec<ObjectId> = all
                .iter()
                .filter(|(id, n)| id != low && n.starts_with(&wanted))
                .map(|(id, _)| *id)
                .collect();
            Some((*low, highs))
        })
        .collect()
}

/// "Bake from" when there is no high-detail object.
pub(crate) const FROM_ITSELF: &str = crate::text!("The object itself");

/// A map's name in the window.
pub(crate) fn map_name(kind: MapKind) -> &'static str {
    match kind {
        MapKind::Normal => tr("Normal"),
        MapKind::Occlusion => tr("Ambient occlusion"),
        MapKind::Curvature => tr("Curvature"),
        MapKind::Thickness => tr("Thickness"),
        MapKind::Id => tr("Colour ID"),
        MapKind::Position => tr("Position"),
        MapKind::Height => tr("Height"),
    }
}

/// What a map is for.
fn map_help(kind: MapKind) -> &'static str {
    match kind {
        MapKind::Normal => tr(
            "The high-detail shape's bumps and bevels as a tangent-space normal map, lit on the \
             low shape in a game as if they were there",
        ),
        MapKind::Occlusion => tr("Dark in creases and corners where light doesn't reach"),
        MapKind::Curvature => tr("Light on outer edges, dark in inner ones: for wear and dirt"),
        MapKind::Thickness => tr("Light where the shape is thin: for light shining through"),
        MapKind::Id => tr("A flat colour per material, to select parts by in a paint program"),
        MapKind::Position => tr("Where each part is on the shape: for gradients bottom to top"),
        MapKind::Height => tr(
            "How far above or below the low shape the high-detail one lies: grey on it",
        ),
    }
}

/// The colour a map's empty space is filled with: flat for normals,
/// open for occlusion, middle grey for curvature and height, black else.
fn background(kind: MapKind) -> [u8; 3] {
    match kind {
        MapKind::Normal => [128, 128, 255],
        MapKind::Occlusion => [255, 255, 255],
        MapKind::Curvature | MapKind::Height => [128, 128, 128],
        _ => [0, 0, 0],
    }
}

/// The window's state.
#[derive(Debug)]
pub(crate) struct Baker {
    pub(crate) open: bool,
    /// Which maps, in `MapKind::ALL`'s order.
    pub(crate) maps: [bool; 7],
    /// The high-detail object, if any.
    pub(crate) high: Option<ObjectId>,
    /// Each selected `_low` object baked from its `_high` ones (decision
    /// 0251), instead of `high`.
    pub(crate) by_name: bool,
    /// Whether the cage is drawn on the model while the window is open.
    pub(crate) show_cage: bool,
    /// The cage's push, as a share of the object's size.
    pub(crate) cage: f32,
    /// Normal maps with green down (DirectX).
    pub(crate) green_down: bool,
    /// The bake going on.
    job: Option<Job>,
    /// What the last bake did.
    pub(crate) last: Option<String>,
    /// The live preview on the model (decision 0252).
    pub(crate) preview: Preview,
}

/// What the live preview shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PreviewOf {
    /// A map.
    Map(MapKind),
    /// Where the bake goes wrong.
    Problems,
}

/// The live preview: a small bake shown on the model as settings change.
#[derive(Debug)]
pub(crate) struct Preview {
    pub(crate) on: bool,
    pub(crate) of: PreviewOf,
    /// What the shown preview was made from.
    made_for: Option<u64>,
    job: Option<PreviewJob>,
    /// The last problem counts, when baked from a high object.
    pub(crate) problems: Option<bake::maps::Problems>,
    /// The surface shown before the preview, to go back to.
    before: Option<crate::viewport::Surface>,
}

impl Default for Preview {
    fn default() -> Self {
        Self {
            on: false,
            of: PreviewOf::Map(MapKind::Normal),
            made_for: None,
            job: None,
            problems: None,
            before: None,
        }
    }
}

/// A preview bake on another thread.
struct PreviewJob {
    key: u64,
    progress: Arc<bake::Progress>,
    handle: std::thread::JoinHandle<Option<(paint::Image, Option<bake::maps::Problems>)>>,
}

impl std::fmt::Debug for PreviewJob {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreviewJob").field("key", &self.key).finish()
    }
}

/// The preview's size in pixels (at most; smaller textures stay theirs).
pub(crate) const PREVIEW_SIZE: u32 = 256;

/// The preview of `order` (its first object), at most [`PREVIEW_SIZE`].
pub(crate) fn preview_of(
    order: &BakeOrder,
    of: PreviewOf,
    progress: &bake::Progress,
) -> Option<(paint::Image, Option<bake::maps::Problems>)> {
    let surface = paint::PaintSurface::shaded(&order.low, order.low_hardness);
    if !surface.has_uvs() {
        return None;
    }
    let mut slots: Vec<u32> = surface.triangles().iter().map(|t| t.slot).collect();
    slots.sort_unstable();
    slots.dedup();
    let source = bake::Source {
        surface: &surface,
        slots: &slots,
        size: order.size.min(PREVIEW_SIZE),
    };
    let high = order
        .high
        .as_ref()
        .map(|(mesh, hardness)| High::of(&paint::PaintSurface::shaded(mesh, *hardness)));
    let extra = bake::maps::cage_pushes(&surface, &order.low);
    match of {
        PreviewOf::Map(kind) => bake::maps::bake_map_caged(
            &source,
            high.as_ref(),
            kind,
            &order.settings,
            Some(&extra),
            progress,
        )
        .map(|image| (image, None)),
        PreviewOf::Problems => {
            let high = high?;
            bake::maps::problem_map(&source, &high, &order.settings, Some(&extra), progress)
                .map(|(image, counts)| (image, Some(counts)))
        }
    }
}

/// What a problem means and what to do, for the window.
pub(crate) fn problem_words(problem: bake::maps::Problem, share: f32) -> String {
    let percent = (share * 100.0).max(0.1);
    match problem {
        bake::maps::Problem::Missed => crate::trf!(
            "{:.1}% of the texture found nothing to bake (red): the high-detail model lies further \
             out or in than the cage reaches there. Fix: raise Cage, or paint the cage out there.",
            percent
        ),
        bake::maps::Problem::WrongPart => crate::trf!(
            "{:.1}% probably found another part of the high-detail model, behind the one meant \
             (purple): the cage reaches too far there. Fix: lower Cage, or paint the cage in there.",
            percent
        ),
        bake::maps::Problem::Skewed => crate::trf!(
            "{:.1}% meets the high-detail surface at a steep slant, so its detail smears (yellow). \
             Fix: add an edge loop or a bevel to the low model there so it follows the high one, or \
             make that edge hard and a seam.",
            percent
        ),
    }
}

impl Default for Baker {
    fn default() -> Self {
        Self {
            open: false,
            maps: [true, true, true, false, true, false, false],
            high: None,
            by_name: false,
            show_cage: true,
            cage: MapSettings::default().cage,
            green_down: false,
            job: None,
            last: None,
            preview: Preview::default(),
        }
    }
}

/// A bake on another thread.
struct Job {
    progress: Arc<bake::Progress>,
    /// Which map of how many.
    step: Arc<std::sync::atomic::AtomicUsize>,
    count: usize,
    handle: std::thread::JoinHandle<Result<Vec<PathBuf>, String>>,
}

impl std::fmt::Debug for Job {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Job").field("count", &self.count).finish()
    }
}

/// Everything a bake needs, made on the window's thread.
pub(crate) struct BakeOrder {
    /// The file names start with this.
    pub(crate) name: String,
    pub(crate) low: Arc<scene::Mesh>,
    pub(crate) low_hardness: mesh::Hardness,
    pub(crate) high: Option<(Arc<scene::Mesh>, mesh::Hardness)>,
    /// The high-detail objects' names, for the window's note.
    pub(crate) high_names: Vec<String>,
    pub(crate) maps: Vec<MapKind>,
    pub(crate) size: u32,
    pub(crate) padding: u32,
    pub(crate) settings: MapSettings,
}

/// Bakes `order` into `folder`: the files written, or why not.
pub(crate) fn run_order(order: &BakeOrder, folder: &Path, progress: &bake::Progress, step: &std::sync::atomic::AtomicUsize) -> Result<Vec<PathBuf>, String> {
    let surface = paint::PaintSurface::shaded(&order.low, order.low_hardness);
    if !surface.has_uvs() {
        return Err(tr("the object has no UVs").to_string());
    }
    let mut slots: Vec<u32> = surface.triangles().iter().map(|t| t.slot).collect();
    slots.sort_unstable();
    slots.dedup();
    let source = bake::Source {
        surface: &surface,
        slots: &slots,
        size: order.size,
    };
    let high = order
        .high
        .as_ref()
        .map(|(mesh, hardness)| High::of(&paint::PaintSurface::shaded(mesh, *hardness)));
    std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
    let extra = bake::maps::cage_pushes(&surface, &order.low);
    let mut written = Vec::new();
    for (i, kind) in order.maps.iter().enumerate() {
        step.fetch_add(usize::from(i > 0), Relaxed);
        let Some(image) = bake::maps::bake_map_caged(
            &source,
            high.as_ref(),
            *kind,
            &order.settings,
            Some(&extra),
            progress,
        ) else {
            return Err(tr("cancelled").to_string());
        };
        let png = fileio::bake_file::bake_png(&image, order.padding, background(*kind))
            .ok_or_else(|| tr("the picture couldn't be written").to_string())?;
        let path = folder.join(format!("{}_{}.png", order.name, kind.file_suffix()));
        std::fs::write(&path, png).map_err(|e| e.to_string())?;
        written.push(path);
    }
    Ok(written)
}

/// `name` made safe for a file name.
fn file_safe(name: &str) -> String {
    let safe: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    if safe.trim_matches('_').is_empty() {
        "bake".to_string()
    } else {
        safe
    }
}

impl Workbench {
    /// The object the bake is onto: the first selected with a shape.
    fn bake_target(&self) -> Option<(ObjectId, Arc<scene::Mesh>)> {
        self.own_shapes().into_iter().find(|(_, m)| m.face_count() > 0)
    }

    /// Objects with a shape a bake can come from, besides `low`.
    fn bake_sources(&self, low: Option<ObjectId>) -> Vec<(ObjectId, String)> {
        let scene = self.document.scene();
        scene
            .walk()
            .into_iter()
            .filter_map(|(id, _)| {
                let object = scene.object(id)?;
                (Some(id) != low && object.kind.has_shape() && scene.evaluated(id).is_some())
                    .then(|| (id, object.name.clone()))
            })
            .collect()
    }

    /// The orders for a bake now: one for the object chosen, or with
    /// Matched by name one for each selected `_low` object (decision
    /// 0251); or why there can't be any.
    pub(crate) fn bake_orders(&self) -> Result<Vec<BakeOrder>, String> {
        if !self.baker.by_name {
            return self.bake_order().map(|o| vec![o]);
        }
        let scene = self.document.scene();
        let pairs = name_pairs(scene, &self.document.selection());
        if pairs.is_empty() {
            return Err(tr(
                "Matched by name: select objects whose names end in _low, with partners whose \
                 names start the same and go on with _high.",
            )
            .to_string());
        }
        pairs
            .into_iter()
            .map(|(low, highs)| self.order_for(low, &highs))
            .collect()
    }

    /// The order for a bake now, or why there can't be one.
    pub(crate) fn bake_order(&self) -> Result<BakeOrder, String> {
        let Some((low_id, low)) = self.bake_target() else {
            return Err(tr("Select the low-detail object to bake onto first.").to_string());
        };
        if !mesh::uvs::has_uvs(&low) {
            return Err(tr("The selected object has no UVs: unwrap it first.").to_string());
        }
        let maps: Vec<MapKind> = MapKind::ALL
            .iter()
            .zip(self.baker.maps)
            .filter(|(_, on)| *on)
            .map(|(k, _)| *k)
            .collect();
        if maps.is_empty() {
            return Err(tr("Choose at least one map.").to_string());
        }
        let highs: Vec<ObjectId> = self.baker.high.into_iter().collect();
        self.order_for(low_id, &highs)
    }

    /// The order for baking `low` from `highs` (none: from itself).
    fn order_for(&self, low_id: ObjectId, highs: &[ObjectId]) -> Result<BakeOrder, String> {
        let scene = self.document.scene();
        let Some(low) = scene.evaluated(low_id).map(|e| e.mesh) else {
            return Err(tr("The object to bake onto has no shape.").to_string());
        };
        if !mesh::uvs::has_uvs(&low) {
            let name = scene.object(low_id).map_or_else(String::new, |o| o.name.clone());
            return Err(crate::trf!("{} has no UVs: unwrap it first.", name));
        }
        let maps: Vec<MapKind> = MapKind::ALL
            .iter()
            .zip(self.baker.maps)
            .filter(|(_, on)| *on)
            .map(|(k, _)| *k)
            .collect();
        if maps.is_empty() {
            return Err(tr("Choose at least one map.").to_string());
        }
        // In the world, so a high-detail object placed over the low one
        // bakes where it is; several high ones (floating details among
        // them) as one.
        let mut high_mesh = scene::Mesh::new();
        let mut high_names = Vec::new();
        let mut hardness = None;
        for h in highs.iter().filter(|h| **h != low_id) {
            let Some(evaluated) = scene.evaluated(*h) else {
                continue;
            };
            mesh::parts::append(&mut high_mesh, &evaluated.mesh, scene.world_matrix(*h));
            high_names.extend(scene.object(*h).map(|o| o.name.clone()));
            hardness.get_or_insert(self.smoothing_of(*h));
        }
        let high = (high_mesh.face_count() > 0)
            .then(|| (Arc::new(high_mesh), hardness.unwrap_or(mesh::Hardness::Marked)));
        let mut low_world = scene::Mesh::new();
        mesh::parts::append(&mut low_world, &low, scene.world_matrix(low_id));
        let name = scene.object(low_id).map_or_else(String::new, |o| o.name.clone());
        Ok(BakeOrder {
            name: file_safe(&name),
            low: Arc::new(low_world),
            low_hardness: self.smoothing_of(low_id),
            high,
            high_names,
            maps,
            size: scene.texture().size,
            padding: self.uv_settings.padding.max(1),
            settings: MapSettings {
                cage: self.baker.cage.clamp(*CAGE_RANGE.start(), *CAGE_RANGE.end()),
                green_down: self.baker.green_down,
                ..MapSettings::default()
            },
        })
    }

    /// Starts baking into `folder` (from the folder window).
    pub fn bake_into(&mut self, folder: PathBuf) {
        if self.baker.job.is_some() {
            return;
        }
        let orders = match self.bake_orders() {
            Ok(orders) => orders,
            Err(why) => {
                self.baker.last = Some(why);
                return;
            }
        };
        let progress = Arc::new(bake::Progress::default());
        let step = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count = orders.iter().map(|o| o.maps.len()).sum();
        let (p, s) = (Arc::clone(&progress), Arc::clone(&step));
        let handle = std::thread::spawn(move || {
            let mut written = Vec::new();
            for (i, order) in orders.iter().enumerate() {
                if i > 0 {
                    s.fetch_add(1, Relaxed);
                }
                written.extend(run_order(order, &folder, &p, &s)?);
            }
            Ok(written)
        });
        self.baker.job = Some(Job {
            progress,
            step,
            count,
            handle,
        });
        self.baker.last = None;
    }

    /// Whether a bake is going on.
    pub(crate) fn baking_maps(&self) -> bool {
        self.baker.job.is_some()
    }

    /// Picks up a finished bake.
    fn collect_bake(&mut self) {
        if !self.baker.job.as_ref().is_some_and(|j| j.handle.is_finished()) {
            return;
        }
        let Some(job) = self.baker.job.take() else {
            return;
        };
        let said = match job.handle.join() {
            Ok(Ok(files)) => {
                let folder = files
                    .first()
                    .and_then(|f| f.parent())
                    .map_or_else(String::new, |f| f.display().to_string());
                crate::trf!(
                    "Baked {} maps into {}.",
                    files.len(),
                    folder
                )
            }
            Ok(Err(why)) => crate::trf!("The bake stopped: {}.", why),
            Err(_) => tr("The bake stopped: something went wrong.").to_string(),
        };
        self.note = Some(said.clone());
        self.baker.last = Some(said);
    }

    /// Waits for the bake going on (for tests).
    #[cfg(test)]
    pub(crate) fn finish_bake(&mut self) {
        while self.baker.job.as_ref().is_some_and(|j| !j.handle.is_finished()) {
            std::thread::yield_now();
        }
        self.collect_bake();
    }

    /// What the preview would be made from now, as a number: the shapes
    /// (by their place in memory, which changes with every edit) and the
    /// settings. `None` when there is nothing to preview.
    fn preview_key(&self) -> Option<u64> {
        use std::hash::{Hash, Hasher};
        let (low, _) = self.bake_target()?;
        let scene = self.document.scene();
        let highs: Vec<ObjectId> = if self.baker.by_name {
            name_pairs(scene, &[low]).into_iter().flat_map(|(_, h)| h).collect()
        } else {
            self.baker.high.into_iter().collect()
        };
        let mut h = std::collections::hash_map::DefaultHasher::new();
        for id in std::iter::once(low).chain(highs) {
            id.hash(&mut h);
            scene
                .evaluated(id)
                .map(|e| Arc::as_ptr(&e.mesh) as usize)
                .hash(&mut h);
            scene.world_matrix(id).to_cols_array().map(f32::to_bits).hash(&mut h);
        }
        self.baker.cage.to_bits().hash(&mut h);
        self.baker.green_down.hash(&mut h);
        self.baker.by_name.hash(&mut h);
        format!("{:?}", self.baker.preview.of).hash(&mut h);
        scene.texture().size.hash(&mut h);
        Some(h.finish())
    }

    /// Each frame while the window is open: starts a preview bake when
    /// what it would show has changed, and shows a finished one on the
    /// model (the Texture picture surface, decision 0244) and behind the
    /// UVs.
    pub(crate) fn update_bake_preview(&mut self, ctx: &egui::Context) {
        if let Some(job) = &self.baker.preview.job
            && job.handle.is_finished()
            && let Some(job) = self.baker.preview.job.take()
        {
            if let Ok(Some((image, problems))) = job.handle.join() {
                let size = image.size();
                let picture = crate::viewport::Picture::square(
                    tr("Bake preview"),
                    size,
                    size,
                    &image.to_rgba(),
                );
                self.viewport.set_picture(picture);
                let options = self.viewport.options_mut();
                if self.baker.preview.before.is_none() {
                    self.baker.preview.before = Some(options.surface);
                }
                options.surface = crate::viewport::Surface::Texture;
                self.baker.preview.problems = problems;
                self.baker.preview.made_for = Some(job.key);
            }
            ctx.request_repaint();
        }
        if !self.baker.open || !self.baker.preview.on {
            return;
        }
        let Some(key) = self.preview_key() else {
            return;
        };
        let running = self.baker.preview.job.as_ref().map(|j| j.key);
        if self.baker.preview.made_for == Some(key) || running == Some(key) {
            return;
        }
        if let Some(job) = &self.baker.preview.job {
            // Out of date: stop it and start again when it has stopped.
            job.progress.cancel.store(true, Relaxed);
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
            return;
        }
        let Some(order) = self.bake_orders().ok().and_then(|o| o.into_iter().next()) else {
            return;
        };
        let of = self.baker.preview.of;
        let progress = Arc::new(bake::Progress::default());
        let p = Arc::clone(&progress);
        let handle = std::thread::spawn(move || preview_of(&order, of, &p));
        self.baker.preview.job = Some(PreviewJob {
            key,
            progress,
            handle,
        });
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }

    /// Stops the preview: the surface goes back to what it was.
    pub(crate) fn stop_bake_preview(&mut self) {
        let preview = &mut self.baker.preview;
        preview.on = false;
        preview.made_for = None;
        preview.problems = None;
        if let Some(job) = preview.job.take() {
            job.progress.cancel.store(true, Relaxed);
        }
        if let Some(before) = preview.before.take() {
            self.viewport.options_mut().surface = before;
            self.viewport.set_picture(None);
        }
    }

    /// Waits for the preview going on and shows it (for tests).
    #[cfg(test)]
    pub(crate) fn finish_bake_preview(&mut self, ctx: &egui::Context) {
        while self
            .baker
            .preview
            .job
            .as_ref()
            .is_some_and(|j| !j.handle.is_finished())
        {
            std::thread::yield_now();
        }
        self.update_bake_preview(ctx);
    }

    /// The window, while open.
    pub(super) fn baker_window(&mut self, ctx: &egui::Context, actions: &mut Vec<WorkbenchAction>) {
        self.collect_bake();
        self.update_bake_preview(ctx);
        if !self.baker.open {
            return;
        }
        if self.baker.job.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        let low = self.bake_target();
        let sources = self.bake_sources(low.as_ref().map(|(id, _)| *id));
        let low_name = low
            .as_ref()
            .and_then(|(id, _)| self.document.scene().object(*id))
            .map(|o| o.name.clone());
        let size = self.document.scene().texture().size;
        let mut bake = false;
        let mut cancel = false;
        let mut close = false;
        let outcome = crate::dialog::Dialog::new(tr(BAKER_TITLE), 480.0).show(ctx, |d| {
            d.header(true, |_| {});
            d.body(|ui| self.baker_body(ui, low_name.as_deref(), &sources, size));
            let busy = self.baker.job.as_ref().map(|j| {
                (
                    j.step.load(Relaxed) + 1,
                    j.count,
                    j.progress.share(),
                )
            });
            let note = match busy {
                Some((step, count, share)) => Some(crate::trf!(
                    "Baking map {} of {}… {}%",
                    step,
                    count,
                    (share * 100.0).floor()
                )),
                None => self.baker.last.clone(),
            };
            d.footer(note.as_deref(), |ui| {
                if busy.is_some() {
                    if crate::dialog::plain_button(ui, tr("Cancel")).clicked() {
                        cancel = true;
                    }
                } else {
                    if crate::dialog::main_button(ui, tr(BAKE_BUTTON)).clicked() {
                        bake = true;
                    }
                    if crate::dialog::plain_button(ui, tr("Close")).clicked() {
                        close = true;
                    }
                }
            });
        });
        if cancel && let Some(job) = &self.baker.job {
            job.progress.cancel.store(true, Relaxed);
        }
        if bake {
            match self.bake_orders() {
                Ok(_) => actions.push(WorkbenchAction::ChooseBakeFolder),
                Err(why) => self.baker.last = Some(why),
            }
        }
        if close || outcome.closed {
            self.baker.open = false;
            self.stop_bake_preview();
        }
    }

    fn baker_body(&mut self, ui: &mut Ui, low: Option<&str>, sources: &[(ObjectId, String)], size: u32) {
        crate::dialog::row(ui, tr("Onto"), |ui| match low {
            Some(name) => ui.label(name),
            None => ui.weak(tr("Select the low-detail object")),
        });
        ui.checkbox(&mut self.baker.by_name, tr(BY_NAME_LABEL)).on_hover_text(tr(
            "Bake each selected object whose name ends in _low from the objects named like it \
             with _high (crate_low from crate_high, and crate_high_bolts as floating details). \
             Each pair is baked on its own, as if moved apart, so neighbours don't spoil each \
             other's bake.",
        ));
        if !self.baker.by_name {
        crate::dialog::row(ui, tr("Bake from"), |ui| {
            let chosen = self
                .baker
                .high
                .and_then(|h| sources.iter().find(|(id, _)| *id == h))
                .map_or_else(|| tr(FROM_ITSELF).to_string(), |(_, n)| n.clone());
            egui::ComboBox::from_id_salt("bake from")
                .selected_text(chosen)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.baker.high, None, tr(FROM_ITSELF));
                    for (id, name) in sources {
                        ui.selectable_value(&mut self.baker.high, Some(*id), name);
                    }
                })
                .response
                .on_hover_text(tr(
                    "The high-detail object whose bumps and bevels the normal and height maps \
                     take. Place it over the low one. The other maps come from the low object \
                     itself.",
                ))
        });
        }
        ui.separator();
        for (on, kind) in self.baker.maps.iter_mut().zip(MapKind::ALL) {
            ui.checkbox(on, map_name(kind)).on_hover_text(map_help(kind));
        }
        ui.separator();
        crate::dialog::row(ui, tr("Cage"), |ui| {
            let mut percent = self.baker.cage * 100.0;
            let response = ui
                .add(
                    egui::Slider::new(&mut percent, CAGE_RANGE.start() * 100.0..=CAGE_RANGE.end() * 100.0)
                        .logarithmic(true)
                        .suffix(tr("% of its size")),
                )
                .on_hover_text(tr(
                    "How far out from the low object the bake looks for the high-detail one, \
                     and how far in: enough to reach every bump, not so much it finds the wrong \
                     part (2% standard)",
                ));
            if response.changed() {
                self.baker.cage = percent / 100.0;
            }
            response
        });
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.baker.show_cage, tr("Show the cage"))
                .on_hover_text(tr("Draw the cage over the selected object, in cyan"));
            let painting = self.cage_brush_on();
            if ui
                .selectable_label(painting, tr(PAINT_CAGE_LABEL))
                .on_hover_text(tr(
                    "Drag over points to push the cage further out there; Ctrl + drag pulls it \
                     in. [ and ] change the size; Esc stops.",
                ))
                .clicked()
            {
                if painting {
                    self.stop_cage_brush();
                } else {
                    self.start_cage_brush();
                }
            }
            if ui.button(tr("Clear")).on_hover_text(tr("Clear the painted cage")).clicked() {
                self.clear_cage();
            }
        });
        ui.checkbox(
            &mut self.baker.green_down,
            tr("Normal maps green down (DirectX, for Unreal)"),
        )
        .on_hover_text(tr(
            "Off (standard): green up, as Godot and Blender read normal maps. On: green down, \
             as Unreal and other DirectX engines do.",
        ));
        ui.separator();
        ui.horizontal(|ui| {
            let mut on = self.baker.preview.on;
            if ui
                .checkbox(&mut on, tr(PREVIEW_LABEL))
                .on_hover_text(tr(
                    "Bake small and show it on the model and behind the UVs, again whenever the \
                     models or the settings change",
                ))
                .changed()
            {
                if on {
                    self.baker.preview.on = true;
                } else {
                    self.stop_bake_preview();
                }
            }
            let shown = match self.baker.preview.of {
                PreviewOf::Map(kind) => map_name(kind).to_string(),
                PreviewOf::Problems => tr(PROBLEMS_LABEL).to_string(),
            };
            egui::ComboBox::from_id_salt("bake preview of")
                .selected_text(shown)
                .show_ui(ui, |ui| {
                    for kind in MapKind::ALL {
                        ui.selectable_value(&mut self.baker.preview.of, PreviewOf::Map(kind), map_name(kind));
                    }
                    ui.selectable_value(&mut self.baker.preview.of, PreviewOf::Problems, tr(PROBLEMS_LABEL))
                        .on_hover_text(tr(
                            "Where the bake goes wrong, coloured on the model: red found nothing, \
                             purple probably another part, yellow smeared",
                        ));
                });
            if self.baker.preview.job.is_some() {
                ui.spinner();
            }
        });
        if self.baker.preview.on
            && let Some(problems) = self.baker.preview.problems
        {
            let mut any = false;
            for problem in bake::maps::Problem::ALL {
                let share = problems.share(problem);
                if share >= 0.005 {
                    any = true;
                    ui.label(egui::RichText::new(problem_words(problem, share)).small());
                }
            }
            if !any {
                ui.label(egui::RichText::new(tr(NO_PROBLEMS)).small());
            }
        }
        ui.label(
            egui::RichText::new(crate::trf!(
                "{} × {} pixels (the project's texture size), padded {} pixels round the pieces.",
                size,
                size,
                self.uv_settings.padding.max(1)
            ))
            .small(),
        );
    }
}
