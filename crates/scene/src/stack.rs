//! An object's operation stack: the edits recorded on top of its base
//! (a primitive's settings, or a plain mesh), and how the final shape is
//! worked out from them (DESIGN.md §4.2, decisions
//! [0006](../../../docs/decisions/0006-base-plus-operation-stack.md) and
//! [0021](../../../docs/decisions/0021-object-stack-foundation.md)).
//!
//! The shape is evaluated from the bottom up: the base's mesh, then each
//! layer's operations in order. Each layer's result is cached, so a change
//! in one layer only re-evaluates from that layer upward, and a change to
//! the base (a cube's segments, say) replays every recorded edit on the
//! new shape. Until the Edit Layers interface arrives, every edit goes
//! into one layer, so the user sees a plain object.
//!
//! Each recorded edit is an [`Entry`]: its operation, an id ([`EditId`],
//! unique among the object's edits) and two switches, on/off and pinned
//! ([decision 0044](../../../docs/decisions/0044-edits-that-hold.md)). A switched-off edit is skipped. The
//! switches are there for the edit list to come (Phase 6).

use crate::object::{ObjectId, ObjectKind};
use glam::{Mat4, Vec3};
use mesh::edit::{self, Bend, BevelShape, Grouping, Outcome, SlitLine};
use mesh::{Anchors, EdgeKey, FaceId, Mesh, VertexId};
use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, Mutex};

/// The name of the layer edits go into until layers can be managed.
pub const EDITS_LAYER: &str = "Edits";

/// The most polygons a shape may grow to through its edits. An edit that
/// would make the shape larger is skipped, with a warning, so neither a
/// damaged or hostile file nor a slip of the hand (a 64-row bevel on every
/// edge of a heavy shape) can freeze the app or run it out of memory
/// ([decision 0045](../../../docs/decisions/0045-limits-for-files.md)).
pub const MAX_SHAPE_POLYGONS: usize = 30_000_000;

/// One recorded edit. Elements are named by id, so replaying the same
/// history gives the same result; ids that are no longer in the shape are
/// skipped and reported (see [`StackWarning`]). More kinds arrive with the
/// modelling tools.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Operation {
    /// Moves points by an offset, in the object's own space (centimetres).
    MovePoints {
        /// The points.
        points: Vec<VertexId>,
        /// How far.
        offset: Vec3,
        /// Where the points were, to find them again if the shape's points
        /// are renumbered (decision 0032). `None`: by ID only.
        anchors: Option<Anchors>,
    },
    /// Turns, scales or moves points with a matrix, in the object's own
    /// space (decision 0030).
    TransformPoints {
        /// The points.
        points: Vec<VertexId>,
        /// The change, an affine transform.
        matrix: Mat4,
        /// As for [`Self::MovePoints`].
        anchors: Option<Anchors>,
    },
    /// Pushes polygons out along their facing, building walls between
    /// the old place and the new (decision 0038). The polygons keep their
    /// IDs.
    ExtrudePolygons {
        /// The polygons.
        faces: Vec<FaceId>,
        /// How far, in the object's own space (centimetres); negative
        /// pushes them in.
        distance: f32,
        /// Neighbours as one piece, or each on its own.
        grouping: Grouping,
        /// Where the polygons were, to find them again (decision 0032).
        anchors: Option<Anchors>,
    },
    /// Pushes polygons out with a bend first: each piece turns a corner,
    /// keeping its full size, then goes on straight (decision 0050).
    ExtrudeBent {
        /// The polygons.
        faces: Vec<FaceId>,
        /// How far it goes on straight after the bend, in the object's own
        /// space (centimetres).
        distance: f32,
        /// Neighbours as one piece, or each on its own.
        grouping: Grouping,
        /// The bend: angle, direction, radius and rows.
        bend: Bend,
        /// Where the polygons were, to find them again (decision 0032).
        anchors: Option<Anchors>,
    },
    /// Shrinks polygons inward, flat where they are, leaving a ring of new
    /// polygons around them (decision 0038).
    InsetPolygons {
        /// The polygons.
        faces: Vec<FaceId>,
        /// How far in, in the object's own space (centimetres).
        distance: f32,
        /// Neighbours as one piece, or each on its own.
        grouping: Grouping,
        /// Where the polygons were, to find them again (decision 0032).
        anchors: Option<Anchors>,
    },
    /// Grows a strip of new polygons outward from edges on an open
    /// border (decision 0038).
    ExtrudeEdges {
        /// The edges.
        edges: Vec<EdgeKey>,
        /// How far, in the object's own space (centimetres).
        distance: f32,
        /// Edges joined at a point as one piece, or each on its own.
        grouping: Grouping,
        /// Where the edges were, to find them again (decision 0032).
        anchors: Option<Anchors>,
    },
    /// Cuts edges away, putting a strip of polygons in their place
    /// (decision 0040).
    BevelEdges {
        /// The edges.
        edges: Vec<EdgeKey>,
        /// Its width (in the object's own space, centimetres), rows and
        /// profile.
        shape: BevelShape,
        /// Where the edges were, to find them again (decision 0032).
        anchors: Option<Anchors>,
    },
    /// Cuts corners away, putting a polygon in their place (decision
    /// 0040).
    BevelPoints {
        /// The points.
        points: Vec<VertexId>,
        /// How far along each edge the cut goes, in the object's own
        /// space (centimetres).
        width: f32,
        /// Where the points were, to find them again (decision 0032).
        anchors: Option<Anchors>,
    },
    /// Cuts a new loop of edges through the ring of four-sided polygons
    /// across an edge (decision 0047).
    LoopCut {
        /// The edge the cut runs across.
        edge: EdgeKey,
        /// How many evenly spaced cuts (1 to [`mesh::loopcut::MAX_CUTS`]).
        cuts: u32,
        /// Where the cuts sit across the edge, 0 to 1 from its lower point
        /// id (0.5: centred).
        position: f32,
        /// Where the edge was, its lower point first, to find it again
        /// (decision 0032).
        anchors: Option<Anchors>,
    },
    /// Joins points (or new points along edges) with new edges across
    /// their polygons (decision 0051).
    Connect {
        /// The points joined (Points mode); empty when edges are.
        points: Vec<VertexId>,
        /// The edges cut and joined (Edges mode).
        edges: Vec<EdgeKey>,
        /// How many cuts along each edge.
        cuts: u32,
        /// Where they were, to find them again (decision 0032).
        anchors: Option<Anchors>,
    },
    /// A band of polygons between two open borders, or between two
    /// polygons, which go (decision 0051).
    Bridge {
        /// The border edges (both borders), or none.
        edges: Vec<EdgeKey>,
        /// The two polygons, or none.
        faces: Vec<FaceId>,
        /// The rows the band has.
        segments: u32,
        /// How many steps round the band is turned (closed loops).
        twist: i32,
        /// Where they were, to find them again (decision 0032).
        anchors: Option<Anchors>,
    },
    /// Points melted into one, at their middle or onto a target point
    /// (decision 0051).
    Weld {
        /// The points.
        points: Vec<VertexId>,
        /// The point they melt onto, if any.
        target: Option<VertexId>,
        /// Where the points were (the target last), to find them again.
        anchors: Option<Anchors>,
    },
    /// Deletes points, edges or polygons, with every polygon using them,
    /// leaving a hole (decision 0052).
    DeleteElements {
        /// The points.
        points: Vec<VertexId>,
        /// The edges.
        edges: Vec<EdgeKey>,
        /// The polygons.
        faces: Vec<FaceId>,
        /// Where they were, to find them again (decision 0032).
        anchors: Option<Anchors>,
    },
    /// Takes points, edges or polygons away but keeps the surface closed:
    /// the polygons around them merge (decision 0052).
    Dissolve {
        /// The points.
        points: Vec<VertexId>,
        /// The edges.
        edges: Vec<EdgeKey>,
        /// The polygons.
        faces: Vec<FaceId>,
        /// Where they were, to find them again (decision 0032).
        anchors: Option<Anchors>,
    },
    /// Closes holes with polygons: those touching these points, or every
    /// hole when there are none (decision 0052).
    FillHoles {
        /// Points on the holes' borders.
        points: Vec<VertexId>,
        /// Where they were, to find them again (decision 0032).
        anchors: Option<Anchors>,
    },
    /// Turns polygons inside out (decision 0052).
    FlipPolygons {
        /// The polygons.
        faces: Vec<FaceId>,
        /// Where they were, to find them again (decision 0032).
        anchors: Option<Anchors>,
    },
    /// Turns every polygon facing the wrong way round (decision 0052).
    FixFacing,
    /// Marks edges hard (a crisp break in the shading) or soft (smooth
    /// shading across) (decision 0053).
    SetHardEdges {
        /// The edges.
        edges: Vec<EdgeKey>,
        /// Hard, or soft.
        hard: bool,
        /// Where they were, to find them again (decision 0032).
        anchors: Option<Anchors>,
    },
    /// Marks every edge whose polygons meet more sharply than `angle`
    /// degrees hard and the rest soft (decision 0053). Worked out from the
    /// shape as it is.
    HardEdgesByAngle {
        /// The angle, 0 to 180 degrees.
        angle: f32,
    },
    /// Cuts new edges along a line drawn over the shape, as seen from
    /// where it was drawn (decision 0049). Worked out from the shape as it
    /// is, so it needs no ids or anchors.
    Slit {
        /// The line, where it is seen from, and whether it cuts through.
        line: SlitLine,
    },
}

