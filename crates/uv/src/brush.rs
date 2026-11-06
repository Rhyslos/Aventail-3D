//! UV brushes ([decision 0238](../../../docs/decisions/0238-uv-brushes.md)):
//! grab, relax and pinch UVs under a soft round brush in the UV Editor.
//!
//! A brush works on **spots**: the places in the texture where a point's
//! corners sit together (a point inside a piece is one spot; a point on a
//! seam is one spot per side). Moving a spot moves all its corners, so a
//! piece bends and never tears. Pinned corners (decision 0109) and locked
//! pieces (decision 0112) don't move.
//!
//! Each dab moves spots within `radius` of the brush's middle by a smooth
//! falloff: full in the middle, nothing at the rim.
//!
//! - **Grab** moves them with the mouse.
//! - **Relax** pulls each towards the middle of its neighbours (the spots
//!   it shares a polygon side with), evening the UVs out.
//! - **Pinch** pulls them towards the brush's middle (or, the other way,
//!   pushes them out).

use glam::Vec2;
use mesh::uvs::Uvs;
use mesh::{FaceId, Mesh, VertexId};
use std::collections::{BTreeSet, HashMap};

use crate::pieces::Corner;

/// The brushes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UvBrushKind {
    /// Move the UVs with the mouse.
    #[default]
    Grab,
    /// Even the UVs out.
    Relax,
    /// Pull the UVs to the middle (or push them out).
    Pinch,
}

impl UvBrushKind {
    /// Every brush, in menu order.
    pub const ALL: [Self; 3] = [Self::Grab, Self::Relax, Self::Pinch];
}

/// One spot: where it is now, where it was as the stroke began, its
/// corners, its neighbours, and whether it may move.
#[derive(Debug, Clone, PartialEq)]
struct Spot {
    at: Vec2,
    corners: Vec<(FaceId, VertexId)>,
    neighbours: Vec<usize>,
    fixed: bool,
}

/// One object's UVs as spots, for a stroke.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Spots {
    spots: Vec<Spot>,
    /// Whether each spot moved this stroke.
    moved: Vec<bool>,
}

/// Where corners count as one spot.
const SAME: f32 = 1e-5;

/// How much of the falloff is used, at `d` from the middle of a brush of
/// `radius`: 1 in the middle, smoothly to 0 at the rim.
pub fn falloff(d: f32, radius: f32) -> f32 {
    if radius.is_nan() || d.is_nan() || radius <= 0.0 || d >= radius {
        return 0.0;
    }
    let t = d / radius;
    let k = 1.0 - t * t;
    k * k
}

impl Spots {
    /// The spots of `mesh`'s UVs (empty without UVs).
    pub fn of(mesh: &Mesh) -> Self {
        let Some(uvs) = Uvs::of(mesh) else {
            return Self::default();
        };
        let pins = mesh::uvs::pins(mesh);
        let lock = mesh::uvs::locked(mesh);
        let (starts, _) = mesh.face_layout();
        // Each point's spots, found by place.
        let mut by_point: HashMap<VertexId, Vec<usize>> = HashMap::new();
        let mut spots: Vec<Spot> = Vec::new();
        let mut corner_spot: HashMap<(FaceId, VertexId), usize> = HashMap::new();
        for (f, list) in mesh.faces() {
            let start = starts[f.get() as usize] as usize;
            let locked = lock
                .as_ref()
                .is_some_and(|l| l.get(f.get() as usize).copied().unwrap_or(false));
            for (k, v) in list.iter().enumerate() {
                let i = start + k;
                let at = uvs.at(i);
                let pinned = pins
                    .as_ref()
                    .is_some_and(|p| p.get(i).copied().unwrap_or(false));
                let list = by_point.entry(*v).or_default();
                let found = list
                    .iter()
                    .copied()
                    .find(|s| spots[*s].at.distance(at) <= SAME);
                let s = match found {
                    Some(s) => s,
                    None => {
                        spots.push(Spot {
                            at,
                            corners: Vec::new(),
                            neighbours: Vec::new(),
                            fixed: false,
                        });
                        list.push(spots.len() - 1);
                        spots.len() - 1
                    }
                };
                spots[s].corners.push((f, *v));
                spots[s].fixed |= pinned || locked;
                corner_spot.insert((f, *v), s);
            }
        }
        // Neighbours along polygon sides.
        let mut links: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); spots.len()];
        for (f, list) in mesh.faces() {
            let n = list.len();
            for k in 0..n {
                let (Some(&a), Some(&b)) = (
                    corner_spot.get(&(f, list[k])),
                    corner_spot.get(&(f, list[(k + 1) % n])),
                ) else {
                    continue;
                };
                if a != b {
                    links[a].insert(b);
                    links[b].insert(a);
                }
            }
        }
        for (s, l) in spots.iter_mut().zip(links) {
            s.neighbours = l.into_iter().collect();
        }
        let moved = vec![false; spots.len()];
        Self { spots, moved }
    }

    /// How many spots there are.
    pub fn len(&self) -> usize {
        self.spots.len()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.spots.is_empty()
    }

    /// One dab of `kind` at `centre` with `radius` (texture widths).
    /// `pull` is the mouse's move since the last dab (Grab); `strength`
    /// (0 to 1) how much of the way Relax and Pinch go per dab; `out`
    /// pushes instead of pulling (Pinch). Returns how many spots moved.
    pub fn dab(
        &mut self,
        kind: UvBrushKind,
        centre: Vec2,
        radius: f32,
        pull: Vec2,
        strength: f32,
        out: bool,
    ) -> usize {
        if !(centre.is_finite() && radius.is_finite() && pull.is_finite() && strength.is_finite()) {
            return 0;
        }
        let strength = strength.clamp(0.0, 1.0);
        let reached: Vec<(usize, f32)> = self
            .spots
            .iter()
            .enumerate()
            .filter(|(_, s)| !s.fixed)
            .filter_map(|(i, s)| {
                let w = falloff(s.at.distance(centre), radius);
                (w > 0.0).then_some((i, w))
            })
            .collect();
        let moves: Vec<(usize, Vec2)> = reached
            .iter()
            .filter_map(|&(i, w)| {
                let s = &self.spots[i];
                let d = match kind {
                    UvBrushKind::Grab => pull * w,
                    UvBrushKind::Relax => {
                        if s.neighbours.is_empty() {
                            return None;
                        }
                        let mean = s.neighbours.iter().map(|n| self.spots[*n].at).sum::<Vec2>()
                            / s.neighbours.len() as f32;
                        (mean - s.at) * w * strength
                    }
                    UvBrushKind::Pinch => {
                        let to = (centre - s.at) * w * strength * 0.25;
                        if out { -to } else { to }
                    }
                };
                (d.is_finite() && d.length_squared() > 0.0).then_some((i, d))
            })
            .collect();
        for (i, d) in &moves {
            self.spots[*i].at += *d;
            self.moved[*i] = true;
        }
        moves.len()
    }

    /// The corners of every spot moved this stroke, with their UVs now.
    pub fn moved_corners(&self) -> Vec<Corner> {
        let mut out = Vec::new();
        for (s, moved) in self.spots.iter().zip(&self.moved) {
            if *moved {
                for (f, v) in &s.corners {
                    out.push((*f, *v, s.at));
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests;
