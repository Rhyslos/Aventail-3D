//! Remesh for sculpting in the workbench ([decision 0312](../../../../docs/decisions/0312-remesh-for-sculpting.md)):
//! Ctrl + R (and the Sculpt menu, the Sculpt tab and the brush bar) bakes
//! the sculpted shape, and any other shapes selected, into one even
//! surface at the voxel size. The work runs on other threads with a
//! progress window and Cancel (Esc); when it ends, one
//! `Command::RemeshForSculpting` makes it one undo step. Past
//! [`HEAVY_POLYGONS`] it asks first; past [`MAX_SCULPT_POLYGONS`] it is
//! refused before it starts, with the voxel size that fits. While the
//! voxel size changes, a grid of that size shows on the shape.

use super::Workbench;
use crate::sculpt_masks::SculptAction;
use crate::text::tr;
use crate::viewport::SurfaceGrid;
use egui::{Key, Modifiers};
use mesh::remesh::{
    HEAVY_POLYGONS, MAX_SCULPT_POLYGONS, Progress, RemeshProblem, Remeshed, SculptRemeshOptions,
};
use scene::{Command, Material, Mesh, ObjectId};
use std::collections::HashMap;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// How long the voxel grid shows on the shape after the size changes.
const GRID_SHOWN: Duration = Duration::from_millis(1500);
/// How many voxels across the grid shown on the shape is.
const GRID_CELLS: u32 = 8;
/// A shape with fewer polygons has its area worked out at once; a bigger
/// one on another thread.
const AREA_AT_ONCE_BELOW: usize = 200_000;

/// What is remeshed, made ready before it starts.
pub(super) struct Order {
    id: ObjectId,
    joined: Vec<ObjectId>,
    /// The shapes as they were (the object's own first): if any changes
    /// before the remesh ends, it is dropped.
    was: Vec<Arc<Mesh>>,
    shape: Arc<Mesh>,
    materials: Option<Vec<Material>>,
    options: SculptRemeshOptions,
    polygons: u64,
}

/// A remesh under way.
pub(super) struct Running {
    order: Order,
    progress: Arc<Progress>,
    job: JoinHandle<Result<Remeshed, RemeshProblem>>,
    key: u64,
}

/// Remesh for sculpting's state in Sculpt mode.
#[derive(Default)]
pub(super) struct Remeshing {
    /// A remesh under way.
    running: Option<Running>,
    /// A heavy remesh waiting for its question's answer.
    asking: Option<Order>,
    /// Each shape's projected area, by object and the shape it was worked
    /// out for (the pointer of its `Arc`).
    areas: HashMap<(ObjectId, usize), f64>,
    /// Areas being worked out on other threads.
    pending: HashMap<(ObjectId, usize), JoinHandle<f64>>,
    /// The voxel size last seen, and until when its grid shows.
    voxel_seen: Option<f32>,
    grid_until: Option<Instant>,
}

impl Remeshing {
    /// Whether a remesh is under way.
    pub(super) fn busy(&self) -> bool {
        self.running.is_some()
    }

    /// Stops any remesh, and the question (leaving Sculpt mode).
    pub(super) fn stop(&mut self) {
        if let Some(running) = self.running.take() {
            running.progress.stop();
        }
        self.asking = None;
        self.voxel_seen = None;
        self.grid_until = None;
    }
}

impl Workbench {
    /// The shapes Remesh for sculpting works on: the sculpted one, and the
    /// others selected that have a shape of their own.
    fn remesh_shapes(&self) -> Option<(ObjectId, Vec<ObjectId>)> {
        let id = self.sculpting.live.as_ref()?.id;
        let scene = self.document.scene();
        let others = self
            .document
            .selection()
            .into_iter()
            .filter(|o| *o != id && scene.sculpt_target(*o).is_some())
            .collect();
        Some((id, others))
    }

    /// Object `id`'s largest scale: voxel sizes are in the world's
    /// centimetres, shapes in their own.
    fn shape_scale(&self, id: ObjectId) -> f32 {
        let scale = self
            .document
            .scene()
            .shape_matrix(id)
            .to_scale_rotation_translation()
            .0
            .abs()
            .max_element();
        if scale.is_finite() && scale > 1e-6 {
            scale
        } else {
            1.0
        }
    }

