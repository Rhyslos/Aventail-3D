//! One level of subdivision whose points can be worked out again
//! ([decision 0315](../../../../docs/decisions/0315-detail-levels.md)): what a
//! sculpt's detail levels are made of.
//!
//! [`refine`] subdivides a shape once, as [`super::subdivide`] does, but
//! every point keeps its number: the finer shape's first points are the
//! coarser shape's, slot for slot (a slot with no point stays empty, a
//! loose point stays where it is), then come a point for each edge and one
//! for each polygon. So a value kept per point of a level (a sculpt's
//! moves, its mask) names the same point on every finer level.
//!
//! The [`Refinement`] it gives back remembers how the finer points hang on
//! the coarser ones. When the coarser shape's points move (a stroke on a
//! lower detail level), [`Refinement::places`] works the finer points out
//! again with the very sums [`refine`] used, to the last bit, without
//! building the polygons again.

use super::step::{Edges, Layout, stepped_values};
use super::{Cage, SubdivideProblem};
use crate::Mesh;
use glam::Vec3;

/// How one level's points hang on the level below's (see the module
/// notes).
#[derive(Debug, Clone)]
pub struct Refinement {
    /// The coarser shape's polygons, corner by corner (each a point slot).
    layout: Layout,
    /// Its edges.
    edges: Edges,
    /// How many point slots it has.
    slots: usize,
}

/// `mesh` subdivided once (Catmull-Clark, full creases and open borders
/// kept sharp, as [`super::subdivide`] with one level and hard edges not
/// counted), every point keeping its number, and how the new points hang
/// on the old. Refused when a point isn't at a real position.
pub fn refine(mesh: &Mesh) -> Result<(Mesh, Refinement), SubdivideProblem> {
    let cage = Cage::of(mesh, 1, false, true)?;
    let slots = cage.points.len();
    let (refined, layout, edges) = cage.refined_keeping();
    let fine = refined.into_mesh(mesh);
    Ok((
        fine,
        Refinement {
            layout,
            edges,
            slots,
        },
    ))
}

impl Refinement {
    /// How many point slots the coarser shape has.
    pub fn coarse_slots(&self) -> usize {
        self.slots
    }

    /// How many point slots the finer shape has: the coarser one's, one
    /// for each of its edges and one for each of its polygons.
    pub fn fine_slots(&self) -> usize {
        self.slots + self.edges.len() + self.layout.face_count()
    }

    /// Where the finer shape's points are when the coarser shape's are at
    /// `coarse` (by slot; what a slot with no point holds doesn't
    /// matter): exactly what [`refine`] would give. `None` when `coarse`
    /// isn't one place per slot.
    pub fn places(&self, coarse: &[Vec3]) -> Option<Vec<Vec3>> {
        (coarse.len() == self.slots).then(|| stepped_values(coarse, &self.layout, &self.edges, true))
    }

    /// [`Self::places`] for the points of `coarse` as they are now.
    /// `None` when it hasn't the point slots this was made for.
    pub fn places_of(&self, coarse: &Mesh) -> Option<Vec<Vec3>> {
        if coarse.point_slot_count() != self.slots {
            return None;
        }
        let places: Vec<Vec3> = coarse
            .point_slots()
            .map(|p| p.unwrap_or(Vec3::ZERO))
            .collect();
        self.places(&places)
    }

    /// A value per point of the coarser shape carried to the finer one's
    /// points without smoothing: each old point keeps its own, a point on
    /// an edge takes the middle of its ends', a polygon's point the
    /// average of its corners'. `None` when `coarse` isn't one value per
    /// slot.
    pub fn carried(&self, coarse: &[f32]) -> Option<Vec<f32>> {
        if coarse.len() != self.slots {
            return None;
        }
        let mut fine = Vec::with_capacity(self.fine_slots());
        fine.extend_from_slice(coarse);
        for e in 0..self.edges.len() {
            let [a, b] = self.edges.ends(e);
            fine.push((coarse[a as usize] + coarse[b as usize]) * 0.5);
        }
        for f in 0..self.layout.face_count() {
            let corners = &self.layout.corners[self.layout.range(f)];
            let sum: f32 = corners.iter().map(|c| coarse[*c as usize]).sum();
            fine.push(sum / corners.len().max(1) as f32);
        }
        Some(fine)
    }

    /// Roughly how many bytes it holds.
    pub fn memory_size(&self) -> usize {
        (self.layout.starts.len() + self.layout.corners.len()) * 4 + self.edges.memory_size()
    }
}
