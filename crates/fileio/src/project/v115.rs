//! Version 115 of the project file ([decision 0315](../../../../docs/decisions/0315-detail-levels.md)):
//! version 114's, with each Sculpt layer's detail levels, stored with its
//! sculpt ([`super::v112::SculptEditV112::levels`]): each level above the
//! shape it was sculpted on (the structure it was made for, where its
//! points are with nothing sculpted and the chunks of points that moved,
//! packed as version 112 packs them), the level shown, the level exports
//! and bakes take, and the level the mask is on. Files before version 115
//! hold no levels; an older app would open a project without them and
//! lose them on saving, so the version goes up.
//!
//! Reading checks the count of levels and each level's points before
//! unpacking anything, and refuses a level shown or masked that isn't
//! there.

use super::v1::Problem;
use super::v107::PackedBytes;
use super::v112::{ChunkV112, chunks_of, moves_from, pack, unpack};
use crate::limits::Bounded;
use sculpt::levels::{ExportLevel, MAX_LEVELS};
use sculpt::{MAX_POINTS, Sculpt};
use serde::{Deserialize, Serialize};
use words::tr;

/// One detail level above the shape a sculpt was made on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct LevelV115 {
    /// The structure of the shape coming into the layer that it was made
    /// for.
    pub(crate) structure: u64,
    /// How many point slots.
    pub(crate) points: u64,
    /// Where its points are with nothing sculpted, packed.
    pub(crate) places: PackedBytes,
    /// The chunks that moved.
    pub(crate) chunks: Bounded<ChunkV112>,
}

/// Which level an export or a bake takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub(crate) enum ExportV115 {
    /// The level shown.
    #[default]
    Shown,
    /// The finest.
    Finest,
    /// This level.
    Level(u8),
}

/// A sculpt's detail levels.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct LevelsV115 {
    /// The levels above the shape it was sculpted on, coarsest first.
    pub(crate) above: Bounded<LevelV115>,
    /// The level shown (0: the shape it was sculpted on).
    pub(crate) shown: u8,
    /// The level exports and bakes take.
    pub(crate) export: ExportV115,
    /// The level the mask has a value for each point of.
    pub(crate) mask_level: u8,
}

impl Default for LevelsV115 {
    fn default() -> Self {
        Self {
            above: Bounded::new(),
            shown: 0,
            export: ExportV115::Shown,
            mask_level: 0,
        }
    }
}

impl LevelsV115 {
    /// `sculpt`'s levels to store.
    pub(crate) fn of(sculpt: &Sculpt) -> Self {
        let above = (1..sculpt.level_count())
            .filter_map(|k| {
                let moves = sculpt.level_moves(k)?;
                Some(LevelV115 {
                    structure: sculpt.level_structure(k)?,
                    points: moves.len() as u64,
                    places: pack(sculpt.level_places(k)?),
                    chunks: chunks_of(moves),
                })
            })
            .collect();
        Self {
            above,
            shown: sculpt.shown() as u8,
            export: match sculpt.export() {
                ExportLevel::Shown => ExportV115::Shown,
                ExportLevel::Finest => ExportV115::Finest,
                ExportLevel::Level(level) => ExportV115::Level(level),
            },
            mask_level: sculpt.mask_level() as u8,
        }
    }

    /// `sculpt` (the shape it was sculpted on, read already) with the
    /// stored levels, checked: too many levels, a level with too many
    /// points or fewer than the one below, chunks that don't fit it, or a
    /// level shown, exported or masked that isn't there, is damage.
    pub(crate) fn onto(self, mut sculpt: Sculpt) -> Result<Sculpt, Problem> {
        let damaged = || -> Problem { tr("a Sculpt layer's detail levels are damaged").into() };
        let top = self.above.len();
        if top > MAX_LEVELS
            || usize::from(self.shown) > top
            || usize::from(self.mask_level) > top
            || matches!(self.export, ExportV115::Level(level) if usize::from(level) > top)
        {
            return Err(damaged());
        }
        for level in self.above.0 {
            let len = usize::try_from(level.points).map_err(|_| damaged())?;
            if len > MAX_POINTS {
                return Err(damaged());
            }
            let places = unpack(&level.places, len)?;
            let moves = moves_from(len, level.chunks)?;
            sculpt = sculpt
                .with_level(level.structure, places, moves)
                .ok_or_else(damaged)?;
        }
        let export = match self.export {
            ExportV115::Shown => ExportLevel::Shown,
            ExportV115::Finest => ExportLevel::Finest,
            ExportV115::Level(level) => ExportLevel::Level(level),
        };
        let sculpt = sculpt.with_export(export);
        if self.shown == 0 {
            return Ok(sculpt);
        }
        sculpt.showing(usize::from(self.shown)).ok_or_else(damaged)
    }
}
