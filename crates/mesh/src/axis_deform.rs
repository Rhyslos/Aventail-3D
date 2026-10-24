//! Bend, twist and taper ([decision 0158](../../../docs/decisions/0158-bend-twist-taper.md)):
//! the work of the *Bend, twist or taper* modifier, one modifier with a
//! **Kind** choice, built on the deformer groundwork ([`crate::deform`],
//! decision 0144).
//!
//! Each works in **its own space**: its origin at the **Centre** (the
//! object's pivot moved by [`AxisDeformOptions::centre`]), its Z along the
//! chosen **Axis** of the shape, and (for a bend) its X the way the shape
//! bends, turned round the axis by [`AxisDeformOptions::direction`]
//! ([`frame`]).
//!
//! It acts between **Limits** along the axis: shares of the shape's
//! length along it (0 to 1, shown as 0 % to 100 %), so the standard 0 to
//! 1 covers the whole shape whatever its size ([`limits`]). Past the
//! limits the shape carries on rigidly, as the limit's end leaves it
//! ([`crate::deform::AxisRange::clamp`]). The centre's place along the
//! axis, brought within the limits, stays where it is: the shape bends,
//! twists and tapers *away from* it.
//!
//! - **Bend:** the stretch within the limits curls into an arc turning
//!   through **Angle** in all, towards the bend's X.
//! - **Twist:** each slice across the axis turns round it, by **Angle**
//!   in all over the stretch.
//! - **Taper:** each slice is scaled across the axis, by `1 + Amount` at
//!   one share of the length from the centre, with **Curve** bulging the
//!   middle of the stretch out (or in, below 0). Never below nothing.
//!
//! Only points move: polygons, ids, UVs, seams, creases, hard edges and
//! materials stay, as for every deformer. Limits with no length change
//! nothing. Deterministic.

use crate::Mesh;
use crate::deform::{AxisRange, DeformProblem, Deformer, deform};
use glam::{Mat4, Vec3};
use words::tr;

/// What the modifier does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AxisDeformKind {
    /// Curls the shape into an arc.
    #[default]
    Bend,
    /// Turns each slice round the axis.
    Twist,
    /// Scales each slice across the axis.
    Taper,
}

impl AxisDeformKind {
    /// Every kind, in the order the interface lists them.
    pub const ALL: [Self; 3] = [Self::Bend, Self::Twist, Self::Taper];

    /// Its name, as the interface shows it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Bend => tr("Bend"),
            Self::Twist => tr("Twist"),
            Self::Taper => tr("Taper"),
        }
    }
}

/// The shape's own axis a deformer works along.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DeformAxis {
    /// Its X.
    X,
    /// Its Y.
    Y,
    /// Its Z, up: a standing shape bends over, twists and tapers upward.
    #[default]
    Z,
}

impl DeformAxis {
    /// Every axis, in order.
    pub const ALL: [Self; 3] = [Self::X, Self::Y, Self::Z];

    /// Its name, as the interface shows it.
    pub fn name(self) -> &'static str {
        match self {
            Self::X => tr("X"),
            Self::Y => tr("Y"),
            Self::Z => tr("Z"),
        }
    }

    /// The deformer's X, Y and Z in the shape's space before
    /// [`AxisDeformOptions::direction`] turns them: the axis is Z, the
    /// next axis round (X → Y → Z → X) after it is X.
    fn basis(self) -> [Vec3; 3] {
        match self {
            Self::X => [Vec3::Y, Vec3::Z, Vec3::X],
            Self::Y => [Vec3::Z, Vec3::X, Vec3::Y],
            Self::Z => [Vec3::X, Vec3::Y, Vec3::Z],
        }
    }
}

/// The *Bend, twist or taper* modifier's settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisDeformOptions {
    /// Bend, twist or taper.
    pub kind: AxisDeformKind,
    /// The shape's axis it works along.
    pub axis: DeformAxis,
    /// A bend's or twist's angle over the whole stretch within the
    /// limits, in degrees (below 0, the other way).
    pub angle: f32,
    /// Which way a bend goes, in degrees round the axis: 0 towards the
    /// next axis round (Z bends towards X, X towards Y, Y towards Z).
    pub direction: f32,
    /// A taper's change in size, as a share, one whole stretch from the
    /// centre: -0.5 makes it half as wide there, 1 twice as wide.
    pub amount: f32,
    /// A taper's bulge in the middle of the stretch, as a share (below 0,
    /// pinched in).
    pub curve: f32,
    /// Where the stretch it works on starts and ends along the axis, as
    /// shares of the shape's length along it (0 to 1, either order).
    pub limits: [f32; 2],
    /// Where its centre is, from the object's pivot, in centimetres in
    /// the shape's space.
    pub centre: Vec3,
}

impl Default for AxisDeformOptions {
    fn default() -> Self {
        Self {
            kind: AxisDeformKind::Bend,
            axis: DeformAxis::Z,
            angle: Self::STANDARD_ANGLE,
            direction: 0.0,
            amount: Self::STANDARD_AMOUNT,
            curve: 0.0,
            limits: [0.0, 1.0],
            centre: Vec3::ZERO,
        }
    }
}

