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
use std::sync::Arc;
use tools::{Camera, Element, ElementKind, PickScene};

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
/// of element, the hovered element and the selected elements' revision.
type BuiltFor = (u64, Option<ElementKind>, Option<ObjectElement>, u64);

/// Keeps track of the element under the mouse and the overlay.
#[derive(Debug, Default)]
pub(crate) struct ComponentTracker {
    /// The scene the mouse is looked up in, and the drawing version it was
    /// built from.
    scene: Option<(u64, PickScene)>,
    hovered: Option<ObjectElement>,
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
            (Some(kind), Some(pointer)) => self.find(kind, drawing, pointer),
            _ => None,
        };
        let wanted = (
            drawing.version,
            kind,
            self.hovered,
            document.element_revision(),
        );
        if self.built_for != Some(wanted) {
            let version = self.overlay.version + 1;
            self.overlay = Arc::new(match kind {
                Some(kind) => build(kind, drawing, document, self.hovered, version),
                None => ComponentOverlay {
                    version,
                    ..ComponentOverlay::default()
                },
            });
            self.built_for = Some(wanted);
        }
        Arc::clone(&self.overlay)
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
        if self
            .scene
            .as_ref()
            .is_none_or(|(v, _)| *v != drawing.version)
        {
            let scene = PickScene::new(
                drawing
                    .items
                    .iter()
                    .map(|item| (item.mesh.as_ref(), item.transform, item.selected)),
            );
            self.scene = Some((drawing.version, scene));
        }
        let (_, scene) = self.scene.get_or_insert_with(Default::default);
        scene
    }

    fn find(
        &mut self,
        kind: ElementKind,
        drawing: &SceneDrawing,
        pointer: ViewPointer,
    ) -> Option<ObjectElement> {
        let found =
            self.pick_scene(drawing)
                .under(kind, &pointer.camera, pointer.size, pointer.at)?;
        let item = drawing.items.get(found.object)?;
        Some((item.id, found.element))
    }
}

/// How an element is drawn: under the mouse, selected, or plain.
fn style(hovered: bool, selected: bool) -> OverlayStyle {
    if hovered {
        OverlayStyle::Hovered
    } else if selected {
        OverlayStyle::Selected
    } else {
        OverlayStyle::Plain
    }
}

/// The overlay for `kind`: the edited objects' edges (and points, in the
/// Points mode; selected polygons, in the Polygons mode), with the
/// selected and hovered elements picked out.
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
        for edge in item.mesh.edges() {
            let (a, b) = edge.ends();
            if let (Some(a), Some(b)) = (place(a), place(b)) {
                let element = Element::Edge(edge);
                let chosen = kind == ElementKind::Edges && selected.contains(element);
                overlay
                    .lines
                    .push(([a, b], style(hovered_here == Some(element), chosen)));
            }
        }
        if kind == ElementKind::Points {
            for (v, p) in item.mesh.vertices() {
                let element = Element::Point(v);
                overlay.points.push((
                    item.transform.transform_point3(p),
                    style(hovered_here == Some(element), selected.contains(element)),
                ));
            }
        }
        if kind == ElementKind::Polygons {
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
    fn the_points_mode_draws_every_point_and_edge_of_the_selection() {
        let mut tracker = ComponentTracker::default();
        let overlay = tracker.update(Mode::Points, &drawing(true), &Document::new(), None);
        assert_eq!(overlay.points.len(), 8);
        assert_eq!(overlay.lines.len(), 12);
        assert!(overlay.tints.is_empty());
        assert!(
            overlay
                .points
                .iter()
                .all(|(_, s)| *s == OverlayStyle::Plain)
        );
        // The edges mode draws edges only.
        let overlay = tracker.update(Mode::Edges, &drawing(true), &Document::new(), None);
        assert!(overlay.points.is_empty());
        assert_eq!(overlay.lines.len(), 12);
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
