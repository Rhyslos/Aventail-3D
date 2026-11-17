//! The live sculpt (decision 0304): what a stroke works on while it runs,
//! on the interface thread. It holds the shape coming into the Sculpt
//! layer, the shape as sculpted now, and the tree of boxes that finds the
//! surface and the points in a brush's reach.
//!
//! Each dab finds its points, works out their new places on every core
//! for big brushes, moves them and refits the boxes round them. When the
//! stroke ends, [`LiveSculpt::finish`] turns the points it moved into
//! moves along the surface, chunk by chunk, giving the new [`Sculpt`] for
//! the stroke's undo step; the shape is set to exactly what evaluating
//! that sculpt gives, so the document and the live sculpt agree to the
//! last bit.

use crate::brush::{self, BrushKind, Dab};
use crate::frames::{frame, newell};
use crate::layer::{Sculpt, sculpted_places};
use crate::moves::CHUNK;
use crate::tree::{FaceTree, RayHit};
use glam::Vec3;
use mesh::{FaceId, Mesh, VertexId};
use std::collections::BTreeSet;
use std::sync::Arc;

/// The live sculpt of one object.
pub struct LiveSculpt {
    input: Arc<Mesh>,
    sculpt: Sculpt,
    /// Whether the sculpt had to be fitted to a changed shape on starting.
    fitted: bool,
    mesh: Mesh,
    tree: FaceTree,
    /// Each point's mark from the last dab that gathered it.
    seen: Vec<u32>,
    dab_mark: u32,
    /// Each point's mark from the last stroke that moved it.
    moved: Vec<u32>,
    stroke_mark: u32,
    /// The chunks the stroke under way moved points in.
    touched: BTreeSet<usize>,
    /// Whether the shape changed since it was last handed out.
    changed: bool,
    /// The shape last handed out.
    shown: Option<Arc<Mesh>>,
}

/// A point in a dab's reach.
#[derive(Debug, Clone, Copy)]
struct Reached {
    point: VertexId,
    at: Vec3,
    distance: f32,
}

impl LiveSculpt {
    /// Starts sculpting on `input` (the shape coming into the Sculpt
    /// layer) with `sculpt` (the layer's sculpting so far, fitted to
    /// `input` if its points changed; nothing for a new layer).
    pub fn new(input: Arc<Mesh>, sculpt: Option<&Sculpt>) -> Self {
        let (sculpt, fitted) = match sculpt {
            Some(s) => s.fitted_to(&input),
            None => (Sculpt::new(&input), false),
        };
        let mut mesh = (*input).clone();
        sculpt.apply(&mut mesh);
        let tree = FaceTree::new(&mesh);
        let slots = mesh.point_slot_count();
        Self {
            input,
            sculpt,
            fitted,
            mesh,
            tree,
            seen: vec![0; slots],
            dab_mark: 0,
            moved: vec![0; slots],
            stroke_mark: 1,
            touched: BTreeSet::new(),
            changed: true,
            shown: None,
        }
    }

    /// The shape coming into the Sculpt layer.
    pub fn input(&self) -> &Arc<Mesh> {
        &self.input
    }

    /// The sculpting as of the last finished stroke, made for the input.
    pub fn sculpt(&self) -> &Sculpt {
        &self.sculpt
    }

    /// Whether the layer's sculpting had to be fitted to a changed shape.
    pub fn was_fitted(&self) -> bool {
        self.fitted
    }

    /// The shape as sculpted now.
    pub fn mesh(&self) -> &Mesh {
        &self.mesh
    }

    /// Whether a stroke has moved points since it started.
    pub fn stroking(&self) -> bool {
        !self.touched.is_empty()
    }

    /// The shape as sculpted now, shared: a new one only when it changed
    /// since last asked (sharing every part that didn't).
    pub fn shape(&mut self) -> Arc<Mesh> {
        match &self.shown {
            Some(shown) if !self.changed => Arc::clone(shown),
            _ => {
                let shape = Arc::new(self.mesh.clone());
                self.shown = Some(Arc::clone(&shape));
                self.changed = false;
                shape
            }
        }
    }

    /// Whether the shape changed since [`Self::shape`] was last asked.
    pub fn changed(&self) -> bool {
        self.changed
    }

    /// Where the line from `origin` along `way` (the object's space) meets
    /// the shape as sculpted now.
    pub fn hit(&self, origin: Vec3, way: Vec3) -> Option<RayHit> {
        self.tree.ray_hit(&self.mesh, origin, way)
    }

