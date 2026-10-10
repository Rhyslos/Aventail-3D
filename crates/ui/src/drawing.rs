//! What the viewport draws of the scene: each shown object's mesh, where
//! it is, and whether it is selected. Rebuilt only when the scene or the
//! selection changes, and a primitive's mesh only when its settings do.

use crate::viewport::{DEFAULT_FRAME, Frame};
use glam::Mat4;
use scene::{Bounds, Document, Mesh, ObjectId, ObjectKind};
use std::collections::HashMap;
use std::sync::Arc;

/// One object to draw.
#[derive(Debug, Clone, PartialEq)]
pub struct DrawItem {
    /// Which object it is.
    pub id: ObjectId,
    /// Its shape, in its own space.
    pub mesh: Arc<Mesh>,
    /// Where it is in the world.
    pub transform: Mat4,
    /// Whether it is selected.
    pub selected: bool,
}

/// Everything the viewport draws of the scene. `version` changes whenever
/// anything in it does, so a renderer can tell when to upload it again.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SceneDrawing {
    /// Goes up with every change.
    pub version: u64,
    /// The shown objects that have a shape, top to bottom.
    pub items: Vec<DrawItem>,
}

/// What the viewport needs to know about the scene each frame.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SceneSummary {
    /// What to draw.
    pub drawing: Arc<SceneDrawing>,
    /// The box around everything shown, if anything shown has a shape.
    pub all: Option<Bounds>,
    /// The box around the selection (and what is inside it), if any of it
    /// is shown and has a shape.
    pub selected: Option<Bounds>,
}

impl SceneSummary {
    /// What to fit in the view for `what`: the selection (or, with nothing
    /// selected, everything), everything, or the middle of the scene if
    /// nothing shown has a shape.
    pub fn bounds_for(&self, what: Frame) -> Bounds {
        let wanted = match what {
            Frame::All => self.all,
            Frame::Selected => self.selected.or(self.all),
        };
        wanted.unwrap_or(DEFAULT_FRAME)
    }
}

/// Keeps the summary up to date with the document, rebuilding as little
/// as possible.
#[derive(Debug, Default)]
pub(crate) struct DrawingCache {
    /// The scene revision and selection the summary was built for.
    built_for: Option<(u64, Vec<ObjectId>)>,
    /// Each primitive's settings and the mesh built from them.
    meshes: HashMap<ObjectId, (ObjectKind, Arc<Mesh>)>,
    summary: SceneSummary,
}

impl DrawingCache {
    /// The summary of `document`, rebuilt if the scene or selection has
    /// changed since last time.
    pub(crate) fn update(&mut self, document: &Document) -> &SceneSummary {
        let key = (document.revision(), document.selection());
        if self.built_for.as_ref() == Some(&key) {
            return &self.summary;
        }
        let scene = document.scene();
        let mut items = Vec::new();
        let mut meshes = HashMap::new();
        for (id, _) in scene.walk() {
            let Some(object) = scene.object(id) else {
                continue;
            };
            if !scene.is_shown(id) {
                continue;
            }
            let mesh = match self.meshes.remove(&id) {
                Some((kind, mesh)) if kind == object.kind => mesh,
                _ => match object.kind.mesh() {
                    Some(mesh) => Arc::new(mesh),
                    None => continue,
                },
            };
            meshes.insert(id, (object.kind, Arc::clone(&mesh)));
            items.push(DrawItem {
                id,
                mesh,
                transform: scene.world_matrix(id),
                selected: document.is_selected(id),
            });
        }
        self.meshes = meshes;
        let version = self.summary.drawing.version + 1;
        self.summary = SceneSummary {
            drawing: Arc::new(SceneDrawing { version, items }),
            all: scene.bounds(),
            selected: scene.bounds_of(key.1.iter().copied()),
        };
        self.built_for = Some(key);
        &self.summary
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scene::{Command, PrimitiveKind};

    fn create(doc: &mut Document, kind: PrimitiveKind) -> ObjectId {
        doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
            .unwrap();
        doc.selection()[0]
    }

    #[test]
    fn an_empty_scene_draws_nothing_and_frames_the_middle() {
        let mut cache = DrawingCache::default();
        let summary = cache.update(&Document::new()).clone();
        assert!(summary.drawing.items.is_empty());
        assert_eq!(summary.bounds_for(Frame::All), DEFAULT_FRAME);
    }

    #[test]
    fn shown_shapes_are_drawn_and_hidden_ones_and_groups_are_not() {
        let mut doc = Document::new();
        let cube = create(&mut doc, PrimitiveKind::Cube);
        let sphere = create(&mut doc, PrimitiveKind::Sphere);
        doc.execute(Command::Create(ObjectKind::Group)).unwrap();
        doc.execute(Command::SetVisible {
            ids: vec![sphere],
            visible: false,
        })
        .unwrap();
        let mut cache = DrawingCache::default();
        let summary = cache.update(&doc).clone();
        let drawn: Vec<ObjectId> = summary.drawing.items.iter().map(|i| i.id).collect();
        assert_eq!(drawn, [cube]);
        assert_eq!(summary.all, Some(PrimitiveKind::Cube.standard().bounds()));
    }

    #[test]
    fn selection_shows_and_frames() {
        let mut doc = Document::new();
        let cube = create(&mut doc, PrimitiveKind::Cube);
        let plane = create(&mut doc, PrimitiveKind::Plane);
        doc.set_selection([plane]);
        let mut cache = DrawingCache::default();
        let summary = cache.update(&doc).clone();
        let selected: Vec<bool> = summary.drawing.items.iter().map(|i| i.selected).collect();
        assert_eq!(selected, [true, false], "the plane (on top) is selected");
        assert_eq!(
            summary.bounds_for(Frame::Selected),
            PrimitiveKind::Plane.standard().bounds()
        );
        let _ = cube;
    }

    #[test]
    fn nothing_is_rebuilt_until_something_changes() {
        let mut doc = Document::new();
        create(&mut doc, PrimitiveKind::Sphere);
        let mut cache = DrawingCache::default();
        let first = cache.update(&doc).drawing.clone();
        let again = cache.update(&doc).drawing.clone();
        assert!(Arc::ptr_eq(&first, &again), "same frame, same drawing");
        // Selecting rebuilds the list but keeps the built mesh.
        doc.clear_selection();
        let after = cache.update(&doc).drawing.clone();
        assert!(after.version > first.version);
        assert!(Arc::ptr_eq(&first.items[0].mesh, &after.items[0].mesh));
        // Undoing the creation leaves nothing to draw.
        doc.undo();
        assert!(cache.update(&doc).drawing.items.is_empty());
    }
}
