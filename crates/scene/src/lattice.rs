//! Lattice cages and the Lattice modifier
//! ([decision 0144](../../../docs/decisions/0144-lattice-cage.md)).
//!
//! A cage is an object of its own ([`crate::ObjectKind::Lattice`]): a box
//! of points ([`mesh::lattice::Lattice`]) drawn as lines, with no shape to
//! export. Each shape it bends has a Lattice modifier naming it
//! ([`LatticeOptions::cage`]); the modifier reads the cage through the
//! stack's sources (decision 0142), so moving a cage point, or the cage,
//! or the shape, works the modifier out again. Deleting the cage removes
//! the modifiers in the same undo step; copies made together with their
//! cage follow the copy.
//!
//! The modifier is the first **deformer** (`mesh::deform`): it moves the
//! shape's points through the cage's space and keeps everything else.
//! Bend, twist and taper follow the same way.

use crate::modifier::{ModifierContext, SourceData};
use crate::object::ObjectId;
use mesh::Mesh;
use mesh::deform::deform;
use words::{text, tr};

/// The Lattice modifier's settings: the cage it follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LatticeOptions {
    /// The lattice object whose points bend the shape.
    pub cage: ObjectId,
}

impl LatticeOptions {
    /// No cage yet: the modifier's standard settings, which no shape can
    /// take ([`crate::Command::AddLattice`] makes a cage and names it).
    pub const NO_CAGE: ObjectId = ObjectId(u64::MAX);
}

impl Default for LatticeOptions {
    fn default() -> Self {
        Self {
            cage: Self::NO_CAGE,
        }
    }
}

/// Why a lattice change was given to something else.
pub(crate) const NOT_A_CAGE: &str = text!("that object isn't a lattice cage");

/// The reason, marked for the language layer, a cage change can't be made.
pub(crate) fn problem_reason(problem: mesh::lattice::LatticeProblem) -> &'static str {
    use mesh::lattice::LatticeProblem as P;
    match problem {
        P::NotANumber => text!("a cage point would go to a place that isn't a number"),
        P::TooFar => text!("a cage point would go more than 10 km away"),
        P::Damaged => text!("the cage's points don't match its divisions or size"),
    }
}

/// Why the Lattice modifier has nothing to follow.
const NO_CAGE_FOUND: &str = text!("its cage isn't there");

/// Where the Lattice modifier's work is: about 20 points cost as much as
/// one polygon in a boolean.
pub(crate) fn lattice_work(input: &Mesh) -> usize {
    input.vertex_count() / 20
}

/// `input` (in the object's space) bent by the cage in `context`.
pub(crate) fn apply_lattice(
    input: &Mesh,
    options: &LatticeOptions,
    context: &ModifierContext,
) -> Result<Mesh, String> {
    let source = context
        .sources
        .iter()
        .find(|s| s.id == options.cage)
        .ok_or_else(|| tr(NO_CAGE_FOUND).to_string())?;
    let SourceData::Cage(cage) = &source.data else {
        return Err(tr(NO_CAGE_FOUND).to_string());
    };
    if cage.is_at_rest() {
        return Ok(input.clone());
    }
    // `source.matrix` carries the cage's space into the shape's: the
    // deformer needs the other way round.
    let into = if source.matrix.determinant().abs() >= 1e-12 {
        source.matrix.inverse()
    } else {
        return Err(tr("the cage is flattened (scaled to nothing along an axis)").to_string());
    };
    deform(input, into, &cage.deformer()).map_err(|why| why.to_string())
}

impl crate::Scene {
    /// The same scene with object `id` (stored as an empty group: older
    /// parts of the file hold a cage so) given back its cage, for a
    /// project being opened. Refused for an object that isn't an empty
    /// group with no edits.
    pub fn with_lattice(
        mut self,
        id: ObjectId,
        lattice: mesh::lattice::Lattice,
    ) -> Result<Self, &'static str> {
        let object = self
            .object_mut(id)
            .ok_or_else(|| tr("a lattice names an object that isn't there"))?;
        if object.kind != crate::ObjectKind::Group || !object.stack.is_empty() {
            return Err(tr("a lattice names an object that isn't stored as one"));
        }
        object.kind = crate::ObjectKind::Lattice(lattice);
        Ok(self)
    }
}
