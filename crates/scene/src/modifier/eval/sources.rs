//! The other objects a modifier reads, gathered without working out
//! their own modifiers on the asking thread (a source not worked out yet
//! is worked out by the job that reads it).

use super::{StackJob, StackState};
use crate::modifier::{ModifierKind, Purpose, SourceData, SourceShape};
use crate::object::ObjectId;
use crate::tree::Scene;
use std::sync::Arc;

/// A source whose modifiers are still to be worked out, and where its
/// shape goes in [`Planned::sources`].
#[derive(Debug, Clone)]
pub(super) struct PendingSource {
    pub(super) at: usize,
    pub(super) job: StackJob,
}

impl Scene {
    /// The shapes `kind` reads for object `id`, each placed into `id`'s
    /// space; a source without a shape is left out. Never works a source's
    /// modifiers out here: one whose result isn't kept comes with its
    /// shape before them and the job to work them out, for the job of
    /// `id` to do first.
    pub(super) fn source_shapes(
        &self,
        id: ObjectId,
        kind: &ModifierKind,
        purpose: Purpose,
    ) -> (Vec<SourceShape>, Vec<PendingSource>) {
        let into = self.shape_matrix(id);
        let back = if into.determinant().abs() >= 1e-12 {
            into.inverse()
        } else {
            glam::Mat4::IDENTITY
        };
        let names = self.material_list(id);
        let mut sources: Vec<SourceShape> = Vec::new();
        let mut pending: Vec<PendingSource> = Vec::new();
        for source in kind.sources().into_iter().filter(|source| *source != id) {
            let Some(object) = self.object(source) else {
                continue;
            };
            // A lattice's cage is read as its points (decision 0144), a
            // curve bent along as its line (decision 0147); anything else
            // as its final shape.
            let data = match &object.kind {
                // An array's centre: only where it stands (decision 0153).
                _ if matches!(kind, ModifierKind::Array(_)) => SourceData::Place,
                crate::ObjectKind::Lattice(cage) => SourceData::Cage(Arc::new(cage.clone())),
                crate::ObjectKind::Curve(curve)
                    if matches!(kind, ModifierKind::BendAlongCurve(_)) =>
                {
                    SourceData::Curve(Arc::new(curve.shape.curve()))
                }
                _ => {
                    let Some(input) = self.unmodified_shape(source) else {
                        continue;
                    };
                    match self.stack_state(source, purpose, Arc::clone(&input)) {
                        None => SourceData::Shape(input),
                        Some(StackState::Ready(result)) => SourceData::Shape(result.mesh),
                        Some(StackState::Needed(job)) => {
                            pending.push(PendingSource {
                                at: sources.len(),
                                job,
                            });
                            SourceData::Shape(input)
                        }
                    }
                }
            };
            let materials = self
                .material_list(source)
                .iter()
                .map(|m| names.iter().position(|n| n.name == m.name).unwrap_or(0) as u32)
                .collect();
            sources.push(SourceShape {
                id: source,
                data,
                matrix: back * self.shape_matrix(source),
                pivot: back
                    .transform_point3(self.world_matrix(source).transform_point3(glam::Vec3::ZERO)),
                materials,
            });
        }
        (sources, pending)
    }
}