/// An edit's id: unique among the edits of its object while they exist,
/// so an edit can be named however the list changes around it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EditId(u64);

impl EditId {
    /// The id as a number.
    pub fn get(self) -> u64 {
        self.0
    }

    /// The id with this number, as saved earlier.
    pub const fn from_raw(number: u64) -> Self {
        Self(number)
    }
}

/// One recorded edit in a layer: its operation, its id, and its switches.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// The edit's id.
    pub id: EditId,
    /// A switched-off edit is skipped: the shape is as if it weren't
    /// there, but it can be switched on again.
    pub enabled: bool,
    /// Pinned edits stay changeable; unpinned old ones may later be frozen
    /// into the shape to keep big models quick (Phase 6).
    pub pinned: bool,
    /// What the edit does.
    pub operation: Operation,
}

impl Entry {
    /// A switched-on, unpinned edit.
    pub fn new(id: EditId, operation: Operation) -> Self {
        Self {
            id,
            enabled: true,
            pinned: false,
            operation,
        }
    }
}

/// The settings of a recorded edit that can still be changed afterwards
/// (the Attributes panel's Last edit): an extrude, inset, bevel or loop
/// cut. Those an edit doesn't have are `None`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EditSettings {
    /// The distance, or a bevel's width (centimetres, own space).
    pub distance: Option<f32>,
    /// Neighbours together or each on its own (extrude and inset).
    pub grouping: Option<Grouping>,
    /// A bevel's rows.
    pub segments: Option<u32>,
    /// A bevel's profile, flat (0) to round (1).
    pub profile: Option<f32>,
    /// A loop cut's number of cuts.
    pub cuts: Option<u32>,
    /// Where a loop cut sits across its edge, 0 to 1.
    pub position: Option<f32>,
    /// Whether a slit cuts through to the far side.
    pub through: Option<bool>,
    /// An extrude's bend (straight for an extrude of polygons without
    /// one; none for other edits).
    pub bend: Option<Bend>,
    /// A bridge's rows and twist.
    pub bridge: Option<(u32, i32)>,
    /// Hard edges by angle's angle, in degrees.
    pub angle: Option<f32>,
}

/// What applying an operation did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Applied {
    /// How many of its elements weren't found.
    pub(crate) missing: usize,
    /// Whether its points had to be found again by position, because the
    /// shape's points were renumbered.
    pub(crate) found_again: bool,
    /// Whether it was skipped because the shape would grow past
    /// [`MAX_SHAPE_POLYGONS`].
    pub(crate) too_big: bool,
}

