//! The modifier stack ([decision 0142](../../../docs/decisions/0142-modifier-stack-and-remesh.md)):
//! each object has an ordered list of modifiers, non-destructive changes
//! worked out whenever its shape is shown or exported (remesh first; the
//! smoothing, decimate, deformers and generators of Phase 10 join it).
//!
//! An object's shape is worked out in this order:
//!
//! 1. its base and its recorded edits, layer by layer
//!    ([`Scene::evaluated`]: what the Points, Edges and Polygons modes
//!    edit and pick);
//! 2. its booleans (decision 0135);
//! 3. its live mirror (decision 0088);
//! 4. its modifiers, first to last, each taking the one before's result
//!    ([`Scene::final_shape`]).
//!
//! Each modifier has a kind with its settings ([`ModifierKind`]), an
//! on/off switch, and a *Show in viewport* switch: off, the viewport
//! skips it (for a heavy one while modelling) but exports and Bake into
//! shape still use it. What a modifier gives can differ between the
//! viewport and an export ([`Purpose`]): the smoothing modifier, for one,
//! has a level for each.
//!
//! The list changes only through commands ([`crate::Command::AddModifier`],
//! `ChangeModifier`, `RemoveModifier`, `MoveModifier`), so undo works. A
//! modifier that can't do its work (remesh on an open shape) passes its
//! input on unchanged, and says why ([`Scene::modifier_problems`]).
//!
//! To add a kind of modifier, see the recipe in decision 0142.

mod eval;

use crate::object::{ObjectId, ObjectKind};
use crate::tree::Scene;
use glam::Mat4;
use mesh::Mesh;
use mesh::remesh::RemeshOptions;
use std::sync::Arc;
use words::{text, tr};

pub(crate) use eval::StackCache;
pub use eval::{MODIFIERS_IN_BACKGROUND_FROM, StackDone, StackJob, StackResult, StackState};

/// The most modifiers one object can have.
pub const MAX_MODIFIERS: usize = 32;

/// What a shape is worked out for: drawing in the viewport, or an export
/// (and Bake into shape, which keeps what an export would get).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Purpose {
    /// Drawn in the viewport: modifiers with *Show in viewport* off are
    /// skipped.
    Viewport,
    /// Exported or baked: every modifier that is on.
    Export,
}

/// The families modifiers are grouped in, on the modifiers palette and in
/// the Mesh menu's Modifiers submenu, in this order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ModifierFamily {
    /// Make more shape: smoothing, array, shell, bevel, wireframe.
    Generate,
    /// Move points: bend, twist, lattice, displace.
    Deform,
    /// Tidy the polygons: remesh, decimate, weld, quadify.
    CleanUp,
    /// Ready a shape for a game: normals, LODs, collision, triangles.
    Game,
}

impl ModifierFamily {
    /// Every family, in order.
    pub const ALL: [Self; 4] = [Self::Generate, Self::Deform, Self::CleanUp, Self::Game];

    /// Its name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Generate => tr("Generate"),
            Self::Deform => tr("Deform"),
            Self::CleanUp => tr("Clean up"),
            Self::Game => tr("Game ready"),
        }
    }
}

/// A modifier's kind and its settings. Every kind is listed here; adding
/// one makes the compiler point at every place that needs it.
#[derive(Debug, Clone, PartialEq)]
pub enum ModifierKind {
    /// Voxel remesh (decision 0142): the shape rebuilt as an even surface
    /// of quads.
    Remesh(RemeshOptions),
}

impl ModifierKind {
    /// One of each kind with its standard settings, in the order the
    /// palette and the menu list them.
    pub fn standard_kinds() -> Vec<Self> {
        vec![Self::Remesh(RemeshOptions::default())]
    }

