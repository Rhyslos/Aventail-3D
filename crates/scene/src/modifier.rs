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

pub use crate::lattice::LatticeOptions;
use crate::object::{ObjectId, ObjectKind};
use crate::tree::Scene;
use glam::Mat4;
use mesh::Mesh;
use mesh::decimate::DecimateOptions;
use mesh::remesh::RemeshOptions;
use mesh::subdivide::SubdivideOptions;
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
    /// Subdivision smoothing (decision 0143): Catmull-Clark, with creases.
    Subdivide(SubdivideOptions),
    /// A lattice cage's bend (decision 0144): the shape's points moved
    /// through a cage object's box of points. The first deformer.
    Lattice(LatticeOptions),
    /// Decimation (decision 0145): fewer triangles with the shape, its
    /// seams, hard edges, material borders and open borders kept.
    Decimate(DecimateOptions),
}

impl ModifierKind {
    /// One of each kind with its standard settings, in the order the
    /// palette and the menu list them.
    pub fn standard_kinds() -> Vec<Self> {
        vec![
            Self::Subdivide(SubdivideOptions::default()),
            Self::Lattice(LatticeOptions::default()),
            Self::Remesh(RemeshOptions::default()),
            Self::Decimate(DecimateOptions::default()),
        ]
    }

    /// Its name, as the interface shows it.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Remesh(_) => tr("Remesh"),
            Self::Subdivide(_) => tr("Subdivide"),
            Self::Lattice(_) => tr("Lattice"),
            Self::Decimate(_) => tr("Decimate"),
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
            Self::Subdivide(_) => tr(
                "Smooth the shape by subdividing it: every polygon is split into four and the \
                 corners rounded off, level by level. Creased edges stay sharp.",
            ),
            Self::Lattice(_) => tr(
                "Put a cage of points round the selected shapes: move the cage's points (press 1) \
                 and the shapes inside bend, squash and swell smoothly with them.",
            ),
            Self::Decimate(_) => tr(
                "Lower the shape's polygon count while keeping its shape: flat areas lose the \
                 most. UV seams, hard edges, material borders and open borders stay. For game \
                 models and levels of detail.",
            ),
        }
    }

    /// Its family.
    pub fn family(&self) -> ModifierFamily {
        match self {
            Self::Remesh(_) => ModifierFamily::CleanUp,
            Self::Subdivide(_) => ModifierFamily::Generate,
            Self::Lattice(_) => ModifierFamily::Deform,
            Self::Decimate(_) => ModifierFamily::CleanUp,
        }
    }

    /// Whether it gives an export something else than the viewport (the
    /// Subdivide modifier's two levels).
    pub fn differs_by_purpose(&self) -> bool {
        match self {
            Self::Remesh(_) | Self::Lattice(_) | Self::Decimate(_) => false,
            Self::Subdivide(o) => {
                let o = o.clamped();
                o.levels != o.export_levels
            }
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
            Self::Subdivide(options) => Self::Subdivide(options.clamped()),
            Self::Lattice(options) => Self::Lattice(*options),
            Self::Decimate(options) => Self::Decimate(options.clamped()),
        }
    }

    /// The other objects it reads (a lattice's cage; later a curve to
    /// bend along). A modifier whose source is deleted is removed with it,
    /// in the same undo step.
    pub fn sources(&self) -> Vec<ObjectId> {
        match self {
            Self::Remesh(_) | Self::Subdivide(_) | Self::Decimate(_) => Vec::new(),
            Self::Lattice(options) => vec![options.cage],
        }
    }

    /// Whether an object of `kind` can be one of its sources: a lattice
    /// reads a cage.
    pub fn takes_source(&self, kind: &ObjectKind) -> bool {
        match self {
            Self::Lattice(_) => matches!(kind, ObjectKind::Lattice(_)),
            Self::Remesh(_) | Self::Subdivide(_) | Self::Decimate(_) => kind.has_shape(),
        }
    }

    /// The same modifier with each source renamed by `rename` (for copies
    /// made together with their sources).
    pub fn with_sources_renamed(&self, rename: impl Fn(ObjectId) -> ObjectId) -> Self {
        match self {
            Self::Remesh(_) | Self::Subdivide(_) | Self::Decimate(_) => self.clone(),
            Self::Lattice(options) => Self::Lattice(LatticeOptions {
                cage: rename(options.cage),
            }),
        }
    }

    /// Roughly how much work it is on `input` for `purpose`, in polygons
    /// of a boolean's worth: from [`MODIFIERS_IN_BACKGROUND_FROM`], the
    /// viewport works it out on another thread.
    pub fn work(&self, input: &Mesh, purpose: Purpose) -> usize {
        match self {
            // About 40 grid points cost as much as one polygon in a boolean
            // (both about 20 ms for 5 000 in a release build).
            Self::Remesh(options) => {
                mesh::remesh::voxel_count(input, options.voxel_size).unwrap_or(0) as usize / 40
            }
            // About 16 new polygons cost as much as one polygon in a
            // boolean (10 000 polygons at level 2 make 160 000 in about
            // 40 ms in a release build).
            Self::Subdivide(options) => {
                let levels = subdivide_levels(options, purpose);
                if levels == 0 {
                    0
                } else {
                    usize::try_from(mesh::subdivide::polygon_count(input, levels) / 16)
                        .unwrap_or(usize::MAX)
                }
            }
            Self::Lattice(_) => crate::lattice::lattice_work(input),
            // About one triangle of the input costs as much as one polygon
            // in a boolean (100 000 triangles take about half a second in
            // a release build, 5 000 polygons of a boolean 20 ms).
            Self::Decimate(options) => {
                let triangles = mesh::decimate::triangle_count(input);
                if options.target(triangles) >= triangles {
                    0
                } else {
                    triangles
                }
            }
        }
    }

    /// Its result on `input`, or why it can't be made (in plain words,
    /// starting with a small letter). `input` is in the object's own
    /// space, as are the sources in `context`.
    pub(crate) fn apply(&self, input: &Mesh, context: &ModifierContext) -> Result<Mesh, String> {
        match self {
            Self::Remesh(options) => {
                mesh::remesh::remesh(input, options).map_err(|why| why.to_string())
            }
            Self::Subdivide(options) => {
                // Hard edges count as fully creased only where they are what
                // the shading shows: on an object smoothed by angle every
                // corner sharper than the angle looks hard, which would keep
                // a whole low-polygon cage from rounding at all.
                let hard_sharp =
                    options.hard_edges_sharp && matches!(context.hardness, mesh::Hardness::Marked);
                mesh::subdivide::subdivide(
                    input,
                    subdivide_levels(options, context.purpose),
                    hard_sharp,
                )
                .map_err(|why| why.to_string())
            }
            Self::Lattice(options) => crate::lattice::apply_lattice(input, options, context),
            // The hard edges kept are those the object's shading shows
            // (marked, or sharper than its angle).
            Self::Decimate(options) => mesh::decimate::decimate(input, options, context.hardness)
                .map_err(|why| why.to_string()),
        }
    }
}

