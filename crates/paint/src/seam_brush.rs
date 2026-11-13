//! The seam blend brush ([decision 0294](../../../docs/decisions/0294-seam-blend-brush.md)):
//! an effect brush that evens out the two sides of a UV seam as it is
//! stroked along it, on the layer being painted.
//!
//! Each dab first finds the seams near it on the shape ([`SeamBrushShape`])
//! and, the first time a stroke comes near a seam, works out for every
//! painted texel near it on either side the colour across the seam it is
//! pulled towards and how strongly: the same mapping as the Seam blend
//! layer (`seam_blend::blend_side`), half way on the seam and fading out
//! by smoothstep as far as the brush reaches, read from the layer as it
//! was when the stroke began. Painting a texel then mixes it towards that
//! colour by the dab's weight and the brush's flow. A dab on the model
//! covers both sides of a seam (they are the same place on the surface),
//! so both move towards the same middle colour and the join closes.
//! Texels away from every seam are left alone.

use crate::image::{Image, TILE};
use crate::seam_blend::{MAX_WIDTH, MIN_WIDTH, SeamLayout, Side, Sink, blend_side};
use crate::seams::Seam;
use crate::surface::PaintSurface;
use glam::Vec3;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// A shape made ready for the seam blend brush: its seams on the
/// material being painted, where its triangles lie on the texture, and
/// its seams by the triangles along them. Made once and kept while the
/// shape and the texture size stay the same.
#[derive(Debug)]
pub struct SeamBrushShape {
    surface: Arc<PaintSurface>,
    seams: Arc<Vec<Seam>>,
    layout: SeamLayout,
    /// Each triangle's seams (numbers in `seams`).
    by_triangle: HashMap<usize, Vec<u32>>,
}

impl SeamBrushShape {
    /// `surface`'s `seams` (on the material in `slots`, as
    /// [`crate::seams::seams_of`] finds them) for a `size`-pixel texture.
    pub fn new(
        surface: Arc<PaintSurface>,
        slots: &[u32],
        seams: Arc<Vec<Seam>>,
        size: u32,
    ) -> Self {
        let mut by_triangle: HashMap<usize, Vec<u32>> = HashMap::new();
        for (i, seam) in seams.iter().enumerate() {
            for side in seam.sides {
                by_triangle.entry(side.triangle).or_default().push(i as u32);
            }
        }
        Self {
            layout: SeamLayout::of(&surface, slots, size),
            surface,
            seams,
            by_triangle,
        }
    }

    /// Whether it was made for `surface` on a `size`-pixel texture.
    pub fn is_for(&self, surface: &Arc<PaintSurface>, size: u32) -> bool {
        Arc::ptr_eq(&self.surface, surface) && self.layout.size() == size
    }

    /// The texture size it was made for, in pixels.
    pub fn size(&self) -> u32 {
        self.layout.size()
    }

    /// Its seams.
    pub fn seams(&self) -> &[Seam] {
        &self.seams
    }

    /// The seams (numbers in [`Self::seams`]) whose edge comes within
    /// `radius` of `centre`, in the shape's own space, each once.
    pub fn seams_near(&self, centre: glam::Vec3, radius: f32) -> Vec<u32> {
        if !centre.is_finite() || !radius.is_finite() || radius < 0.0 {
            return Vec::new();
        }
        let mut triangles = Vec::new();
        self.surface.near(centre, radius, &mut triangles);
        let mut out: Vec<u32> = triangles
            .iter()
            .filter_map(|t| self.by_triangle.get(t))
            .flatten()
            .copied()
            .filter(|&i| segment_distance(centre, self.seams[i as usize].ends) <= radius)
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }
}

