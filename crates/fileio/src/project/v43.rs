//! Version 43 of the project file's contents: version 41's development
//! model (also used by version 42) with each object's modifier stack
//! ([decision 0142](../../../../docs/decisions/0142-modifier-stack-and-remesh.md)).
//!
//! Modifiers get a version of their own, as booleans and curves did,
//! because an older app would skip them without a word: it would show
//! the shapes unmodified and lose the modifiers on saving. With a new
//! version, the older app says it needs updating instead.
//!
//! A new kind of modifier adds a variant to [`KindV43`] and moves
//! [`super::FORMAT_VERSION`] up by one (as version 42 added an edit to
//! version 41's model), so an older app says it needs updating rather
//! than failing to read the kind. Still CBOR, so settings can be added
//! to a kind with a standard value (`#[serde(default)]`) until the alpha.
//!
//! Version 44 ([decision 0143](../../../../docs/decisions/0143-subdivide-and-creases.md))
//! adds the Subdivide modifier here, and one more kind of recorded edit,
//! creases (`v37::OperationV37::SetCreases`): files before version 44
//! never hold either.
//!
//! Version 45 ([decision 0144](../../../../docs/decisions/0144-lattice-cage.md))
//! adds lattice cages ([`ProjectV43::lattices`], see `v45`) and the
//! Lattice modifier naming one; files before version 45 hold neither.
//!
//! Version 46 ([decision 0145](../../../../docs/decisions/0145-decimate.md))
//! adds the Decimate modifier; files before version 46 never hold it.
//!
//! Version 47 ([decision 0146](../../../../docs/decisions/0146-scatter.md))
//! adds the Scatter modifier, naming the shapes it scatters; files before
//! version 47 never hold it.
//!
//! Version 48 ([decision 0147](../../../../docs/decisions/0147-curve-deform-and-shrinkwrap.md))
//! adds the Bend along curve and Shrinkwrap modifiers, naming the curve or
//! the target they read; files before version 48 never hold them.
//!
//! Version 49 ([decision 0148](../../../../docs/decisions/0148-weighted-normals.md))
//! adds the Weighted normals modifier; files before version 49 never hold
//! it. (Custom normals baked into a shape are a corner attribute layer,
//! saved as every layer is since version 5.)

use super::v1::Problem;
use super::v41::ProjectV41;
use crate::limits::Bounded;
use scene::{
    BendAxis, BendOptions, CurveBendSettings, DecimateKeep, DecimateOptions, Document, FaceId,
    LatticeOptions, Modifier, ModifierKind, ObjectId, PlaceOn, ReduceBy, RemeshOptions,
    ScatterOptions, ScatterSettings, ShrinkwrapOptions, ShrinkwrapSettings, SubdivideOptions,
    VertexId, WeightBy, WeightedNormalsOptions, WrapMode,
};
use serde::{Deserialize, Serialize};
use words::{tr, trf};

/// Version 43 of the file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ProjectV43 {
    /// Version 41's model, as it was.
    pub(crate) project: ProjectV41,
    /// Each object's modifiers (objects not listed have none).
    #[serde(default = "Bounded::new")]
    pub(crate) modifiers: Bounded<ModifiersV43>,
    /// Each lattice cage (version 45; the objects are empty groups in
    /// version 41's model).
    #[serde(default = "Bounded::new")]
    pub(crate) lattices: Bounded<super::v45::LatticeV45>,
}

/// One object's modifiers, first to last.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ModifiersV43 {
    /// The object.
    pub(crate) id: u64,
    /// Its modifiers.
    pub(crate) list: Bounded<ModifierV43>,
}

/// One modifier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ModifierV43 {
    /// What it does, with its settings.
    pub(crate) kind: KindV43,
    /// Whether it is on.
    #[serde(default = "on")]
    pub(crate) on: bool,
    /// Whether the viewport shows it.
    #[serde(default = "on")]
    pub(crate) in_viewport: bool,
}

fn on() -> bool {
    true
}

