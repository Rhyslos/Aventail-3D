//! A drawing: the pieces on one shape canvas, in order, its mirror line
//! and its reference image.
//!
//! Every change makes a new drawing ([`Drawing::with_piece`] and the
//! rest), checked, so the scene can keep the old one for undo. A change
//! that can't be made says why ([`Problem`]).
//!
//! Pieces have ids that are never reused and names. Each piece keeps its
//! own shape (`path`) apart from where it is placed (`place`: moved,
//! turned, sized, flipped), so **linked copies** (milestone 12) can share
//! one shape while each sits in its own place: change the shape of one
//! and every copy linked to it changes. Pieces can be gathered into
//! **groups**, which keep their pieces together in the order. Copies,
//! links, groups and the order are in `assembly`.

mod assembly;
mod checks;
mod variety;

use checks::check_piece;

pub use assembly::{Group, GroupId, LinkId, PieceCopies};
pub use variety::{
    VARY_ANGLE, VARY_COLOUR, VARY_SIZE, VEIN_DEPTH, VEIN_SHADE, VEIN_WIDTH, Variation, Vary, Vein,
    first_seed, next_seed,
};

use crate::edge::Edge;
use crate::path::{Change, Path};
use crate::reference::Reference;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;
use words::text;

/// The most pieces a drawing holds.
pub const MAX_PIECES: usize = 256;
/// The most points in all of a drawing's pieces together.
pub const MAX_ALL_KNOTS: usize = 16_384;
/// The longest piece name, in bytes.
pub const MAX_NAME: usize = 100;
/// How far off one line (a share of the texture) a shape's points must
/// reach to have room inside.
const MIN_SPREAD: f32 = 1e-5;

/// A piece's id in its drawing: never reused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PieceId(pub u32);

/// What a piece does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PieceKind {
    /// Adds its inside to the shape.
    Add,
    /// Cuts its inside out of the pieces under it (a cut-out).
    Cut,
    /// A line: drawn in the outline, never filled (open paths are lines).
    Line,
}

impl PieceKind {
    /// Its plain name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Add => text!("Add"),
            Self::Cut => text!("Cut out"),
            Self::Line => text!("Line"),
        }
    }
}

/// A new piece's colour: white, so the Colours output matches the Mask
/// until colours are chosen.
pub const STANDARD_COLOUR: [f32; 3] = [1.0, 1.0, 1.0];

/// One piece of a drawing.
#[derive(Debug, Clone, PartialEq)]
pub struct Piece {
    /// Its id.
    pub id: PieceId,
    /// Its name, shown in the pieces list.
    pub name: String,
    /// Its shape, before it is placed (shared by every piece linked to
    /// it). [`Piece::placed`] is where it is on the canvas.
    pub path: Path,
    /// What it does (shared by every piece linked to it).
    pub kind: PieceKind,
    /// Whether the mirror line, when on, mirrors it.
    pub mirrored: bool,
    /// Where it is placed: its shape moved, turned, sized or flipped.
    pub place: Change,
    /// Its colour (sRGB, 0 to 1), in the Shape box's Colours output.
    pub colour: [f32; 3],
    /// How high it stands, 0 to 1, in the Height output.
    pub height: f32,
    /// The pieces sharing its shape (linked copies) have the same link.
    pub link: LinkId,
    /// The group it is in, if any.
    pub group: Option<GroupId>,
    /// Its own seed: how it varies when its copies vary (milestone 13).
    pub seed: u32,
    /// Its edge (shared by every piece linked to it).
    pub edge: Edge,
    /// How its copies vary (shared by every piece linked to it).
    pub vary: Vary,
    /// Its vein, when it is a Line (shared by every piece linked to it).
    pub vein: Vein,
}

impl Piece {
    /// Its path where it is on the canvas (its shape placed).
    pub fn placed(&self) -> Path {
        self.path.changed(&self.place)
    }

    /// Whether it shares its shape with `other`: what linked copies have
    /// in common (the shape, what it does, its edge, how copies vary and
    /// its vein).
    pub fn same_shape(&self, other: &Piece) -> bool {
        self.path == other.path
            && self.kind == other.kind
            && self.edge == other.edge
            && self.vary == other.vary
            && self.vein == other.vein
    }
}

/// The mirror line: an upright line across the canvas. While it is on,
/// every piece (unless told not to) is drawn on both sides of it, so a
/// leaf drawn on one side comes out whole.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mirror {
    /// Whether it mirrors.
    pub on: bool,
    /// Where it is across the texture (0 to 1).
    pub x: f32,
}