    /// Carries out Ctrl + R (the Sculpt menu, tab and bar): makes the order
    /// ready, refuses one too fine, asks about a heavy one, else starts it.
    pub(super) fn ask_remesh(&mut self) {
        if self.sculpting.stroke.is_some() || self.sculpting.moving.is_some() {
            return;
        }
        if self.sculpting.remeshing.busy() {
            self.note = Some(tr("Already remeshing: a moment.").to_string());
            return;
        }
        let Some((id, others)) = self.remesh_shapes() else {
            self.note = Some(tr("Select a shape to remesh it.").to_string());
            return;
        };
        let scene = self.document.scene();
        let Some(own) = scene.evaluated(id).map(|e| e.mesh) else {
            return;
        };
        let mut was = vec![Arc::clone(&own)];
        was.extend(
            others
                .iter()
                .filter_map(|o| scene.evaluated(*o).map(|e| e.mesh)),
        );
        let (shape, materials) = if others.is_empty() {
            (own, None)
        } else {
            let Some((joined, materials)) = scene.joined_shape(id, &others) else {
                return;
            };
            (Arc::new(joined), materials)
        };
        let scale = self.shape_scale(id);
        let settings = self.attributes.sculpt.remesh;
        let voxel = settings.voxel_size / scale;
        let area = mesh::remesh::projected_area(&shape);
        let polygons = mesh::remesh::polygons_from(area, voxel);
        if polygons > MAX_SCULPT_POLYGONS {
            let problem = RemeshProblem::TooManyPolygons {
                polygons,
                smallest: mesh::remesh::voxel_for(area, MAX_SCULPT_POLYGONS) * scale,
            };
            self.note = Some(crate::trf!("Couldn't remesh: {}.", problem));
            return;
        }
        let order = Order {
            id,
            joined: others,
            was,
            shape,
            materials,
            options: SculptRemeshOptions {
                voxel_size: voxel,
                keep_detail: settings.keep_detail,
                fix_holes: settings.fix_holes,
                ..Default::default()
            },
            polygons,
        };
        if polygons > HEAVY_POLYGONS {
            self.sculpting.remeshing.asking = Some(order);
        } else {
            self.start_remesh(order);
        }
    }

    /// Starts `order` on another thread.
    fn start_remesh(&mut self, order: Order) {
        let progress = Arc::new(Progress::default());
        let (shape, options, watch) = (
            Arc::clone(&order.shape),
            order.options,
            Arc::clone(&progress),
        );
        let job = std::thread::spawn(move || {
            mesh::remesh::remesh_for_sculpting(&shape, &options, None, &watch)
        });
        let key = self.new_step_key();
        self.sculpting.remeshing.running = Some(Running {
            order,
            progress,
            job,
            key,
        });
    }

    /// Each frame in Sculpt mode: a finished remesh becomes one undo step
    /// (dropped if the shapes changed meanwhile), and the estimate and
    /// voxel grid are kept up to date.
    pub(super) fn keep_remeshing(&mut self, ctx: &egui::Context) {
        if let Some(running) = &self.sculpting.remeshing.running {
            if running.job.is_finished() {
                if let Some(running) = self.sculpting.remeshing.running.take() {
                    self.finish_remesh(running);
                }
            } else {
                ctx.request_repaint_after(Duration::from_millis(50));
            }
        }
        self.attributes.sculpt.remesh.polygons = self.remesh_estimate();
        if !self.sculpting.remeshing.pending.is_empty() {
            ctx.request_repaint_after(Duration::from_millis(50));
        }
        self.show_voxel_grid(ctx);
    }

