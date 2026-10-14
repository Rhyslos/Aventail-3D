//! The Points, Edges and Polygons modes in the viewport: which element is
//! under the mouse, and what to draw over the objects being edited (the
//! selected ones). See [decision 0027](../../../docs/decisions/0027-component-modes.md).
//!
//! Each frame the tracker is told the mode, what is drawn and where the
//! mouse is; it finds the element under the mouse (through
//! [`tools::PickScene`], rebuilt only when the scene drawing changes) and
//! rebuilds the overlay only when the mode, the drawing or the hovered
//! element changes.

use crate::drawing::{ComponentOverlay, DrawItem, OverlayStyle, SceneDrawing};
use crate::palette::Mode;
use glam::{Vec2, Vec3};
use scene::{Document, ElementSelection, ObjectId};
use std::collections::HashMap;
use std::sync::Arc;
use tools::{Camera, Element, ElementKind, MeshPick, PickScene};

/// The mouse over a view: the view's camera, its size and where the mouse
/// is in it (both in screen points, from the view's top-left corner).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ViewPointer {
    pub(crate) camera: Camera,
    pub(crate) size: Vec2,
    pub(crate) at: Vec2,
}

/// The kind of element a mode works on, if any.
pub(crate) fn kind_for(mode: Mode) -> Option<ElementKind> {
    match mode {
        Mode::Points => Some(ElementKind::Points),
        Mode::Edges => Some(ElementKind::Edges),
        Mode::Polygons => Some(ElementKind::Polygons),
        Mode::Model | Mode::Object | Mode::Uv => None,
    }
}

/// An element of an object.
pub(crate) type ObjectElement = (ObjectId, Element);

/// What the overlay was last built for: the drawing's version, the kind
/// of element, the hovered element, the selected elements' revision and
/// the tool lines' revision.
type BuiltFor = (u64, Option<ElementKind>, Option<ObjectElement>, u64, u64);

/// Keeps track of the element under the mouse and the overlay.
#[derive(Debug, Default)]
pub(crate) struct ComponentTracker {
    /// The scene the mouse is looked up in, and the drawing version it was
    /// built from.
    scene: Option<(u64, PickScene)>,
    /// Each drawn object's mesh, indexed for picking: kept while the mesh
    /// stays the same, and updated quickly when only its points moved.
    picks: HashMap<ObjectId, Arc<MeshPick>>,
    /// Indexes being built in the background for shapes that are new.
    building: HashMap<ObjectId, (Arc<scene::Mesh>, std::thread::JoinHandle<MeshPick>)>,
    /// Whether a drag is under way: the scene stays as it was when the
    /// drag began (what moves is never a snap target, so nothing it holds
    /// goes out of date), and nothing is looked up under the mouse.
    frozen: bool,
    hovered: Option<ObjectElement>,
    /// Whether a tool that finds its own elements under the mouse is in
    /// use (Loop cut): the mode's element isn't looked up or highlighted.
    quiet: bool,
    /// Lines a tool shows over the shapes (Loop cut's new loop), in the
    /// world, and a count that goes up whenever they change.
    tool_lines: (Vec<[Vec3; 2]>, u64),
    built_for: Option<BuiltFor>,
    overlay: Arc<ComponentOverlay>,
}

impl ComponentTracker {
    /// Brings the hovered element and the overlay up to date. Returns the
    /// overlay to draw.
    pub(crate) fn update(
        &mut self,
        mode: Mode,
        drawing: &SceneDrawing,
        document: &Document,
        pointer: Option<ViewPointer>,
    ) -> Arc<ComponentOverlay> {
        let kind = kind_for(mode);
        self.hovered = match (kind, pointer) {
            (Some(kind), Some(pointer)) if !self.frozen && !self.quiet => {
                self.find(kind, drawing, pointer)
            }
            _ => None,
        };
        let wanted = (
            drawing.version,
            kind,
            self.hovered,
            document.element_revision(),
            self.tool_lines.1,
        );
        if self.built_for != Some(wanted) {
            let version = self.overlay.version + 1;
            let mut overlay = match kind {
                Some(kind) => build(kind, drawing, document, self.hovered, version),
                None => ComponentOverlay {
                    version,
                    ..ComponentOverlay::default()
                },
            };
            if kind.is_some() {
                let lines = self.tool_lines.0.iter();
                overlay
                    .lines
                    .extend(lines.map(|line| (*line, OverlayStyle::Hovered)));
            }
            self.overlay = Arc::new(overlay);
            self.built_for = Some(wanted);
        }
        Arc::clone(&self.overlay)
    }