    /// Its name, as the interface shows it.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Remesh(_) => tr("Remesh"),
        }
    }

    /// What it does, in a sentence or two (the palette's and the menu's
    /// tooltip).
    pub fn description(&self) -> &'static str {
        match self {
            Self::Remesh(_) => tr(
                "Rebuild the shape as an even surface of square polygons, whatever its polygons \
                 were: for sculpted, joined or boolean-cut shapes. It needs a closed solid.",
            ),
        }
    }

    /// Its family.
    pub fn family(&self) -> ModifierFamily {
        match self {
            Self::Remesh(_) => ModifierFamily::CleanUp,
        }
    }

    /// Whether `other` is the same kind (whatever its settings).
    pub fn same_kind(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }

    /// Its settings brought within their limits (values that aren't real
    /// numbers become the standard ones).
    pub fn clamped(&self) -> Self {
        match self {
            Self::Remesh(options) => Self::Remesh(options.clamped()),
        }
    }

    /// The other objects whose shapes it reads (a lattice's cage, a curve
    /// to bend along). None so far. A modifier whose source is deleted is
    /// removed with it, in the same undo step.
    pub fn sources(&self) -> Vec<ObjectId> {
        match self {
            Self::Remesh(_) => Vec::new(),
        }
    }

    /// The same modifier with each source renamed by `rename` (for copies
    /// made together with their sources).
    pub fn with_sources_renamed(&self, _rename: impl Fn(ObjectId) -> ObjectId) -> Self {
        match self {
            Self::Remesh(_) => self.clone(),
        }
    }

    /// Roughly how much work it is on `input`, in polygons of a boolean's
    /// worth: from [`MODIFIERS_IN_BACKGROUND_FROM`], the viewport works it
    /// out on another thread.
    pub fn work(&self, input: &Mesh) -> usize {
        match self {
            // About 40 grid points cost as much as one polygon in a boolean
            // (both about 20 ms for 5 000 in a release build).
            Self::Remesh(options) => {
                mesh::remesh::voxel_count(input, options.voxel_size).unwrap_or(0) as usize / 40
            }
        }
    }

    /// Its result on `input`, or why it can't be made (in plain words,
    /// starting with a small letter). `input` is in the object's own
    /// space, as are the sources in `context`.
    pub(crate) fn apply(&self, input: &Mesh, _context: &ModifierContext) -> Result<Mesh, String> {
        match self {
            Self::Remesh(options) => {
                mesh::remesh::remesh(input, options).map_err(|why| why.to_string())
            }
        }
    }
}

/// What a modifier is worked out with besides its input.
#[derive(Debug, Clone)]
pub struct ModifierContext {
    /// What the shape is for.
    pub purpose: Purpose,
    /// Its sources' final shapes, placed into the object's own space, in
    /// the order [`ModifierKind::sources`] lists them.
    pub sources: Vec<SourceShape>,
}

/// Another object's shape a modifier reads.
#[derive(Debug, Clone)]
pub struct SourceShape {
    /// The object.
    pub id: ObjectId,
    /// Its final shape, in its own space.
    pub mesh: Arc<Mesh>,
    /// From its space into the space of the object with the modifier.
    pub matrix: Mat4,
}

impl PartialEq for SourceShape {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.matrix == other.matrix
            && (Arc::ptr_eq(&self.mesh, &other.mesh) || *self.mesh == *other.mesh)
    }
}

/// One modifier on an object.
#[derive(Debug, Clone, PartialEq)]
pub struct Modifier {
    /// What it does, with its settings.
    pub kind: ModifierKind,
    /// Whether it does anything: off, the shape is as if it weren't there.
    pub on: bool,
    /// Whether the viewport shows it (when on). Off, the viewport skips it
    /// while exports and Bake into shape still use it.
    pub in_viewport: bool,
}

impl Modifier {
    /// A new modifier of `kind` (settings brought within limits), on and
    /// shown.
    pub fn new(kind: ModifierKind) -> Self {
        Self {
            kind: kind.clamped(),
            on: true,
            in_viewport: true,
        }
    }

    /// Whether it is worked out for `purpose`.
    pub fn applies(&self, purpose: Purpose) -> bool {
        self.on && (purpose == Purpose::Export || self.in_viewport)
    }
}