impl Default for Mirror {
    fn default() -> Self {
        Self { on: false, x: 0.5 }
    }
}

/// Why a change to a drawing can't be made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Problem {
    /// A path that isn't closed was to be filled or cut out.
    OpenPath,
    /// A shape with no room inside (its points on one line, or fewer than
    /// two).
    NoInside,
    /// A line needs two points.
    ShortLine,
    /// Too many pieces, or points in all.
    TooMany,
    /// A point or handle not a real number, or too far out.
    OutOfPlace,
    /// Sized down so far it would vanish.
    Squashed,
    /// Linked copies that don't share one shape (a damaged file).
    BadLink,
    /// A group that isn't there or whose pieces aren't together.
    BadGroup,
    /// A name that is empty or too long.
    BadName,
    /// The piece isn't there any more.
    Gone,
}

impl Problem {
    /// Why, in plain words (an English key for the language layer).
    pub fn describe(self) -> &'static str {
        match self {
            Self::OpenPath => text!(
                "An open path can't be filled or cut out: it has no inside until its last \
                 point joins its first."
            ),
            Self::NoInside => text!(
                "This shape has no room inside: its points are on one line, or there are too \
                 few of them."
            ),
            Self::ShortLine => text!("A line needs at least two points."),
            Self::TooMany => text!("The canvas can't hold more pieces or points."),
            Self::OutOfPlace => text!("That's too far from the texture."),
            Self::Squashed => text!(
                "That would size the piece down to nothing. Size it up a little first, or \
                 remove it."
            ),
            Self::BadLink => text!("Linked copies must share one shape."),
            Self::BadGroup => {
                text!("A group's pieces must be together in the order, in a group that is there.")
            }
            Self::BadName => text!("A piece needs a name of up to 100 letters."),
            Self::Gone => text!("That piece isn't on the canvas any more."),
        }
    }
}

/// The pieces on one shape canvas, in order (later pieces over earlier
/// ones), the mirror line and the reference image.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Drawing {
    pieces: Vec<Piece>,
    groups: Vec<Group>,
    next_id: u32,
    mirror: Mirror,
    reference: Option<Arc<Reference>>,
}

/// A drawing as plain parts, to save or to rebuild one from a file
/// ([`Drawing::from_parts`] checks everything).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DrawingParts {
    /// The pieces, in order.
    pub pieces: Vec<Piece>,
    /// The groups, each with at least one piece.
    pub groups: Vec<Group>,
    /// The id the next piece (group, link) gets.
    pub next_id: u32,
    /// The mirror line.
    pub mirror: Mirror,
    /// The reference image.
    pub reference: Option<Arc<Reference>>,
}

impl Drawing {
    /// An empty drawing.
    pub fn new() -> Self {
        Self {
            next_id: 1,
            ..Self::default()
        }
    }

    /// Its pieces, in order (the last on top).
    pub fn pieces(&self) -> &[Piece] {
        &self.pieces
    }

    /// Piece `id`.
    pub fn piece(&self, id: PieceId) -> Option<&Piece> {
        self.pieces.iter().find(|p| p.id == id)
    }

    /// The id the next piece gets.
    pub fn next_id(&self) -> PieceId {
        PieceId(self.next_id)
    }

    /// The mirror line.
    pub fn mirror(&self) -> Mirror {
        self.mirror
    }

    /// The reference image, if any.
    pub fn reference(&self) -> Option<&Arc<Reference>> {
        self.reference.as_ref()
    }

    /// Whether it has no pieces.
    pub fn is_empty(&self) -> bool {
        self.pieces.is_empty()
    }

    /// A key for what it gives (pieces and mirror; not the reference
    /// image, which is only a guide): the same drawing gives the same key.
    pub fn key(&self) -> u64 {
        let mut h = DefaultHasher::new();
        self.pieces.len().hash(&mut h);
        for p in &self.pieces {
            (p.id, p.kind, p.mirrored).hash(&mut h);
            p.path.hash_into(&mut h);
            let c = &p.place;
            for v in [c.about, c.matrix.x_axis, c.matrix.y_axis, c.by] {
                (v.x.to_bits(), v.y.to_bits()).hash(&mut h);
            }
            (p.colour.map(f32::to_bits), p.height.to_bits()).hash(&mut h);
            let e = &p.edge;
            (e.kind, e.amount.to_bits(), e.size.to_bits(), e.seed, p.seed).hash(&mut h);
            let v = &p.vary;
            (
                v.on,
                v.size.to_bits(),
                v.angle.to_bits(),
                v.colour.to_bits(),
            )
                .hash(&mut h);
            let w = &p.vein;
            (w.width.to_bits(), w.depth.to_bits(), w.shade.to_bits()).hash(&mut h);
        }
        (self.mirror.on, self.mirror.x.to_bits()).hash(&mut h);
        h.finish()
    }

