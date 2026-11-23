//! A Sculpt layer's data (decision 0304): how far each point has moved,
//! measured along the surface, and the shape it was sculpted on.
//!
//! A [`Sculpt`] is a cheap handle: copies share everything, and a stroke
//! makes a new one that shares every chunk it didn't touch. It never
//! changes once made.

use crate::levels::{self, ExportLevel, Level, MAX_LEVELS};
use crate::mask::{self, Mask};
use crate::moves::{CHUNK, Chunk, Moves, same_chunk};
use crate::nearest::Grid;
use glam::Vec3;
use mesh::Mesh;
use mesh::subdivide::Refinement;
use std::sync::{Arc, Weak};

pub use crate::levels::sculpted_places;

/// The most points a sculpt may have (a file claiming more is damaged):
/// ten times the heavy test shape.
pub const MAX_POINTS: usize = 30_000_000;

/// One Sculpt layer's sculpting.
#[derive(Clone)]
pub struct Sculpt(Arc<Inner>);

#[derive(Clone)]
struct Inner {
    /// The detail levels (decision 0315), the shape coming in first: its
    /// structure ([`Mesh::structure_fingerprint`]), where its points were
    /// (for fitting the sculpt again when the shape under it changes its
    /// points) and each point's move along its frame. There is always
    /// one.
    levels: Vec<Level>,
    /// The level shown, and sculpted on.
    shown: u8,
    /// The level an export or a bake takes.
    export: ExportLevel,
    /// How much each point is held still (decision 0310): not part of
    /// the shape, only of what brushes and the gizmo may move. One value
    /// for each point of level `mask_level`, the level it was last
    /// changed on.
    mask: Mask,
    mask_level: u8,
    /// Where the stroke that made this sculpt left its points, and where
    /// they were before it (decision 0313): not part of the sculpt, and
    /// never saved, only what makes undoing and redoing it instant.
    kept: Option<Arc<Kept>>,
}

/// What a stroke keeps so it can be undone and redone without working
/// its points out again (decision 0313): the points it moved, where they
/// were before it and where it left them, on the shape it was sculpted
/// on. About 28 bytes a point moved, counted in the undo step's memory.
pub(crate) struct Kept {
    /// The sculpt the stroke started from.
    from: Weak<Inner>,
    /// The shape coming in that the places were worked out on.
    input: Weak<Mesh>,
    /// The points the stroke moved, by slot.
    pub(crate) points: Vec<u32>,
    /// Where they were before it.
    pub(crate) before: Vec<Vec3>,
    /// Where it left them.
    pub(crate) after: Vec<Vec3>,
}

impl Kept {
    fn memory_size(&self) -> usize {
        self.points.len() * (4 + 12 + 12)
    }
}

/// Which way to go with a stroke's [`Kept`] places.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Way {
    /// Undo it: its points go back to where they were.
    Back,
    /// Redo it: its points go where it left them.
    Again,
}

/// How a sculpt met the shape it was applied to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Fit {
    /// The shape's points changed since it was sculpted, so each point took
    /// the moves of the nearest point it was sculpted on (the layer shows
    /// a badge).
    pub found_again: bool,
}

impl Sculpt {
    fn of(inner: Inner) -> Self {
        Self(Arc::new(inner))
    }

    /// This sculpt with `change` made to it (what a stroke kept for undo
    /// goes: it belongs to the sculpt it was made with).
    fn changed(&self, change: impl FnOnce(&mut Inner)) -> Self {
        let mut inner = (*self.0).clone();
        inner.kept = None;
        change(&mut inner);
        Self::of(inner)
    }

    /// A sculpt with nothing moved yet, made for `mesh`.
    pub fn new(mesh: &Mesh) -> Self {
        let places: Vec<Vec3> = mesh.point_slots().map(|p| p.unwrap_or(Vec3::NAN)).collect();
        Self::of(Inner {
            mask: Mask::zero(places.len()),
            levels: vec![Level {
                structure: mesh.structure_fingerprint(),
                moves: Moves::zero(places.len()),
                places: places.into(),
            }],
            shown: 0,
            export: ExportLevel::Shown,
            mask_level: 0,
            kept: None,
        })
    }

    /// A sculpt from its parts (for files): `None` when they don't fit
    /// together (a move per place), there are too many, or a move isn't a
    /// real number.
    pub fn from_parts(structure: u64, places: Vec<Vec3>, moves: Moves) -> Option<Self> {
        if places.len() != moves.len() || places.len() > MAX_POINTS || !moves.is_valid() {
            return None;
        }
        Some(Self::of(Inner {
            mask: Mask::zero(places.len()),
            levels: vec![Level {
                structure,
                places: places.into(),
                moves,
            }],
            shown: 0,
            export: ExportLevel::Shown,
            mask_level: 0,
            kept: None,
        }))
    }

