//! The struts' and joints' shapes ([decision 0159](../../../../docs/decisions/0159-struts.md)).
//!
//! Both are a **piece**: rings of points, each a regular polygon round
//! the piece's axis, counter-clockwise seen from its far end, bottom to
//! top; a band of quads between each ring and the next, and a polygon
//! closing each end. A strut has two rings, one at each end of its edge;
//! a joint has several, its sides bulging out to a ball.

use super::StrutsOptions;
use crate::{EdgeKey, FaceId, Mesh, VertexId};
use glam::Vec3;
use std::f32::consts::{PI, TAU};

/// One strut or joint, before it is added to a mesh.
pub(super) struct Piece {
    /// The rings, bottom to top, each as many points as the profile has
    /// sides, counter-clockwise seen from the top.
    pub(super) rings: Vec<Vec<Vec3>>,
    /// Whether every edge of the profile's corners and between the rings
    /// is hard (a square strut's).
    pub(super) hard_corners: bool,
    /// Whether the edges round the two ends are hard (a strut's).
    pub(super) hard_ends: bool,
    /// The polygon whose material it takes.
    pub(super) material_from: FaceId,
}

/// What a polygon of a piece is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Part {
    /// The polygon closing the bottom ring (its corners the ring's,
    /// backwards).
    Bottom,
    /// The quad between ring `.0` and the next, from point `.1` to the
    /// next round.
    Band(usize, usize),
    /// The polygon closing the top ring.
    Top,
}

/// A piece as added to a mesh: its points, ring by ring, and its
/// polygons.
pub(super) struct Made {
    /// Each ring's points.
    pub(super) rings: Vec<Vec<VertexId>>,
    /// Each polygon, and what it is.
    pub(super) faces: Vec<(FaceId, Part)>,
}

impl Made {
    /// The edges to mark hard, as `piece` says.
    pub(super) fn hard_edges(&self, piece: &Piece) -> Vec<(VertexId, VertexId)> {
        let mut out = Vec::new();
        let last = self.rings.len().saturating_sub(1);
        for (k, ring) in self.rings.iter().enumerate() {
            let n = ring.len();
            for j in 0..n {
                let end = k == 0 || k == last;
                if piece.hard_corners || (piece.hard_ends && end) {
                    out.push((ring[j], ring[(j + 1) % n]));
                }
                if piece.hard_corners
                    && let Some(next) = self.rings.get(k + 1)
                {
                    out.push((ring[j], next[j]));
                }
            }
        }
        out
    }
}

impl Piece {
    /// Adds the piece's points and polygons to `mesh`. `None` if the mesh
    /// is full.
    pub(super) fn add_to(&self, mesh: &mut Mesh) -> Option<Made> {
        let mut rings: Vec<Vec<VertexId>> = Vec::with_capacity(self.rings.len());
        for ring in &self.rings {
            let ids: Option<Vec<VertexId>> =
                ring.iter().map(|p| mesh.add_vertex(*p).ok()).collect();
            rings.push(ids?);
        }
        let mut faces = Vec::new();
        let (first, last) = (rings.first()?, rings.last()?);
        let bottom: Vec<VertexId> = first.iter().rev().copied().collect();
        faces.push((mesh.add_face(&bottom).ok()?, Part::Bottom));
        for k in 0..rings.len() - 1 {
            let (low, high) = (&rings[k], &rings[k + 1]);
            let n = low.len();
            for j in 0..n {
                let next = (j + 1) % n;
                let quad = [low[j], low[next], high[next], high[j]];
                faces.push((mesh.add_face(&quad).ok()?, Part::Band(k, j)));
            }
        }
        faces.push((mesh.add_face(last).ok()?, Part::Top));
        Some(Made { rings, faces })
    }
}

/// A direction square to `d` (unit length), the same for the same `d`.
fn any_square_to(d: Vec3) -> Vec3 {
    let a = d.abs();
    let other = if a.x <= a.y && a.x <= a.z {
        Vec3::X
    } else if a.y <= a.z {
        Vec3::Y
    } else {
        Vec3::Z
    };
    d.cross(other).try_normalize().unwrap_or(Vec3::X)
}