    /// The points within `radius` of `centre`, each once, and the
    /// polygons with a corner among them.
    fn reach(&mut self, centre: Vec3, radius: f32) -> (Vec<Reached>, Vec<FaceId>) {
        self.dab_mark = self.dab_mark.wrapping_add(1);
        if self.dab_mark == 0 {
            self.seen.fill(0);
            self.dab_mark = 1;
        }
        let mark = self.dab_mark;
        let mut points = Vec::new();
        let mut faces = Vec::new();
        let (mesh, seen) = (&self.mesh, &mut self.seen);
        self.tree.near(centre, radius, |face| {
            let Some(corners) = mesh.face(face) else {
                return;
            };
            let mut inside = false;
            for v in corners {
                let Some(p) = mesh.position(*v) else {
                    continue;
                };
                let distance = p.distance(centre);
                if distance > radius {
                    continue;
                }
                inside = true;
                let slot = &mut seen[v.get() as usize];
                if *slot != mark {
                    *slot = mark;
                    points.push(Reached {
                        point: *v,
                        at: p,
                        distance,
                    });
                }
            }
            if inside {
                faces.push(face);
            }
        });
        (points, faces)
    }

    /// Applies one dab. Returns how many points it moved.
    pub fn dab(&mut self, dab: &Dab) -> usize {
        if !(dab.radius > 0.0) || !dab.centre.is_finite() || dab.strength <= 0.0 {
            return 0;
        }
        let (reached, faces) = self.reach(dab.centre, dab.radius);
        if reached.is_empty() {
            return 0;
        }
        let new: Vec<Vec3> = match dab.kind {
            BrushKind::Draw => {
                let facing = faces
                    .iter()
                    .filter_map(|f| self.mesh.face(*f))
                    .map(|c| newell(&self.mesh, c))
                    .sum::<Vec3>()
                    .try_normalize();
                let Some(facing) = facing else {
                    return 0;
                };
                in_parts(&reached, |r| brush::draw(dab, facing, r.at, r.distance))
            }
            BrushKind::Smooth => {
                let mesh = &self.mesh;
                let topology = mesh.topology();
                in_parts(&reached, |r| {
                    let middle = neighbours_middle(mesh, topology, r.point).unwrap_or(r.at);
                    brush::smooth(dab, middle, r.at, r.distance)
                })
            }
        };
        let mut moved: Vec<VertexId> = Vec::with_capacity(reached.len());
        for (r, p) in reached.iter().zip(new) {
            if p == r.at || !p.is_finite() {
                continue;
            }
            self.mesh.set_position(r.point, p);
            let i = r.point.get() as usize;
            self.moved[i] = self.stroke_mark;
            self.touched.insert(i / CHUNK);
            moved.push(r.point);
        }
        if !moved.is_empty() {
            self.tree.refit_points(&self.mesh, &moved);
            self.changed = true;
        }
        moved.len()
    }

    /// Ends the stroke: the points it moved become moves along the
    /// surface, and the shape takes exactly the places the new sculpt
    /// gives. Returns the new sculpt, or `None` if nothing moved.
    pub fn finish(&mut self) -> Option<Sculpt> {
        let touched = std::mem::take(&mut self.touched);
        if touched.is_empty() {
            return None;
        }
        let mark = self.stroke_mark;
        self.stroke_mark = self.stroke_mark.wrapping_add(1).max(1);
        if self.stroke_mark == 1 {
            self.moved.fill(0);
        }
        let len = self.sculpt.len();
        let mut moves = self.sculpt.moves().clone();
        let mut changed: Vec<usize> = Vec::new();
        let input = &self.input;
        let topology = input.topology();
        for c in touched {
            let start = c * CHUNK;
            let end = (start + CHUNK).min(len);
            let mut chunk: Vec<Vec3> = (start..end).map(|i| moves.get(i)).collect();
            for i in start..end {
                if self.moved[i] != mark {
                    continue;
                }
                let v = VertexId::from_raw(i as u32);
                let (Some(from), Some(now)) = (input.position(v), self.mesh.position(v)) else {
                    continue;
                };
                chunk[i - start] = frame(input, topology, v).from_object(now - from);
                changed.push(i);
            }
            moves.set_chunk(c, chunk);
        }
        let new = self.sculpt.with_moves(moves)?;
        // Exactly where evaluating the new sculpt puts them.
        let places = sculpted_places(input, &changed, |i| new.moves().get(i));
        for (i, p) in changed.into_iter().zip(places) {
            if let Some(p) = p {
                self.mesh.set_position(VertexId::from_raw(i as u32), p);
            }
        }
        self.sculpt = new.clone();
        self.fitted = false;
        self.changed = true;
        Some(new)
    }