impl Operation {
    /// A short description, such as "Move 3 points".
    pub fn describe(&self) -> String {
        match self {
            Self::MovePoints { points, .. } => match points.len() {
                1 => "Move 1 point".to_string(),
                n => format!("Move {n} points"),
            },
            Self::TransformPoints { points, .. } => match points.len() {
                1 => "Transform 1 point".to_string(),
                n => format!("Transform {n} points"),
            },
            Self::ExtrudePolygons { faces, .. } => counted("Extrude", faces.len(), "polygon"),
            Self::ExtrudeBent { faces, bend, .. } if bend.bends() => {
                format!("{} with a bend", counted("Extrude", faces.len(), "polygon"))
            }
            Self::ExtrudeBent { faces, .. } => counted("Extrude", faces.len(), "polygon"),
            Self::InsetPolygons { faces, .. } => counted("Inset", faces.len(), "polygon"),
            Self::ExtrudeEdges { edges, .. } => counted("Extrude", edges.len(), "edge"),
            Self::BevelEdges { edges, .. } => counted("Bevel", edges.len(), "edge"),
            Self::BevelPoints { points, .. } => counted("Bevel", points.len(), "point"),
            Self::LoopCut { cuts: 1, .. } => "Loop cut".to_string(),
            Self::LoopCut { cuts, .. } => format!("Loop cut ({cuts} cuts)"),
            Self::Connect { points, edges, .. } if edges.is_empty() => {
                counted("Connect", points.len(), "point")
            }
            Self::Connect { edges, .. } => counted("Connect", edges.len(), "edge"),
            Self::Bridge { faces, .. } if !faces.is_empty() => "Bridge 2 polygons".to_string(),
            Self::DeleteElements {
                points,
                edges,
                faces,
                ..
            } => elements("Delete", points, edges, faces),
            Self::Dissolve {
                points,
                edges,
                faces,
                ..
            } => elements("Dissolve", points, edges, faces),
            Self::FillHoles { points, .. } if points.is_empty() => "Fill every hole".to_string(),
            Self::FillHoles { .. } => "Fill hole".to_string(),
            Self::FlipPolygons { faces, .. } => counted("Flip", faces.len(), "polygon"),
            Self::FixFacing => "Fix facing".to_string(),
            Self::SetHardEdges { edges, hard, .. } => {
                let verb = if *hard { "Harden" } else { "Soften" };
                counted(verb, edges.len(), "edge")
            }
            Self::HardEdgesByAngle { .. } => "Hard edges by angle".to_string(),
            Self::Bridge { edges, .. } => counted("Bridge", edges.len(), "edge"),
            Self::Weld { points, target, .. } => {
                let n = points.len() + usize::from(target.is_some_and(|t| !points.contains(&t)));
                counted("Weld", n, "point")
            }
            Self::Slit { line } if line.through => "Slit through".to_string(),
            Self::Slit { .. } => "Slit".to_string(),
        }
    }

    /// The settings that can still be changed, for an extrude, inset or
    /// bevel.
    pub fn settings(&self) -> Option<EditSettings> {
        let plain = |distance: f32, grouping: Grouping| EditSettings {
            distance: Some(distance),
            grouping: Some(grouping),
            ..EditSettings::default()
        };
        match self {
            Self::ExtrudePolygons {
                distance, grouping, ..
            } => Some(EditSettings {
                bend: Some(Bend::default()),
                ..plain(*distance, *grouping)
            }),
            Self::ExtrudeBent {
                distance,
                grouping,
                bend,
                ..
            } => Some(EditSettings {
                bend: Some(*bend),
                ..plain(*distance, *grouping)
            }),
            Self::InsetPolygons {
                distance, grouping, ..
            }
            | Self::ExtrudeEdges {
                distance, grouping, ..
            } => Some(plain(*distance, *grouping)),
            Self::BevelEdges { shape, .. } => Some(EditSettings {
                distance: Some(shape.width),
                segments: Some(shape.segments),
                profile: Some(shape.profile),
                ..EditSettings::default()
            }),
            Self::BevelPoints { width, .. } => Some(EditSettings {
                distance: Some(*width),
                ..EditSettings::default()
            }),
            Self::LoopCut { cuts, position, .. } => Some(EditSettings {
                cuts: Some(*cuts),
                position: Some(*position),
                ..EditSettings::default()
            }),
            Self::Slit { line } => Some(EditSettings {
                through: Some(line.through),
                ..EditSettings::default()
            }),
            Self::Connect { edges, cuts, .. } if !edges.is_empty() => Some(EditSettings {
                cuts: Some(*cuts),
                ..EditSettings::default()
            }),
            Self::Bridge {
                segments, twist, ..
            } => Some(EditSettings {
                bridge: Some((*segments, *twist)),
                ..EditSettings::default()
            }),
            Self::HardEdgesByAngle { angle } => Some(EditSettings {
                angle: Some(*angle),
                ..EditSettings::default()
            }),
            Self::Connect { .. }
            | Self::Weld { .. }
            | Self::DeleteElements { .. }
            | Self::Dissolve { .. }
            | Self::FillHoles { .. }
            | Self::FlipPolygons { .. }
            | Self::SetHardEdges { .. }
            | Self::FixFacing => None,
            Self::MovePoints { .. } | Self::TransformPoints { .. } => None,
        }
    }

    /// The same edit with new settings (those it doesn't have are
    /// ignored; an edit without settings is unchanged).
    #[must_use]
    pub fn with(&self, settings: EditSettings) -> Self {
        // A straight extrude given a bend becomes a bent one.
        if let (
            Self::ExtrudePolygons {
                faces,
                distance,
                grouping,
                anchors,
            },
            Some(bend),
        ) = (self, settings.bend)
            && bend.bends()
        {
            let bent = Self::ExtrudeBent {
                faces: faces.clone(),
                distance: *distance,
                grouping: *grouping,
                bend,
                anchors: anchors.clone(),
            };
            return bent.with(settings);
        }
        let mut changed = self.clone();
        match &mut changed {
            Self::ExtrudeBent {
                distance,
                grouping,
                bend,
                ..
            } => {
                if let Some(d) = settings.distance {
                    *distance = d;
                }
                if let Some(g) = settings.grouping {
                    *grouping = g;
                }
                if let Some(b) = settings.bend {
                    *bend = b;
                }
            }
            Self::ExtrudePolygons {
                distance, grouping, ..
            }
            | Self::InsetPolygons {
                distance, grouping, ..
            }
            | Self::ExtrudeEdges {
                distance, grouping, ..
            } => {
                if let Some(d) = settings.distance {
                    *distance = d;
                }
                if let Some(g) = settings.grouping {
                    *grouping = g;
                }
            }
            Self::BevelEdges { shape, .. } => {
                if let Some(w) = settings.distance {
                    shape.width = w;
                }
                if let Some(n) = settings.segments {
                    shape.segments = n;
                }
                if let Some(p) = settings.profile {
                    shape.profile = p;
                }
            }
            Self::BevelPoints { width, .. } => {
                if let Some(w) = settings.distance {
                    *width = w;
                }
            }
            Self::LoopCut { cuts, position, .. } => {
                if let Some(n) = settings.cuts {
                    *cuts = n;
                }
                if let Some(p) = settings.position {
                    *position = p;
                }
            }
            Self::Slit { line } => {
                if let Some(t) = settings.through {
                    line.through = t;
                }
            }
            Self::Connect { cuts, .. } => {
                if let Some(n) = settings.cuts {
                    *cuts = n;
                }
            }
            Self::Bridge {
                segments, twist, ..
            } => {
                if let Some((s, t)) = settings.bridge {
                    *segments = s;
                    *twist = t;
                }
            }
            Self::HardEdgesByAngle { angle } => {
                if let Some(a) = settings.angle {
                    *angle = a.clamp(0.0, mesh::hardness::MAX_ANGLE);
                }
            }
            Self::Weld { .. }
            | Self::DeleteElements { .. }
            | Self::Dissolve { .. }
            | Self::FillHoles { .. }
            | Self::FlipPolygons { .. }
            | Self::SetHardEdges { .. }
            | Self::FixFacing => {}
            Self::MovePoints { .. } | Self::TransformPoints { .. } => {}
        }
        changed
    }