    /// The same sculpt with one more detail level on top, from its parts
    /// (for files): `None` when they don't fit together, there are
    /// [`MAX_LEVELS`] already, the level has too many points or fewer
    /// than the one below, or a move isn't a real number.
    pub fn with_level(&self, structure: u64, places: Vec<Vec3>, moves: Moves) -> Option<Self> {
        let below = self.0.levels.last().map_or(0, |l| l.places.len());
        if places.len() != moves.len()
            || places.len() > MAX_POINTS
            || places.len() < below
            || !moves.is_valid()
            || self.top() >= MAX_LEVELS
        {
            return None;
        }
        Some(self.changed(|inner| {
            inner.levels.push(Level {
                structure,
                places: places.into(),
                moves,
            });
        }))
    }

    /// The same sculpt with a fresh detail level on top, its points at
    /// `places` with nothing sculpted, shown.
    pub(crate) fn with_new_level(&self, places: Vec<Vec3>) -> Option<Self> {
        let moves = Moves::zero(places.len());
        let new = self.with_level(self.structure(), places, moves)?;
        new.showing(new.top())
    }

    /// The same sculpt showing detail level `level`. `None` when it has
    /// no such level.
    pub fn showing(&self, level: usize) -> Option<Self> {
        (level <= self.top()).then(|| self.changed(|inner| inner.shown = level as u8))
    }

    /// The same sculpt with exports and bakes taking `export`.
    pub fn with_export(&self, export: ExportLevel) -> Self {
        self.changed(|inner| inner.export = export)
    }

    /// The same sculpt without the levels above the one shown; the mask
    /// comes down to it if it was on one of them. `None` when the level
    /// shown is the finest.
    pub fn without_higher(&self) -> Option<Self> {
        let shown = self.shown();
        if shown >= self.top() {
            return None;
        }
        let mask = self.mask_down_to(shown);
        Some(self.changed(|inner| {
            inner.levels.truncate(shown + 1);
            if let Some(mask) = mask {
                inner.mask = mask;
                inner.mask_level = shown as u8;
            }
        }))
    }

    /// The mask brought down to `level`, when it is on a finer one: the
    /// finer level's first points are that level's. `None` when it is on
    /// `level` or below already.
    pub(crate) fn mask_down_to(&self, level: usize) -> Option<Mask> {
        if self.mask_level() <= level {
            return None;
        }
        let len = self.0.levels.get(level)?.places.len();
        let values = self.0.mask.values();
        Some(match values.get(..len) {
            Some(first) => Mask::from_values(first),
            None => Mask::zero(len),
        })
    }

    /// The same sculpt with other moves on the level shown (of the same
    /// length).
    pub fn with_moves(&self, moves: Moves) -> Option<Self> {
        self.with_moves_at(self.shown(), moves)
    }

    /// The same sculpt with other moves on detail level `level` (of the
    /// same length).
    pub fn with_moves_at(&self, level: usize, moves: Moves) -> Option<Self> {
        let now = self.0.levels.get(level)?;
        (moves.len() == now.moves.len() && moves.is_valid())
            .then(|| self.changed(|inner| inner.levels[level].moves = moves))
    }

    /// The same sculpt with another mask, a value for each point of the
    /// level shown.
    pub fn with_mask(&self, mask: Mask) -> Option<Self> {
        self.with_mask_at(self.shown(), mask)
    }

    /// The same sculpt with another mask, a value for each point of
    /// detail level `level`.
    pub fn with_mask_at(&self, level: usize, mask: Mask) -> Option<Self> {
        let now = self.0.levels.get(level)?;
        (mask.len() == now.places.len()).then(|| {
            self.changed(|inner| {
                inner.mask = mask;
                inner.mask_level = level as u8;
            })
        })
    }

    /// This sculpt, made by a stroke from `from` on `input`, keeping
    /// where the stroke left `points` (`after`) and where they were
    /// (`before`), so undoing and redoing it needn't work them out again.
    pub(crate) fn keeping(
        mut self,
        from: &Sculpt,
        input: &Arc<Mesh>,
        points: Vec<u32>,
        before: Vec<Vec3>,
        after: Vec<Vec3>,
    ) -> Self {
        if points.len() == before.len()
            && points.len() == after.len()
            && let Some(inner) = Arc::get_mut(&mut self.0)
        {
            inner.kept = Some(Arc::new(Kept {
                from: Arc::downgrade(&from.0),
                input: Arc::downgrade(input),
                points,
                before,
                after,
            }));
        }
        self
    }

