//! An object's collision shapes worked out ([decision 0151](../../../docs/decisions/0151-collision-hulls.md)):
//! the hulls its Collision setting gives a game engine, to draw over the
//! viewport and to export.
//!
//! *The shape itself* has none (it is the shape). *Wrapped* is one hull
//! round every point, as Godot makes from a `-convcol` name. *Wrapped in
//! parts* is the shape split into convex parts
//! ([`mesh::convex_parts`]), each a hull; a shape that can't be split
//! (open, say) collides as one hull of at most
//! [`mesh::convex_parts::MAX_HULL_CORNERS`] corners instead, with the
//! reason. *A box* is the box round the shape square to its pivot, as the
//! export writes it.
//!
//! Worked out from a shape given by the caller (the viewport's, or an
//! export's), and kept per object while the shape and settings stay the
//! same. Parts take a while, so the interface works them out on another
//! thread ([`CollisionJob`]), as heavy modifiers are.

use crate::Purpose;
use crate::object::{Collision, ObjectId};
use crate::tree::Scene;
use glam::{Mat4, Vec3};
use mesh::Mesh;
use mesh::convex_parts::{ConvexPartsOptions, MAX_HULL_CORNERS, convex_parts, wrapped};
use mesh::hull::{Hull, convex_hull};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use words::trf;

/// From this much work ([`CollisionJob::work`]), the interface works an
/// object's collision shapes out on another thread.
pub const COLLISION_IN_BACKGROUND_FROM: usize = 50_000;

/// An object's collision shapes, in its shape's own space.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CollisionShapes {
    /// The hulls.
    pub hulls: Vec<Hull>,
    /// Why they aren't what the setting asks for (a shape that couldn't be
    /// split, or has no inside), in plain words starting with a capital.
    pub problem: Option<String>,
}

/// What an object's collision shapes are worked out from.
#[derive(Debug, Clone, PartialEq)]
struct Setting {
    collision: Collision,
    parts: ConvexPartsOptions,
    /// The shape's place relative to its pivot (for a box).
    offset: Mat4,
}

/// The collision shapes `collision` gives `mesh` (see the module notes),
/// `offset` placing the shape relative to its pivot. Nothing for *None*
/// and *The shape itself*.
pub fn collision_shapes(
    mesh: &Mesh,
    collision: Collision,
    parts: &ConvexPartsOptions,
    offset: Mat4,
) -> CollisionShapes {
    let one = |hull: Result<Hull, mesh::hull::HullProblem>| match hull {
        Ok(hull) => CollisionShapes {
            hulls: vec![hull],
            problem: None,
        },
        Err(why) => CollisionShapes {
            hulls: Vec::new(),
            problem: Some(trf!("No collision shape: {}.", why)),
        },
    };
    match collision {
        Collision::None | Collision::Mesh => CollisionShapes::default(),
        Collision::Convex => one(wrapped(mesh, usize::MAX)),
        Collision::Box => one(box_round(mesh, offset)),
        Collision::Parts => match convex_parts(mesh, parts) {
            Ok(hulls) => CollisionShapes {
                hulls,
                problem: None,
            },
            Err(why) => match wrapped(mesh, MAX_HULL_CORNERS) {
                Ok(hull) => CollisionShapes {
                    hulls: vec![hull],
                    problem: Some(trf!(
                        "Not split into parts: {}. It collides as one wrapped shape instead.",
                        why
                    )),
                },
                Err(flat) => CollisionShapes {
                    hulls: Vec::new(),
                    problem: Some(trf!("No collision shape: {}.", flat)),
                },
            },
        },
    }
}

/// The box round `mesh`'s points square to its pivot (`offset` placing
/// the shape relative to it), back in the shape's space.
fn box_round(mesh: &Mesh, offset: Mat4) -> Result<Hull, mesh::hull::HullProblem> {
    let offset = if offset.determinant().abs() > 1e-12 && offset.is_finite() {
        offset
    } else {
        Mat4::IDENTITY
    };
    let back = offset.inverse();
    let around = mesh::Bounds::around(
        mesh.faces()
            .flat_map(|(_, corners)| corners.iter().filter_map(|v| mesh.position(*v)))
            .filter(|p| p.is_finite())
            .map(|p| offset.transform_point3(p)),
    );
    let Some(around) = around else {
        return Err(mesh::hull::HullProblem::Empty);
    };
    let corners: Vec<Vec3> = around
        .corners()
        .iter()
        .map(|c| back.transform_point3(*c))
        .collect();
    convex_hull(&corners)
}

/// About how much work working out `collision` for `mesh` is, in
/// polygons: splitting into parts is always heavy enough for another
/// thread.
fn work(mesh: &Mesh, collision: Collision) -> usize {
    match collision {
        Collision::None | Collision::Mesh | Collision::Box => 0,
        Collision::Convex => mesh.face_count() * 4,
        Collision::Parts => COLLISION_IN_BACKGROUND_FROM + mesh.face_count(),
    }
}

/// An object's collision shapes still to work out.
#[derive(Debug, Clone)]
pub struct CollisionJob {
    id: ObjectId,
    purpose: Purpose,
    input: Arc<Mesh>,
    setting: Setting,
}

impl CollisionJob {
    /// About how much work it is (see [`COLLISION_IN_BACKGROUND_FROM`]).
    pub fn work(&self) -> usize {
        work(&self.input, self.setting.collision)
    }

    /// Does the work, on any thread.
    pub fn run(self) -> CollisionDone {
        let shapes = collision_shapes(
            &self.input,
            self.setting.collision,
            &self.setting.parts,
            self.setting.offset,
        );
        CollisionDone {
            id: self.id,
            purpose: self.purpose,
            kept: Kept {
                input: self.input,
                setting: self.setting,
                shapes: Arc::new(shapes),
                fresh: true,
            },
        }
    }
}

