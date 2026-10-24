//! Struts ([decision 0159](../../../docs/decisions/0159-struts.md)): every
//! edge of a shape turned into a bar of its own, with a joint at each
//! point. A frame, a cage, a grille, scaffolding, a lattice tower.
//!
//! Not the viewport's *Wireframe* look, which only draws the edges as
//! lines: these are real polygons, which export and bake.
//!
//! - **A strut** per edge: a closed prism along the edge, its profile a
//!   regular polygon of [`StrutsOptions::sides`] sides (4, a square bar, is
//!   the standard; more for a round one), with a flat side facing the way
//!   the edge's first polygon faces, so a square frame lies flat on the
//!   surface and square with a box's sides. Each end is closed with a
//!   polygon.
//! - **A joint** per point: a closed, faceted ball round the point, as
//!   many sides round as the struts, big enough to hide the struts' ends
//!   (a cube for square struts). One flat faces the point's first edge.
//! - **Thickness** is the struts' width. With **even thickness** (the
//!   standard) it is measured across the flats, so a strut is exactly that
//!   wide whatever its sides; off, across the corners, so the strut fits
//!   in a round rod that thick (and a square one is thinner on its flats).
//! - **Keep the faces** keeps the shape's own polygons as well (with their
//!   ids), the struts round them: a panel in a frame. Off (the standard),
//!   only the struts and joints are left.
//!
//! **Hard edges** are the profile's corners when it has 6 sides or fewer
//! (a square strut shades flat on each side, a round one smooth), and
//! where each strut's ends meet its sides. **UVs:** each strut and each
//! joint is laid flat as an island of its own (see [`layout`]), all at the
//! same texel density, packed into the texture square (with Keep the
//! faces, into the square above it, so the shape's own UVs stay as they
//! were), with seams round each island. **Materials:** a strut takes the
//! material of the first polygon along its edge, a joint of the first
//! polygon round its point.
//!
//! The pieces overlap at the joints (they aren't welded into one
//! surface): each is closed, which renders, bakes and exports cleanly,
//! and nothing can fold however the edges meet. Deterministic: the same
//! shape and settings always give the same mesh.

mod layout;
mod pieces;

use crate::custom_normals::forget_custom_normals;
use crate::material::{set_material, slot_of};
use crate::uvs::{Uvs, set_seams, set_uvs, uv_splits};
use crate::{EdgeKey, FaceId, Mesh, VertexId};
use glam::Vec2;
use pieces::Piece;
use std::collections::BTreeMap;
use std::f32::consts::PI;
use words::trf;

/// The Struts modifier's settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StrutsOptions {
    /// How wide each strut is, in centimetres (standard 2 cm).
    pub thickness: f32,
    /// How many sides a strut's profile has (standard 4, a square bar).
    pub sides: u32,
    /// Whether the thickness is measured across the flats (on) or the
    /// corners.
    pub even: bool,
    /// Whether the shape's own polygons stay as well (off).
    pub keep_faces: bool,
}

impl StrutsOptions {
    /// The standard thickness: 2 cm.
    pub const STANDARD_THICKNESS: f32 = 2.0;
    /// The thickest a strut can be: 100 m.
    pub const MAX_THICKNESS: f32 = 10_000.0;
    /// The standard number of sides: a square strut.
    pub const STANDARD_SIDES: u32 = 4;
    /// The fewest sides: a triangle.
    pub const MIN_SIDES: u32 = 3;
    /// The most sides.
    pub const MAX_SIDES: u32 = 24;
    /// Profiles with this many sides or fewer have hard corners.
    pub const HARD_UP_TO: u32 = 6;

    /// The settings within limits: a thickness from 0 to
    /// [`Self::MAX_THICKNESS`] (a negative one is its size), the standard
    /// one when it isn't a number; sides from [`Self::MIN_SIDES`] to
    /// [`Self::MAX_SIDES`].
    #[must_use]
    pub fn clamped(self) -> Self {
        let thickness = if self.thickness.is_finite() {
            self.thickness.abs().min(Self::MAX_THICKNESS)
        } else {
            Self::STANDARD_THICKNESS
        };
        Self {
            thickness,
            sides: self.sides.clamp(Self::MIN_SIDES, Self::MAX_SIDES),
            ..self
        }
    }

    /// A strut's profile: how far its flats and its corners are from the
    /// edge.
    fn radii(&self) -> (f32, f32) {
        let half = self.thickness / 2.0;
        let flat_to_corner = (PI / self.sides as f32).cos();
        if self.even {
            (half, half / flat_to_corner)
        } else {
            (half * flat_to_corner, half)
        }
    }

    /// How many rings of points a joint has, bottom to top.
    fn joint_rings(&self) -> u32 {
        (self.sides.div_ceil(2)).max(2)
    }
}

impl Default for StrutsOptions {
    fn default() -> Self {
        Self {
            thickness: Self::STANDARD_THICKNESS,
            sides: Self::STANDARD_SIDES,
            even: true,
            keep_faces: false,
        }
    }
}

/// Why struts can't be made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrutsProblem {
    /// The result would have more polygons than allowed.
    TooManyPolygons {
        /// How many it would have.
        polygons: u64,
        /// The most allowed.
        most: usize,
    },
    /// The mesh is full: no more ids.
    Full,
}

impl std::fmt::Display for StrutsProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooManyPolygons { polygons, most } => f.write_str(&trf!(
                "the result would have {} polygons, and the most is {}: fewer sides, or a \
                 simpler shape",
                polygons,
                most
            )),
            Self::Full => f.write_str(words::tr("the result would have too many polygons")),
        }
    }
}