    /// Whether a tool that finds its own elements under the mouse is in
    /// use (see `quiet`).
    pub(crate) fn set_quiet(&mut self, quiet: bool) {
        self.quiet = quiet;
    }

    /// The lines a tool shows over the shapes, in the world (none: `[]`).
    pub(crate) fn set_tool_lines(&mut self, lines: Vec<[Vec3; 2]>) {
        if self.tool_lines.0 != lines {
            self.tool_lines.0 = lines;
            self.tool_lines.1 += 1;
        }
    }

    /// Keeps the scene as it is while a drag is under way (see `frozen`).
    pub(crate) fn freeze(&mut self, frozen: bool) {
        self.frozen = frozen;
    }

    /// Starts indexing new shapes in the background, so the first click
    /// on a heavy shape doesn't wait for it ([decision 0035](../../../docs/decisions/0035-heavy-meshes.md)).
    /// Shapes whose points only moved are left alone: they are updated
    /// quickly when next needed. Their connections ([`scene::Mesh::topology`])
    /// are worked out on the same thread, for the tools that need them.
    pub(crate) fn prepare(&mut self, drawing: &SceneDrawing) {
        /// Below this many polygons, indexing is quick enough to do when
        /// needed.
        const BACKGROUND_FROM: usize = 50_000;
        if self.frozen {
            return;
        }
        for item in &drawing.items {
            if item.mesh.face_count() < BACKGROUND_FROM {
                continue;
            }
            let known = self.picks.get(&item.id).is_some_and(|p| {
                Arc::ptr_eq(p.mesh(), &item.mesh) || p.mesh().same_structure(&item.mesh)
            });
            let running = self.building.get(&item.id).is_some_and(|(mesh, handle)| {
                Arc::ptr_eq(mesh, &item.mesh) || !handle.is_finished()
            });
            if known || running {
                continue;
            }
            let mesh = Arc::clone(&item.mesh);
            let handle = std::thread::spawn(move || {
                let pick = MeshPick::new(Arc::clone(&mesh));
                let _ = mesh.topology();
                pick
            });
            self.building
                .insert(item.id, (Arc::clone(&item.mesh), handle));
        }
    }

    /// Whether shapes are still being indexed in the background.
    #[cfg(test)]
    pub(crate) fn preparing(&self) -> bool {
        self.building
            .values()
            .any(|(_, handle)| !handle.is_finished())
    }

    /// The index for `item`: kept, finished in the background, updated
    /// from the kept one, or built now.
    fn pick_for(&mut self, item: &DrawItem) -> Arc<MeshPick> {
        if let Some(kept) = self.picks.get(&item.id)
            && Arc::ptr_eq(kept.mesh(), &item.mesh)
        {
            return Arc::clone(kept);
        }
        if let Some((mesh, _)) = self.building.get(&item.id)
            && Arc::ptr_eq(mesh, &item.mesh)
            && let Some((_, handle)) = self.building.remove(&item.id)
            && let Ok(pick) = handle.join()
        {
            return Arc::new(pick);
        }
        match self.picks.get(&item.id) {
            Some(kept) => Arc::new(kept.moved(Arc::clone(&item.mesh))),
            None => Arc::new(MeshPick::new(Arc::clone(&item.mesh))),
        }
    }

    /// The overlay as last built.
    pub(crate) fn overlay(&self) -> Arc<ComponentOverlay> {
        Arc::clone(&self.overlay)
    }

    /// The element under the mouse, if any.
    pub(crate) fn hovered(&self) -> Option<ObjectElement> {
        self.hovered
    }

