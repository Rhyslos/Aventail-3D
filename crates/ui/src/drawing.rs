//! What the viewport draws of the scene: each shown object's final shape
//! (its base with its recorded edits), where it is, and whether it is
//! selected. Rebuilt only when the scene or the selection changes; the
//! shapes themselves come from the scene's cache, which works one out
//! again only when that object's base or edits change.

use crate::viewport::{DEFAULT_FRAME, Frame};
use glam::{Mat4, Vec3};
use scene::{Bounds, Document, Mesh, ObjectId};
use std::sync::Arc;
use tools::ElementKind;

/// One object to draw.
#[derive(Debug, Clone, PartialEq)]
pub struct DrawItem {
    /// Which object it is.
    pub id: ObjectId,
    /// Its shape, in its own space.
    pub mesh: Arc<Mesh>,
    /// What of it is shown: the same shape, or a copy without its hidden
    /// polygons (decision 0055). What is picked and edited.
    pub shown: Arc<Mesh>,
    /// What is drawn: `shown`, with the object's live mirror's copies
    /// added (decision 0088), or `shown` itself without a mirror. The
    /// copies come after `shown`'s own points and polygons, which keep
    /// their ids.
    pub drawn: Arc<Mesh>,
    /// Where it is in the world.
    pub transform: Mat4,
    /// Whether it is selected.
    pub selected: bool,
    /// Its materials' colours by slot (sRGB, 0 to 1); empty while it has
    /// no materials of its own, for the standard surface colour.
    pub colors: Vec<[f32; 3]>,
}

/// Everything the viewport draws of the scene. `version` changes whenever
/// anything in it does, so a renderer can tell when to upload it again.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SceneDrawing {
    /// Goes up with every change.
    pub version: u64,
    /// The shown objects that have a shape, top to bottom.
    pub items: Vec<DrawItem>,
    /// Whether the selected objects are being edited (the Points, Edges or
    /// Polygons mode): their edges are drawn plainly, to show their
    /// structure, instead of outlined in the selection colour.
    pub editing: bool,
    /// Whether the edited objects show every point (the Points mode).
    pub points: bool,
    /// The live mirrors' planes to show (decision 0088), each once.
    pub mirrors: Vec<MirrorPlane>,
}

/// A live mirror's plane, shown as a faint red sheet with a clearer line
/// where it meets the ground (decision 0088).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MirrorPlane {
    /// Which world axis it mirrors across: 0 X, 1 Y, 2 Z. The plane is
    /// square to that axis.
    pub axis: usize,
    /// Where it crosses that axis, in centimetres.
    pub at: f32,
    /// The box around the objects mirrored across it (with their twins),
    /// so the sheet is drawn around them.
    pub around: Bounds,
}

impl MirrorPlane {
    /// The sheet's four corners in the world, a little larger than the
    /// objects around it (at least 2 m across).
    pub fn corners(&self) -> [Vec3; 4] {
        let (u, v) = match self.axis {
            0 => (1, 2),
            1 => (0, 2),
            _ => (0, 1),
        };
        let centre = self.around.centre();
        let size = self.around.max - self.around.min;
        let half = |a: usize| (size[a] * 0.65).max(100.0);
        let corner = |su: f32, sv: f32| {
            let mut p = Vec3::ZERO;
            p[self.axis] = self.at;
            p[u] = centre[u] + su * half(u);
            p[v] = centre[v] + sv * half(v);
            p
        };
        [
            corner(-1.0, -1.0),
            corner(1.0, -1.0),
            corner(1.0, 1.0),
            corner(-1.0, 1.0),
        ]
    }
}

/// How an element in the overlay is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayStyle {
    /// Under the mouse: in the light accent colour, larger.
    Hovered,
    /// Selected: in the accent colour, larger.
    Selected,
    /// A hard edge of an edited object (Display → Hard edges).
    Hard,
    /// A polygon's normal: a short line from its middle the way it faces
    /// (Display → Normals).
    Normal,
}

/// What the Points, Edges and Polygons modes draw over the edited objects,
/// in the world: the selected elements and the one under the mouse. (The
/// edited objects' other points and edges are drawn with their shapes;
/// see [`SceneDrawing::editing`].) Small, so it can be rebuilt as the
/// mouse moves. `version` changes whenever anything in it does.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ComponentOverlay {
    /// Goes up with every change.
    pub version: u64,
    /// Points, drawn as dots.
    pub points: Vec<(Vec3, OverlayStyle)>,
    /// Edges, drawn as lines.
    pub lines: Vec<([Vec3; 2], OverlayStyle)>,
    /// Selected polygons and the one under the mouse, as triangles to
    /// tint.
    pub tints: Vec<([Vec3; 3], OverlayStyle)>,
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
    /// The elements to draw over the edited objects, if in the Points,
    /// Edges or Polygons mode.
    pub overlay: Arc<ComponentOverlay>,
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