    /// The places that take the shape from this sculpt to `to` on
    /// `input` at once, when one was made by a stroke from the other on
    /// that very shape, and which way to use them.
    pub(crate) fn kept_way_to(&self, to: &Sculpt, input: &Arc<Mesh>) -> Option<(Arc<Kept>, Way)> {
        let on = |k: &Kept| Weak::as_ptr(&k.input) == Arc::as_ptr(input);
        if let Some(k) = &self.0.kept
            && Weak::as_ptr(&k.from) == Arc::as_ptr(&to.0)
            && on(k)
        {
            return Some((Arc::clone(k), Way::Back));
        }
        match &to.0.kept {
            Some(k) if Weak::as_ptr(&k.from) == Arc::as_ptr(&self.0) && on(k) => {
                Some((Arc::clone(k), Way::Again))
            }
            _ => None,
        }
    }

    /// How much each point is held still: a value for each point of
    /// level [`Self::mask_level`].
    pub fn mask(&self) -> &Mask {
        &self.0.mask
    }

    /// The detail level the mask was last changed on. On a finer level it
    /// is carried to the new points; on a coarser one, that level's
    /// points keep theirs.
    pub fn mask_level(&self) -> usize {
        usize::from(self.0.mask_level).min(self.top())
    }

    /// The structure of the shape it was sculpted on.
    pub fn structure(&self) -> u64 {
        self.0.levels.first().map_or(0, |l| l.structure)
    }

    /// How many detail levels it has, the shape coming in counted: one
    /// for a sculpt with no finer level.
    pub fn level_count(&self) -> usize {
        self.0.levels.len()
    }

    /// Its finest detail level (0 with none added).
    pub fn top(&self) -> usize {
        self.0.levels.len().saturating_sub(1)
    }

    /// The detail level shown, and sculpted on.
    pub fn shown(&self) -> usize {
        usize::from(self.0.shown).min(self.top())
    }

    /// Which level an export or a bake takes.
    pub fn export(&self) -> ExportLevel {
        self.0.export
    }

    /// The detail level an export or a bake takes.
    pub fn export_level(&self) -> usize {
        match self.0.export {
            ExportLevel::Shown => self.shown(),
            ExportLevel::Finest => self.top(),
            ExportLevel::Level(level) => usize::from(level).min(self.top()),
        }
    }

    /// This sculpt as an export or a bake takes it: showing its export
    /// level. `None` when that is the level shown.
    pub fn for_export(&self) -> Option<Self> {
        let level = self.export_level();
        if level == self.shown() {
            None
        } else {
            self.showing(level)
        }
    }

    fn level(&self, level: usize) -> &Level {
        // There is always a level, and `min` keeps within them.
        &self.0.levels[level.min(self.top())]
    }

    /// Where detail level `level`'s points are with nothing sculpted, by
    /// slot. `None` when it has no such level.
    pub fn level_places(&self, level: usize) -> Option<&[Vec3]> {
        self.0.levels.get(level).map(|l| &*l.places)
    }

    /// Each point's move on detail level `level`. `None` when it has no
    /// such level.
    pub fn level_moves(&self, level: usize) -> Option<&Moves> {
        self.0.levels.get(level).map(|l| &l.moves)
    }

    /// The structure of the shape coming in that detail level `level`
    /// was made for. `None` when it has no such level.
    pub fn level_structure(&self, level: usize) -> Option<u64> {
        self.0.levels.get(level).map(|l| l.structure)
    }

    /// Whether detail level `level` is the same in `other`.
    pub(crate) fn same_level(&self, other: &Self, level: usize) -> bool {
        match (self.0.levels.get(level), other.0.levels.get(level)) {
            (Some(a), Some(b)) => a.same(b),
            _ => false,
        }
    }

    /// Where the points of the level shown were, by slot.
    pub fn places(&self) -> &[Vec3] {
        &self.level(self.shown()).places
    }

    /// Each point's move on the level shown.
    pub fn moves(&self) -> &Moves {
        &self.level(self.shown()).moves
    }

    /// How many point slots the level shown has.
    pub fn len(&self) -> usize {
        self.level(self.shown()).places.len()
    }

    /// Whether the level shown has no points.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Whether every move is a real number.
    pub fn is_valid(&self) -> bool {
        self.0.levels.iter().all(|l| l.moves.is_valid())
    }

