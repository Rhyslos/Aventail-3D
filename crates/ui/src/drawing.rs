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
    /// What is drawn: `shown`, cut by the object's booleans (decision
    /// 0135: then worked out from its whole shape, hidden polygons and
    /// all, and its points and polygons are new), with the object's live
    /// mirror's copies added (decision 0088), or `shown` itself without
    /// either. The copies come after the shape's own points and polygons.
    /// With modifiers shown in the viewport (decision 0142), their result
    /// on the whole shape, cut and mirrored.
    pub drawn: Arc<Mesh>,
    /// Where it is in the world.
    pub transform: Mat4,
    /// Whether it is selected.
    pub selected: bool,
    /// Its materials' colours by slot (sRGB, 0 to 1); empty while it has
    /// no materials of its own, for the standard surface colour.
    pub colors: Vec<[f32; 3]>,
    /// Where its shading breaks: its own smoothing (decision 0126; an
    /// instance's is its original's).
    pub hardness: mesh::Hardness,
    /// Whether it is drawn as an outline only: a cutter, whose shape cuts
    /// another object (decision 0135). Its faces aren't drawn, and a click
    /// picks it only where no other object is under the mouse.
    pub outline: bool,
    /// For a curve object (decision 0138): its line, points and handles.
    /// In the Points mode, a selected curve's `mesh` and `shown` are its
    /// points and handle ends as loose points (named by
    /// `mesh::curve::CurvePart::code`), so they are found, selected and
    /// moved as a mesh's points are; in the Edges and Polygons modes they
    /// are empty (a curve has none of its own). `drawn` is its solid.
    pub curve: Option<Arc<CurveDrawing>>,
    /// For a lattice cage (decision 0144): its lines and points. In the
    /// Points mode, a selected cage's `mesh` and `shown` are its points as
    /// loose points (named by their place in the cage), so they are found,
    /// selected and moved as a mesh's points are; otherwise they are empty.
    /// `drawn` is always empty: a cage has no surface.
    pub cage: Option<Arc<CageDrawing>>,
}

/// What is drawn of a lattice cage, in its own space (decision 0144).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CageDrawing {
    /// Each line between neighbouring points.
    pub lines: Vec<[Vec3; 2]>,
    /// Its points.
    pub points: Vec<Vec3>,
}

impl CageDrawing {
    /// The drawing of `lattice`.
    pub fn of(lattice: &scene::Lattice) -> Self {
        let points = lattice.points().to_vec();
        Self {
            lines: lattice
                .lines()
                .into_iter()
                .map(|(a, b)| [points[a], points[b]])
                .collect(),
            points,
        }
    }
}

/// What is drawn of a curve object, in its own space (decision 0138).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CurveDrawing {
    /// The curve followed finely: drawn as its line.
    pub line: Vec<Vec3>,
    /// Its points and the ends of the handles that pull, by part.
    pub parts: Vec<(mesh::curve::CurvePart, Vec3)>,
    /// Each handle that pulls, from its point to its end.
    pub handles: Vec<[Vec3; 2]>,
}

impl CurveDrawing {
    /// How finely a curve's line follows it: pieces per stretch.
    const LINE_DETAIL: u32 = 24;

