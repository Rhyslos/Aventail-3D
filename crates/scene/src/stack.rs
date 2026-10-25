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
use mesh::edit::{self, Bend, BevelShape, ExtrudeShape, Grouping, Outcome, SlitLine};
use mesh::{Anchors, EdgeKey, FaceId, Mesh, VertexId};
use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, Mutex};
use words::{text, tr, trf};

/// The name of the layer edits go into until layers can be managed.
/// English, the key of its translation: new layers are named with
/// [`Stack::next_layer_name`], in the language in use.
pub const EDITS_LAYER: &str = text!("Edits");

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
        /// Which way it goes, its end's size, its rows and whether its end
        /// is flat (decision 0070). Extrudes saved before file format 17
        /// have `ExtrudeShape::EARLIER`.
        shape: ExtrudeShape,
        /// How much the object was stretched along its own axes when the
        /// edit was made or last changed, so it keeps its shape as seen
        /// (decision 0082); `Vec3::ONE` for none.
        stretch: Vec3,
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
        /// How much the object was stretched along its own X, Y and Z
        /// when the bend was made or last changed (its scale on each axis
        /// over their average), so the bend keeps its shape as seen
        /// (decision 0081). `Vec3::ONE` for an object scaled evenly, and
        /// for bends saved before file format 21.
        stretch: Vec3,
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
        /// How much the object was stretched along its own axes when the
        /// edit was made or last changed, so it keeps its shape as seen
        /// (decision 0082); `Vec3::ONE` for none.
        stretch: Vec3,
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
        /// How much the object was stretched along its own axes when the
        /// edit was made or last changed, so it keeps its shape as seen
        /// (decision 0082); `Vec3::ONE` for none.
        stretch: Vec3,
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
        /// How much the object was stretched along its own axes when the
        /// edit was made or last changed, so it keeps its shape as seen
        /// (decision 0082); `Vec3::ONE` for none.
        stretch: Vec3,
        /// Where the edges were, to find them again (decision 0032).
        anchors: Option<Anchors>,
    },
    /// Cuts a groove into the shape (or raises a ridge) along edges, in
    /// one step (decision 0086).
    Groove {
        /// The edges.
        edges: Vec<EdgeKey>,
        /// Its width, depth (negative in, positive out) and sides.
        shape: edit::GrooveShape,
        /// How much the object was stretched along its own axes when the
        /// edit was made or last changed (decision 0082).
        stretch: Vec3,
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
        /// How much the object was stretched along its own axes when the
        /// edit was made or last changed, so it keeps its shape as seen
        /// (decision 0082); `Vec3::ONE` for none.
        stretch: Vec3,
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
    /// Places the whole shape by a matrix: what applying an object's
    /// position, rotation or scale records, so the object looks the same
    /// while its own placement goes back to none (decision 0089).
    PlaceShape {
        /// Where every point goes, in the shape's own space.
        matrix: Mat4,
    },
    /// Gives polygons a material slot (decision 0074).
    SetMaterial {
        /// The polygons.
        faces: Vec<FaceId>,
        /// The slot, in the object's list of materials.
        slot: u32,
        /// Where they were, to find them again (decision 0032).
        anchors: Option<Anchors>,
    },
    /// Moves, turns or scales UVs in the texture (decision 0097): those of
    /// `faces` and `points`, and the corners sharing their spots, so
    /// islands never tear (`mesh::uvs::transform_uvs`).
    TransformUvs {
        /// The polygons whose UVs change.
        faces: Vec<FaceId>,
        /// The points whose UVs change.
        points: Vec<VertexId>,
        /// The change, in the texture's units (the square is 0 to 1).
        transform: glam::Affine2,
        /// Where the polygons (or, with none, the points) were, to find
        /// them again (decision 0032).
        anchors: Option<Anchors>,
    },
    /// Pins (or unpins) the UVs of these polygons and points, so
    /// unwrapping and relaxing leave them where they are (decision 0109):
    /// see `mesh::uvs::set_pins`.
    SetUvPins {
        /// The polygons whose UVs are pinned.
        faces: Vec<FaceId>,
        /// The points whose UVs are pinned.
        points: Vec<VertexId>,
        /// Pin (`true`) or unpin.
        pin: bool,
        /// Where the polygons (or, with none, the points) were, to find
        /// them again (decision 0032).
        anchors: Option<Anchors>,
    },
    /// Lays the pieces holding these polygons flat along their seams
    /// (decisions 0099 and 0109): see `uv::unwrap_with`.
    Unwrap {
        /// Polygons of the pieces to unwrap.
        faces: Vec<FaceId>,
        /// Every choice the unwrap makes (decisions 0109 and 0110), so it
        /// replays the same however the app's settings change.
        options: uv::UnwrapOptions,
        /// Where the polygons were, to find them again (decision 0032).
        anchors: Option<Anchors>,
    },
    /// Lays the quad pieces holding these polygons out as straight grids
    /// (decision 0099).
    StraightenUvs {
        /// Polygons of the pieces to straighten.
        faces: Vec<FaceId>,
        /// Where the polygons were, to find them again.
        anchors: Option<Anchors>,
    },
    /// Scales the pieces holding these polygons so each has this texel
    /// density (decision 0100): texture widths per scene unit.
    SetTexelDensity {
        /// Polygons of the pieces to scale.
        faces: Vec<FaceId>,
        /// The density, in texture widths per scene unit.
        density: f32,
        /// Where the polygons were, to find them again.
        anchors: Option<Anchors>,
    },
    /// Lays the pieces holding these polygons straight onto a trim
    /// sheet's row (decision 0106): see `uv::trim::place`. The row is kept
    /// as its band of the square, so changing the trim sheet later doesn't
    /// move pieces already laid on it.
    PlaceOnTrim {
        /// Polygons of the pieces to lay on the row.
        faces: Vec<FaceId>,
        /// Where the row starts, from the bottom of the square (0 to 1).
        bottom: f32,
        /// Where it ends.
        top: f32,
        /// Where the polygons were, to find them again (decision 0032).
        anchors: Option<Anchors>,
    },
    /// Moves whole UV islands to where packing put them (decision 0101):
    /// each island's polygons and its move (turn, scale and place), in
    /// the texture's units. Exactly those polygons' corners move.
    PackUvs {
        /// Each island's polygons and move.
        islands: Vec<(Vec<FaceId>, glam::Affine2)>,
    },
    /// Puts a saved UV layout back (decision 0114): the UVs of the
    /// polygons it knows and the seams between them, as a layout snapshot
    /// held them. See `mesh::uvs::restore_layout`.
    RestoreUvs {
        /// The layout (shared with the snapshot it came from).
        layout: std::sync::Arc<mesh::uvs::UvLayout>,
    },
    /// Evens out the UVs of these polygons (decision 0099).
    RelaxUvs {
        /// The polygons whose corners move.
        faces: Vec<FaceId>,
        /// How many steps of evening out (decision 0110; the standard is
        /// `uv::unwrap::RELAX_STEPS`).
        steps: u32,
        /// Where the polygons were, to find them again.
        anchors: Option<Anchors>,
    },
    /// Marks edges as seams, where the surface is cut open to lie flat,
    /// or clears them (decision 0098).
    SetSeams {
        /// The edges.
        edges: Vec<EdgeKey>,
        /// Mark, or clear.
        seam: bool,
        /// Where they were, to find them again (decision 0032).
        anchors: Option<Anchors>,
    },
    /// Tells the automatic UV tools where seams should or shouldn't go
    /// (decision 0111): see `mesh::uvs::set_seam_preference`.
    SetSeamPreference {
        /// The edges.
        edges: Vec<EdgeKey>,
        /// Prefer, never, or nothing (clears it).
        preference: mesh::uvs::SeamPreference,
        /// Where they were, to find them again (decision 0032).
        anchors: Option<Anchors>,
    },
    /// Gives the pieces holding these polygons a stretch budget of their
    /// own, or takes it away (decision 0111): see `mesh::uvs::set_budget`.
    SetStretchBudget {
        /// The polygons (the budget is set on these; a piece takes the
        /// least of its polygons').
        faces: Vec<FaceId>,
        /// The budget, or `None` to clear it.
        budget: Option<f32>,
        /// Where the polygons were, to find them again.
        anchors: Option<Anchors>,
    },
    /// Locks (or unlocks) the UVs of these polygons, so no automatic tool
    /// changes them (decision 0112): see `mesh::uvs::set_locked`.
    SetUvLock {
        /// The polygons (whole pieces, as the tools choose them).
        faces: Vec<FaceId>,
        /// Lock, or unlock.
        lock: bool,
        /// Where the polygons were, to find them again.
        anchors: Option<Anchors>,
    },
    /// Gives these polygons' pieces their own unwrap settings, or takes
    /// them away (all `None`) (decision 0112): see `uv::PieceSettings`.
    SetPieceSettings {
        /// The polygons (whole pieces, as the tools choose them).
        faces: Vec<FaceId>,
        /// The settings.
        settings: uv::PieceSettings,
        /// Where the polygons were, to find them again.
        anchors: Option<Anchors>,
    },
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
    /// Slides points along their edges towards a neighbour each
    /// (decision 0055).
    SlidePoints {
        /// Each point, its neighbour and how far along the edge.
        slides: Vec<edit::Slide>,
        /// Where the points were, then their neighbours (decision 0032).
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
    /// An edit made with symmetry on: made on its own elements and on
    /// their mirrors across `mirror` ([decision 0059](../../../docs/decisions/0059-symmetry-for-every-edit.md),
    /// [`crate::mirroring`]). Its settings are the edit's own, so changing
    /// them changes both sides.
    Mirrored {
        /// The edit as it was made.
        op: Box<Operation>,
        /// The mirror planes.
        mirror: mesh::symmetry::Mirror,
    },
    /// Joins the two halves of a symmetrical shape at the middle, once,
    /// when asked (the Symmetry menu's Weld the middle now, decision 0059):
    /// points within `distance` of a mirror plane are put on it, and
    /// points meeting there become one.
    WeldMiddle {
        /// The mirror planes.
        mirror: mesh::symmetry::Mirror,
        /// How near counts, in the object's own space (centimetres).
        distance: f32,
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
    /// A straight extrude's way, end size, steps and flat end (decision
    /// 0070); none for a bent extrude or other edits.
    pub extrude: Option<ExtrudeShape>,
    /// How much the object is stretched along each of its axes, for an
    /// extrude, inset or bevel (decisions 0081 and 0082); none to keep
    /// what it has.
    pub stretch: Option<Vec3>,
    /// A bevel of edges' skew, even width, stopping at overlaps and
    /// welding (decision 0085); none for other edits.
    pub bevel_options: Option<edit::BevelOptions>,
    /// A groove's width, depth and sides (decision 0086; its depth is
    /// also `distance`); none for other edits.
    pub groove: Option<edit::GrooveShape>,
}

impl EditSettings {
    /// These settings with a bevel of edges made the improved way
    /// (decision 0141). A bevel recorded before it keeps its old shape
    /// until a person changes one of its settings: then it gets rounded
    /// corners and stops at its neighbours point by point, as new bevels
    /// do.
    #[must_use]
    pub fn improved(self) -> Self {
        Self {
            bevel_options: self.bevel_options.map(|o| edit::BevelOptions {
                improved: true,
                ..o
            }),
            ..self
        }
    }
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
                1 => tr("Move 1 point").to_string(),
                n => trf!("Move {} points", n),
            },
            Self::TransformPoints { points, .. } => match points.len() {
                1 => tr("Transform 1 point").to_string(),
                n => trf!("Transform {} points", n),
            },
            Self::ExtrudePolygons { faces, .. } => {
                counted(text!("Extrude"), faces.len(), text!("polygon"))
            }
            Self::ExtrudeBent { faces, bend, .. } if bend.spirals() => {
                trf!(
                    "{} with a spiral",
                    counted(text!("Extrude"), faces.len(), text!("polygon"))
                )
            }
            Self::ExtrudeBent { faces, bend, .. } if bend.bends() => {
                trf!(
                    "{} with a bend",
                    counted(text!("Extrude"), faces.len(), text!("polygon"))
                )
            }
            Self::ExtrudeBent { faces, .. } => {
                counted(text!("Extrude"), faces.len(), text!("polygon"))
            }
            Self::InsetPolygons { faces, .. } => {
                counted(text!("Inset"), faces.len(), text!("polygon"))
            }
            Self::ExtrudeEdges { edges, .. } => {
                counted(text!("Extrude"), edges.len(), text!("edge"))
            }
            Self::BevelEdges { edges, .. } => counted(text!("Bevel"), edges.len(), text!("edge")),
            Self::Groove { edges, shape, .. } => {
                let what = if shape.depth > 0.0 {
                    text!("Ridge along")
                } else {
                    text!("Groove along")
                };
                counted(what, edges.len(), text!("edge"))
            }
            Self::BevelPoints { points, .. } => {
                counted(text!("Bevel"), points.len(), text!("point"))
            }
            Self::LoopCut { cuts: 1, .. } => tr("Loop cut").to_string(),
            Self::LoopCut { cuts, .. } => trf!("Loop cut ({} cuts)", cuts),
            Self::Connect { points, edges, .. } if edges.is_empty() => {
                counted(text!("Connect"), points.len(), text!("point"))
            }
            Self::Connect { edges, .. } => counted(text!("Connect"), edges.len(), text!("edge")),
            Self::Bridge { faces, .. } if !faces.is_empty() => tr("Bridge 2 polygons").to_string(),
            Self::DeleteElements {
                points,
                edges,
                faces,
                ..
            } => elements(text!("Delete"), points, edges, faces),
            Self::Dissolve {
                points,
                edges,
                faces,
                ..
            } => elements(text!("Dissolve"), points, edges, faces),
            Self::FillHoles { points, .. } if points.is_empty() => {
                tr("Fill every hole").to_string()
            }
            Self::FillHoles { .. } => tr("Fill hole").to_string(),
            Self::FlipPolygons { faces, .. } => {
                counted(text!("Flip"), faces.len(), text!("polygon"))
            }
            Self::SetMaterial { faces, .. } => {
                counted(text!("Set the material of"), faces.len(), text!("polygon"))
            }
            Self::TransformUvs { faces, points, .. } if faces.is_empty() => {
                counted(text!("Move the UVs of"), points.len(), text!("point"))
            }
            Self::TransformUvs { faces, .. } => {
                counted(text!("Move the UVs of"), faces.len(), text!("polygon"))
            }
            Self::SetUvPins {
                faces, points, pin, ..
            } => {
                let verb = if *pin {
                    text!("Pin the UVs of")
                } else {
                    text!("Unpin the UVs of")
                };
                if faces.is_empty() {
                    counted(verb, points.len(), text!("point"))
                } else {
                    counted(verb, faces.len(), text!("polygon"))
                }
            }
            Self::Unwrap { faces, .. } => counted(text!("Unwrap"), faces.len(), text!("polygon")),
            Self::StraightenUvs { faces, .. } => counted(
                text!("Straighten the UVs of"),
                faces.len(),
                text!("polygon"),
            ),
            Self::RelaxUvs { faces, .. } => {
                counted(text!("Relax the UVs of"), faces.len(), text!("polygon"))
            }
            Self::SetTexelDensity { faces, .. } => counted(
                text!("Set the texel density of"),
                faces.len(),
                text!("polygon"),
            ),
            Self::PackUvs { islands } => counted(text!("Pack"), islands.len(), text!("island")),
            Self::RestoreUvs { layout } => counted(
                text!("Restore the UVs of"),
                layout.polygon_count(),
                text!("polygon"),
            ),
            Self::PlaceOnTrim { faces, .. } => counted(
                text!("Put on the trim sheet"),
                faces.len(),
                text!("polygon"),
            ),
            Self::FixFacing => tr("Fix normals").to_string(),
            Self::PlaceShape { .. } => tr("Apply the placement").to_string(),
            Self::SetSeams { edges, seam, .. } => {
                let verb = if *seam {
                    text!("Mark as seams")
                } else {
                    text!("Clear the seams of")
                };
                counted(verb, edges.len(), text!("edge"))
            }
            Self::SetSeamPreference {
                edges, preference, ..
            } => {
                let verb = match preference {
                    mesh::uvs::SeamPreference::Prefer => text!("Prefer seams on"),
                    mesh::uvs::SeamPreference::Never => text!("Allow no seams on"),
                    mesh::uvs::SeamPreference::Free => text!("Clear the seam preference of"),
                };
                counted(verb, edges.len(), text!("edge"))
            }
            Self::SetStretchBudget { faces, budget, .. } => {
                let verb = if budget.is_some() {
                    text!("Set the stretch budget of")
                } else {
                    text!("Clear the stretch budget of")
                };
                counted(verb, faces.len(), text!("polygon"))
            }
            Self::SetUvLock { faces, lock, .. } => {
                let verb = if *lock {
                    text!("Lock the UVs of")
                } else {
                    text!("Unlock the UVs of")
                };
                counted(verb, faces.len(), text!("polygon"))
            }
            Self::SetPieceSettings {
                faces, settings, ..
            } => {
                let verb = if settings.is_empty() {
                    text!("Clear the piece settings of")
                } else {
                    text!("Set the piece settings of")
                };
                counted(verb, faces.len(), text!("polygon"))
            }
            Self::SetHardEdges { edges, hard, .. } => {
                let verb = if *hard {
                    text!("Harden")
                } else {
                    text!("Soften")
                };
                counted(verb, edges.len(), text!("edge"))
            }
            Self::HardEdgesByAngle { .. } => tr("Hard edges by angle").to_string(),
            Self::SlidePoints { slides, .. } => {
                counted(text!("Slide"), slides.len(), text!("point"))
            }
            Self::Bridge { edges, .. } => counted(text!("Bridge"), edges.len(), text!("edge")),
            Self::Weld { points, target, .. } => {
                let n = points.len() + usize::from(target.is_some_and(|t| !points.contains(&t)));
                counted(text!("Weld"), n, text!("point"))
            }
            Self::Slit { line } if line.through => tr("Slit through").to_string(),
            Self::Slit { .. } => tr("Slit").to_string(),
            Self::Mirrored { op, .. } => trf!("{}, mirrored", op.describe()),
            Self::WeldMiddle { .. } => tr("Weld the middle").to_string(),
        }
    }

    /// The edit itself: for a mirrored edit, the edit as it was made;
    /// otherwise this one.
    pub fn inner(&self) -> &Operation {
        match self {
            Self::Mirrored { op, .. } => op,
            _ => self,
        }
    }

    /// The settings that can still be changed, for an extrude, inset or
    /// bevel.
    pub fn settings(&self) -> Option<EditSettings> {
        if let Self::Mirrored { op, .. } = self {
            return op.settings();
        }
        let stretch = self.keeps_stretch().then(|| self.stretch());
        self.own_settings().map(|settings| EditSettings {
            stretch,
            ..settings
        })
    }

    /// The settings, without the stretch.
    fn own_settings(&self) -> Option<EditSettings> {
        let plain = |distance: f32, grouping: Grouping| EditSettings {
            distance: Some(distance),
            grouping: Some(grouping),
            ..EditSettings::default()
        };
        match self {
            Self::ExtrudePolygons {
                distance,
                grouping,
                shape,
                ..
            } => Some(EditSettings {
                bend: Some(Bend::default()),
                extrude: Some(*shape),
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
                bevel_options: Some(shape.options),
                ..EditSettings::default()
            }),
            Self::BevelPoints { width, .. } => Some(EditSettings {
                distance: Some(*width),
                ..EditSettings::default()
            }),
            Self::Groove { shape, .. } => Some(EditSettings {
                distance: Some(shape.depth),
                groove: Some(*shape),
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
            | Self::SetMaterial { .. }
            | Self::TransformUvs { .. }
            | Self::SetUvPins { .. }
            | Self::Unwrap { .. }
            | Self::StraightenUvs { .. }
            | Self::RelaxUvs { .. }
            | Self::SetTexelDensity { .. }
            | Self::SetStretchBudget { .. }
            | Self::SetUvLock { .. }
            | Self::SetPieceSettings { .. }
            | Self::PlaceOnTrim { .. }
            | Self::PackUvs { .. }
            | Self::RestoreUvs { .. }
            | Self::SetHardEdges { .. }
            | Self::SetSeams { .. }
            | Self::SetSeamPreference { .. }
            | Self::SlidePoints { .. }
            | Self::FixFacing
            | Self::PlaceShape { .. } => None,
            Self::MovePoints { .. }
            | Self::TransformPoints { .. }
            | Self::Mirrored { .. }
            | Self::WeldMiddle { .. } => None,
        }
    }

    /// The same edit with new settings (those it doesn't have are
    /// ignored; an edit without settings is unchanged).
    #[must_use]
    pub fn with(&self, settings: EditSettings) -> Self {
        if let Self::Mirrored { op, mirror } = self {
            return Self::Mirrored {
                op: Box::new(op.with(settings)),
                mirror: *mirror,
            };
        }
        // A straight extrude given a bend becomes a bent one.
        if let (
            Self::ExtrudePolygons {
                faces,
                distance,
                grouping,
                stretch,
                anchors,
                ..
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
                stretch: *stretch,
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
                distance,
                grouping,
                shape,
                ..
            } => {
                if let Some(d) = settings.distance {
                    *distance = d;
                }
                if let Some(g) = settings.grouping {
                    *grouping = g;
                }
                if let Some(s) = settings.extrude {
                    *shape = s.clamped();
                }
            }
            Self::InsetPolygons {
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
                if let Some(o) = settings.bevel_options {
                    shape.options = o.clamped();
                }
            }
            Self::BevelPoints { width, .. } => {
                if let Some(w) = settings.distance {
                    *width = w;
                }
            }
            Self::Groove { shape, .. } => {
                if let Some(g) = settings.groove {
                    *shape = g;
                }
                if let Some(d) = settings.distance {
                    shape.depth = d;
                }
                if let Some(clamped) = shape.clamped() {
                    *shape = clamped;
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
            | Self::SetMaterial { .. }
            | Self::TransformUvs { .. }
            | Self::SetUvPins { .. }
            | Self::Unwrap { .. }
            | Self::StraightenUvs { .. }
            | Self::RelaxUvs { .. }
            | Self::SetTexelDensity { .. }
            | Self::SetStretchBudget { .. }
            | Self::SetUvLock { .. }
            | Self::SetPieceSettings { .. }
            | Self::PlaceOnTrim { .. }
            | Self::PackUvs { .. }
            | Self::RestoreUvs { .. }
            | Self::SetHardEdges { .. }
            | Self::SetSeams { .. }
            | Self::SetSeamPreference { .. }
            | Self::SlidePoints { .. }
            | Self::FixFacing
            | Self::PlaceShape { .. } => {}
            Self::MovePoints { .. }
            | Self::TransformPoints { .. }
            | Self::Mirrored { .. }
            | Self::WeldMiddle { .. } => {}
        }
        if let Some(s) = settings.stretch
            && let Some(stretch) = changed.stretch_mut()
        {
            *stretch = mesh::stretch::usable(s);
        }
        // A bend taken back to nothing makes a straight extrude again,
        // shaped as asked (straight out as standard, decision 0070).
        if let Self::ExtrudeBent {
            faces,
            distance,
            grouping,
            bend,
            stretch,
            anchors,
        } = &changed
            && !bend.bends()
            && settings.bend.is_some()
        {
            return Self::ExtrudePolygons {
                faces: faces.clone(),
                distance: *distance,
                grouping: *grouping,
                shape: settings.extrude.unwrap_or_default().clamped(),
                stretch: *stretch,
                anchors: anchors.clone(),
            };
        }
        changed
    }

    /// Whether the edit is worked out as the object is seen, keeping the
    /// object's stretch (decisions 0081 and 0082): extrudes, insets and
    /// bevels, which measure across the shape. Moves and turns of points
    /// are already in the object's own space, and cuts don't measure.
    pub fn keeps_stretch(&self) -> bool {
        matches!(
            self.inner(),
            Self::ExtrudePolygons { .. }
                | Self::ExtrudeBent { .. }
                | Self::InsetPolygons { .. }
                | Self::ExtrudeEdges { .. }
                | Self::BevelEdges { .. }
                | Self::BevelPoints { .. }
                | Self::Groove { .. }
        )
    }

    /// How much the object was stretched along its own axes when the edit
    /// was made or last changed; `Vec3::ONE` for none, and for edits that
    /// don't keep one ([`Self::keeps_stretch`]).
    pub fn stretch(&self) -> Vec3 {
        match self.inner() {
            Self::ExtrudePolygons { stretch, .. }
            | Self::ExtrudeBent { stretch, .. }
            | Self::InsetPolygons { stretch, .. }
            | Self::ExtrudeEdges { stretch, .. }
            | Self::BevelEdges { stretch, .. }
            | Self::BevelPoints { stretch, .. }
            | Self::Groove { stretch, .. } => *stretch,
            _ => Vec3::ONE,
        }
    }

    /// The edit's stretch, to change, if it keeps one.
    fn stretch_mut(&mut self) -> Option<&mut Vec3> {
        match self {
            Self::Mirrored { op, .. } => op.stretch_mut(),
            Self::ExtrudePolygons { stretch, .. }
            | Self::ExtrudeBent { stretch, .. }
            | Self::InsetPolygons { stretch, .. }
            | Self::ExtrudeEdges { stretch, .. }
            | Self::BevelEdges { stretch, .. }
            | Self::BevelPoints { stretch, .. }
            | Self::Groove { stretch, .. } => Some(stretch),
            _ => None,
        }
    }

    /// The same edit with `stretch` (made usable, see
    /// [`mesh::stretch::usable`]); an edit that doesn't keep one is
    /// unchanged.
    #[must_use]
    pub fn with_stretch(&self, stretch: Vec3) -> Self {
        let mut changed = self.clone();
        if let Some(s) = changed.stretch_mut() {
            *s = mesh::stretch::usable(stretch);
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
            Self::Groove { shape, .. } => shape.width.is_finite() && shape.depth.is_finite(),
            Self::BevelPoints { width, .. } => width.is_finite(),
            Self::LoopCut { position, .. } => position.is_finite(),
            Self::Connect { .. }
            | Self::Bridge { .. }
            | Self::Weld { .. }
            | Self::DeleteElements { .. }
            | Self::Dissolve { .. }
            | Self::FillHoles { .. }
            | Self::FlipPolygons { .. }
            | Self::SetMaterial { .. }
            | Self::SetHardEdges { .. }
            | Self::SetSeams { .. }
            | Self::SetUvPins { .. }
            | Self::StraightenUvs { .. }
            | Self::FixFacing => true,
            Self::Unwrap { options, .. } => options.is_real(),
            Self::RelaxUvs { steps, .. } => *steps <= uv::unwrap::MAX_RELAX_STEPS,
            Self::SetTexelDensity { density, .. } => density.is_finite() && *density > 0.0,
            Self::SetSeamPreference { .. } => true,
            Self::SetStretchBudget { budget, .. } => {
                budget.is_none_or(|b| mesh::uvs::BUDGET_RANGE.contains(&b))
            }
            Self::SetUvLock { .. } => true,
            Self::SetPieceSettings { settings, .. } => settings.is_real(),
            Self::PlaceOnTrim { bottom, top, .. } => {
                bottom.is_finite()
                    && top.is_finite()
                    && *bottom >= 0.0
                    && top > bottom
                    && *top <= 1.0
            }
            Self::PackUvs { islands } => islands.iter().all(|(_, t)| t.is_finite()),
            Self::RestoreUvs { layout } => layout.is_real(),
            Self::TransformUvs { transform, .. } => transform.is_finite(),
            Self::PlaceShape { matrix } => matrix.is_finite() && matrix.determinant().abs() > 1e-12,
            Self::SlidePoints { slides, .. } => slides.iter().all(|s| s.amount.is_finite()),
            Self::HardEdgesByAngle { angle } => angle.is_finite(),
            Self::ExtrudeBent { distance, bend, .. } => {
                distance.is_finite()
                    && [bend.angle, bend.direction, bend.radius, bend.rise]
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
            Self::Mirrored { op, mirror } => op.numbers_are_real() && mirror.centre.is_finite(),
            Self::WeldMiddle { mirror, distance } => {
                mirror.centre.is_finite() && distance.is_finite()
            }
        };
        own && anchors_ok && self.stretch().is_finite()
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
            | Self::Groove { anchors, .. }
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
            | Self::SetMaterial { anchors, .. }
            | Self::TransformUvs { anchors, .. }
            | Self::SetUvPins { anchors, .. }
            | Self::Unwrap { anchors, .. }
            | Self::StraightenUvs { anchors, .. }
            | Self::RelaxUvs { anchors, .. }
            | Self::SetTexelDensity { anchors, .. }
            | Self::SetStretchBudget { anchors, .. }
            | Self::SetUvLock { anchors, .. }
            | Self::SetPieceSettings { anchors, .. }
            | Self::PlaceOnTrim { anchors, .. }
            | Self::SetHardEdges { anchors, .. }
            | Self::SetSeams { anchors, .. }
            | Self::SetSeamPreference { anchors, .. }
            | Self::SlidePoints { anchors, .. } => anchors.as_ref(),
            Self::FixFacing | Self::PlaceShape { .. } | Self::HardEdgesByAngle { .. } => None,
            Self::Slit { .. } | Self::PackUvs { .. } | Self::RestoreUvs { .. } => None,
            Self::Mirrored { op, .. } => op.anchors(),
            Self::WeldMiddle { .. } => None,
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
            | Self::Groove { anchors, .. }
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
            | Self::SetMaterial { anchors, .. }
            | Self::TransformUvs { anchors, .. }
            | Self::SetUvPins { anchors, .. }
            | Self::Unwrap { anchors, .. }
            | Self::StraightenUvs { anchors, .. }
            | Self::RelaxUvs { anchors, .. }
            | Self::SetTexelDensity { anchors, .. }
            | Self::SetStretchBudget { anchors, .. }
            | Self::SetUvLock { anchors, .. }
            | Self::SetPieceSettings { anchors, .. }
            | Self::PlaceOnTrim { anchors, .. }
            | Self::SetHardEdges { anchors, .. }
            | Self::SetSeams { anchors, .. }
            | Self::SetSeamPreference { anchors, .. }
            | Self::SlidePoints { anchors, .. } => anchors.as_mut(),
            Self::FixFacing | Self::PlaceShape { .. } | Self::HardEdgesByAngle { .. } => None,
            Self::Slit { .. } | Self::PackUvs { .. } | Self::RestoreUvs { .. } => None,
            Self::Mirrored { op, .. } => op.anchors_mut(),
            Self::WeldMiddle { .. } => None,
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
            Self::Mirrored { op, .. } => op.points(),
            _ => (&[], self.anchors()),
        }
    }

    /// Whether the operation reshapes polygons (adds, removes or rewires
    /// them), so UVs it leaves without room are mended afterwards
    /// ([decision 0104](../../../docs/decisions/0104-uvs-that-survive-modelling.md)).
    /// Moving points, flipping polygons and UV, seam, hardness and
    /// material edits don't: their results are what was asked for.
    pub fn mends_uvs(&self) -> bool {
        match self {
            Self::ExtrudePolygons { .. }
            | Self::ExtrudeBent { .. }
            | Self::InsetPolygons { .. }
            | Self::ExtrudeEdges { .. }
            | Self::BevelEdges { .. }
            | Self::Groove { .. }
            | Self::BevelPoints { .. }
            | Self::LoopCut { .. }
            | Self::Connect { .. }
            | Self::Bridge { .. }
            | Self::Weld { .. }
            | Self::DeleteElements { .. }
            | Self::Dissolve { .. }
            | Self::FillHoles { .. }
            | Self::Slit { .. }
            | Self::WeldMiddle { .. } => true,
            Self::MovePoints { .. }
            | Self::TransformPoints { .. }
            | Self::SlidePoints { .. }
            | Self::FlipPolygons { .. }
            | Self::FixFacing
            | Self::PlaceShape { .. }
            | Self::SetMaterial { .. }
            | Self::TransformUvs { .. }
            | Self::SetUvPins { .. }
            | Self::Unwrap { .. }
            | Self::StraightenUvs { .. }
            | Self::RelaxUvs { .. }
            | Self::PackUvs { .. }
            | Self::RestoreUvs { .. }
            | Self::SetTexelDensity { .. }
            | Self::SetStretchBudget { .. }
            | Self::SetUvLock { .. }
            | Self::SetPieceSettings { .. }
            | Self::PlaceOnTrim { .. }
            | Self::SetSeams { .. }
            | Self::SetSeamPreference { .. }
            | Self::SetHardEdges { .. }
            | Self::HardEdgesByAngle { .. } => false,
            // Each side mends as it is applied.
            Self::Mirrored { .. } => false,
        }
    }

    /// Applies the operation to `mesh`, then mends the UVs it left without
    /// room (see [`Self::mends_uvs`]).
    fn apply(&self, mesh: &mut Mesh, context: &EvalContext) -> Applied {
        let before = (self.mends_uvs()
            && context.mending.on
            && mesh.attribute_values(mesh::attributes::UV).is_some())
        .then(|| mesh.clone());
        let applied = self.apply_plain(mesh, context);
        if let Some(before) = before {
            uv::mend::mend_with(&before, mesh, &context.mending);
        }
        applied
    }

    /// Applies the operation to `mesh`. Its ids are used as they are when
    /// the shape has the structure the edit was made on (decision 0044), or
    /// its anchored places still fit. Otherwise (a primitive given more
    /// segments, say) its elements are found again by their anchors.
    fn apply_plain(&self, mesh: &mut Mesh, context: &EvalContext) -> Applied {
        // With symmetry: on both sides, together or as copies.
        if let Self::Mirrored { op, mirror } = self {
            // Clipping needs the points as they were, to keep them on
            // their side.
            let before = mirror.clip.then(|| mesh.clone());
            let applied = match crate::mirroring::both_sides(op, mesh, mirror) {
                crate::mirroring::Both::Together(all) => all.apply(mesh, context),
                crate::mirroring::Both::Copies(copies) => {
                    let mut applied = op.apply(mesh, context);
                    for copy in copies {
                        let more = copy.apply(mesh, context);
                        applied.missing += more.missing;
                        applied.found_again |= more.found_again;
                        applied.too_big |= more.too_big;
                    }
                    applied
                }
            };
            if let Some(before) = before {
                mesh::symmetry::keep_sides(&before, mesh, mirror);
            }
            return applied;
        }
        let same = self.anchors().is_some_and(|a| a.same_structure(mesh));
        // The points at the anchored places, when the ids can't be trusted.
        let refound = match self.anchors() {
            Some(anchors) if same || anchors.still_fit(mesh) => None,
            // By place, points on the same spot told apart by their stable
            // names (decision 0090).
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
                shape,
                stretch,
                ..
            } => {
                let faces = refound
                    .as_ref()
                    .map_or_else(|| faces.clone(), |r| faces_within(mesh, &r.0));
                // A wall of every row round each polygon.
                let rows = shape.clamped().steps as usize;
                if too_big(corner_count(mesh, &faces).saturating_mul(rows)) {
                    return skipped;
                }
                mesh::stretch::as_seen(mesh, *stretch, |mesh| {
                    edit::extrude_shaped(mesh, &faces, *distance, *grouping, *shape)
                })
            }
            Self::ExtrudeBent {
                faces,
                distance,
                grouping,
                bend,
                stretch,
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
                mesh::stretch::as_seen(mesh, *stretch, |mesh| {
                    edit::extrude_bent(mesh, &faces, *distance, *grouping, *bend)
                })
            }
            Self::InsetPolygons {
                faces,
                distance,
                grouping,
                stretch,
                ..
            } => {
                let faces = refound
                    .as_ref()
                    .map_or_else(|| faces.clone(), |r| faces_within(mesh, &r.0));
                if too_big(corner_count(mesh, &faces)) {
                    return skipped;
                }
                mesh::stretch::as_seen(mesh, *stretch, |mesh| {
                    edit::inset_polygons(mesh, &faces, *distance, *grouping)
                })
            }
            Self::ExtrudeEdges {
                edges,
                distance,
                grouping,
                stretch,
                ..
            } => {
                let edges = refound
                    .as_ref()
                    .map_or_else(|| edges.clone(), |r| edges_within(mesh, &r.0));
                if too_big(edges.len()) {
                    return skipped;
                }
                mesh::stretch::as_seen(mesh, *stretch, |mesh| {
                    edit::extrude_edges(mesh, &edges, *distance, *grouping)
                })
            }
            Self::BevelEdges {
                edges,
                shape,
                stretch,
                ..
            } => {
                let edges = refound
                    .as_ref()
                    .map_or_else(|| edges.clone(), |r| edges_within(mesh, &r.0));
                // A strip of rows per edge, and a corner polygon at each end.
                let rows = shape.segments.clamp(1, mesh::bevel::MAX_SEGMENTS) as usize;
                if too_big(edges.len().saturating_mul(rows + 2)) {
                    return skipped;
                }
                mesh::stretch::as_seen(mesh, *stretch, |mesh| {
                    edit::bevel_edges(mesh, &edges, *shape)
                })
            }
            Self::Groove {
                edges,
                shape,
                stretch,
                ..
            } => {
                let edges = refound
                    .as_ref()
                    .map_or_else(|| edges.clone(), |r| edges_within(mesh, &r.0));
                // A strip and its walls per edge, and ends.
                if too_big(edges.len().saturating_mul(8)) {
                    return skipped;
                }
                mesh::stretch::as_seen(mesh, *stretch, |mesh| {
                    edit::groove_edges(mesh, &edges, *shape)
                })
            }
            Self::BevelPoints {
                points,
                width,
                stretch,
                ..
            } => {
                let points = refound.as_ref().map_or(points.as_slice(), |r| &r.0);
                // One corner polygon per point.
                if too_big(points.len()) {
                    return skipped;
                }
                mesh::stretch::as_seen(mesh, *stretch, |mesh| {
                    edit::bevel_points(mesh, points, *width)
                })
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
            Self::SetMaterial { faces, slot, .. } => {
                let faces = refound
                    .as_ref()
                    .map_or_else(|| faces.clone(), |r| faces_within(mesh, &r.0));
                edit::set_material(mesh, &faces, *slot)
            }
            Self::TransformUvs {
                faces,
                points,
                transform,
                ..
            } => {
                // The anchors hold the polygons' corners, or the points.
                let (faces, points) = match (&refound, faces.is_empty()) {
                    (Some(r), false) => (faces_within(mesh, &r.0), Vec::new()),
                    (Some(r), true) => (Vec::new(), r.0.clone()),
                    (None, _) => (faces.clone(), points.clone()),
                };
                mesh::uvs::transform_uvs(mesh, &faces, &points, *transform)
            }
            Self::SetUvPins {
                faces, points, pin, ..
            } => {
                // The anchors hold the polygons' corners, or the points.
                let (faces, points) = match (&refound, faces.is_empty()) {
                    (Some(r), false) => (faces_within(mesh, &r.0), Vec::new()),
                    (Some(r), true) => (Vec::new(), r.0.clone()),
                    (None, _) => (faces.clone(), points.clone()),
                };
                mesh::uvs::set_pins(mesh, &faces, &points, *pin)
            }
            Self::Unwrap { faces, options, .. } => {
                let faces = refound
                    .as_ref()
                    .map_or_else(|| faces.clone(), |r| faces_within(mesh, &r.0));
                uv::unwrap_with(mesh, &faces, *options)
            }
            Self::StraightenUvs { faces, .. } => {
                let faces = refound
                    .as_ref()
                    .map_or_else(|| faces.clone(), |r| faces_within(mesh, &r.0));
                uv::straighten(mesh, &faces)
            }
            Self::RelaxUvs { faces, steps, .. } => {
                let faces = refound
                    .as_ref()
                    .map_or_else(|| faces.clone(), |r| faces_within(mesh, &r.0));
                uv::relax(
                    mesh,
                    &faces,
                    (*steps).min(uv::unwrap::MAX_RELAX_STEPS) as usize,
                )
            }
            Self::PackUvs { islands } => uv::pack::place(mesh, islands),
            Self::RestoreUvs { layout } => mesh::uvs::restore_layout(mesh, layout),
            Self::SetTexelDensity { faces, density, .. } => {
                let faces = refound
                    .as_ref()
                    .map_or_else(|| faces.clone(), |r| faces_within(mesh, &r.0));
                uv::density::set_density(mesh, &faces, *density)
            }
            Self::SetStretchBudget { faces, budget, .. } => {
                let faces = refound
                    .as_ref()
                    .map_or_else(|| faces.clone(), |r| faces_within(mesh, &r.0));
                mesh::uvs::set_budget(mesh, &faces, *budget)
            }
            Self::SetUvLock { faces, lock, .. } => {
                let faces = refound
                    .as_ref()
                    .map_or_else(|| faces.clone(), |r| faces_within(mesh, &r.0));
                mesh::uvs::set_locked(mesh, &faces, *lock)
            }
            Self::SetPieceSettings {
                faces, settings, ..
            } => {
                let faces = refound
                    .as_ref()
                    .map_or_else(|| faces.clone(), |r| faces_within(mesh, &r.0));
                mesh::uvs::set_piece_values(mesh, &faces, settings.values())
            }
            Self::PlaceOnTrim {
                faces, bottom, top, ..
            } => {
                let faces = refound
                    .as_ref()
                    .map_or_else(|| faces.clone(), |r| faces_within(mesh, &r.0));
                uv::trim::place(mesh, &faces, *bottom, *top)
            }
            Self::FixFacing => edit::fix_facing(mesh),
            Self::PlaceShape { matrix } => edit::place(mesh, *matrix),
            Self::SetSeams { edges, seam, .. } => {
                let edges = refound
                    .as_ref()
                    .map_or_else(|| edges.clone(), |r| edges_within(mesh, &r.0));
                mesh::uvs::set_seams(mesh, &edges, *seam)
            }
            Self::SetSeamPreference {
                edges, preference, ..
            } => {
                let edges = refound
                    .as_ref()
                    .map_or_else(|| edges.clone(), |r| edges_within(mesh, &r.0));
                mesh::uvs::set_seam_preference(mesh, &edges, *preference)
            }
            Self::SetHardEdges { edges, hard, .. } => {
                let edges = refound
                    .as_ref()
                    .map_or_else(|| edges.clone(), |r| edges_within(mesh, &r.0));
                edit::set_hard_edges(mesh, &edges, *hard)
            }
            Self::HardEdgesByAngle { angle } => edit::hard_edges_by_angle(mesh, *angle),
            Self::SlidePoints { slides, anchors } => match (&refound, anchors) {
                (Some((found, _)), Some(anchors)) => {
                    // Each point and neighbour is the one found nearest
                    // where it was.
                    let mesh::AnchorShape::Points(places) = &anchors.shape else {
                        return skipped;
                    };
                    let nearest = |was: Vec3| {
                        found.iter().copied().min_by(|a, b| {
                            let d = |v: &VertexId| {
                                mesh.position(*v).map_or(f32::MAX, |p| p.distance(was))
                            };
                            d(a).total_cmp(&d(b))
                        })
                    };
                    let n = slides.len();
                    let moved: Vec<edit::Slide> = slides
                        .iter()
                        .enumerate()
                        .filter_map(|(i, s)| {
                            Some(edit::Slide {
                                point: nearest(*places.get(i)?)?,
                                towards: nearest(*places.get(n + i)?)?,
                                amount: s.amount,
                            })
                        })
                        .collect();
                    edit::slide_points(mesh, &moved)
                }
                _ => edit::slide_points(mesh, slides),
            },
            // Handled above.
            Self::Mirrored { .. } => return skipped,
            Self::WeldMiddle { mirror, distance } => {
                let n = mesh::symmetry::weld_middle(mesh, mirror, *distance);
                Outcome {
                    matched: n,
                    missing: 0,
                    skipped: usize::from(n == 0),
                }
            }
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
fn elements(
    verb: &'static str,
    points: &[VertexId],
    edges: &[EdgeKey],
    faces: &[FaceId],
) -> String {
    if !faces.is_empty() {
        counted(verb, faces.len(), text!("polygon"))
    } else if !edges.is_empty() {
        counted(verb, edges.len(), text!("edge"))
    } else {
        counted(verb, points.len(), text!("point"))
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

/// "Extrude 1 polygon", "Inset 4 polygons", in the language in use:
/// `verb` and `thing` are English texts marked with `text!`.
fn counted(verb: &'static str, n: usize, thing: &'static str) -> String {
    if n == 1 {
        trf!("{} 1 {}", tr(verb), tr(thing))
    } else {
        trf!("{} {} {}s", tr(verb), n, tr(thing))
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

    /// The layer new edits go into (decision 0091): the top one that is
    /// switched on, or `None` if there is none (a new layer is added).
    pub fn recording_layer(&self) -> Option<usize> {
        self.layers.iter().rposition(|l| l.enabled)
    }

    /// A name for a new layer that no layer has: "Edits" for the first,
    /// then "Edits 2", "Edits 3" and so on.
    pub fn next_layer_name(&self) -> String {
        let taken = |name: &str| self.layers.iter().any(|l| l.name == name);
        let base = tr(EDITS_LAYER);
        if !taken(base) {
            return base.to_string();
        }
        (2..)
            .map(|n| trf!("{} {}", base, n))
            .find(|name| !taken(name))
            .unwrap_or_else(|| base.to_string())
    }
}

impl Stack {
    /// Why the edits up to `edit` can't be frozen (decision 0092), or
    /// `None` if they can: one of them is kept editable or switched off,
    /// or a layer they are in is off.
    pub fn freeze_problem(&self, edit: EditId) -> Option<&'static str> {
        let Some((layer, index, _)) = self.entry(edit) else {
            return Some(tr("that edit no longer exists"));
        };
        for (l, each) in self.layers[..=layer].iter().enumerate() {
            if !each.enabled {
                return Some(tr(
                    "a layer with edits to freeze is switched off: switch it on or delete it first",
                ));
            }
            let upto = if l == layer {
                index + 1
            } else {
                each.entries.len()
            };
            // Kept-editable (pinned) edits no longer hold a bake back: the
            // switch is gone (decision 0126), and old projects may have it.
            for e in &each.entries[..upto] {
                if !e.enabled {
                    return Some(tr(
                        "an edit to freeze is switched off: switch it on or delete it first",
                    ));
                }
            }
        }
        None
    }
}

/// How many edits freezing up to `edit` would freeze: every edit in the
/// layers below its layer, and those up to it in its own (0 if it isn't
/// there).
pub fn frozen_count(stack: &Stack, edit: EditId) -> usize {
    stack.entry(edit).map_or(0, |(layer, index, _)| {
        stack.layers[..layer]
            .iter()
            .map(|l| l.entries.len())
            .sum::<usize>()
            + index
            + 1
    })
}

/// A change to one edit layer (decision 0091), for
/// [`crate::Command::ChangeLayer`].
#[derive(Debug, Clone, PartialEq)]
pub enum LayerChange {
    /// Switches the layer on or off. A switched-off layer passes the shape
    /// below it through unchanged.
    SetEnabled(bool),
    /// Gives the layer another name (not empty).
    Rename(String),
    /// Moves the layer one place up (applied later) or down (earlier).
    Move {
        /// Up, towards the top of the list.
        up: bool,
    },
    /// Moves the layer's edits to the end of the layer below, and removes
    /// it. Both layers must be switched on.
    MergeDown,
    /// Removes the layer and its edits.
    Delete,
}

impl LayerChange {
    /// A short description for the Edit menu, in the language in use.
    pub fn describe(&self) -> &'static str {
        tr(match self {
            Self::SetEnabled(true) => text!("Switch layer on"),
            Self::SetEnabled(false) => text!("Switch layer off"),
            Self::Rename(_) => text!("Rename layer"),
            Self::Move { up: true } => text!("Move layer up"),
            Self::Move { up: false } => text!("Move layer down"),
            Self::MergeDown => text!("Merge layer down"),
            Self::Delete => text!("Delete layer"),
        })
    }
}

/// The shape `stack` gives on `base`, worked out from scratch (not
/// cached): for freezing part of a stack (decision 0092). `None` for a
/// kind without a shape.
pub(crate) fn evaluate_once(
    base: &ObjectKind,
    stack: &Stack,
    context: &EvalContext,
) -> Option<Evaluated> {
    StackCache::default().evaluate(ObjectId::from_raw(0), base, stack, context)
}

/// What evaluation can depend on besides the object itself: the project's
/// mending settings (decision 0110). A point in time joins it with
/// animation, without every function that evaluates having to change
/// (DESIGN.md §4.5).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EvalContext {
    /// How edits mend the UVs they break.
    pub mending: crate::MendOptions,
}

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

/// The shape a base gives on its own, or `None` for a kind without one
/// (or a curve that is only a line). `profile` is the curve a sweep takes
/// its cross-section from, when it takes another curve's (decision 0138).
fn base_mesh(base: &ObjectKind, profile: Option<&mesh::curve::Curve>) -> Option<Arc<Mesh>> {
    match base {
        // A curve takes no edits, so its shape needs no names.
        ObjectKind::Curve(c) => c.mesh(profile).map(Arc::new),
        ObjectKind::Group | ObjectKind::Instance(_) => None,
        // Named for the edits on top (decision 0090).
        ObjectKind::Primitive(p) => {
            let mut mesh = p.mesh();
            mesh.name_base();
            Some(Arc::new(mesh))
        }
        // A plain mesh's ids never change: they name its points.
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
    context: &EvalContext,
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
        // The elements it makes are named after it (decision 0090).
        mesh.begin_naming(entry.id.get());
        let applied = entry.operation.apply(&mut mesh, context);
        mesh.end_naming();
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
    context: &EvalContext,
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
    let (mesh, previous) = apply_from(&start, index, &layer.entries, from, &mut warnings, context);
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
    context: &EvalContext,
) -> usize {
    let mut below = out
        .last()
        .map_or_else(|| Arc::clone(input), |l| Arc::clone(&l.mesh));
    let mut applied = 0;
    for (index, layer) in layers.iter().enumerate().skip(from) {
        let (cached, count) = evaluate_layer(&below, index, layer, first.take(), context);
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
    /// The cross-section curve its sweep used, if another curve's.
    profile: Option<mesh::curve::Curve>,
    /// What it was worked out with: a change works it all out again.
    context: EvalContext,
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
        context: &EvalContext,
    ) -> Option<Evaluated> {
        self.evaluate_with(id, base, stack, context, None)
    }

    /// The same, for a curve whose sweep takes its cross-section from
    /// `profile`, another curve (decision 0138): a change to that curve
    /// works the shape out again.
    pub(crate) fn evaluate_with(
        &mut self,
        id: ObjectId,
        base: &ObjectKind,
        stack: &Stack,
        context: &EvalContext,
        profile: Option<&mesh::curve::Curve>,
    ) -> Option<Evaluated> {
        if let Some(entry) = self.entries.get_mut(&id)
            && same_base(&entry.base, base)
            && entry.profile.as_ref() == profile
            && entry.context == *context
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
                context,
            );
            entry.last = Rebuilt::From {
                layer: first_changed,
                operations,
            };
            return Some(entry.result());
        }
        let Some(base_mesh) = base_mesh(base, profile) else {
            // Nothing kept for a kind without a shape.
            self.entries.remove(&id);
            return None;
        };
        let mut layers = Vec::new();
        evaluate_layers(&base_mesh, &stack.layers, 0, None, &mut layers, context);
        let entry = CachedObject {
            base: base.clone(),
            profile: profile.cloned(),
            context: *context,
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
        f.debug_struct("SharedCache").finish_non_exhaustive()
    }
}