    /// A finished remesh: one undo step, or why not.
    fn finish_remesh(&mut self, running: Running) {
        let Running {
            order, job, key, ..
        } = running;
        let result = job.join().unwrap_or(Err(RemeshProblem::TooBig));
        let remeshed = match result {
            Ok(remeshed) => remeshed,
            Err(RemeshProblem::Cancelled) => {
                self.note = Some(tr("Remeshing stopped: nothing changed.").to_string());
                return;
            }
            Err(why) => {
                self.note = Some(crate::trf!("Couldn't remesh: {}.", why));
                return;
            }
        };
        let scene = self.document.scene();
        let now: Vec<Option<Arc<Mesh>>> = std::iter::once(order.id)
            .chain(order.joined.iter().copied())
            .map(|o| scene.evaluated(o).map(|e| e.mesh))
            .collect();
        let same = now.len() == order.was.len()
            && now
                .iter()
                .zip(&order.was)
                .all(|(n, w)| n.as_ref().is_some_and(|n| Arc::ptr_eq(n, w)));
        if !same {
            self.note = Some(
                tr(
                    "The shapes changed while they were remeshed, so the remesh was left out: \
                    press Ctrl + R again.",
                )
                .to_string(),
            );
            return;
        }
        let polygons = remeshed.mesh.face_count();
        let command = Command::RemeshForSculpting {
            id: order.id,
            shape: Arc::new(remeshed.mesh),
            materials: order.materials,
            joined: order.joined.clone(),
        };
        match self
            .document
            .execute_labelled(command, tr(SculptAction::Remesh.name()), Some(key))
        {
            Ok(()) => {
                self.note = Some(if order.joined.is_empty() {
                    crate::trf!(
                        "Remeshed into {} polygons: one even surface, ready to sculpt. \
                         Ctrl + Z takes it back.",
                        polygons
                    )
                } else {
                    crate::trf!(
                        "Merged {} shapes and remeshed them into {} polygons: one even \
                         surface, ready to sculpt. Ctrl + Z takes it back.",
                        order.joined.len() + 1,
                        polygons
                    )
                });
            }
            Err(why) => self.note = Some(crate::trf!("Couldn't remesh: {}.", why)),
        }
    }

    /// About how many polygons remeshing the selected shapes would make
    /// now, from their areas (worked out once per shape; a heavy shape's
    /// on another thread, `None` until it is ready).
    fn remesh_estimate(&mut self) -> Option<u64> {
        let (id, others) = self.remesh_shapes()?;
        let voxel = self.attributes.sculpt.remesh.voxel_size;
        let mut total = 0.0;
        let mut ready = true;
        for o in std::iter::once(id).chain(others) {
            let Some(shape) = self.document.scene().evaluated(o).map(|e| e.mesh) else {
                continue;
            };
            let key = (o, Arc::as_ptr(&shape) as usize);
            let scale = self.shape_scale(o);
            let state = &mut self.sculpting.remeshing;
            if let Some(job) = state.pending.remove(&key) {
                if job.is_finished() {
                    if let Ok(area) = job.join() {
                        state.areas.insert(key, area);
                    }
                } else {
                    state.pending.insert(key, job);
                }
            }
            match state.areas.get(&key) {
                Some(area) => total += area * f64::from(scale * scale),
                None if shape.face_count() < AREA_AT_ONCE_BELOW => {
                    let area = mesh::remesh::projected_area(&shape);
                    state.areas.insert(key, area);
                    total += area * f64::from(scale * scale);
                }
                None => {
                    let shape = Arc::clone(&shape);
                    state.pending.entry(key).or_insert_with(|| {
                        std::thread::spawn(move || mesh::remesh::projected_area(&shape))
                    });
                    ready = false;
                }
            }
        }
        // Areas of shapes no longer shown are forgotten.
        let state = &mut self.sculpting.remeshing;
        if state.areas.len() > 64 {
            state.areas.clear();
        }
        ready.then(|| mesh::remesh::polygons_from(total, voxel))
    }

    /// While the voxel size changes, and a moment after, a grid of that
    /// size on the shape where the view looks.
    fn show_voxel_grid(&mut self, ctx: &egui::Context) {
        let voxel = self.attributes.sculpt.remesh.voxel_size;
        let state = &mut self.sculpting.remeshing;
        let changed = state.voxel_seen.is_some_and(|v| v != voxel);
        state.voxel_seen = Some(voxel);
        if changed {
            state.grid_until = Some(Instant::now() + GRID_SHOWN);
        }
        let showing = state.grid_until.is_some_and(|t| Instant::now() < t);
        if !showing {
            state.grid_until = None;
            self.viewport.set_voxel_grid(None);
            return;
        }
        ctx.request_repaint_after(Duration::from_millis(50));
        let grid = self.voxel_grid_here(voxel);
        self.viewport.set_voxel_grid(grid);
    }

