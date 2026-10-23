//! Live booleans ([decision 0135](../../../docs/decisions/0135-live-booleans.md)):
//! an object can be cut by other objects, its *cutters*, which stay
//! objects of their own. The kept object lists its booleans (each a
//! kind and the cutters that do it, in order); the cut is worked out when
//! its shape is shown, from the cutters' shapes and places at that
//! moment, so moving or editing a cutter moves the cut.
//!
//! The kept object's own edits still work on its uncut shape
//! ([`Scene::evaluated`]); [`Scene::shown_shape`] is the cut one, with
//! its live mirror after the cut.

use crate::object::{ObjectId, ObjectKind};
use crate::tree::Scene;
use glam::Mat4;
use mesh::Mesh;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub use booleans::Kind as BooleanKind;

/// The most cutters one object can have, across its booleans.
pub const MAX_CUTTERS: usize = 64;

/// One boolean on an object: what it does, and the objects that do it
/// (each in turn, in order).
#[derive(Debug, Clone, PartialEq)]
pub struct Boolean {
    /// Union, Subtract or Intersect.
    pub kind: BooleanKind,
    /// The cutters. Never empty in a scene.
    pub cutters: Vec<ObjectId>,
}

/// Why an object's booleans can't be set up, in plain words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BooleanRefusal {
    /// No cutter was given.
    NoCutter,
    /// An object can't cut itself.
    CutsItself,
    /// The cutter's shape already depends on the kept object (it is cut
    /// by it, or shows it): the two would wait on each other forever.
    Loop,
    /// A group has no shape to cut with or be cut.
    NotAShape,
    /// An instance shows its original's shape and has no booleans of its
    /// own.
    Instance,
    /// More than [`MAX_CUTTERS`] cutters.
    TooMany,
}

impl std::fmt::Display for BooleanRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NoCutter => "select the shape to keep, then Ctrl + click the cutters",
            Self::CutsItself => "a shape can't cut itself",
            Self::Loop => {
                "the cutter is already cut by this shape (or shows it), so each would wait for \
                 the other"
            }
            Self::NotAShape => "a group has no shape: pick shapes",
            Self::Instance => {
                "it's an instance: give its original the boolean, or bake it into a shape first"
            }
            Self::TooMany => "a shape can have at most 64 cutters",
        })
    }
}

/// Why a boolean couldn't be worked out: the kept object then shows its
/// shape uncut.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BooleanProblem {
    /// The kept shape isn't a closed solid (or another problem with it).
    Kept(booleans::Problem),
    /// A cutter isn't a closed solid, or is squashed flat.
    Cutter(ObjectId, booleans::Problem),
    /// The objects cut each other in a loop (only in a damaged file).
    Loop,
}

/// A result worked out before, with what it was worked out from.
#[derive(Clone)]
struct Cut {
    kept: Arc<Mesh>,
    cutters: Vec<(BooleanKind, Arc<Mesh>, Mat4)>,
    seam_angle: f32,
    result: Result<Arc<Mesh>, booleans::Problem>,
}

/// The booleans' results, by kept object (not part of what the scene is:
/// copies start empty, and it never affects equality).
#[derive(Default)]
pub(crate) struct CutCache(Mutex<HashMap<ObjectId, Cut>>);

impl CutCache {
    /// Forgets the result for `id` (deleted).
    pub(crate) fn forget(&mut self, id: ObjectId) {
        self.0
            .get_mut()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&id);
    }
}

impl Clone for CutCache {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl PartialEq for CutCache {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl std::fmt::Debug for CutCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CutCache")
    }
}

/// Whether two shapes are the same: the same one, or equal.
fn same(a: &Arc<Mesh>, b: &Arc<Mesh>) -> bool {
    Arc::ptr_eq(a, b) || **a == **b
}

impl Cut {
    fn made_from(
        &self,
        kept: &Arc<Mesh>,
        cutters: &[(BooleanKind, Arc<Mesh>, Mat4)],
        seam_angle: f32,
    ) -> bool {
        self.seam_angle == seam_angle
            && same(&self.kept, kept)
            && self.cutters.len() == cutters.len()
            && self
                .cutters
                .iter()
                .zip(cutters)
                .all(|(a, b)| a.0 == b.0 && a.2 == b.2 && same(&a.1, &b.1))
    }
}

impl Scene {
    /// The object's booleans (empty for none, or an unknown id).
    pub fn booleans(&self, id: ObjectId) -> &[Boolean] {
        self.object(id).map_or(&[], |o| o.booleans.as_slice())
    }

    /// The objects `cutter` cuts, top to bottom.
    pub fn cut_by(&self, cutter: ObjectId) -> Vec<ObjectId> {
        self.walk()
            .into_iter()
            .map(|(id, _)| id)
            .filter(|id| {
                self.booleans(*id)
                    .iter()
                    .any(|b| b.cutters.contains(&cutter))
            })
            .collect()
    }