    /// Roughly how much memory it takes, in bytes (for the undo budget).
    /// The reference image is shared between the steps that keep it, so
    /// it counts only a little.
    pub fn memory_size(&self) -> usize {
        let pieces: usize = self
            .pieces
            .iter()
            .map(|p| 128 + p.name.len() + p.path.knots().len() * 24)
            .sum();
        let groups: usize = self.groups.iter().map(|g| 16 + g.name.len()).sum();
        64 + pieces + groups + self.reference.as_ref().map_or(0, |_| 1024)
    }

    /// The same drawing with a piece added on top, named `name`, made of
    /// `path` doing `kind` (mirrored when the mirror line is on); its id
    /// is [`Self::next_id`]. An open path can only be a line, and a line
    /// is always open.
    pub fn with_piece(&self, name: &str, path: Path, kind: PieceKind) -> Result<Self, Problem> {
        if self.pieces.len() >= MAX_PIECES {
            return Err(Problem::TooMany);
        }
        let piece = Piece {
            id: PieceId(self.next_id),
            name: name.to_string(),
            path,
            kind,
            mirrored: true,
            place: Change::IDENTITY,
            colour: STANDARD_COLOUR,
            height: 1.0,
            link: LinkId(self.next_id),
            group: None,
            seed: first_seed(PieceId(self.next_id)),
            edge: Edge::SMOOTH,
            vary: Vary::OFF,
            vein: Vein::STANDARD,
        };
        check_piece(&piece)?;
        let mut out = self.clone();
        out.pieces.push(piece);
        out.next_id = out.next_id.saturating_add(1);
        out.check_count()?;
        Ok(out)
    }

    /// The same drawing with piece `id`'s path on the canvas (where it is
    /// placed) changed to `path`. Its shape changes to match, and so does
    /// every piece linked to it, each staying where it is placed.
    pub fn with_path(&self, id: PieceId, path: Path) -> Result<Self, Problem> {
        let piece = self.piece(id).ok_or(Problem::Gone)?;
        let back = piece.place.inverse().ok_or(Problem::Squashed)?;
        self.with_shape(id, path.changed(&back))
    }

    /// The same drawing with piece `id`'s shape (before it is placed)
    /// changed to `shape`, for it and every piece linked to it.
    pub fn with_shape(&self, id: PieceId, shape: Path) -> Result<Self, Problem> {
        let link = self.piece(id).ok_or(Problem::Gone)?.link;
        self.changing_linked(link, |p| p.path = shape.clone())
    }

    /// The same drawing with piece `id` doing `kind` (a line opens its
    /// path; adding or cutting out needs a closed one), and every piece
    /// linked to it.
    pub fn with_kind(&self, id: PieceId, kind: PieceKind) -> Result<Self, Problem> {
        let link = self.piece(id).ok_or(Problem::Gone)?.link;
        self.changing_linked(link, |p| {
            p.kind = kind;
            if kind == PieceKind::Line {
                p.path = Path::new(p.path.knots().to_vec(), false);
            }
        })
    }

    /// The same drawing with piece `id` named `name`.
    pub fn with_name(&self, id: PieceId, name: &str) -> Result<Self, Problem> {
        self.changing(id, |p| p.name = name.to_string())
    }

    /// The same drawing with piece `id` mirrored or not.
    pub fn with_mirrored(&self, id: PieceId, mirrored: bool) -> Result<Self, Problem> {
        self.changing(id, |p| p.mirrored = mirrored)
    }

    /// The same drawing with pieces `ids` moved, turned, sized or
    /// flipped by `change`: their places change, not their shapes, so
    /// linked copies keep sharing one shape.
    pub fn with_change(&self, ids: &[PieceId], change: &Change) -> Result<Self, Problem> {
        let mut out = self.clone();
        for id in ids {
            let p = out
                .pieces
                .iter_mut()
                .find(|p| p.id == *id)
                .ok_or(Problem::Gone)?;
            p.place = p.place.then(change);
            check_piece(p)?;
        }
        Ok(out)
    }