    /// The voxel grid where the middle of the view meets the shape (or at
    /// the shape's middle, facing the view).
    fn voxel_grid_here(&self, voxel: f32) -> Option<SurfaceGrid> {
        let live = self.sculpting.live.as_ref()?;
        let camera = *self.viewport.camera();
        let model = self.document.scene().shape_matrix(live.id);
        let inverse = model.inverse();
        if !inverse.is_finite() {
            return None;
        }
        let towards = -camera.forward();
        let (centre, normal) = match live.live.hit(
            inverse.transform_point3(camera.position),
            inverse.transform_vector3(camera.forward()),
        ) {
            Some(hit) => (
                model.transform_point3(hit.point),
                inverse
                    .transpose()
                    .transform_vector3(hit.normal)
                    .normalize_or(towards),
            ),
            None => {
                let bounds = live.live.mesh().bounds()?;
                (model.transform_point3(bounds.centre()), towards)
            }
        };
        Some(SurfaceGrid {
            centre,
            normal: if normal.dot(towards) < 0.0 {
                -normal
            } else {
                normal
            },
            step: voxel,
            cells: GRID_CELLS,
        })
    }

    /// The remesh's windows: the question before a heavy one, and the
    /// progress of one under way, with Cancel (Esc).
    pub(super) fn remesh_windows(&mut self, ctx: &egui::Context) {
        if let Some(order) = &self.sculpting.remeshing.asking {
            let millions = format!("{:.1}", order.polygons as f64 / 1e6);
            let mut answer = None;
            crate::dialog::Dialog::new(tr("Remesh into a heavy shape?"), 460.0)
                .id("sculpt_remesh_heavy")
                .modal()
                .show(ctx, |d| {
                    d.header(false, |_| {});
                    d.body(|ui| {
                        ui.label(crate::trf!(
                            "At this voxel size the shape would have about {} million polygons. \
                             Past {} million, sculpting slows down and baking takes longer.",
                            millions,
                            HEAVY_POLYGONS / 1_000_000
                        ));
                        crate::dialog::help(
                            ui,
                            tr("A bigger voxel size makes fewer polygons. Esc: don't remesh"),
                        );
                    });
                    d.footer(None, |ui| {
                        if crate::dialog::main_button(ui, tr("Remesh anyway")).clicked() {
                            answer = Some(true);
                        }
                        if crate::dialog::plain_button(ui, tr("Don't remesh")).clicked() {
                            answer = Some(false);
                        }
                    });
                });
            if answer.is_none() && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
                answer = Some(false);
            }
            match answer {
                Some(true) => {
                    if let Some(order) = self.sculpting.remeshing.asking.take() {
                        self.start_remesh(order);
                    }
                }
                Some(false) => self.sculpting.remeshing.asking = None,
                None => {}
            }
            return;
        }
        let Some(running) = &self.sculpting.remeshing.running else {
            return;
        };
        let share = running.progress.share();
        let name = self
            .document
            .scene()
            .object(running.order.id)
            .map_or_else(String::new, |o| o.name.clone());
        let mut cancel = false;
        // At the bottom, out of the shape's way.
        crate::dialog::Dialog::new(tr("Remeshing for sculpting"), 400.0)
            .id("sculpt_remeshing")
            .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -90.0])
            .show(ctx, |d| {
                d.header(false, |_| {});
                d.body(|ui| {
                    ui.label(crate::trf!("Remeshing {}…", name));
                    ui.add(egui::ProgressBar::new(share).show_percentage());
                });
                d.footer(None, |ui| {
                    if crate::dialog::plain_button(ui, tr("Cancel")).clicked() {
                        cancel = true;
                    }
                });
            });
        if cancel || ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            running.progress.stop();
        }
    }

    /// Whether the remesh's windows want the keys now.
    pub(super) fn remesh_holds_keys(&self) -> bool {
        let state = &self.sculpting.remeshing;
        state.asking.is_some() || state.running.is_some()
    }
}

#[cfg(test)]
impl Remeshing {
    /// Waits for the remesh under way to end (for tests).
    pub(super) fn wait(&self) {
        while self.running.as_ref().is_some_and(|r| !r.job.is_finished()) {
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// Whether a heavy remesh waits for its answer.
    pub(super) fn asking(&self) -> bool {
        self.asking.is_some()
    }

    /// Takes the voxel grid away now, so a window under test settles.
    pub(super) fn hide_grid(&mut self) {
        self.grid_until = None;
    }
}

/// How long the voxel grid shows (for tests).
#[cfg(test)]
pub(super) fn grid_shown_for() -> Duration {
    GRID_SHOWN
}
