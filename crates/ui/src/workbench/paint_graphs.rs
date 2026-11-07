//! Graph fills in the painter (Phase 12, milestone 5,
//! [decision 0261](../../../../docs/decisions/0261-graphs-in-the-painter.md)):
//! making each graph fill layer's pixels from its material graph, on the
//! model it is painted on, on another thread.
//!
//! A graph fill keeps what its pixels were made from (`GraphFill::made`,
//! a hash of the graph with the layer's own values, the model's shape and
//! UVs, how many times the material repeats across it, the texture size
//! and the realism switch). Whenever the scene or the drawing changes,
//! and nothing is held down, the layers whose hash differs from what they
//! would be made from now are made again, one at a time: after their
//! settings change, after the graph changes in the Material mode, after
//! the model is changed (paint following the model, decision 0189) or the
//! texture size. Undo puts back the settings and pixels together, so a
//! layer undone is already up to date.
//!
//! Making one takes two steps on another thread: the model's maps the
//! graph reads (`bake::model_maps`, kept for the next time), then the
//! graph (`bake::graph_fill`, with an evaluator kept between runs, so
//! after a small change only the boxes it touched are worked out again).
//! The result joins the undo step of the change that asked for it, as
//! made layers' pixels do, if nothing else changed meanwhile; otherwise
//! it is worked out again for the scene as it now is.

use super::Workbench;
use crate::drawing::SceneDrawing;
use bake::{MadeGraph, ModelBake};
use matgraph::{Evaluator, Graph, GraphId, ModelMap, Refusal, Registry};
use paint::{ChannelPart, Content, GraphFill, Layer, LayerPath, PaintSurface, TextureSet};
use std::collections::{BTreeSet, HashMap};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError, channel};

/// What a finished job sends back.
struct Done {
    evaluator: Evaluator,
    bake: Option<(u64, Arc<ModelBake>)>,
    made: Result<MadeGraph, Refusal>,
}

/// A graph fill being made.
struct Job {
    material: String,
    path: LayerPath,
    /// The layer's settings when it started (its `made` aside).
    fill: GraphFill,
    /// What it is being made from (the new `made`).
    key: u64,
    /// The document's revision when it started.
    revision: u64,
    realism: bool,
    from: Receiver<Done>,
    progress: Arc<bake::Progress>,
}

/// The graph fills' state.
#[derive(Default)]
pub(crate) struct GraphFills {
    job: Option<Job>,
    /// Kept between jobs (taken by the one under way).
    evaluator: Option<Evaluator>,
    /// The model's maps last baked for each material, and what they were
    /// baked from.
    bakes: HashMap<String, (u64, Arc<ModelBake>)>,
    /// The scene and drawing last looked through.
    checked: Option<(u64, u64)>,
    /// Layers that couldn't be made from what they would be made from
    /// now (by material and that hash), so they aren't tried again until
    /// it changes: where they were, and why.
    failed: HashMap<(String, u64), (LayerPath, String)>,
}

impl std::fmt::Debug for GraphFills {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GraphFills")
            .field("busy", &self.job.is_some())
            .field("bakes", &self.bakes.len())
            .field("failed", &self.failed.len())
            .finish()
    }
}

impl GraphFills {
    /// Whether a graph fill is being made.
    #[cfg(test)]
    pub(crate) fn busy(&self) -> bool {
        self.job.is_some()
    }

    /// Stops any making under way (a new project).
    pub(super) fn cancel(&mut self) {
        if let Some(job) = self.job.take() {
            job.progress
                .cancel
                .store(true, std::sync::atomic::Ordering::Relaxed);
        }
        self.bakes.clear();
        self.failed.clear();
        self.checked = None;
    }
}

/// A graph fill layer waiting to be made: where, and from what.
struct Wanted {
    material: String,
    path: LayerPath,
    fill: GraphFill,
    graph: Graph,
    key: u64,
    repeats: f32,
}