impl std::error::Error for StrutsProblem {}

/// The points a polygon uses, in id order: each gets a joint.
fn used_points(input: &Mesh) -> Vec<VertexId> {
    let topology = input.topology();
    input
        .vertices()
        .map(|(v, _)| v)
        .filter(|v| !topology.vertex_faces(*v).is_empty())
        .collect()
}

/// How many polygons struts of `input` make (and keep).
pub fn polygon_count(input: &Mesh, options: &StrutsOptions) -> u64 {
    let options = options.clamped();
    let sides = u64::from(options.sides);
    let strut = sides + 2;
    let joint = sides * u64::from(options.joint_rings() - 1) + 2;
    let edges = input.edges().len() as u64;
    let points = used_points(input).len() as u64;
    let kept = if options.keep_faces {
        input.face_count() as u64
    } else {
        0
    };
    edges
        .saturating_mul(strut)
        .saturating_add(points.saturating_mul(joint))
        .saturating_add(kept)
}

/// The Struts modifier's work: every edge of `input` a strut and every
/// point a joint, as `options` say (see the module's notes). A thickness
/// of 0, or a shape with no polygons, gives the input as it is. Refused
/// when the result would have more than `most` polygons.
pub fn struts(input: &Mesh, options: &StrutsOptions, most: usize) -> Result<Mesh, StrutsProblem> {
    let options = options.clamped();
    if input.face_count() == 0 || options.thickness <= 0.0 {
        return Ok(input.clone());
    }
    let polygons = polygon_count(input, &options);
    if polygons > most as u64 {
        return Err(StrutsProblem::TooManyPolygons { polygons, most });
    }
    let mut pieces: Vec<Piece> = Vec::new();
    for edge in input.edges() {
        if let Some(piece) = pieces::strut(input, *edge, &options) {
            pieces.push(piece);
        }
    }
    for point in used_points(input) {
        if let Some(piece) = pieces::joint(input, point, &options) {
            pieces.push(piece);
        }
    }
    let mut out = if options.keep_faces {
        let mut kept = input.clone();
        forget_custom_normals(&mut kept);
        kept
    } else {
        Mesh::new()
    };
    let kept_corners = out.face_layout().1.len();
    let first_point = out.point_slot_count();
    let mut made: Vec<pieces::Made> = Vec::with_capacity(pieces.len());
    for piece in &pieces {
        made.push(piece.add_to(&mut out).ok_or(StrutsProblem::Full)?);
    }
    give_materials(&mut out, input, &pieces, &made);
    give_uvs(&mut out, &pieces, &made, kept_corners, options.keep_faces);
    for (piece, made) in pieces.iter().zip(&made) {
        for (a, b) in made.hard_edges(piece) {
            out.set_hard(a, b, true);
        }
    }
    let new_seams: Vec<EdgeKey> = uv_splits(&out)
        .into_iter()
        .filter(|e| e.ends().0.get() as usize >= first_point)
        .collect();
    set_seams(&mut out, &new_seams, true);
    Ok(out)
}

/// Gives each piece's polygons the material of the polygon it came from,
/// when the shape has more than the first.
fn give_materials(out: &mut Mesh, input: &Mesh, pieces: &[Piece], made: &[pieces::Made]) {
    let mut by_slot: BTreeMap<u32, Vec<FaceId>> = BTreeMap::new();
    for (piece, made) in pieces.iter().zip(made) {
        let slot = slot_of(input, piece.material_from);
        if slot != 0 {
            by_slot
                .entry(slot)
                .or_default()
                .extend(made.faces.iter().map(|(f, _)| *f));
        }
    }
    for (slot, faces) in by_slot {
        set_material(out, &faces, slot);
    }
}

/// Lays every piece flat as its own island, packs them into the texture
/// square (the one above it when the shape's own polygons are kept), and
/// sets the UVs.
fn give_uvs(
    out: &mut Mesh,
    pieces: &[Piece],
    made: &[pieces::Made],
    kept_corners: usize,
    above: bool,
) {
    let corner_count = out.face_layout().1.len();
    let starts: Vec<u32> = out.face_layout().0.to_vec();
    let mut values: Vec<Vec2> = match Uvs::of(out) {
        Some(uvs) => uvs.into_values(corner_count),
        None => vec![Vec2::ZERO; corner_count],
    };
    let islands: Vec<layout::Island> = pieces
        .iter()
        .zip(made)
        .map(|(piece, made)| layout::island(piece, made))
        .collect();
    let lift = if above { Vec2::Y } else { Vec2::ZERO };
    for (island, placed) in islands.iter().zip(layout::pack(&islands)) {
        for (face, uvs) in &island.faces {
            let Some(start) = starts.get(face.get() as usize) else {
                continue;
            };
            for (k, uv) in uvs.iter().enumerate() {
                let index = *start as usize + k;
                if index < kept_corners {
                    continue;
                }
                if let Some(slot) = values.get_mut(index) {
                    let uv = placed.apply(*uv) + lift;
                    *slot = if uv.is_finite() { uv } else { Vec2::ZERO };
                }
            }
        }
    }
    set_uvs(out, values);
}

/// How much work struts are, in polygons of a boolean's worth (four
/// polygons made cost about as much as one in a boolean).
pub fn work(input: &Mesh, options: &StrutsOptions) -> usize {
    let polygons = usize::try_from(polygon_count(input, options)).unwrap_or(usize::MAX);
    polygons / WORK_SHARE
}

/// How many polygons of the result cost as much as one polygon in a
/// boolean, for [`work`].
const WORK_SHARE: usize = 4;

#[cfg(test)]
mod tests;
