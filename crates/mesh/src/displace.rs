//! Displace: pushing a shape's points in and out by a noise
//! ([decision 0157](../../../docs/decisions/0157-displace.md)), the work
//! of the *Displace* modifier: rough rock, lumpy ground, dented metal,
//! cobbles, bark.
//!
//! Each point is pushed by **Strength** times the noise's value where it
//! is, along the way the shape faces there (its polygons' facings round
//! it, bigger ones counting more: [`crate::shrinkwrap::point_normals`]) or
//! along one of the shape's own axes. The noise (`noise`) is written by
//! hand and depends only on the point's place, the **Seed** and the
//! settings, so the same shape always gets the same bumps:
//!
//! - **Scale** is the size of its bumps (a noise of size 1 stretched to
//!   it);
//! - **Detail** adds smaller bumps on the bumps, each layer half the size
//!   and half the height;
//! - **Noise** picks its look: *Smooth* rolls both ways round the surface
//!   (about −1 to 1, so the shape keeps its size), *Ridged* raises sharp
//!   crests and *Cells* raises rounded cells with grooves between them
//!   (both 0 to 1: outward only, inward with a negative strength).
//!
//! It can be limited to **chosen points** (`falloff`), fading out over
//! the **Falloff** distance from them along the surface.
//!
//! Only points move (through [`crate::deform::with_points_moved`]): polygons,
//! ids, UVs, seams, creases, hard edges and materials stay, as for every
//! deformer. A point with no polygons has no facing and stays when pushed
//! along normals; a point sent anywhere that isn't a real number stays.

mod falloff;
pub(crate) mod noise;

use crate::{Mesh, VertexId};
use glam::Vec3;
use std::collections::BTreeSet;
use words::tr;

/// The look of the noise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NoiseKind {
    /// Soft rolling bumps, in and out.
    #[default]
    Smooth,
    /// Sharp crests, outward.
    Ridged,
    /// Rounded cells with grooves between them, outward.
    Cells,
}

impl NoiseKind {
    /// Every kind, in the order the interface lists them.
    pub const ALL: [Self; 3] = [Self::Smooth, Self::Ridged, Self::Cells];

    /// Its name, as the interface shows it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Smooth => tr("Smooth"),
            Self::Ridged => tr("Ridged"),
            Self::Cells => tr("Cells"),
        }
    }
}

/// Which way the points are pushed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DisplaceDirection {
    /// Along the way the shape faces at each point.
    #[default]
    Normals,
    /// Along the shape's X.
    AxisX,
    /// Along the shape's Y.
    AxisY,
    /// Along the shape's Z (up): ground and terrain.
    AxisZ,
}

impl DisplaceDirection {
    /// Every direction, in the order the interface lists them.
    pub const ALL: [Self; 4] = [Self::Normals, Self::AxisX, Self::AxisY, Self::AxisZ];

    /// Its name, as the interface shows it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Normals => tr("Along the normals"),
            Self::AxisX => tr("Along X"),
            Self::AxisY => tr("Along Y"),
            Self::AxisZ => tr("Along Z"),
        }
    }

    /// The axis, for the three that are one.
    fn axis(self) -> Option<Vec3> {
        match self {
            Self::Normals => None,
            Self::AxisX => Some(Vec3::X),
            Self::AxisY => Some(Vec3::Y),
            Self::AxisZ => Some(Vec3::Z),
        }
    }
}

/// How a shape's points are displaced.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DisplaceOptions {
    /// How far the noise's highest value pushes a point, in centimetres
    /// (negative pushes the other way).
    pub strength: f32,
    /// The size of the noise's bumps, in centimetres.
    pub scale: f32,
    /// Another seed gives other bumps; the same seed always the same.
    pub seed: u32,
    /// How many layers of ever smaller bumps (octaves), 1 to
    /// [`Self::MAX_DETAIL`].
    pub detail: u32,
    /// The noise's look.
    pub noise: NoiseKind,
    /// Which way points are pushed.
    pub direction: DisplaceDirection,
    /// With chosen points, how far from them (along the surface) the
    /// push fades out, in centimetres; 0 for a sharp edge.
    pub falloff: f32,
}

impl Default for DisplaceOptions {
    fn default() -> Self {
        Self {
            strength: Self::STANDARD_STRENGTH,
            scale: Self::STANDARD_SCALE,
            seed: 1,
            detail: Self::STANDARD_DETAIL,
            noise: NoiseKind::Smooth,
            direction: DisplaceDirection::Normals,
            falloff: Self::STANDARD_FALLOFF,
        }
    }
}