/// Polygons hidden from view, by object (Shift + H, Ctrl + H; decision
/// 0055). A view setting: not saved and not undone. `revision` goes up
/// with every change.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Hidden {
    pub(crate) revision: u64,
    pub(crate) faces:
        std::collections::HashMap<ObjectId, std::collections::BTreeSet<scene::FaceId>>,
}

impl Hidden {
    /// `mesh` without the polygons hidden on `id` (the same shape when
    /// none are), and the points only they used.
    pub(crate) fn shown(&self, id: ObjectId, mesh: &Arc<Mesh>) -> Arc<Mesh> {
        let Some(hidden) = self.faces.get(&id).filter(|f| !f.is_empty()) else {
            return Arc::clone(mesh);
        };
        let gone: Vec<scene::FaceId> = hidden
            .iter()
            .copied()
            .filter(|f| mesh.face(*f).is_some())
            .collect();
        if gone.is_empty() {
            return Arc::clone(mesh);
        }
        let mut copy = (**mesh).clone();
        mesh::cleanup::delete_elements(&mut copy, &[], &[], &gone);
        Arc::new(copy)
    }
}

/// Keeps the summary up to date with the document, rebuilding as little
/// as possible.
#[derive(Debug, Default)]
pub(crate) struct DrawingCache {
    /// The scene revision, selection and kind of element edited the
    /// summary was built for.
    built_for: Option<(u64, Vec<ObjectId>, Option<ElementKind>, u64)>,
    summary: SceneSummary,
}

impl DrawingCache {
    /// Forgets the summary, so the next [`update`](Self::update) rebuilds
    /// it (for a different document, whose revisions start again). The
    /// version keeps counting up, so a renderer never mistakes the new
    /// scene for one it already drew.
    pub(crate) fn forget(&mut self) {
        self.built_for = None;
    }

