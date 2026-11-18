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

use crate::alpha::{DabAlpha, Laid};
use crate::brush::{self, BrushKind, Dab};
use crate::forms::{self, Plane, Side};
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
    /// Where the stroke's last dab was (for Clay strips' way).
    last_centre: Option<Vec3>,
    /// What a grabbing brush holds, from the stroke's first dab.
    held: Option<Held>,
}

/// What a grabbing brush (Grab, Elastic grab, Snake hook) holds during a
/// stroke.
#[derive(Debug, Clone)]
struct Held {
    /// Where the grab began.
    start: Vec3,
    /// Where it is now.
    anchor: Vec3,
    /// The brush's reach when it began.
    radius: f32,
    /// The points gathered when it began, where they were then (Grab and
    /// Elastic grab; Snake hook gathers afresh each dab).
    points: Vec<Reached>,
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
            last_centre: None,
            held: None,
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
    ///
    /// A grabbing brush ([`BrushKind::grabs`]) takes the lump under its
    /// first dab of a stroke (moving nothing), and each dab after puts
    /// the place it holds at the dab's centre: the interface gives the
    /// place under the mouse, on a plane through [`Self::held`] facing
    /// the view.
    pub fn dab(&mut self, dab: &Dab) -> usize {
        self.dab_with(dab, None)
    }

    /// Applies one dab with an alpha laid on it (decision 0308): each
    /// point moves as [`Self::dab`] would move it, times the alpha under
    /// it. The grabbing brushes take no alpha. Returns how many points it
    /// moved.
    pub fn dab_with(&mut self, dab: &Dab, alpha: Option<&DabAlpha>) -> usize {
        if dab.radius.is_nan()
            || dab.radius <= 0.0
            || !dab.centre.is_finite()
            || dab.strength <= 0.0
        {
            return 0;
        }
        if dab.kind.grabs() {
            return self.grab_dab(dab);
        }
        // A square brush's corners reach past its radius.
        let reach = if dab.kind == BrushKind::ClayStrips {
            dab.radius * std::f32::consts::SQRT_2
        } else {
            dab.radius
        };
        let (reached, faces) = self.reach(dab.centre, reach);
        let last = self.last_centre.replace(dab.centre);
        if reached.is_empty() {
            return 0;
        }
        // The alpha lies on the plane the brush's polygons face.
        let laid = match alpha {
            Some(alpha) => {
                let Some(normal) = self.facing(&faces) else {
                    return 0;
                };
                let way = last.map(|l| dab.centre - l).filter(|w| w.length() > 1e-9);
                Some(Laid::new(alpha, dab.centre, dab.radius, normal, way))
            }
            None => None,
        };
        let new: Vec<Vec3> = match dab.kind {
            BrushKind::Smooth => {
                let mesh = &self.mesh;
                let topology = mesh.topology();
                in_parts(&reached, |r| {
                    let middle = neighbours_middle(mesh, topology, r.point).unwrap_or(r.at);
                    brush::smooth(dab, middle, r.at, r.distance)
                })
            }
            BrushKind::Inflate => {
                let mesh = &self.mesh;
                let topology = mesh.topology();
                in_parts(&reached, |r| match point_facing(mesh, topology, r.point) {
                    Some(n) => forms::inflate(dab, n, r.at, r.distance),
                    None => r.at,
                })
            }
            _ => {
                let Some(facing) = self.facing(&faces) else {
                    return 0;
                };
                match dab.kind {
                    BrushKind::Crease => {
                        in_parts(&reached, |r| forms::crease(dab, facing, r.at, r.distance))
                    }
                    BrushKind::Pinch => {
                        in_parts(&reached, |r| forms::pinch(dab, facing, r.at, r.distance))
                    }
                    BrushKind::ClayStrips | BrushKind::Clay => {
                        let plane = plane_under(dab, &reached, facing);
                        let along =
                            forms::along_plane(&plane, last.map_or(Vec3::ZERO, |l| dab.centre - l));
                        let square = dab.kind == BrushKind::ClayStrips;
                        in_parts(&reached, |r| {
                            let w = if square {
                                let x = forms::square_distance(&plane, along, r.at, dab.radius);
                                forms::weight(dab, x * dab.radius)
                            } else {
                                forms::weight(dab, r.distance)
                            };
                            forms::clay(dab, &plane, r.at, w)
                        })
                    }
                    BrushKind::Flatten | BrushKind::Scrape | BrushKind::Fill => {
                        let plane = plane_under(dab, &reached, facing);
                        let side = match dab.kind {
                            BrushKind::Scrape => Side::Above,
                            BrushKind::Fill => Side::Below,
                            _ => Side::Both,
                        };
                        in_parts(&reached, |r| {
                            forms::flatten(dab, &plane, side, r.at, r.distance)
                        })
                    }
                    _ => in_parts(&reached, |r| brush::draw(dab, facing, r.at, r.distance)),
                }
            }
        };
        let new = match laid {
            // Each move scaled by the alpha under the point.
            Some(laid) => {
                let which: Vec<usize> = (0..reached.len()).collect();
                in_parts(&which, |&i| {
                    let (from, to) = (reached[i].at, new[i]);
                    from + (to - from) * laid.at(from)
                })
            }
            None => new,
        };
        self.place(&reached, new)
    }

