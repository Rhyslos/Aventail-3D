//! Version 59 of the project file ([decision 0158](../../../../docs/decisions/0158-bend-twist-taper.md)):
//! version 58's, with the *Bend, twist or taper* modifier
//! (`v43::KindV43::BendTwistTaper`, its settings stored as
//! [`AxisDeformV59`]). Files before version 59 never hold it.

use scene::{AxisDeformKind, AxisDeformOptions, DeformAxis};
use serde::{Deserialize, Serialize};

/// The *Bend, twist or taper* modifier's settings as stored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct AxisDeformV59 {
    /// 0 bend, 1 twist, 2 taper (anything else is a bend).
    pub(crate) kind: u8,
    /// 0 X, 1 Y, 2 Z (anything else is Z).
    pub(crate) axis: u8,
    /// A bend's or twist's angle, in degrees.
    pub(crate) angle: f32,
    /// A bend's direction round the axis, in degrees.
    pub(crate) direction: f32,
    /// A taper's amount, as a share.
    pub(crate) amount: f32,
    /// A taper's curve, as a share.
    pub(crate) curve: f32,
    /// The limits along the axis, as shares of the shape's length.
    pub(crate) limits: [f32; 2],
    /// The centre from the pivot, in centimetres.
    pub(crate) centre: [f32; 3],
}

/// A kind as stored.
fn kind_number(kind: AxisDeformKind) -> u8 {
    match kind {
        AxisDeformKind::Bend => 0,
        AxisDeformKind::Twist => 1,
        AxisDeformKind::Taper => 2,
    }
}

/// An axis as stored.
fn axis_number(axis: DeformAxis) -> u8 {
    match axis {
        DeformAxis::X => 0,
        DeformAxis::Y => 1,
        DeformAxis::Z => 2,
    }
}

impl AxisDeformV59 {
    /// The stored form of `options`.
    pub(crate) fn of(options: &AxisDeformOptions) -> Self {
        Self {
            kind: kind_number(options.kind),
            axis: axis_number(options.axis),
            angle: options.angle,
            direction: options.direction,
            amount: options.amount,
            curve: options.curve,
            limits: options.limits,
            centre: options.centre.to_array(),
        }
    }

    /// The settings read back (brought within their limits by the scene).
    pub(crate) fn options(&self) -> AxisDeformOptions {
        AxisDeformOptions {
            kind: AxisDeformKind::ALL
                .into_iter()
                .find(|k| kind_number(*k) == self.kind)
                .unwrap_or_default(),
            axis: DeformAxis::ALL
                .into_iter()
                .find(|a| axis_number(*a) == self.axis)
                .unwrap_or_default(),
            angle: self.angle,
            direction: self.direction,
            amount: self.amount,
            curve: self.curve,
            limits: self.limits,
            centre: glam::Vec3::from_array(self.centre),
        }
    }
}