    /// Whether it was sculpted on a shape with `mesh`'s points and
    /// polygons (the points may have moved since).
    pub fn fits(&self, mesh: &Mesh) -> bool {
        let base = self.level(0);
        base.structure == mesh.structure_fingerprint()
            && base.places.len() == mesh.point_slot_count()
    }

    /// This sculpt for `mesh`: itself when it fits; otherwise each of
    /// `mesh`'s points takes the moves of the nearest point it was
    /// sculpted on (decision 0032's promise, kept by position), and the
    /// result is made for `mesh`. The flag says whether it was fitted.
    ///
    /// Only the shape coming in is fitted here. Each finer level is
    /// fitted when it is worked out ([`Self::apply`], the live sculpt),
    /// since its points are only known then; until then it keeps what it
    /// was made for. The mask comes down to the level shown first when it
    /// is on a finer one, and is fitted with its level.
    pub fn fitted_to(&self, mesh: &Mesh) -> (Self, bool) {
        if self.fits(mesh) {
            return (self.clone(), false);
        }
        let old = self.level(0);
        let places: Vec<Vec3> = mesh.point_slots().map(|p| p.unwrap_or(Vec3::NAN)).collect();
        let lower = self.mask_down_to(self.shown());
        let (mask, mask_level) = match lower {
            Some(mask) => (mask, self.shown()),
            None => (self.0.mask.clone(), self.mask_level()),
        };
        let (moves, fitted_mask) = fitted(old, &places, (mask_level == 0).then_some(&mask));
        let fitted = self.changed(|inner| {
            inner.levels[0] = Level {
                structure: mesh.structure_fingerprint(),
                places: places.into(),
                moves,
            };
            inner.mask = fitted_mask.unwrap_or(mask);
            inner.mask_level = mask_level as u8;
        });
        (fitted, true)
    }

    /// Whether detail level `level` (1 or above) was made for the shape
    /// this sculpt is made for now, with the points `refinement` gives
    /// it.
    pub(crate) fn level_fits(&self, level: usize, refinement: &Refinement) -> bool {
        self.0.levels.get(level).is_some_and(|l| {
            l.structure == self.structure()
                && l.places.len() == refinement.fine_slots()
                && l.moves.len() == l.places.len()
        })
    }

    /// This sculpt with detail level `level` (1 or above) fitted by
    /// position to the points `refinement` gives it from the level below
    /// (itself fitted): each new point takes the moves of the nearest
    /// point the level was sculpted on, and the mask's value when the
    /// mask is on this level. `None` when it has no such level.
    pub(crate) fn fitting_level(&self, level: usize, refinement: &Refinement) -> Option<Self> {
        let old = self.0.levels.get(level)?;
        let below = self.0.levels.get(level.checked_sub(1)?)?;
        let below_places: Vec<Vec3> = below
            .places
            .iter()
            .map(|p| if p.is_finite() { *p } else { Vec3::ZERO })
            .collect();
        let mut places = refinement.places(&below_places)?;
        // A slot with no point below has none here.
        for (i, p) in below.places.iter().enumerate() {
            if !p.is_finite() {
                places[i] = Vec3::NAN;
            }
        }
        let with_mask = self.mask_level() == level;
        let (moves, mask) = fitted(old, &places, with_mask.then_some(&self.0.mask));
        let structure = self.structure();
        Some(self.changed(|inner| {
            inner.levels[level] = Level {
                structure,
                places: places.into(),
                moves,
            };
            if let Some(mask) = mask {
                inner.mask = mask;
            }
        }))
    }

    /// Moves `mesh`'s points by the sculpt, each along its frame on
    /// `mesh` as it comes in, and subdivides it up to the level shown,
    /// each level's points moved along their frames in turn. Points in
    /// chunks where nothing moved aren't looked at.
    pub fn apply(&self, mesh: &mut Mesh) -> Fit {
        self.apply_at(mesh, self.shown())
    }

    /// [`Self::apply`] up to detail level `level` instead of the level
    /// shown (the finest when there are fewer). A level that can't be
    /// made (it would pass [`levels::MAX_POLYGONS`]) leaves the shape at
    /// the level below.
    pub fn apply_at(&self, mesh: &mut Mesh, level: usize) -> Fit {
        let (mut sculpt, mut found_again) = self.fitted_to(mesh);
        levels::move_points(mesh, &sculpt.level(0).moves);
        for k in 1..=level.min(sculpt.top()) {
            let Some(step) = levels::step_up(mesh) else {
                break;
            };
            if !sculpt.level_fits(k, &step.refinement) {
                match sculpt.fitting_level(k, &step.refinement) {
                    Some(fitted) => sculpt = fitted,
                    None => break,
                }
                found_again = true;
            }
            *mesh = step.under;
            levels::move_points(mesh, &sculpt.level(k).moves);
        }
        Fit { found_again }
    }