impl Workbench {
    /// Each frame after the drawing is made: takes a finished graph fill,
    /// and starts the next one waiting, when nothing is held down.
    pub(super) fn update_graph_fills(&mut self, drawing: &SceneDrawing, ctx: &egui::Context) {
        self.collect_graph_fill();
        self.show_graph_fill_status();
        if self.graph_fills.job.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(60));
            return;
        }
        let held = ctx.input(|i| i.pointer.any_down()) || self.paint_layers.is_dragging();
        let now = (self.document.revision(), drawing.version);
        if held || self.graph_fills.checked == Some(now) {
            return;
        }
        self.graph_fills.checked = Some(now);
        if let Some(wanted) = self.next_graph_fill(drawing) {
            self.start_graph_fill(wanted, drawing);
            ctx.request_repaint();
        }
    }

    /// The first graph fill layer whose pixels aren't made from what it
    /// would be made from now (and that hasn't failed to be).
    fn next_graph_fill(&mut self, drawing: &SceneDrawing) -> Option<Wanted> {
        let registry = Registry::standard();
        let sets: Vec<(String, Arc<TextureSet>)> = self
            .document
            .scene()
            .paint()
            .sets()
            .map(|(m, s)| (m.to_string(), Arc::clone(s)))
            .collect();
        for (material, set) in sets {
            let layers: Vec<(LayerPath, GraphFill)> = set
                .listed(true)
                .into_iter()
                .filter_map(|(path, _)| Some((path.clone(), set.layer(&path)?.graph.clone()?)))
                .collect();
            if layers.is_empty() {
                continue;
            }
            let layout = self.layout_key(&material, drawing);
            for (path, fill) in layers {
                let Some(graph) = self.document.scene().graphs().get(fill.graph).cloned() else {
                    continue;
                };
                let graph = with_values(registry, &graph, &fill);
                let Some(repeats) = self.material_repeats(&material, drawing, &graph) else {
                    continue;
                };
                let key = made_key(&graph, set.size, layout, repeats, set.realism);
                // Why it couldn't be made from something else is old news.
                self.graph_fills
                    .failed
                    .retain(|(m, k), (p, _)| !(*m == material && *p == path && *k != key));
                if key == fill.made
                    || self
                        .graph_fills
                        .failed
                        .contains_key(&(material.clone(), key))
                {
                    continue;
                }
                return Some(Wanted {
                    material,
                    path,
                    fill,
                    graph,
                    key,
                    repeats,
                });
            }
        }
        None
    }

    /// How many times `graph`'s material repeats across `material`'s
    /// texture square: on its first shape, by that shape's UVs and size
    /// and the graph's real-world size (as the Material mode's preview
    /// lays it, decision 0260). `None` when nothing shown uses it.
    fn material_repeats(
        &mut self,
        material: &str,
        drawing: &SceneDrawing,
        graph: &Graph,
    ) -> Option<f32> {
        let shapes = self.material_shapes(material, drawing);
        let (id, _, _) = shapes.first()?;
        let item = drawing
            .items
            .iter()
            .find(|i| i.id.get() == *id && !i.outline && i.cage.is_none())?;
        let real = matgraph::real_size_cm(Registry::standard(), graph);
        // A shape without UVs: the material once across the texture.
        Some(crate::material::laying::repeats(&item.drawn, item.transform, real).unwrap_or(1.0))
    }

    /// Starts making `wanted` on another thread.
    fn start_graph_fill(&mut self, wanted: Wanted, drawing: &SceneDrawing) {
        let Wanted {
            material,
            path,
            fill,
            graph,
            key,
            repeats,
        } = wanted;
        let Some(set) = self.document.scene().paint().set(&material).cloned() else {
            return;
        };
        let shapes: Vec<(Arc<PaintSurface>, Vec<u32>)> = self
            .material_shapes(&material, drawing)
            .into_iter()
            .map(|(_, surface, slots)| (surface, slots))
            .collect();
        if shapes.is_empty() {
            return;
        }
        let registry = Registry::standard();
        let needed: BTreeSet<ModelMap> = matgraph::model::maps_needed(registry, &graph);
        let bake_key = {
            let mut h = DefaultHasher::new();
            (self.layout_key(&material, drawing), set.size).hash(&mut h);
            needed.hash(&mut h);
            h.finish()
        };
        let kept = self
            .graph_fills
            .bakes
            .get(&material)
            .filter(|(k, _)| *k == bake_key)
            .map(|(_, b)| Arc::clone(b));
        let mut evaluator = self.graph_fills.evaluator.take().unwrap_or_default();
        let settings = bake::BakeSettings {
            rays: self.paint_layers.bakes.quality.rays(),
            ..bake::BakeSettings::default()
        };
        let size = set.size;
        let (to, from) = channel();
        let progress = Arc::new(bake::Progress::default());
        let watched = Arc::clone(&progress);
        std::thread::spawn(move || {
            let bake = match kept {
                Some(bake) => Some((bake_key, bake)),
                None => {
                    let sources: Vec<bake::Source<'_>> = shapes
                        .iter()
                        .map(|(surface, slots)| bake::Source {
                            surface,
                            slots,
                            size,
                        })
                        .collect();
                    bake::model_maps(&sources, &needed, &settings, &watched, bake_key)
                        .map(|b| (bake_key, Arc::new(b)))
                }
            };
            let made = match &bake {
                Some((_, bake)) => {
                    bake::graph_fill(&mut evaluator, registry, &graph, bake, repeats)
                }
                None => Err(Refusal::new(crate::text::tr(
                    "The texture is too small for a graph: graphs start at 16 pixels.",
                ))),
            };
            let _ = to.send(Done {
                evaluator,
                bake,
                made,
            });
        });
        self.graph_fills.job = Some(Job {
            material,
            path,
            fill,
            key,
            revision: self.document.revision(),
            realism: set.realism,
            from,
            progress,
        });
    }

    /// Takes a finished graph fill into its layer.
    fn collect_graph_fill(&mut self) {
        let Some(job) = &self.graph_fills.job else {
            return;
        };
        let done = match job.from.try_recv() {
            Ok(done) => done,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => {
                self.graph_fills.job = None;
                return;
            }
        };
        let Some(job) = self.graph_fills.job.take() else {
            return;
        };
        self.graph_fills.evaluator = Some(done.evaluator);
        if let Some(bake) = done.bake {
            self.graph_fills.bakes.insert(job.material.clone(), bake);
        }
        let made = match done.made {
            Ok(made) => made,
            Err(why) => {
                self.graph_fills
                    .failed
                    .insert((job.material, job.key), (job.path, why.reason));
                // Look again: another layer may be waiting.
                self.graph_fills.checked = None;
                return;
            }
        };
        // Something changed while it was made: look again, for the scene
        // as it is now (quick: the maps and boxes are kept).
        if self.document.revision() != job.revision {
            self.graph_fills.checked = None;
            return;
        }
        let Some(set) = self.document.scene().paint().set(&job.material) else {
            return;
        };
        let mut set = (**set).clone();
        let size = set.size;
        let Some(layer) = set.layer_mut(&job.path) else {
            return;
        };
        if !same_fill(layer.graph.as_ref(), &job.fill) || made.colour.size() != size {
            self.graph_fills.checked = None;
            return;
        }
        put(layer, made, job.key, job.realism);
        let command = scene::Command::SetTextureSet {
            material: job.material,
            set: Some(set),
        };
        if let Err(why) = self.document.execute_with_last(command) {
            self.note = Some(crate::trf!("Couldn't make the graph layer: {}.", why));
        }
        self.graph_fills.checked = None;
    }

    /// Tells the Paint layers panel which graph fill is being made, and
    /// why any couldn't be.
    fn show_graph_fill_status(&mut self) {
        let panel = &mut self.paint_layers;
        panel.graph_making = self
            .graph_fills
            .job
            .as_ref()
            .map(|j| (j.material.clone(), j.path.clone()));
        panel.graph_problems = self
            .graph_fills
            .failed
            .iter()
            .map(|((m, _), (path, why))| ((m.clone(), path.clone()), why.clone()))
            .collect();
    }

    /// Opens graph `id` in the Material mode (the Paint layers panel's
    /// "Open in the graph").
    pub(super) fn open_graph_from_paint(&mut self) {
        if let Some(id) = self.paint_layers.open_graph_asked.take() {
            self.open_graph(id);
        }
    }

    fn open_graph(&mut self, id: GraphId) {
        if self.document.scene().graphs().get(id).is_none() {
            self.note =
                Some(crate::text::tr("That graph isn't in the project any more.").to_string());
            return;
        }
        self.mode = crate::palette::Mode::Material;
        self.material.show_graph(Some(id));
    }
}

