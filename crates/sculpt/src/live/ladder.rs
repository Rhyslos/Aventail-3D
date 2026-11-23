//! The live sculpt's detail levels ([decision 0315](../../../../docs/decisions/0315-detail-levels.md)):
//! each level's shape kept as far as it was worked out, so moving between
//! levels builds no polygons again.
//!
//! A [`Rung`] holds one level: the shape under its moves, how that shape's
//! points hang on the level below ([`Refinement`]), and the level as
//! sculpted when it was last left. A stroke on a level makes every finer
//! rung stale: its polygons stand, and its points are worked out again
//! from the level below when it is next gone to, with the very sums that
//! made them, so the live shape is always exactly what working the sculpt
//! out from nothing gives.

use super::LiveSculpt;
use crate::layer::Sculpt;
use crate::levels::{self, LevelProblem, MAX_LEVELS, MAX_POLYGONS};
use crate::mask::Mask;
use crate::tree::FaceTree;
use glam::Vec3;
use mesh::Mesh;
use mesh::subdivide::Refinement;
use std::sync::Arc;

/// One detail level's shape, kept.
pub(super) struct Rung {
    /// How its points hang on the level below, as sculpted (none for the
    /// shape coming in).
    refinement: Option<Arc<Refinement>>,
    /// The shape under the level's moves.
    under: Arc<Mesh>,
    /// A level below changed since `under`'s points were worked out.
    stale: bool,
    /// The level as sculpted, when it was last left and nothing at or
    /// below it changed since.
    shape: Option<Arc<Mesh>>,
}

impl Rung {
    /// The rung of the shape coming into the layer.
    pub(super) fn base(input: Arc<Mesh>) -> Self {
        Self {
            refinement: None,
            under: input,
            stale: false,
            shape: None,
        }
    }

    /// Roughly how many bytes it holds beyond the shapes themselves.
    pub(super) fn memory_size(&self) -> usize {
        self.refinement.as_ref().map_or(0, |r| r.memory_size())
    }
}

impl LiveSculpt {
    /// The detail level sculpted on (0 is the shape coming in).
    pub fn level(&self) -> usize {
        self.level
    }

    /// The finest detail level there is.
    pub fn top_level(&self) -> usize {
        self.sculpt.top()
    }

    /// How many polygons one more detail level would have.
    pub fn polygons_if_added(&self) -> u64 {
        let top = self.sculpt.top();
        match self.ladder.get(top) {
            Some(rung) => levels::polygons_above(&rung.under),
            None => mesh::subdivide::polygon_count(&self.base, top as u32 + 1),
        }
    }

    /// The stroke-free level sculpted on becomes what it was left as: a
    /// level gone back to, with nothing below changed, costs nothing.
    fn leave(&mut self) {
        let shape = self.shape();
        if let Some(rung) = self.ladder.get_mut(self.level) {
            rung.shape = Some(shape);
        }
    }

    /// The moves of the level sculpted on changed: what it was left as,
    /// and every finer level's points, are out of date.
    pub(super) fn changed_here(&mut self) {
        for (k, rung) in self.ladder.iter_mut().enumerate() {
            if k >= self.level {
                rung.shape = None;
            }
            if k > self.level {
                rung.stale = true;
            }
        }
    }

    /// Level `k` as sculpted, worked out from the shape under it when it
    /// isn't kept. Not for the level sculpted on, unless it was just left.
    fn shape_of(&mut self, k: usize) -> Option<Arc<Mesh>> {
        let rung = self.ladder.get(k)?;
        if let Some(shape) = &rung.shape {
            return Some(Arc::clone(shape));
        }
        let mut mesh = (*rung.under).clone();
        if let Some(moves) = self.sculpt.level_moves(k) {
            levels::move_points(&mut mesh, moves);
        }
        let shape = Arc::new(mesh);
        self.ladder[k].shape = Some(Arc::clone(&shape));
        Some(shape)
    }

