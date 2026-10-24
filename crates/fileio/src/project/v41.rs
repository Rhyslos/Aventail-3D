//! Version 41 of the project file's contents: version 40's development
//! model with curve objects
//! ([decision 0138](../../../../docs/decisions/0138-curves-sweep-lathe.md)).
//!
//! Version 40's model holds a curve object as an empty group, in its place
//! in the tree with its name, transform and settings; this version adds
//! each curve: its points (or ready-made settings) and its solid. Curves
//! get a version of their own, as booleans did, because an older app
//! would open them as empty groups without a word and lose them on saving;
//! with a new version it says it needs updating instead. Still CBOR, so
//! settings can be added with a standard value until the alpha.

use super::v1::Problem;
use super::v40::ProjectV40;
use crate::limits::Bounded;
use glam::Vec3;
use scene::{
    Curve, CurveObject, CurvePoint, CurvePointId, CurveShape, CurveSolid, Detail, Document,
    LatheAxis, LatheShape, ObjectId, ObjectKind, PointKind, ReadyCurve, Sweep, SweepProfile,
};
use serde::{Deserialize, Serialize};
use words::{tr, trf};

/// Version 41 of the file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ProjectV41 {
    /// Version 40's model, as it was (curves in it as empty groups).
    pub(crate) project: ProjectV40,
    /// Each curve object.
    #[serde(default = "Bounded::new")]
    pub(crate) curves: Bounded<CurveV41>,
}

/// One curve object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct CurveV41 {
    /// The object (a group in version 40's model).
    pub(crate) id: u64,
    /// Its curve.
    pub(crate) shape: ShapeV41,
    /// What it makes.
    pub(crate) solid: SolidV41,
}

/// A curve: ready-made settings, or points.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum ShapeV41 {
    Line {
        length: f32,
    },
    Circle {
        radius: f32,
    },
    Arc {
        radius: f32,
        angle: f32,
    },
    Spiral {
        radius: f32,
        height: f32,
        turns: f32,
    },
    Points {
        points: Bounded<PointV41>,
        closed: bool,
        next_id: u32,
    },
}

/// One point of a curve.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) struct PointV41 {
    pub(crate) id: u32,
    pub(crate) position: [f32; 3],
    pub(crate) handle_in: [f32; 3],
    pub(crate) handle_out: [f32; 3],
    /// Smooth or auto: kept so the first files of version 41, which had
    /// only this switch, and these read alike.
    pub(crate) smooth: bool,
    /// Its handle kind (decision 0139), added to version 41 before it was
    /// released; a file without it takes the kind from `smooth`.
    #[serde(default)]
    pub(crate) kind: Option<KindV41>,
}

/// A curve point's handle kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum KindV41 {
    Sharp,
    Smooth,
    Broken,
    Auto,
}

impl KindV41 {
    fn of(kind: PointKind) -> Self {
        match kind {
            PointKind::Sharp => Self::Sharp,
            PointKind::Smooth => Self::Smooth,
            PointKind::Broken => Self::Broken,
            PointKind::Auto => Self::Auto,
        }
    }

    fn kind(self) -> PointKind {
        match self {
            Self::Sharp => PointKind::Sharp,
            Self::Smooth => PointKind::Smooth,
            Self::Broken => PointKind::Broken,
            Self::Auto => PointKind::Auto,
        }
    }
}

/// What a curve makes.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) enum SolidV41 {
    Line,
    Sweep(SweepV41),
    Lathe(LatheV41),
}

/// A sweep's settings. New settings are added with a standard value.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct SweepV41 {
    pub(crate) profile: ProfileV41,
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) sides: u32,
    pub(crate) segments: u32,
    pub(crate) adaptive: bool,
    pub(crate) twist: f32,
    pub(crate) caps: bool,
}

/// A sweep's cross-section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ProfileV41 {
    Circle,
    Square,
    Rectangle,
    /// Another curve object, by id.
    Curve(u64),
}