    /// The scene for finding what is under the mouse or in a region,
    /// built again only when the drawing has changed.
    pub(crate) fn pick_scene(&mut self, drawing: &SceneDrawing) -> &PickScene {
        let stale = self
            .scene
            .as_ref()
            .is_none_or(|(v, _)| *v != drawing.version);
        if stale && !(self.frozen && self.scene.is_some()) {
            let mut picks = HashMap::with_capacity(drawing.items.len());
            for item in &drawing.items {
                picks.insert(item.id, self.pick_for(item));
            }
            self.building
                .retain(|id, _| drawing.items.iter().any(|i| i.id == *id));
            let scene = PickScene::from_picks(
                drawing
                    .items
                    .iter()
                    .map(|item| (Arc::clone(&picks[&item.id]), item.transform, item.selected)),
            );
            self.picks = picks;
            self.scene = Some((drawing.version, scene));
        }
        let (_, scene) = self.scene.get_or_insert_with(Default::default);
        scene
    }

    /// Whether a drawn shape's index is still being built in the
    /// background and the scene would have to wait for it.
    fn waiting_on_background(&self, drawing: &SceneDrawing) -> bool {
        let current = self
            .scene
            .as_ref()
            .is_some_and(|(v, _)| *v == drawing.version);
        !current
            && drawing.items.iter().any(|item| {
                self.building.get(&item.id).is_some_and(|(mesh, handle)| {
                    Arc::ptr_eq(mesh, &item.mesh) && !handle.is_finished()
                })
            })
    }

    /// The element of `kind` under `pointer` on an edited object, if any.
    /// Never waits for a heavy shape's index still being built.
    pub(crate) fn find(
        &mut self,
        kind: ElementKind,
        drawing: &SceneDrawing,
        pointer: ViewPointer,
    ) -> Option<ObjectElement> {
        // Hovering never waits for a heavy shape's index (it would stall
        // the moment a drag ends): nothing is highlighted until it is
        // ready. Clicks still wait, since they need the answer.
        if self.waiting_on_background(drawing) {
            return None;
        }
        let found =
            self.pick_scene(drawing)
                .under(kind, &pointer.camera, pointer.size, pointer.at)?;
        let item = drawing.items.get(found.object)?;
        Some((item.id, found.element))
    }
}

/// The overlay for `kind`: the edited objects' selected elements, and the
/// one under the mouse. It never walks the whole mesh, only what is
/// selected, so it stays quick to rebuild on heavy shapes.
fn build(
    kind: ElementKind,
    drawing: &SceneDrawing,
    document: &Document,
    hovered: Option<ObjectElement>,
    version: u64,
) -> ComponentOverlay {
    let mut overlay = ComponentOverlay {
        version,
        ..ComponentOverlay::default()
    };
    let none = ElementSelection::new();
    for item in drawing.items.iter().filter(|i| i.selected) {
        if !item.transform.is_finite() {
            continue;
        }
        let selected = document.elements(item.id).unwrap_or(&none);
        let hovered_here = hovered.filter(|(id, _)| *id == item.id).map(|(_, e)| e);
        let place = |v| {
            item.mesh
                .position(v)
                .map(|p| item.transform.transform_point3(p))
        };
        match kind {
            ElementKind::Edges => {
                let mut line = |edge: mesh::EdgeKey, style| {
                    let (a, b) = edge.ends();
                    if let (Some(a), Some(b)) = (place(a), place(b)) {
                        overlay.lines.push(([a, b], style));
                    }
                };
                for edge in selected.edges() {
                    if hovered_here != Some(Element::Edge(edge)) {
                        line(edge, OverlayStyle::Selected);
                    }
                }
                if let Some(Element::Edge(edge)) = hovered_here {
                    line(edge, OverlayStyle::Hovered);
                }
            }
            ElementKind::Points => {
                for v in selected.points() {
                    if hovered_here != Some(Element::Point(v))
                        && let Some(p) = place(v)
                    {
                        overlay.points.push((p, OverlayStyle::Selected));
                    }
                }
                if let Some(Element::Point(v)) = hovered_here
                    && let Some(p) = place(v)
                {
                    overlay.points.push((p, OverlayStyle::Hovered));
                }
            }
            ElementKind::Polygons => {
                for face in selected.polygons() {
                    if hovered_here != Some(Element::Polygon(face)) {
                        tint(&mut overlay, item, face, OverlayStyle::Selected);
                    }
                }
                if let Some(Element::Polygon(face)) = hovered_here {
                    tint(&mut overlay, item, face, OverlayStyle::Hovered);
                }
            }
        }
    }
    overlay
}