    /// Makes sure there is a rung for level `k` (1 or above) with its
    /// points up to date, from the level below as sculpted. A rung whose
    /// polygons stand (a level gone back to, or added again after an
    /// undo) only has its points worked out again. Whether there is one.
    fn rung(&mut self, k: usize) -> bool {
        let known = self.ladder.get(k).map(|rung| rung.stale);
        if known == Some(false) {
            return true;
        }
        let Some(below) = k.checked_sub(1).and_then(|b| self.shape_of(b)) else {
            return false;
        };
        match known {
            Some(_) => {
                let rung = &mut self.ladder[k];
                let places = rung.refinement.as_ref().and_then(|r| r.places_of(&below));
                let Some(places) = places else {
                    return false;
                };
                rung.under = Arc::new(levels::with_places(&rung.under, &places));
                rung.stale = false;
                rung.shape = None;
            }
            None => {
                let Some(step) = levels::step_up(&below) else {
                    return false;
                };
                self.ladder.push(Rung {
                    refinement: Some(Arc::new(step.refinement)),
                    under: Arc::new(step.under),
                    stale: false,
                    shape: None,
                });
            }
        }
        true
    }

    /// Works the levels out up to `level`: a rung for each, its points up
    /// to date, and each level's sculpting fitted to it. Returns the level
    /// reached (lower when one couldn't be made).
    fn rise_to(&mut self, level: usize) -> usize {
        let level = level.min(self.sculpt.top());
        for k in 1..=level {
            if !self.rung(k) {
                return k - 1;
            }
            let Some(refinement) = self.ladder[k].refinement.clone() else {
                return k - 1;
            };
            if !self.sculpt.level_fits(k, &refinement) {
                match self.sculpt.fitting_level(k, &refinement) {
                    Some(fitted) => self.sculpt = fitted,
                    None => return k - 1,
                }
                self.fitted = true;
                self.ladder[k].shape = None;
            }
        }
        level
    }

    /// Starts working on `level` (or the highest below it that can be
    /// made): its shape, the boxes round its polygons, and the mask as it
    /// shows there. The level worked on before must have been left, or
    /// what it was left as dropped.
    pub(super) fn enter(&mut self, level: usize) {
        let level = self.rise_to(level);
        let kept = self.ladder[level].shape.clone();
        let shape = match kept.or_else(|| self.shape_of(level)) {
            Some(shape) => shape,
            None => Arc::clone(&self.ladder[level].under),
        };
        self.input = Arc::clone(&self.ladder[level].under);
        self.mesh = (*shape).clone();
        self.shown = Some(shape);
        self.changed = false;
        self.tree = FaceTree::new(&self.mesh);
        let slots = self.mesh.point_slot_count();
        self.seen = vec![0; slots];
        self.dab_mark = 0;
        self.moved = vec![0; slots];
        self.stroke_mark = 1;
        self.touched.clear();
        self.first.clear();
        self.gathered.clear();
        self.gathered_at.clear();
        self.gather_mark = 0;
        self.last_centre.fill(None);
        self.held.fill(None);
        self.moving = None;
        self.level = level;
        self.mask = self.level_mask(&self.sculpt, level);
        self.mask_before = self.mask.clone();
        self.mask_touched.clear();
        self.mask_changed = true;
        self.version += 1;
        self.middle = None;
    }

    /// `sculpt`'s mask as it shows on `level` (whose rungs are worked
    /// out): itself on the level it was changed on; on a coarser level,
    /// that level's points' values (a finer level's first points are the
    /// coarser one's); on a finer one, carried to the new points.
    pub(super) fn level_mask(&self, sculpt: &Sculpt, level: usize) -> Mask {
        let len = self
            .ladder
            .get(level)
            .map_or(0, |rung| rung.under.point_slot_count());
        let (mask, on) = (sculpt.mask(), sculpt.mask_level());
        if mask.is_zero() {
            return Mask::zero(len);
        }
        if on == level && mask.len() == len {
            return mask.clone();
        }
        if on >= level {
            return match mask.values().get(..len) {
                Some(first) => Mask::from_values(first),
                None => Mask::zero(len),
            };
        }
        let mut values: Vec<f32> = mask.values().into_iter().map(f32::from).collect();
        for k in on + 1..=level {
            let carried = self
                .ladder
                .get(k)
                .and_then(|rung| rung.refinement.as_ref())
                .and_then(|r| r.carried(&values));
            match carried {
                Some(carried) => values = carried,
                None => return Mask::zero(len),
            }
        }
        let values: Vec<u8> = values
            .into_iter()
            .map(|v| v.round().clamp(0.0, 255.0) as u8)
            .collect();
        if values.len() == len {
            Mask::from_values(&values)
        } else {
            Mask::zero(len)
        }
    }

