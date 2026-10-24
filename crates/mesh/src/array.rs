//! Arrays ([decision 0153](../../../docs/decisions/0153-array.md)): copies
//! of a shape laid out in a line, a grid or a ring, as one mesh.
//!
//! **The layouts** ([`ArrayLayout`]):
//!
//! - **Line:** [`ArrayOptions::count`] copies, each one step on from the
//!   one before. The step is [`ArrayOptions::offset`], in any direction (a
//!   step up and along makes stairs).
//! - **Grid:** [`ArrayOptions::counts`] copies along X, Y and Z, spaced by
//!   [`ArrayOptions::spacing`] along each.
//! - **Radial:** [`ArrayOptions::count`] copies turned round an axis
//!   through a centre (the object's pivot, or another object's), evenly
//!   over [`ArrayOptions::angle`]. A whole turn (360°) shares it out with
//!   no copy on top of the first; less than a whole turn puts the first
//!   and the last copies at its two ends, as a fan.
//!
//! **Offsets** are *relative to the shape's size* (1 = copies touching,
//! side to side: the shape's box is the size) or *absolute distances*.
//!
//! **The first copy** is the shape itself, keeping its points' and
//! polygons' ids, so selections and recorded edits made on it still find
//! their way; the others get new ids after it. UVs, hard edges, seams,
//! creases and materials come along with each copy. With
//! [`ArrayOptions::uvs_side_by_side`], each copy's UVs are shrunk into a
//! square of their own in the texture, so the copies don't overlap there
//! and can be painted or baked one by one.
//!
//! **Merging:** with [`ArrayOptions::merge`], points of copies that touch
//! (within [`ArrayOptions::merge_distance`]) are welded with Weld by
//! distance ([`crate::weld_distance`], decision 0152), open borders only:
//! the copies of a fence or a pipe join into one surface, and the closed
//! copies of a solid stay closed solids side by side.
//!
//! Deterministic: the same shape and settings always give the same mesh.

use crate::Mesh;
use crate::bounds::Bounds;
use crate::parts::{append, place};
use crate::uvs::{Uvs, set_uvs};
use crate::weld_distance::{WeldOptions, weld_by_distance};
use glam::{Mat4, Vec2, Vec3};
use words::trf;

/// How the copies are laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ArrayLayout {
    /// One after another along a step.
    #[default]
    Line,
    /// Along X, Y and Z, a count for each.
    Grid,
    /// Turned round an axis.
    Radial,
}

impl ArrayLayout {
    /// Every layout, in the order the interface lists them.
    pub const ALL: [Self; 3] = [Self::Line, Self::Grid, Self::Radial];
}

/// The axis a radial array turns round (the object's own X, Y or Z).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ArrayAxis {
    /// Its X axis.
    X,
    /// Its Y axis.
    Y,
    /// Its Z axis, up: a ring lying on the ground.
    #[default]
    Z,
}

impl ArrayAxis {
    /// Every axis, in order.
    pub const ALL: [Self; 3] = [Self::X, Self::Y, Self::Z];

    /// Its direction.
    pub fn direction(self) -> Vec3 {
        match self {
            Self::X => Vec3::X,
            Self::Y => Vec3::Y,
            Self::Z => Vec3::Z,
        }
    }

    /// The way [`ArrayOptions::radius`] moves the shape out from the
    /// centre, across the axis: X for the Y and Z axes, Y for the X axis.
    pub fn outward(self) -> Vec3 {
        match self {
            Self::X => Vec3::Y,
            Self::Y | Self::Z => Vec3::X,
        }
    }
}

