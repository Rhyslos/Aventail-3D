//! Levels of detail in exports ([decision 0155](../../../../docs/decisions/0155-levels-of-detail.md)).
//!
//! With [`super::ExportOptions::lod_levels`] on, an object with levels of
//! detail goes out under its own name as before (its full shape, level
//! 0), with each level beside it as an object of its own called
//! `name_LOD1`, `name_LOD2` and so on, at the same pivot, with the same
//! materials: the naming most game tools look for. Each level's glTF node
//! carries, in its `extras`, its level and the distances it is shown
//! between, in the file's units (`lod_level`, `lod_begin`, `lod_end`; an
//! end of 0 is no end), and so does the object's own node (level 0); in
//! an export for Godot, the levels are hidden (`KHR_node_visibility`), so
//! the imported scene shows the object once until they are set up.
//!
//! Godot doesn't read levels of detail from a glTF file (no `MSFT_lod`,
//! no name ending): it makes its own when it imports one, from the full
//! shape. See the decision for the sources.

use super::{ExportMesh, ExportOptions, ExportedObject, Frames, export_mesh};
use glam::Mat4;
use scene::{ObjectId, Scene};

/// One level of detail of an exported object.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportLevel {
    /// Its shape, in the world of the file (as [`ExportedObject::world`]).
    pub world: ExportMesh,
    /// Its shape around the object's pivot (as [`ExportedObject::local`]).
    pub local: ExportMesh,
    /// How far from the camera a game starts showing it, in the file's
    /// units.
    pub distance: f32,
}

/// Where a level's object stands among its object's levels, for the
/// file's notes on it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LevelMark {
    /// Its level: 0 is the full shape.
    pub level: usize,
    /// From how far it is shown, in the file's units.
    pub begin: f32,
    /// Up to how far it is shown (0: no end).
    pub end: f32,
}

/// The levels of object `id`, each its real shape (decision 0165: the
/// model's with the level's changes to its modifiers, reduced or not, or
/// made by hand), placed by `world` and `local` as its shape is (only
/// those `frames` names), shaded by `shading`; none unless it has levels
/// of detail and the export asks for them.
pub(super) fn levels(
    scene: &Scene,
    id: ObjectId,
    (world, local): (Mat4, Mat4),
    shading: &ExportOptions,
    frames: Frames,
) -> Vec<ExportLevel> {
    if !shading.lod_levels {
        return Vec::new();
    }
    let Some(object) = scene.object(id) else {
        return Vec::new();
    };
    let settings = object.lods.clamped();
    let Some(made) = scene.lod_shapes(id) else {
        return Vec::new();
    };
    let per_cm = shading.units.per_centimetre();
    made.levels
        .iter()
        .zip(&settings.levels)
        .map(|(mesh, level)| ExportLevel {
            world: if frames.world() {
                export_mesh(mesh, world, shading)
            } else {
                ExportMesh::default()
            },
            local: if frames.local() {
                export_mesh(mesh, local, shading)
            } else {
                ExportMesh::default()
            },
            distance: level.distance * per_cm,
        })
        .collect()
}

/// The marks of an object's levels: the full shape (level 0) and each
/// level after it, each shown from its own distance up to the next's.
pub(crate) fn marks(object: &ExportedObject) -> Vec<LevelMark> {
    let mut begins = vec![0.0];
    begins.extend(object.levels.iter().map(|l| l.distance));
    (0..begins.len())
        .map(|level| LevelMark {
            level,
            begin: begins[level],
            end: begins.get(level + 1).copied().unwrap_or(0.0),
        })
        .collect()
}

/// Every level of every object in `objects`, each as an object of its own
/// beside it, named `name_LOD1` and on, with its mark.
pub(crate) fn level_objects(objects: &[ExportedObject]) -> Vec<(ExportedObject, LevelMark)> {
    objects
        .iter()
        .flat_map(|o| {
            let marks = marks(o);
            o.levels
                .iter()
                .zip(marks.into_iter().skip(1))
                .map(move |(level, mark)| {
                    (
                        ExportedObject {
                            name: format!("{}_LOD{}", o.name, mark.level),
                            world: level.world.clone(),
                            local: level.local.clone(),
                            collision: scene::Collision::None,
                            collision_parts: Vec::new(),
                            levels: Vec::new(),
                            outline: None,
                            ..o.clone()
                        },
                        mark,
                    )
                })
        })
        .collect()
}
