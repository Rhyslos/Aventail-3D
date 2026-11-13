//! A drawing: the pieces on one shape canvas, in order, its mirror line
//! and its reference image.
//!
//! Every change makes a new drawing ([`Drawing::with_piece`] and the
//! rest), checked, so the scene can keep the old one for undo. A change
//! that can't be made says why ([`Problem`]).
//!
//! Pieces have ids that are never reused and names, so later milestones
//! (assemblies with linked copies, the edge randomiser) can point at them.

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

/// One piece of a drawing.
#[derive(Debug, Clone, PartialEq)]
pub struct Piece {
    /// Its id.
    pub id: PieceId,
    /// Its name, shown in the pieces list.
    pub name: String,
    /// Its path.
    pub path: Path,
    /// What it does.
    pub kind: PieceKind,
    /// Whether the mirror line, when on, mirrors it.
    pub mirrored: bool,
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
    /// The id the next piece gets.
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

    /// The paths piece `piece` is drawn as: its own, and its mirror image
    /// when the mirror line is on and mirrors it.
    pub fn drawn_paths(&self, piece: &Piece) -> Vec<Path> {
        let mut out = vec![piece.path.clone()];
        if self.mirror.on && piece.mirrored {
            out.push(piece.path.mirrored(self.mirror.x));
        }
        out
    }

    /// A key for what it gives (pieces and mirror; not the reference
    /// image, which is only a guide): the same drawing gives the same key.
    pub fn key(&self) -> u64 {
        let mut h = DefaultHasher::new();
        self.pieces.len().hash(&mut h);
        for p in &self.pieces {
            (p.id, p.kind, p.mirrored).hash(&mut h);
            p.path.hash_into(&mut h);
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
            .map(|p| 64 + p.name.len() + p.path.knots().len() * 24)
            .sum();
        64 + pieces + self.reference.as_ref().map_or(0, |_| 1024)
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
        };
        check_piece(&piece)?;
        let mut out = self.clone();
        out.pieces.push(piece);
        out.next_id = out.next_id.saturating_add(1);
        out.check_count()?;
        Ok(out)
    }

    /// The same drawing with piece `id`'s path changed to `path`.
    pub fn with_path(&self, id: PieceId, path: Path) -> Result<Self, Problem> {
        self.changing(id, |p| p.path = path)
    }

    /// The same drawing with piece `id` doing `kind` (a line opens its
    /// path; adding or cutting out needs a closed one).
    pub fn with_kind(&self, id: PieceId, kind: PieceKind) -> Result<Self, Problem> {
        self.changing(id, |p| {
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

    /// The same drawing with pieces `ids` moved, turned or sized by
    /// `change`.
    pub fn with_change(&self, ids: &[PieceId], change: &Change) -> Result<Self, Problem> {
        let mut out = self.clone();
        for id in ids {
            let p = out
                .pieces
                .iter_mut()
                .find(|p| p.id == *id)
                .ok_or(Problem::Gone)?;
            p.path = p.path.changed(change);
            check_piece(p)?;
        }
        Ok(out)
    }

    /// The same drawing without pieces `ids`.
    pub fn without(&self, ids: &[PieceId]) -> Result<Self, Problem> {
        if ids.iter().any(|id| self.piece(*id).is_none()) {
            return Err(Problem::Gone);
        }
        let mut out = self.clone();
        out.pieces.retain(|p| !ids.contains(&p.id));
        Ok(out)
    }

    /// The same drawing with piece `id` moved `by` places up (positive:
    /// toward the top, drawn later) or down the order, kept in the list.
    pub fn with_order(&self, id: PieceId, by: i32) -> Result<Self, Problem> {
        let at = self
            .pieces
            .iter()
            .position(|p| p.id == id)
            .ok_or(Problem::Gone)?;
        let to = (at as i64 + i64::from(by)).clamp(0, self.pieces.len() as i64 - 1) as usize;
        let mut out = self.clone();
        let piece = out.pieces.remove(at);
        out.pieces.insert(to, piece);
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
            next_id: self.next_id,
            mirror: self.mirror,
            reference: self.reference.clone(),
        }
    }

    /// Rebuilds a drawing from its parts (from a file, so anything may be
    /// wrong): ids unique and below `next_id`, every piece as
    /// [`Self::with_piece`] would allow, the limits kept, the mirror line
    /// within the texture. Says why when it can't.
    pub fn from_parts(parts: DrawingParts) -> Result<Self, Problem> {
        if parts.pieces.len() > MAX_PIECES {
            return Err(Problem::TooMany);
        }
        let mut seen = std::collections::BTreeSet::new();
        for p in &parts.pieces {
            if p.id.0 == 0 || p.id.0 >= parts.next_id || !seen.insert(p.id) {
                return Err(Problem::Gone);
            }
            check_piece(p)?;
        }
        let out = Self {
            pieces: parts.pieces,
            next_id: parts.next_id,
            mirror: Mirror::default(),
            reference: parts.reference,
        };
        out.check_count()?;
        out.with_mirror(parts.mirror)
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

/// Why `piece` can't be kept, if it can't.
fn check_piece(piece: &Piece) -> Result<(), Problem> {
    if piece.name.trim().is_empty() || piece.name.len() > MAX_NAME {
        return Err(Problem::BadName);
    }
    if piece.path.problem().is_some() {
        return Err(if piece.path.knots().len() > crate::path::MAX_KNOTS {
            Problem::TooMany
        } else {
            Problem::OutOfPlace
        });
    }
    match piece.kind {
        PieceKind::Line if piece.path.knots().len() < 2 => Err(Problem::ShortLine),
        PieceKind::Line => Ok(()),
        _ if !piece.path.is_closed() => Err(Problem::OpenPath),
        _ if !has_room(&piece.path) => Err(Problem::NoInside),
        _ => Ok(()),
    }
}

/// Whether a closed path has room inside: at least two points, not all
/// (handles included) on one line. Sides crossing over don't matter: a
/// figure of eight has room in both loops though its signed area is
/// nothing.
fn has_room(path: &Path) -> bool {
    let points: Vec<glam::Vec2> = path
        .knots()
        .iter()
        .flat_map(|k| [k.at, k.at + k.inn, k.at + k.out])
        .collect();
    let Some(first) = points.first().copied() else {
        return false;
    };
    let Some(far) = points
        .iter()
        .copied()
        .max_by(|a, b| a.distance(first).total_cmp(&b.distance(first)))
    else {
        return false;
    };
    let along = far - first;
    if path.knots().len() < 2 || along.length() < MIN_SPREAD {
        return false;
    }
    let across = along.normalize().perp();
    points
        .iter()
        .any(|p| (*p - first).dot(across).abs() >= MIN_SPREAD)
}

#[cfg(test)]
mod tests;
