//! The commands that change curves
//! ([decision 0138](../../../../docs/decisions/0138-curves-sweep-lathe.md)).

use super::{Command, Run, existing};
use crate::curve::{CurveObject, CurveShape, CurveSolid, NOT_A_CURVE, SweepProfile};
use crate::object::{Object, ObjectId, ObjectKind};
use crate::tree::{Edit, Place, SceneError};
use mesh::curve::{Curve, CurveEdit, CurveEnd, CurvePointId, PointKind, ReadyCurve};
use words::{tr, trf};

/// Why curve objects can't be joined: one is a loop.
const CLOSED_HAS_NO_ENDS: &str =
    words::text!("a closed curve has no ends to join: open it or split it first");

impl Run<'_> {
    /// The curve object `id` is.
    fn curve_of(&self, id: ObjectId) -> Result<CurveObject, SceneError> {
        match &self
            .scene
            .object(id)
            .ok_or(SceneError::UnknownObject(id))?
            .kind
        {
            ObjectKind::Curve(c) => Ok(c.clone()),
            _ => Err(SceneError::Curve(NOT_A_CURVE)),
        }
    }

    /// Gives curve object `id` the curve `after` (nothing is recorded if
    /// it is the same).
    fn set_curve(&mut self, id: ObjectId, after: CurveObject) -> Result<(), SceneError> {
        let before = self
            .scene
            .object(id)
            .ok_or(SceneError::UnknownObject(id))?
            .kind
            .clone();
        let after = ObjectKind::Curve(after);
        if before != after {
            self.edit(Edit::SetKind { id, before, after })?;
        }
        Ok(())
    }

    /// Changes curve `id`'s points by `edit`: a ready-made curve becomes
    /// points first.
    pub(super) fn edit_curve(&mut self, id: ObjectId, edit: &CurveEdit) -> Result<(), SceneError> {
        let mut curve = self.curve_of(id)?;
        let edited = curve
            .shape
            .curve()
            .edited(edit)
            .map_err(|why| SceneError::Curve(curve_reason(why)))?;
        if edited == curve.shape.curve() {
            return Ok(());
        }
        curve.shape = CurveShape::Points(edited);
        self.set_curve(id, curve)
    }

    /// Gives curves `ids` the solid `solid` (within limits); a cross-section
    /// from another curve must be a curve object other than each of them.
    pub(super) fn set_curve_solid(
        &mut self,
        ids: &[ObjectId],
        solid: &CurveSolid,
    ) -> Result<(), SceneError> {
        let solid = solid.clamped();
        for id in existing(self.scene, ids)? {
            let mut curve = self.curve_of(id)?;
            self.scene.check_profile(id, &solid)?;
            curve.solid = solid;
            self.set_curve(id, curve)?;
        }
        Ok(())
    }

    /// Gives ready-made curves `ids` new settings, of the same kind.
    pub(super) fn set_ready_curve(
        &mut self,
        ids: &[ObjectId],
        ready: &ReadyCurve,
    ) -> Result<(), SceneError> {
        let ready = ready.clamped();
        for id in existing(self.scene, ids)? {
            let mut curve = self.curve_of(id)?;
            match curve.shape {
                CurveShape::Ready(r) if r.kind() == ready.kind() => {}
                _ => return Err(SceneError::WrongKind(ready.name())),
            }
            curve.shape = CurveShape::Ready(ready);
            self.set_curve(id, curve)?;
        }
        Ok(())
    }

    /// Moves curve `id`'s points by `matrix` (applying its transforms,
    /// decision 0089): a ready-made curve becomes points.
    pub(super) fn transform_curve(
        &mut self,
        id: ObjectId,
        matrix: glam::Mat4,
    ) -> Result<(), SceneError> {
        let mut curve = self.curve_of(id)?;
        let moved = curve
            .shape
            .curve()
            .transformed(matrix)
            .map_err(|why| SceneError::Curve(curve_reason(why)))?;
        curve.shape = CurveShape::Points(moved);
        self.set_curve(id, curve)
    }

    /// Curves whose cross-section is one of `going` (being deleted) take a
    /// round one instead, in the same step.
    pub(super) fn forget_profiles(&mut self, going: &[ObjectId]) -> Result<(), SceneError> {
        let users: Vec<ObjectId> = self
            .scene
            .walk()
            .into_iter()
            .map(|(id, _)| id)
            .filter(|id| !going.contains(id))
            .filter(|id| {
                self.scene
                    .object(*id)
                    .and_then(|o| o.kind.curve())
                    .and_then(|c| c.solid.profile_object())
                    .is_some_and(|p| going.contains(&p))
            })
            .collect();
        for id in users {
            self.round_profile(id)?;
        }
        Ok(())
    }

    /// Curve `id`'s sweep takes a round cross-section.
    fn round_profile(&mut self, id: ObjectId) -> Result<(), SceneError> {
        let mut curve = self.curve_of(id)?;
        if let CurveSolid::Sweep(sweep) = &mut curve.solid {
            sweep.profile = SweepProfile::Circle;
        }
        self.set_curve(id, curve)
    }

    /// A curve put in from elsewhere whose cross-section names an object
    /// that isn't a curve here takes a round one.
    pub(super) fn keep_usable_profile(&mut self, id: ObjectId) -> Result<(), SceneError> {
        let Some(solid) = self
            .scene
            .object(id)
            .and_then(|o| o.kind.curve())
            .map(|c| c.solid)
        else {
            return Ok(());
        };
        if self.scene.check_profile(id, &solid).is_err() {
            self.round_profile(id)?;
        }
        Ok(())
    }

    /// Points the copies' sweeps at the copies of their cross-sections,
    /// where those were copied too.
    pub(super) fn follow_copied_profiles(
        &mut self,
        copied: &[(ObjectId, ObjectId)],
    ) -> Result<(), SceneError> {
        for (_, copy) in copied {
            let Ok(mut curve) = self.curve_of(*copy) else {
                continue;
            };
            let Some(profile) = curve.solid.profile_object() else {
                continue;
            };
            let Some((_, new)) = copied.iter().find(|(original, _)| *original == profile) else {
                continue;
            };
            if let CurveSolid::Sweep(sweep) = &mut curve.solid {
                sweep.profile = SweepProfile::Curve(*new);
            }
            self.set_curve(*copy, curve)?;
        }
        Ok(())
    }
}

