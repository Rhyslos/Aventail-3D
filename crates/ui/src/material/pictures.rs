//! The live pictures on the boxes: the graph worked out on another
//! thread, small while something is being dragged and fuller after, and
//! each box's picture made into a texture the canvas draws (decision
//! 0258).
//!
//! The interface thread never waits: it sends the graph as it is now and
//! draws the last pictures that came back. Only the newest graph sent is
//! worked out (older ones still waiting are skipped), so a fast drag
//! never queues up work.

use egui::{ColorImage, Context, TextureHandle, TextureOptions};
use matgraph::{BoxId, Evaluation, Evaluator, Graph, Registry, Size, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

/// The size a graph is worked out at when nothing is being dragged: the
/// preview's picture.
pub(crate) fn full() -> Size {
    Size::nearest(512)
}
/// The most pixels across a box's own picture is drawn from.
const THUMBNAIL: u32 = 128;
/// How soon to look again for pictures still being worked out, in
/// seconds.
const LOOK_AGAIN: f32 = 0.03;

/// One graph to work out.
struct Job {
    number: u64,
    what: PictureJob,
    size: Size,
}

/// What to work out: the graph shown, with the kinds of box it may use,
/// and inside an own box, the graph it is placed in and the path to it
/// (so the boxes inside get what is plugged into it).
#[derive(Debug, Clone)]
pub(crate) struct PictureJob {
    pub(crate) graph: Arc<Graph>,
    pub(crate) registry: Arc<Registry>,
    pub(crate) top: Option<(Arc<Graph>, Vec<BoxId>)>,
    /// The moment animated boxes are worked out at (the preview's
    /// player, decision 0277); `None`: the graph's own moment.
    pub(crate) time: Option<f32>,
}

impl PictureJob {
    /// The graph alone, with the standard boxes (tests).
    #[cfg(test)]
    pub(crate) fn plain(graph: Arc<Graph>) -> Self {
        Self {
            graph,
            registry: Arc::new(Registry::standard().clone()),
            top: None,
            time: None,
        }
    }

    /// Whether it is the same work as `other`.
    fn same(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.graph, &other.graph)
            && Arc::ptr_eq(&self.registry, &other.registry)
            && self.time.map(f32::to_bits) == other.time.map(f32::to_bits)
            && match (&self.top, &other.top) {
                (None, None) => true,
                (Some((a, p)), Some((b, q))) => Arc::ptr_eq(a, b) && p == q,
                _ => false,
            }
    }

    /// Works it out with `evaluator`.
    fn run(&self, evaluator: &mut Evaluator, size: Size) -> Evaluation {
        evaluator.set_time(self.time);
        if let Some((top, path)) = &self.top
            && let Ok((_, evaluation)) = evaluator.run_inside(&self.registry, top, path, size)
        {
            return evaluation;
        }
        evaluator.run(&self.registry, &self.graph, size, &[])
    }
}

/// One graph worked out.
pub(crate) struct Done {
    number: u64,
    /// Every box's pictures, or why it couldn't run.
    pub(crate) evaluation: Evaluation,
}

/// The thread working graphs out.
struct Worker {
    jobs: Sender<Job>,
    done: Receiver<Done>,
}

impl Worker {
    /// Starts the thread, or `None` if the system won't give one (graphs
    /// are then worked out here, as the tests do).
    fn start() -> Option<Self> {
        let (jobs, take) = channel::<Job>();
        let (give, done) = channel::<Done>();
        std::thread::Builder::new()
            .name("material graphs".into())
            .spawn(move || {
                let mut evaluator = Evaluator::new();
                while let Ok(mut job) = take.recv() {
                    // Only the newest graph matters.
                    while let Ok(newer) = take.try_recv() {
                        job = newer;
                    }
                    let evaluation = job.what.run(&mut evaluator, job.size);
                    let done = Done {
                        number: job.number,
                        evaluation,
                    };
                    if give.send(done).is_err() {
                        break;
                    }
                }
            })
            .ok()?;
        Some(Self { jobs, done })
    }
}

/// Where graphs are worked out.
enum Runner {
    /// Not needed yet.
    Idle,
    /// On another thread.
    Thread(Worker),
    /// Here, on the interface thread (no thread could be had, or a test).
    Here(Box<Evaluator>),
}

/// The live pictures of the graph shown.
pub(crate) struct Pictures {
    runner: Runner,
    /// What was sent last, and its size.
    sent: Option<(PictureJob, Size)>,
    sent_number: u64,
    /// The newest pictures back.
    shown: Option<Done>,
    /// Each box's texture, by box, with the picture it was made from.
    textures: HashMap<BoxId, (usize, TextureHandle)>,
    /// Work out graphs here rather than on a thread (tests).
    here: bool,
    /// The sizes worked out at while dragging and after.
    sizes: (Size, Size),
}

impl Default for Pictures {
    fn default() -> Self {
        Self {
            runner: Runner::Idle,
            sent: None,
            sent_number: 0,
            shown: None,
            textures: HashMap::new(),
            here: cfg!(test),
            // Tests work small: what matters there is that pictures come.
            sizes: if cfg!(test) {
                (Size::nearest(32), Size::nearest(64))
            } else {
                (Size::PREVIEW, full())
            },
        }
    }
}

