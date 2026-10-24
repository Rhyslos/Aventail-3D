//! Curve objects ([decision 0138](../../../docs/decisions/0138-curves-sweep-lathe.md)):
//! a Bézier curve (ready-made from settings, or points drawn and edited)
//! and the solid it makes, if any: a sweep of a cross-section along it,
//! or a lathe spinning it round an axis. Both are live: change the curve
//! or a setting and the solid follows. A curve with neither is drawn as
//! a line and has no shape of its own (nothing to export or cut with).

use crate::ObjectId;
use mesh::Mesh;
use mesh::curve::{Curve, Detail, LatheShape, Profile, ReadyCurve, SweepShape, lathe, sweep};
use words::{text, tr};

/// A curve object's curve: ready-made (its settings stay editable, like a
/// primitive's) or its own points. Editing a ready-made curve's points
/// turns it into points first.
#[derive(Debug, Clone, PartialEq)]
pub enum CurveShape {
    /// A ready-made curve with its settings.
    Ready(ReadyCurve),
    /// Points, drawn or edited.
    Points(Curve),
}

impl CurveShape {
    /// The curve, in the object's own space.
    pub fn curve(&self) -> Curve {
        match self {
            Self::Ready(ready) => ready.curve(),
            Self::Points(curve) => curve.clone(),
        }
    }

    /// The name new objects of it get: the ready-made kind's, or "Curve".
    pub fn name(&self) -> &'static str {
        match self {
            Self::Ready(ready) => ready.name(),
            Self::Points(_) => tr("Curve"),
        }
    }
}

/// The cross-section a sweep carries along its curve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SweepProfile {
    /// A circle `width` across, with `sides`: pipes and cables.
    Circle,
    /// A square `width` across.
    Square,
    /// A rectangle `width` across and `height` up.
    Rectangle,
    /// Another curve object's curve, laid flat on its plane, round its
    /// pivot: rails, trims and mouldings.
    Curve(ObjectId),
}

impl SweepProfile {
    /// The built-in profiles, in the order the Attributes panel lists them.
    pub const BUILT_IN: [Self; 3] = [Self::Circle, Self::Square, Self::Rectangle];

    /// Its name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Circle => tr("Circle"),
            Self::Square => tr("Square"),
            Self::Rectangle => tr("Rectangle"),
            Self::Curve(_) => tr("Another curve"),
        }
    }
}

/// A sweep's settings (lengths in centimetres, angles in degrees).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sweep {
    /// The cross-section.
    pub profile: SweepProfile,
    /// How wide the built-in cross-section is (a circle's diameter).
    pub width: f32,
    /// How tall a rectangle is.
    pub height: f32,
    /// How many sides a circle has.
    pub sides: u32,
    /// How finely it follows the curve.
    pub detail: Detail,
    /// How far the cross-section turns from start to end.
    pub twist: f32,
    /// Whether an open curve's ends are closed.
    pub caps: bool,
}

impl Default for Sweep {
    fn default() -> Self {
        Self {
            profile: SweepProfile::Circle,
            width: 10.0,
            height: 5.0,
            sides: 12,
            detail: Detail::default(),
            twist: 0.0,
            caps: true,
        }
    }
}

/// `value` within `low` to `high`; not a number becomes `standard`.
fn within(value: f32, low: f32, high: f32, standard: f32) -> f32 {
    if value.is_finite() {
        value.clamp(low, high)
    } else {
        standard
    }
}

impl Sweep {
    /// The same settings within limits.
    pub fn clamped(self) -> Self {
        let standard = Self::default();
        Self {
            width: within(self.width, sweep::MIN_SIZE, sweep::MAX_SIZE, standard.width),
            height: within(
                self.height,
                sweep::MIN_SIZE,
                sweep::MAX_SIZE,
                standard.height,
            ),
            sides: self.sides.clamp(*sweep::SIDES.start(), *sweep::SIDES.end()),
            detail: self.detail.clamped(),
            twist: within(self.twist, -sweep::MAX_TWIST, sweep::MAX_TWIST, 0.0),
            ..self
        }
    }

