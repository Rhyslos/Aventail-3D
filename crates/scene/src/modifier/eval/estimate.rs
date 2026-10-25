//! Keeping the work estimate of an object's modifiers while nothing
//! changes, so asking for it every frame while heavy work is under way on
//! another thread costs nothing.

use super::{Planned, StackCache, Step, last_shape};
use crate::modifier::{ModifierKind, Purpose, SourceShape};
use crate::object::ObjectId;
use mesh::Mesh;
use std::sync::Arc;

/// What a planned modifier's work estimate depends on.
#[derive(Debug, Clone, PartialEq)]
struct WorkKey {
    kind: ModifierKind,
    sources: Vec<SourceShape>,
    pending: Vec<(usize, ObjectId)>,
    hardness: mesh::Hardness,
    pivot: glam::Vec3,
}

/// The work estimate last made for an object's modifiers, kept so asking
/// again every frame while heavy work is under way elsewhere costs
/// nothing ([`StackJob::work`]).
#[derive(Debug, Clone)]
pub(super) struct Estimate {
    /// The shape they start from (not kept alive by this).
    input: std::sync::Weak<Mesh>,
    /// How many modifiers were kept from before.
    done: usize,
    plan: Vec<WorkKey>,
    /// The work of `plan` itself, without its pending sources' (those
    /// keep their own).
    work: usize,
}

impl Planned {
    /// What its estimate depends on, to keep with it ([`Estimate`]).
    fn work_key(&self) -> WorkKey {
        WorkKey {
            kind: self.kind.clone(),
            sources: self.sources.clone(),
            pending: self.pending.iter().map(|p| (p.at, p.job.id)).collect(),
            hardness: self.hardness,
            pivot: self.pivot,
        }
    }
}

impl StackCache {
    /// The work of `plan` on `input` after `done`, kept from the last time
    /// it was asked for when nothing has changed since, else worked out
    /// and kept.
    pub(super) fn plan_work(
        &self,
        key: (ObjectId, Purpose),
        input: &Arc<Mesh>,
        done: &[Step],
        plan: &[Planned],
    ) -> usize {
        let mut estimates = self
            .estimates
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let keys: Vec<WorkKey> = plan.iter().map(Planned::work_key).collect();
        if let Some(kept) = estimates.get(&key)
            && std::ptr::eq(kept.input.as_ptr(), Arc::as_ptr(input))
            && kept.done == done.len()
            && kept.plan == keys
        {
            return kept.work;
        }
        let start = last_shape(input, done);
        let work = plan
            .iter()
            .map(|p| p.kind.work_with(&start, key.1, &p.sources))
            .fold(0, usize::saturating_add);
        estimates.insert(
            key,
            Estimate {
                input: Arc::downgrade(input),
                done: done.len(),
                plan: keys,
                work,
            },
        );
        work
    }
}