/// The Array modifier's settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArrayOptions {
    /// How the copies are laid out.
    pub layout: ArrayLayout,
    /// How many copies (the shape included) in a line or a ring.
    pub count: u32,
    /// How many copies along X, Y and Z in a grid.
    pub counts: [u32; 3],
    /// Whether [`offset`](Self::offset) and [`spacing`](Self::spacing) are
    /// shares of the shape's size (1 = touching) rather than distances in
    /// centimetres.
    pub relative: bool,
    /// A line's step from one copy to the next.
    pub offset: Vec3,
    /// A grid's step along X, Y and Z.
    pub spacing: Vec3,
    /// How far round a ring goes, in degrees (360: a whole turn).
    pub angle: f32,
    /// The axis a ring turns round.
    pub axis: ArrayAxis,
    /// How far, in centimetres, each copy of a ring is moved out from the
    /// centre before it is turned ([`ArrayAxis::outward`]).
    pub radius: f32,
    /// Whether the points where copies touch are welded into one.
    pub merge: bool,
    /// How close, in centimetres, touching points must be to weld.
    pub merge_distance: f32,
    /// Whether each copy's UVs go in a square of their own in the texture.
    pub uvs_side_by_side: bool,
}

impl ArrayOptions {
    /// The standard number of copies in a line.
    pub const STANDARD_COUNT: u32 = 3;
    /// The standard number of copies in a ring.
    pub const STANDARD_RADIAL_COUNT: u32 = 8;
    /// The most copies along a line, a ring or one side of a grid.
    pub const MAX_COUNT: u32 = 1_000;
    /// The most copies in all.
    pub const MAX_COPIES: u64 = 100_000;
    /// The largest offset, spacing or radius: 1 km, or 100 000 times the
    /// shape's size.
    pub const MAX_DISTANCE: f32 = 100_000.0;

    /// The settings brought within their limits: counts from 1 to
    /// [`Self::MAX_COUNT`], distances within [`Self::MAX_DISTANCE`], the
    /// angle within a turn either way; values that aren't real numbers
    /// become the standard ones.
    #[must_use]
    pub fn clamped(self) -> Self {
        let standard = Self::default();
        let count = |n: u32| n.clamp(1, Self::MAX_COUNT);
        let distance = |value: f32, otherwise: f32| {
            if value.is_finite() {
                value.clamp(-Self::MAX_DISTANCE, Self::MAX_DISTANCE)
            } else {
                otherwise
            }
        };
        let vector = |value: Vec3, otherwise: Vec3| {
            Vec3::new(
                distance(value.x, otherwise.x),
                distance(value.y, otherwise.y),
                distance(value.z, otherwise.z),
            )
        };
        let weld = WeldOptions {
            distance: self.merge_distance,
            ..WeldOptions::default()
        }
        .clamped();
        Self {
            layout: self.layout,
            count: count(self.count),
            counts: self.counts.map(count),
            relative: self.relative,
            offset: vector(self.offset, standard.offset),
            spacing: vector(self.spacing, standard.spacing),
            angle: if self.angle.is_finite() {
                self.angle.clamp(-360.0, 360.0)
            } else {
                standard.angle
            },
            axis: self.axis,
            radius: distance(self.radius, standard.radius),
            merge: self.merge,
            merge_distance: weld.distance,
            uvs_side_by_side: self.uvs_side_by_side,
        }
    }

    /// How many copies it makes (the shape included).
    pub fn copies(&self) -> u64 {
        let options = self.clamped();
        match options.layout {
            ArrayLayout::Line | ArrayLayout::Radial => u64::from(options.count),
            ArrayLayout::Grid => options.counts.iter().map(|n| u64::from(*n)).product(),
        }
    }

    /// The turn from one copy of a ring to the next, in degrees.
    pub fn radial_step(&self) -> f32 {
        let options = self.clamped();
        let count = options.count as f32;
        if options.angle.abs() >= 360.0 - 1e-3 {
            options.angle / count
        } else if options.count > 1 {
            options.angle / (count - 1.0)
        } else {
            0.0
        }
    }
}

impl Default for ArrayOptions {
    fn default() -> Self {
        Self {
            layout: ArrayLayout::Line,
            count: Self::STANDARD_COUNT,
            counts: [3, 3, 1],
            relative: true,
            offset: Vec3::X,
            spacing: Vec3::ONE,
            angle: 360.0,
            axis: ArrayAxis::Z,
            radius: 0.0,
            merge: false,
            merge_distance: WeldOptions::STANDARD_DISTANCE,
            uvs_side_by_side: false,
        }
    }
}

