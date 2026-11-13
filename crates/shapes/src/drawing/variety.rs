//! Variety (Phase 12 milestone 13, decision 0275): edges, varied copies
//! and veins.
//!
//! - **Edges** ([`Edge`]) belong with a shape, so linked copies share
//!   theirs; [`Drawing::with_edge`] changes every linked copy.
//! - **Varied copies** ([`Vary`], also shared by linked copies): with
//!   "each copy differs" on, every piece is drawn a little bigger or
//!   smaller, turned a little and shifted in colour, by its own seed
//!   ([`Piece::seed`]), and its edge's teeth come out its own. A group
//!   varies as one, by its bottom piece, so a leaf and its veins stay
//!   together.
//! - **The mirror line:** a mirrored piece's mirror image has the same
//!   teeth mirrored, unless copies vary (then it is varied as a copy of
//!   its own). A piece touching the mirror line (half a leaf drawn
//!   against it) varies as a whole leaf: sized about the line, never
//!   turned, and its teeth fade out at the line so the halves meet.
//! - **Veins** ([`Vein`]): a Line piece's groove or ridge in the Height
//!   and darker or lighter line in the Colours (filled by `raster::veins`).
//!
//! All of it is worked out in [`Drawing::drawn_paths`] after a piece is
//! placed, so its shape and place stay as drawn and moved.

use super::{Drawing, Piece, PieceId, Problem};
use crate::edge::{Edge, edged, mix};
use crate::path::{Change, Path};
use glam::{Mat2, Vec2};
use std::ops::RangeInclusive;

/// How much bigger or smaller a varied copy may be (a share: 0.5 is ±50%).
pub const VARY_SIZE: RangeInclusive<f32> = 0.0..=0.5;
/// How far a varied copy may be turned either way, in radians (to 180°).
pub const VARY_ANGLE: RangeInclusive<f32> = 0.0..=std::f32::consts::PI;
/// How much a varied copy's colour may shift (1: lighter or darker by
/// half, tinted by a fifth).
pub const VARY_COLOUR: RangeInclusive<f32> = 0.0..=1.0;
/// How wide a vein is, a share of the texture.
pub const VEIN_WIDTH: RangeInclusive<f32> = 0.001..=0.1;
/// How deep a vein's groove (below 0) or high its ridge (above 0) is, a
/// share of the full height.
pub const VEIN_DEPTH: RangeInclusive<f32> = -1.0..=1.0;
/// How much darker (below 0) or lighter (above 0) a vein is.
pub const VEIN_SHADE: RangeInclusive<f32> = -1.0..=1.0;

/// How a shape's copies vary: shared by linked copies, each varied by its
/// own seed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vary {
    /// "Each copy differs": whether copies vary at all.
    pub on: bool,
    /// How much bigger or smaller ([`VARY_SIZE`]).
    pub size: f32,
    /// How far turned either way ([`VARY_ANGLE`], radians).
    pub angle: f32,
    /// How much the colour shifts ([`VARY_COLOUR`]).
    pub colour: f32,
}

impl Default for Vary {
    fn default() -> Self {
        Self::OFF
    }
}

impl Vary {
    /// A new shape's: off, with ranges that show at once when switched on.
    pub const OFF: Self = Self {
        on: false,
        size: 0.15,
        angle: 0.26,
        colour: 0.15,
    };

    /// Its numbers brought within their ranges, or `None` when one isn't
    /// a number.
    pub fn fitted(self) -> Option<Self> {
        let ok = [self.size, self.angle, self.colour]
            .iter()
            .all(|v| v.is_finite());
        ok.then(|| Self {
            on: self.on,
            size: self.size.clamp(*VARY_SIZE.start(), *VARY_SIZE.end()),
            angle: self.angle.clamp(*VARY_ANGLE.start(), *VARY_ANGLE.end()),
            colour: self.colour.clamp(*VARY_COLOUR.start(), *VARY_COLOUR.end()),
        })
    }
}

/// A Line piece's vein: shared by linked copies.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vein {
    /// How wide ([`VEIN_WIDTH`]).
    pub width: f32,
    /// Groove (below 0) or ridge (above 0) in the Height ([`VEIN_DEPTH`]).
    pub depth: f32,
    /// Darker (below 0) or lighter (above 0) in the Colours
    /// ([`VEIN_SHADE`]).
    pub shade: f32,
}