    /// Whether every number the operation holds is a real number (not
    /// infinite or NaN). Edits with other numbers are refused, as they
    /// couldn't be saved and opened again.
    pub fn numbers_are_real(&self) -> bool {
        let anchors_ok = self.anchors().is_none_or(|a| {
            a.bounds.min.is_finite()
                && a.bounds.max.is_finite()
                && a.points.iter().all(|(_, p)| p.is_finite())
                && match &a.shape {
                    mesh::AnchorShape::Points(p) => p.iter().all(|v| v.is_finite()),
                    mesh::AnchorShape::Edges(e) => {
                        e.iter().all(|[a, b]| a.is_finite() && b.is_finite())
                    }
                    mesh::AnchorShape::Polygons(p) => p.iter().flatten().all(|v| v.is_finite()),
                }
        });
        let own = match self {
            Self::MovePoints { offset, .. } => offset.is_finite(),
            Self::TransformPoints { matrix, .. } => matrix.is_finite(),
            Self::ExtrudePolygons { distance, .. }
            | Self::InsetPolygons { distance, .. }
            | Self::ExtrudeEdges { distance, .. } => distance.is_finite(),
            Self::BevelEdges { shape, .. } => shape.width.is_finite() && shape.profile.is_finite(),
            Self::BevelPoints { width, .. } => width.is_finite(),
            Self::LoopCut { position, .. } => position.is_finite(),
            Self::Connect { .. }
            | Self::Bridge { .. }
            | Self::Weld { .. }
            | Self::DeleteElements { .. }
            | Self::Dissolve { .. }
            | Self::FillHoles { .. }
            | Self::FlipPolygons { .. }
            | Self::SetHardEdges { .. }
            | Self::FixFacing => true,
            Self::HardEdgesByAngle { angle } => angle.is_finite(),
            Self::ExtrudeBent { distance, bend, .. } => {
                distance.is_finite()
                    && [bend.angle, bend.direction, bend.radius]
                        .iter()
                        .all(|x| x.is_finite())
            }
            Self::Slit { line } => {
                line.points.iter().all(|p| p.is_finite())
                    && match line.eye {
                        edit::Eye::At(e) => e.is_finite(),
                        edit::Eye::Along(d) => d.is_finite(),
                    }
            }
        };
        own && anchors_ok
    }

    /// Where the operation's elements were, if it keeps that.
    pub fn anchors(&self) -> Option<&Anchors> {
        match self {
            Self::MovePoints { anchors, .. }
            | Self::TransformPoints { anchors, .. }
            | Self::ExtrudePolygons { anchors, .. }
            | Self::InsetPolygons { anchors, .. }
            | Self::ExtrudeEdges { anchors, .. }
            | Self::BevelEdges { anchors, .. }
            | Self::BevelPoints { anchors, .. }
            | Self::LoopCut { anchors, .. }
            | Self::ExtrudeBent { anchors, .. }
            | Self::Connect { anchors, .. }
            | Self::Bridge { anchors, .. }
            | Self::Weld { anchors, .. }
            | Self::DeleteElements { anchors, .. }
            | Self::Dissolve { anchors, .. }
            | Self::FillHoles { anchors, .. }
            | Self::FlipPolygons { anchors, .. }
            | Self::SetHardEdges { anchors, .. } => anchors.as_ref(),
            Self::FixFacing | Self::HardEdgesByAngle { .. } => None,
            Self::Slit { .. } => None,
        }
    }

    /// Where the operation's elements were, to change (for opening saved
    /// files that keep more about them).
    pub fn anchors_mut(&mut self) -> Option<&mut Anchors> {
        match self {
            Self::MovePoints { anchors, .. }
            | Self::TransformPoints { anchors, .. }
            | Self::ExtrudePolygons { anchors, .. }
            | Self::InsetPolygons { anchors, .. }
            | Self::ExtrudeEdges { anchors, .. }
            | Self::BevelEdges { anchors, .. }
            | Self::BevelPoints { anchors, .. }
            | Self::LoopCut { anchors, .. }
            | Self::ExtrudeBent { anchors, .. }
            | Self::Connect { anchors, .. }
            | Self::Bridge { anchors, .. }
            | Self::Weld { anchors, .. }
            | Self::DeleteElements { anchors, .. }
            | Self::Dissolve { anchors, .. }
            | Self::FillHoles { anchors, .. }
            | Self::FlipPolygons { anchors, .. }
            | Self::SetHardEdges { anchors, .. } => anchors.as_mut(),
            Self::FixFacing | Self::HardEdgesByAngle { .. } => None,
            Self::Slit { .. } => None,
        }
    }

    /// The distance of an extrude or inset (or a bevel's width), if it
    /// is one: what the Attributes panel lets you change afterwards.
    pub fn distance(&self) -> Option<f32> {
        self.settings().and_then(|s| s.distance)
    }

    /// The same extrude or inset with a new distance and grouping
    /// (anything else unchanged).
    #[must_use]
    pub fn with_settings(&self, distance: f32, grouping: Grouping) -> Self {
        match self.settings() {
            Some(s) => self.with(EditSettings {
                distance: Some(distance),
                grouping: Some(grouping),
                ..s
            }),
            None => self.clone(),
        }
    }

    /// The grouping of an extrude or inset, if it is one.
    pub fn grouping(&self) -> Option<Grouping> {
        self.settings().and_then(|s| s.grouping)
    }

    /// The points the operation names, and its anchors. Empty for
    /// operations that name polygons or edges.
    pub fn points(&self) -> (&[VertexId], Option<&Anchors>) {
        match self {
            Self::MovePoints { points, .. }
            | Self::TransformPoints { points, .. }
            | Self::BevelPoints { points, .. }
            | Self::Weld { points, .. } => (points, self.anchors()),
            _ => (&[], self.anchors()),
        }
    }

