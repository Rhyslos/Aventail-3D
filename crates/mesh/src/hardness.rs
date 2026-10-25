//! Hard and soft edges: marking where the shading breaks into a crisp
//! edge and where it flows smoothly ([decision 0053](../../../docs/decisions/0053-hard-and-soft-edges.md)).
//!
//! - [`set_hard_edges`] marks chosen edges hard or soft.
//! - [`hard_edges_by_angle`] marks every edge whose two polygons meet more
//!   sharply than an angle hard, and the rest soft, as many programs'
//!   *auto-smooth* does.
//! - [`sharp_edges`] lists those edges, without changing anything.
//! - [`harden_new_edges`] marks the sharp edges of polygons an edit just
//!   made hard, so new corners come out crisp.
//!
//! Hardness is only about shading ([`crate::shading`]): the shape itself
//! doesn't change. Edges on an open border have one polygon, so shading
//! never flows across them; they are left as they are.

use crate::edit::Outcome;
use crate::{EdgeKey, Mesh};

/// The largest angle accepted, in degrees: polygons can't meet more
/// sharply than folded flat onto each other.
pub const MAX_ANGLE: f32 = 180.0;

/// The angle, in degrees, from which edges count as sharp unless the user
/// says otherwise: Hard edges by angle starts here, and new edges an edit
/// makes are marked hard from here ([`harden_new_edges`]).
pub const DEFAULT_ANGLE: f32 = 30.0;

/// Marks hard the sharp edges (see [`sharp_edges`]) of the polygons from
/// slot `first_new` on: the polygons an edit just made. Nothing is made
/// soft. Bridge and Fill hole use it, so a band between two boxes keeps
/// crisp corners instead of smearing the shading round them. Returns how
/// many edges were marked.
pub fn harden_new_edges(mesh: &mut Mesh, first_new: usize, angle: f32) -> usize {
    let new_edges: std::collections::BTreeSet<EdgeKey> = mesh
        .face_slots()
        .skip(first_new)
        .flatten()
        .flat_map(|corners| {
            let n = corners.len();
            (0..n).map(move |i| EdgeKey::new(corners[i], corners[(i + 1) % n]))
        })
        .collect();
    if new_edges.is_empty() {
        return 0;
    }
    let cos = angle.clamp(0.0, MAX_ANGLE).to_radians().cos();
    let sharp: Vec<EdgeKey> = {
        let topology = mesh.topology();
        new_edges
            .into_iter()
            .filter(|e| match topology.edge_uses(*e) {
                [one, two] => {
                    let (n1, n2) = (mesh.face_normal(one.face), mesh.face_normal(two.face));
                    n1.length_squared() > 0.5
                        && n2.length_squared() > 0.5
                        && n1.dot(n2) < cos - 1e-6
                }
                _ => false,
            })
            .collect()
    };
    let mut marked = 0;
    for edge in sharp {
        let (a, b) = edge.ends();
        if !mesh.is_hard(a, b) {
            mesh.set_hard(a, b, true);
            marked += 1;
        }
    }
    marked
}

/// Marks `edges` hard (or soft, with `hard` false). Edges not in the mesh
/// are missing; edges already so are skipped.
pub fn set_hard_edges(mesh: &mut Mesh, edges: &[EdgeKey], hard: bool) -> Outcome {
    let mut outcome = Outcome::default();
    let topology = mesh.topology().clone();
    for edge in edges {
        let (a, b) = edge.ends();
        if !topology.has_edge(*edge) {
            outcome.missing += 1;
        } else if mesh.is_hard(a, b) == hard {
            outcome.skipped += 1;
        } else {
            mesh.set_hard(a, b, hard);
            outcome.matched += 1;
        }
    }
    outcome
}

/// The edges where `mesh`'s shading breaks under `hardness`: by angle,
/// the sharp edges ([`sharp_edges`]); by the marks, the marked hard edges
/// the shape still has (a mark left on a gone edge isn't listed). With
/// custom normals (decision 0148), the edges where they break the
/// shading ([`crate::custom_normals::split_edges`]), whatever `hardness`.
pub fn shading_hard_edges(mesh: &Mesh, hardness: crate::Hardness) -> Vec<EdgeKey> {
    if let Some(split) = crate::custom_normals::split_edges(mesh) {
        return split;
    }
    match hardness {
        crate::Hardness::ByAngle(angle) => sharp_edges(mesh, angle),
        crate::Hardness::Marked => {
            let topology = mesh.topology();
            mesh.hard_edges()
                .filter(|e| topology.has_edge(*e))
                .collect()
        }
    }
}

/// The edges whose polygons meet at more than `angle` degrees (how far
/// one polygon's facing turns from the other's: 0° is flat), and every
/// edge shared by three or more polygons. Border edges are never listed.
pub fn sharp_edges(mesh: &Mesh, angle: f32) -> Vec<EdgeKey> {
    let cos = angle.clamp(0.0, MAX_ANGLE).to_radians().cos();
    let topology = mesh.topology();
    let mut out: Vec<EdgeKey> = topology
        .edges()
        .filter(|e| {
            let uses = topology.edge_uses(*e);
            match uses {
                [_] | [] => false,
                [one, two] => {
                    let (n1, n2) = (mesh.face_normal(one.face), mesh.face_normal(two.face));
                    // A polygon with no area has no facing: leave it soft.
                    n1.length_squared() > 0.5
                        && n2.length_squared() > 0.5
                        && n1.dot(n2) < cos - 1e-6
                }
                _ => true,
            }
        })
        .collect();
    out.sort();
    out
}

/// Marks the edges sharper than `angle` degrees hard and every other
/// edge between two polygons soft (see [`sharp_edges`]). Returns the
/// edges changed as matched; nothing to change is skipped.
pub fn hard_edges_by_angle(mesh: &mut Mesh, angle: f32) -> Outcome {
    let sharp: std::collections::BTreeSet<EdgeKey> = sharp_edges(mesh, angle).into_iter().collect();
    let inner: Vec<EdgeKey> = {
        let topology = mesh.topology();
        topology
            .edges()
            .filter(|e| topology.edge_uses(*e).len() >= 2)
            .collect()
    };
    let mut changed = 0;
    for edge in inner {
        let (a, b) = edge.ends();
        let hard = sharp.contains(&edge);
        if mesh.is_hard(a, b) != hard {
            mesh.set_hard(a, b, hard);
            changed += 1;
        }
    }
    Outcome {
        matched: changed,
        missing: 0,
        skipped: usize::from(changed == 0),
    }
}

#[cfg(test)]
mod tests;