/// Tints a polygon of `item`; a hovered one is outlined too.
fn tint(overlay: &mut ComponentOverlay, item: &DrawItem, face: scene::FaceId, how: OverlayStyle) {
    let Some(corners) = item.mesh.face(face) else {
        return;
    };
    let world: Vec<Vec3> = corners
        .iter()
        .filter_map(|v| item.mesh.position(*v))
        .map(|p| item.transform.transform_point3(p))
        .collect();
    if let Some((first, rest)) = world.split_first() {
        for pair in rest.windows(2) {
            overlay.tints.push(([*first, pair[0], pair[1]], how));
        }
    }
    if how == OverlayStyle::Hovered {
        for (i, a) in world.iter().enumerate() {
            let b = world[(i + 1) % world.len()];
            overlay.lines.push(([*a, b], OverlayStyle::Hovered));
        }
    }
}

/// The hovered element in words, for the status bar: "point 4 of Cube".
pub(crate) fn describe(document: &Document, (id, element): ObjectElement) -> String {
    let name = document
        .scene()
        .object(id)
        .map_or_else(|| "an object".to_string(), |o| o.name.clone());
    match element {
        Element::Point(v) => format!("point {} of {name}", v.get()),
        Element::Edge(e) => {
            let (a, b) = e.ends();
            format!("edge {}–{} of {name}", a.get(), b.get())
        }
        Element::Polygon(f) => format!("polygon {} of {name}", f.get()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Mat4;
    use scene::PrimitiveKind;

    fn drawing(selected: bool) -> SceneDrawing {
        SceneDrawing {
            version: 1,
            items: vec![DrawItem {
                id: ObjectId::from_raw(7),
                mesh: Arc::new(PrimitiveKind::Cube.standard().mesh()),
                transform: Mat4::IDENTITY,
                selected,
            }],
            editing: true,
            points: false,
        }
    }

    fn pointer_at(world: Vec3) -> ViewPointer {
        let camera = Camera::looking_at(Vec3::new(300.0, -500.0, 250.0), Vec3::new(0.0, 0.0, 50.0));
        let size = Vec2::new(1000.0, 600.0);
        ViewPointer {
            camera,
            size,
            at: camera.project(world, size).unwrap(),
        }
    }

    #[test]
    fn modes_map_to_element_kinds() {
        assert_eq!(kind_for(Mode::Points), Some(ElementKind::Points));
        assert_eq!(kind_for(Mode::Edges), Some(ElementKind::Edges));
        assert_eq!(kind_for(Mode::Polygons), Some(ElementKind::Polygons));
        for mode in [Mode::Model, Mode::Object, Mode::Uv] {
            assert_eq!(kind_for(mode), None);
        }
    }

    #[test]
    fn the_overlay_holds_only_what_is_selected_or_under_the_mouse() {
        // Plain points and edges are drawn with the shape itself, so with
        // nothing selected and nothing under the mouse the overlay is
        // empty.
        let mut tracker = ComponentTracker::default();
        let overlay = tracker.update(Mode::Points, &drawing(true), &Document::new(), None);
        assert!(overlay.points.is_empty() && overlay.lines.is_empty());
        assert!(overlay.tints.is_empty());
        // Selected edges are drawn in the Edges mode.
        let mut doc = Document::new();
        doc.execute(scene::Command::Create(scene::ObjectKind::Primitive(
            PrimitiveKind::Cube.standard(),
        )))
        .unwrap();
        let id = doc.selection()[0];
        let mut drawn = drawing(true);
        drawn.items[0].id = id;
        let edges: Vec<mesh::EdgeKey> = drawn.items[0].mesh.edges()[..3].to_vec();
        let mut chosen = ElementSelection::new();
        chosen.combine(
            ElementKind::Edges,
            edges.iter().map(|e| Element::Edge(*e)),
            mesh::Combine::Add,
        );
        doc.select_elements(vec![(id, chosen)], "Select", None);
        let overlay = tracker.update(Mode::Edges, &drawn, &doc, None);
        assert_eq!(overlay.lines.len(), 3);
        assert!(
            overlay
                .lines
                .iter()
                .all(|(_, s)| *s == OverlayStyle::Selected)
        );
        assert!(overlay.points.is_empty(), "the Edges mode draws edges only");
    }

    #[test]
    fn unselected_objects_and_other_modes_draw_nothing() {
        let mut tracker = ComponentTracker::default();
        let overlay = tracker.update(Mode::Points, &drawing(false), &Document::new(), None);
        assert!(overlay.points.is_empty() && overlay.lines.is_empty());
        let overlay = tracker.update(Mode::Model, &drawing(true), &Document::new(), None);
        assert!(overlay.points.is_empty() && overlay.lines.is_empty());
    }

    #[test]
    fn the_hovered_element_is_picked_out() {
        let mut tracker = ComponentTracker::default();
        let corner = Vec3::new(50.0, -50.0, 100.0);
        let overlay = tracker.update(
            Mode::Points,
            &drawing(true),
            &Document::new(),
            Some(pointer_at(corner)),
        );
        let (id, component) = tracker.hovered().unwrap();
        assert_eq!(id, ObjectId::from_raw(7));
        assert!(matches!(component, Element::Point(_)));
        let hovered: Vec<_> = overlay
            .points
            .iter()
            .filter(|(_, s)| *s == OverlayStyle::Hovered)
            .collect();
        assert_eq!(hovered.len(), 1);
        assert!(hovered[0].0.distance(corner) < 1e-3);

        let face_middle = Vec3::new(0.0, -50.0, 50.0);
        let overlay = tracker.update(
            Mode::Polygons,
            &drawing(true),
            &Document::new(),
            Some(pointer_at(face_middle)),
        );
        assert!(matches!(tracker.hovered(), Some((_, Element::Polygon(_)))));
        assert_eq!(overlay.tints.len(), 2, "a quad is two triangles");
        assert_eq!(
            overlay
                .lines
                .iter()
                .filter(|(_, s)| *s == OverlayStyle::Hovered)
                .count(),
            4,
            "its outline"
        );
    }

    #[test]
    fn the_overlay_is_rebuilt_only_when_something_changes() {
        let mut tracker = ComponentTracker::default();
        let first = tracker.update(Mode::Edges, &drawing(true), &Document::new(), None);
        let again = tracker.update(Mode::Edges, &drawing(true), &Document::new(), None);
        assert!(Arc::ptr_eq(&first, &again));
        let other = tracker.update(Mode::Points, &drawing(true), &Document::new(), None);
        assert!(other.version > first.version);
        let mut moved = drawing(true);
        moved.version = 2;
        let newer = tracker.update(Mode::Points, &moved, &Document::new(), None);
        assert!(newer.version > other.version);
    }

    fn heavy_drawing(version: u64, mesh: Arc<scene::Mesh>) -> SceneDrawing {
        SceneDrawing {
            version,
            items: vec![DrawItem {
                id: ObjectId::from_raw(3),
                mesh,
                transform: Mat4::IDENTITY,
                selected: true,
            }],
            editing: true,
            points: false,
        }
    }

    #[test]
    fn heavy_shapes_are_indexed_in_the_background() {
        let ball = Arc::new(mesh::samples::heavy_ball(60_000, 100.0));
        let drawing = heavy_drawing(1, Arc::clone(&ball));
        let mut tracker = ComponentTracker::default();
        tracker.prepare(&drawing);
        assert_eq!(tracker.building.len(), 1, "started on another thread");
        // Asking again doesn't start it twice.
        tracker.prepare(&drawing);
        assert_eq!(tracker.building.len(), 1);
        // The scene waits for it when needed, and finds the shape.
        let pointer = pointer_at(Vec3::new(0.0, -100.0, 100.0));
        let scene = tracker.pick_scene(&drawing);
        assert!(
            scene
                .object_under(&pointer.camera, pointer.size, pointer.at)
                .is_some()
        );
        assert!(tracker.building.is_empty());
        // Light shapes are indexed when needed, not in the background.
        let light = heavy_drawing(2, Arc::new(PrimitiveKind::Cube.standard().mesh()));
        tracker.prepare(&light);
        assert!(tracker.building.is_empty());
    }

    #[test]
    fn hovering_does_not_wait_for_a_background_index() {
        let ball = Arc::new(mesh::samples::heavy_ball(60_000, 100.0));
        let drawing = heavy_drawing(1, Arc::clone(&ball));
        let mut tracker = ComponentTracker::default();
        tracker.prepare(&drawing);
        let pointer = pointer_at(Vec3::new(0.0, -100.0, 100.0));
        let busy = tracker.preparing();
        tracker.update(Mode::Polygons, &drawing, &Document::new(), Some(pointer));
        if busy && tracker.preparing() {
            assert_eq!(tracker.hovered(), None, "nothing until it is ready");
            assert_eq!(tracker.building.len(), 1, "still building, not joined");
        }
        while tracker.preparing() {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let mut later = drawing.clone();
        later.version = 1;
        tracker.update(Mode::Polygons, &later, &Document::new(), Some(pointer));
        assert!(tracker.hovered().is_some(), "found once it is ready");
    }

    #[test]
    fn during_a_drag_the_scene_is_kept_and_nothing_is_looked_up() {
        let ball = Arc::new(mesh::samples::heavy_ball(2_000, 100.0));
        let drawing = heavy_drawing(1, Arc::clone(&ball));
        let mut tracker = ComponentTracker::default();
        let pointer = pointer_at(Vec3::new(0.0, -100.0, 100.0));
        tracker.update(Mode::Polygons, &drawing, &Document::new(), Some(pointer));
        assert!(tracker.hovered().is_some());
        tracker.freeze(true);
        // A newer drawing (the shape moving) keeps the scene as it was,
        // and nothing is hovered while dragging.
        let mut moved = (*ball).clone();
        let v = moved.vertices().next().unwrap().0;
        moved.set_position(v, Vec3::splat(500.0));
        let newer = heavy_drawing(2, Arc::new(moved));
        tracker.update(Mode::Polygons, &newer, &Document::new(), Some(pointer));
        assert_eq!(tracker.hovered(), None);
        let kept = tracker.pick_scene(&newer).triangle_count();
        assert_eq!(
            tracker.scene.as_ref().unwrap().0,
            1,
            "still the first drawing"
        );
        assert_eq!(kept, ball.face_layout().1.len() - 2 * ball.face_count());
        tracker.freeze(false);
        tracker.pick_scene(&newer);
        assert_eq!(tracker.scene.as_ref().unwrap().0, 2, "brought up to date");
    }

    #[test]
    fn the_hovered_element_reads_naturally() {
        let mut doc = Document::new();
        doc.execute(scene::Command::Create(scene::ObjectKind::Primitive(
            PrimitiveKind::Cube.standard(),
        )))
        .unwrap();
        let id = doc.selection()[0];
        let v = scene::VertexId::from_raw;
        assert_eq!(
            describe(&doc, (id, Element::Point(v(4)))),
            "point 4 of Cube"
        );
        assert_eq!(
            describe(&doc, (id, Element::Edge(mesh::EdgeKey::new(v(5), v(2))))),
            "edge 2–5 of Cube"
        );
        assert_eq!(
            describe(&doc, (id, Element::Polygon(scene::FaceId::from_raw(3)))),
            "polygon 3 of Cube"
        );
    }
}