    /// Applies the operation to `mesh`. Its ids are used as they are when
    /// the shape has the structure the edit was made on (decision 0044), or
    /// its anchored places still fit. Otherwise (a primitive given more
    /// segments, say) its elements are found again by their anchors.
    fn apply(&self, mesh: &mut Mesh) -> Applied {
        let same = self.anchors().is_some_and(|a| a.same_structure(mesh));
        // The points at the anchored places, when the ids can't be trusted.
        let refound = match self.anchors() {
            Some(anchors) if same || anchors.still_fit(mesh) => None,
            Some(anchors) => Some(anchors.find(mesh, mesh.topology())),
            None => None,
        };
        // Skipped (changing nothing) if it could grow the shape too large.
        let polygons = mesh.face_count();
        let too_big = |extra: usize| polygons.saturating_add(extra) > MAX_SHAPE_POLYGONS;
        let skipped = Applied {
            missing: 0,
            found_again: refound.is_some(),
            too_big: true,
        };
        let outcome: Outcome = match self {
            Self::MovePoints { points, offset, .. } => {
                let points = refound.as_ref().map_or(points.as_slice(), |r| &r.0);
                edit::move_points(mesh, points, *offset)
            }
            Self::TransformPoints { points, matrix, .. } => {
                let points = refound.as_ref().map_or(points.as_slice(), |r| &r.0);
                edit::transform_points(mesh, points, *matrix)
            }
            Self::ExtrudePolygons {
                faces,
                distance,
                grouping,
                ..
            } => {
                let faces = refound
                    .as_ref()
                    .map_or_else(|| faces.clone(), |r| faces_within(mesh, &r.0));
                if too_big(corner_count(mesh, &faces)) {
                    return skipped;
                }
                edit::extrude_polygons(mesh, &faces, *distance, *grouping)
            }
            Self::ExtrudeBent {
                faces,
                distance,
                grouping,
                bend,
                ..
            } => {
                let faces = refound
                    .as_ref()
                    .map_or_else(|| faces.clone(), |r| faces_within(mesh, &r.0));
                // A wall of every ring round each polygon.
                let rings = bend.rings(*distance);
                if too_big(corner_count(mesh, &faces).saturating_mul(rings)) {
                    return skipped;
                }
                edit::extrude_bent(mesh, &faces, *distance, *grouping, *bend)
            }
            Self::InsetPolygons {
                faces,
                distance,
                grouping,
                ..
            } => {
                let faces = refound
                    .as_ref()
                    .map_or_else(|| faces.clone(), |r| faces_within(mesh, &r.0));
                if too_big(corner_count(mesh, &faces)) {
                    return skipped;
                }
                edit::inset_polygons(mesh, &faces, *distance, *grouping)
            }
            Self::ExtrudeEdges {
                edges,
                distance,
                grouping,
                ..
            } => {
                let edges = refound
                    .as_ref()
                    .map_or_else(|| edges.clone(), |r| edges_within(mesh, &r.0));
                if too_big(edges.len()) {
                    return skipped;
                }
                edit::extrude_edges(mesh, &edges, *distance, *grouping)
            }
            Self::BevelEdges { edges, shape, .. } => {
                let edges = refound
                    .as_ref()
                    .map_or_else(|| edges.clone(), |r| edges_within(mesh, &r.0));
                // A strip of rows per edge, and a corner polygon at each end.
                let rows = shape.segments.clamp(1, mesh::bevel::MAX_SEGMENTS) as usize;
                if too_big(edges.len().saturating_mul(rows + 2)) {
                    return skipped;
                }
                edit::bevel_edges(mesh, &edges, *shape)
            }
            Self::BevelPoints { points, width, .. } => {
                let points = refound.as_ref().map_or(points.as_slice(), |r| &r.0);
                // One corner polygon per point.
                if too_big(points.len()) {
                    return skipped;
                }
                edit::bevel_points(mesh, points, *width)
            }
            Self::LoopCut {
                edge,
                cuts,
                position,
                anchors,
            } => {
                let (edge, position) = match &refound {
                    None => (*edge, *position),
                    Some((points, _)) => {
                        let Some(found) = edges_within(mesh, points).first().copied() else {
                            return Applied {
                                missing: 1,
                                found_again: true,
                                too_big: false,
                            };
                        };
                        (found, facing(mesh, found, anchors.as_ref(), *position))
                    }
                };
                // Each polygon of the ring gains one polygon per cut.
                let cuts_made = (*cuts).clamp(1, mesh::loopcut::MAX_CUTS) as usize;
                if too_big(mesh::loopcut::ring_polygons(mesh, edge).saturating_mul(cuts_made)) {
                    return skipped;
                }
                edit::loop_cut(mesh, edge, *cuts, position)
            }
            Self::Connect {
                points,
                edges,
                cuts,
                ..
            } => {
                if edges.is_empty() {
                    let points = refound.as_ref().map_or(points.as_slice(), |r| &r.0);
                    if too_big(points.len()) {
                        return skipped;
                    }
                    edit::connect_points(mesh, points)
                } else {
                    let edges = refound
                        .as_ref()
                        .map_or_else(|| edges.clone(), |r| edges_within(mesh, &r.0));
                    let rows = (*cuts).clamp(1, mesh::join::MAX_CONNECT_CUTS) as usize;
                    if too_big(edges.len().saturating_mul(rows)) {
                        return skipped;
                    }
                    edit::connect_edges(mesh, &edges, *cuts)
                }
            }
            Self::Bridge {
                edges,
                faces,
                segments,
                twist,
                ..
            } => {
                let rows = (*segments).clamp(1, mesh::join::MAX_BRIDGE_SEGMENTS) as usize;
                if faces.is_empty() {
                    let edges = refound
                        .as_ref()
                        .map_or_else(|| edges.clone(), |r| edges_within(mesh, &r.0));
                    if too_big(edges.len().saturating_mul(rows)) {
                        return skipped;
                    }
                    edit::bridge_edges(mesh, &edges, *segments, *twist)
                } else {
                    let faces = refound
                        .as_ref()
                        .map_or_else(|| faces.clone(), |r| faces_within(mesh, &r.0));
                    if too_big(corner_count(mesh, &faces).saturating_mul(rows)) {
                        return skipped;
                    }
                    edit::bridge_faces(mesh, &faces, *segments, *twist)
                }
            }
            Self::Weld {
                points,
                target,
                anchors,
            } => match &refound {
                None => edit::weld(mesh, points, *target),
                Some((found, _)) => {
                    // The target is the point found nearest where it was.
                    let target = target.and_then(|_| {
                        let mesh::AnchorShape::Points(places) = &anchors.as_ref()?.shape else {
                            return None;
                        };
                        let was = *places.last()?;
                        found.iter().copied().min_by(|a, b| {
                            let d = |v: &VertexId| {
                                mesh.position(*v).map_or(f32::MAX, |p| p.distance(was))
                            };
                            d(a).total_cmp(&d(b))
                        })
                    });
                    edit::weld(mesh, found, target)
                }
            },
            Self::DeleteElements {
                points,
                edges,
                faces,
                ..
            } => {
                let (points, edges, faces) =
                    elements_in(mesh, refound.as_ref(), points, edges, faces);
                edit::delete_elements(mesh, &points, &edges, &faces)
            }
            Self::Dissolve {
                points,
                edges,
                faces,
                ..
            } => {
                let (points, edges, faces) =
                    elements_in(mesh, refound.as_ref(), points, edges, faces);
                if !faces.is_empty() {
                    edit::dissolve_faces(mesh, &faces)
                } else if !edges.is_empty() {
                    edit::dissolve_edges(mesh, &edges)
                } else {
                    edit::dissolve_points(mesh, &points)
                }
            }
            Self::FillHoles { points, .. } => {
                let points = refound.as_ref().map_or(points.as_slice(), |r| &r.0);
                // One polygon per hole, at most one per border edge.
                if too_big(mesh.topology().border_edges().len()) {
                    return skipped;
                }
                edit::fill_holes(mesh, points)
            }
            Self::FlipPolygons { faces, .. } => {
                let faces = refound
                    .as_ref()
                    .map_or_else(|| faces.clone(), |r| faces_within(mesh, &r.0));
                edit::flip_faces(mesh, &faces)
            }
            Self::FixFacing => edit::fix_facing(mesh),
            Self::SetHardEdges { edges, hard, .. } => {
                let edges = refound
                    .as_ref()
                    .map_or_else(|| edges.clone(), |r| edges_within(mesh, &r.0));
                edit::set_hard_edges(mesh, &edges, *hard)
            }
            Self::HardEdgesByAngle { angle } => edit::hard_edges_by_angle(mesh, *angle),
            Self::Slit { line } => {
                // One more polygon for each piece cut off.
                if too_big(mesh::slit::slit_size(mesh, line)) {
                    return skipped;
                }
                edit::slit(mesh, line)
            }
        };
        match refound {
            Some((_, not_found)) => Applied {
                missing: not_found,
                found_again: true,
                too_big: false,
            },
            None => Applied {
                missing: outcome.missing,
                found_again: false,
                too_big: false,
            },
        }
    }
}