impl Run<'_> {
    /// Whether every object in `ids` that exists is a curve, and there are
    /// two or more.
    pub(super) fn all_curves(&self, ids: &[ObjectId]) -> bool {
        let found: Vec<_> = ids.iter().filter_map(|id| self.scene.object(*id)).collect();
        found.len() >= 2 && found.iter().all(|o| o.kind.curve().is_some())
    }

    /// Curve `id`'s points as `other`'s own space has them.
    fn curve_in_space_of(&self, id: ObjectId, other: ObjectId) -> Result<Curve, SceneError> {
        let matrix = self.scene.shape_matrix(other).inverse() * self.scene.shape_matrix(id);
        self.curve_of(id)?
            .shape
            .curve()
            .transformed(matrix)
            .map_err(|why| SceneError::Curve(curve_reason(why)))
    }

    /// Gives curve `id` the points `curve`.
    fn set_points(&mut self, id: ObjectId, curve: Curve) -> Result<(), SceneError> {
        let mut object = self.curve_of(id)?;
        object.shape = CurveShape::Points(curve);
        self.set_curve(id, object)
    }

    /// Deletes `others`, anything inside them taking their place.
    fn delete_joined(&mut self, others: &[ObjectId]) -> Result<(), SceneError> {
        for other in others {
            if !self.scene.contains(*other) {
                continue;
            }
            let place = self.place_of(*other)?;
            let children = self.scene.children(Some(*other)).to_vec();
            if !children.is_empty() {
                self.move_into(&children, place.parent, place.index)?;
            }
            self.command(&Command::Delete(vec![*other]))?;
        }
        Ok(())
    }

    /// See [`Command::JoinCurveEnds`].
    pub(super) fn join_curve_ends(
        &mut self,
        (a, a_point): (ObjectId, CurvePointId),
        (b, b_point): (ObjectId, CurvePointId),
        weld: bool,
    ) -> Result<Option<Vec<ObjectId>>, SceneError> {
        let not_ends = || SceneError::Curve(curve_reason(mesh::curve::CurveError::NotEnds));
        let first = self.curve_of(a)?.shape.curve();
        let a_end = first.end_of(a_point).ok_or_else(not_ends)?;
        if a == b {
            let b_end = first.end_of(b_point).ok_or_else(not_ends)?;
            if a_end == b_end {
                return Err(not_ends());
            }
            self.edit_curve(a, &CurveEdit::CloseEnds { weld })?;
            return Ok(Some(vec![a]));
        }
        let second = self.curve_in_space_of(b, a)?;
        let b_end = second.end_of(b_point).ok_or_else(not_ends)?;
        let joined = mesh::curve::join(&first, a_end, &second, b_end, weld)
            .map_err(|why| SceneError::Curve(curve_reason(why)))?;
        self.set_points(a, joined)?;
        self.delete_joined(&[b])?;
        Ok(Some(vec![a]))
    }

    /// Joins curve objects `ids` into the first, end to end: each time the
    /// curve whose end is nearest one of the growing curve's ends joins on
    /// there (melting ends on one spot). See [`Command::Join`].
    pub(super) fn join_curves(
        &mut self,
        ids: &[ObjectId],
    ) -> Result<Option<Vec<ObjectId>>, SceneError> {
        // In the order given: the first stays (existing() would give tree
        // order).
        existing(self.scene, ids)?;
        let mut ordered: Vec<ObjectId> = Vec::with_capacity(ids.len());
        for id in ids {
            if !ordered.contains(id) {
                ordered.push(*id);
            }
        }
        let Some((&target, others)) = ordered.split_first() else {
            return Ok(None);
        };
        let mut joined = self.curve_of(target)?.shape.curve();
        let mut left: Vec<(ObjectId, Curve)> = Vec::new();
        for id in others {
            left.push((*id, self.curve_in_space_of(*id, target)?));
        }
        if joined.closed() || left.iter().any(|(_, c)| c.closed()) {
            return Err(SceneError::Curve(CLOSED_HAS_NO_ENDS));
        }
        let ends = |c: &Curve| -> [(CurveEnd, glam::Vec3); 2] {
            let first = c.points().first().map_or(glam::Vec3::ZERO, |p| p.position);
            let last = c.points().last().map_or(glam::Vec3::ZERO, |p| p.position);
            [(CurveEnd::Start, first), (CurveEnd::End, last)]
        };
        while !left.is_empty() {
            let mut best: Option<(f32, usize, CurveEnd, CurveEnd)> = None;
            for (k, (_, curve)) in left.iter().enumerate() {
                for (mine, at) in ends(&joined) {
                    for (theirs, there) in ends(curve) {
                        let d = at.distance(there);
                        if best.is_none_or(|b| d < b.0) {
                            best = Some((d, k, mine, theirs));
                        }
                    }
                }
            }
            let Some((_, k, mine, theirs)) = best else {
                break;
            };
            let (_, curve) = left.remove(k);
            joined = mesh::curve::join(&joined, mine, &curve, theirs, false)
                .map_err(|why| SceneError::Curve(curve_reason(why)))?;
            if joined.closed() && !left.is_empty() {
                return Err(SceneError::Curve(CLOSED_HAS_NO_ENDS));
            }
        }
        self.set_points(target, joined)?;
        self.delete_joined(others)?;
        Ok(Some(vec![target]))
    }

    /// See [`Command::SplitCurve`].
    pub(super) fn split_curve(
        &mut self,
        id: ObjectId,
        points: &[CurvePointId],
    ) -> Result<Option<Vec<ObjectId>>, SceneError> {
        let curve = self.curve_of(id)?.shape.curve();
        let mut pieces = mesh::curve::split(&curve, points)
            .map_err(|why| SceneError::Curve(curve_reason(why)))?
            .into_iter();
        let Some(first) = pieces.next() else {
            return Ok(None);
        };
        if first == curve {
            return Ok(None);
        }
        let original = self
            .scene
            .object(id)
            .ok_or(SceneError::UnknownObject(id))?
            .clone();
        self.set_points(id, first)?;
        let mut made = vec![id];
        let mut place = self.place_of(id)?;
        for piece in pieces {
            let ObjectKind::Curve(mut kind) = original.kind.clone() else {
                break;
            };
            kind.shape = CurveShape::Points(piece);
            let mut object = Object::new(ObjectKind::Curve(kind));
            object.name = self.scene.unused_name(&original.name);
            object.transform = original.transform;
            object.offset = original.offset;
            object.visible = original.visible;
            object.materials = original.materials.clone();
            object.collision = original.collision;
            object.mirror = original.mirror;
            object.hardness = original.hardness;
            place = Place {
                parent: place.parent,
                index: place.index + 1,
            };
            made.push(self.insert(object, place)?);
        }
        Ok(Some(made))
    }
}