/// `graph` with `fill`'s own values of its settings.
pub(crate) fn with_values(registry: &Registry, graph: &Graph, fill: &GraphFill) -> Graph {
    graph.with_settings(
        registry,
        fill.values
            .iter()
            .map(|v| (v.node, v.key.as_str(), v.value)),
    )
}

/// What a graph fill's pixels are made from, hashed: `graph` (with the
/// layer's values), the texture size, the model's shape and UVs
/// (`layout`), the repeats and the realism switch. Never 0 (a layer not
/// made yet).
pub(crate) fn made_key(graph: &Graph, size: u32, layout: u64, repeats: f32, realism: bool) -> u64 {
    let mut h = DefaultHasher::new();
    // The graph's every part, as written out (graphs hold floats, which
    // don't hash; the writing is the same for the same graph).
    format!("{:?}", graph.to_parts()).hash(&mut h);
    (size, layout, repeats.to_bits(), realism).hash(&mut h);
    h.finish().max(1)
}

/// Whether `now` is the same graph fill as `then`, made or not.
fn same_fill(now: Option<&GraphFill>, then: &GraphFill) -> bool {
    now.is_some_and(|n| n.graph == then.graph && n.values == then.values)
}

/// Puts `made` into `layer`: its colour, and with `realism` its
/// roughness, metal and height; marked made from `key`.
fn put(layer: &mut Layer, made: MadeGraph, key: u64, realism: bool) {
    layer.content = Content::Paint(made.colour);
    let part = |image| realism.then_some(ChannelPart::Paint(image));
    layer.channels.roughness = part(made.roughness);
    layer.channels.metal = part(made.metal);
    layer.channels.height = part(made.height);
    if let Some(fill) = &mut layer.graph {
        fill.made = key;
    }
}
