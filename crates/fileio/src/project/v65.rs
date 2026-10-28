//! Version 65 of the project file ([decision 0179](../../../../docs/decisions/0179-the-round-brush.md)):
//! version 64's, with the strokes painted on each paint layer
//! (`v64::LayerV64::strokes`, stored as [`StrokeV65`]): the brush and each
//! dab's place, facing and size on the shape, kept so a later version can
//! paint them again at another texture size or on a changed shape. Files
//! before version 65 hold none.

use super::v1::Problem;
use crate::limits::Bounded;
use paint::{BrushKind, DabRecord, FillBy, FillRecord, StrokeRecord};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use words::tr;

/// A stroke as stored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct StrokeV65 {
    /// 0 paint, 1 erase.
    pub(crate) kind: u8,
    /// Its colour.
    pub(crate) color: [u8; 4],
    /// Its hardness.
    pub(crate) hardness: f32,
    /// Its flow.
    pub(crate) flow: f32,
    /// Whether it painted hidden parts too.
    pub(crate) through: bool,
    /// Its dabs: place (3), way towards the camera (3) and radius, seven
    /// numbers each.
    pub(crate) dabs: Bounded<[f32; 7]>,
    /// What the polygon fill tool filled (version 67): the polygon's id
    /// and 0 polygon, 1 UV piece, 2 material.
    #[serde(default = "Bounded::new")]
    pub(crate) fills: Bounded<(u32, u8)>,
    /// Each dab's strength, 0 to 1 (version 69): none when every dab had
    /// full strength (and in files before version 69).
    #[serde(default = "Bounded::new")]
    pub(crate) strengths: Bounded<f32>,
}

/// The strokes to store for a layer.
pub(crate) fn strokes_of(strokes: &[Arc<StrokeRecord>]) -> Bounded<StrokeV65> {
    strokes
        .iter()
        .map(|s| StrokeV65 {
            kind: match s.kind {
                BrushKind::Paint => 0,
                BrushKind::Erase => 1,
            },
            color: s.color,
            hardness: s.hardness,
            flow: s.flow,
            through: s.through,
            dabs: s
                .dabs
                .iter()
                .map(|d| {
                    [
                        d.at.x, d.at.y, d.at.z, d.toward.x, d.toward.y, d.toward.z, d.radius,
                    ]
                })
                .collect(),
            fills: s
                .fills
                .iter()
                .map(|f| (f.face, fill_by_index(f.by)))
                .collect(),
            strengths: if s.dabs.iter().all(|d| d.strength == 1.0) {
                Bounded::new()
            } else {
                s.dabs.iter().map(|d| d.strength).collect()
            },
        })
        .collect()
}

fn fill_by_index(by: FillBy) -> u8 {
    match by {
        FillBy::Polygon => 0,
        FillBy::Piece => 1,
        FillBy::Material => 2,
    }
}

/// The stored strokes, checked: an unknown kind is refused (made by a
/// newer version), numbers out of range are damage.
pub(crate) fn strokes_from(stored: Bounded<StrokeV65>) -> Result<Vec<Arc<StrokeRecord>>, Problem> {
    let mut out = Vec::new();
    for s in stored {
        let kind = match s.kind {
            0 => BrushKind::Paint,
            1 => BrushKind::Erase,
            _ => {
                return Err(tr(
                    "it has a brush stroke of a kind this version doesn't know (made by a newer \
                     version?)",
                )
                .into());
            }
        };
        if !s.strengths.is_empty() && s.strengths.len() != s.dabs.len() {
            return Err(tr("its painted textures are damaged").into());
        }
        let strength = |i: usize| s.strengths.get(i).copied().unwrap_or(1.0);
        let record = StrokeRecord {
            kind,
            color: s.color,
            hardness: s.hardness,
            flow: s.flow,
            through: s.through,
            dabs: s
                .dabs
                .iter()
                .enumerate()
                .map(|(i, d)| DabRecord {
                    at: glam::Vec3::new(d[0], d[1], d[2]),
                    toward: glam::Vec3::new(d[3], d[4], d[5]),
                    radius: d[6],
                    strength: strength(i),
                })
                .collect(),
            fills: s
                .fills
                .into_iter()
                .map(|(face, by)| {
                    let by = match by {
                        0 => FillBy::Polygon,
                        1 => FillBy::Piece,
                        2 => FillBy::Material,
                        _ => return None,
                    };
                    Some(FillRecord { face, by })
                })
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| -> Problem {
                    tr(
                        "it has a polygon fill of a kind this version doesn't know (made by a \
                        newer version?)",
                    )
                    .into()
                })?,
        };
        if !record.is_valid() {
            return Err(tr("its painted textures are damaged").into());
        }
        out.push(Arc::new(record));
    }
    Ok(out)
}
