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
use crate::frames::{AT_HAND, at_hand, frame, newell, newell_at};
use crate::layer::{Sculpt, Way, sculpted_places};
use crate::mask::{FULL, Mask};
use crate::moves::CHUNK;
use crate::parts;
use crate::symmetry::{self, NotSymmetric, Repeat, Symmetry};
use crate::tree::{FaceTree, RayHit};
use glam::{Mat4, Vec3};
use mesh::{FaceId, Mesh, VertexId};
use std::collections::BTreeSet;
use std::sync::Arc;

#[cfg(test)]
mod kept_tests;

/// The live sculpt of one object.
pub struct LiveSculpt {
    input: Arc<Mesh>,
    sculpt: Sculpt,
    /// Whether the sculpt had to be fitted to a changed shape on starting.
    fitted: bool,
    mesh: Mesh,
    tree: FaceTree,
    /// Each point's mark from the last stroke that moved it.
    moved: Vec<u32>,
    stroke_mark: u32,
    /// The chunks the stroke under way moved points in.
    touched: BTreeSet<usize>,
    /// Each point the stroke under way moved, and where it was before
    /// (decision 0313): kept with the stroke's undo step, and what taking
    /// the stroke back puts back.
    first: Vec<(u32, Vec3)>,
    /// Whether the shape changed since it was last handed out.
    changed: bool,
    /// The shape last handed out.
    shown: Option<Arc<Mesh>>,
    /// Where the stroke's last dab was (for Clay strips' way), per copy
    /// of the dab (symmetry).
    last_centre: Vec<Option<Vec3>>,
    /// What a grabbing brush holds, from the stroke's first dab, per copy
    /// of the dab.
    held: Vec<Option<Held>>,
    /// The mask as painted now (decision 0310).
    mask: Mask,
    /// The chunks of the mask the stroke under way changed.
    mask_touched: BTreeSet<usize>,
    /// The mask's values last handed out for drawing, and whether the
    /// mask changed since.
    mask_shown: Option<Arc<[u8]>>,
    mask_changed: bool,
    /// Each copy of a dab (symmetry), the dab itself first.
    copies: Vec<Repeat>,
    /// For putting the copies of a dab together: each point's mark (the
    /// high half) and its place in the list being put together (the low
    /// half).
    gathered: Vec<u64>,
    gather_mark: u32,
    /// The points the gizmo moves with a mask on (decision 0310): where
    /// each was when the move began, and how free it is.
    moving: Option<Vec<Reached>>,
    /// Counts every change of the shape or mask, for the unmasked part's
    /// middle kept below.
    version: u64,
    middle: Option<(u64, Option<Vec3>)>,
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
    /// Elastic grab; Snake hook gathers afresh each dab), shared with the
    /// dabs that move them.
    points: Arc<Vec<Reached>>,
    /// The tree's leaves with a corner among them, whose boxes the dabs
    /// refit.
    leaves: Arc<Vec<u32>>,
}

/// One copy of a dab, worked out but not yet placed: the points it moves,
/// their new places, and the tree's leaves with a corner among them
/// (whose boxes are refitted after).
struct Part {
    points: Points,
    new: Vec<Vec3>,
    leaves: Arc<Vec<u32>>,
}

/// The points a dab moves: its own, or those a grabbing brush holds.
enum Points {
    Own(Vec<Reached>),
    Held(Arc<Vec<Reached>>),
}

impl Points {
    /// Whether each point's place is where the point is now: so for the
    /// points a dab just gathered, not for those a grab took hold of
    /// earlier.
    fn fresh(&self) -> bool {
        matches!(self, Self::Own(_))
    }
}

/// What a dab finds in its reach.
struct Found {
    /// The points within reach, each once.
    points: Vec<Reached>,
    /// The tree's leaves holding a polygon with a corner among them.
    leaves: Vec<u32>,
    /// The area-weighted facings of the polygons with a corner among them,
    /// added together leaf by leaf (not of unit length); nothing when it
    /// wasn't asked for.
    facing: Vec3,
}

/// Leaves of the tree to a block of a dab's work: about as many polygons
/// as [`parts::BLOCK`].
const LEAF_BLOCK: usize = 512;

impl std::ops::Deref for Points {
    type Target = [Reached];

