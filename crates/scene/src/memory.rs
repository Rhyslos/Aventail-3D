//! Rough sizes in memory, so the undo history can stay within a budget
//! (DESIGN.md §4.3).
//!
//! These are estimates: they count what each piece holds (names, meshes,
//! recorded operations, and a mesh's topology, built or estimated) and the
//! fixed size of its parts, but not the allocator's own overheads. A mesh
//! shared between the scene and the history is counted in full every time
//! a step holds it, so the history is, if anything, over-counted: it never
//! takes more memory than the budget allows.

use crate::history::Step;
use crate::object::{Object, ObjectKind};
use crate::stack::{Entry, Layer, Operation, Stack};
use crate::tree::Edit;
use std::mem::{size_of, size_of_val};

/// A string's size: what it has room for.
fn text(s: &str) -> usize {
    s.len()
}

/// What an object kind holds beyond its own fixed size.
fn kind(kind: &ObjectKind) -> usize {
    match kind {
        ObjectKind::Mesh(mesh) => mesh.memory_size(),
        ObjectKind::Group | ObjectKind::Primitive(_) => 0,
    }
}

/// What an operation holds beyond its own fixed size.
pub(crate) fn operation_size(op: &Operation) -> usize {
    match op {
        Operation::MovePoints {
            points, anchors, ..
        }
        | Operation::TransformPoints {
            points, anchors, ..
        }
        | Operation::BevelPoints {
            points, anchors, ..
        } => {
            std::mem::size_of_val(points.as_slice())
                + anchors.as_ref().map_or(0, mesh::Anchors::memory_size)
        }
        Operation::ExtrudePolygons { faces, anchors, .. }
        | Operation::ExtrudeBent { faces, anchors, .. }
        | Operation::InsetPolygons { faces, anchors, .. } => {
            std::mem::size_of_val(faces.as_slice())
                + anchors.as_ref().map_or(0, mesh::Anchors::memory_size)
        }
        Operation::Connect {
            points,
            edges,
            anchors,
            ..
        } => {
            std::mem::size_of_val(points.as_slice())
                + std::mem::size_of_val(edges.as_slice())
                + anchors.as_ref().map_or(0, mesh::Anchors::memory_size)
        }
        Operation::Bridge {
            edges,
            faces,
            anchors,
            ..
        } => {
            std::mem::size_of_val(edges.as_slice())
                + std::mem::size_of_val(faces.as_slice())
                + anchors.as_ref().map_or(0, mesh::Anchors::memory_size)
        }
        Operation::Weld {
            points, anchors, ..
        } => {
            std::mem::size_of_val(points.as_slice())
                + anchors.as_ref().map_or(0, mesh::Anchors::memory_size)
        }
        Operation::DeleteElements {
            points,
            edges,
            faces,
            anchors,
        }
        | Operation::Dissolve {
            points,
            edges,
            faces,
            anchors,
        } => {
            std::mem::size_of_val(points.as_slice())
                + std::mem::size_of_val(edges.as_slice())
                + std::mem::size_of_val(faces.as_slice())
                + anchors.as_ref().map_or(0, mesh::Anchors::memory_size)
        }
        Operation::FillHoles { points, anchors } => {
            std::mem::size_of_val(points.as_slice())
                + anchors.as_ref().map_or(0, mesh::Anchors::memory_size)
        }
        Operation::FlipPolygons { faces, anchors } => {
            std::mem::size_of_val(faces.as_slice())
                + anchors.as_ref().map_or(0, mesh::Anchors::memory_size)
        }
        Operation::FixFacing | Operation::HardEdgesByAngle { .. } => 0,
        Operation::SetHardEdges { edges, anchors, .. } => {
            std::mem::size_of_val(edges.as_slice())
                + anchors.as_ref().map_or(0, mesh::Anchors::memory_size)
        }
        Operation::Slit { line } => std::mem::size_of_val(line.points.as_slice()),
        Operation::LoopCut { anchors, .. } => {
            anchors.as_ref().map_or(0, mesh::Anchors::memory_size)
        }
        Operation::ExtrudeEdges { edges, anchors, .. }
        | Operation::BevelEdges { edges, anchors, .. } => {
            std::mem::size_of_val(edges.as_slice())
                + anchors.as_ref().map_or(0, mesh::Anchors::memory_size)
        }
    }
}