    /// Whether `sculpt` differs from the live sculpt's only in the moves
    /// and mask of the level sculpted on: what a stroke, or its undo,
    /// changes.
    pub(super) fn differs_only_here(&self, sculpt: &Sculpt) -> bool {
        let now = &self.sculpt;
        sculpt.fits(&self.base)
            && sculpt.level_count() == now.level_count()
            && sculpt.shown() == self.level
            && now.shown() == self.level
            && (0..now.level_count()).all(|k| {
                if k == self.level {
                    sculpt.level_structure(k) == now.level_structure(k)
                        && sculpt.level_places(k).map(<[Vec3]>::len)
                            == now.level_places(k).map(<[Vec3]>::len)
                } else {
                    sculpt.same_level(now, k)
                }
            })
    }

    /// Brings the live sculpt to `sculpt` level by level: the levels
    /// below the lowest that differs stand, the rest are worked out again
    /// (their points only, where their polygons stand).
    pub(super) fn resync(&mut self, sculpt: &Sculpt) {
        let (sculpt, fitted) = sculpt.fitted_to(&self.base);
        let count = sculpt.level_count().max(self.sculpt.level_count());
        // A sculpt fitted to a changed shape differs from the start.
        let differs = if fitted {
            0
        } else {
            (0..count)
                .find(|k| !sculpt.same_level(&self.sculpt, *k))
                .unwrap_or(count)
        };
        if self.level < differs {
            self.leave();
        }
        // Rungs above the levels there are now stay, stale: a level added
        // again (a redo) then needs only its points.
        for (k, rung) in self.ladder.iter_mut().enumerate() {
            if k >= differs {
                rung.shape = None;
            }
            if k > differs {
                rung.stale = true;
            }
        }
        self.fitted = fitted;
        let shown = sculpt.shown();
        self.sculpt = sculpt;
        self.enter(shown);
    }

    /// Goes to detail level `level` (Page Up and Page Down): the stroke
    /// under way, if any, is taken back first. Returns the new sculpt for
    /// the undo step, or `None` when it is the level sculpted on already
    /// or there is no such level.
    pub fn set_level(&mut self, level: usize) -> Option<Sculpt> {
        self.cancel();
        if level == self.level || level > self.sculpt.top() {
            return None;
        }
        self.leave();
        self.sculpt = self.sculpt.showing(level)?;
        self.enter(level);
        if self.level != self.sculpt.shown() {
            self.sculpt = self.sculpt.showing(self.level)?;
        }
        Some(self.sculpt.clone())
    }