/// How many corners the polygons `faces` of `mesh` have in all: an upper
/// bound on the walls an extrude or the ring an inset adds.
fn corner_count(mesh: &Mesh, faces: &[FaceId]) -> usize {
    faces
        .iter()
        .map(|f| mesh.face(*f).map_or(0, <[VertexId]>::len))
        .sum()
}

/// A loop cut's `position` across `edge`, found again by its anchors:
/// measured from the end nearest where the edge's lower point was, so the
/// cut stays on the same side when the points are renumbered.
fn facing(mesh: &Mesh, edge: EdgeKey, anchors: Option<&Anchors>, position: f32) -> f32 {
    let Some(mesh::AnchorShape::Edges(ends)) = anchors.map(|a| &a.shape) else {
        return position;
    };
    let Some([was_low, _]) = ends.first() else {
        return position;
    };
    let (low, high) = edge.ends();
    match (mesh.position(low), mesh.position(high)) {
        (Some(l), Some(h)) if h.distance(*was_low) < l.distance(*was_low) => 1.0 - position,
        _ => position,
    }
}

/// "Delete 3 polygons", "Dissolve 1 edge": by the kind of element named.
fn elements(verb: &str, points: &[VertexId], edges: &[EdgeKey], faces: &[FaceId]) -> String {
    if !faces.is_empty() {
        counted(verb, faces.len(), "polygon")
    } else if !edges.is_empty() {
        counted(verb, edges.len(), "edge")
    } else {
        counted(verb, points.len(), "point")
    }
}

/// An edit's points, edges and polygons in `mesh`: as they are, or found
/// again at the anchored places.
fn elements_in(
    mesh: &Mesh,
    refound: Option<&(Vec<VertexId>, usize)>,
    points: &[VertexId],
    edges: &[EdgeKey],
    faces: &[FaceId],
) -> (Vec<VertexId>, Vec<EdgeKey>, Vec<FaceId>) {
    match refound {
        None => (points.to_vec(), edges.to_vec(), faces.to_vec()),
        Some((found, _)) => {
            if !faces.is_empty() {
                (Vec::new(), Vec::new(), faces_within(mesh, found))
            } else if !edges.is_empty() {
                (Vec::new(), edges_within(mesh, found), Vec::new())
            } else {
                (found.clone(), Vec::new(), Vec::new())
            }
        }
    }
}

/// "Extrude 1 polygon", "Inset 4 polygons".
fn counted(verb: &str, n: usize, thing: &str) -> String {
    if n == 1 {
        format!("{verb} 1 {thing}")
    } else {
        format!("{verb} {n} {thing}s")
    }
}

/// The polygons of `mesh` whose corners are all among `points`: the
/// polygons found again at an edit's anchored places.
fn faces_within(mesh: &Mesh, points: &[VertexId]) -> Vec<FaceId> {
    let points: BTreeSet<VertexId> = points.iter().copied().collect();
    mesh.faces()
        .filter(|(_, corners)| corners.iter().all(|v| points.contains(v)))
        .map(|(f, _)| f)
        .collect()
}

/// The edges of `mesh` with both ends among `points`.
fn edges_within(mesh: &Mesh, points: &[VertexId]) -> Vec<EdgeKey> {
    let points: BTreeSet<VertexId> = points.iter().copied().collect();
    mesh.edges()
        .iter()
        .filter(|e| {
            let (a, b) = e.ends();
            points.contains(&a) && points.contains(&b)
        })
        .copied()
        .collect()
}

/// A named group of edits that can be switched off as a whole.
#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    /// Its name, for the future Edit Layers panel.
    pub name: String,
    /// A switched-off layer passes the shape below it through unchanged.
    pub enabled: bool,
    /// Its edits, in the order they apply.
    pub entries: Vec<Entry>,
}