impl AxisDeformOptions {
    /// The standard angle of a bend or twist: 45°.
    pub const STANDARD_ANGLE: f32 = 45.0;
    /// The standard taper: half as wide one stretch from the centre.
    pub const STANDARD_AMOUNT: f32 = -0.5;
    /// The largest angle either way: ten whole turns.
    pub const MAX_ANGLE: f32 = 3_600.0;
    /// The largest taper amount or curve either way (ten times as wide).
    pub const MAX_AMOUNT: f32 = 10.0;
    /// The furthest the centre may be from the pivot along each axis, in
    /// centimetres (1 km).
    pub const MAX_CENTRE: f32 = 100_000.0;

    /// The same options within limits; a number that isn't a real number
    /// becomes the standard one. The direction is brought within 0° to
    /// 360°.
    pub fn clamped(self) -> Self {
        let within = |value: f32, most: f32, standard: f32| {
            if value.is_finite() {
                value.clamp(-most, most)
            } else {
                standard
            }
        };
        let share = |value: f32, standard: f32| {
            if value.is_finite() {
                value.clamp(0.0, 1.0)
            } else {
                standard
            }
        };
        let direction = if self.direction.is_finite() {
            let d = self.direction.rem_euclid(360.0);
            // `rem_euclid` can round up to 360 itself for tiny negatives.
            if d >= 360.0 { 0.0 } else { d }
        } else {
            0.0
        };
        Self {
            kind: self.kind,
            axis: self.axis,
            angle: within(self.angle, Self::MAX_ANGLE, Self::STANDARD_ANGLE),
            direction,
            amount: within(self.amount, Self::MAX_AMOUNT, Self::STANDARD_AMOUNT),
            curve: within(self.curve, Self::MAX_AMOUNT, 0.0),
            limits: [share(self.limits[0], 0.0), share(self.limits[1], 1.0)],
            centre: Vec3::new(
                within(self.centre.x, Self::MAX_CENTRE, 0.0),
                within(self.centre.y, Self::MAX_CENTRE, 0.0),
                within(self.centre.z, Self::MAX_CENTRE, 0.0),
            ),
        }
    }
}

/// The deformer's space in the shape's: a matrix carrying its points
/// (centre at the origin, the axis along Z, a bend's way along X) into
/// the shape's space. `pivot` is the object's pivot in the shape's space.
/// Always turns and moves only, never scales.
pub fn frame(options: &AxisDeformOptions, pivot: Vec3) -> Mat4 {
    let options = options.clamped();
    let [x, y, z] = options.axis.basis();
    let (sin, cos) = options.direction.to_radians().sin_cos();
    let along = x * cos + y * sin;
    let across = z.cross(along);
    let origin = if pivot.is_finite() { pivot } else { Vec3::ZERO } + options.centre;
    Mat4::from_cols(
        along.extend(0.0),
        across.extend(0.0),
        z.extend(0.0),
        origin.extend(1.0),
    )
}

/// Where the shape lies along the deformer's axis (in its space: from the
/// centre), and the stretch within the limits, for `mesh` in the shape's
/// space. `None` for a shape with no points.
pub fn limits(
    mesh: &Mesh,
    options: &AxisDeformOptions,
    pivot: Vec3,
) -> Option<(AxisRange, AxisRange)> {
    let options = options.clamped();
    let into = frame(&options, pivot).inverse();
    let (mut low, mut high) = (f32::INFINITY, f32::NEG_INFINITY);
    for (_, p) in mesh.vertices() {
        let z = into.transform_point3(p).z;
        if z.is_finite() {
            low = low.min(z);
            high = high.max(z);
        }
    }
    let whole = AxisRange::new(low, high)?;
    let at = |share: f32| whole.from() + whole.length() * share;
    let range = AxisRange::new(at(options.limits[0]), at(options.limits[1]))?;
    Some((whole, range))
}

/// Roughly how much work deforming `mesh` is, in polygons of a boolean's
/// worth: about 20 points cost as much as one (decision 0144), so from
/// 100 000 points the viewport works it out on another thread.
pub fn work(mesh: &Mesh) -> usize {
    mesh.vertex_count() / 20
}

/// `mesh` bent, twisted or tapered as `options` says, round `pivot` (the
/// object's pivot, in the shape's space). A shape with no points, or
/// limits with no length, comes back as it was.
pub fn axis_deform(
    mesh: &Mesh,
    options: &AxisDeformOptions,
    pivot: Vec3,
) -> Result<Mesh, DeformProblem> {
    let options = options.clamped();
    let Some((_, range)) = limits(mesh, &options, pivot) else {
        return Ok(mesh.clone());
    };
    let Some(deformer) = AxisDeformer::new(&options, range) else {
        return Ok(mesh.clone());
    };
    deform(mesh, frame(&options, pivot).inverse(), &deformer)
}