/// A lathe's settings. New settings are added with a standard value.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct LatheV41 {
    pub(crate) axis: AxisV41,
    pub(crate) angle: f32,
    pub(crate) segments: u32,
    pub(crate) along: u32,
    pub(crate) adaptive: bool,
    pub(crate) caps: bool,
}

/// A lathe's axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum AxisV41 {
    X,
    Y,
    Z,
}

impl Default for SweepV41 {
    fn default() -> Self {
        Self::of(&Sweep::default())
    }
}

impl Default for LatheV41 {
    fn default() -> Self {
        Self::of(&LatheShape::default())
    }
}

fn array(v: Vec3) -> [f32; 3] {
    v.to_array()
}

impl SweepV41 {
    fn of(s: &Sweep) -> Self {
        Self {
            profile: match s.profile {
                SweepProfile::Circle => ProfileV41::Circle,
                SweepProfile::Square => ProfileV41::Square,
                SweepProfile::Rectangle => ProfileV41::Rectangle,
                SweepProfile::Curve(id) => ProfileV41::Curve(id.get()),
            },
            width: s.width,
            height: s.height,
            sides: s.sides,
            segments: s.detail.segments,
            adaptive: s.detail.adaptive,
            twist: s.twist,
            caps: s.caps,
        }
    }

    fn sweep(self) -> Sweep {
        Sweep {
            profile: match self.profile {
                ProfileV41::Circle => SweepProfile::Circle,
                ProfileV41::Square => SweepProfile::Square,
                ProfileV41::Rectangle => SweepProfile::Rectangle,
                ProfileV41::Curve(id) => SweepProfile::Curve(ObjectId::from_raw(id)),
            },
            width: self.width,
            height: self.height,
            sides: self.sides,
            detail: Detail {
                segments: self.segments,
                adaptive: self.adaptive,
            },
            twist: self.twist,
            caps: self.caps,
        }
        .clamped()
    }
}

impl LatheV41 {
    fn of(l: &LatheShape) -> Self {
        Self {
            axis: match l.axis {
                LatheAxis::X => AxisV41::X,
                LatheAxis::Y => AxisV41::Y,
                LatheAxis::Z => AxisV41::Z,
            },
            angle: l.angle,
            segments: l.segments,
            along: l.detail.segments,
            adaptive: l.detail.adaptive,
            caps: l.caps,
        }
    }

    fn lathe(self) -> LatheShape {
        LatheShape {
            axis: match self.axis {
                AxisV41::X => LatheAxis::X,
                AxisV41::Y => LatheAxis::Y,
                AxisV41::Z => LatheAxis::Z,
            },
            angle: self.angle,
            segments: self.segments,
            detail: Detail {
                segments: self.along,
                adaptive: self.adaptive,
            },
            caps: self.caps,
        }
        .clamped()
    }
}

impl CurveV41 {
    /// The file form of curve object `id`.
    fn of(id: ObjectId, curve: &CurveObject) -> Self {
        let shape = match &curve.shape {
            CurveShape::Ready(ReadyCurve::Line { length }) => ShapeV41::Line { length: *length },
            CurveShape::Ready(ReadyCurve::Circle { radius }) => {
                ShapeV41::Circle { radius: *radius }
            }
            CurveShape::Ready(ReadyCurve::Arc { radius, angle }) => ShapeV41::Arc {
                radius: *radius,
                angle: *angle,
            },
            CurveShape::Ready(ReadyCurve::Spiral {
                radius,
                height,
                turns,
            }) => ShapeV41::Spiral {
                radius: *radius,
                height: *height,
                turns: *turns,
            },
            CurveShape::Points(c) => ShapeV41::Points {
                points: c
                    .points()
                    .iter()
                    .map(|p| PointV41 {
                        id: p.id.get(),
                        position: array(p.position),
                        handle_in: array(p.handle_in),
                        handle_out: array(p.handle_out),
                        smooth: p.is_smooth(),
                        kind: Some(KindV41::of(p.kind)),
                    })
                    .collect(),
                closed: c.closed(),
                next_id: c.next_id(),
            },
        };
        let solid = match &curve.solid {
            CurveSolid::Line => SolidV41::Line,
            CurveSolid::Sweep(s) => SolidV41::Sweep(SweepV41::of(s)),
            CurveSolid::Lathe(l) => SolidV41::Lathe(LatheV41::of(l)),
        };
        Self {
            id: id.get(),
            shape,
            solid,
        }
    }