impl Layer {
    /// An empty, switched-on layer.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            enabled: true,
            entries: Vec::new(),
        }
    }
}

/// The layers on top of an object's base, bottom first. Empty for a new
/// object. Changed only through commands.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Stack {
    pub(crate) layers: Vec<Layer>,
}

impl Stack {
    /// A stack of these layers, bottom first (for opening saved files;
    /// in use, the stack only changes through commands).
    pub fn from_layers(layers: Vec<Layer>) -> Self {
        Self { layers }
    }

    /// The layers, bottom first.
    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }

    /// Whether nothing is recorded.
    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }

    /// How many edits are recorded, in every layer.
    pub fn operation_count(&self) -> usize {
        self.layers.iter().map(|l| l.entries.len()).sum()
    }

    /// The id the next recorded edit gets: one more than the highest id
    /// in the stack (1 for the first).
    pub fn next_edit_id(&self) -> EditId {
        let highest = self
            .layers
            .iter()
            .flat_map(|l| &l.entries)
            .map(|e| e.id.0)
            .max()
            .unwrap_or(0);
        EditId(highest.saturating_add(1))
    }

    /// The edit with id `id`: its layer, its place in the layer, and the
    /// edit.
    pub fn entry(&self, id: EditId) -> Option<(usize, usize, &Entry)> {
        self.layers.iter().enumerate().find_map(|(l, layer)| {
            layer
                .entries
                .iter()
                .position(|e| e.id == id)
                .map(|i| (l, i, &layer.entries[i]))
        })
    }

    /// An id two edits share, if any (a damaged file).
    pub fn repeated_edit_id(&self) -> Option<EditId> {
        let mut seen = BTreeSet::new();
        self.layers
            .iter()
            .flat_map(|l| &l.entries)
            .map(|e| e.id)
            .find(|id| !seen.insert(*id))
    }
}

/// What evaluation can depend on besides the object itself. Empty for
/// now; a point in time joins it with animation, without every function
/// that evaluates having to change (DESIGN.md §4.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EvalContext {}

/// A recorded operation that no longer fully matches the shape under it:
/// some of the elements it names are gone (a cube was given fewer
/// segments, for example). The rest of the operation still applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StackWarning {
    /// Which layer, from the bottom (0).
    pub layer: usize,
    /// Which operation in the layer, from the first (0).
    pub operation: usize,
    /// How many of its elements are missing.
    pub missing: usize,
    /// Whether its points were found again by position, because the
    /// shape's points were renumbered (decision 0032). Not a problem in
    /// itself: the edit still applies where it was made.
    pub found_again: bool,
    /// Whether the edit was skipped because it would have grown the shape
    /// past [`MAX_SHAPE_POLYGONS`].
    pub too_big: bool,
}

/// An object's final shape and any warnings from working it out.
#[derive(Debug, Clone, PartialEq)]
pub struct Evaluated {
    /// The shape, in the object's own space.
    pub mesh: Arc<Mesh>,
    /// Operations that no longer fully match, bottom first.
    pub warnings: Vec<StackWarning>,
}

/// The shape a base gives on its own, or `None` for a kind without one.
fn base_mesh(base: &ObjectKind) -> Option<Arc<Mesh>> {
    match base {
        ObjectKind::Group => None,
        ObjectKind::Primitive(p) => Some(Arc::new(p.mesh())),
        ObjectKind::Mesh(mesh) => Some(Arc::clone(mesh)),
    }
}

/// Whether two bases give the same shape, without comparing whole meshes
/// when they are the same one.
fn same_base(a: &ObjectKind, b: &ObjectKind) -> bool {
    match (a, b) {
        (ObjectKind::Mesh(x), ObjectKind::Mesh(y)) => Arc::ptr_eq(x, y) || x == y,
        _ => a == b,
    }
}

/// Applies `operations[from..]` of layer `layer` to `start` (the shape
/// after the first `from` operations), adding warnings to `warnings`.
/// Returns the shape after all of them and, when there was at least one
/// to apply, the shape just before the last one: the checkpoint that lets
/// a drag or a Last edit change replay only that one (decision 0042).
fn apply_from(
    start: &Arc<Mesh>,
    layer: usize,
    entries: &[Entry],
    from: usize,
    warnings: &mut Vec<StackWarning>,
) -> (Arc<Mesh>, Option<Arc<Mesh>>) {
    if from >= entries.len() {
        return (Arc::clone(start), None);
    }
    let last = entries.len() - 1;
    let mut previous = (from == last).then(|| Arc::clone(start));
    let mut mesh = (**start).clone();
    for (op_index, entry) in entries.iter().enumerate().skip(from) {
        if op_index == last && previous.is_none() {
            // One extra copy, so the next change to the last edit is quick.
            previous = Some(Arc::new(mesh.clone()));
        }
        if !entry.enabled {
            continue;
        }
        let applied = entry.operation.apply(&mut mesh);
        if applied.missing > 0 || applied.found_again || applied.too_big {
            warnings.push(StackWarning {
                layer,
                operation: op_index,
                missing: applied.missing,
                found_again: applied.found_again,
                too_big: applied.too_big,
            });
        }
    }
    (Arc::new(mesh), previous)
}

/// Evaluates layer `index` on top of `below`, reusing what `cached` (the
/// same layer's last result on the same shape below) still has right.
/// Returns the new cached layer and how many operations were applied.
fn evaluate_layer(
    below: &Arc<Mesh>,
    index: usize,
    layer: &Layer,
    cached: Option<CachedLayer>,
) -> (CachedLayer, usize) {
    if !layer.enabled || layer.entries.is_empty() {
        let cached = CachedLayer {
            layer: layer.clone(),
            mesh: Arc::clone(below),
            previous: None,
            warnings: Vec::new(),
        };
        return (cached, 0);
    }
    // Where to start: after the longest run of operations that is still
    // the same, from the nearest shape the cache kept (none, all but the
    // last, or all of them).
    let (start, from, kept_previous, mut warnings) = match cached {
        Some(old) if old.layer.enabled => {
            let same = old
                .layer
                .entries
                .iter()
                .zip(&layer.entries)
                .take_while(|(a, b)| a == b)
                .count();
            let kept = old.layer.entries.len();
            let resume = if same == kept {
                // Everything it had is still right; its checkpoint too.
                Some((old.mesh, kept, old.previous))
            } else if kept > 0 && same == kept - 1 {
                old.previous.map(|previous| (previous, kept - 1, None))
            } else {
                None
            };
            match resume {
                Some((mesh, from, kept_previous)) => {
                    let warnings = old
                        .warnings
                        .into_iter()
                        .filter(|w| w.operation < from)
                        .collect();
                    (mesh, from, kept_previous, warnings)
                }
                None => (Arc::clone(below), 0, None, Vec::new()),
            }
        }
        _ => (Arc::clone(below), 0, None, Vec::new()),
    };
    let (mesh, previous) = apply_from(&start, index, &layer.entries, from, &mut warnings);
    let applied = layer.entries.len() - from;
    // Nothing new applied: the checkpoint is still the one it had, if the
    // layer is as long as before (after an undo it isn't known).
    let previous = if applied == 0 {
        kept_previous
    } else {
        previous
    };
    (
        CachedLayer {
            layer: layer.clone(),
            mesh,
            previous,
            warnings,
        },
        applied,
    )
}

