//! Version 54 of the project file ([decision 0153](../../../../docs/decisions/0153-array.md)):
//! version 53's, with the Array modifier (`v43::KindV43::Array`, its
//! settings stored as [`ArrayV54`]). Files before version 54 never hold
//! it.

use glam::Vec3;
use scene::{ArrayAxis, ArrayLayout, ArrayOptions, ArraySettings, ObjectId};
use serde::{Deserialize, Serialize};

/// The Array modifier's settings as stored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ArrayV54 {
    /// The layout: 0 line, 1 grid, 2 radial (anything else is a line).
    pub(crate) layout: u8,
    /// Copies in a line or a ring.
    pub(crate) count: u32,
    /// Copies along X, Y and Z in a grid.
    pub(crate) counts: [u32; 3],
    /// Whether offsets are shares of the shape's size.
    pub(crate) relative: bool,
    /// A line's step.
    pub(crate) offset: [f32; 3],
    /// A grid's step along each axis.
    pub(crate) spacing: [f32; 3],
    /// A ring's angle, in degrees.
    pub(crate) angle: f32,
    /// A ring's axis: 0 X, 1 Y, 2 Z (anything else is Z).
    pub(crate) axis: u8,
    /// How far a ring's copies move out, in centimetres.
    pub(crate) radius: f32,
    /// The object a ring turns round (none for the pivot).
    pub(crate) centre: Option<u64>,
    /// Whether touching copies are welded.
    pub(crate) merge: bool,
    /// How close, in centimetres.
    pub(crate) merge_distance: f32,
    /// Whether each copy's UVs get a square of their own.
    pub(crate) uvs_side_by_side: bool,
}

/// A layout as stored.
fn layout_number(layout: ArrayLayout) -> u8 {
    match layout {
        ArrayLayout::Line => 0,
        ArrayLayout::Grid => 1,
        ArrayLayout::Radial => 2,
    }
}

/// An axis as stored.
fn axis_number(axis: ArrayAxis) -> u8 {
    match axis {
        ArrayAxis::X => 0,
        ArrayAxis::Y => 1,
        ArrayAxis::Z => 2,
    }
}

impl ArrayV54 {
    /// The stored form of `settings`.
    pub(crate) fn of(settings: &ArraySettings) -> Self {
        let o = &settings.options;
        Self {
            layout: layout_number(o.layout),
            count: o.count,
            counts: o.counts,
            relative: o.relative,
            offset: o.offset.to_array(),
            spacing: o.spacing.to_array(),
            angle: o.angle,
            axis: axis_number(o.axis),
            radius: o.radius,
            centre: settings.centre.map(|c| c.get()),
            merge: o.merge,
            merge_distance: o.merge_distance,
            uvs_side_by_side: o.uvs_side_by_side,
        }
    }

    /// The settings read back (brought within their limits by the scene;
    /// a centre that isn't in the file is refused there as damaged).
    pub(crate) fn settings(&self) -> ArraySettings {
        ArraySettings {
            options: ArrayOptions {
                layout: ArrayLayout::ALL
                    .into_iter()
                    .find(|l| layout_number(*l) == self.layout)
                    .unwrap_or_default(),
                count: self.count,
                counts: self.counts,
                relative: self.relative,
                offset: Vec3::from_array(self.offset),
                spacing: Vec3::from_array(self.spacing),
                angle: self.angle,
                axis: ArrayAxis::ALL
                    .into_iter()
                    .find(|a| axis_number(*a) == self.axis)
                    .unwrap_or_default(),
                radius: self.radius,
                merge: self.merge,
                merge_distance: self.merge_distance,
                uvs_side_by_side: self.uvs_side_by_side,
            },
            centre: self.centre.map(ObjectId::from_raw),
        }
    }
}