/// `towards` made square to the unit direction `axis` and unit length,
/// or any direction square to `axis` when it runs along it.
fn square_to(axis: Vec3, towards: Vec3) -> Vec3 {
    let flat = towards - axis * towards.dot(axis);
    if flat.length_squared() > 1e-12 {
        flat.normalize()
    } else {
        any_square_to(axis)
    }
}

/// A regular polygon of `sides` corners round the origin in the plane of
/// `across` and `up` (unit, square to each other), counter-clockwise from
/// `across` to `up`, its corners `radius` away, with a flat facing
/// `across`.
fn profile(sides: u32, radius: f32, across: Vec3, up: Vec3) -> Vec<Vec3> {
    (0..sides)
        .map(|j| {
            let angle = PI / sides as f32 + TAU * j as f32 / sides as f32;
            (across * angle.cos() + up * angle.sin()) * radius
        })
        .collect()
}

/// The strut along `edge`, or `None` if it has no length (or its points
/// aren't there).
pub(super) fn strut(input: &Mesh, edge: EdgeKey, options: &StrutsOptions) -> Option<Piece> {
    let (a, b) = edge.ends();
    let (pa, pb) = (input.position(a)?, input.position(b)?);
    let along = pb - pa;
    if !along.is_finite() || along.length_squared() < 1e-12 {
        return None;
    }
    let along = along.normalize();
    let topology = input.topology();
    let first = topology.edge_faces(edge).min()?;
    let up = square_to(along, input.face_normal(first));
    let side = along.cross(up);
    let (_, corner) = options.radii();
    let ring = profile(options.sides, corner, up, side);
    Some(Piece {
        rings: vec![
            ring.iter().map(|p| pa + *p).collect(),
            ring.iter().map(|p| pb + *p).collect(),
        ],
        hard_corners: options.sides <= StrutsOptions::HARD_UP_TO,
        hard_ends: true,
        material_from: first,
    })
}

/// The joint round `point`, or `None` if no polygon uses it.
///
/// A faceted ball whose every face is the same distance from the point:
/// the struts' corner radius, so it covers their ends. Its rings lie
/// along the facing of the point's first polygon (the lowest id), and one
/// flat faces along the point's first edge, so a square strut along it
/// meets a face square on, and a box's corners get joints square with
/// the box.
pub(super) fn joint(input: &Mesh, point: VertexId, options: &StrutsOptions) -> Option<Piece> {
    let centre = input.position(point)?;
    if !centre.is_finite() {
        return None;
    }
    let topology = input.topology();
    let material_from = topology.vertex_faces(point).iter().copied().min()?;
    let axis = input.face_normal(material_from);
    let first = topology
        .vertex_edges(point)
        .iter()
        .filter_map(|e| {
            let (a, b) = e.ends();
            let other = if a == point { b } else { a };
            let d = input.position(other)? - centre;
            let flat = d - axis * d.dot(axis);
            (flat.length_squared() > 1e-12).then_some(flat)
        })
        .next()
        .unwrap_or_else(|| any_square_to(axis));
    let across = square_to(axis, first);
    let up = axis.cross(across);
    let (_, size) = options.radii();
    let sides = options.sides;
    let rings = options.joint_rings();
    // The ring polygon's and the up-and-down outline's corners, so their
    // flats lie at 1.
    let round = 1.0 / (PI / sides as f32).cos();
    let over = 1.0 / (PI / (2 * rings) as f32).cos();
    let unit = profile(sides, round, across, up);
    let ring_list = (0..rings)
        .map(|k| {
            let angle = -PI / 2.0 + PI * (k as f32 + 0.5) / rings as f32;
            let (height, out) = (over * angle.sin(), over * angle.cos());
            unit.iter()
                .map(|p| centre + (*p * out + axis * height) * size)
                .collect()
        })
        .collect();
    let hard = sides <= StrutsOptions::HARD_UP_TO;
    Some(Piece {
        rings: ring_list,
        hard_corners: hard,
        hard_ends: hard,
        material_from,
    })
}