/// What an edit of a curve's points is called in the Edit menu.
pub(super) fn describe_edit(edit: &CurveEdit) -> String {
    match edit {
        CurveEdit::Transform { .. } => tr("Move curve points").to_string(),
        CurveEdit::Delete { .. } => tr("Delete curve points").to_string(),
        CurveEdit::SetKind { kind, .. } => match kind {
            PointKind::Sharp => tr("Make points sharp").to_string(),
            PointKind::Smooth => tr("Make points smooth").to_string(),
            PointKind::Broken => tr("Break points' handles").to_string(),
            PointKind::Auto => tr("Give points auto handles").to_string(),
        },
        CurveEdit::SetClosed(true) => tr("Close curve").to_string(),
        CurveEdit::SetClosed(false) => tr("Open curve").to_string(),
        CurveEdit::Insert { .. } => tr("Add curve point").to_string(),
        CurveEdit::Extend { points, .. } => match points.len() {
            1 => tr("Carry on the curve by 1 point").to_string(),
            n => trf!("Carry on the curve by {} points", n),
        },
        CurveEdit::Reverse => tr("Reverse curve").to_string(),
        CurveEdit::RoundCorners { .. } => tr("Round corners").to_string(),
        CurveEdit::Resample { count } => trf!("Space {} points evenly", count),
        CurveEdit::CloseEnds { weld: true } => tr("Weld curve ends").to_string(),
        CurveEdit::CloseEnds { weld: false } => tr("Connect curve ends").to_string(),
    }
}

/// A curve error's reason, as text for the language layer.
fn curve_reason(why: mesh::curve::CurveError) -> &'static str {
    use mesh::curve::CurveError;
    match why {
        CurveError::NotANumber => {
            words::text!("a curve point isn't a number, or is more than 10 km away")
        }
        CurveError::TooManyPoints => words::text!("a curve can have at most 4096 points"),
        CurveError::TooFewPoints => {
            words::text!("a curve needs at least two points: delete the whole object instead")
        }
        CurveError::BadIds => words::text!("a curve's points have repeated or impossible ids"),
        CurveError::LoopTooSmall => words::text!("a closed curve needs at least three points"),
        CurveError::OutOfIds => words::text!(
            "this curve has had 65536 points made over time, the most it can: draw it again"
        ),
        CurveError::NotEnds => words::text!("that works on the two ends of open curves"),
    }
}