    /// The bytes this sculpt holds that `other` doesn't share: what an undo
    /// step changing one into the other keeps.
    pub fn memory_apart(&self, other: &Self) -> usize {
        if Arc::ptr_eq(&self.0, &other.0) {
            return 0;
        }
        let mut total = 0;
        for (k, level) in self.0.levels.iter().enumerate() {
            let theirs = other.0.levels.get(k);
            if !theirs.is_some_and(|t| Arc::ptr_eq(&level.places, &t.places)) {
                total += std::mem::size_of_val(&*level.places);
            }
            let their_chunks = theirs.map_or(&[][..], |t| t.moves.chunks());
            total += level
                .moves
                .chunks()
                .iter()
                .enumerate()
                .filter(|(i, c)| !their_chunks.get(*i).is_some_and(|t| same_chunk(c, t)))
                .map(|(_, c)| chunk_bytes(c))
                .sum::<usize>();
        }
        let theirs = other.0.mask.chunks();
        let masked: usize = self
            .0
            .mask
            .chunks()
            .iter()
            .enumerate()
            .filter(|(i, c)| !theirs.get(*i).is_some_and(|t| mask::same_chunk(c, t)))
            .map(|(_, c)| c.as_ref().map_or(0, |c| c.len()))
            .sum();
        let kept = match (&self.0.kept, &other.0.kept) {
            (Some(a), Some(b)) if Arc::ptr_eq(a, b) => 0,
            (Some(a), _) => a.memory_size(),
            (None, _) => 0,
        };
        total + masked + kept
    }

    /// All the bytes it holds.
    pub fn memory_size(&self) -> usize {
        self.0
            .levels
            .iter()
            .map(|l| std::mem::size_of_val(&*l.places) + l.moves.memory_size())
            .sum::<usize>()
            + self.0.mask.memory_size()
            + self.0.kept.as_ref().map_or(0, |k| k.memory_size())
    }
}

fn chunk_bytes(c: &Chunk) -> usize {
    c.as_ref().map_or(0, |c| std::mem::size_of_val(&**c))
}

/// A level's moves, and `mask` if given, fitted to points at `places`:
/// each takes the moves (and mask value) of the nearest point the level
/// was sculpted on. Worked out on every core for many points.
fn fitted(old: &Level, places: &[Vec3], mask: Option<&Mask>) -> (Moves, Option<Mask>) {
    let len = places.len();
    let mut moves = Moves::zero(len);
    let wanted_mask = mask.filter(|m| !m.is_zero() && m.len() == old.places.len());
    let Some(grid) = Grid::new(&old.places) else {
        return (moves, mask.map(|_| Mask::zero(len)));
    };
    if old.moves.is_zero() && wanted_mask.is_none() {
        return (moves, mask.map(|_| Mask::zero(len)));
    }
    // Each new point's nearest old point, once for the moves and mask.
    let nearest: Vec<Option<u32>> = mesh::parallel::in_parallel(len, |range| {
        range
            .map(|i| grid.nearest(&old.places, places[i]))
            .collect::<Vec<_>>()
    })
    .into_iter()
    .flatten()
    .collect();
    if !old.moves.is_zero() {
        for c in 0..len.div_ceil(CHUNK) {
            let start = c * CHUNK;
            let chunk: Vec<Vec3> = (start..(start + CHUNK).min(len))
                .map(|i| nearest[i].map_or(Vec3::ZERO, |j| old.moves.get(j as usize)))
                .collect();
            moves.set_chunk(c, chunk);
        }
    }
    let mask = mask.map(|_| match wanted_mask {
        Some(m) => {
            let values: Vec<u8> = nearest
                .iter()
                .map(|j| j.map_or(0, |j| m.get(j as usize)))
                .collect();
            Mask::from_values(&values)
        }
        None => Mask::zero(len),
    });
    (moves, mask)
}

impl PartialEq for Sculpt {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
            || (self.0.levels.len() == other.0.levels.len()
                && self.0.shown == other.0.shown
                && self.0.export == other.0.export
                && self.0.mask_level == other.0.mask_level
                && self.0.levels.iter().zip(&other.0.levels).all(|(a, b)| a.same(b))
                && self.0.mask == other.0.mask)
    }
}

impl std::fmt::Debug for Sculpt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sculpt")
            .field("points", &self.len())
            .field("levels", &self.level_count())
            .field("shown", &self.shown())
            .field("moves", &self.moves())
            .field("mask", &self.0.mask)
            .finish()
    }
}
