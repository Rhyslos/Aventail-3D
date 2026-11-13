//! Leaf sheets (Phase 12 milestone 14, [decision 0276](../../../docs/decisions/0276-foliage-output.md)):
//! a drawing's leaves and twigs laid out on one texture (an atlas), each
//! whole and apart from the others, for alpha cards in a game.
//!
//! **Cards.** Each group is a card, and every piece touching another
//! (overlapping it, or sitting inside it) is on that one's card: a leaf
//! and its veins and bites, a twig and its leaves. A card needs something
//! filled (an Add piece); cut-outs and lines touching nothing are left
//! off the sheet.
//!
//! **Laying out.** The cards are packed by the app's UV packer
//! (`uv::pack`, decision 0101) by the hulls round their outlines (a band
//! stretched round each: quick to pack, never overlapping), as large as they
//! will all go, every one at the same scale (so leaves keep their sizes
//! against each other), `gap` apart and half a gap from the sheet's
//! edges, turned to fit when `turn` is on. Only the finished outlines
//! move: each piece's edge, variation and mirror image are worked out as
//! drawn first ([`Drawing::drawn_paths`]), then the card's move is
//! applied, so the sheet shows exactly what the canvas does, only
//! somewhere else. Nothing crosses the sheet's edges, so nothing wraps.
//! The layout is in shares of the texture, the same at every size.

use crate::drawing::{Drawing, PieceId, PieceKind};
use crate::path::{Change, Path};
use glam::{Mat2, Vec2};
use std::collections::BTreeMap;

/// The widest gap allowed between cards, a share of the sheet.
pub const MAX_GAP: f32 = 0.1;

/// How finely outlines are followed when packing and testing touches, a
/// share of the texture.
const TOLERANCE: f64 = 0.002;

/// How a sheet is laid out.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SheetOptions {
    /// The gap between cards, a share of the sheet (0 to [`MAX_GAP`]).
    pub gap: f32,
    /// Whether cards may be turned to fit better.
    pub turn: bool,
}

impl Default for SheetOptions {
    fn default() -> Self {
        Self {
            gap: 0.02,
            turn: true,
        }
    }
}

/// A drawing's cards laid out on a sheet: where each piece on a card
/// goes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Sheet {
    /// Each piece on a card, and the move taking its drawn outlines to
    /// the sheet.
    moves: BTreeMap<PieceId, Change>,
    /// The pieces on each card.
    cards: Vec<Vec<PieceId>>,
}

impl Sheet {
    /// `drawing`'s cards laid out as `options` say. Empty for a drawing
    /// with nothing filled.
    pub fn of(drawing: &Drawing, options: SheetOptions) -> Self {
        let cards = cards(drawing);
        let pieces: Vec<uv::pack::Piece> = cards
            .iter()
            .map(|card| uv::pack::Piece {
                // Every piece on the card, so cut-outs and lines reaching
                // past the filled part stay on the sheet too.
                polygons: card
                    .iter()
                    .filter_map(|id| drawing.piece(*id))
                    .flat_map(|p| drawing.drawn_paths(p))
                    .map(|path| hull(&path.flattened(TOLERANCE)))
                    .filter(|points| points.len() >= 2)
                    .collect(),
            })
            .collect();
        let gap = if options.gap.is_finite() {
            options.gap.clamp(0.0, MAX_GAP)
        } else {
            SheetOptions::default().gap
        };
        let pack = uv::pack::PackOptions {
            padding: gap,
            rotate: options.turn,
            stack: false,
            cells: 256,
            region: None,
            tiles: 1,
        };
        let Some(placed) = uv::pack::pack(&pieces, &pack) else {
            return Self::default();
        };
        let mut moves = BTreeMap::new();
        for (card, t) in cards.iter().zip(placed) {
            let change = Change {
                about: Vec2::ZERO,
                matrix: Mat2::from_cols(t.matrix2.x_axis, t.matrix2.y_axis),
                by: t.translation,
            };
            if !change.is_usable() {
                continue;
            }
            for id in card {
                moves.insert(*id, change);
            }
        }
        Self { moves, cards }
    }

