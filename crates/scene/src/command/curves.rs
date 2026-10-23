//! The commands that change curves
//! ([decision 0138](../../../../docs/decisions/0138-curves-sweep-lathe.md)).

use super::{Run, existing};
use crate::curve::{CurveObject, CurveShape, CurveSolid, NOT_A_CURVE, SweepProfile};
use crate::object::{ObjectId, ObjectKind};
use crate::tree::{Edit, SceneError};
use mesh::curve::{CurveEdit, ReadyCurve};

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
    }
}