impl Default for Vein {
    fn default() -> Self {
        Self::STANDARD
    }
}

impl Vein {
    /// A new line's: a gentle groove, a little darker.
    pub const STANDARD: Self = Self {
        width: 0.012,
        depth: -0.3,
        shade: -0.25,
    };
    /// No vein (lines from files before veins).
    pub const NONE: Self = Self {
        width: 0.012,
        depth: 0.0,
        shade: 0.0,
    };

    /// Whether it changes nothing.
    pub fn is_none(&self) -> bool {
        self.depth == 0.0 && self.shade == 0.0
    }

    /// Its numbers brought within their ranges, or `None` when one isn't
    /// a number.
    pub fn fitted(self) -> Option<Self> {
        let ok = [self.width, self.depth, self.shade]
            .iter()
            .all(|v| v.is_finite());
        ok.then(|| Self {
            width: self.width.clamp(*VEIN_WIDTH.start(), *VEIN_WIDTH.end()),
            depth: self.depth.clamp(*VEIN_DEPTH.start(), *VEIN_DEPTH.end()),
            shade: self.shade.clamp(*VEIN_SHADE.start(), *VEIN_SHADE.end()),
        })
    }
}

/// A new piece's seed, from its id (so copies made at different times
/// differ, and older files open with the seeds they would have had).
pub fn first_seed(id: PieceId) -> u32 {
    (mix(0x5EED, u64::from(id.0)) >> 32) as u32
}

/// The seed after `seed`: what the ↻ button rolls to.
pub fn next_seed(seed: u32) -> u32 {
    (mix(u64::from(seed), 0x0011_D1CE) >> 32) as u32
}

/// How one piece is varied where it is drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Variation {
    /// The size and turn, after its place.
    pub change: Change,
    /// The same for its mirror image (after mirroring).
    pub mirror_change: Change,
    /// Its colour times this.
    pub tint: [f32; 3],
    /// Whether it varies at all.
    pub on: bool,
    /// Whether its group (or it) touches the mirror line, mirrored.
    pub touches: bool,
}

impl Variation {
    const NONE: Self = Self {
        change: Change::IDENTITY,
        mirror_change: Change::IDENTITY,
        tint: [1.0; 3],
        on: false,
        touches: false,
    };
}

impl Drawing {
    /// How piece `piece` is varied: by itself, or by its group's bottom
    /// piece (a group varies as one, about its middle).
    pub fn variation(&self, piece: &Piece) -> Variation {
        let unit: Vec<&Piece> = match piece.group {
            Some(g) => self.pieces.iter().filter(|p| p.group == Some(g)).collect(),
            None => vec![piece],
        };
        let lead = unit.first().copied().unwrap_or(piece);
        let mirror = self.mirror;
        let touches = mirror.on
            && unit.iter().any(|p| {
                p.mirrored
                    && p.placed()
                        .bounds()
                        .is_some_and(|(lo, hi)| lo.x <= mirror.x + 1e-4 && hi.x >= mirror.x - 1e-4)
            });
        let Some(vary) = lead.vary.fitted().filter(|v| v.on) else {
            return Variation {
                touches,
                ..Variation::NONE
            };
        };
        let middle = super::assembly::bounds_middle(unit.iter().copied()).unwrap_or(Vec2::ZERO);
        let r = |k: u64| {
            let h = mix(u64::from(lead.seed), k);
            (h >> 40) as f32 / (1u64 << 24) as f32 * 2.0 - 1.0
        };
        let sized = |k: u64| 1.0 + vary.size * r(k);
        let (change, mirror_change) = if touches {
            // A half against the line: sized about the line, never
            // turned, so it still meets its mirror image.
            let about = Vec2::new(mirror.x, middle.y);
            let c = Change::scaled(about, Vec2::splat(sized(1)));
            (c, c)
        } else {
            let mirrored_middle = Vec2::new(2.0 * mirror.x - middle.x, middle.y);
            (
                varied(middle, sized(1), vary.angle * r(2)),
                varied(mirrored_middle, sized(3), vary.angle * r(4)),
            )
        };
        let light = 1.0 + vary.colour * 0.5 * r(5);
        let tint = [6, 7, 8].map(|k| light * (1.0 + vary.colour * 0.2 * r(k)));
        Variation {
            change,
            mirror_change,
            tint,
            on: true,
            touches,
        }
    }