/// Why an object's modifiers can't be set up, in plain words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModifierRefusal {
    /// A group has no shape to change.
    NotAShape,
    /// An instance shows its original's shape, modifiers and all.
    Instance,
    /// More than [`MAX_MODIFIERS`].
    TooMany,
    /// A source isn't a shape in the scene, is the object itself, or
    /// already depends on it.
    BadSource,
    /// The place given isn't in the list.
    NoSuchModifier,
    /// A change gave a modifier another kind.
    OtherKind,
}

impl std::fmt::Display for ModifierRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let why = match self {
            Self::NotAShape => text!("a group has no shape: select shapes"),
            Self::Instance => text!(
                "it's an instance: give its original the modifier, or bake it into a shape first"
            ),
            Self::TooMany => text!("a shape can have at most 32 modifiers"),
            Self::BadSource => text!(
                "a modifier reads a shape that isn't there, is the shape itself, or already uses it"
            ),
            Self::NoSuchModifier => text!("that modifier isn't there"),
            Self::OtherKind => text!("a modifier can't change into another kind"),
        };
        f.write_str(tr(why))
    }
}

impl std::error::Error for ModifierRefusal {}

impl Scene {
    /// The object's modifiers, first to last (empty for none, or an
    /// unknown id).
    pub fn modifiers(&self, id: ObjectId) -> &[Modifier] {
        self.object(id).map_or(&[], |o| o.modifiers.as_slice())
    }

    /// The object whose modifiers `id`'s shape gets: its own, or an
    /// instance's original's.
    pub fn modifier_owner(&self, id: ObjectId) -> ObjectId {
        match self.object(id).map(|o| &o.kind) {
            Some(ObjectKind::Instance(source)) => *source,
            _ => id,
        }
    }

    /// Whether `modifiers` would make sense on `id`: a shape, not an
    /// instance, at most [`MAX_MODIFIERS`], every source a shape in the
    /// scene other than `id` that doesn't depend on it.
    pub fn check_modifiers(
        &self,
        id: ObjectId,
        modifiers: &[Modifier],
    ) -> Result<(), ModifierRefusal> {
        if modifiers.is_empty() {
            return Ok(());
        }
        let object = self.object(id).ok_or(ModifierRefusal::NotAShape)?;
        match object.kind {
            ObjectKind::Group => return Err(ModifierRefusal::NotAShape),
            ObjectKind::Instance(_) => return Err(ModifierRefusal::Instance),
            _ => {}
        }
        if modifiers.len() > MAX_MODIFIERS {
            return Err(ModifierRefusal::TooMany);
        }
        for source in modifiers.iter().flat_map(|m| m.kind.sources()) {
            let usable = source != id
                && self.object(source).is_some_and(|o| o.kind.has_shape())
                && !self.shape_uses(source, id);
            if !usable {
                return Err(ModifierRefusal::BadSource);
            }
        }
        Ok(())
    }

    /// The same scene with object `id`'s modifiers set, for a project
    /// being opened (not a step to undo: use the modifier commands).
    /// Settings are brought within limits; modifiers that don't make
    /// sense ([`Scene::check_modifiers`]) are refused.
    pub fn with_modifiers(
        mut self,
        id: ObjectId,
        modifiers: Vec<Modifier>,
    ) -> Result<Self, ModifierRefusal> {
        let modifiers: Vec<Modifier> = modifiers
            .into_iter()
            .map(|m| Modifier {
                kind: m.kind.clamped(),
                ..m
            })
            .collect();
        self.check_modifiers(id, &modifiers)?;
        if let Some(object) = self.object_mut(id) {
            object.modifiers = modifiers;
        }
        Ok(self)
    }

    /// Whether any of the object's modifiers (an instance's original's)
    /// is worked out for `purpose`.
    pub fn has_modifiers_for(&self, id: ObjectId, purpose: Purpose) -> bool {
        self.modifiers(self.modifier_owner(id))
            .iter()
            .any(|m| m.applies(purpose))
    }
}