impl std::fmt::Debug for Pictures {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pictures")
            .field("sent_number", &self.sent_number)
            .field("textures", &self.textures.len())
            .finish_non_exhaustive()
    }
}

impl Pictures {
    /// Sends `graph` to be worked out if it changed (small while
    /// `dragging`), and takes in what came back. `None` forgets the
    /// pictures (no graph shown).
    pub(crate) fn update(&mut self, ctx: &Context, job: Option<PictureJob>, dragging: bool) {
        let Some(what) = job else {
            self.sent = None;
            self.shown = None;
            self.textures.clear();
            return;
        };
        let size = if dragging { self.sizes.0 } else { self.sizes.1 };
        let same = self
            .sent
            .as_ref()
            .is_some_and(|(w, s)| w.same(&what) && *s == size);
        if !same {
            self.sent_number += 1;
            self.sent = Some((what.clone(), size));
            self.send(Job {
                number: self.sent_number,
                what,
                size,
            });
        }
        if let Runner::Thread(worker) = &self.runner {
            let came: Vec<Done> = worker.done.try_iter().collect();
            for done in came {
                self.take(done);
            }
            if self.waiting() {
                ctx.request_repaint_after(std::time::Duration::from_secs_f32(LOOK_AGAIN));
            }
        }
    }

    /// The tab is going to sleep (decision 0267): the thread working
    /// graphs out stops (it ends once its job is done, as nothing more
    /// can be sent), and the pictures and their textures are let go of.
    /// The next [`update`](Self::update) starts again. Returns about how
    /// many bytes the textures took.
    pub(crate) fn sleep(&mut self) -> u64 {
        let bytes = self
            .textures
            .values()
            .map(|(_, t)| {
                let [w, h] = t.size();
                (w * h * 4) as u64
            })
            .sum();
        self.runner = Runner::Idle;
        self.sent = None;
        self.shown = None;
        self.textures.clear();
        bytes
    }

    /// Whether a thread is working graphs out (for tests).
    #[cfg(test)]
    pub(crate) fn running(&self) -> bool {
        !matches!(self.runner, Runner::Idle)
    }

    /// Whether pictures are still being worked out for the graph sent.
    pub(crate) fn waiting(&self) -> bool {
        self.shown
            .as_ref()
            .is_none_or(|d| d.number != self.sent_number)
    }

    fn send(&mut self, job: Job) {
        if matches!(self.runner, Runner::Idle) {
            self.runner = match (!self.here).then(Worker::start).flatten() {
                Some(worker) => Runner::Thread(worker),
                None => Runner::Here(Box::new(Evaluator::new())),
            };
        }
        match &mut self.runner {
            Runner::Thread(worker) => {
                if let Err(failed) = worker.jobs.send(job) {
                    // The thread has gone: carry on here.
                    let mut evaluator = Box::new(Evaluator::new());
                    let job = failed.0;
                    let evaluation = job.what.run(&mut evaluator, job.size);
                    self.runner = Runner::Here(evaluator);
                    self.take(Done {
                        number: job.number,
                        evaluation,
                    });
                }
            }
            Runner::Here(evaluator) => {
                let evaluation = job.what.run(evaluator, job.size);
                self.take(Done {
                    number: job.number,
                    evaluation,
                });
            }
            Runner::Idle => {}
        }
    }

    fn take(&mut self, done: Done) {
        if self.shown.as_ref().is_none_or(|d| d.number < done.number) {
            self.shown = Some(done);
        }
    }

    /// The newest pictures back, if any.
    pub(crate) fn shown(&self) -> Option<&Done> {
        self.shown.as_ref()
    }

    /// What box `id`'s picture shows now, if it ran.
    pub(crate) fn value(&self, id: BoxId) -> Option<&Value> {
        self.shown.as_ref()?.evaluation.picture(id)
    }

    /// Box `id`'s picture as a texture, made when its picture changed.
    pub(crate) fn texture(&mut self, ctx: &Context, id: BoxId) -> Option<egui::TextureId> {
        let value = self.value(id)?.clone();
        let key = picture_key(&value);
        if let Some((k, handle)) = self.textures.get(&id)
            && *k == key
        {
            return Some(handle.id());
        }
        let image = colour_image(&value, THUMBNAIL);
        let handle = ctx.load_texture(
            format!("material box {}", id.0),
            image,
            TextureOptions::LINEAR,
        );
        let texture = handle.id();
        self.textures.insert(id, (key, handle));
        Some(texture)
    }

    /// Forgets the textures of boxes no longer in `graph`.
    pub(crate) fn keep_only(&mut self, graph: &Graph) {
        self.textures.retain(|id, _| graph.get(*id).is_some());
    }