    /// Adds a detail level on top of the finest (the finest as sculpted,
    /// subdivided once) and goes to it. Returns the new sculpt for the
    /// undo step, or why it couldn't.
    pub fn add_level(&mut self) -> Result<Sculpt, LevelProblem> {
        self.cancel();
        let top = self.sculpt.top();
        if top >= MAX_LEVELS {
            return Err(LevelProblem::TooManyLevels);
        }
        self.leave();
        if self.rise_to(top) < top {
            return Err(LevelProblem::NoShape);
        }
        let shape = self.shape_of(top).ok_or(LevelProblem::NoShape)?;
        let polygons = levels::polygons_above(&shape);
        if polygons > MAX_POLYGONS {
            return Err(LevelProblem::TooHeavy { polygons });
        }
        // A rung kept from before (the level was added and taken back)
        // needs at most its points worked out again.
        if !self.rung(top + 1) {
            return Err(LevelProblem::NoShape);
        }
        let refinement = self.ladder[top + 1]
            .refinement
            .clone()
            .ok_or(LevelProblem::NoShape)?;
        let under = self
            .sculpt
            .level_places(top)
            .ok_or(LevelProblem::NoShape)?;
        let below: Vec<Vec3> = under
            .iter()
            .map(|p| if p.is_finite() { *p } else { Vec3::ZERO })
            .collect();
        let mut places = refinement.places(&below).ok_or(LevelProblem::NoShape)?;
        // A slot with no point below has none on the new level.
        for (i, p) in under.iter().enumerate() {
            if !p.is_finite() {
                places[i] = Vec3::NAN;
            }
        }
        self.sculpt = self
            .sculpt
            .with_new_level(places)
            .ok_or(LevelProblem::NoShape)?;
        // What a level deleted or taken back there was left as isn't the
        // new level's shape.
        self.ladder[top + 1].shape = None;
        self.enter(top + 1);
        Ok(self.sculpt.clone())
    }

    /// Deletes the detail levels above the one sculpted on: their
    /// sculpting goes. Returns the new sculpt for the undo step, or `None`
    /// when this is the finest level.
    pub fn delete_higher(&mut self) -> Option<Sculpt> {
        self.cancel();
        let new = self.sculpt.without_higher()?;
        // The rungs above stay, for a level added again; what the levels
        // deleted were left as goes.
        for rung in self.ladder.iter_mut().skip(self.level + 1) {
            rung.shape = None;
        }
        self.sculpt = new.clone();
        Some(new)
    }

    /// Sets which detail level exports and bakes take. Returns the new
    /// sculpt for the undo step, or `None` when that is what they take
    /// already.
    pub fn set_export(&mut self, export: levels::ExportLevel) -> Option<Sculpt> {
        if self.sculpt.export() == export {
            return None;
        }
        self.cancel();
        let new = self.sculpt.with_export(export);
        self.sculpt = new.clone();
        Some(new)
    }

    /// What deleting the detail levels below the one sculpted on leaves:
    /// that level as sculpted, as a shape of its own, and a sculpt made
    /// for it holding the finer levels (their moves as they are) and the
    /// mask. The shape of every level kept is exactly what it was.
    /// `None` on level 0, or when a finer level can't be worked out.
    /// Nothing changes here: the interface makes the shape the object's
    /// base, with the sculpt on it, as one undo step.
    pub fn without_lower(&mut self) -> Option<(Arc<Mesh>, Sculpt)> {
        self.cancel();
        let from = self.level;
        let top = self.sculpt.top();
        if from == 0 {
            return None;
        }
        self.leave();
        if self.rise_to(top) < top {
            return None;
        }
        let base = self.shape_of(from)?;
        let mut new = Sculpt::new(&base);
        let structure = new.structure();
        let mut places: Vec<Vec3> = base.point_slots().map(|p| p.unwrap_or(Vec3::ZERO)).collect();
        for k in from + 1..=top {
            places = self.ladder[k].refinement.as_ref()?.places(&places)?;
            let mut kept = places.clone();
            for (i, p) in base.point_slots().enumerate() {
                if p.is_none() {
                    kept[i] = Vec3::NAN;
                }
            }
            new = new.with_level(structure, kept, self.sculpt.level_moves(k)?.clone())?;
        }
        // The mask, on its own level or carried up to the new level 0.
        let on = self.sculpt.mask_level().max(from);
        let mask = if on == self.sculpt.mask_level() {
            self.sculpt.mask().clone()
        } else {
            self.level_mask(&self.sculpt, on)
        };
        if !mask.is_zero() {
            new = new.with_mask_at(on - from, mask)?;
        }
        let export = match self.sculpt.export() {
            levels::ExportLevel::Level(level) => {
                levels::ExportLevel::Level(level.saturating_sub(from as u8))
            }
            other => other,
        };
        Some((base, new.with_export(export)))
    }
}