    /// The same drawing without pieces `ids` (a group left with none goes
    /// too).
    pub fn without(&self, ids: &[PieceId]) -> Result<Self, Problem> {
        if ids.iter().any(|id| self.piece(*id).is_none()) {
            return Err(Problem::Gone);
        }
        let mut out = self.clone();
        out.pieces.retain(|p| !ids.contains(&p.id));
        out.drop_empty_groups();
        Ok(out)
    }

    /// The same drawing with the mirror line `mirror` (its place kept
    /// across the texture, 0 to 1).
    pub fn with_mirror(&self, mirror: Mirror) -> Result<Self, Problem> {
        if !mirror.x.is_finite() {
            return Err(Problem::OutOfPlace);
        }
        let mut out = self.clone();
        out.mirror = Mirror {
            on: mirror.on,
            x: mirror.x.clamp(0.0, 1.0),
        };
        Ok(out)
    }

    /// The same drawing with reference image `reference` (or none).
    pub fn with_reference(&self, reference: Option<Arc<Reference>>) -> Self {
        let mut out = self.clone();
        out.reference = reference;
        out
    }

    /// Its plain parts, to save.
    pub fn to_parts(&self) -> DrawingParts {
        DrawingParts {
            pieces: self.pieces.clone(),
            groups: self.groups.clone(),
            next_id: self.next_id,
            mirror: self.mirror,
            reference: self.reference.clone(),
        }
    }

    /// Rebuilds a drawing from its parts (from a file, so anything may be
    /// wrong): ids unique and below `next_id`, every piece as
    /// [`Self::with_piece`] would allow, linked pieces sharing one shape,
    /// each group's pieces together, the limits kept, the mirror line
    /// within the texture; colours, heights, edges, varying and veins are
    /// brought into range.
    /// Says why when it can't.
    pub fn from_parts(parts: DrawingParts) -> Result<Self, Problem> {
        if parts.pieces.len() > MAX_PIECES || parts.groups.len() > MAX_PIECES {
            return Err(Problem::TooMany);
        }
        let mut seen = std::collections::BTreeSet::new();
        let mut pieces = parts.pieces;
        for p in &mut pieces {
            if p.id.0 == 0 || p.id.0 >= parts.next_id || !seen.insert(p.id) {
                return Err(Problem::Gone);
            }
            if p.link.0 == 0 || p.link.0 >= parts.next_id {
                return Err(Problem::BadLink);
            }
            if !p.colour.iter().all(|c| c.is_finite()) || !p.height.is_finite() {
                return Err(Problem::OutOfPlace);
            }
            p.colour = p.colour.map(|c| c.clamp(0.0, 1.0));
            p.height = p.height.clamp(0.0, 1.0);
            p.edge = p.edge.fitted().ok_or(Problem::OutOfPlace)?;
            p.vary = p.vary.fitted().ok_or(Problem::OutOfPlace)?;
            p.vein = p.vein.fitted().ok_or(Problem::OutOfPlace)?;
            check_piece(p)?;
        }
        let mut out = Self {
            pieces,
            groups: parts.groups,
            next_id: parts.next_id,
            mirror: Mirror::default(),
            reference: parts.reference,
        };
        out.drop_empty_groups();
        out.check_links()?;
        out.check_groups()?;
        out.check_count()?;
        out.with_mirror(parts.mirror)
    }

    /// The same drawing with every piece of link `link` changed by
    /// `change`, checked.
    fn changing_linked(&self, link: LinkId, change: impl Fn(&mut Piece)) -> Result<Self, Problem> {
        let mut out = self.clone();
        for p in out.pieces.iter_mut().filter(|p| p.link == link) {
            change(p);
            check_piece(p)?;
        }
        out.check_count()?;
        Ok(out)
    }

    /// The same drawing with piece `id` changed by `change`, checked.
    fn changing(&self, id: PieceId, change: impl FnOnce(&mut Piece)) -> Result<Self, Problem> {
        let mut out = self.clone();
        let p = out
            .pieces
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or(Problem::Gone)?;
        change(p);
        check_piece(p)?;
        out.check_count()?;
        Ok(out)
    }

    /// Whether its points together are within [`MAX_ALL_KNOTS`].
    fn check_count(&self) -> Result<(), Problem> {
        let all: usize = self.pieces.iter().map(|p| p.path.knots().len()).sum();
        if all > MAX_ALL_KNOTS {
            Err(Problem::TooMany)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests;