    /// The drawing of `curve`.
    pub fn of(curve: &mesh::curve::Curve) -> Self {
        let parts = curve.parts();
        let handles = curve
            .points()
            .iter()
            .flat_map(|p| [(p.position, p.handle_in), (p.position, p.handle_out)])
            .filter(|(_, h)| h.length_squared() > 1e-12)
            .map(|(at, h)| [at, at + h])
            .collect();
        Self {
            line: curve.polyline(Self::LINE_DETAIL),
            parts,
            handles,
        }
    }
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
    /// The project's texture size in pixels (decision 0100), for the
    /// checker's squares.
    pub texture_size: u32,
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
    /// so the sheet is drawn around them; in `space`.
    pub around: Bounds,
    /// The space the plane is in: the world's (identity), or for a mirror
    /// on the object, its pivot's (decision 0126).
    pub space: glam::Mat4,
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
            self.space.transform_point3(p)
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
    /// A seam of an edited object (Display → Seams, and the UV mode).
    Seam,
    /// An edge where seams are preferred (decision 0111), shown with seams.
    SeamPrefer,
    /// An edge where no seam may go (decision 0111), shown with seams.
    SeamNever,
    /// A creased edge (decision 0143), shown whenever points, edges or
    /// polygons are being edited.
    Crease,
    /// An edge with a bevel weight (decision 0149), shown as creases are.
    BevelWeight,
    /// A polygon's normal: a short line from its middle the way it faces
    /// (Display → Normals).
    Normal,
    /// A curve's line (decision 0138).
    Curve,
    /// A selected curve's line.
    CurveSelected,
    /// A curve's handles, and its points and handle ends not selected, in
    /// the Points mode.
    Handle,
    /// An edge of an object's collision shape (decision 0151): the
    /// selected objects', or every object's with Display → Show
    /// collision.
    Collision,
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
    /// Heavy cuts being worked out on other threads (decision 0135).
    cutting: std::collections::HashMap<ObjectId, std::thread::JoinHandle<scene::CutDone>>,
    /// Each kept object's last cut shown, to show while a newer one is
    /// worked out.
    last_cut: std::collections::HashMap<ObjectId, Arc<Mesh>>,
    /// Heavy modifiers being worked out on other threads (decision 0142).
    stacking: std::collections::HashMap<ObjectId, std::thread::JoinHandle<scene::StackDone>>,
    /// Each object's last shape after its modifiers, to show while newer
    /// work is done.
    last_stack: std::collections::HashMap<ObjectId, Arc<Mesh>>,
}