    /// Waits for the pictures being worked out (tests and reference
    /// pictures), at most `seconds`.
    #[cfg(test)]
    pub(crate) fn settle(&mut self, seconds: f32) {
        let until = std::time::Instant::now() + std::time::Duration::from_secs_f32(seconds);
        while self.waiting() && std::time::Instant::now() < until {
            if let Runner::Thread(worker) = &self.runner
                && let Ok(done) = worker
                    .done
                    .recv_timeout(std::time::Duration::from_millis(20))
            {
                self.take(done);
            } else if !matches!(self.runner, Runner::Thread(_)) {
                break;
            }
        }
    }

    /// Works graphs out on a thread, as the app does (a test).
    #[cfg(test)]
    pub(crate) fn on_a_thread() -> Self {
        Self {
            here: false,
            sizes: (Size::PREVIEW, full()),
            ..Self::default()
        }
    }
}

/// Names the picture a value shows, so a texture is made again only when
/// it changes (the evaluator shares unchanged pictures).
pub(crate) fn picture_key(value: &Value) -> usize {
    match value {
        Value::Shape(g) | Value::Height(g) => Arc::as_ptr(g) as usize,
        Value::Colour(c) => Arc::as_ptr(c) as usize,
        Value::Material(m) => Arc::as_ptr(m) as usize,
    }
}

/// The colour a value shows at pixel `(x, y)` of its own size: greys,
/// colours, or a material's colour lit by its normal from the top left.
pub(crate) fn colour_at(value: &Value, x: i64, y: i64) -> [f32; 3] {
    match value {
        Value::Shape(g) | Value::Height(g) => {
            let v = g.at(x, y);
            [v, v, v]
        }
        Value::Colour(c) => c.at(x, y),
        Value::Material(m) => {
            let n = glam::Vec3::from(m.normal.at(x, y));
            let light = glam::Vec3::new(-0.45, 0.55, 0.7).normalize();
            let lit = 0.45 + 0.55 * n.dot(light).max(0.0);
            let shade = lit * (0.55 + 0.45 * m.occlusion.at(x, y));
            m.colour.at(x, y).map(|c| c * shade)
        }
    }
}

/// `value` as an egui picture at most `most` pixels across (every few
/// pixels taken, for a small one).
pub(crate) fn colour_image(value: &Value, most: u32) -> ColorImage {
    let size = value.size().max(1);
    let side = size.min(most).max(1);
    let step = i64::from((size / side).max(1));
    let mut pixels = Vec::with_capacity((side * side) as usize);
    for y in 0..i64::from(side) {
        for x in 0..i64::from(side) {
            let [r, g, b] = colour_at(value, x * step, y * step);
            // A see-through material (an alpha card, decision 0276) shows
            // what is behind it where it is cut away.
            let alpha = value
                .material()
                .map_or(1.0, |m| m.opacity_at(x * step, y * step));
            pixels.push(egui::Color32::from_rgba_unmultiplied(
                byte(r),
                byte(g),
                byte(b),
                byte(alpha),
            ));
        }
    }
    ColorImage::new([side as usize, side as usize], pixels)
}

/// A colour channel from 0 to 1 as a byte.
pub(crate) fn byte(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec2;
    use matgraph::GraphEdit;

    fn a_graph() -> Arc<Graph> {
        let mut g = Graph::new("Stone");
        g.apply(
            Registry::standard(),
            &GraphEdit::AddBox {
                kind: "noise".into(),
                at: Vec2::ZERO,
            },
        )
        .unwrap();
        Arc::new(g)
    }

    #[test]
    fn pictures_come_back_from_the_thread() {
        let ctx = Context::default();
        let mut pictures = Pictures::on_a_thread();
        let graph = a_graph();
        pictures.update(&ctx, Some(PictureJob::plain(graph.clone())), true);
        pictures.settle(30.0);
        assert!(!pictures.waiting());
        let value = pictures.value(BoxId(1)).expect("Noise's picture");
        assert_eq!(value.size(), Size::PREVIEW.get(), "small while dragging");
        // Let go: the fuller size follows.
        pictures.update(&ctx, Some(PictureJob::plain(graph.clone())), false);
        pictures.settle(30.0);
        assert_eq!(pictures.value(BoxId(1)).unwrap().size(), full().get());
    }

    #[test]
    fn a_texture_is_made_again_only_when_the_picture_changes() {
        let ctx = Context::default();
        let mut pictures = Pictures::default();
        let graph = a_graph();
        pictures.update(&ctx, Some(PictureJob::plain(graph.clone())), false);
        let first = pictures.texture(&ctx, BoxId(1)).unwrap();
        assert_eq!(pictures.texture(&ctx, BoxId(1)), Some(first));
        assert!(pictures.texture(&ctx, BoxId(99)).is_none());
        // No graph: everything forgotten.
        pictures.update(&ctx, None, false);
        assert!(pictures.value(BoxId(1)).is_none());
    }

    #[test]
    fn pictures_are_drawn_small() {
        let value = Value::flat_grey(matgraph::ValueKind::Height, full(), 0.5).unwrap();
        let image = colour_image(&value, THUMBNAIL);
        assert_eq!(image.size, [THUMBNAIL as usize; 2]);
        assert_eq!(image.pixels[0], egui::Color32::from_gray(128));
        assert_eq!(byte(2.0), 255);
        assert_eq!(byte(f32::NAN), 0);
    }
}
