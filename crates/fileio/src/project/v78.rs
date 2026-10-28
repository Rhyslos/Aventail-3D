//! Version 78 of the project file ([decision 0207](../../../../docs/decisions/0207-realism-channels.md)):
//! version 77's, with the realism channels: whether a texture set uses
//! them (`v64::SetV64::realism`) and each layer's roughness, metal and
//! glow (`v64::LayerV64::channels`, stored as [`ChannelsV78`]). Files
//! before version 78 hold none.

use super::v1::Problem;
use super::v64::{TileV64, image_from, tiles_of};
use crate::limits::Bounded;
use paint::{ChannelPart, Channels, Content};
use serde::{Deserialize, Serialize};
use words::tr;

/// A layer's realism channels as stored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ChannelsV78 {
    #[serde(default)]
    pub(crate) roughness: Option<PartV78>,
    #[serde(default)]
    pub(crate) metal: Option<PartV78>,
    #[serde(default)]
    pub(crate) glow: Option<PartV78>,
    /// Its height (version 81).
    #[serde(default)]
    pub(crate) height: Option<PartV78>,
}

/// One channel's part: painted tiles, or one value for a fill layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct PartV78 {
    pub(crate) tiles: Bounded<TileV64>,
    #[serde(default)]
    pub(crate) fill: Option<[u8; 4]>,
}

fn part_of(part: &ChannelPart) -> PartV78 {
    match part {
        ChannelPart::Paint(image) => PartV78 {
            tiles: tiles_of(image),
            fill: None,
        },
        ChannelPart::Fill(value) => PartV78 {
            tiles: Bounded::new(),
            fill: Some(*value),
        },
    }
}

/// A layer's channels as stored, when it changes any.
pub(crate) fn channels_of(channels: &Channels) -> Option<ChannelsV78> {
    (!channels.is_empty()).then(|| ChannelsV78 {
        roughness: channels.roughness.as_ref().map(part_of),
        metal: channels.metal.as_ref().map(part_of),
        glow: channels.glow.as_ref().map(part_of),
        height: channels.height.as_ref().map(part_of),
    })
}

/// The stored channels of a layer holding `content`, in a set `size`
/// pixels across, checked: painted parts only on paint layers, values
/// only on fills, none on groups.
pub(crate) fn channels_from(
    stored: ChannelsV78,
    content: &Content,
    size: u32,
) -> Result<Channels, Problem> {
    let damaged = || -> Problem { tr("its painted textures are damaged").into() };
    let part = |stored: Option<PartV78>| -> Result<Option<ChannelPart>, Problem> {
        let Some(stored) = stored else {
            return Ok(None);
        };
        match (content, stored.fill) {
            (Content::Fill(_), Some(value)) if stored.tiles.is_empty() => {
                Ok(Some(ChannelPart::Fill(value)))
            }
            (Content::Paint(_), None) => {
                Ok(Some(ChannelPart::Paint(image_from(stored.tiles, size)?)))
            }
            _ => Err(damaged()),
        }
    };
    Ok(Channels {
        roughness: part(stored.roughness)?,
        metal: part(stored.metal)?,
        glow: part(stored.glow)?,
        height: part(stored.height)?,
    })
}