/// What a layer holds beyond its own fixed size.
fn layer(layer: &Layer) -> usize {
    text(&layer.name)
        + layer
            .entries
            .iter()
            .map(|e| size_of::<Entry>() + operation_size(&e.operation))
            .sum::<usize>()
}

/// What a stack holds beyond its own fixed size.
fn stack(stack: &Stack) -> usize {
    stack
        .layers()
        .iter()
        .map(|l| size_of::<Layer>() + layer(l))
        .sum()
}

/// What an object holds beyond its own fixed size.
fn object(object: &Object) -> usize {
    text(&object.name) + kind(&object.kind) + stack(&object.stack)
}

/// Roughly how many bytes one edit takes up.
pub(crate) fn edit(edit: &Edit) -> usize {
    size_of::<Edit>()
        + match edit {
            Edit::Insert { object: o, .. } | Edit::Remove { object: o, .. } => object(o),
            Edit::Relocate { .. } | Edit::SetVisible { .. } | Edit::SetTransform { .. } => 0,
            Edit::Rename { before, after, .. } => text(before) + text(after),
            Edit::SetKind { before, after, .. } => kind(before) + kind(after),
            Edit::InsertLayer { layer: l, .. } | Edit::RemoveLayer { layer: l, .. } => layer(l),
            Edit::InsertEntry { entry, .. } | Edit::RemoveEntry { entry, .. } => {
                operation_size(&entry.operation)
            }
        }
}

/// Roughly how many bytes one undo step takes up.
pub(crate) fn step(step: &Step) -> usize {
    size_of::<Step>()
        + text(&step.label)
        + step.edits.iter().map(edit).sum::<usize>()
        + step
            .selections
            .iter()
            .map(|s| size_of_val(s) + s.before.memory_size() + s.after.memory_size())
            .sum::<usize>()
        + step.objects.as_ref().map_or(0, |o| {
            size_of_val(o.before.as_slice()) + size_of_val(o.after.as_slice())
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::ObjectId;
    use crate::tree::Place;
    use mesh::{Primitive, PrimitiveKind, VertexId};
    use std::sync::Arc;

    fn insert(kind: ObjectKind) -> Edit {
        Edit::Insert {
            id: ObjectId(1),
            object: Object::new(kind),
            place: Place {
                parent: None,
                index: 0,
            },
        }
    }

    #[test]
    fn small_edits_are_small_and_meshes_count_in_full() {
        let group = edit(&insert(ObjectKind::Group));
        assert!(group < 1_000, "{group}");
        let cube = edit(&insert(ObjectKind::Primitive(Primitive::Cube(
            Default::default(),
        ))));
        assert!(cube < 1_000, "settings are small: {cube}");
        let sphere = Arc::new(PrimitiveKind::Sphere.standard().mesh());
        let mesh = edit(&insert(ObjectKind::Mesh(Arc::clone(&sphere))));
        assert!(mesh >= sphere.memory_size());
    }

    #[test]
    fn longer_names_and_more_points_take_more() {
        let short = edit(&Edit::Rename {
            id: ObjectId(1),
            before: "A".into(),
            after: "B".into(),
        });
        let long = edit(&Edit::Rename {
            id: ObjectId(1),
            before: "A".into(),
            after: "B".repeat(500),
        });
        assert_eq!(long - short, 499);
        let points = |n: u32| Edit::InsertEntry {
            id: ObjectId(1),
            layer: 0,
            index: 0,
            entry: Entry::new(
                crate::stack::EditId::from_raw(1),
                Operation::MovePoints {
                    points: (0..n).map(VertexId::from_raw).collect(),
                    offset: glam::Vec3::X,
                    anchors: None,
                },
            ),
        };
        assert_eq!(edit(&points(100)) - edit(&points(0)), 400);
    }

    #[test]
    fn a_step_counts_its_label_and_edits() {
        let edits = vec![insert(ObjectKind::Group), insert(ObjectKind::Group)];
        let total = step(&Step {
            label: "Create group".into(),
            edits: edits.clone(),
            selections: Vec::new(),
            objects: None,
            merge_key: None,
        });
        let parts: usize = edits.iter().map(edit).sum();
        assert_eq!(total, size_of::<Step>() + "Create group".len() + parts);
    }
}