/// A modifier's kind and settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum KindV43 {
    /// Voxel remesh.
    Remesh {
        /// Voxel size, in centimetres.
        voxel_size: f32,
        /// Relax passes.
        smooth: u32,
    },
    /// Subdivision smoothing (version 44).
    Subdivide {
        /// Levels in the viewport.
        levels: u32,
        /// Levels for exports and baking.
        export_levels: u32,
        /// Whether hard edges stay sharp.
        hard_edges_sharp: bool,
    },
    /// A lattice cage's bend (version 45).
    Lattice {
        /// The cage object.
        cage: u64,
    },
    /// Decimation (version 46).
    Decimate {
        /// Whether the target is a number of triangles (else a ratio).
        by_triangles: bool,
        /// The share of triangles kept.
        ratio: f32,
        /// The triangles to end with.
        triangles: u32,
        /// Whether UV seams stay.
        keep_seams: bool,
        /// Whether hard edges stay.
        keep_hard_edges: bool,
        /// Whether material borders stay.
        keep_materials: bool,
        /// Whether open borders stay.
        keep_borders: bool,
        /// Whether flat pairs of triangles join into quads.
        quads_where_flat: bool,
    },
    /// Copies of other shapes spread over the surface (version 47).
    Scatter {
        /// Where the copies go: 0 points, 1 polygon centres, 2 random
        /// (anything else is random).
        place_on: u8,
        /// How many copies a random spread makes.
        count: u32,
        /// The seed of the random choices.
        seed: u32,
        /// The least distance between copies, in centimetres.
        spacing: f32,
        /// Whether copies stand on the surface.
        align: bool,
        /// The most random turn, in degrees.
        random_turn: f32,
        /// The smallest random size.
        scale_min: f32,
        /// The largest random size.
        scale_max: f32,
        /// The offset along the surface's direction, in centimetres.
        offset: f32,
        /// The shapes scattered.
        shapes: Bounded<u64>,
        /// The polygons copies go on (none for the whole surface).
        #[serde(default = "Bounded::new")]
        polygons: Bounded<u32>,
    },
    /// The shape bent along a curve object (version 48).
    BendAlongCurve {
        /// The curve object.
        curve: u64,
        /// Which axis runs along it: 0 the longest, 1 X, 2 Y, 3 Z
        /// (anything else is the longest).
        axis: u8,
        /// Whether the shape is stretched to the curve's length.
        stretch: bool,
        /// How far along the curve it starts, in centimetres.
        offset: f32,
        /// The twist from start to end, in degrees.
        twist: f32,
    },
    /// The shape's points moved onto a target's surface (version 48).
    Shrinkwrap {
        /// The target object.
        target: u64,
        /// How points find the surface: 0 nearest, 1 along normals, 2 to
        /// 4 along X, Y or Z (anything else is nearest).
        mode: u8,
        /// How far off the surface, in centimetres.
        offset: f32,
        /// The points that move (none for all).
        #[serde(default = "Bounded::new")]
        points: Bounded<u32>,
    },
    /// Weighted normals (version 49).
    WeightedNormals {
        /// What makes a polygon count for more: 0 area, 1 angle, 2 both
        /// (anything else is area).
        weight_by: u8,
        /// How far the normals turn towards the weighted ones, 0 to 1.
        strength: f32,
        /// How much the biggest polygon at a point wins, 0 to 1.
        influence: f32,
        /// Whether the shading still breaks at hard edges.
        keep_hard_edges: bool,
    },
    /// Live bevel (version 50).
    LiveBevel(super::v50::LiveBevelV50),
    /// Quadify (version 51).
    Quadify(super::v51::QuadifyV51),
}

/// Weighted normals' weighting as stored.
fn weight_number(weight_by: WeightBy) -> u8 {
    match weight_by {
        WeightBy::Area => 0,
        WeightBy::Angle => 1,
        WeightBy::AreaAndAngle => 2,
    }
}

/// Bend along curve's axis as stored.
fn axis_number(axis: BendAxis) -> u8 {
    match axis {
        BendAxis::Longest => 0,
        BendAxis::X => 1,
        BendAxis::Y => 2,
        BendAxis::Z => 3,
    }
}

/// Shrinkwrap's mode as stored.
fn mode_number(mode: WrapMode) -> u8 {
    match mode {
        WrapMode::Nearest => 0,
        WrapMode::Normals => 1,
        WrapMode::AxisX => 2,
        WrapMode::AxisY => 3,
        WrapMode::AxisZ => 4,
    }
}