    fn deref(&self) -> &[Reached] {
        match self {
            Self::Own(points) => points,
            Self::Held(points) => points,
        }
    }
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
        // Worked out here, on the thread that makes a heavy shape ready,
        // not by the first dab: each dab looks up the polygons round its
        // points.
        let slots = mesh.point_slot_count();
        Self {
            input,
            fitted,
            mesh,
            tree,
            moved: vec![0; slots],
            stroke_mark: 1,
            touched: BTreeSet::new(),
            first: Vec::new(),
            changed: true,
            shown: None,
            last_centre: vec![None],
            held: vec![None],
            mask: sculpt.mask().clone(),
            mask_touched: BTreeSet::new(),
            mask_shown: None,
            mask_changed: true,
            copies: vec![Repeat::SAME],
            gathered: Vec::new(),
            gather_mark: 0,
            moving: None,
            version: 0,
            middle: None,
            sculpt,
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

    /// Whether a stroke has moved points or changed the mask since it
    /// started.
    pub fn stroking(&self) -> bool {
        !self.touched.is_empty() || !self.mask_touched.is_empty()
    }

    /// Sets the symmetry each dab is repeated with (decision 0310). Taken
    /// from the next stroke.
    pub fn set_symmetry(&mut self, symmetry: Symmetry) {
        let copies = symmetry.copies();
        if copies != self.copies && !self.stroking() && self.held.iter().all(Option::is_none) {
            self.last_centre = vec![None; copies.len()];
            self.held = vec![None; copies.len()];
            self.copies = copies;
        }
    }

    /// The copies of a dab, the dab itself first.
    pub fn copies(&self) -> &[Repeat] {
        &self.copies
    }

    /// The mask as painted now.
    pub fn mask(&self) -> &Mask {
        &self.mask
    }

    /// Whether any point is masked.
    pub fn has_mask(&self) -> bool {
        !self.mask.is_zero()
    }

    /// Every point's mask value, for drawing: `None` when nothing is
    /// masked; shared, and new only when the mask changed since last
    /// asked.
    pub fn mask_values(&mut self) -> Option<Arc<[u8]>> {
        if self.mask_changed {
            self.mask_shown = (!self.mask.is_zero()).then(|| self.mask.values().into());
            self.mask_changed = false;
        }
        self.mask_shown.clone()
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

    /// The points within `radius` of `centre`, each once, the tree's
    /// leaves with a corner among them, and the facings of the polygons
    /// with a corner among them added up when `facing`.
    fn reach(&self, centre: Vec3, radius: f32, facing: bool) -> Found {
        let mut near = Vec::new();
        self.tree.near_leaves(centre, radius, |leaf| near.push(leaf));
        // On every core, a block of leaves each (decision 0313). Each
        // point of a polygon is with one leaf only, so it comes once
        // whichever core finds it, in the same order on any number of
        // cores; and every point in reach is with a leaf found here, as
        // its polygons' boxes hold it.
        let (mesh, tree) = (&self.mesh, &self.tree);
        let found = parts::in_blocks_of(near.len(), LEAF_BLOCK, |range| {
            let mut leaves = Vec::with_capacity(range.len());
            let mut points = Vec::new();
            let mut facings = Vec::new();
            for &leaf in &near[range] {
                // A leaf well inside the reach: all its polygons count,
                // and their facings are kept added up. At the rim, each
                // polygon is asked for a corner in reach.
                let sum = if tree.leaf_within(leaf, centre, radius) {
                    Some(if facing {
                        tree.leaf_facing(leaf)
                    } else {
                        Vec3::ZERO
                    })
                } else {
                    facing_in_reach(mesh, tree.leaf_faces(leaf), centre, radius, facing)
                };
                let Some(sum) = sum else {
                    continue;
                };
                leaves.push(leaf);
                if facing {
                    facings.push(sum);
                }
                for &point in tree.leaf_points(leaf) {
                    let point = VertexId::from_raw(point);
                    let Some(at) = mesh.position(point) else {
                        continue;
                    };
                    let distance = at.distance(centre);
                    if distance <= radius {
                        points.push(Reached {
                            point,
                            at,
                            distance,
                        });
                    }
                }
            }
            (leaves, points, facings)
        });
        // Added up in blocks, in order: the same on any number of cores.
        let facing = parts::sum_listed(found.iter().flat_map(|(_, _, n)| n.iter().copied()));
        let mut leaves = Vec::with_capacity(found.iter().map(|(l, ..)| l.len()).sum());
        let mut points = Vec::with_capacity(found.iter().map(|(_, p, _)| p.len()).sum());
        for (l, p, _) in found {
            leaves.extend(l);
            points.extend(p);
        }
        Found {
            points,
            leaves,
            facing,
        }
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
        let copies = self.copies.clone();
        if dab.kind == BrushKind::Mask {
            return copies
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let (dab, alpha) = repeated(dab, alpha, c);
                    self.mask_dab(&dab, alpha.as_ref(), i, c.reflects)
                })
                .sum();
        }
        if copies.len() == 1 {
            return match self.dab_one(dab, alpha, 0, false) {
                Some(part) => {
                    let fresh = part.points.fresh();
                    self.place(&part.points, part.new, Some(&part.leaves), fresh)
                }
                None => 0,
            };
        }
        // Every copy worked out from the shape as it was before the dab,
        // then put together: a symmetric shape stays symmetric.
        let parts: Vec<_> = copies
            .iter()
            .enumerate()
            .filter_map(|(i, c)| {
                let (dab, alpha) = repeated(dab, alpha, c);
                self.dab_one(&dab, alpha.as_ref(), i, c.reflects)
            })
            .collect();
        self.place_together(parts)
    }

