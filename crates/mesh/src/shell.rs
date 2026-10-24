//! Shells ([decision 0154](../../../docs/decisions/0154-shell.md)): a
//! surface given thickness, as a sheet of metal, a cup's wall or a
//! cloth's hem.
//!
//! The surface is copied, the copy turned to face the other way, and the
//! two pulled apart by [`ShellOptions::thickness`]:
//!
//! - **Inward** (the standard) keeps the surface where it is, as the
//!   outside, and puts the copy behind it: a shape modelled to size stays
//!   that size outside.
//! - **Outward** keeps the surface as the inside and grows the thickness
//!   in front of it.
//! - **Both sides** moves each half the thickness.
//!
//! With **even thickness** ([`offsets`]), points at corners move further,
//! by the angle there, so the walls keep the thickness right into sharp
//! corners (at most [`offsets::MAX_STRETCH`] times it). Without it, every
//! point moves the thickness along its direction.
//!
//! **Rims:** where the surface has open borders (a plane, a cup's lip),
//! [`ShellOptions::fill_rims`] joins the two layers with a strip of
//! quads. Their edges along the layers are marked hard, so the rim shades
//! as its own face, and as UV seams. If the shape has UVs, each rim strip
//! gets its own: laid out in rows just above the texture square, its
//! length along U, at the shape's texel density ([`rims`]), so they never
//! overlap the shape's own UVs. The layer behind copies the surface's UVs.
//!
//! **Ids:** the surface in front keeps its points' and polygons' ids
//! (selections and recorded edits still find their way); the copy behind
//! and the rims get new ids after them.
//!
//! **Not fixed:** where the thickness is more than a part is wide (a
//! thin spike, a narrow slot, a small closed shape thickened inward), the
//! two layers pass through each other. Use a smaller thickness there.
//!
//! Deterministic: the same shape and settings always give the same mesh.

pub mod offsets;
mod rims;

use crate::builder::{Builder, CornerFrom, PointFrom};
use crate::custom_normals::forget_custom_normals;
use crate::{FaceId, Mesh, VertexId};
use std::collections::HashMap;
use words::trf;

/// Which way the thickness grows from the surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ShellDirection {
    /// Behind the surface, which stays as the outside (the standard).
    #[default]
    Inward,
    /// In front of the surface, which stays as the inside.
    Outward,
    /// Half each way.
    Both,
}

impl ShellDirection {
    /// Every direction, in the order the interface lists them.
    pub const ALL: [Self; 3] = [Self::Inward, Self::Outward, Self::Both];

    /// How far, in thicknesses along the facing, the layer in front and
    /// the layer behind move.
    fn moves(self) -> (f32, f32) {
        match self {
            Self::Inward => (0.0, -1.0),
            Self::Outward => (1.0, 0.0),
            Self::Both => (0.5, -0.5),
        }
    }
}

/// The Shell modifier's settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShellOptions {
    /// How thick, in centimetres (standard 1 cm).
    pub thickness: f32,
    /// Which way it grows.
    pub direction: ShellDirection,
    /// Whether corners move further so the thickness stays even (on).
    pub even: bool,
    /// Whether open borders get a rim joining the two layers (on).
    pub fill_rims: bool,
}

impl ShellOptions {
    /// The standard thickness: 1 cm.
    pub const STANDARD_THICKNESS: f32 = 1.0;
    /// The thickest a shell can be: 100 m.
    pub const MAX_THICKNESS: f32 = 10_000.0;

    /// The settings within limits: a thickness from 0 to
    /// [`Self::MAX_THICKNESS`] (a negative one is its size: the direction
    /// says which way), the standard one when it isn't a number.
    #[must_use]
    pub fn clamped(self) -> Self {
        let thickness = if self.thickness.is_finite() {
            self.thickness.abs().min(Self::MAX_THICKNESS)
        } else {
            Self::STANDARD_THICKNESS
        };
        Self { thickness, ..self }
    }
}

impl Default for ShellOptions {
    fn default() -> Self {
        Self {
            thickness: Self::STANDARD_THICKNESS,
            direction: ShellDirection::Inward,
            even: true,
            fill_rims: true,
        }
    }
}

/// Why a shell can't be made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellProblem {
    /// The result would have more polygons than allowed.
    TooManyPolygons {
        /// How many it would have.
        polygons: u64,
        /// The most allowed.
        most: usize,
    },
}

impl std::fmt::Display for ShellProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooManyPolygons { polygons, most } => f.write_str(&trf!(
                "the result would have {} polygons, and the most is {}: a simpler shape",
                polygons,
                most
            )),
        }
    }
}

impl std::error::Error for ShellProblem {}

/// The open border edges of `mesh`, each as its one polygon runs along
/// it (from `.0` to `.1`, in `.2`), in a fixed order.
fn borders(mesh: &Mesh) -> Vec<(VertexId, VertexId, FaceId)> {
    let topology = mesh.topology();
    let mut out = Vec::new();
    for edge in topology.edges() {
        let [only] = topology.edge_uses(edge) else {
            continue;
        };
        let (low, high) = edge.ends();
        let (a, b) = if only.forward {
            (low, high)
        } else {
            (high, low)
        };
        out.push((a, b, only.face));
    }
    out
}