    /// Its built-in cross-section, or for another curve's, that curve's
    /// (`None` if it has less than a line).
    fn cross_section(&self, profile: Option<&Curve>) -> Option<Profile> {
        match self.profile {
            SweepProfile::Circle => Some(Profile::circle(self.width, self.sides)),
            SweepProfile::Square => Some(Profile::rectangle(self.width, self.width)),
            SweepProfile::Rectangle => Some(Profile::rectangle(self.width, self.height)),
            SweepProfile::Curve(_) => Profile::from_curve(profile?, self.detail),
        }
    }
}

/// The solid a curve makes, if any.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CurveSolid {
    /// None: the curve is a line (a guide, or another sweep's profile).
    Line,
    /// A cross-section carried along the curve.
    Sweep(Sweep),
    /// The curve spun round one of its axes.
    Lathe(LatheShape),
}

impl CurveSolid {
    /// The name of its kind, for menus.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Line => tr("Line only"),
            Self::Sweep(_) => tr("Sweep"),
            Self::Lathe(_) => tr("Lathe"),
        }
    }

    /// The same settings within limits.
    pub fn clamped(self) -> Self {
        match self {
            Self::Line => Self::Line,
            Self::Sweep(s) => Self::Sweep(s.clamped()),
            Self::Lathe(l) => Self::Lathe(l.clamped()),
        }
    }

    /// The other curve object it uses as its cross-section, if any.
    pub fn profile_object(&self) -> Option<ObjectId> {
        match self {
            Self::Sweep(Sweep {
                profile: SweepProfile::Curve(id),
                ..
            }) => Some(*id),
            _ => None,
        }
    }
}

/// A curve object: its curve and the solid it makes.
#[derive(Debug, Clone, PartialEq)]
pub struct CurveObject {
    /// The curve.
    pub shape: CurveShape,
    /// What it makes.
    pub solid: CurveSolid,
}

impl CurveObject {
    /// A curve of `shape` that is only a line.
    pub fn line(shape: CurveShape) -> Self {
        Self {
            shape,
            solid: CurveSolid::Line,
        }
    }

    /// The solid's shape, in the object's own space, with `profile` the
    /// curve of the object its sweep takes its cross-section from (if it
    /// does). `None` for a curve that is only a line.
    pub fn mesh(&self, profile: Option<&Curve>) -> Option<Mesh> {
        if self.solid == CurveSolid::Line {
            return None;
        }
        let curve = self.shape.curve();
        match self.solid.clamped() {
            CurveSolid::Line => None,
            CurveSolid::Sweep(s) => {
                let shape = SweepShape {
                    detail: s.detail,
                    twist: s.twist,
                    caps: s.caps,
                };
                Some(match s.cross_section(profile) {
                    Some(section) => sweep::sweep(&curve, &section, &shape),
                    None => Mesh::new(),
                })
            }
            CurveSolid::Lathe(l) => Some(lathe::lathe(&curve, &l)),
        }
    }

    /// Why its solid shows nothing, if it doesn't: for the Attributes
    /// panel.
    pub fn problem(&self, profile: Option<&Curve>) -> Option<&'static str> {
        let curve = self.shape.curve();
        if curve.is_empty() {
            return Some(tr("the curve's points are all on one spot"));
        }
        match self.solid {
            CurveSolid::Sweep(s) if s.cross_section(profile).is_none() => Some(match profile {
                None => tr("pick a curve for the cross-section"),
                Some(_) => tr("the cross-section curve's points are all on one spot"),
            }),
            CurveSolid::Lathe(l) => {
                let axis = l.axis.vector();
                let off = curve
                    .polyline(4)
                    .iter()
                    .any(|p| (*p - axis * p.dot(axis)).length() > 1e-3);
                (!off).then(|| tr("the whole curve lies on the axis it spins round"))
            }
            _ => None,
        }
    }
}

/// Why a curve change was refused (decision 0138), marked for the
/// language layer.
pub(crate) const NOT_A_CURVE: &str = text!("it isn't a curve");
/// See [`NOT_A_CURVE`].
pub(crate) const RECORD_ON_CURVE: &str =
    text!("a curve's shape comes from its points: use Bake into shape to edit it as a mesh");
/// See [`NOT_A_CURVE`].
pub(crate) const PROFILE_NOT_A_CURVE: &str =
    text!("the cross-section has to be another curve object");
/// See [`NOT_A_CURVE`].
pub(crate) const PROFILE_ITSELF: &str = text!("a curve can't be its own cross-section");