    /// Where one copy of a dab (copy `copy` of [`Self::copies`]) moves
    /// the points in its reach, each held back by the mask: the points
    /// and their new places, nothing moved yet. `reflects` turns the
    /// alpha over.
    fn dab_one(
        &mut self,
        dab: &Dab,
        alpha: Option<&DabAlpha>,
        copy: usize,
        reflects: bool,
    ) -> Option<Part> {
        if dab.kind.grabs() {
            return self.grab_dab(dab, copy);
        }
        // A square brush's corners reach past its radius.
        let reach = if dab.kind == BrushKind::ClayStrips {
            dab.radius * std::f32::consts::SQRT_2
        } else {
            dab.radius
        };
        // Smooth and Inflate go by each point's own surroundings; the rest,
        // and an alpha, by the way the polygons in reach face.
        let faced = alpha.is_some() || !matches!(dab.kind, BrushKind::Smooth | BrushKind::Inflate);
        let Found {
            points: reached,
            leaves,
            facing,
        } = self.reach(dab.centre, reach, faced);
        let facing = facing.try_normalize();
        let last = self.last_centre.get_mut(copy)?.replace(dab.centre);
        if reached.is_empty() {
            return None;
        }
        // The alpha lies on the plane the brush's polygons face.
        let laid = match alpha {
            Some(alpha) => {
                let normal = facing?;
                let way = last.map(|l| dab.centre - l).filter(|w| w.length() > 1e-9);
                let laid = Laid::new(alpha, dab.centre, dab.radius, normal, way);
                Some(if reflects { laid.mirrored() } else { laid })
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
                let facing = facing?;
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
        // Each move scaled by the alpha under the point and by how free
        // the mask leaves it.
        let masked = self.has_mask();
        let new = if laid.is_some() || masked {
            let mask = &self.mask;
            parts::each_of(reached.len(), |i| {
                let (r, to) = (&reached[i], new[i]);
                let mut share = laid.as_ref().map_or(1.0, |laid| laid.at(r.at));
                if masked {
                    share *= mask.free(r.point.get() as usize);
                }
                r.at + (to - r.at) * share
            })
        } else {
            new
        };
        Some(Part {
            points: Points::Own(reached),
            new,
            leaves: Arc::new(leaves),
        })
    }

    /// Puts the copies of one dab together: each point moves by the sum
    /// of what each copy would move it. Returns how many points moved.
    fn place_together(&mut self, parts: Vec<Part>) -> usize {
        /// The half of a point's entry that holds its mark.
        const MARK: u64 = !0 << 32;
        /// A point no earlier copy moves.
        const NEW: u32 = u32::MAX;
        let slots = self.mesh.point_slot_count();
        if self.gathered.len() != slots {
            self.gathered = vec![0; slots];
        }
        self.gather_mark = self.gather_mark.wrapping_add(1);
        if self.gather_mark == 0 {
            self.gathered.fill(0);
            self.gather_mark = 1;
        }
        let mark = u64::from(self.gather_mark) << 32;
        let mut reached: Vec<Reached> = Vec::new();
        let mut moves: Vec<Vec3> = Vec::new();
        let mut leaves: Vec<u32> = Vec::new();
        let last = parts.len().saturating_sub(1);
        for (n, part) in parts.into_iter().enumerate() {
            leaves.extend_from_slice(&part.leaves);
            let (points, new) = self.where_now(part);
            if n == 0 {
                // The first copy's points: each once, so none is looked
                // up; only marked, for the copies after it.
                moves = parts::each_of(points.len(), |i| new[i] - points[i].at);
                for (i, r) in points.iter().enumerate() {
                    self.gathered[r.point.get() as usize] = mark | i as u64;
                }
                reached = points;
                continue;
            }
            // Which of its points an earlier copy moves too, looked up on
            // every core; then added to that copy's move, or to the list.
            let gathered = &self.gathered;
            let found = parts::each_of(points.len(), |i| {
                let entry = gathered[points[i].point.get() as usize];
                if entry & MARK == mark {
                    entry as u32
                } else {
                    NEW
                }
            });
            for ((r, to), at) in points.iter().zip(new).zip(found) {
                let by = to - r.at;
                if at != NEW {
                    moves[at as usize] += by;
                    continue;
                }
                if n != last {
                    self.gathered[r.point.get() as usize] = mark | reached.len() as u64;
                }
                reached.push(*r);
                moves.push(by);
            }
        }
        let new = parts::each_of(reached.len(), |i| reached[i].at + moves[i]);
        self.place(&reached, new, Some(&leaves), true)
    }

    /// The points of one copy of a dab, each where it is now, and where
    /// the copy puts them. The points a dab gathered are there already;
    /// those a grab took hold of earlier are looked up.
    fn where_now(&self, part: Part) -> (Vec<Reached>, Vec<Vec3>) {
        let mut new = part.new;
        match part.points {
            Points::Own(mut points) => {
                let count = points.len().min(new.len());
                points.truncate(count);
                new.truncate(count);
                (points, new)
            }
            Points::Held(held) => {
                let mesh = &self.mesh;
                let count = held.len().min(new.len());
                let now = parts::each_of(count, |i| mesh.position(held[i].point));
                let mut points = Vec::with_capacity(count);
                let mut kept = Vec::with_capacity(count);
                for ((r, at), to) in held.iter().zip(now).zip(new) {
                    if let Some(at) = at {
                        points.push(Reached { at, ..*r });
                        kept.push(to);
                    }
                }
                (points, kept)
            }
        }
    }

    /// One copy of a dab of the Mask brush: masks the points in reach by
    /// the brush (unmasks them, inverted), times the alpha. Returns how
    /// many points' values changed.
    fn mask_dab(
        &mut self,
        dab: &Dab,
        alpha: Option<&DabAlpha>,
        copy: usize,
        reflects: bool,
    ) -> usize {
        let Found {
            points: reached,
            facing,
            ..
        } = self.reach(dab.centre, dab.radius, alpha.is_some());
        let last = self
            .last_centre
            .get_mut(copy)
            .and_then(|l| l.replace(dab.centre));
        if reached.is_empty() {
            return 0;
        }
        let laid = match (alpha, facing.try_normalize()) {
            (Some(alpha), Some(normal)) => {
                let way = last.map(|l| dab.centre - l).filter(|w| w.length() > 1e-9);
                let laid = Laid::new(alpha, dab.centre, dab.radius, normal, way);
                Some(if reflects { laid.mirrored() } else { laid })
            }
            _ => None,
        };
        let mut by_chunk: std::collections::BTreeMap<usize, Vec<(usize, f32)>> =
            std::collections::BTreeMap::new();
        for r in &reached {
            let mut w = forms::weight(dab, r.distance) * dab.strength.clamp(0.0, 1.0);
            if let Some(laid) = &laid {
                w *= laid.at(r.at);
            }
            if w > 0.0 {
                let i = r.point.get() as usize;
                by_chunk.entry(i / CHUNK).or_default().push((i, w));
            }
        }
        let mut changed = 0;
        for (c, points) in by_chunk {
            let start = c * CHUNK;
            let end = (start + CHUNK).min(self.mask.len());
            let mut values: Vec<u8> = match &self.mask.chunks()[c] {
                Some(values) => values.to_vec(),
                None => vec![0; end - start],
            };
            let mut any = false;
            for (i, w) in points {
                let v = f32::from(values[i - start]);
                let to = if dab.invert {
                    v - v * w
                } else {
                    v + (f32::from(FULL) - v) * w
                };
                let to = to.round().clamp(0.0, f32::from(FULL)) as u8;
                if to != values[i - start] {
                    values[i - start] = to;
                    changed += 1;
                    any = true;
                }
            }
            if any {
                self.mask.set_chunk(c, values);
                self.mask_touched.insert(c);
            }
        }
        if changed > 0 {
            self.mask_changed = true;
            self.version += 1;
        }
        changed
    }

    /// A dab of a grabbing brush (see [`Self::dab`]), for copy `copy`:
    /// the points it moves and their new places, held back by the mask.
    fn grab_dab(&mut self, dab: &Dab, copy: usize) -> Option<Part> {
        let Some((start, anchor, radius, points, held_leaves)) =
            self.held.get(copy)?.as_ref().map(|h| {
                (
                    h.start,
                    h.anchor,
                    h.radius,
                    Arc::clone(&h.points),
                    Arc::clone(&h.leaves),
                )
            })
        else {
            let reach = match dab.kind {
                BrushKind::ElasticGrab => dab.radius * forms::ELASTIC_REACH,
                BrushKind::SnakeHook => 0.0,
                _ => dab.radius,
            };
            let (points, leaves) = if reach > 0.0 {
                let found = self.reach(dab.centre, reach, false);
                (found.points, found.leaves)
            } else {
                (Vec::new(), Vec::new())
            };
            self.held[copy] = Some(Held {
                start: dab.centre,
                anchor: dab.centre,
                radius: dab.radius,
                points: Arc::new(points),
                leaves: Arc::new(leaves),
            });
            return None;
        };
        let first = Dab { radius, ..*dab };
        let total = dab.centre - start;
        let (reached, new, leaves) = match dab.kind {
            BrushKind::SnakeHook => {
                let motion = dab.centre - anchor;
                let Found {
                    points: reached,
                    leaves,
                    ..
                } = self.reach(anchor, dab.radius, false);
                let new = in_parts(&reached, |r| r.at + motion * forms::weight(dab, r.distance));
                (Points::Own(reached), new, Arc::new(leaves))
            }
            kind => {
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
                (Points::Held(points), new, held_leaves)
            }
        };
        if let Some(Some(held)) = self.held.get_mut(copy) {
            held.anchor = dab.centre;
        }
        // Held back by the mask, from where each point is now (Grab's
        // places are worked out from where it took hold).
        if !self.has_mask() {
            return Some(Part {
                points: reached,
                new,
                leaves,
            });
        }
        let (mesh, mask) = (&self.mesh, &self.mask);
        let grab = dab.kind == BrushKind::Grab;
        let new = parts::each_of(reached.len(), |i| {
            let r = &reached[i];
            let from = if grab {
                r.at
            } else {
                mesh.position(r.point).unwrap_or(r.at)
            };
            from + (new[i] - from) * mask.free(r.point.get() as usize)
        });
        Some(Part {
            points: reached,
            new,
            leaves,
        })
    }

    /// Where a grabbing brush holds the surface now, in the object's own
    /// space, once its stroke's first dab has taken hold.
    pub fn held(&self) -> Option<Vec3> {
        self.held.first()?.as_ref().map(|h| h.anchor)
    }

    /// Moves the points `reached` to `new`, marking them moved, and
    /// refits the boxes of `leaves` (the tree's leaves with a corner among
    /// them; worked out from the points when not given). `fresh` says
    /// each of `reached` holds where its point is now, so none is looked
    /// up. Returns how many moved.
    fn place(
        &mut self,
        reached: &[Reached],
        new: Vec<Vec3>,
        leaves: Option<&[u32]>,
        fresh: bool,
    ) -> usize {
        // Which points move, and which of them for the first time in this
        // stroke (with where they were), on every core.
        let (mesh, marks, mark) = (&self.mesh, &self.moved, self.stroke_mark);
        let blocks = parts::in_blocks(reached.len(), |range| {
            let mut moving = Vec::with_capacity(range.len());
            let mut first = Vec::new();
            for i in range {
                let (r, Some(&p)) = (&reached[i], new.get(i)) else {
                    continue;
                };
                let now = if fresh {
                    r.at
                } else {
                    match mesh.position(r.point) {
                        Some(now) => now,
                        None => continue,
                    }
                };
                if !p.is_finite() || now == p {
                    continue;
                }
                moving.push((r.point, p));
                if marks.get(r.point.get() as usize) != Some(&mark) {
                    first.push((r.point.get(), now));
                }
            }
            (moving, first)
        });
        let mut moving = Vec::with_capacity(blocks.iter().map(|(m, _)| m.len()).sum());
        let mut last_chunk = usize::MAX;
        for (part, first) in blocks {
            moving.extend(part);
            // A chunk is first touched by a point first moved.
            for (i, was) in first {
                let Some(slot) = self.moved.get_mut(i as usize) else {
                    continue;
                };
                if *slot != mark {
                    *slot = mark;
                    self.first.push((i, was));
                    if i as usize / CHUNK != last_chunk {
                        last_chunk = i as usize / CHUNK;
                        self.touched.insert(last_chunk);
                    }
                }
            }
        }
        if moving.is_empty() {
            return 0;
        }
        self.mesh.set_positions(&moving);
        match leaves {
            Some(leaves) => self.tree.refit_near(&self.mesh, leaves),
            None => {
                let points: Vec<VertexId> = moving.iter().map(|(v, _)| *v).collect();
                self.tree.refit_points(&self.mesh, &points);
            }
        }
        self.changed = true;
        self.version += 1;
        moving.len()
    }

    /// Ends the stroke: the points it moved become moves along the
    /// surface, and the shape takes exactly the places the new sculpt
    /// gives. Returns the new sculpt, or `None` if nothing moved.
    pub fn finish(&mut self) -> Option<Sculpt> {
        self.last_centre.fill(None);
        self.held.fill(None);
        self.moving = None;
        let touched = std::mem::take(&mut self.touched);
        let mask_touched = std::mem::take(&mut self.mask_touched);
        if touched.is_empty() {
            if mask_touched.is_empty() {
                return None;
            }
            let new = self.sculpt.with_mask(self.mask.clone())?;
            self.sculpt = new.clone();
            return Some(new);
        }
        self.next_stroke_mark();
        let mut first = std::mem::take(&mut self.first);
        first.sort_unstable_by_key(|(i, _)| *i);
        let len = self.sculpt.len();
        let (input, mesh) = (&self.input, &self.mesh);
        let topology = input.topology();
        // Each moved point's move along its frame, on every core.
        let measured: Vec<Option<Vec3>> = parts::each(&first, |&(i, _)| {
            let v = VertexId::from_raw(i);
            let (from, now) = (input.position(v)?, mesh.position(v)?);
            Some(frame(input, topology, v).from_object(now - from))
        });
        let mut moves = self.sculpt.moves().clone();
        let mut changed: Vec<usize> = Vec::with_capacity(first.len());
        let mut before: Vec<Vec3> = Vec::with_capacity(first.len());
        let mut at = 0;
        for c in touched {
            let start = c * CHUNK;
            let end = (start + CHUNK).min(len);
            let mut chunk: Vec<Vec3> = (start..end).map(|i| moves.get(i)).collect();
            while at < first.len() && (first[at].0 as usize) < end {
                let (i, was) = (first[at].0 as usize, first[at].1);
                if i >= start
                    && let Some(m) = measured[at]
                {
                    chunk[i - start] = m;
                    changed.push(i);
                    before.push(was);
                }
                at += 1;
            }
            moves.set_chunk(c, chunk);
        }
        let mut new = self.sculpt.with_moves(moves)?;
        if !mask_touched.is_empty() {
            new = new.with_mask(self.mask.clone())?;
        }
        // Exactly where evaluating the new sculpt puts them.
        let places = sculpted_places(input, &changed, |i| new.moves().get(i));
        let mut after = Vec::with_capacity(changed.len());
        let mut exact = Vec::with_capacity(changed.len());
        for (&i, p) in changed.iter().zip(places) {
            let v = VertexId::from_raw(i as u32);
            let p = p.or_else(|| self.mesh.position(v)).unwrap_or(Vec3::NAN);
            exact.push((v, p));
            after.push(p);
        }
        self.mesh.set_positions(&exact);
        // The boxes and kept facings follow, to the last bit.
        let placed: Vec<VertexId> = exact.iter().map(|(v, _)| *v).collect();
        self.tree.refit_points(&self.mesh, &placed);
        let points = changed.into_iter().map(|i| i as u32).collect();
        let new = new.keeping(&self.sculpt, &self.input, points, before, after);
        self.sculpt = new.clone();
        self.fitted = false;
        self.changed = true;
        Some(new)
    }

    /// Starts the marks of the next stroke.
    fn next_stroke_mark(&mut self) {
        self.stroke_mark = self.stroke_mark.wrapping_add(1).max(1);
        if self.stroke_mark == 1 {
            self.moved.fill(0);
        }
    }

    /// Takes back the stroke under way (Esc): its points go back to where
    /// the sculpt has them.
    pub fn cancel(&mut self) {
        self.last_centre.fill(None);
        self.held.fill(None);
        self.moving = None;
        let mask_touched = std::mem::take(&mut self.mask_touched);
        if !mask_touched.is_empty() {
            for c in mask_touched {
                match &self.sculpt.mask().chunks()[c] {
                    Some(values) => self.mask.set_chunk(c, values.to_vec()),
                    None => {
                        let len = crate::moves::chunk_len(self.mask.len(), c);
                        self.mask.set_chunk(c, vec![0; len]);
                    }
                }
            }
            self.mask_changed = true;
            self.version += 1;
        }
        self.touched.clear();
        self.next_stroke_mark();
        // Each point back where it was before the stroke: exactly where
        // the sculpt has it.
        let first = std::mem::take(&mut self.first);
        if first.is_empty() {
            return;
        }
        let back: Vec<(VertexId, Vec3)> = first
            .iter()
            .map(|(i, p)| (VertexId::from_raw(*i), *p))
            .collect();
        self.put(&back);
    }

    /// Puts points where `places` says (each a point and its place),
    /// refitting the boxes round them.
    fn put(&mut self, places: &[(VertexId, Vec3)]) {
        let mesh = &self.mesh;
        let moving: Vec<(VertexId, Vec3)> = places
            .iter()
            .filter(|(v, p)| p.is_finite() && mesh.position(*v).is_some_and(|now| now != *p))
            .copied()
            .collect();
        if moving.is_empty() {
            return;
        }
        self.mesh.set_positions(&moving);
        let points: Vec<VertexId> = moving.iter().map(|(v, _)| *v).collect();
        self.tree.refit_points(&self.mesh, &points);
        self.changed = true;
        self.version += 1;
    }

    /// Brings the live sculpt to `sculpt` (the document's, after an undo
    /// or redo), working out again only the chunks that differ. `sculpt`
    /// must be made for the input ([`Sculpt::fits`]); otherwise it is
    /// fitted to it first.
    pub fn follow(&mut self, sculpt: &Sculpt) {
        self.cancel();
        // An undo or redo of a stroke made here: its places were kept
        // with it (decision 0313), so nothing is worked out again.
        if let Some((kept, way)) = self.sculpt.kept_way_to(sculpt, &self.input) {
            let places = match way {
                Way::Back => &kept.before,
                Way::Again => &kept.after,
            };
            let back: Vec<(VertexId, Vec3)> = kept
                .points
                .iter()
                .zip(places)
                .map(|(i, p)| (VertexId::from_raw(*i), *p))
                .collect();
            self.put(&back);
            self.follow_mask(sculpt);
            self.sculpt = sculpt.clone();
            self.fitted = false;
            return;
        }
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
        self.follow_mask(&sculpt);
        self.sculpt = sculpt;
        self.fitted = fitted;
        self.reset(&which);
    }

    /// Takes `sculpt`'s mask, if it differs.
    fn follow_mask(&mut self, sculpt: &Sculpt) {
        if sculpt.mask() != &self.mask {
            self.mask = sculpt.mask().clone();
            self.mask_changed = true;
            self.version += 1;
        }
    }

    /// Puts points `which` where the sculpt has them.
    fn reset(&mut self, which: &[usize]) {
        if which.is_empty() {
            return;
        }
        let sculpt = &self.sculpt;
        let places = sculpted_places(&self.input, which, |i| sculpt.moves().get(i));
        let places: Vec<(VertexId, Vec3)> = which
            .iter()
            .zip(places)
            .filter_map(|(i, p)| Some((VertexId::from_raw(*i as u32), p?)))
            .collect();
        self.put(&places);
    }

    /// Changes the whole mask at once (invert, clear, grow, a mask from
    /// cavity …): the stroke under way, if any, is taken back first.
    /// Returns the new sculpt for the undo step, or `None` when the mask
    /// didn't change.
    pub fn set_mask(&mut self, mask: Mask) -> Option<Sculpt> {
        self.cancel();
        if mask == self.mask || mask.len() != self.mask.len() {
            return None;
        }
        let new = self.sculpt.with_mask(mask.clone())?;
        self.mask = mask;
        self.mask_changed = true;
        self.version += 1;
        self.sculpt = new.clone();
        Some(new)
    }

    /// The middle of the unmasked part, each point counted by how free
    /// it is: where the gizmo goes with a mask on. `None` when everything
    /// is masked.
    pub fn unmasked_middle(&mut self) -> Option<Vec3> {
        if let Some((version, middle)) = self.middle
            && version == self.version
        {
            return middle;
        }
        let (mesh, mask) = (&self.mesh, &self.mask);
        let parts = mesh::parallel::in_parallel(mesh.point_slot_count(), |range| {
            let (mut sum, mut total) = (glam::DVec3::ZERO, 0.0f64);
            for i in range {
                let Some(p) = mesh.position(VertexId::from_raw(i as u32)) else {
                    continue;
                };
                let w = f64::from(mask.free(i));
                sum += p.as_dvec3() * w;
                total += w;
            }
            (sum, total)
        });
        let (sum, total) = parts
            .into_iter()
            .fold((glam::DVec3::ZERO, 0.0), |(s, t), (a, b)| (s + a, t + b));
        let middle = (total > 1e-9).then(|| (sum / total).as_vec3());
        self.middle = Some((self.version, middle));
        middle
    }

    /// Starts moving the unmasked part with the gizmo: every point not
    /// fully masked, where it is now and how free it is.
    pub fn begin_move(&mut self) {
        self.cancel();
        let (mesh, mask) = (&self.mesh, &self.mask);
        let points = mesh::parallel::in_parallel(mesh.point_slot_count(), |range| {
            range
                .filter_map(|i| {
                    let point = VertexId::from_raw(i as u32);
                    let at = mesh.position(point)?;
                    let free = mask.free(i);
                    (free > 0.0).then_some(Reached {
                        point,
                        at,
                        distance: free,
                    })
                })
                .collect::<Vec<_>>()
        });
        self.moving = Some(points.concat());
    }

    /// Moves the unmasked part by `change` (in the shape's own space) from
    /// where it was when the move began, partly masked points partly.
    /// Returns how many points moved.
    pub fn move_unmasked(&mut self, change: Mat4) -> usize {
        let Some(points) = self.moving.take() else {
            return 0;
        };
        if !change.is_finite() {
            self.moving = Some(points);
            return 0;
        }
        let new = in_parts(&points, |r| {
            r.at + (change.transform_point3(r.at) - r.at) * r.distance
        });
        let moved = self.place(&points, new, None, false);
        self.moving = Some(points);
        moved
    }

    /// Copies one side's sculpting and mask onto the other across the
    /// plane square to `axis` (`positive`: from the + side to the −
    /// side), for a sculpt that drifted. Every point on the side written
    /// to needs a point at its reflected place on the shape under the
    /// sculpt. Returns the new sculpt for the undo step (`None` when
    /// nothing changed), or why it couldn't.
    pub fn make_symmetrical(
        &mut self,
        axis: usize,
        positive: bool,
    ) -> Result<Option<Sculpt>, NotSymmetric> {
        self.cancel();
        let places: Vec<Vec3> = self
            .input
            .point_slots()
            .map(|p| p.unwrap_or(Vec3::NAN))
            .collect();
        let real = places.iter().filter(|p| p.is_finite());
        let low = real.clone().fold(Vec3::INFINITY, |a, p| a.min(*p));
        let high = real.fold(Vec3::NEG_INFINITY, |a, p| a.max(*p));
        let size = if low.is_finite() {
            (high - low).length()
        } else {
            1.0
        };
        let pairs = symmetry::mirror_pairs(&places, axis, positive, size.max(1e-6) * 1e-4)?;
        let mut flip = Vec3::ONE;
        flip[axis.min(2)] = -1.0;
        let mut reached = Vec::with_capacity(pairs.len());
        let mut new = Vec::with_capacity(pairs.len());
        let mut values = self.mask.values();
        for (i, j) in pairs {
            let point = VertexId::from_raw(i as u32);
            let (Some(at), Some(from)) = (
                self.mesh.position(point),
                self.mesh.position(VertexId::from_raw(j as u32)),
            ) else {
                continue;
            };
            values[i] = values[j];
            // Already in place (but for rounding): left alone, so a
            // symmetrical sculpt isn't changed again.
            let to = from * flip;
            if to.distance(at) <= size * 1e-6 {
                continue;
            }
            reached.push(Reached {
                point,
                at,
                distance: 0.0,
            });
            new.push(to);
        }
        self.place(&reached, new, None, false);
        let mask = Mask::from_values(&values);
        if mask != self.mask {
            self.mask = mask;
            self.mask_changed = true;
            self.version += 1;
            for c in 0..self.mask.chunks().len() {
                self.mask_touched.insert(c);
            }
        }
        Ok(self.finish())
    }

    /// Roughly how many bytes it holds beyond the shapes themselves.
    pub fn memory_size(&self) -> usize {
        self.tree.memory_size()
            + self.moved.len() * 4
            + self.gathered.len() * 8
    }
}

/// The copy `repeat` of a dab and its alpha: its middle carried there,
/// the alpha's top too, and its turn the other way when it reflects.
fn repeated<'a>(
    dab: &Dab,
    alpha: Option<&DabAlpha<'a>>,
    repeat: &Repeat,
) -> (Dab, Option<DabAlpha<'a>>) {
    let dab = Dab {
        centre: repeat.turn * dab.centre,
        ..*dab
    };
    let alpha = alpha.map(|a| DabAlpha {
        up: repeat.turn * a.up,
        angle: if repeat.reflects { -a.angle } else { a.angle },
        ..*a
    });
    (dab, alpha)
}

/// The area-weighted facings, added together in order, of those of the
/// polygons `faces` (their slots) with a corner within `radius` of
/// `centre`: `None` when none has one. Nothing is added up unless
/// `facing`.
fn facing_in_reach(
    mesh: &Mesh,
    faces: &[u32],
    centre: Vec3,
    radius: f32,
    facing: bool,
) -> Option<Vec3> {
    let mut sum = None;
    for &face in faces {
        let Some(corners) = mesh.face(FaceId::from_raw(face)) else {
            continue;
        };
        // The corners' places, looked up once each.
        let mut places = [Vec3::ZERO; AT_HAND];
        let whole = at_hand(mesh, corners, &mut places);
        let reached = if whole {
            places[..corners.len()]
                .iter()
                .any(|p| p.distance(centre) <= radius)
        } else {
            corners
                .iter()
                .filter_map(|v| mesh.position(*v))
                .any(|p| p.distance(centre) <= radius)
        };
        if !reached {
            continue;
        }
        if !facing {
            return Some(Vec3::ZERO);
        }
        let own = if whole {
            newell_at(&places[..corners.len()])
        } else {
            newell(mesh, corners)
        };
        sum = Some(sum.unwrap_or(Vec3::ZERO) + own);
    }
    sum
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
    let blocks = parts::in_blocks(reached.len(), |range| {
        let (mut sum, mut total) = (Vec3::ZERO, 0.0);
        for r in &reached[range] {
            let w = forms::weight(dab, r.distance);
            sum += r.at * w;
            total += w;
        }
        (sum, total)
    });
    for (s, t) in blocks {
        sum += s;
        total += t;
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

/// `work` on each of `items`, on every core when there are many.
fn in_parts<T: Sync>(items: &[T], work: impl Fn(&T) -> Vec3 + Sync) -> Vec<Vec3> {
    parts::each(items, work)
}
