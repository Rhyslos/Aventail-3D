//! The two modifiers that lay a shape onto another object
//! ([decision 0147](../../../docs/decisions/0147-curve-deform-and-shrinkwrap.md)):
//! **Bend along curve**, which bends the shape along a curve object, and
//! **Shrinkwrap**, which moves its points onto a target shape's surface.
//!
//! Each reads its other object through the stack's sources (decision
//! 0142): a curve's points (not its solid), or a target's final shape
//! (hidden or not), placed into the shape's space. So the curve and the
//! target stay live: reshape or move either and the shape follows.
//! Deleting the curve or the target removes the modifier in the same undo
//! step; copies made together with their curve or target follow the copy;
//! loops (a shape wrapped onto a shape made from it) are refused.
//!
//! The work is in `mesh`: [`mesh::curve::bend`] and [`mesh::shrinkwrap`].
//! Both are deformers: only the points move, so UVs, seams and materials
//! stay as they were.

use crate::modifier::{ModifierContext, SourceData};
use crate::object::ObjectId;
use mesh::curve::bend::{BendOptions, bend_along};
use mesh::shrinkwrap::{ShrinkwrapOptions, shrinkwrap};
use mesh::{Mesh, VertexId};
use std::collections::BTreeSet;
use words::{text, tr};

/// No object yet: the standard settings name none, which no shape can
/// take (Bend along curve and Shrinkwrap onto name theirs).
const NOTHING: ObjectId = ObjectId(u64::MAX);

/// The Bend along curve modifier's settings: the curve, and how the shape
/// lies along it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurveBendSettings {
    /// How the shape lies along the curve.
    pub options: BendOptions,
    /// The curve object it bends along.
    pub curve: ObjectId,
}

impl Default for CurveBendSettings {
    fn default() -> Self {
        Self {
            options: BendOptions::default(),
            curve: NOTHING,
        }
    }
}

impl CurveBendSettings {
    /// Bending along `curve` with the standard settings.
    pub fn of(curve: ObjectId) -> Self {
        Self {
            curve,
            ..Self::default()
        }
    }

    /// The settings within limits.
    pub fn clamped(&self) -> Self {
        Self {
            options: self.options.clamped(),
            curve: self.curve,
        }
    }
}

/// The Shrinkwrap modifier's settings: the target, how points find its
/// surface, and which points move.
#[derive(Debug, Clone, PartialEq)]
pub struct ShrinkwrapSettings {
    /// How the points find the surface, and how far off it they end up.
    pub options: ShrinkwrapOptions,
    /// The shape wrapped onto.
    pub target: ObjectId,
    /// The points that move, by id (from the shape as modelled), each
    /// once, in order; empty for every point.
    pub points: Vec<VertexId>,
}

impl Default for ShrinkwrapSettings {
    fn default() -> Self {
        Self {
            options: ShrinkwrapOptions::default(),
            target: NOTHING,
            points: Vec::new(),
        }
    }
}

impl ShrinkwrapSettings {
    /// Wrapping every point onto `target` with the standard settings.
    pub fn of(target: ObjectId) -> Self {
        Self {
            target,
            ..Self::default()
        }
    }

    /// The settings within limits: points listed once each, in order.
    pub fn clamped(&self) -> Self {
        let points: BTreeSet<VertexId> = self.points.iter().copied().collect();
        Self {
            options: self.options.clamped(),
            target: self.target,
            points: points.into_iter().collect(),
        }
    }
}

/// Why Bend along curve has nothing to follow.
const NO_CURVE: &str = text!("its curve isn't there");
/// Why Shrinkwrap has nothing to wrap onto.
const NO_TARGET: &str = text!("its target has no shape: give it polygons, or pick another");

/// How much work bending is: about 20 points cost as much as one polygon
/// in a boolean, as for a lattice.
pub(crate) fn bend_work(input: &Mesh) -> usize {
    input.vertex_count() / 20
}

/// How much work wrapping is, in polygons of a boolean's worth: building
/// the tree round the target's triangles (about 0.6 microseconds each, so
/// a polygon of the target is about a quarter of one) and finding each
/// point's place (2 to 3 microseconds near the surface: about one each).
pub(crate) fn shrinkwrap_work(input: &Mesh, sources: &[crate::modifier::SourceShape]) -> usize {
    let target: usize = sources
        .iter()
        .filter_map(|s| match &s.data {
            SourceData::Shape(mesh) => Some(mesh.face_count()),
            _ => None,
        })
        .sum();
    target / 4 + input.vertex_count()
}

/// `input` (in the object's space) bent along the curve in `context`.
pub(crate) fn apply_bend(
    input: &Mesh,
    settings: &CurveBendSettings,
    context: &ModifierContext,
) -> Result<Mesh, String> {
    let source = context
        .sources
        .iter()
        .find(|s| s.id == settings.curve)
        .ok_or_else(|| tr(NO_CURVE).to_string())?;
    let SourceData::Curve(curve) = &source.data else {
        return Err(tr(NO_CURVE).to_string());
    };
    let curve = curve.transformed(source.matrix).map_err(|_| {
        tr("the curve can't be placed: it is flattened, or too far away").to_string()
    })?;
    bend_along(input, &curve, context.pivot, &settings.options).map_err(|why| why.to_string())
}

/// `input` (in the object's space) wrapped onto the target in `context`.
pub(crate) fn apply_shrinkwrap(
    input: &Mesh,
    settings: &ShrinkwrapSettings,
    context: &ModifierContext,
) -> Result<Mesh, String> {
    let source = context
        .sources
        .iter()
        .find(|s| s.id == settings.target)
        .ok_or_else(|| tr(NO_TARGET).to_string())?;
    let SourceData::Shape(target) = &source.data else {
        return Err(tr(NO_TARGET).to_string());
    };
    let only: Option<BTreeSet<VertexId>> =
        (!settings.points.is_empty()).then(|| settings.points.iter().copied().collect());
    if let Some(only) = &only
        && !only.iter().any(|v| input.position(*v).is_some())
    {
        return Err(
            tr("none of its chosen points are left: choose them again, or use all points")
                .to_string(),
        );
    }
    let tree = mesh::bvh::TriangleTree::of_mesh(target, source.matrix);
    shrinkwrap(input, &tree, only.as_ref(), &settings.options).map_err(|why| why.to_string())
}