impl DrawingCache {
    /// Forgets the summary, so the next [`update`](Self::update) rebuilds
    /// it (for a different document, whose revisions start again). The
    /// version keeps counting up, so a renderer never mistakes the new
    /// scene for one it already drew.
    pub(crate) fn forget(&mut self) {
        self.built_for = None;
        // Cuts and modifiers of the old document's objects aren't this
        // one's.
        self.last_cut.clear();
        self.cutting.clear();
        self.last_stack.clear();
        self.stacking.clear();
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
        // A heavy cut or modifier worked out since: kept by the scene, and
        // drawn.
        let cuts = self.collect_cuts(document.scene());
        if self.collect_stacks(document.scene()) || cuts {
            self.built_for = None;
        }
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
        let cutters = scene.cutters();
        let mut items = Vec::new();
        for (id, _) in scene.walk() {
            if !scene.is_shown(id) {
                continue;
            }
            // A curve is drawn as its line, with its solid if it has one
            // (decision 0138).
            let curve = scene
                .object(id)
                .and_then(|o| o.kind.curve())
                .map(|c| c.shape.curve());
            // A cage is drawn as its lines (decision 0144).
            let cage = scene.object(id).and_then(|o| o.kind.lattice()).cloned();
            let shape = match (scene.evaluated(id), &curve) {
                (Some(shape), _) => shape,
                (None, Some(_)) => scene::Evaluated {
                    mesh: Arc::new(Mesh::default()),
                    warnings: Vec::new(),
                },
                (None, None) if cage.is_some() => scene::Evaluated {
                    mesh: Arc::new(Mesh::default()),
                    warnings: Vec::new(),
                },
                (None, None) => continue,
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
            let hardness = scene
                .object(own)
                .map_or(mesh::Hardness::Marked, |o| o.hardness);
            let shown = hidden.shown(id, &shape.mesh);
            // Cut by its booleans, if they work (decision 0135).
            let cut = self.cut_of(scene, id);
            let drawn = if scene.has_modifiers_for(id, scene::Purpose::Viewport) {
                // Modifiers work on the whole shape, hidden polygons and
                // all, after the mirror (decision 0142).
                let whole = cut.unwrap_or_else(|| Arc::clone(&shape.mesh));
                let input = scene.mirrored_shape(id, whole);
                self.modified(scene, id, input)
            } else {
                let cut = cut.unwrap_or_else(|| Arc::clone(&shown));
                scene
                    .with_mirror(id, &cut)
                    .map_or_else(|| Arc::clone(&cut), Arc::new)
            };
            let selected = document.is_selected(id);
            if let Some(cage) = cage {
                // Its points to edit in the Points mode, else nothing to
                // pick but its lines.
                let points = match editing {
                    Some(ElementKind::Points) if selected => Arc::new(cage.points_mesh()),
                    _ => Arc::new(Mesh::default()),
                };
                items.push(DrawItem {
                    id,
                    drawn: Arc::new(Mesh::default()),
                    shown: Arc::clone(&points),
                    mesh: points,
                    transform: scene.shape_matrix(id),
                    selected,
                    colors: Vec::new(),
                    hardness,
                    outline: false,
                    curve: None,
                    cage: Some(Arc::new(CageDrawing::of(&cage))),
                });
                continue;
            }
            let (mesh, shown, curve) = match curve {
                Some(curve) => {
                    // Its points to edit in the Points mode; nothing in the
                    // Edges and Polygons modes.
                    let parts = match editing {
                        Some(ElementKind::Points) if selected => Arc::new(curve.parts_mesh()),
                        Some(_) if selected => Arc::new(Mesh::default()),
                        _ => Arc::clone(&shape.mesh),
                    };
                    (
                        Arc::clone(&parts),
                        parts,
                        Some(Arc::new(CurveDrawing::of(&curve))),
                    )
                }
                None => (shape.mesh, shown, None),
            };
            items.push(DrawItem {
                id,
                drawn,
                shown,
                mesh,
                transform: scene.shape_matrix(id),
                selected,
                colors,
                hardness,
                outline: cutters.contains(&id),
                curve,
                cage: None,
            });
        }
        // Each live mirror's planes, once, around the objects that use
        // them.
        let mut mirrors: Vec<MirrorPlane> = Vec::new();
        for item in &items {
            let Some(mirror) = scene.object(item.id).and_then(|o| o.mirror) else {
                continue;
            };
            let Some(world_box) = scene.bounds_of([item.id]) else {
                continue;
            };
            // A mirror on the object stands in its pivot's space.
            let space = if mirror.on_object {
                scene.world_matrix(item.id)
            } else {
                glam::Mat4::IDENTITY
            };
            if space.determinant().abs() < 1e-12 {
                continue;
            }
            let back = space.inverse();
            let Some(around) =
                Bounds::around(world_box.corners().map(|c| back.transform_point3(c)))
            else {
                continue;
            };
            for axis in (0..3).filter(|a| mirror.axes[*a]) {
                let at = mirror.centre[axis];
                match mirrors.iter_mut().find(|m| {
                    m.axis == axis && (m.at - at).abs() < 1e-3 && m.space.abs_diff_eq(space, 1e-4)
                }) {
                    Some(plane) => plane.around = plane.around.union(around),
                    None => mirrors.push(MirrorPlane {
                        axis,
                        at,
                        around,
                        space,
                    }),
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
                texture_size: scene.texture().size,
            }),
            all: scene.bounds(),
            selected: scene.bounds_of(key.1.iter().copied()),
            overlay: Arc::clone(&self.summary.overlay),
        };
        self.built_for = Some(key);
        &self.summary
    }

    /// Whether heavy cuts or modifiers are being worked out on other
    /// threads (the window keeps drawing until they are in).
    pub(crate) fn busy(&self) -> bool {
        !self.cutting.is_empty() || !self.stacking.is_empty()
    }

    /// Hands every finished cut to the scene; whether any was.
    fn collect_cuts(&mut self, scene: &scene::Scene) -> bool {
        let done: Vec<ObjectId> = self
            .cutting
            .iter()
            .filter(|(_, job)| job.is_finished())
            .map(|(id, _)| *id)
            .collect();
        for id in &done {
            if let Some(job) = self.cutting.remove(id)
                && let Ok(cut) = job.join()
            {
                scene.store_cut(cut);
            }
        }
        !done.is_empty()
    }

    /// The object's cut shape, if it has booleans that work: ready, or
    /// worked out now when light. A heavy one is worked out on another
    /// thread ([`scene::CUT_IN_BACKGROUND_FROM`]), and its last cut (or
    /// nothing, so the uncut shape) is shown meanwhile.
    fn cut_of(&mut self, scene: &scene::Scene, id: ObjectId) -> Option<Arc<Mesh>> {
        let cut = match scene.cut_state(id)? {
            scene::CutState::Ready(result) => result.ok(),
            scene::CutState::Needed(job) if job.polygons() < scene::CUT_IN_BACKGROUND_FROM => {
                scene.cut_shape(id).and_then(Result::ok)
            }
            scene::CutState::Needed(job) => {
                // One at a time per object: a newer cut waits until the one
                // under way is in, so a drag doesn't pile up work.
                self.cutting
                    .entry(id)
                    .or_insert_with(|| std::thread::spawn(move || job.run()));
                return self.last_cut.get(&id).cloned();
            }
        };
        match &cut {
            Some(mesh) => self.last_cut.insert(id, Arc::clone(mesh)),
            None => self.last_cut.remove(&id),
        };
        cut
    }
}

mod stack;

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
