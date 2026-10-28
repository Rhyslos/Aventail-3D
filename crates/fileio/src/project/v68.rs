//! Version 68 of the project file ([decision 0183](../../../../docs/decisions/0183-colour.md)):
//! version 67's, with the project's colour palettes
//! (`v64::PaintV64::palettes`, stored as [`PaletteV68`]: a name and its
//! colours). Files before version 68 hold none.

use super::v1::Problem;
use crate::limits::Bounded;
use paint::PaintStore;
use paint::palette::Palette;
use serde::{Deserialize, Serialize};
use words::tr;

/// A palette as stored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct PaletteV68 {
    /// Its name.
    pub(crate) name: String,
    /// Its colours (sRGB).
    pub(crate) colours: Bounded<[u8; 3]>,
}

/// The palettes to store: `None` when there are none.
pub(crate) fn palettes_of(palettes: &[Palette]) -> Option<Bounded<PaletteV68>> {
    (!palettes.is_empty()).then(|| {
        palettes
            .iter()
            .map(|p| PaletteV68 {
                name: p.name.clone(),
                colours: p.colours.iter().copied().collect(),
            })
            .collect()
    })
}

/// The stored palettes, checked: too many, or too many colours or too
/// long a name, is damage.
pub(crate) fn palettes_from(stored: Bounded<PaletteV68>) -> Result<Vec<Palette>, Problem> {
    let palettes: Vec<Palette> = stored
        .into_iter()
        .map(|p| Palette {
            name: p.name,
            colours: p.colours.into_iter().collect(),
        })
        .collect();
    if !PaintStore::palettes_are_valid(&palettes) {
        return Err(tr("its colour palettes are damaged").into());
    }
    Ok(palettes)
}
