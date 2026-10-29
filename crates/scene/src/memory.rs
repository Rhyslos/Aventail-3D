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
use crate::object::{Object, ObjectId, ObjectKind};
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
        ObjectKind::Group | ObjectKind::Primitive(_) | ObjectKind::Instance(_) => 0,
        ObjectKind::Curve(c) => match &c.shape {
            crate::CurveShape::Points(curve) => std::mem::size_of_val(curve.points()),
            crate::CurveShape::Ready(_) => 0,
        },
        ObjectKind::Lattice(l) => std::mem::size_of_val(l.points()),
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
        }
        | Operation::WeldByDistance {
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
        Operation::FlipPolygons { faces, anchors }
        | Operation::SetMaterial { faces, anchors, .. }
        | Operation::Unwrap { faces, anchors, .. }
        | Operation::StraightenUvs { faces, anchors }
        | Operation::RelaxUvs { faces, anchors, .. }
        | Operation::SetTexelDensity { faces, anchors, .. }
        | Operation::SetStretchBudget { faces, anchors, .. }
        | Operation::SetUvLock { faces, anchors, .. }
        | Operation::SetPieceSettings { faces, anchors, .. }
        | Operation::PlaceOnTrim { faces, anchors, .. }
        | Operation::Quadify { faces, anchors, .. } => {
            std::mem::size_of_val(faces.as_slice())
                + anchors.as_ref().map_or(0, mesh::Anchors::memory_size)
        }
        Operation::TransformUvs {
            faces,
            points,
            anchors,
            ..
        }
        | Operation::SetUvPins {
            faces,
            points,
            anchors,
            ..
        } => {
            std::mem::size_of_val(faces.as_slice())
                + std::mem::size_of_val(points.as_slice())
                + anchors.as_ref().map_or(0, mesh::Anchors::memory_size)
        }
        Operation::PackUvs { islands } => islands
            .iter()
            .map(|(faces, _)| std::mem::size_of_val(faces.as_slice()) + size_of::<glam::Affine2>())
            .sum(),
        Operation::RestoreUvs { layout } => layout.memory_size(),
        Operation::FixFacing
        | Operation::PlaceShape { .. }
        | Operation::HardEdgesByAngle { .. } => 0,
        Operation::PaintColours { corners } => std::mem::size_of_val(corners.as_slice()),
        Operation::SlidePoints { slides, anchors } => {
            std::mem::size_of_val(slides.as_slice())
                + anchors.as_ref().map_or(0, mesh::Anchors::memory_size)
        }
        Operation::SetHardEdges { edges, anchors, .. }
        | Operation::SetCreases { edges, anchors, .. }
        | Operation::SetBevelWeights { edges, anchors, .. }
        | Operation::SetSeams { edges, anchors, .. }
        | Operation::SetSeamPreference { edges, anchors, .. } => {
            std::mem::size_of_val(edges.as_slice())
                + anchors.as_ref().map_or(0, mesh::Anchors::memory_size)
        }
        Operation::Slit { line } => std::mem::size_of_val(line.points.as_slice()),
        Operation::Mirrored { op, .. } => operation_size(op),
        Operation::WeldMiddle { .. } => 0,
        Operation::LoopCut { anchors, .. } => {
            anchors.as_ref().map_or(0, mesh::Anchors::memory_size)
        }
        Operation::ExtrudeEdges { edges, anchors, .. }
        | Operation::BevelEdges { edges, anchors, .. }
        | Operation::Groove { edges, anchors, .. } => {
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
            Edit::Relocate { .. }
            | Edit::SetVisible { .. }
            | Edit::SetTransform { .. }
            | Edit::SetOffset { .. }
            | Edit::SetUnits { .. }
            | Edit::SetKitGrid { .. }
            | Edit::SetTexture { .. }
            | Edit::SetMending { .. }
            | Edit::SetCollision { .. }
            | Edit::SetCollisionParts { .. }
            | Edit::SetOutline { .. }
            | Edit::SetLods { .. }
            | Edit::SetHardness { .. }
            | Edit::SetMirror { .. } => 0,
            Edit::SetBooleans { before, after, .. } => [before, after]
                .iter()
                .flat_map(|list| list.iter())
                .map(|b| 16 + b.cutters.len() * std::mem::size_of::<ObjectId>())
                .sum(),
            Edit::SetModifiers { before, after, .. } => {
                (before.len() + after.len()) * std::mem::size_of::<crate::Modifier>()
            }
            Edit::SetExportTarget { before, after } => [before, after]
                .iter()
                .filter_map(|t| t.as_ref())
                .map(|t| t.folder.len())
                .sum(),
            Edit::SetSnapshots { before, after } => {
                // Layouts are shared: only those one side has cost memory.
                let only = |a: &[crate::UvSnapshot], b: &[crate::UvSnapshot]| -> usize {
                    a.iter()
                        .flat_map(|s| s.objects.iter())
                        .filter(|(_, l)| {
                            !b.iter()
                                .flat_map(|s| s.objects.iter())
                                .any(|(_, m)| std::sync::Arc::ptr_eq(l, m))
                        })
                        .map(|(_, l)| l.memory_size())
                        .sum()
                };
                [before, after]
                    .iter()
                    .flat_map(|list| list.iter())
                    .map(|s| s.name.len() + s.objects.len() * size_of::<(crate::ObjectId, usize)>())
                    .sum::<usize>()
                    + only(before, after)
                    + only(after, before)
            }
            Edit::SetTextureSet {
                material,
                before,
                after,
            } => material.len() + paint_change(before.as_deref(), after.as_deref()),
            Edit::SetPalettes { before, after } => before
                .iter()
                .chain(after)
                .map(|p| p.name.len() + p.colours.len() * 3)
                .sum(),
            Edit::SetTrim { before, after } => [before, after]
                .iter()
                .flat_map(|t| t.rows.iter())
                .map(|r| r.name.len() + std::mem::size_of::<crate::TrimRow>())
                .sum(),
            Edit::SetMaterials { before, after, .. } => {
                (before.len() + after.len()) * size_of::<crate::Material>()
                    + before
                        .iter()
                        .chain(after)
                        .map(|m| m.name.len())
                        .sum::<usize>()
            }
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

/// The memory a texture set change holds on to: the tiles and stroke
/// records one side has and the other doesn't (both are shared, so
/// unchanged ones are free), and the layers' names and settings. Counted
/// in one pass over each side (decision 0188): a 4K set of 20 layers has
/// tens of thousands of tiles.
fn paint_change(before: Option<&paint::TextureSet>, after: Option<&paint::TextureSet>) -> usize {
    // Mostly a stroke changed some pixels and the layers are laid out as
    // they were: then they're compared side by side, which is quicker.
    if let (Some(a), Some(b)) = (before, after)
        && let Some(size) = side_by_side(&a.layers, &b.layers)
    {
        return size;
    }
    let (old, new) = (PaintParts::of(before), PaintParts::of(after));
    old.only_in(&new) + new.only_in(&old) + old.layers + new.layers
}

/// [`paint_change`] for two lists of layers laid out alike (the same
/// kinds, groups and masks in the same places); `None` if they aren't.
fn side_by_side(a: &[paint::Layer], b: &[paint::Layer]) -> Option<usize> {
    if a.len() != b.len() {
        return None;
    }
    let mut total = 0;
    for (x, y) in a.iter().zip(b) {
        total += 2 * size_of::<paint::Layer>() + x.name.len() + y.name.len();
        total += match (&x.content, &y.content) {
            (paint::Content::Paint(i), paint::Content::Paint(j)) => tiles_apart(i, j)?,
            (paint::Content::Group(c), paint::Content::Group(d)) => side_by_side(c, d)?,
            (paint::Content::Fill(_), paint::Content::Fill(_)) => 0,
            _ => return None,
        };
        total += match (&x.mask, &y.mask) {
            (Some(m), Some(n)) => {
                tiles_apart(&m.image, &n.image)? + strokes_apart(&m.strokes, &n.strokes)
            }
            (None, None) => 0,
            _ => return None,
        };
        total += strokes_apart(&x.strokes, &y.strokes);
    }
    Some(total)
}

/// The bytes of the tiles of `a` and `b` that aren't shared, place by
/// place; `None` for images of different sizes.
fn tiles_apart(a: &paint::Image, b: &paint::Image) -> Option<usize> {
    if a.size() != b.size() {
        return None;
    }
    let tiles = a
        .tiles()
        .iter()
        .zip(b.tiles())
        .map(|(x, y)| match (x, y) {
            (Some(x), Some(y)) if std::sync::Arc::ptr_eq(x, y) => 0,
            (Some(_), Some(_)) => 2,
            (Some(_), None) | (None, Some(_)) => 1,
            (None, None) => 0,
        })
        .sum::<usize>();
    Some(tiles * paint::TILE_BYTES)
}

/// The bytes of the stroke records of `a` and `b` that aren't shared,
/// after those they share at the start (strokes are added at the end).
fn strokes_apart(
    a: &[std::sync::Arc<paint::StrokeRecord>],
    b: &[std::sync::Arc<paint::StrokeRecord>],
) -> usize {
    let shared = a
        .iter()
        .zip(b)
        .take_while(|(x, y)| std::sync::Arc::ptr_eq(x, y))
        .count();
    a[shared..]
        .iter()
        .chain(&b[shared..])
        .map(|s| stroke_size(s))
        .sum()
}

/// What a stroke record holds.
fn stroke_size(stroke: &paint::StrokeRecord) -> usize {
    size_of::<paint::StrokeRecord>()
        + size_of_val(stroke.dabs.as_slice())
        + size_of_val(stroke.fills.as_slice())
}

/// What a texture set holds, by the address of each shared part.
#[derive(Default)]
struct PaintParts {
    /// Tiles (layers' and masks'), each `TILE_BYTES`.
    tiles: std::collections::HashSet<usize>,
    /// Stroke records and their sizes.
    strokes: std::collections::HashMap<usize, usize>,
    /// The layers' own sizes and names.
    layers: usize,
}

impl PaintParts {
    fn of(set: Option<&paint::TextureSet>) -> Self {
        let mut parts = Self::default();
        if let Some(set) = set {
            parts.add(&set.layers);
        }
        parts
    }

    fn add(&mut self, layers: &[paint::Layer]) {
        for layer in layers {
            self.layers += size_of::<paint::Layer>() + layer.name.len();
            match &layer.content {
                paint::Content::Paint(image) => self.add_image(image),
                paint::Content::Group(children) => self.add(children),
                paint::Content::Fill(_) => {}
            }
            self.add_strokes(&layer.strokes);
            if let Some(mask) = &layer.mask {
                self.add_image(&mask.image);
                self.add_strokes(&mask.strokes);
            }
        }
    }

    fn add_image(&mut self, image: &paint::Image) {
        for tile in image.tiles().iter().flatten() {
            self.tiles.insert(std::sync::Arc::as_ptr(tile) as usize);
        }
    }

    fn add_strokes(&mut self, strokes: &[std::sync::Arc<paint::StrokeRecord>]) {
        for stroke in strokes {
            self.strokes
                .insert(std::sync::Arc::as_ptr(stroke) as usize, stroke_size(stroke));
        }
    }

    /// The bytes of the shared parts this has and `other` doesn't.
    fn only_in(&self, other: &Self) -> usize {
        let tiles = self.tiles.difference(&other.tiles).count() * paint::TILE_BYTES;
        let strokes: usize = self
            .strokes
            .iter()
            .filter(|(at, _)| !other.strokes.contains_key(at))
            .map(|(_, size)| size)
            .sum();
        tiles + strokes
    }
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

    /// A texture set change from `before` to `after`.
    fn paint_edit(before: &paint::TextureSet, after: &paint::TextureSet) -> usize {
        edit(&Edit::SetTextureSet {
            material: "M".into(),
            before: Some(Arc::new(before.clone())),
            after: Some(Arc::new(after.clone())),
        })
    }

    /// A set of `size` pixels with one paint layer painted all over.
    fn painted(size: u32) -> paint::TextureSet {
        let mut set = paint::TextureSet::new(size).unwrap();
        let path = set.add_paint_layer("Paint", None).unwrap();
        let image = set.layer_mut(&path).unwrap().image_mut().unwrap();
        for index in 0..image.tile_count() {
            image.set_tile(index, Some(Arc::new(vec![9; paint::TILE_BYTES])));
        }
        set
    }

    #[test]
    fn a_texture_set_change_counts_only_the_tiles_and_strokes_it_changed() {
        let before = painted(256);
        let unchanged = paint_edit(&before, &before);
        // Only the layers' own bookkeeping (a few hundred bytes each, more
        // since the realism channels, decision 0207), far under a tile.
        assert!(unchanged < 2_000, "shared tiles are free: {unchanged}");
        // One tile repainted: the old one and the new one are kept.
        let mut after = before.clone();
        let image = after.layers[0].image_mut().unwrap();
        image.set_tile(3, Some(Arc::new(vec![1; paint::TILE_BYTES])));
        assert_eq!(
            paint_edit(&before, &after) - unchanged,
            2 * paint::TILE_BYTES
        );
        // A stroke record: its dabs count once.
        let mut stroked = before.clone();
        stroked.layers[0]
            .strokes
            .push(Arc::new(paint::StrokeRecord {
                kind: paint::BrushKind::Paint,
                color: [0; 4],
                hardness: 1.0,
                flow: 1.0,
                through: false,
                dabs: vec![paint::DabRecord::flat(glam::Vec2::ZERO, 0.1, 1.0); 100],
                fills: Vec::new(),
                tip: paint::Tip::Round,
                alignment: Default::default(),
            }));
        let stroke = paint_edit(&before, &stroked) - unchanged;
        assert!(stroke >= 100 * size_of::<paint::DabRecord>(), "{stroke}");
    }

    #[test]
    fn tiles_in_groups_and_masks_count_too() {
        let before = painted(128);
        let mut after = before.clone();
        // Grouped: the same tiles, so only the layers' sizes change.
        after.group_layer(&[0], "Group").unwrap();
        let grouped = paint_edit(&before, &after);
        assert!(grouped < 4 * paint::TILE_BYTES, "{grouped}");
        // A mask painted inside the group.
        let mut masked = after.clone();
        let layer = masked.layer_mut(&[0, 0]).unwrap();
        let mut mask = paint::Mask::new(128, true).unwrap();
        mask.image
            .set_tile(0, Some(Arc::new(vec![5; paint::TILE_BYTES])));
        layer.mask = Some(mask);
        assert_eq!(
            paint_edit(&after, &masked) - paint_edit(&after, &after),
            paint::TILE_BYTES
        );
    }

    #[test]
    fn side_by_side_and_by_address_count_alike() {
        let before = painted(256);
        let mut after = before.clone();
        let image = after.layers[0].image_mut().unwrap();
        image.set_tile(3, Some(Arc::new(vec![1; paint::TILE_BYTES])));
        image.set_tile(4, None);
        let quick = side_by_side(&before.layers, &after.layers).unwrap();
        let (old, new) = (PaintParts::of(Some(&before)), PaintParts::of(Some(&after)));
        assert_eq!(
            quick,
            old.only_in(&new) + new.only_in(&old) + old.layers + new.layers
        );
        // Laid out differently: counted by address.
        let mut more = after.clone();
        more.layers.push(paint::Layer::fill("Fill", [1, 2, 3, 4]));
        assert!(side_by_side(&after.layers, &more.layers).is_none());
    }

    #[test]
    fn a_big_texture_set_is_counted_quickly() {
        // 4096 px: 4096 tiles a layer, twenty layers.
        let mut set = paint::TextureSet::new(4096).unwrap();
        for n in 0..20 {
            let path = set.add_paint_layer(&format!("L{n}"), None).unwrap();
            let image = set.layer_mut(&path).unwrap().image_mut().unwrap();
            let tile = Arc::new(vec![n as u8; paint::TILE_BYTES]);
            for index in 0..image.tile_count() {
                image.set_tile(index, Some(Arc::clone(&tile)));
            }
        }
        let mut after = set.clone();
        after.layers[0]
            .image_mut()
            .unwrap()
            .set_tile(0, Some(Arc::new(vec![0; paint::TILE_BYTES])));
        let start = std::time::Instant::now();
        let size = paint_edit(&set, &after);
        assert!(size >= paint::TILE_BYTES);
        // Checking every tile against every other took many seconds;
        // counting by address takes a moment, even in a debug build.
        assert!(start.elapsed().as_secs_f32() < 2.0, "{:?}", start.elapsed());
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