/// The deformer itself, in its own space (see the module notes).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisDeformer {
    kind: AxisDeformKind,
    /// The stretch it works on along Z.
    range: AxisRange,
    /// The place along Z that stays: the centre, within the range.
    still: f32,
    /// A bend's or twist's turn per centimetre along Z, in radians.
    rate: f32,
    amount: f32,
    curve: f32,
}

impl AxisDeformer {
    /// The deformer for `options` over `range` (along its Z). `None` when
    /// it would change nothing: a range with no length, or no angle,
    /// amount or curve.
    pub fn new(options: &AxisDeformOptions, range: AxisRange) -> Option<Self> {
        let options = options.clamped();
        let length = range.length();
        if length <= 1e-6 * range.from().abs().max(range.to().abs()).max(1.0) {
            return None;
        }
        let rate = options.angle.to_radians() / length;
        let nothing = match options.kind {
            AxisDeformKind::Bend | AxisDeformKind::Twist => rate == 0.0,
            AxisDeformKind::Taper => options.amount == 0.0 && options.curve == 0.0,
        };
        (!nothing).then_some(Self {
            kind: options.kind,
            range,
            still: range.clamp(0.0),
            rate,
            amount: options.amount,
            curve: options.curve,
        })
    }

    /// Where the axis's point at `z` goes, and its way along and across
    /// the bend there (for a bend; straight up and along X otherwise).
    fn bent_axis(&self, z: f32) -> (Vec3, Vec3, Vec3) {
        let at = self.range.clamp(z);
        let d = at - self.still;
        let turn = self.rate * d;
        let (sin, cos) = turn.sin_cos();
        // `sin(turn) / rate` and `(1 - cos(turn)) / rate`, written so a
        // tiny turn (a huge radius) loses no precision.
        let (up, out) = if turn.abs() < 1e-4 {
            (d, d * turn * 0.5)
        } else {
            (d * sin / turn, d * (1.0 - cos) / turn)
        };
        let along = Vec3::new(sin, 0.0, cos);
        let outward = Vec3::new(cos, 0.0, -sin);
        let on = Vec3::new(out, 0.0, self.still + up) + along * (z - at);
        (on, along, outward)
    }
}

impl Deformer for AxisDeformer {
    fn moved(&self, p: Vec3) -> Vec3 {
        let at = self.range.clamp(p.z);
        match self.kind {
            AxisDeformKind::Bend => {
                let (on, _, outward) = self.bent_axis(p.z);
                on + outward * p.x + Vec3::Y * p.y
            }
            AxisDeformKind::Twist => {
                let (sin, cos) = (self.rate * (at - self.still)).sin_cos();
                Vec3::new(p.x * cos - p.y * sin, p.x * sin + p.y * cos, p.z)
            }
            AxisDeformKind::Taper => {
                let from_centre = (at - self.still) / self.range.length();
                let t = self.range.fraction(p.z);
                let scale =
                    (1.0 + self.amount * from_centre + self.curve * 4.0 * t * (1.0 - t)).max(0.0);
                Vec3::new(p.x * scale, p.y * scale, p.z)
            }
        }
    }
}

/// What the viewport draws to show a deformer (decision 0158), in the
/// shape's space: its axis through the shape as it bends, twists or
/// tapers, a cross at each end of the stretch it works on (turned and
/// scaled with it), and its centre.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DeformHelper {
    /// The axis, end to end of the shape, as a line through these points.
    pub axis: Vec<Vec3>,
    /// The crosses at the limits: two lines each.
    pub limits: Vec<[Vec3; 2]>,
    /// The centre (the object's pivot moved by the Centre setting).
    pub centre: Vec3,
}

/// The helper showing `options` on `mesh` (in the shape's space) with the
/// object's pivot at `pivot`. `None` for a shape with no points.
pub fn helper(mesh: &Mesh, options: &AxisDeformOptions, pivot: Vec3) -> Option<DeformHelper> {
    /// Lines along the axis.
    const PIECES: u32 = 32;
    let options = options.clamped();
    let (whole, range) = limits(mesh, &options, pivot)?;
    let out = frame(&options, pivot);
    let deformer = AxisDeformer::new(&options, range);
    let moved = |p: Vec3| out.transform_point3(deformer.map_or(p, |d| d.moved(p)));
    let axis = (0..=PIECES)
        .map(|i| moved(Vec3::Z * (whole.from() + whole.length() * i as f32 / PIECES as f32)))
        .collect();
    // Each cross a fifth of the shape's length across, at least 1 cm.
    let half = (whole.length() * 0.1).max(0.5);
    let limits = [range.from(), range.to()]
        .into_iter()
        .flat_map(|z| {
            [
                [
                    moved(Vec3::new(-half, 0.0, z)),
                    moved(Vec3::new(half, 0.0, z)),
                ],
                [
                    moved(Vec3::new(0.0, -half, z)),
                    moved(Vec3::new(0.0, half, z)),
                ],
            ]
        })
        .collect();
    Some(DeformHelper {
        axis,
        limits,
        centre: out.transform_point3(Vec3::ZERO),
    })
}

#[cfg(test)]
mod tests;
