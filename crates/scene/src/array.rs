//! The Array modifier ([decision 0153](../../../docs/decisions/0153-array.md)):
//! copies of the shape in a line, a grid or a ring, as one shape.
//!
//! A ring turns round the object's own pivot, or round another object's
//! (its **centre**), read through the stack's sources (decision 0142) for
//! where it stands only: move the centre object and the ring follows.
//! Deleting the centre object puts the ring back on the pivot, in the same
//! undo step, rather than removing the modifier; copies made together with
//! their centre object turn round the copy.
//!
//! The work is [`mesh::array`]; merging touching copies is Weld by
//! distance's (decision 0152).

use crate::modifier::ModifierContext;
use crate::object::ObjectId;
use mesh::Mesh;
use mesh::array::{ArrayLayout, ArrayOptions};

/// The Array modifier's settings: the layout and, for a ring, the object
/// it turns round.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ArraySettings {
    /// How the copies are laid out.
    pub options: ArrayOptions,
    /// The object a ring turns round; `None` for the shape's own pivot.
    pub centre: Option<ObjectId>,
}

impl ArraySettings {
    /// The settings within limits.
    #[must_use]
    pub fn clamped(&self) -> Self {
        Self {
            options: self.options.clamped(),
            centre: self.centre,
        }
    }

    /// Whether it reads where the object's own pivot is: a ring with no
    /// centre object turns round it.
    pub fn reads_pivot(&self) -> bool {
        self.options.layout == ArrayLayout::Radial && self.centre.is_none()
    }
}

/// How much work an array is, in polygons of a boolean's worth.
pub(crate) fn array_work(input: &Mesh, settings: &ArraySettings) -> usize {
    mesh::array::work(input, &settings.options)
}

/// `input` (in the object's space) copied as `settings` lays out: a ring
/// turns round its centre object's pivot when it has one (and it's there),
/// else round the object's own pivot.
pub(crate) fn apply_array(
    input: &Mesh,
    settings: &ArraySettings,
    context: &ModifierContext,
) -> Result<Mesh, String> {
    let centre = settings
        .centre
        .and_then(|id| context.sources.iter().find(|s| s.id == id))
        .map_or(context.pivot, |source| source.pivot);
    mesh::array::array(input, &settings.options, centre, crate::MAX_SHAPE_POLYGONS)
        .map_err(|why| why.to_string())
}
