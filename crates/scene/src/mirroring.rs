//! Symmetry for every modelling edit
//! ([decision 0059](../../../docs/decisions/0059-symmetry-for-every-edit.md)).
//!
//! An edit made with symmetry on is recorded as [`Operation::Mirrored`]:
//! the edit as it was made, and the mirror planes. When the shape is worked
//! out, the edit's mirror is found on the shape as it is just before the
//! edit, by position ([`mesh::symmetry::Partners`]), in one of two ways:
//!
//! - **Together** (extrude, inset, bevel, delete, dissolve, fill hole,
//!   flip, hard and soft edges): the mirrored elements join the edit's
//!   own, and it is made once on them all. Neighbours across the middle
//!   then move as one piece, as they would if selected by hand.
//! - **As a copy** (loop cut, slit, connect, bridge, weld, bent extrude):
//!   the edit is made, then a mirrored copy of it, each with its own
//!   elements. A copy that would be the edit itself (a loop cut whose ring
//!   crosses the middle, a selection that is its own mirror) is left out.
//!
//! The edit keeps its settings in one place, so the Last edit panel
//! changes both sides at once. Elements without a mirror (where the shape
//! isn't symmetrical) are found again by their mirrored anchors, if the
//! shape has anything there, and are otherwise skipped.

use crate::stack::Operation;
use mesh::edit::{Eye, SlitLine};
use mesh::symmetry::{Mirror, Partners};
use mesh::{EdgeKey, Element, ElementKind, FaceId, Mesh, VertexId};
use std::collections::BTreeSet;

/// How an edit is made on both sides.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Both {
    /// One edit on the elements of both sides.
    Together(Operation),
    /// The edit, then these mirrored copies of it.
    Copies(Vec<Operation>),
}

/// Whether symmetry can be used with `op` at all: the modelling edits. Moves
/// mirror themselves while dragging, and whole-shape edits need nothing.
pub fn mirrors(op: &Operation) -> bool {
    match op {
        Operation::ExtrudePolygons { .. }
        | Operation::ExtrudeBent { .. }
        | Operation::InsetPolygons { .. }
        | Operation::ExtrudeEdges { .. }
        | Operation::BevelEdges { .. }
        | Operation::Groove { .. }
        | Operation::BevelPoints { .. }
        | Operation::LoopCut { .. }
        | Operation::Connect { .. }
        | Operation::Bridge { .. }
        | Operation::Weld { .. }
        | Operation::DeleteElements { .. }
        | Operation::Dissolve { .. }
        | Operation::FlipPolygons { .. }
        | Operation::SetMaterial { .. }
        | Operation::SetHardEdges { .. }
        | Operation::SetSeams { .. }
        | Operation::SetSeamPreference { .. }
        | Operation::Slit { .. } => true,
        Operation::FillHoles { points, .. } => !points.is_empty(),
        Operation::MovePoints { .. }
        | Operation::TransformPoints { .. }
        // UVs aren't mirrored in the texture: each half has its own.
        | Operation::TransformUvs { .. }
        | Operation::SetUvPins { .. }
        | Operation::Unwrap { .. }
        | Operation::StraightenUvs { .. }
        | Operation::RelaxUvs { .. }
        | Operation::SetTexelDensity { .. }
        | Operation::SetStretchBudget { .. }
        | Operation::PlaceOnTrim { .. }
        | Operation::PackUvs { .. }
        | Operation::SlidePoints { .. }
        | Operation::FixFacing
        | Operation::PlaceShape { .. }
        | Operation::HardEdgesByAngle { .. }
        | Operation::Mirrored { .. }
        | Operation::WeldMiddle { .. } => false,
    }
}

/// Each element of a list, then its mirrors not already in it.
fn with_mirrors<T: Copy + Ord>(own: &[T], mirrors: impl IntoIterator<Item = T>) -> Vec<T> {
    let mut seen: BTreeSet<T> = own.iter().copied().collect();
    let mut out = own.to_vec();
    for m in mirrors {
        if seen.insert(m) {
            out.push(m);
        }
    }
    out
}

/// Whether two lists hold the same elements, in any order.
fn same<T: Copy + Ord>(a: &[T], b: &[T]) -> bool {
    a.iter().copied().collect::<BTreeSet<T>>() == b.iter().copied().collect::<BTreeSet<T>>()
}

/// One mirror's view of an edit's elements.
struct Side<'a> {
    partners: Partners<'a>,
    mirror: Mirror,
    reflect: [bool; 3],
}

impl Side<'_> {
    /// Each point's mirror; where the shape has none, the point itself
    /// (the mirrored anchors then find what is there, if anything).
    fn points(&self, points: &[VertexId]) -> Vec<VertexId> {
        points
            .iter()
            .map(|v| self.partners.point(*v).unwrap_or(*v))
            .collect()
    }

    fn edges(&self, edges: &[EdgeKey]) -> Vec<EdgeKey> {
        edges
            .iter()
            .map(|e| self.partners.edge(*e).unwrap_or(*e))
            .collect()
    }

    fn faces(&self, faces: &[FaceId]) -> Vec<FaceId> {
        faces
            .iter()
            .map(|f| self.partners.face(*f).unwrap_or(*f))
            .collect()
    }

    fn anchors(&self, anchors: &Option<mesh::Anchors>) -> Option<mesh::Anchors> {
        anchors.as_ref().map(|a| {
            a.mirrored(&self.mirror, self.reflect, |v| {
                self.partners.point(v).unwrap_or(v)
            })
        })
    }
}