    /// The move taking piece `id`'s drawn outlines to the sheet; `None`
    /// for a piece left off it.
    pub fn change(&self, id: PieceId) -> Option<&Change> {
        self.moves.get(&id)
    }

    /// The pieces on each card, in the order found.
    pub fn cards(&self) -> &[Vec<PieceId>] {
        &self.cards
    }

    /// Whether nothing is on the sheet.
    pub fn is_empty(&self) -> bool {
        self.moves.is_empty()
    }

    /// The paths piece `piece` of `drawing` is drawn as on the sheet: its
    /// drawn outlines moved with its card, or none when it is off the
    /// sheet.
    pub fn drawn_paths(&self, drawing: &Drawing, piece: &crate::Piece) -> Vec<Path> {
        match self.change(piece.id) {
            Some(change) => drawing
                .drawn_paths(piece)
                .iter()
                .map(|p| p.changed(change))
                .collect(),
            None => Vec::new(),
        }
    }

    /// How much piece `id`'s card is sized (1 off the sheet): lines' widths
    /// follow it.
    pub fn scale_of(&self, id: PieceId) -> f32 {
        self.change(id)
            .map_or(1.0, |c| c.matrix.determinant().abs().sqrt())
    }
}

/// The cards of `drawing`: each group, and every piece touching another,
/// together; only cards with a filled piece, in the order of their first
/// piece.
pub fn cards(drawing: &Drawing) -> Vec<Vec<PieceId>> {
    let pieces = drawing.pieces();
    let outlines: Vec<Outline> = pieces
        .iter()
        .map(|p| Outline::of(&drawing.drawn_paths(p), p.kind != PieceKind::Line))
        .collect();
    let mut joined = Joins::new(pieces.len());
    for i in 0..pieces.len() {
        for j in i + 1..pieces.len() {
            let same_group = pieces[i].group.is_some() && pieces[i].group == pieces[j].group;
            if same_group || outlines[i].touches(&outlines[j]) {
                joined.join(i, j);
            }
        }
    }
    let mut by_root: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in 0..pieces.len() {
        by_root.entry(joined.root(i)).or_default().push(i);
    }
    let mut out: Vec<Vec<usize>> = by_root
        .into_values()
        .filter(|card| card.iter().any(|i| pieces[*i].kind == PieceKind::Add))
        .collect();
    out.sort_by_key(|card| card[0]);
    out.into_iter()
        .map(|card| card.into_iter().map(|i| pieces[i].id).collect())
        .collect()
}

/// Which pieces are joined into one card (union by root).
struct Joins(Vec<usize>);

impl Joins {
    fn new(n: usize) -> Self {
        Self((0..n).collect())
    }

    fn root(&mut self, mut i: usize) -> usize {
        while self.0[i] != i {
            self.0[i] = self.0[self.0[i]];
            i = self.0[i];
        }
        i
    }

    fn join(&mut self, a: usize, b: usize) {
        let (a, b) = (self.root(a), self.root(b));
        if a != b {
            self.0[a.max(b)] = a.min(b);
        }
    }
}

/// A piece's drawn outlines as points close together, for telling
/// whether two pieces touch.
struct Outline {
    /// Each outline's points, no further apart than [`TOLERANCE`] allows.
    rings: Vec<Vec<Vec2>>,
    /// Whether the outlines enclose anything (not a line).
    closed: bool,
    /// The box round them all.
    bounds: Option<(Vec2, Vec2)>,
}