/// Collision shapes worked out, to hand to [`Scene::store_collision`].
#[derive(Debug, Clone)]
pub struct CollisionDone {
    id: ObjectId,
    purpose: Purpose,
    kept: Kept,
}

impl CollisionDone {
    /// The shapes.
    pub fn shapes(&self) -> Arc<CollisionShapes> {
        Arc::clone(&self.kept.shapes)
    }
}

/// Whether an object's collision shapes are worked out.
#[derive(Debug, Clone)]
pub enum CollisionState {
    /// They are.
    Ready(Arc<CollisionShapes>),
    /// They still need working out.
    Needed(CollisionJob),
}

/// Collision shapes kept, with what they were made from.
#[derive(Debug, Clone)]
struct Kept {
    input: Arc<Mesh>,
    setting: Setting,
    shapes: Arc<CollisionShapes>,
    /// Whether it was made from the shape last asked about for its
    /// purpose: once a newer shape is asked about, it is kept to show
    /// meanwhile, but no longer said to be the object's.
    fresh: bool,
}

/// Each object's collision shapes last worked out (not part of what the
/// scene is: copies start empty, and it never affects equality).
#[derive(Default)]
pub(crate) struct CollisionCache(Mutex<HashMap<(ObjectId, Purpose), Kept>>);

impl CollisionCache {
    /// Forgets `id`'s (deleted).
    pub(crate) fn forget(&mut self, id: ObjectId) {
        self.0
            .get_mut()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .retain(|(object, _), _| *object != id);
    }

    /// `id`'s for `purpose`, if it was made from the shape last asked
    /// about.
    fn fresh(&self, id: ObjectId, purpose: Purpose) -> Option<Kept> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&(id, purpose))
            .filter(|kept| kept.fresh)
            .cloned()
    }

    /// `id`'s for `purpose` if it was made with `setting` from `shape`;
    /// otherwise, as a newer shape is now asked about, the one kept is no
    /// longer fresh.
    fn made_from(
        &self,
        id: ObjectId,
        purpose: Purpose,
        setting: &Setting,
        shape: &Arc<Mesh>,
    ) -> Option<Kept> {
        let mut kept = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let entry = kept.get_mut(&(id, purpose))?;
        if entry.setting == *setting
            && (Arc::ptr_eq(&entry.input, shape) || *entry.input == **shape)
        {
            entry.fresh = true;
            return Some(entry.clone());
        }
        entry.fresh = false;
        None
    }
}

impl CollisionCache {
    /// A copy holding the same results, for
    /// [`crate::Scene::copy_with_results`].
    pub(crate) fn copied(&self) -> Self {
        Self(Mutex::new(
            self.0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone(),
        ))
    }
}

impl Clone for CollisionCache {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl PartialEq for CollisionCache {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl std::fmt::Debug for CollisionCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CollisionCache").finish_non_exhaustive()
    }
}

impl Scene {
    /// The object's collision setting, as its shapes are worked out from.
    fn collision_setting(&self, id: ObjectId) -> Option<Setting> {
        let object = self.object(id)?;
        Some(Setting {
            collision: object.collision,
            parts: object.collision_parts.clamped(),
            offset: object.offset.matrix(),
        })
    }

    /// Whether the object's collision shapes for `shape` (its final shape,
    /// as the viewport shows it or an export makes it) are worked out, or
    /// the work to do. `None` when its collision has no shapes of its own
    /// (*None*, *The shape itself*) or it isn't in the scene.
    pub fn collision_state(
        &self,
        id: ObjectId,
        purpose: Purpose,
        shape: Arc<Mesh>,
    ) -> Option<CollisionState> {
        let setting = self.collision_setting(id)?;
        if matches!(setting.collision, Collision::None | Collision::Mesh) {
            return None;
        }
        if let Some(kept) = self.collisions.made_from(id, purpose, &setting, &shape) {
            return Some(CollisionState::Ready(kept.shapes));
        }
        Some(CollisionState::Needed(CollisionJob {
            id,
            purpose,
            input: shape,
            setting,
        }))
    }

    /// Keeps collision shapes worked out elsewhere ([`CollisionJob::run`]),
    /// for [`Scene::collision_state`] to find while the shape and setting
    /// stay the same.
    pub fn store_collision(&self, done: CollisionDone) {
        self.collisions
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert((done.id, done.purpose), done.kept);
    }

    /// The object's collision shapes for `shape`, worked out here, on
    /// this thread, when not kept from before (an export's). `None` as for
    /// [`Scene::collision_state`].
    pub fn collision_shapes(
        &self,
        id: ObjectId,
        purpose: Purpose,
        shape: Arc<Mesh>,
    ) -> Option<Arc<CollisionShapes>> {
        match self.collision_state(id, purpose, shape)? {
            CollisionState::Ready(shapes) => Some(shapes),
            CollisionState::Needed(job) => {
                let done = job.run();
                let shapes = done.shapes();
                self.store_collision(done);
                Some(shapes)
            }
        }
    }

    /// Why the object's collision shapes last worked out aren't what its
    /// setting asks for, if they aren't (for the Attributes panel; nothing
    /// while they are still to be worked out).
    pub fn collision_problem(&self, id: ObjectId) -> Option<String> {
        let setting = self.collision_setting(id)?;
        // The viewport's, for the shape it last asked about.
        let kept = self.collisions.fresh(id, Purpose::Viewport)?;
        if kept.setting != setting {
            return None;
        }
        kept.shapes.problem.clone()
    }
}