    /// Every object that cuts another: drawn as outlines and left out of
    /// exports (the shapes they cut carry the result).
    pub fn cutters(&self) -> std::collections::BTreeSet<ObjectId> {
        self.walk()
            .into_iter()
            .flat_map(|(id, _)| self.booleans(id).iter().flat_map(|b| b.cutters.clone()))
            .collect()
    }

    /// Whether `id` cuts any object: it is then drawn as an outline.
    pub fn is_cutter(&self, id: ObjectId) -> bool {
        !self.cut_by(id).is_empty()
    }

    /// The objects whose shapes `id`'s shape is made from: its cutters,
    /// or an instance's original.
    fn shape_sources(&self, id: ObjectId) -> Vec<ObjectId> {
        let Some(object) = self.object(id) else {
            return Vec::new();
        };
        let mut out: Vec<ObjectId> = object
            .booleans
            .iter()
            .flat_map(|b| b.cutters.iter().copied())
            .collect();
        if let ObjectKind::Instance(source) = object.kind {
            out.push(source);
        }
        out
    }

    /// Whether `id`'s shape is made, at any depth, from `other`'s (or is
    /// it).
    pub fn shape_uses(&self, id: ObjectId, other: ObjectId) -> bool {
        let mut seen = Vec::new();
        let mut waiting = vec![id];
        while let Some(at) = waiting.pop() {
            if at == other {
                return true;
            }
            // Bounded: each object is looked at once.
            if seen.contains(&at) {
                continue;
            }
            seen.push(at);
            waiting.extend(self.shape_sources(at));
        }
        false
    }

    /// Whether `booleans` would make sense on `id`: a shape, not an
    /// instance, its cutters shapes other than itself that don't depend
    /// on it, at most [`MAX_CUTTERS`] of them.
    pub fn check_booleans(&self, id: ObjectId, booleans: &[Boolean]) -> Result<(), BooleanRefusal> {
        if booleans.is_empty() {
            return Ok(());
        }
        let object = self.object(id).ok_or(BooleanRefusal::NotAShape)?;
        match object.kind {
            ObjectKind::Group => return Err(BooleanRefusal::NotAShape),
            ObjectKind::Instance(_) => return Err(BooleanRefusal::Instance),
            _ => {}
        }
        let mut count = 0;
        for boolean in booleans {
            if boolean.cutters.is_empty() {
                return Err(BooleanRefusal::NoCutter);
            }
            for cutter in &boolean.cutters {
                count += 1;
                if *cutter == id {
                    return Err(BooleanRefusal::CutsItself);
                }
                if !self.object(*cutter).is_some_and(|o| o.kind.has_shape()) {
                    return Err(BooleanRefusal::NotAShape);
                }
                if self.shape_uses(*cutter, id) {
                    return Err(BooleanRefusal::Loop);
                }
            }
        }
        if count > MAX_CUTTERS {
            return Err(BooleanRefusal::TooMany);
        }
        Ok(())
    }

    /// The object's cut shape, in its own space: its final shape
    /// ([`Scene::evaluated`]) with its booleans applied, or why that
    /// couldn't be done. `None` for an object without booleans (an
    /// instance has its original's). Worked out again only when the
    /// shapes, places or kinds involved change.
    pub fn cut_shape(&self, id: ObjectId) -> Option<Result<Arc<Mesh>, BooleanProblem>> {
        self.cut_shape_from(id, &mut Vec::new())
    }

    /// Why the object's booleans can't be worked out, in plain words,
    /// naming the cutter; `None` when they can (or it has none).
    pub fn boolean_problem(&self, id: ObjectId) -> Option<String> {
        let name = |id: ObjectId| {
            self.object(id)
                .map_or_else(|| id.to_string(), |o| o.name.clone())
        };
        match self.cut_shape(id)? {
            Ok(_) => None,
            Err(BooleanProblem::Kept(problem)) => Some(capitalised(&problem.to_string())),
            Err(BooleanProblem::Cutter(cutter, problem)) => {
                let reason = match problem {
                    booleans::Problem::CutterNotClosed(_) => {
                        "isn't a closed solid: it has holes in its surface, or edges shared by \
                         more than two polygons"
                            .to_string()
                    }
                    booleans::Problem::CutterFlat(_) => "is squashed flat".to_string(),
                    other => format!("can't be used: {other}"),
                };
                Some(format!("The cutter {} {reason}", name(cutter)))
            }
            Err(BooleanProblem::Loop) => Some("The shapes cut each other in a loop".to_string()),
        }
    }

    /// The object's whole shape as other objects see it when it cuts
    /// them: cut by its own booleans (uncut if they fail), with its live
    /// mirror's copies.
    fn cutting_shape(&self, id: ObjectId, visiting: &mut Vec<ObjectId>) -> Option<Arc<Mesh>> {
        let shape = match self.cut_shape_from(id, visiting) {
            Some(Ok(cut)) => cut,
            _ => self.evaluated(id)?.mesh,
        };
        Some(self.with_mirror(id, &shape).map_or(shape, Arc::new))
    }

