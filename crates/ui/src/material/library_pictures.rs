//! The library browser's pictures of graphs (decision 0292): each graph
//! on its card shows its material, worked out small on a thread of its
//! own, one graph at a time, and kept.
//!
//! A card's picture is made again only when its graph changes (the
//! scene gives a changed graph a new `Arc`), never while scrolling, and
//! not while a button is held down: a slider dragged on the canvas with
//! the browser open doesn't queue up work. Until a new picture comes,
//! the card keeps its old one. Starters and your own boxes bring their
//! own pictures and need none of this.

use super::library::ItemId;
use super::pictures::colour_image;
use egui::{ColorImage, Context, TextureHandle, TextureOptions};
use matgraph::{Evaluator, Graph, Registry, Size};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

/// How many pixels across a graph's card picture is.
pub(crate) const CARD_PICTURE: u32 = 96;
/// How soon to look again for a picture being worked out, in seconds.
const LOOK_AGAIN: f32 = 0.05;

/// The size a card's graph is worked out at (small in tests).
fn worked_out() -> Size {
    if cfg!(test) {
        Size::nearest(32)
    } else {
        Size::nearest(128)
    }
}

/// One graph to work out for its card.
struct Job {
    id: ItemId,
    graph: Arc<Graph>,
    registry: Arc<Registry>,
}

/// A card's picture back: `None` when the graph can't run (it shows the
/// graph mark).
struct Done {
    id: ItemId,
    graph: Arc<Graph>,
    image: Option<ColorImage>,
}

impl Job {
    fn run(self, evaluator: &mut Evaluator) -> Done {
        let image = super::preview::shown_box(&self.graph, None, &[]).and_then(|out| {
            let evaluation = evaluator.run(&self.registry, &self.graph, worked_out(), &[]);
            evaluation
                .picture(out)
                .map(|value| colour_image(value, CARD_PICTURE))
        });
        Done {
            id: self.id,
            graph: self.graph,
            image,
        }
    }
}

/// The thread working cards out.
struct Worker {
    jobs: Sender<Job>,
    done: Receiver<Done>,
}

impl Worker {
    fn start() -> Option<Self> {
        let (jobs, take) = channel::<Job>();
        let (give, done) = channel::<Done>();
        std::thread::Builder::new()
            .name("library cards".into())
            .spawn(move || {
                // One core: the canvas's own pictures come first.
                let mut evaluator = Evaluator::with_threads(1);
                while let Ok(job) = take.recv() {
                    if give.send(job.run(&mut evaluator)).is_err() {
                        break;
                    }
                }
            })
            .ok()?;
        Some(Self { jobs, done })
    }
}

/// A card's picture, with the graph it was made from.
struct Made {
    from: Arc<Graph>,
    texture: Option<TextureHandle>,
}

/// The pictures on the graphs' cards.
#[derive(Default)]
pub(crate) struct CardPictures {
    worker: Option<Worker>,
    /// Work out cards here: no thread could be had (tests always do).
    here: bool,
    made: BTreeMap<ItemId, Made>,
    /// The card being worked out, and its graph.
    sent: Option<(ItemId, Arc<Graph>)>,
    /// The cards shown this frame, with what they show.
    wanted: Vec<(ItemId, Arc<Graph>, Arc<Registry>)>,
    /// How many pictures have been worked out (tests).
    #[cfg(test)]
    pub(crate) runs: u64,
}

impl std::fmt::Debug for CardPictures {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CardPictures")
            .field("made", &self.made.len())
            .finish_non_exhaustive()
    }
}

impl CardPictures {
    /// Card `id` shows `graph` this frame; its picture, as made last
    /// (perhaps from the graph before a change), if there is one.
    pub(crate) fn picture(
        &mut self,
        id: &ItemId,
        graph: &Arc<Graph>,
        registry: &Arc<Registry>,
    ) -> Option<egui::TextureId> {
        self.wanted
            .push((id.clone(), graph.clone(), registry.clone()));
        self.made
            .get(id)
            .and_then(|m| m.texture.as_ref())
            .map(TextureHandle::id)
    }

    /// After the cards are drawn: takes in pictures that came back, sends
    /// the first card shown whose graph changed (when nothing is being
    /// worked out and `busy` is false: no button held), and forgets the
    /// pictures of graphs no longer in `existing` (a card hidden by a
    /// search keeps its picture for when it shows again).
    pub(crate) fn update(&mut self, ctx: &Context, busy: bool, existing: &BTreeSet<ItemId>) {
        let wanted = std::mem::take(&mut self.wanted);
        if let Some(worker) = &self.worker {
            let came: Vec<Done> = worker.done.try_iter().collect();
            for done in came {
                self.take(ctx, done);
            }
        }
        self.made.retain(|id, _| existing.contains(id));
        if self.sent.is_none() && !busy {
            let next = wanted.into_iter().find(|(id, graph, _)| {
                self.made
                    .get(id)
                    .is_none_or(|m| !Arc::ptr_eq(&m.from, graph))
            });
            if let Some((id, graph, registry)) = next {
                self.send(
                    ctx,
                    Job {
                        id,
                        graph,
                        registry,
                    },
                );
            }
        }
        if self.sent.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_secs_f32(LOOK_AGAIN));
        }
    }

    fn send(&mut self, ctx: &Context, job: Job) {
        #[cfg(test)]
        {
            self.runs += 1;
        }
        if self.worker.is_none() && !self.here && !cfg!(test) {
            self.worker = Worker::start();
        }
        self.sent = Some((job.id.clone(), job.graph.clone()));
        match &self.worker {
            Some(worker) => {
                if let Err(failed) = worker.jobs.send(job) {
                    // The thread has gone: work it out here.
                    self.worker = None;
                    self.here = true;
                    let done = failed.0.run(&mut Evaluator::with_threads(1));
                    self.take(ctx, done);
                }
            }
            None => {
                let done = job.run(&mut Evaluator::with_threads(1));
                self.take(ctx, done);
            }
        }
    }

    fn take(&mut self, ctx: &Context, done: Done) {
        if self
            .sent
            .as_ref()
            .is_some_and(|(id, g)| *id == done.id && Arc::ptr_eq(g, &done.graph))
        {
            self.sent = None;
        }
        let texture = done.image.map(|image| {
            ctx.load_texture(
                format!("library card {:?}", done.id),
                image,
                TextureOptions::LINEAR,
            )
        });
        self.made.insert(
            done.id,
            Made {
                from: done.graph,
                texture,
            },
        );
    }

    /// The tab is going to sleep (decision 0267): the pictures are let go
    /// of, and the thread ends. Returns about how many bytes they took.
    pub(crate) fn sleep(&mut self) -> u64 {
        let bytes = self
            .made
            .values()
            .filter_map(|m| m.texture.as_ref())
            .map(|t| {
                let [w, h] = t.size();
                (w * h * 4) as u64
            })
            .sum();
        self.made.clear();
        self.sent = None;
        self.worker = None;
        bytes
    }

    /// Whether a card is being worked out (tests).
    #[cfg(test)]
    pub(crate) fn waiting(&self) -> bool {
        self.sent.is_some()
    }
}