/// Evaluates `layers` from `from` upward on top of what `out` already
/// holds (or `input`), adding each layer's result to `out`. `first` is the
/// cached result of layer `from`, whose checkpoints may still be usable.
/// Returns how many operations were applied.
fn evaluate_layers(
    input: &Arc<Mesh>,
    layers: &[Layer],
    from: usize,
    mut first: Option<CachedLayer>,
    out: &mut Vec<CachedLayer>,
) -> usize {
    let mut below = out
        .last()
        .map_or_else(|| Arc::clone(input), |l| Arc::clone(&l.mesh));
    let mut applied = 0;
    for (index, layer) in layers.iter().enumerate().skip(from) {
        let (cached, count) = evaluate_layer(&below, index, layer, first.take());
        applied += count;
        below = Arc::clone(&cached.mesh);
        out.push(cached);
    }
    applied
}

/// What the cache had to work out on its last evaluation of an object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Rebuilt {
    /// Nothing: the cached shape was still right.
    Nothing,
    /// The base and every layer.
    Everything,
    /// The layers from `layer` upward, applying `operations` operations
    /// in all (the rest came from checkpoints).
    From { layer: usize, operations: usize },
}

/// One layer's cached result.
#[derive(Debug, Clone)]
struct CachedLayer {
    /// The layer as it was evaluated, to tell when it changes.
    layer: Layer,
    /// The shape after all its operations.
    mesh: Arc<Mesh>,
    /// The shape after all but its last operation, when known: dragging or
    /// changing the last edit starts from here (decision 0042).
    previous: Option<Arc<Mesh>>,
    warnings: Vec<StackWarning>,
}

/// One object's cached evaluation.
#[derive(Debug, Clone)]
struct CachedObject {
    base: ObjectKind,
    base_mesh: Arc<Mesh>,
    layers: Vec<CachedLayer>,
    last: Rebuilt,
}

impl CachedObject {
    fn result(&self) -> Evaluated {
        Evaluated {
            mesh: self
                .layers
                .last()
                .map_or_else(|| Arc::clone(&self.base_mesh), |l| Arc::clone(&l.mesh)),
            warnings: self
                .layers
                .iter()
                .flat_map(|l| l.warnings.clone())
                .collect(),
        }
    }
}

/// Every object's evaluated shape, layer by layer, worked out when first
/// asked for and kept until the object changes.
#[derive(Debug, Default)]
pub(crate) struct StackCache {
    entries: HashMap<ObjectId, CachedObject>,
}

impl StackCache {
    /// The shape of object `id` with base `base` and `stack`, from the
    /// cache where it still matches. `None` for a kind without a shape.
    pub(crate) fn evaluate(
        &mut self,
        id: ObjectId,
        base: &ObjectKind,
        stack: &Stack,
        _context: &EvalContext,
    ) -> Option<Evaluated> {
        if let Some(entry) = self.entries.get_mut(&id)
            && same_base(&entry.base, base)
        {
            // The first layer that changed (or is new), if any.
            let first_changed = stack
                .layers
                .iter()
                .zip(&entry.layers)
                .position(|(now, cached)| *now != cached.layer)
                .unwrap_or(entry.layers.len().min(stack.layers.len()));
            if first_changed == stack.layers.len() && entry.layers.len() == stack.layers.len() {
                entry.last = Rebuilt::Nothing;
                return Some(entry.result());
            }
            // The changed layer itself can start from its checkpoints; the
            // layers above it sit on a new shape and start over.
            let mut old = entry.layers.split_off(first_changed).into_iter();
            let operations = evaluate_layers(
                &entry.base_mesh.clone(),
                &stack.layers,
                first_changed,
                old.next(),
                &mut entry.layers,
            );
            entry.last = Rebuilt::From {
                layer: first_changed,
                operations,
            };
            return Some(entry.result());
        }
        let base_mesh = base_mesh(base)?;
        let mut layers = Vec::new();
        evaluate_layers(&base_mesh, &stack.layers, 0, None, &mut layers);
        let entry = CachedObject {
            base: base.clone(),
            base_mesh,
            layers,
            last: Rebuilt::Everything,
        };
        let result = entry.result();
        self.entries.insert(id, entry);
        Some(result)
    }

    /// Forgets an object (it was deleted).
    pub(crate) fn forget(&mut self, id: ObjectId) {
        self.entries.remove(&id);
    }

    /// What the last evaluation of `id` had to work out.
    #[cfg(test)]
    pub(crate) fn last_rebuilt(&self, id: ObjectId) -> Option<Rebuilt> {
        self.entries.get(&id).map(|e| e.last)
    }
}

/// The cache as the scene keeps it: shared behind a lock so the shape can
/// be worked out while the scene is only being read. Not part of what the
/// scene *is*: a copy of a scene starts with an empty cache, and two
/// scenes compare equal whatever their caches hold.
#[derive(Default)]
pub(crate) struct SharedCache(Mutex<StackCache>);

impl SharedCache {
    /// Runs `f` with the cache. A panic elsewhere while it was held leaves
    /// a cache that is still usable (at worst it re-evaluates).
    pub(crate) fn with<T>(&self, f: impl FnOnce(&mut StackCache) -> T) -> T {
        let mut cache = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        f(&mut cache)
    }

    /// The cache, when the scene is being changed (no locking needed).
    pub(crate) fn get_mut(&mut self) -> &mut StackCache {
        self.0
            .get_mut()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl Clone for SharedCache {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl PartialEq for SharedCache {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl std::fmt::Debug for SharedCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SharedCache")
    }
}