    fn cut_shape_from(
        &self,
        id: ObjectId,
        visiting: &mut Vec<ObjectId>,
    ) -> Option<Result<Arc<Mesh>, BooleanProblem>> {
        let object = self.object(id)?;
        if let ObjectKind::Instance(source) = object.kind {
            return self
                .object(source)
                .filter(|o| !matches!(o.kind, ObjectKind::Instance(_)))
                .and_then(|_| self.cut_shape_from(source, visiting));
        }
        if object.booleans.is_empty() {
            return None;
        }
        if visiting.contains(&id) {
            return Some(Err(BooleanProblem::Loop));
        }
        let kept = self.evaluated(id)?.mesh;
        visiting.push(id);
        let into = self.shape_matrix(id);
        let back = (into.determinant().abs() >= 1e-12).then(|| into.inverse());
        let names = self.material_list(id);
        let mut cutters: Vec<(BooleanKind, Arc<Mesh>, Mat4, ObjectId)> = Vec::new();
        for boolean in &object.booleans {
            for cutter in &boolean.cutters {
                if visiting.contains(cutter) {
                    visiting.pop();
                    return Some(Err(BooleanProblem::Loop));
                }
                let Some(shape) = self.cutting_shape(*cutter, visiting) else {
                    continue;
                };
                let shape = self.with_materials_of(&names, *cutter, shape);
                let matrix = back.unwrap_or(Mat4::IDENTITY) * self.shape_matrix(*cutter);
                cutters.push((boolean.kind, shape, matrix, *cutter));
            }
        }
        visiting.pop();
        let seam_angle = match object.hardness {
            mesh::Hardness::ByAngle(angle) => angle,
            mesh::Hardness::Marked => crate::STANDARD_SMOOTHING_ANGLE,
        };
        let result = self.cut_cached(id, &kept, &cutters, seam_angle);
        Some(result.map_err(|problem| match problem {
            booleans::Problem::CutterNotClosed(i) | booleans::Problem::CutterFlat(i) => {
                let cutter = cutters.get(i).map_or(id, |c| c.3);
                BooleanProblem::Cutter(cutter, problem)
            }
            other => BooleanProblem::Kept(other),
        }))
    }

    /// The boolean worked out, or taken from the cache when nothing it
    /// depends on has changed.
    fn cut_cached(
        &self,
        id: ObjectId,
        kept: &Arc<Mesh>,
        cutters: &[(BooleanKind, Arc<Mesh>, Mat4, ObjectId)],
        seam_angle: f32,
    ) -> Result<Arc<Mesh>, booleans::Problem> {
        let key: Vec<(BooleanKind, Arc<Mesh>, Mat4)> = cutters
            .iter()
            .map(|(k, m, t, _)| (*k, Arc::clone(m), *t))
            .collect();
        let cache = &self.cuts.0;
        if let Some(cut) = cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&id)
            .filter(|c| c.made_from(kept, &key, seam_angle))
        {
            return cut.result.clone();
        }
        let list: Vec<booleans::Cutter> = key
            .iter()
            .map(|(kind, mesh, matrix)| booleans::Cutter {
                kind: *kind,
                mesh,
                matrix: *matrix,
            })
            .collect();
        let options = booleans::Options {
            seam_angle,
            max_polygons: crate::MAX_SHAPE_POLYGONS,
        };
        let result = booleans::apply(kept, &list, &options).map(Arc::new);
        cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(
                id,
                Cut {
                    kept: Arc::clone(kept),
                    cutters: key,
                    seam_angle,
                    result: result.clone(),
                },
            );
        result
    }

    /// `shape` (the cutter's) with its material slots renumbered into the
    /// kept object's list `names`: by name, or the first material where
    /// the kept object has no material of that name.
    fn with_materials_of(
        &self,
        names: &[crate::Material],
        cutter: ObjectId,
        shape: Arc<Mesh>,
    ) -> Arc<Mesh> {
        let own = self.material_list(cutter);
        let renumber: Vec<u32> = own
            .iter()
            .map(|m| names.iter().position(|n| n.name == m.name).unwrap_or(0) as u32)
            .collect();
        if renumber.iter().enumerate().all(|(i, n)| i as u32 == *n) {
            return shape;
        }
        let mut changed = (*shape).clone();
        for (old, new) in renumber.iter().enumerate() {
            if old as u32 != *new {
                let faces = mesh::material::faces_with_slot(&shape, old as u32);
                mesh::material::set_material(&mut changed, &faces, *new);
            }
        }
        Arc::new(changed)
    }
}

/// `text` with its first letter a capital.
fn capitalised(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}