impl Outline {
    fn of(paths: &[Path], closed: bool) -> Self {
        let step = TOLERANCE as f32 * 4.0;
        let rings: Vec<Vec<Vec2>> = paths
            .iter()
            .map(|p| {
                let points = p.flattened(TOLERANCE);
                let mut dense = Vec::with_capacity(points.len());
                let count = if closed {
                    points.len()
                } else {
                    points.len().saturating_sub(1)
                };
                for k in 0..count {
                    let (a, b) = (points[k], points[(k + 1) % points.len()]);
                    let n = ((a.distance(b) / step).ceil() as usize).clamp(1, 512);
                    dense.extend((0..n).map(|i| a.lerp(b, i as f32 / n as f32)));
                }
                if !closed && let Some(last) = points.last() {
                    dense.push(*last);
                }
                dense.retain(|v| v.is_finite());
                dense
            })
            .filter(|r| !r.is_empty())
            .collect();
        let bounds = rings
            .iter()
            .flatten()
            .fold(None, |b: Option<(Vec2, Vec2)>, p| {
                Some(b.map_or((*p, *p), |(lo, hi)| (lo.min(*p), hi.max(*p))))
            });
        Self {
            rings,
            closed,
            bounds,
        }
    }

    /// Whether `p` is inside one of its outlines (even-odd per outline).
    fn contains(&self, p: Vec2) -> bool {
        self.closed && self.rings.iter().any(|ring| inside(ring, p))
    }

    /// Whether the two touch: their boxes meet, and a point of one lies
    /// inside the other (outlines are followed in small steps, so two
    /// crossing pieces always have one). Outlines only grazing each other
    /// are apart.
    fn touches(&self, other: &Outline) -> bool {
        let (Some((a0, a1)), Some((b0, b1))) = (self.bounds, other.bounds) else {
            return false;
        };
        let near = TOLERANCE as f32 * 2.0;
        if a0.x > b1.x + near || b0.x > a1.x + near || a0.y > b1.y + near || b0.y > a1.y + near {
            return false;
        }
        let points_in = |a: &Outline, b: &Outline| a.rings.iter().flatten().any(|p| b.contains(*p));
        points_in(self, other) || points_in(other, self)
    }
}

/// The most corners an outline is packed by: the packer tests every
/// cell against every corner, so a toothed outline's thousands would be
/// slow.
const PACKED_CORNERS: usize = 48;

/// The convex hull round `points` (finite ones only), at most
/// [`PACKED_CORNERS`] corners: what a card is packed by. Covering more
/// than the outline (its dents) costs a little room but never lets cards
/// overlap; a hull with more corners keeps evenly spaced ones (a hair
/// inside the round parts, which the gap covers).
fn hull(points: &[Vec2]) -> Vec<Vec2> {
    let mut sorted: Vec<Vec2> = points.iter().copied().filter(|p| p.is_finite()).collect();
    sorted.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    sorted.dedup();
    if sorted.len() < 3 {
        return sorted;
    }
    let turn = |o: Vec2, a: Vec2, b: Vec2| (a - o).perp_dot(b - o);
    let mut lower: Vec<Vec2> = Vec::new();
    for p in &sorted {
        while lower.len() >= 2 && turn(lower[lower.len() - 2], lower[lower.len() - 1], *p) <= 0.0 {
            lower.pop();
        }
        lower.push(*p);
    }
    let mut upper: Vec<Vec2> = Vec::new();
    for p in sorted.iter().rev() {
        while upper.len() >= 2 && turn(upper[upper.len() - 2], upper[upper.len() - 1], *p) <= 0.0 {
            upper.pop();
        }
        upper.push(*p);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    if lower.len() > PACKED_CORNERS {
        let n = lower.len();
        lower = (0..PACKED_CORNERS)
            .map(|i| lower[i * n / PACKED_CORNERS])
            .collect();
    }
    lower
}

/// Whether `p` is inside the closed `ring` (even-odd).
fn inside(ring: &[Vec2], p: Vec2) -> bool {
    let mut odd = false;
    for k in 0..ring.len() {
        let (a, b) = (ring[k], ring[(k + 1) % ring.len()]);
        if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x) {
            odd = !odd;
        }
    }
    odd
}

#[cfg(test)]
mod tests;