/// How `op`, made on `mesh` with symmetry across `mirror`, is made on both
/// sides.
pub(crate) fn both_sides(op: &Operation, mesh: &Mesh, mirror: &Mirror) -> Both {
    let sides: Vec<Side> = mirror
        .reflections()
        .into_iter()
        .map(|reflect| Side {
            partners: Partners::new(mesh, *mirror, reflect),
            mirror: *mirror,
            reflect,
        })
        .collect();
    // The anchors of the edit's own elements and all their mirrors.
    let joined = |anchors: &Option<mesh::Anchors>| {
        anchors.as_ref().map(|own| {
            sides
                .iter()
                .filter_map(|s| s.anchors(anchors))
                .fold(own.clone(), |all, m| all.joined(&m))
        })
    };
    let all_points = |p: &[VertexId]| with_mirrors(p, sides.iter().flat_map(|s| s.points(p)));
    let all_edges = |e: &[EdgeKey]| with_mirrors(e, sides.iter().flat_map(|s| s.edges(e)));
    let all_faces = |f: &[FaceId]| with_mirrors(f, sides.iter().flat_map(|s| s.faces(f)));
    let together = |op: Operation| Both::Together(op);
    match op {
        Operation::ExtrudePolygons {
            faces,
            distance,
            grouping,
            shape,
            stretch,
            anchors,
        } => together(Operation::ExtrudePolygons {
            faces: all_faces(faces),
            distance: *distance,
            grouping: *grouping,
            // Along an axis, each side goes out the way it faces, so the
            // mirrored side goes the mirrored way.
            shape: *shape,
            stretch: *stretch,
            anchors: joined(anchors),
        }),
        Operation::InsetPolygons {
            faces,
            distance,
            grouping,
            stretch,
            anchors,
        } => together(Operation::InsetPolygons {
            faces: all_faces(faces),
            distance: *distance,
            grouping: *grouping,
            stretch: *stretch,
            anchors: joined(anchors),
        }),
        Operation::ExtrudeEdges {
            edges,
            distance,
            grouping,
            stretch,
            anchors,
        } => together(Operation::ExtrudeEdges {
            edges: all_edges(edges),
            distance: *distance,
            grouping: *grouping,
            stretch: *stretch,
            anchors: joined(anchors),
        }),
        Operation::BevelEdges {
            edges,
            shape,
            stretch,
            anchors,
        } => together(Operation::BevelEdges {
            edges: all_edges(edges),
            shape: *shape,
            stretch: *stretch,
            anchors: joined(anchors),
        }),
        Operation::Groove {
            edges,
            shape,
            stretch,
            anchors,
        } => together(Operation::Groove {
            edges: all_edges(edges),
            shape: *shape,
            stretch: *stretch,
            anchors: joined(anchors),
        }),
        Operation::BevelPoints {
            points,
            width,
            stretch,
            anchors,
        } => together(Operation::BevelPoints {
            points: all_points(points),
            width: *width,
            stretch: *stretch,
            anchors: joined(anchors),
        }),
        Operation::DeleteElements {
            points,
            edges,
            faces,
            anchors,
        } => together(Operation::DeleteElements {
            points: all_points(points),
            edges: all_edges(edges),
            faces: all_faces(faces),
            anchors: joined(anchors),
        }),
        Operation::Dissolve {
            points,
            edges,
            faces,
            anchors,
        } => together(Operation::Dissolve {
            points: all_points(points),
            edges: all_edges(edges),
            faces: all_faces(faces),
            anchors: joined(anchors),
        }),
        Operation::FillHoles { points, anchors } => together(Operation::FillHoles {
            points: all_points(points),
            anchors: joined(anchors),
        }),
        Operation::FlipPolygons { faces, anchors } => together(Operation::FlipPolygons {
            faces: all_faces(faces),
            anchors: joined(anchors),
        }),
        Operation::SetMaterial {
            faces,
            slot,
            anchors,
        } => together(Operation::SetMaterial {
            faces: all_faces(faces),
            slot: *slot,
            anchors: joined(anchors),
        }),
        Operation::SetSeams {
            edges,
            seam,
            anchors,
        } => together(Operation::SetSeams {
            edges: all_edges(edges),
            seam: *seam,
            anchors: joined(anchors),
        }),
        Operation::SetSeamPreference {
            edges,
            preference,
            anchors,
        } => together(Operation::SetSeamPreference {
            edges: all_edges(edges),
            preference: *preference,
            anchors: joined(anchors),
        }),
        Operation::SetHardEdges {
            edges,
            hard,
            anchors,
        } => together(Operation::SetHardEdges {
            edges: all_edges(edges),
            hard: *hard,
            anchors: joined(anchors),
        }),
        Operation::ExtrudeBent {
            faces,
            distance,
            grouping,
            bend,
            stretch,
            anchors,
        } => {
            // The pipe turns the mirrored way, as seen on a stretched
            // object (a facing is stretched the opposite way to points).
            let stretch = *stretch;
            let facing = faces
                .iter()
                .map(|f| mesh.face_normal(*f) / mesh::stretch::usable(stretch))
                .sum::<glam::Vec3>()
                .try_normalize();
            Both::Copies(
                sides
                    .iter()
                    .filter_map(|s| {
                        let mirrored = s.faces(faces);
                        if same(&mirrored, faces) {
                            return None;
                        }
                        let mut bend = *bend;
                        // A mirror turns a spiral the other way round
                        // (right-handed into left-handed).
                        bend.mirrored = !bend.mirrored;
                        if let Some(facing) = facing
                            && let Some(towards) =
                                mesh::extrude::bend_towards(facing, bend.direction)
                        {
                            bend.direction = mesh::extrude::bend_direction(
                                Mirror::vector(s.reflect, facing),
                                Mirror::vector(s.reflect, towards),
                            );
                        }
                        Some(Operation::ExtrudeBent {
                            faces: mirrored,
                            distance: *distance,
                            grouping: *grouping,
                            bend,
                            stretch,
                            anchors: s.anchors(anchors),
                        })
                    })
                    .collect(),
            )
        }
        Operation::LoopCut {
            edge,
            cuts,
            position,
            anchors,
        } => {
            let ring: BTreeSet<EdgeKey> = mesh::selection::ring_through(
                mesh,
                mesh.topology(),
                *edge,
                None,
                ElementKind::Edges,
            )
            .into_iter()
            .filter_map(|e| match e {
                Element::Edge(e) => Some(e),
                _ => None,
            })
            .chain([*edge])
            .collect();
            Both::Copies(
                sides
                    .iter()
                    .filter_map(|s| {
                        let mirrored = s.partners.edge(*edge)?;
                        // A ring crossing the middle is its own mirror.
                        if ring.contains(&mirrored) {
                            return None;
                        }
                        // Measured from the mirrored edge's lower point.
                        let low = s.partners.point(edge.ends().0)?;
                        let position = if mirrored.ends().0 == low {
                            *position
                        } else {
                            1.0 - *position
                        };
                        Some(Operation::LoopCut {
                            edge: mirrored,
                            cuts: *cuts,
                            position,
                            anchors: s.anchors(anchors),
                        })
                    })
                    .collect(),
            )
        }
        Operation::Connect {
            points,
            edges,
            cuts,
            anchors,
        } => Both::Copies(
            sides
                .iter()
                .filter_map(|s| {
                    let (p, e) = (s.points(points), s.edges(edges));
                    if same(&p, points) && same(&e, edges) {
                        return None;
                    }
                    Some(Operation::Connect {
                        points: p,
                        edges: e,
                        cuts: *cuts,
                        anchors: s.anchors(anchors),
                    })
                })
                .collect(),
        ),
        Operation::Bridge {
            edges,
            faces,
            segments,
            twist,
            anchors,
        } => Both::Copies(
            sides
                .iter()
                .filter_map(|s| {
                    let (e, f) = (s.edges(edges), s.faces(faces));
                    if same(&e, edges) && same(&f, faces) {
                        return None;
                    }
                    Some(Operation::Bridge {
                        edges: e,
                        faces: f,
                        segments: *segments,
                        twist: *twist,
                        anchors: s.anchors(anchors),
                    })
                })
                .collect(),
        ),
        Operation::Weld {
            points,
            target,
            anchors,
        } => Both::Copies(
            sides
                .iter()
                .filter_map(|s| {
                    let p = s.points(points);
                    let t = target.map(|t| s.partners.point(t).unwrap_or(t));
                    if same(&p, points) && t == *target {
                        return None;
                    }
                    Some(Operation::Weld {
                        points: p,
                        target: t,
                        anchors: s.anchors(anchors),
                    })
                })
                .collect(),
        ),
        Operation::Slit { line } => Both::Copies(
            sides
                .iter()
                .filter_map(|s| {
                    let points: Vec<glam::Vec3> = line
                        .points
                        .iter()
                        .map(|p| s.mirror.point(s.reflect, *p))
                        .collect();
                    // A line that is its own mirror (forwards or back).
                    let close = |a: &[glam::Vec3], b: &[glam::Vec3]| {
                        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.distance(*y) <= 1e-3)
                    };
                    let mut back = points.clone();
                    back.reverse();
                    if close(&points, &line.points) || close(&back, &line.points) {
                        return None;
                    }
                    let eye = match line.eye {
                        Eye::At(e) => Eye::At(s.mirror.point(s.reflect, e)),
                        Eye::Along(d) => Eye::Along(Mirror::vector(s.reflect, d)),
                    };
                    Some(Operation::Slit {
                        line: SlitLine {
                            points,
                            eye,
                            through: line.through,
                        },
                    })
                })
                .collect(),
        ),
        // Nothing to mirror: made once.
        _ => Both::Copies(Vec::new()),
    }
}