/// How far `p` is from the segment between `ends`.
fn segment_distance(p: Vec3, [a, b]: [Vec3; 2]) -> f32 {
    let d = b - a;
    let t = if d.length_squared() > 0.0 {
        ((p - a).dot(d) / d.length_squared()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    p.distance(a + d * t)
}

/// What a texel near a seam is pulled towards: the colour across the
/// seam, how strongly (0 to 0.5) and how far it is from the seam.
#[derive(Debug, Clone, Copy)]
struct Pull {
    colour: [u8; 4],
    strength: f32,
    distance: f32,
}

const NO_PULL: Pull = Pull {
    colour: [0; 4],
    strength: 0.0,
    distance: f32::INFINITY,
};

/// A seam blend stroke's pulls: the texels near the seams it has come
/// near so far, worked out from the layer as the stroke began.
#[derive(Debug, Clone)]
pub(crate) struct SeamPulls {
    shape: Arc<SeamBrushShape>,
    /// The seams worked out so far.
    done: HashSet<u32>,
    /// Each tile's pulls (none for a tile no seam reaches).
    tiles: HashMap<usize, Box<[Pull]>>,
    across: u32,
}

impl SeamPulls {
    pub(crate) fn new(shape: Arc<SeamBrushShape>) -> Self {
        Self {
            across: crate::image::tiles_across(shape.size()),
            shape,
            done: HashSet::new(),
            tiles: HashMap::new(),
        }
    }

    /// Works out the pulls of the seams near a dab at `centre` reaching
    /// `radius` (both in the shape's own space), from `base`; the blend
    /// reaches as far from each seam as the dab does. Whether any seam is
    /// under the dab.
    pub(crate) fn reach(&mut self, base: &Image, centre: Vec3, radius: f32) -> bool {
        if base.size() != self.shape.size() {
            return false;
        }
        let shape = Arc::clone(&self.shape);
        // A seam a little past the dab still pulls the texels under it.
        let near = shape.seams_near(centre, radius * 2.0);
        let size = base.size() as f32;
        let mut under = false;
        for i in near {
            let seam = &shape.seams[i as usize];
            under |= segment_distance(centre, seam.ends) <= radius;
            if !self.done.insert(i) {
                continue;
            }
            let long = seam.ends[0].distance(seam.ends[1]);
            if !long.is_finite() || long <= 1e-9 {
                continue;
            }
            for k in 0..2 {
                let (from, to) = (seam.sides[k], seam.sides[1 - k]);
                // The dab's radius in this side's texels.
                let texels = from.uvs[0].distance(from.uvs[1]) * size / long;
                let width = (radius * texels).clamp(MIN_WIDTH, MAX_WIDTH);
                if width.is_finite() {
                    blend_side(
                        Side::on(base, &shape.surface, &shape.layout, &from),
                        Side::on(base, &shape.surface, &shape.layout, &to),
                        width,
                        self,
                    );
                }
            }
        }
        under
    }

    /// What texel (`x`, `y`) is pulled towards and how strongly, if a
    /// seam reaches it.
    pub(crate) fn at(&self, x: u32, y: u32) -> Option<([u8; 4], f32)> {
        let index = ((y / TILE) * self.across + x / TILE) as usize;
        let at = ((y % TILE) * TILE + x % TILE) as usize;
        let pull = self.tiles.get(&index)?[at];
        (pull.strength > 0.0).then_some((pull.colour, pull.strength))
    }
}

impl Sink for SeamPulls {
    fn nearer(&self, x: u32, y: u32, distance: f32) -> bool {
        let index = ((y / TILE) * self.across + x / TILE) as usize;
        let at = ((y % TILE) * TILE + x % TILE) as usize;
        self.tiles
            .get(&index)
            .is_some_and(|pulls| pulls[at].distance <= distance)
    }

    fn put(&mut self, x: u32, y: u32, distance: f32, colour: [u8; 4], strength: f32) {
        let index = ((y / TILE) * self.across + x / TILE) as usize;
        let at = ((y % TILE) * TILE + x % TILE) as usize;
        let pulls = self
            .tiles
            .entry(index)
            .or_insert_with(|| vec![NO_PULL; (TILE * TILE) as usize].into());
        if distance < pulls[at].distance {
            pulls[at] = Pull {
                colour,
                strength,
                distance,
            };
        }
    }
}

#[cfg(test)]
mod tests;
