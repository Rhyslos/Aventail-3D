//! The triangles the export makes, drawn over the view ([decision 0161](../../../../docs/decisions/0161-triangulate.md)):
//! with Display → Show triangles, every shown shape's polygons get faint
//! lines where the export's cut divides them into triangles
//! ([`mesh::triangulate::inner_lines`], the very cut the export and the
//! renderer use). Triangles have nothing to add, so a shape made of them
//! shows only its own edges.
//!
//! Each object's lines are kept while its drawn shape stays the same
//! shape (moving the object only moves them), and worked out again when
//! it changes.

use super::{GUIDES_UP_TO, OverlayLine};
use crate::drawing::{OverlayStyle, SceneDrawing};
use glam::Vec3;
use scene::ObjectId;
use std::collections::HashMap;
use std::sync::Arc;

/// Lines in an object's shape's space.
type Lines = Arc<Vec<[Vec3; 2]>>;

/// Each object's lines, with the shape they were made for.
#[derive(Debug, Default)]
pub(crate) struct TriangleLines {
    kept: HashMap<ObjectId, (Arc<scene::Mesh>, Lines)>,
}

impl TriangleLines {
    /// The lines to draw, in the world: every shown shape's, with `show`;
    /// none otherwise (and nothing is kept).
    pub(crate) fn lines(&mut self, drawing: &SceneDrawing, show: bool) -> Vec<OverlayLine> {
        if !show {
            self.kept.clear();
            return Vec::new();
        }
        let mut out = Vec::new();
        let mut seen = Vec::new();
        for item in &drawing.items {
            // A cutter goes out as part of what it cuts (decision 0135),
            // and a shape too heavy for guides is left out.
            if item.outline || !item.transform.is_finite() || item.drawn.face_count() > GUIDES_UP_TO
            {
                continue;
            }
            seen.push(item.id);
            let lines = self.lines_of(item.id, &item.drawn);
            let world = |p: Vec3| item.transform.transform_point3(p);
            out.extend(
                lines
                    .iter()
                    .map(|[a, b]| ([world(*a), world(*b)], OverlayStyle::Triangle)),
            );
        }
        self.kept.retain(|id, _| seen.contains(id));
        out
    }

    /// Object `id`'s lines for `shape`: kept, or worked out now.
    fn lines_of(&mut self, id: ObjectId, shape: &Arc<scene::Mesh>) -> Lines {
        if let Some((kept, lines)) = self.kept.get(&id)
            && Arc::ptr_eq(kept, shape)
        {
            return Arc::clone(lines);
        }
        let lines = Arc::new(mesh::triangulate::inner_lines(shape));
        self.kept
            .insert(id, (Arc::clone(shape), Arc::clone(&lines)));
        lines
    }
}