    /// Piece `piece`'s colour where it is drawn: its own, shifted when
    /// copies vary.
    pub fn drawn_colour(&self, piece: &Piece) -> [f32; 3] {
        let tint = self.variation(piece).tint;
        [0, 1, 2].map(|i| (piece.colour[i] * tint[i]).clamp(0.0, 1.0))
    }

    /// The paths piece `piece` is drawn as: its shape where it is placed,
    /// varied and with its edge, and its mirror image when the mirror
    /// line is on and mirrors it. The one place a piece becomes what is
    /// filled, so the pictures and the canvas's outlines agree.
    pub fn drawn_paths(&self, piece: &Piece) -> Vec<Path> {
        let v = self.variation(piece);
        let placed = piece.placed();
        let base = placed.changed(&v.change);
        let edge_line = |path: &Path, stream: u64| {
            if piece.kind == super::PieceKind::Line {
                return path.clone();
            }
            let seed = if v.on {
                (mix(
                    mix(u64::from(piece.edge.seed), u64::from(piece.seed)),
                    stream,
                ) >> 32) as u32
            } else {
                piece.edge.seed
            };
            let fade = v.touches.then_some(self.mirror.x);
            edged(path, &piece.edge, seed, fade)
        };
        let first = edge_line(&base, 0);
        let mut out = Vec::with_capacity(2);
        if self.mirror.on && piece.mirrored {
            let x = self.mirror.x;
            let second = if !v.on {
                first.mirrored(x)
            } else if v.touches {
                edge_line(&base.mirrored(x), 1)
            } else {
                edge_line(&placed.mirrored(x).changed(&v.mirror_change), 1)
            };
            out.push(first);
            out.push(second);
        } else {
            out.push(first);
        }
        out
    }

    /// The same drawing with piece `id`'s edge set to `edge`, for it and
    /// every piece linked to it (the edge belongs with the shape).
    pub fn with_edge(&self, id: PieceId, edge: Edge) -> Result<Self, Problem> {
        let edge = edge.fitted().ok_or(Problem::OutOfPlace)?;
        let link = self.piece(id).ok_or(Problem::Gone)?.link;
        self.changing_linked(link, |p| p.edge = edge)
    }

    /// The same drawing with piece `id`'s copies varying by `vary`, for it
    /// and every piece linked to it.
    pub fn with_vary(&self, id: PieceId, vary: Vary) -> Result<Self, Problem> {
        let vary = vary.fitted().ok_or(Problem::OutOfPlace)?;
        let link = self.piece(id).ok_or(Problem::Gone)?.link;
        self.changing_linked(link, |p| p.vary = vary)
    }

    /// The same drawing with piece `id`'s vein set to `vein`, for it and
    /// every piece linked to it.
    pub fn with_vein(&self, id: PieceId, vein: Vein) -> Result<Self, Problem> {
        let vein = vein.fitted().ok_or(Problem::OutOfPlace)?;
        let link = self.piece(id).ok_or(Problem::Gone)?.link;
        self.changing_linked(link, |p| p.vein = vein)
    }

    /// The same drawing with pieces `ids` each given a new seed (the ↻
    /// button): varied copies come out another way.
    pub fn with_new_seeds(&self, ids: &[PieceId]) -> Result<Self, Problem> {
        let mut out = self.clone();
        for id in ids {
            let p = out
                .pieces
                .iter_mut()
                .find(|p| p.id == *id)
                .ok_or(Problem::Gone)?;
            p.seed = next_seed(p.seed);
        }
        Ok(out)
    }
}

/// Sized by `size` and turned by `angle` about `about`.
fn varied(about: Vec2, size: f32, angle: f32) -> Change {
    Change {
        about,
        matrix: Mat2::from_angle(angle) * size,
        by: Vec2::ZERO,
    }
}

#[cfg(test)]
mod tests;