/// The levels a Subdivide modifier uses for `purpose`.
pub fn subdivide_levels(options: &SubdivideOptions, purpose: Purpose) -> u32 {
    let options = options.clamped();
    match purpose {
        Purpose::Viewport => options.levels,
        Purpose::Export => options.export_levels,
    }
}

/// What a modifier is worked out with besides its input.
#[derive(Debug, Clone)]
pub struct ModifierContext {
    /// What the shape is for.
    pub purpose: Purpose,
    /// How the object is smoothed (decision 0126): by angle, or by its
    /// marked hard edges. Modifiers that treat hard edges specially (the
    /// Subdivide modifier's *Hard edges stay sharp*) read the marks only
    /// when the object shades by them.
    pub hardness: mesh::Hardness,
    /// Its sources' final shapes, placed into the object's own space, in
    /// the order [`ModifierKind::sources`] lists them.
    pub sources: Vec<SourceShape>,
}

/// Another object a modifier reads.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceShape {
    /// The object.
    pub id: ObjectId,
    /// What the modifier reads of it, in its own space.
    pub data: SourceData,
    /// From its space into the space of the object with the modifier.
    pub matrix: Mat4,
}

/// What a modifier reads of another object.
#[derive(Debug, Clone)]
pub enum SourceData {
    /// A shape's final shape.
    Shape(Arc<Mesh>),
    /// A lattice's cage (decision 0144).
    Cage(Arc<mesh::lattice::Lattice>),
}

impl PartialEq for SourceData {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Shape(a), Self::Shape(b)) => Arc::ptr_eq(a, b) || **a == **b,
            (Self::Cage(a), Self::Cage(b)) => Arc::ptr_eq(a, b) || **a == **b,
            _ => false,
        }
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
            // A group, or a lattice's cage (decision 0144).
            _ if !object.kind.has_shape() => return Err(ModifierRefusal::NotAShape),
            ObjectKind::Instance(_) => return Err(ModifierRefusal::Instance),
            _ => {}
        }
        if modifiers.len() > MAX_MODIFIERS {
            return Err(ModifierRefusal::TooMany);
        }
        for (kind, source) in modifiers
            .iter()
            .flat_map(|m| m.kind.sources().into_iter().map(move |s| (&m.kind, s)))
        {
            let usable = source != id
                && self
                    .object(source)
                    .is_some_and(|o| kind.takes_source(&o.kind))
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