/// Why an array can't be made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrayProblem {
    /// More copies than [`ArrayOptions::MAX_COPIES`].
    TooManyCopies {
        /// How many it would make.
        copies: u64,
    },
    /// The result would have more polygons than allowed.
    TooManyPolygons {
        /// How many it would have.
        polygons: u64,
        /// The most allowed.
        most: usize,
    },
}

impl std::fmt::Display for ArrayProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooManyCopies { copies } => f.write_str(&trf!(
                "it would make {} copies, and the most is 100 000: lower the counts",
                copies
            )),
            Self::TooManyPolygons { polygons, most } => f.write_str(&trf!(
                "the result would have {} polygons, and the most is {}: fewer copies, or a \
                 simpler shape",
                polygons,
                most
            )),
        }
    }
}

impl std::error::Error for ArrayProblem {}

/// Where each copy goes: one matrix per copy, the shape's own first, in
/// the shape's space. `centre` is where a ring turns round (the pivot or
/// the centre object, in the shape's space); `size` is the shape's box
/// size, for relative offsets.
pub fn copy_matrices(options: &ArrayOptions, centre: Vec3, size: Vec3) -> Vec<Mat4> {
    let options = options.clamped();
    let scale = if options.relative { size } else { Vec3::ONE };
    match options.layout {
        ArrayLayout::Line => {
            let step = options.offset * scale;
            (0..options.count)
                .map(|i| Mat4::from_translation(step * i as f32))
                .collect()
        }
        ArrayLayout::Grid => {
            let step = options.spacing * scale;
            let [nx, ny, nz] = options.counts;
            let mut out = Vec::new();
            for z in 0..nz {
                for y in 0..ny {
                    for x in 0..nx {
                        let at = Vec3::new(x as f32, y as f32, z as f32) * step;
                        out.push(Mat4::from_translation(at));
                    }
                }
            }
            out
        }
        ArrayLayout::Radial => {
            let centre = if centre.is_finite() {
                centre
            } else {
                Vec3::ZERO
            };
            let axis = options.axis.direction();
            let out_by = Mat4::from_translation(options.axis.outward() * options.radius);
            let step = options.radial_step().to_radians();
            (0..options.count)
                .map(|i| {
                    Mat4::from_translation(centre)
                        * Mat4::from_axis_angle(axis, step * i as f32)
                        * Mat4::from_translation(-centre)
                        * out_by
                })
                .collect()
        }
    }
}

/// The Array modifier's work: `input` copied as `options` lays out (see
/// the module's notes), with a ring turning round `centre` (in the
/// shape's space). Refused when it would make more than
/// [`ArrayOptions::MAX_COPIES`] copies or more than `most` polygons.
pub fn array(
    input: &Mesh,
    options: &ArrayOptions,
    centre: Vec3,
    most: usize,
) -> Result<Mesh, ArrayProblem> {
    let options = options.clamped();
    let copies = options.copies();
    if copies > ArrayOptions::MAX_COPIES {
        return Err(ArrayProblem::TooManyCopies { copies });
    }
    let polygons = copies.saturating_mul(input.face_count() as u64);
    if polygons > most as u64 {
        return Err(ArrayProblem::TooManyPolygons { polygons, most });
    }
    if input.face_count() == 0 || copies <= 1 && options.layout != ArrayLayout::Radial {
        return Ok(input.clone());
    }
    let size = Bounds::around(input.vertices().map(|(_, p)| p)).map_or(Vec3::ZERO, |b| b.size());
    let matrices = copy_matrices(&options, centre, size);
    let uvs = options
        .uvs_side_by_side
        .then(|| UvSquares::of(input, matrices.len()))
        .flatten();
    let copy_of = |index: usize| -> Option<Mesh> {
        let squares = uvs.as_ref()?;
        let mut copy = input.clone();
        set_uvs(&mut copy, squares.values_for(index));
        Some(copy)
    };
    let mut out = copy_of(0).unwrap_or_else(|| input.clone());
    if let Some(first) = matrices.first()
        && *first != Mat4::IDENTITY
    {
        place(&mut out, *first);
    }
    for (index, matrix) in matrices.iter().enumerate().skip(1) {
        if !matrix.is_finite() {
            continue;
        }
        match copy_of(index) {
            Some(copy) => append(&mut out, &copy, *matrix),
            None => append(&mut out, input, *matrix),
        }
    }
    if options.merge && matrices.len() > 1 {
        let weld = WeldOptions {
            distance: options.merge_distance,
            only_borders: true,
            keep_seams: true,
        };
        weld_by_distance(&mut out, None, &weld);
    }
    Ok(out)
}