impl KindV43 {
    fn of(kind: &ModifierKind) -> Self {
        match kind {
            ModifierKind::Remesh(o) => Self::Remesh {
                voxel_size: o.voxel_size,
                smooth: o.smooth,
            },
            ModifierKind::Subdivide(o) => Self::Subdivide {
                levels: o.levels,
                export_levels: o.export_levels,
                hard_edges_sharp: o.hard_edges_sharp,
            },
            ModifierKind::Lattice(o) => Self::Lattice { cage: o.cage.get() },
            ModifierKind::Decimate(o) => Self::Decimate {
                by_triangles: o.reduce_by == ReduceBy::Triangles,
                ratio: o.ratio,
                triangles: o.triangles,
                keep_seams: o.keep.seams,
                keep_hard_edges: o.keep.hard_edges,
                keep_materials: o.keep.materials,
                keep_borders: o.keep.borders,
                quads_where_flat: o.quads_where_flat,
            },
            ModifierKind::Scatter(s) => Self::Scatter {
                place_on: match s.options.place_on {
                    PlaceOn::Points => 0,
                    PlaceOn::Centres => 1,
                    PlaceOn::Random => 2,
                },
                count: s.options.count,
                seed: s.options.seed,
                spacing: s.options.spacing,
                align: s.options.align,
                random_turn: s.options.random_turn,
                scale_min: s.options.scale_min,
                scale_max: s.options.scale_max,
                offset: s.options.offset,
                shapes: s.shapes.iter().map(|id| id.get()).collect(),
                polygons: s.polygons.iter().map(|f| f.get()).collect(),
            },
            ModifierKind::BendAlongCurve(b) => Self::BendAlongCurve {
                curve: b.curve.get(),
                axis: axis_number(b.options.axis),
                stretch: b.options.stretch,
                offset: b.options.offset,
                twist: b.options.twist,
            },
            ModifierKind::Shrinkwrap(w) => Self::Shrinkwrap {
                target: w.target.get(),
                mode: mode_number(w.options.mode),
                offset: w.options.offset,
                points: w.points.iter().map(|v| v.get()).collect(),
            },
            ModifierKind::WeightedNormals(o) => Self::WeightedNormals {
                weight_by: weight_number(o.weight_by),
                strength: o.strength,
                influence: o.influence,
                keep_hard_edges: o.keep_hard_edges,
            },
            ModifierKind::LiveBevel(o) => Self::LiveBevel(super::v50::LiveBevelV50::of(o)),
            ModifierKind::Quadify(o) => Self::Quadify(super::v51::QuadifyV51::of(o)),
        }
    }

    /// The kind, its settings brought within limits by the scene.
    fn kind(&self) -> ModifierKind {
        match self {
            Self::Remesh { voxel_size, smooth } => ModifierKind::Remesh(RemeshOptions {
                voxel_size: *voxel_size,
                smooth: *smooth,
            }),
            Self::Subdivide {
                levels,
                export_levels,
                hard_edges_sharp,
            } => ModifierKind::Subdivide(SubdivideOptions {
                levels: *levels,
                export_levels: *export_levels,
                hard_edges_sharp: *hard_edges_sharp,
            }),
            Self::Lattice { cage } => ModifierKind::Lattice(LatticeOptions {
                cage: ObjectId::from_raw(*cage),
            }),
            Self::Decimate {
                by_triangles,
                ratio,
                triangles,
                keep_seams,
                keep_hard_edges,
                keep_materials,
                keep_borders,
                quads_where_flat,
            } => ModifierKind::Decimate(DecimateOptions {
                reduce_by: if *by_triangles {
                    ReduceBy::Triangles
                } else {
                    ReduceBy::Ratio
                },
                ratio: *ratio,
                triangles: *triangles,
                keep: DecimateKeep {
                    seams: *keep_seams,
                    hard_edges: *keep_hard_edges,
                    materials: *keep_materials,
                    borders: *keep_borders,
                },
                quads_where_flat: *quads_where_flat,
            }),
            Self::Scatter {
                place_on,
                count,
                seed,
                spacing,
                align,
                random_turn,
                scale_min,
                scale_max,
                offset,
                shapes,
                polygons,
            } => ModifierKind::Scatter(ScatterSettings {
                options: ScatterOptions {
                    place_on: match place_on {
                        0 => PlaceOn::Points,
                        1 => PlaceOn::Centres,
                        _ => PlaceOn::Random,
                    },
                    count: *count,
                    seed: *seed,
                    spacing: *spacing,
                    align: *align,
                    random_turn: *random_turn,
                    scale_min: *scale_min,
                    scale_max: *scale_max,
                    offset: *offset,
                },
                shapes: shapes.iter().map(|id| ObjectId::from_raw(*id)).collect(),
                polygons: polygons.iter().map(|f| FaceId::from_raw(*f)).collect(),
            }),
            Self::BendAlongCurve {
                curve,
                axis,
                stretch,
                offset,
                twist,
            } => ModifierKind::BendAlongCurve(CurveBendSettings {
                options: BendOptions {
                    axis: BendAxis::ALL
                        .into_iter()
                        .find(|a| axis_number(*a) == *axis)
                        .unwrap_or_default(),
                    stretch: *stretch,
                    offset: *offset,
                    twist: *twist,
                },
                curve: ObjectId::from_raw(*curve),
            }),
            Self::Shrinkwrap {
                target,
                mode,
                offset,
                points,
            } => ModifierKind::Shrinkwrap(ShrinkwrapSettings {
                options: ShrinkwrapOptions {
                    mode: WrapMode::ALL
                        .into_iter()
                        .find(|m| mode_number(*m) == *mode)
                        .unwrap_or_default(),
                    offset: *offset,
                },
                target: ObjectId::from_raw(*target),
                points: points.iter().map(|v| VertexId::from_raw(*v)).collect(),
            }),
            Self::WeightedNormals {
                weight_by,
                strength,
                influence,
                keep_hard_edges,
            } => ModifierKind::WeightedNormals(WeightedNormalsOptions {
                weight_by: match weight_by {
                    1 => WeightBy::Angle,
                    2 => WeightBy::AreaAndAngle,
                    _ => WeightBy::Area,
                },
                strength: *strength,
                influence: *influence,
                keep_hard_edges: *keep_hard_edges,
            }),
            Self::LiveBevel(o) => ModifierKind::LiveBevel(o.options()),
            Self::Quadify(o) => ModifierKind::Quadify(o.options()),
        }
    }
}