    /// Takes back the stroke under way (Esc): its points go back to where
    /// the sculpt has them.
    pub fn cancel(&mut self) {
        let touched = std::mem::take(&mut self.touched);
        let mark = self.stroke_mark;
        self.stroke_mark = self.stroke_mark.wrapping_add(1).max(1);
        let len = self.sculpt.len();
        let which: Vec<usize> = touched
            .into_iter()
            .flat_map(|c| c * CHUNK..((c + 1) * CHUNK).min(len))
            .filter(|i| self.moved[*i] == mark)
            .collect();
        self.reset(&which);
    }

    /// Brings the live sculpt to `sculpt` (the document's, after an undo
    /// or redo), working out again only the chunks that differ. `sculpt`
    /// must be made for the input ([`Sculpt::fits`]); otherwise it is
    /// fitted to it first.
    pub fn follow(&mut self, sculpt: &Sculpt) {
        self.cancel();
        let (sculpt, fitted) = sculpt.fitted_to(&self.input);
        let len = sculpt.len();
        // Only the points whose moves differ: an undone stroke touched a
        // few of each chunk's points.
        let (new, old) = (sculpt.moves(), self.sculpt.moves());
        let which: Vec<usize> = new
            .differing(old)
            .into_iter()
            .flat_map(|c| c * CHUNK..((c + 1) * CHUNK).min(len))
            .filter(|i| new.get(*i) != old.get(*i))
            .collect();
        self.sculpt = sculpt;
        self.fitted = fitted;
        self.reset(&which);
    }

    /// Puts points `which` where the sculpt has them.
    fn reset(&mut self, which: &[usize]) {
        if which.is_empty() {
            return;
        }
        let sculpt = &self.sculpt;
        let places = sculpted_places(&self.input, which, |i| sculpt.moves().get(i));
        let mut moved = Vec::new();
        for (i, p) in which.iter().zip(places) {
            let Some(p) = p else { continue };
            let v = VertexId::from_raw(*i as u32);
            if self.mesh.position(v) != Some(p) {
                self.mesh.set_position(v, p);
                moved.push(v);
            }
        }
        if !moved.is_empty() {
            self.tree.refit_points(&self.mesh, &moved);
            self.changed = true;
        }
    }

    /// Roughly how many bytes it holds beyond the shapes themselves.
    pub fn memory_size(&self) -> usize {
        self.tree.memory_size() + (self.seen.len() + self.moved.len()) * 4
    }
}

/// The middle of point `v`'s neighbours. On an open edge, only its
/// neighbours along the edge, so the edge keeps its line instead of
/// shrinking inwards.
fn neighbours_middle(mesh: &Mesh, topology: &mesh::Topology, v: VertexId) -> Option<Vec3> {
    let edges = topology.vertex_edges(v);
    // Inside a closed surface a point has as many edges as polygons; only
    // a point with more can be on an open edge, so only then are its
    // edges looked up one by one.
    let maybe_border = edges.len() > topology.vertex_faces(v).len();
    let border: Vec<_> = if maybe_border {
        edges.iter().filter(|e| topology.is_border(**e)).collect()
    } else {
        Vec::new()
    };
    let chosen: Vec<_> = if border.is_empty() {
        edges.iter().collect()
    } else {
        border
    };
    let mut sum = Vec3::ZERO;
    let mut n = 0.0;
    for e in chosen {
        if let Some(p) = mesh.position(mesh::topology::other_end(*e, v)) {
            sum += p;
            n += 1.0;
        }
    }
    (n > 0.0).then(|| sum / n)
}

/// Below this many points per core, a dab's work stays on one core.
const PER_CORE: usize = 16_384;

/// `work` on each of `items`, on every core when there are many.
fn in_parts<T: Sync>(items: &[T], work: impl Fn(&T) -> Vec3 + Sync) -> Vec<Vec3> {
    let cores = std::thread::available_parallelism()
        .map_or(1, std::num::NonZeroUsize::get)
        .min(items.len() / PER_CORE)
        .max(1);
    if cores == 1 {
        return items.iter().map(&work).collect();
    }
    let part = items.len().div_ceil(cores);
    std::thread::scope(|scope| {
        let handles: Vec<_> = items
            .chunks(part)
            .map(|chunk| {
                let work = &work;
                scope.spawn(move || chunk.iter().map(work).collect::<Vec<_>>())
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap_or_else(|e| std::panic::resume_unwind(e)))
            .collect()
    })
}