impl DisplaceOptions {
    /// The standard strength: 5 cm.
    pub const STANDARD_STRENGTH: f32 = 5.0;
    /// The standard size of the bumps: 25 cm.
    pub const STANDARD_SCALE: f32 = 25.0;
    /// The standard detail: 3 layers.
    pub const STANDARD_DETAIL: u32 = 3;
    /// The standard falloff: 20 cm.
    pub const STANDARD_FALLOFF: f32 = 20.0;
    /// The strongest push either way, in centimetres (100 m).
    pub const MAX_STRENGTH: f32 = 10_000.0;
    /// The smallest bumps, in centimetres (1 mm).
    pub const MIN_SCALE: f32 = 0.1;
    /// The biggest bumps, in centimetres (1 km).
    pub const MAX_SCALE: f32 = 100_000.0;
    /// The highest seed.
    pub const MAX_SEED: u32 = 99_999;
    /// The most layers of detail.
    pub const MAX_DETAIL: u32 = 8;
    /// The longest falloff, in centimetres (1 km).
    pub const MAX_FALLOFF: f32 = 100_000.0;

    /// The same options within limits; a length that isn't a real number
    /// becomes the standard one.
    pub fn clamped(self) -> Self {
        let within = |value: f32, low: f32, high: f32, standard: f32| {
            if value.is_finite() {
                value.clamp(low, high)
            } else {
                standard
            }
        };
        Self {
            strength: within(
                self.strength,
                -Self::MAX_STRENGTH,
                Self::MAX_STRENGTH,
                Self::STANDARD_STRENGTH,
            ),
            scale: within(
                self.scale,
                Self::MIN_SCALE,
                Self::MAX_SCALE,
                Self::STANDARD_SCALE,
            ),
            seed: self.seed.min(Self::MAX_SEED),
            detail: self.detail.clamp(1, Self::MAX_DETAIL),
            noise: self.noise,
            direction: self.direction,
            falloff: within(self.falloff, 0.0, Self::MAX_FALLOFF, Self::STANDARD_FALLOFF),
        }
    }

    /// The noise's value at `p` (in the shape's space), before Strength:
    /// about −1 to 1 for Smooth, 0 to 1 for Ridged and Cells.
    pub fn value_at(&self, p: Vec3) -> f32 {
        let options = self.clamped();
        noise::layered(
            options.noise,
            options.seed,
            options.detail,
            p / options.scale,
        )
    }
}

/// Roughly how much work displacing `mesh` is, in polygons of a boolean's
/// worth (from 5 000, the viewport works it out on another thread).
/// Measured in a release build on one thread: a layer of smooth noise
/// takes about 0.1 microseconds a point, of ridged about 0.2 and of cells
/// about 0.3 (a boolean's polygon about 4 microseconds); limiting it to
/// chosen points adds about 0.4 microseconds a point.
pub fn work(mesh: &Mesh, options: &DisplaceOptions, limited: bool) -> usize {
    let options = options.clamped();
    let per_layer = match options.noise {
        NoiseKind::Smooth => 1,
        NoiseKind::Ridged => 2,
        NoiseKind::Cells => 3,
    };
    let tenths = options.detail as usize * per_layer + if limited { 4 } else { 0 };
    mesh.vertex_count().saturating_mul(tenths) / 40
}

/// `mesh` with its points pushed by the noise `options` describes: every
/// point, or (when `only` is given) the points in it, fading out over the
/// falloff round them along the surface. Only the points move.
pub fn displace(mesh: &Mesh, only: Option<&BTreeSet<VertexId>>, options: &DisplaceOptions) -> Mesh {
    let options = options.clamped();
    if options.strength == 0.0 {
        return mesh.clone();
    }
    let normals = match options.direction {
        DisplaceDirection::Normals => crate::shrinkwrap::point_normals(mesh),
        _ => Vec::new(),
    };
    let weights = only.map(|only| falloff::weights(mesh, only, options.falloff));
    crate::deform::with_points_moved(mesh, |id, p| {
        let at = id.get() as usize;
        let weight = weights
            .as_ref()
            .map_or(1.0, |w| w.get(at).copied().unwrap_or(0.0));
        if weight <= 0.0 {
            return p;
        }
        let way = match options.direction.axis() {
            Some(axis) => axis,
            None => normals.get(at).copied().unwrap_or(Vec3::ZERO),
        };
        if way == Vec3::ZERO {
            return p;
        }
        let value = noise::layered(
            options.noise,
            options.seed,
            options.detail,
            p / options.scale,
        );
        p + way * (value * options.strength * weight)
    })
}

#[cfg(test)]
mod tests;
