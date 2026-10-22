//! Version 39 of the project file's contents: the development format
//! ([decision 0126](../../../../docs/decisions/0126-rework-and-first-use.md)).
//!
//! Until the alpha the file format doesn't get a new version for every
//! feature. Version 39 holds version 38's model, encoded with CBOR (a
//! self-describing encoding) instead of postcard, so fields can be added
//! here with a standard value (`#[serde(default)]`) and older development
//! files still open. Proper versioning starts again at the alpha.
//!
//! Beside version 38's model, each object's development settings:
//! its smoothing (by angle, or by marked edges only), whether its live
//! mirror belongs to the object (moves and turns with it) or to the world,
//! and, for shapes newer than version 38's model knows (decision 0129),
//! their settings: version 38's model holds such a shape as its mesh.

use super::v1::Problem;
use super::v38::{EntryV38, ProjectV38};
use crate::limits::Bounded;
use scene::{Document, Hardness, ObjectId, ObjectKind, Primitive};
use serde::{Deserialize, Serialize};

/// Version 39 of the file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ProjectV39 {
    /// Version 38's model, as it was.
    pub(crate) project: ProjectV38<EntryV38>,
    /// Each object's development settings (objects not listed keep the
    /// standard ones).
    #[serde(default)]
    pub(crate) objects: Bounded<ObjectV39>,
}

/// One object's development settings. New settings are added here, each
/// with a standard value, so files without them still open.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct ObjectV39 {
    /// The object.
    pub(crate) id: u64,
    /// Smoothed by this angle (with marked hard edges added), or only by
    /// marked hard edges (`None`).
    pub(crate) smoothing: Option<f32>,
    /// Whether its live mirror belongs to the object.
    pub(crate) mirror_on_object: bool,
    /// The settings of a shape newer than version 38's model knows.
    pub(crate) shape: Option<ShapeV39>,
}

/// A shape newer than version 38's model knows, with its settings
/// (decision 0129). Read back through [`Primitive::clamped`], so any value
/// is brought within limits.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) enum ShapeV39 {
    Icosphere {
        radius: f32,
        subdivisions: u32,
    },
    Torus {
        radius: f32,
        tube_radius: f32,
        sides: u32,
        tube_sides: u32,
    },
    Capsule {
        radius: f32,
        height: f32,
        sides: u32,
        cap_rings: u32,
        height_segments: u32,
    },
    Dome {
        radius: f32,
        sides: u32,
        rings: u32,
        base: bool,
    },
    Elbow {
        bend_radius: f32,
        pipe_radius: f32,
        inner_radius: f32,
        angle: f32,
        sides: u32,
        segments: u32,
    },
    Sector {
        radius: f32,
        inner_radius: f32,
        angle: f32,
        height: f32,
        sides: u32,
    },
    Gear {
        radius: f32,
        tooth_depth: f32,
        teeth: u32,
        height: f32,
        hole_radius: f32,
    },
    HexGrid {
        size: f32,
        rings: u32,
    },
    HexSphere {
        radius: f32,
        subdivisions: u32,
    },
    Stairs {
        width: f32,
        height: f32,
        depth: f32,
        steps: u32,
    },
    Arch {
        width: f32,
        leg_height: f32,
        thickness: f32,
        depth: f32,
        segments: u32,
    },
}

impl ShapeV39 {
    /// The file form of `p`, if it's one of the newer kinds.
    pub(crate) fn of(p: &Primitive) -> Option<Self> {
        Some(match *p {
            Primitive::Icosphere(c) => Self::Icosphere {
                radius: c.radius,
                subdivisions: c.subdivisions,
            },
            Primitive::Torus(c) => Self::Torus {
                radius: c.radius,
                tube_radius: c.tube_radius,
                sides: c.sides,
                tube_sides: c.tube_sides,
            },
            Primitive::Capsule(c) => Self::Capsule {
                radius: c.radius,
                height: c.height,
                sides: c.sides,
                cap_rings: c.cap_rings,
                height_segments: c.height_segments,
            },
            Primitive::Dome(c) => Self::Dome {
                radius: c.radius,
                sides: c.sides,
                rings: c.rings,
                base: c.base,
            },
            Primitive::Elbow(c) => Self::Elbow {
                bend_radius: c.bend_radius,
                pipe_radius: c.pipe_radius,
                inner_radius: c.inner_radius,
                angle: c.angle,
                sides: c.sides,
                segments: c.segments,
            },
            Primitive::Sector(c) => Self::Sector {
                radius: c.radius,
                inner_radius: c.inner_radius,
                angle: c.angle,
                height: c.height,
                sides: c.sides,
            },
            Primitive::Gear(c) => Self::Gear {
                radius: c.radius,
                tooth_depth: c.tooth_depth,
                teeth: c.teeth,
                height: c.height,
                hole_radius: c.hole_radius,
            },
            Primitive::HexGrid(c) => Self::HexGrid {
                size: c.size,
                rings: c.rings,
            },
            Primitive::HexSphere(c) => Self::HexSphere {
                radius: c.radius,
                subdivisions: c.subdivisions,
            },
            Primitive::Stairs(c) => Self::Stairs {
                width: c.width,
                height: c.height,
                depth: c.depth,
                steps: c.steps,
            },
            Primitive::Arch(c) => Self::Arch {
                width: c.width,
                leg_height: c.leg_height,
                thickness: c.thickness,
                depth: c.depth,
                segments: c.segments,
            },
            _ => return None,
        })
    }