/// How much work an array is, in polygons of a boolean's worth: about
/// eight polygons appended cost as much as one in a boolean (as for a
/// scatter, decision 0146), and a merge goes through every point again.
/// Too many copies is no work: the array is refused at once.
pub fn work(input: &Mesh, options: &ArrayOptions) -> usize {
    let options = options.clamped();
    if options.copies() > ArrayOptions::MAX_COPIES {
        return 0;
    }
    let copies = usize::try_from(options.copies()).unwrap_or(0);
    let appended = copies.saturating_mul(input.face_count()) / 8;
    let merged = if options.merge {
        copies.saturating_mul(input.vertex_count()) / 5
    } else {
        0
    };
    appended.saturating_add(merged)
}

/// Where each copy's UVs go when they are laid side by side: a grid of
/// squares in the texture, as close to square as the copies allow, each
/// copy's UVs (their box fitted, keeping their shape) in its own square
/// with a small margin.
struct UvSquares {
    /// Every corner's UV of the shape, in its corner order.
    values: Vec<Vec2>,
    /// The lowest corner of the box round the shape's UVs.
    min: Vec2,
    /// What the UVs are multiplied by to fit a square.
    fit: f32,
    /// How many squares across.
    columns: usize,
    /// One square's width.
    square: f32,
}

impl UvSquares {
    /// The share of a square left empty round each copy's UVs.
    const MARGIN: f32 = 0.02;

    /// The squares for `copies` copies of `mesh`, if it has UVs.
    fn of(mesh: &Mesh, copies: usize) -> Option<Self> {
        let (starts, corners) = mesh.face_layout();
        let values = Uvs::of(mesh)?.into_values(corners.len());
        let mut min = Vec2::splat(f32::INFINITY);
        let mut max = Vec2::splat(f32::NEG_INFINITY);
        for (f, list) in mesh.faces() {
            let start = starts[f.get() as usize] as usize;
            for uv in values.iter().skip(start).take(list.len()) {
                if uv.is_finite() {
                    min = min.min(*uv);
                    max = max.max(*uv);
                }
            }
        }
        if !min.is_finite() || !max.is_finite() {
            return None;
        }
        let extent = (max - min).max_element().max(1e-6);
        let copies = copies.max(1);
        let columns = (copies as f64).sqrt().ceil().max(1.0) as usize;
        let rows = copies.div_ceil(columns);
        let square = 1.0 / columns.max(rows) as f32;
        Some(Self {
            values,
            min,
            fit: square * (1.0 - 2.0 * Self::MARGIN) / extent,
            columns,
            square,
        })
    }

    /// Copy `index`'s UVs, in the shape's corner order.
    fn values_for(&self, index: usize) -> Vec<Vec2> {
        let cell = Vec2::new((index % self.columns) as f32, (index / self.columns) as f32)
            * self.square
            + Vec2::splat(self.square * Self::MARGIN);
        self.values
            .iter()
            .map(|uv| {
                if uv.is_finite() {
                    cell + (*uv - self.min) * self.fit
                } else {
                    cell
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests;