    /// The summary of `document`, with the selected objects' `editing`
    /// elements shown (in the Points, Edges or Polygons mode), rebuilt if
    /// the scene or selection has changed since last time.
    pub(crate) fn update(
        &mut self,
        document: &Document,
        editing: Option<ElementKind>,
        hidden: &Hidden,
    ) -> &SceneSummary {
        let key = (
            document.revision(),
            document.selection(),
            editing,
            hidden.revision,
        );
        if self.built_for.as_ref() == Some(&key) {
            return &self.summary;
        }
        let scene = document.scene();
        let mut items = Vec::new();
        for (id, _) in scene.walk() {
            if !scene.is_shown(id) {
                continue;
            }
            let Some(shape) = scene.evaluated(id) else {
                continue;
            };
            // An instance shows its original's materials.
            let own = match scene.object(id).map(|o| &o.kind) {
                Some(scene::ObjectKind::Instance(source)) => *source,
                _ => id,
            };
            let colors = if scene.object(own).is_some_and(|o| !o.materials.is_empty()) {
                scene.material_list(id).iter().map(|m| m.color).collect()
            } else {
                Vec::new()
            };
            let shown = hidden.shown(id, &shape.mesh);
            let drawn = scene
                .with_mirror(id, &shown)
                .map_or_else(|| Arc::clone(&shown), Arc::new);
            items.push(DrawItem {
                id,
                drawn,
                shown,
                mesh: shape.mesh,
                transform: scene.shape_matrix(id),
                selected: document.is_selected(id),
                colors,
            });
        }
        // Each live mirror's planes, once, around the objects that use
        // them.
        let mut mirrors: Vec<MirrorPlane> = Vec::new();
        for item in &items {
            let Some(mirror) = scene.object(item.id).and_then(|o| o.mirror) else {
                continue;
            };
            let Some(around) = scene.bounds_of([item.id]) else {
                continue;
            };
            for axis in (0..3).filter(|a| mirror.axes[*a]) {
                let at = mirror.centre[axis];
                match mirrors
                    .iter_mut()
                    .find(|m| m.axis == axis && (m.at - at).abs() < 1e-3)
                {
                    Some(plane) => plane.around = plane.around.union(around),
                    None => mirrors.push(MirrorPlane { axis, at, around }),
                }
            }
        }
        let version = self.summary.drawing.version + 1;
        self.summary = SceneSummary {
            drawing: Arc::new(SceneDrawing {
                version,
                items,
                editing: editing.is_some(),
                points: editing == Some(ElementKind::Points),
                mirrors,
            }),
            all: scene.bounds(),
            selected: scene.bounds_of(key.1.iter().copied()),
            overlay: Arc::clone(&self.summary.overlay),
        };
        self.built_for = Some(key);
        &self.summary
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scene::{Command, ObjectKind, Operation, PrimitiveKind, VertexId};

    fn create(doc: &mut Document, kind: PrimitiveKind) -> ObjectId {
        doc.execute(Command::Create(ObjectKind::Primitive(kind.standard())))
            .unwrap();
        doc.selection()[0]
    }

    #[test]
    fn a_different_document_gets_a_new_version() {
        // Regression: opening or restoring a project mid-session reset the
        // version to the one the GPU had just drawn, so nothing was drawn.
        let mut cache = DrawingCache::default();
        let empty = cache
            .update(&Document::new(), None, &Hidden::default())
            .drawing
            .version;
        let mut other = Document::new();
        create(&mut other, PrimitiveKind::Cube);
        let mut fresh = Document::new();
        create(&mut fresh, PrimitiveKind::Cube);
        let first = cache
            .update(&other, None, &Hidden::default())
            .drawing
            .version;
        assert!(first > empty);
        cache.forget();
        let after = cache.update(&fresh, None, &Hidden::default());
        assert!(after.drawing.version > first);
        assert_eq!(after.drawing.items.len(), 1);
    }

    #[test]
    fn an_empty_scene_draws_nothing_and_frames_the_middle() {
        let mut cache = DrawingCache::default();
        let summary = cache
            .update(&Document::new(), None, &Hidden::default())
            .clone();
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
        let summary = cache.update(&doc, None, &Hidden::default()).clone();
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
        let summary = cache.update(&doc, None, &Hidden::default()).clone();
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
        let first = cache.update(&doc, None, &Hidden::default()).drawing.clone();
        let again = cache.update(&doc, None, &Hidden::default()).drawing.clone();
        assert!(Arc::ptr_eq(&first, &again), "same frame, same drawing");
        // Selecting rebuilds the list but keeps the built mesh.
        doc.clear_selection();
        let after = cache.update(&doc, None, &Hidden::default()).drawing.clone();
        assert!(after.version > first.version);
        assert!(Arc::ptr_eq(&first.items[0].mesh, &after.items[0].mesh));
        // Undoing the selection, then the creation, leaves nothing to draw.
        doc.undo();
        doc.undo();
        assert!(
            cache
                .update(&doc, None, &Hidden::default())
                .drawing
                .items
                .is_empty()
        );
    }

    #[test]
    fn recorded_edits_are_drawn_and_framed() {
        let mut doc = Document::new();
        let cube = create(&mut doc, PrimitiveKind::Cube);
        let mut cache = DrawingCache::default();
        let before = cache.update(&doc, None, &Hidden::default()).clone();
        let top: Vec<VertexId> = before.drawing.items[0]
            .mesh
            .vertices()
            .filter(|(_, p)| p.z > 50.0)
            .map(|(v, _)| v)
            .collect();
        doc.execute(Command::Record {
            id: cube,
            operation: Operation::MovePoints {
                points: top,
                offset: glam::Vec3::Z * 100.0,
                anchors: None,
            },
        })
        .unwrap();
        let after = cache.update(&doc, None, &Hidden::default()).clone();
        assert_ne!(after.drawing.items[0].mesh, before.drawing.items[0].mesh);
        assert_eq!(after.all.unwrap().max.z, 200.0, "framing sees the edit");
    }

    #[test]
    fn a_shape_made_editable_looks_the_same() {
        let mut doc = Document::new();
        let sphere = create(&mut doc, PrimitiveKind::Sphere);
        let mut cache = DrawingCache::default();
        let before = cache.update(&doc, None, &Hidden::default()).drawing.items[0]
            .mesh
            .clone();
        doc.execute(Command::MakeEditable(vec![sphere])).unwrap();
        let after = cache.update(&doc, None, &Hidden::default()).drawing.items[0]
            .mesh
            .clone();
        assert_eq!(*after, *before);
    }
}