    /// The primitive, its settings brought within limits.
    pub(crate) fn primitive(self) -> Primitive {
        use mesh::primitives as p;
        let primitive = match self {
            Self::Icosphere {
                radius,
                subdivisions,
            } => Primitive::Icosphere(p::IcosphereParams {
                radius,
                subdivisions,
            }),
            Self::Torus {
                radius,
                tube_radius,
                sides,
                tube_sides,
            } => Primitive::Torus(p::TorusParams {
                radius,
                tube_radius,
                sides,
                tube_sides,
            }),
            Self::Capsule {
                radius,
                height,
                sides,
                cap_rings,
                height_segments,
            } => Primitive::Capsule(p::CapsuleParams {
                radius,
                height,
                sides,
                cap_rings,
                height_segments,
            }),
            Self::Dome {
                radius,
                sides,
                rings,
                base,
            } => Primitive::Dome(p::DomeParams {
                radius,
                sides,
                rings,
                base,
            }),
            Self::Elbow {
                bend_radius,
                pipe_radius,
                inner_radius,
                angle,
                sides,
                segments,
            } => Primitive::Elbow(p::ElbowParams {
                bend_radius,
                pipe_radius,
                inner_radius,
                angle,
                sides,
                segments,
            }),
            Self::Sector {
                radius,
                inner_radius,
                angle,
                height,
                sides,
            } => Primitive::Sector(p::SectorParams {
                radius,
                inner_radius,
                angle,
                height,
                sides,
            }),
            Self::Gear {
                radius,
                tooth_depth,
                teeth,
                height,
                hole_radius,
            } => Primitive::Gear(p::GearParams {
                radius,
                tooth_depth,
                teeth,
                height,
                hole_radius,
            }),
            Self::HexGrid { size, rings } => Primitive::HexGrid(p::HexGridParams { size, rings }),
            Self::HexSphere {
                radius,
                subdivisions,
            } => Primitive::HexSphere(p::HexSphereParams {
                radius,
                subdivisions,
            }),
            Self::Stairs {
                width,
                height,
                depth,
                steps,
            } => Primitive::Stairs(p::StairsParams {
                width,
                height,
                depth,
                steps,
            }),
            Self::Arch {
                width,
                leg_height,
                thickness,
                depth,
                segments,
            } => Primitive::Arch(p::ArchParams {
                width,
                leg_height,
                thickness,
                depth,
                segments,
            }),
        };
        primitive.clamped()
    }
}

impl Default for ObjectV39 {
    fn default() -> Self {
        Self {
            id: 0,
            smoothing: Some(scene::STANDARD_SMOOTHING_ANGLE),
            mirror_on_object: false,
            shape: None,
        }
    }
}

impl ProjectV39 {
    /// The file model of a document.
    pub(crate) fn from_document(document: &Document) -> Self {
        let scene = document.scene();
        let objects = scene
            .walk()
            .iter()
            .filter_map(|(id, _)| {
                let object = scene.object(*id)?;
                Some(ObjectV39 {
                    id: id.get(),
                    smoothing: match object.hardness {
                        Hardness::ByAngle(angle) => Some(angle),
                        Hardness::Marked => None,
                    },
                    mirror_on_object: object.mirror.is_some_and(|m| m.on_object),
                    shape: match &object.kind {
                        ObjectKind::Primitive(p) => ShapeV39::of(p),
                        _ => None,
                    },
                })
            })
            .collect();
        Self {
            project: ProjectV38::from_document(document),
            objects,
        }
    }

    /// The document this file model describes, after checking every value.
    pub(crate) fn into_document(self) -> Result<Document, Problem> {
        let mut document = self.project.into_document()?;
        let mut seen = std::collections::BTreeSet::new();
        for object in self.objects {
            if !seen.insert(object.id) {
                return Err("an object's settings are listed twice".into());
            }
            let hardness = object.smoothing.map_or(Hardness::Marked, Hardness::ByAngle);
            document = document
                .with_object_settings(
                    ObjectId::from_raw(object.id),
                    hardness,
                    object.mirror_on_object,
                )
                .map_err(Problem::from)?;
            if let Some(shape) = object.shape {
                document = document
                    .with_primitive(ObjectId::from_raw(object.id), shape.primitive())
                    .map_err(Problem::from)?;
            }
        }
        Ok(document)
    }
}

/// The CBOR bytes of `model`.
pub(crate) fn encode(model: &ProjectV39) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    ciborium::into_writer(model, &mut bytes).map_err(|e| e.to_string())?;
    Ok(bytes)
}

/// The model in CBOR `bytes`; anything after it is refused.
pub(crate) fn decode(bytes: &[u8]) -> Result<ProjectV39, String> {
    let mut reader = std::io::Cursor::new(bytes);
    let model = ciborium::from_reader(&mut reader).map_err(|e| e.to_string())?;
    if reader.position() as usize != bytes.len() {
        return Err("there is unexpected data after the scene".into());
    }
    Ok(model)
}