impl ProjectV43 {
    /// The file model of a document.
    pub(crate) fn from_document(document: &Document) -> Self {
        let scene = document.scene();
        let modifiers = scene
            .walk()
            .iter()
            .filter(|(id, _)| !scene.modifiers(*id).is_empty())
            .map(|(id, _)| ModifiersV43 {
                id: id.get(),
                list: scene
                    .modifiers(*id)
                    .iter()
                    .map(|m| ModifierV43 {
                        kind: KindV43::of(&m.kind),
                        on: m.on,
                        in_viewport: m.in_viewport,
                    })
                    .collect(),
            })
            .collect();
        Self {
            project: ProjectV41::from_document(document),
            modifiers,
            lattices: super::v45::lattices_of(document),
        }
    }

    /// The document this file model describes, after checking every value.
    pub(crate) fn into_document(self) -> Result<Document, Problem> {
        let document = self.project.into_document()?;
        // The cages first: modifiers name them.
        let mut document = super::v45::with_lattices(document, self.lattices)?;
        let mut seen = std::collections::BTreeSet::new();
        for entry in self.modifiers {
            if !seen.insert(entry.id) {
                return Err(tr("an object's modifiers are listed twice").into());
            }
            let id = ObjectId::from_raw(entry.id);
            if !document.scene().contains(id) {
                return Err(tr("its modifiers name an object that isn't there").into());
            }
            let list: Vec<Modifier> = entry
                .list
                .0
                .into_iter()
                .map(|m| Modifier {
                    kind: m.kind.kind(),
                    on: m.on,
                    in_viewport: m.in_viewport,
                })
                .collect();
            document = document
                .with_modifiers(id, list)
                .map_err(|why| Problem::from(trf!("a modifier can't be used: {}", why)))?;
        }
        Ok(document)
    }
}

/// The CBOR bytes of `model`.
pub(crate) fn encode(model: &ProjectV43) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    ciborium::into_writer(model, &mut bytes).map_err(|e| e.to_string())?;
    Ok(bytes)
}

/// The model in CBOR `bytes`; anything after it is refused.
pub(crate) fn decode(bytes: &[u8]) -> Result<ProjectV43, String> {
    let mut reader = std::io::Cursor::new(bytes);
    let model = ciborium::from_reader(&mut reader).map_err(|e| e.to_string())?;
    if reader.position() as usize != bytes.len() {
        return Err(tr("there is unexpected data after the scene").into());
    }
    Ok(model)
}