    /// A dab of a grabbing brush (see [`Self::dab`]).
    fn grab_dab(&mut self, dab: &Dab) -> usize {
        let Some(held) = &self.held else {
            let reach = match dab.kind {
                BrushKind::ElasticGrab => dab.radius * forms::ELASTIC_REACH,
                BrushKind::SnakeHook => 0.0,
                _ => dab.radius,
            };
            let points = if reach > 0.0 {
                self.reach(dab.centre, reach).0
            } else {
                Vec::new()
            };
            self.held = Some(Held {
                start: dab.centre,
                anchor: dab.centre,
                radius: dab.radius,
                points,
            });
            return 0;
        };
        let (start, anchor) = (held.start, held.anchor);
        let first = Dab {
            radius: held.radius,
            ..*dab
        };
        let total = dab.centre - start;
        let moved = match dab.kind {
            BrushKind::SnakeHook => {
                let motion = dab.centre - anchor;
                let (reached, _) = self.reach(anchor, dab.radius);
                let new = in_parts(&reached, |r| r.at + motion * forms::weight(dab, r.distance));
                self.place(&reached, new)
            }
            kind => {
                let points = self
                    .held
                    .as_mut()
                    .map(|h| std::mem::take(&mut h.points))
                    .unwrap_or_default();
                let new = if kind == BrushKind::ElasticGrab {
                    // Step by step from where the points are now: each
                    // small bend keeps the volume, where one big one
                    // from the start wouldn't.
                    let (mesh, motion) = (&self.mesh, dab.centre - anchor);
                    in_parts(&points, |r| {
                        let now = mesh.position(r.point).unwrap_or(r.at);
                        forms::elastic(&first, anchor, now, now.distance(anchor), motion)
                    })
                } else {
                    in_parts(&points, |r| forms::grab(&first, r.at, r.distance, total))
                };
                let moved = self.place(&points, new);
                if let Some(held) = &mut self.held {
                    held.points = points;
                }
                moved
            }
        };
        if let Some(held) = &mut self.held {
            held.anchor = dab.centre;
        }
        moved
    }

    /// Where a grabbing brush holds the surface now, in the object's own
    /// space, once its stroke's first dab has taken hold.
    pub fn held(&self) -> Option<Vec3> {
        self.held.as_ref().map(|h| h.anchor)
    }

    /// The area-weighted facing of `faces`.
    fn facing(&self, faces: &[FaceId]) -> Option<Vec3> {
        faces
            .iter()
            .filter_map(|f| self.mesh.face(*f))
            .map(|c| newell(&self.mesh, c))
            .sum::<Vec3>()
            .try_normalize()
    }

    /// Moves the points `reached` to `new`, marking them moved. Returns
    /// how many moved.
    fn place(&mut self, reached: &[Reached], new: Vec<Vec3>) -> usize {
        let mut moved: Vec<VertexId> = Vec::with_capacity(reached.len());
        for (r, p) in reached.iter().zip(new) {
            if !p.is_finite() || self.mesh.position(r.point) == Some(p) {
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
        self.last_centre = None;
        self.held = None;
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
        self.last_centre = None;
        self.held = None;
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

/// The facing of point `v`: its polygons' area-weighted facing.
fn point_facing(mesh: &Mesh, topology: &mesh::Topology, v: VertexId) -> Option<Vec3> {
    topology
        .vertex_faces(v)
        .iter()
        .filter_map(|f| mesh.face(*f))
        .map(|c| newell(mesh, c))
        .sum::<Vec3>()
        .try_normalize()
}

/// The plane under a dab: through the points in reach, each counted by
/// the brush's strength there, facing `facing`.
fn plane_under(dab: &Dab, reached: &[Reached], facing: Vec3) -> Plane {
    let (mut sum, mut total) = (Vec3::ZERO, 0.0);
    for r in reached {
        let w = forms::weight(dab, r.distance);
        sum += r.at * w;
        total += w;
    }
    let point = if total > 1e-12 {
        sum / total
    } else {
        reached.iter().map(|r| r.at).sum::<Vec3>() / reached.len().max(1) as f32
    };
    Plane {
        point,
        normal: facing,
    }
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