/// How many polygons a shell of `input` has.
pub fn polygon_count(input: &Mesh, options: &ShellOptions) -> u64 {
    let faces = input.face_count() as u64;
    let rims = if options.fill_rims {
        input.topology().border_edges().len() as u64
    } else {
        0
    };
    faces.saturating_mul(2).saturating_add(rims)
}

/// The Shell modifier's work: `input` given thickness as `options` say
/// (see the module's notes). A thickness of 0, or an empty shape, gives
/// the input as it is. Refused when the result would have more than
/// `most` polygons.
pub fn shell(input: &Mesh, options: &ShellOptions, most: usize) -> Result<Mesh, ShellProblem> {
    let options = options.clamped();
    if input.face_count() == 0 || options.thickness <= 0.0 {
        return Ok(input.clone());
    }
    let polygons = polygon_count(input, &options);
    if polygons > most as u64 {
        return Err(ShellProblem::TooManyPolygons { polygons, most });
    }
    let offsets = offsets::offsets(input, options.even);
    let (front, back) = options.direction.moves();
    let place = |v: VertexId, share: f32| -> Option<glam::Vec3> {
        let p = input.position(v)?;
        let offset = offsets.get(&v).copied().unwrap_or_default();
        Some(p + offset * (share * options.thickness))
    };
    let borders = if options.fill_rims {
        borders(input)
    } else {
        Vec::new()
    };
    let mut out = input.clone();
    // Each rim polygon, with the border it fills: corners `b`, `a`, `a`
    // behind, `b` behind.
    let mut rim_faces: Vec<rims::Rim> = Vec::new();
    {
        let mut build = Builder::new(input, &mut out);
        let behind = back_layer(&mut build, input, |v| place(v, back));
        for &(a, b, face) in &borders {
            if let (Some(&a2), Some(&b2)) = (behind.get(&a), behind.get(&b)) {
                let corners = [
                    (b, CornerFrom::Copy(face, b)),
                    (a, CornerFrom::Copy(face, a)),
                    (a2, CornerFrom::Copy(face, a)),
                    (b2, CornerFrom::Copy(face, b)),
                ];
                if let Some(rim) = build.face(&corners, Some(face)) {
                    rim_faces.push(rims::Rim {
                        face: rim,
                        front: (a, b),
                        behind: (a2, b2),
                    });
                }
            }
        }
    }
    // The layer in front: the surface itself, keeping its ids.
    for (v, _) in input.vertices() {
        if let Some(p) = place(v, front)
            && offsets.contains_key(&v)
        {
            out.set_position(v, p);
        }
    }
    forget_custom_normals(&mut out);
    rims::finish(&mut out, input, &rim_faces);
    Ok(out)
}

/// Adds the layer behind: a copy of every point a polygon uses (at
/// `place`), and of every polygon turned to face the other way, with
/// their values and their edges' (hard, seams, creases). Returns each
/// point's copy.
fn back_layer(
    build: &mut Builder,
    input: &Mesh,
    place: impl Fn(VertexId) -> Option<glam::Vec3>,
) -> HashMap<VertexId, VertexId> {
    let mut copies: HashMap<VertexId, VertexId> = HashMap::new();
    for (_, corners) in input.faces() {
        for v in corners {
            if copies.contains_key(v) {
                continue;
            }
            if let Some(p) = place(*v) {
                copies.insert(*v, build.point(p, PointFrom::Copy(*v)));
            }
        }
    }
    for (face, corners) in input.faces() {
        let turned: Option<Vec<(VertexId, CornerFrom)>> = corners
            .iter()
            .rev()
            .map(|v| Some((*copies.get(v)?, CornerFrom::Copy(face, *v))))
            .collect();
        if let Some(turned) = turned {
            build.face(&turned, Some(face));
        }
    }
    for edge in input.edges() {
        let (a, b) = edge.ends();
        if let (Some(&a2), Some(&b2)) = (copies.get(&a), copies.get(&b)) {
            build.copy_edge(*edge, a2, b2);
        }
    }
    copies
}

/// How much work a shell is, in polygons of a boolean's worth (see the
/// note on [`WORK_SHARE`]). Quick, as it is asked for often: the rims are
/// counted (which needs the shape's edges) only for shapes of fewer than
/// [`COUNT_RIMS_BELOW`] polygons; a bigger shape is heavy whatever its
/// borders, and counts its two sides.
pub fn work(input: &Mesh, options: &ShellOptions) -> usize {
    let polygons = if input.face_count() < COUNT_RIMS_BELOW {
        polygon_count(input, options)
    } else {
        (input.face_count() as u64).saturating_mul(2)
    };
    usize::try_from(polygons).unwrap_or(usize::MAX) / WORK_SHARE
}

/// Below this many polygons, [`work`] counts a shell's rims too (finding
/// the borders of 20 000 polygons takes about 2 ms in a release build).
const COUNT_RIMS_BELOW: usize = 20_000;

/// How many polygons of a shell's result cost as much as one polygon in
/// a boolean, for [`work`].
const WORK_SHARE: usize = 4;

#[cfg(test)]
mod tests;
