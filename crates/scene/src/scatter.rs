//! The Scatter modifier ([decision 0146](../../../docs/decisions/0146-scatter.md)):
//! copies of other shapes spread over an object's surface.
//!
//! The shapes scattered stay objects of their own, the modifier's
//! *sources* (decision 0142), read through the stack as their final shapes
//! (hidden or not), so editing a rock changes every copy. Deleting one
//! takes it out of the scatter, and deleting the last removes the
//! modifier, in the same undo step; copies made together with their
//! shapes scatter the copied shapes; a shape can't be scattered onto
//! itself, nor onto a shape made from it (a loop).
//!
//! The work is `mesh::scatter`: the surface with the copies added as one
//! mesh, so exports get it as one mesh (glTF's GPU instancing extension
//! isn't read by Godot, decision 0146). Each copy's materials are matched
//! to the surface's by name, as a cutter's are.

use crate::modifier::{ModifierContext, SourceData};
use crate::object::ObjectId;
use mesh::scatter::{ScatterOptions, ScatterShape};
use mesh::{FaceId, Mesh};
use std::collections::BTreeSet;
use std::sync::Arc;
use words::tr;

/// The most shapes one Scatter modifier scatters.
pub const MAX_SCATTER_SHAPES: usize = 64;

/// The Scatter modifier's settings: what it scatters, where, and how.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ScatterSettings {
    /// Where and how the copies go.
    pub options: ScatterOptions,
    /// The shapes scattered (each copy is one of them, picked at random).
    pub shapes: Vec<ObjectId>,
    /// The surface's polygons copies go on, by id (from the shape as
    /// modelled); empty for the whole surface.
    pub polygons: Vec<FaceId>,
}

impl ScatterSettings {
    /// Scattering `shapes` with the standard settings over the whole
    /// surface.
    pub fn of(shapes: Vec<ObjectId>) -> Self {
        Self {
            shapes,
            ..Self::default()
        }
        .clamped()
    }

    /// The settings brought within limits: shapes listed once each (at
    /// most [`MAX_SCATTER_SHAPES`]), polygons listed once each in order.
    pub fn clamped(&self) -> Self {
        let mut shapes: Vec<ObjectId> = Vec::new();
        for id in &self.shapes {
            if !shapes.contains(id) && shapes.len() < MAX_SCATTER_SHAPES {
                shapes.push(*id);
            }
        }
        let polygons: BTreeSet<FaceId> = self.polygons.iter().copied().collect();
        Self {
            options: self.options.clamped(),
            shapes,
            polygons: polygons.into_iter().collect(),
        }
    }
}

/// Where the Scatter modifier's work is: the polygons it would add (an
/// upper bound) over about 8, as appending a polygon costs about an eighth
/// of one in a boolean.
pub(crate) fn scatter_work(
    input: &Mesh,
    settings: &ScatterSettings,
    sources: &[crate::modifier::SourceShape],
) -> usize {
    let sizes: Vec<usize> = sources
        .iter()
        .filter_map(|s| match &s.data {
            SourceData::Shape(mesh) => Some(mesh.face_count()),
            _ => None,
        })
        .collect();
    mesh::scatter::polygons_added(input, &settings.options, &sizes) / 8
}

/// `input` (in the object's space) with the shapes in `context` scattered
/// over it.
pub(crate) fn apply_scatter(
    input: &Mesh,
    settings: &ScatterSettings,
    context: &ModifierContext,
) -> Result<Mesh, String> {
    // Each shape in the settings' order, its materials renumbered into
    // the surface's, its pivot moved to the origin.
    let mut shapes: Vec<(Arc<Mesh>, glam::Mat4)> = Vec::new();
    for id in &settings.shapes {
        let Some(source) = context.sources.iter().find(|s| s.id == *id) else {
            continue;
        };
        let SourceData::Shape(mesh) = &source.data else {
            continue;
        };
        let mesh = renumbered(mesh, &source.materials);
        let matrix = glam::Mat4::from_translation(-source.pivot) * source.matrix;
        shapes.push((mesh, matrix));
    }
    if shapes.is_empty() {
        return Err(tr("its shapes to scatter aren't there").to_string());
    }
    let shapes: Vec<ScatterShape<'_>> = shapes
        .iter()
        .map(|(mesh, matrix)| ScatterShape {
            mesh,
            matrix: *matrix,
        })
        .collect();
    let only: Option<BTreeSet<FaceId>> =
        (!settings.polygons.is_empty()).then(|| settings.polygons.iter().copied().collect());
    mesh::scatter::scatter(
        input,
        only.as_ref(),
        &shapes,
        &settings.options,
        crate::MAX_SHAPE_POLYGONS,
    )
    .map_err(|why| why.to_string())
}

/// `mesh` with its material slots renumbered: slot `i` becomes
/// `materials[i]` (unchanged when `materials` keeps every slot).
fn renumbered(mesh: &Arc<Mesh>, materials: &[u32]) -> Arc<Mesh> {
    if materials.iter().enumerate().all(|(i, n)| i as u32 == *n) {
        return Arc::clone(mesh);
    }
    let mut changed = (**mesh).clone();
    for (old, new) in materials.iter().enumerate() {
        if old as u32 != *new {
            let faces = mesh::material::faces_with_slot(mesh, old as u32);
            mesh::material::set_material(&mut changed, &faces, *new);
        }
    }
    Arc::new(changed)
}