    /// The curve object, every value checked or brought within limits.
    fn curve(self) -> Result<CurveObject, Problem> {
        let ready = |r: ReadyCurve| Ok(CurveShape::Ready(r.clamped()));
        let shape = match self.shape {
            ShapeV41::Line { length } => ready(ReadyCurve::Line { length }),
            ShapeV41::Circle { radius } => ready(ReadyCurve::Circle { radius }),
            ShapeV41::Arc { radius, angle } => ready(ReadyCurve::Arc { radius, angle }),
            ShapeV41::Spiral {
                radius,
                height,
                turns,
            } => ready(ReadyCurve::Spiral {
                radius,
                height,
                turns,
            }),
            ShapeV41::Points {
                points,
                closed,
                next_id,
            } => {
                let points = points
                    .0
                    .into_iter()
                    .map(|p| {
                        let (handle_in, handle_out) = (
                            Vec3::from_array(p.handle_in),
                            Vec3::from_array(p.handle_out),
                        );
                        let has_handles = handle_in.length_squared() > 1e-12
                            || handle_out.length_squared() > 1e-12;
                        CurvePoint {
                            id: CurvePointId::from_raw(p.id),
                            position: Vec3::from_array(p.position),
                            handle_in,
                            handle_out,
                            kind: p.kind.map_or_else(
                                || PointKind::from_smooth(p.smooth, has_handles),
                                KindV41::kind,
                            ),
                        }
                    })
                    .collect();
                Curve::from_parts(points, closed, next_id)
                    .map(CurveShape::Points)
                    .map_err(|why| Problem::from(trf!("a curve can't be used: {}", why)))
            }
        }?;
        let solid = match self.solid {
            SolidV41::Line => CurveSolid::Line,
            SolidV41::Sweep(s) => CurveSolid::Sweep(s.sweep()),
            SolidV41::Lathe(l) => CurveSolid::Lathe(l.lathe()),
        };
        Ok(CurveObject { shape, solid })
    }
}

impl ProjectV41 {
    /// The file model of a document.
    pub(crate) fn from_document(document: &Document) -> Self {
        let scene = document.scene();
        let curves = scene
            .walk()
            .iter()
            .filter_map(|(id, _)| match &scene.object(*id)?.kind {
                ObjectKind::Curve(c) => Some(CurveV41::of(*id, c)),
                _ => None,
            })
            .collect();
        Self {
            project: ProjectV40::from_document(document),
            curves,
        }
    }

    /// The document this file model describes, after checking every value.
    pub(crate) fn into_document(self) -> Result<Document, Problem> {
        let mut document = self.project.into_document()?;
        let mut seen = std::collections::BTreeSet::new();
        for entry in self.curves {
            if !seen.insert(entry.id) {
                return Err(tr("a curve is listed twice").into());
            }
            let id = ObjectId::from_raw(entry.id);
            let curve = entry.curve()?;
            document = document.with_curve(id, curve).map_err(Problem::from)?;
        }
        document
            .check_profiles()
            .map_err(|why| Problem::from(trf!("a curve can't be used: {}", why)))?;
        Ok(document)
    }
}

/// The CBOR bytes of `model` (version 43 is written now; tests still
/// write version 41 files).
#[cfg(test)]
pub(crate) fn encode(model: &ProjectV41) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    ciborium::into_writer(model, &mut bytes).map_err(|e| e.to_string())?;
    Ok(bytes)
}

/// The model in CBOR `bytes`; anything after it is refused.
pub(crate) fn decode(bytes: &[u8]) -> Result<ProjectV41, String> {
    let mut reader = std::io::Cursor::new(bytes);
    let model = ciborium::from_reader(&mut reader).map_err(|e| e.to_string())?;
    if reader.position() as usize != bytes.len() {
        return Err(tr("there is unexpected data after the scene").into());
    }
    Ok(model)
}
